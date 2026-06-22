use anyhow::{bail, Result};
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::TensorRef};
use serde::Deserialize;
use tokenizers::Tokenizer;
use std::sync::atomic::{AtomicBool, Ordering};

pub static DISABLE_LIVE_RESULTS: AtomicBool = AtomicBool::new(false);

const CATALOG: &[u8] = include_bytes!("../../assets/catalog.bin");
const TOKENIZER: &[u8] = include_bytes!("../../assets/model/tokenizer.json");

#[derive(Clone)]
pub struct CatalogEntry {
    pub id: String,
    pub control_name: String,
    pub breadcrumb_path: String,
    pub launch_command: String,
    pub source: String,
    pub description: String,
    pub synonyms: String,
}

#[derive(Clone)]
pub struct SearchResult {
    pub entry: CatalogEntry,
    pub score: f32,
}

#[derive(Deserialize)]
struct MetaJson {
    control_name: String,
    breadcrumb_path: String,
    launch_command: String,
    source: String,
    id: String,
    description: String,
    synonyms: String,
}

#[derive(Clone)]
pub struct AnchorCategory {
    pub name: &'static str,
    pub target_id: &'static str,
    pub translation_tip: &'static str,
    pub phrases: &'static [&'static str],
    pub vecs: Vec<Vec<f32>>,
}

#[derive(Clone)]
pub struct AppInfo {
    pub name: String,
    pub path: String,
}

pub struct SearchEngine {
    vecs: Vec<f32>,
    meta: Vec<CatalogEntry>,
    n: usize,
    dim: usize,
    session: Session,
    tokenizer: Tokenizer,
    anchor_categories: Vec<AnchorCategory>,
    apps: Vec<AppInfo>,
}

impl SearchEngine {
    pub fn new(model_path: &std::path::Path) -> Result<Self> {
        if CATALOG.len() < 8 {
            bail!("catalog.bin too small");
        }
        let n   = u32::from_le_bytes(CATALOG[0..4].try_into()?) as usize;
        let dim = u32::from_le_bytes(CATALOG[4..8].try_into()?) as usize;

        let mut off = 8usize;
        let mut vecs = Vec::with_capacity(n * dim);
        let mut meta = Vec::with_capacity(n);

        for _ in 0..n {
            let vb = dim * 4;
            for chunk in CATALOG[off..off + vb].chunks_exact(4) {
                vecs.push(f32::from_le_bytes(chunk.try_into()?));
            }
            off += vb;
            let ml = u16::from_le_bytes(CATALOG[off..off + 2].try_into()?) as usize;
            off += 2;
            let m: MetaJson = serde_json::from_slice(&CATALOG[off..off + ml])?;
            off += ml;
            meta.push(CatalogEntry {
                id: m.id,
                control_name: m.control_name,
                breadcrumb_path: m.breadcrumb_path,
                launch_command: m.launch_command,
                source: m.source,
                description: m.description,
                synonyms: m.synonyms,
            });
        }

        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .with_intra_threads(1)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .commit_from_file(model_path)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let tokenizer = Tokenizer::from_bytes(TOKENIZER)
            .map_err(|e| anyhow::anyhow!("tokenizer: {e}"))?;

        let mut anchor_categories = vec![
            AnchorCategory {
                name: "eyes_hurt",
                target_id: "system.night_light",
                translation_tip: "Translation Tip: Eye strain? Filter blue light or adjust display brightness. | System > Display > Brightness & color > Night light",
                phrases: &["my eyes hurt", "eye strain", "screen too bright", "reduce blue light", "eyes hurt", "blue light filter"],
                vecs: vec![],
            },
            AnchorCategory {
                name: "internet_slow",
                target_id: "system.troubleshoot.network-internet-troubleshooter",
                translation_tip: "Translation Tip: Slow connection? Diagnose network adapter and DNS. | System > Troubleshoot > Other troubleshooters > Network and Internet",
                phrases: &["internet is slow", "wi-fi is slow", "wifi is slow", "slow network", "connection speed", "slow internet", "slow wifi"],
                vecs: vec![],
            },
            AnchorCategory {
                name: "mouse_flying",
                target_id: "bluetooth-devices.mouse.enhance-pointer-precision",
                translation_tip: "Translation Tip: Erratic mouse? Adjust cursor speed and pointer precision. | Bluetooth & devices > Mouse > Enhance pointer precision",
                phrases: &["mouse is flying", "cursor moving too fast", "erratic mouse speed", "mouse speed too high", "pointer speed is fast"],
                vecs: vec![],
            },
            AnchorCategory {
                name: "battery_dying",
                target_id: "system.power.energy_saver",
                translation_tip: "Translation Tip: Battery low? Enable Energy Saver to extend power. | System > Power & battery > Energy saver",
                phrases: &["battery is dying", "battery low", "running out of power", "extend battery life", "battery saver", "laptop dying"],
                vecs: vec![],
            },
            AnchorCategory {
                name: "cant_see_text",
                target_id: "text_size.text_size",
                translation_tip: "Translation Tip: Text too small? Adjust scale or make text bigger. | Text size > Text size",
                phrases: &["can't see text", "text is too small", "make font size bigger", "increase UI scale", "font is tiny", "screen is too small"],
                vecs: vec![],
            },
        ];

        let mut engine = Self { vecs, meta, n, dim, session, tokenizer, anchor_categories: vec![], apps: vec![] };
        for cat in &mut anchor_categories {
            for phrase in cat.phrases {
                let phrase_with_prefix = format!("query: {}", phrase);
                if let Ok(v) = engine.embed(&phrase_with_prefix) {
                    cat.vecs.push(v);
                }
            }
        }
        engine.anchor_categories = anchor_categories;
        engine.apps = scan_apps();
        let _ = engine.search("settings", 1);
        Ok(engine)
    }

    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let q = query.trim();
        if q.is_empty() { return vec![]; }

        let q_lower = q.to_lowercase();
        let stop_words = ["what", "is", "a", "the", "to", "for", "in", "of", "and", "or", "with", "on", "at", "by", "from", "about", "how", "this", "it", "my", "your"];
        let q_words: Vec<&str> = q_lower.split_whitespace()
            .filter(|w| !stop_words.contains(w))
            .collect();

        // BAAI models require queries to be prefixed with "query: "
        let query_with_prefix = format!("query: {}", q);
        let qvec = match self.embed(&query_with_prefix) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let mut scores: Vec<(usize, f32)> = (0..self.n)
            .map(|i| {
                let entry = &self.meta[i];

                // 1. Semantic score
                let sem_score: f32 = self.vecs[i * self.dim..][..self.dim]
                    .iter().zip(&qvec).map(|(a, b)| a * b).sum();

                // 2. Lexical score
                let mut lex_score = 0.0f32;
                let name_lower = entry.control_name.to_lowercase();

                // Title-level matching
                if name_lower == q_lower {
                    lex_score += 1.0;
                } else if name_lower.starts_with(&q_lower) {
                    lex_score += 0.7;
                } else if name_lower.contains(&q_lower) {
                    lex_score += 0.4;
                }

                // Position-based boost in Title
                if let Some(idx) = name_lower.find(&q_lower) {
                    let char_idx = name_lower[..idx].chars().count();
                    let total_chars = name_lower.chars().count();
                    if total_chars > 0 {
                        lex_score += 0.5 * (1.0 - (char_idx as f32 / total_chars as f32));
                    }
                }

                // Word overlap in title
                if !q_words.is_empty() {
                    let name_words: Vec<&str> = name_lower.split_whitespace().collect();
                    let mut matched_words = 0;
                    for qw in &q_words {
                        if name_words.contains(qw) {
                            matched_words += 1;
                        }
                    }
                    lex_score += 0.5 * (matched_words as f32 / q_words.len() as f32);
                }

                // Synonyms matching (split by pipe '|')
                let syn_lower = entry.synonyms.to_lowercase();
                let syn_list: Vec<&str> = syn_lower.split('|').collect();
                let mut syn_boost = 0.0f32;
                for syn in &syn_list {
                    let s_trimmed = syn.trim();
                    if s_trimmed == q_lower {
                        syn_boost = syn_boost.max(0.8);
                    } else if s_trimmed.starts_with(&q_lower) {
                        syn_boost = syn_boost.max(0.6);
                    } else if s_trimmed.contains(&q_lower) {
                        syn_boost = syn_boost.max(0.4);
                    }
                }

                // Word match in synonyms
                if !q_words.is_empty() {
                    let mut matched_words_in_syn = 0;
                    for qw in &q_words {
                        for syn in &syn_list {
                            let syn_words: Vec<&str> = syn.split_whitespace().collect();
                            if syn_words.contains(qw) {
                                matched_words_in_syn += 1;
                                break;
                            }
                        }
                    }
                    syn_boost += 0.3 * (matched_words_in_syn as f32 / q_words.len() as f32);
                }
                lex_score += syn_boost;

                // Breadcrumb matching
                let breadcrumb_lower = entry.breadcrumb_path.to_lowercase();
                if breadcrumb_lower.contains(&q_lower) {
                    lex_score += 0.2;
                }

                // Parent category matching (boost if search words match the parent category path, excluding the item itself)
                if let Some(last_arrow_idx) = breadcrumb_lower.rfind('>') {
                    let parent_categories = &breadcrumb_lower[..last_arrow_idx];
                    let mut matched_words_in_parent = 0;
                    for qw in &q_words {
                        if parent_categories.contains(qw) {
                            matched_words_in_parent += 1;
                        }
                    }
                    if !q_words.is_empty() {
                        lex_score += 0.3 * (matched_words_in_parent as f32 / q_words.len() as f32);
                    }
                }

                // Description matching
                let desc_lower = entry.description.to_lowercase();
                if desc_lower.contains(&q_lower) {
                    lex_score += 0.1;
                }

                // Boost legacy control panel options if they have any match
                if entry.source.to_lowercase().contains("legacy") && lex_score > 0.0 {
                    lex_score += 0.25;
                }

                (i, sem_score + lex_score * 0.25)
            })
            .collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut conv_results = self.get_conversational_results(&qvec);

        let mut final_results = get_live_results(q);
        let mut vec_results: Vec<SearchResult> = scores.into_iter()
            .filter(|(_, s)| *s > 0.62)
            .map(|(i, score)| SearchResult { entry: self.meta[i].clone(), score })
            .collect();
            
        if !conv_results.is_empty() {
            vec_results.retain(|vr| {
                !conv_results.iter().any(|cr| cr.entry.id == vr.entry.id)
            });
        }

        if !final_results.is_empty() {
            vec_results.retain(|vr| {
                !final_results.iter().any(|fr| {
                    // Check if control_name has substantial overlap or exact match
                    let fr_name = fr.entry.control_name.to_lowercase();
                    let vr_name = vr.entry.control_name.to_lowercase();
                    fr_name == vr_name || (fr_name.contains("battery") && vr_name.contains("battery"))
                })
            });
        }
        
        let mut app_matches = Vec::new();
        for app in &self.apps {
            let app_lower = app.name.to_lowercase();
            let mut score = 0.0f32;
            
            if app_lower == q_lower {
                score = 3.0; // Exact match
            } else if app_lower.starts_with(&q_lower) {
                score = 2.5; // Prefix match
            } else if q_lower.starts_with(&app_lower) {
                score = 2.2; // App name is a prefix of the query
            } else if app_lower.contains(&q_lower) {
                score = 1.8; // Substring match
            } else if q_lower.contains(&app_lower) {
                score = 1.5; // Query contains app name
            } else {
                let app_words: Vec<&str> = app_lower.split_whitespace().collect();
                let mut matched = 0;
                for qw in &q_words {
                    if app_words.contains(qw) {
                        matched += 1;
                    }
                }
                if matched > 0 && !q_words.is_empty() {
                    let ratio = matched as f32 / q_words.len() as f32;
                    if ratio >= 0.5 {
                        score = 0.8 + 0.4 * ratio;
                    }
                }
            }

            if score > 0.0 {
                app_matches.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("app.{}", app.name),
                        control_name: app.name.clone(),
                        breadcrumb_path: format!("Applications > {}", app.name),
                        launch_command: format!("shell:AppsFolder\\{}", app.path),
                        source: "app".to_string(),
                        description: format!("Launch {}", app.name),
                        synonyms: app.name.to_lowercase(),
                    },
                    score,
                });
            }
        }
        
        app_matches.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let mut merged = Vec::new();
        merged.append(&mut app_matches);
        merged.append(&mut vec_results);
        merged.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        conv_results.append(&mut final_results);
        final_results = conv_results;
        final_results.append(&mut merged);
        
        final_results.truncate(top_k.saturating_sub(1));

        let encoded_query = url_encode(q);
        final_results.push(SearchResult {
            entry: CatalogEntry {
                id: "web_search".to_string(),
                control_name: format!("Search Google for \"{}\"", q),
                breadcrumb_path: "Web > Google Search > Open in default browser".to_string(),
                launch_command: format!("https://www.google.com/search?q={}", encoded_query),
                source: "web".to_string(),
                description: format!("Opens default browser and searches Google for '{}'.", q),
                synonyms: "google search web internet online".to_string(),
            },
            score: 0.0,
        });

        final_results
    }

    fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        let enc = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow::anyhow!("encode: {e}"))?;

        let ids: Vec<i64>   = enc.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64>  = enc.get_attention_mask().iter().map(|&x| x as i64).collect();
        let types: Vec<i64> = enc.get_type_ids().iter().map(|&x| x as i64).collect();
        let seq = ids.len();

        let input_ids_t = TensorRef::<i64>::from_array_view(([1usize, seq], ids.as_slice()))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let attn_mask_t = TensorRef::<i64>::from_array_view(([1usize, seq], mask.as_slice()))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let type_ids_t  = TensorRef::<i64>::from_array_view(([1usize, seq], types.as_slice()))
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let outputs = self.session.run(ort::inputs![
            "input_ids"      => input_ids_t,
            "attention_mask" => attn_mask_t,
            "token_type_ids" => type_ids_t,
        ])?;

        let (_, hidden) = outputs["last_hidden_state"].try_extract_tensor::<f32>()?;

        Ok(mean_pool_norm(hidden, &mask, seq, self.dim))
    }

    fn get_conversational_results(&self, qvec: &[f32]) -> Vec<SearchResult> {
        if DISABLE_LIVE_RESULTS.load(Ordering::Relaxed) {
            return vec![];
        }
        let mut best_category: Option<&AnchorCategory> = None;
        let mut best_similarity = 0.0f32;

        for cat in &self.anchor_categories {
            let mut max_similarity = 0.0f32;
            for v in &cat.vecs {
                let sim: f32 = v.iter().zip(qvec).map(|(a, b)| a * b).sum();
                if sim > max_similarity {
                    max_similarity = sim;
                }
            }
            if max_similarity > best_similarity {
                best_similarity = max_similarity;
                best_category = Some(cat);
            }
        }

        if let Some(cat) = best_category {
            if best_similarity >= 0.80 {
                if let Some(entry) = self.meta.iter().find(|e| e.id == cat.target_id) {
                    let mut modified_entry = entry.clone();
                    modified_entry.breadcrumb_path = cat.translation_tip.to_string();
                    modified_entry.source = "translated".to_string();
                    return vec![SearchResult {
                        entry: modified_entry,
                        score: 5.0,
                    }];
                }
            }
        }
        vec![]
    }
}

pub fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.as_bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            b' ' => {
                encoded.push('+');
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

fn mean_pool_norm(hidden: &[f32], mask: &[i64], seq: usize, dim: usize) -> Vec<f32> {
    let mut sum = vec![0.0f32; dim];
    let mut count = 0u32;
    for t in 0..seq {
        if mask[t] == 1 {
            for d in 0..dim { sum[d] += hidden[t * dim + d]; }
            count += 1;
        }
    }
    if count > 0 { for x in &mut sum { *x /= count as f32; } }
    let norm = sum.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 { for x in &mut sum { *x /= norm; } }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_search_accuracy() {
        DISABLE_LIVE_RESULTS.store(true, Ordering::Relaxed);
        let exe = std::env::current_exe().expect("failed to get current exe");
        let parent = exe.parent().expect("failed to get parent");
        let mut model_path = parent.join("model_int8.onnx");
        if !model_path.exists() {
            model_path = parent.parent().expect("failed to get grandparent").join("model_int8.onnx");
        }
        let mut engine = SearchEngine::new(&model_path).expect("Failed to initialize engine");
        
        let queries = vec![
            ("stop mouse from jumping", vec!["pointer precision", "pointer speed", "mouse"]),
            ("disable startup programs", vec!["startup", "autostart"]),
            ("change time zone", vec!["time zone", "timezone"]),
            ("turn off notifications", vec!["notification"]),
            ("fix blurry text", vec!["ccd cleartype", "cleartype", "dpi", "blurry", "scale"]),
            ("allow apps through firewall", vec!["firewall"]),
            ("make text bigger", vec!["text size", "font size", "scale", "display"]),
            ("change display brightness", vec!["brightness"]),
            ("connect to wifi", vec!["wi-fi", "wifi", "wireless"]),
            ("remove a printer", vec!["printer", "print"]),
            ("enable dark mode", vec!["dark", "color mode", "theme", "appearance"]),
            ("change screen resolution", vec!["resolution", "display"]),
            ("set default browser", vec!["default app", "default browser", "browser"]),
            ("disable auto updates", vec!["update", "windows update"]),
            ("sleep settings", vec!["sleep", "power"]),
            ("change wallpaper", vec!["wallpaper", "background", "desktop background"]),
            ("enable bluetooth", vec!["bluetooth"]),
            ("disable touchpad", vec!["touchpad", "trackpad"]),
            ("configure microphone", vec!["microphone", "input device"]),
            ("change language", vec!["language", "region"]),
            ("set up fingerprint login", vec!["fingerprint", "biometric", "windows hello"]),
            ("clear storage space", vec!["storage", "disk cleanup", "disk space"]),
            ("rename this computer", vec!["computer name", "rename pc", "device name"]),
            ("change sound output device", vec!["sound output", "audio output", "speaker", "playback"]),
            ("reduce eye strain at night", vec!["night light", "blue light", "color temperature"]),
            ("stop apps from running in background", vec!["background app"]),
            ("speed up animations", vec!["animation", "visual effect", "transition"]),
            ("uninstall a program", vec!["uninstall", "remove app", "apps & features"]),
            ("disable cortana", vec!["cortana", "search"]),
            ("set proxy settings", vec!["proxy"]),
            ("change mouse speed", vec!["pointer speed", "mouse speed", "cursor speed"]),
            ("flip screen upside down", vec!["rotation", "orientation", "display"]),
            ("enable remote desktop", vec!["remote desktop", "rdp"]),
            ("set up vpn", vec!["vpn", "virtual private"]),
            ("configure parental controls", vec!["parental", "family safety", "child"]),
            ("map network drive", vec!["network drive", "map drive"]),
            ("change power plan", vec!["power plan", "battery saver", "performance"]),
            ("set up email account", vec!["email", "mail", "account"]),
            ("configure taskbar", vec!["taskbar"]),
            ("disable location services", vec!["location"]),
            ("change keyboard layout", vec!["keyboard layout", "input method", "language"]),
            ("enable magnifier", vec!["magnifier"]),
            ("set up multiple monitors", vec!["multiple display", "second screen", "extend"]),
            ("change user account picture", vec!["account picture", "profile picture", "user photo"]),
            ("disable password requirement", vec!["password", "sign-in", "sign in option"]),
            ("configure storage sense", vec!["storage sense"]),
            ("enable developer mode", vec!["developer mode"]),
            ("sync settings between devices", vec!["sync", "backup", "cloud"]),
            ("change default search engine", vec!["search", "default search"]),
        ];

        let mut hits = 0;
        let mut misses = vec![];

        for (q, keywords) in &queries {
            let results = engine.search(q, 3);
            let mut hit = false;
            for r in &results {
                let haystack = format!(
                    "{} {} {}",
                    r.entry.control_name,
                    r.entry.breadcrumb_path,
                    r.entry.synonyms
                ).to_lowercase();
                if keywords.iter().any(|kw| haystack.contains(&kw.to_lowercase())) {
                    hit = true;
                    break;
                }
            }
            if hit {
                hits += 1;
            } else {
                let got = if results.is_empty() {
                    "None".to_string()
                } else {
                    format!("{} ({})", results[0].entry.control_name, results[0].entry.breadcrumb_path)
                };
                misses.push((q, got));
            }
        }

        let hit_rate = (hits as f32 / queries.len() as f32) * 100.0;
        println!("Rust Hit@3 rate: {}/{} = {:.1}%", hits, queries.len(), hit_rate);
        if !misses.is_empty() {
            println!("Misses:");
            for (q, got) in misses {
                println!("  Query '{}' -> got: {}", q, got);
            }
        }

        assert!(hit_rate >= 70.0, "Hit rate was only {:.1}% (target: >= 70.0%)", hit_rate);
    }

    #[test]
    fn test_enumerate_apps_folder() {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
        use windows::Win32::UI::Shell::{SHGetKnownFolderItem, FOLDERID_AppsFolder, KF_FLAG_DEFAULT, IShellItem, IEnumShellItems, BHID_EnumItems, SIGDN_NORMALDISPLAY, SIGDN_DESKTOPABSOLUTEPARSING};
        use windows::Win32::Foundation::HANDLE;

        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
            let apps_folder: IShellItem = SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, HANDLE::default()).unwrap();
            let enum_items: IEnumShellItems = apps_folder.BindToHandler(None, &BHID_EnumItems).unwrap();
            let mut items = [None];
            let mut fetched = 0;
            let mut count = 0;
            while enum_items.Next(&mut items, Some(&mut fetched)).is_ok() && fetched == 1 {
                if let Some(item) = &items[0] {
                    let display_name_ptr = item.GetDisplayName(SIGDN_NORMALDISPLAY).unwrap();
                    let parsing_name_ptr = item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING).unwrap();
                    
                    let mut len = 0;
                    while *display_name_ptr.0.add(len) != 0 { len += 1; }
                    let display_name = String::from_utf16_lossy(std::slice::from_raw_parts(display_name_ptr.0, len));
                    
                    let mut len = 0;
                    while *parsing_name_ptr.0.add(len) != 0 { len += 1; }
                    let parsing_name = String::from_utf16_lossy(std::slice::from_raw_parts(parsing_name_ptr.0, len));
                    
                    println!("App: {} -> {}", display_name, parsing_name);
                    windows::Win32::System::Com::CoTaskMemFree(Some(display_name_ptr.0 as *const _));
                    windows::Win32::System::Com::CoTaskMemFree(Some(parsing_name_ptr.0 as *const _));
                    count += 1;
                    if count >= 10 {
                        break;
                    }
                }
            }
        }
    }
}

#[repr(C)]
struct SYSTEM_POWER_STATUS {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[repr(C)]
struct MEMORYSTATUSEX {
    dw_length: u32,
    dw_memory_load: u32,
    ull_total_phys: u64,
    ull_avail_phys: u64,
    ull_total_page_file: u64,
    ull_avail_page_file: u64,
    ull_total_virtual: u64,
    ull_avail_virtual: u64,
    ull_avail_extended_virtual: u64,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(lpSystemPowerStatus: *mut SYSTEM_POWER_STATUS) -> i32;
    fn GetDiskFreeSpaceExW(
        lpDirectoryName: *const u16,
        lpFreeBytesAvailableToCaller: *mut u64,
        lpTotalNumberOfBytes: *mut u64,
        lpTotalNumberOfFreeBytes: *mut u64,
    ) -> i32;
    fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> i32;
}

fn get_local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

fn get_live_results(query: &str) -> Vec<SearchResult> {
    if DISABLE_LIVE_RESULTS.load(Ordering::Relaxed) {
        return vec![];
    }
    let q = query.to_lowercase();
    let mut results = vec![];

    // 1. Battery Status
    if q.contains("battery") || q.contains("power") || q.contains("charge") {
        let mut status = SYSTEM_POWER_STATUS {
            ac_line_status: 0,
            battery_flag: 0,
            battery_life_percent: 0,
            system_status_flag: 0,
            battery_life_time: 0,
            battery_full_life_time: 0,
        };
        unsafe {
            if GetSystemPowerStatus(&mut status) != 0 {
                let percent = status.battery_life_percent;
                if percent != 255 {
                    let state = if status.ac_line_status == 1 {
                        "Charging"
                    } else if status.ac_line_status == 0 {
                        "Discharging"
                    } else {
                        "Unknown State"
                    };
                    
                    results.push(SearchResult {
                        entry: CatalogEntry {
                            id: "live.battery".to_string(),
                            control_name: "Battery Status".to_string(),
                            breadcrumb_path: format!("System > Power & battery > Currently {}% ({})", percent, state),
                            launch_command: "ms-settings:powersleep".to_string(),
                            source: "LIVE".to_string(),
                            description: "Shows the current battery level and power state.".to_string(),
                            synonyms: "battery percentage power life status".to_string(),
                        },
                        score: 2.0,
                    });
                }
            }
        }
    }

    // 2. Local IP Address
    if q.contains("ip") || q.contains("network") || q.contains("address") || q.contains("wifi") || q.contains("ethernet") {
        if let Some(ip) = get_local_ip() {
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: "live.ip".to_string(),
                    control_name: "Copy Local IP Address".to_string(),
                    breadcrumb_path: format!("Network > Connection > {} (Press Enter to copy)", ip),
                    launch_command: format!("copy:{}", ip),
                    source: "ACTION".to_string(),
                    description: "Copies your current local IP address to the clipboard.".to_string(),
                    synonyms: "ip address local network connection".to_string(),
                },
                score: 2.0,
            });
        }
    }

    // 3. System RAM Usage
    if q.contains("ram") || q.contains("memory") || q.contains("perf") || q.contains("speed") {
        let mut mem = MEMORYSTATUSEX {
            dw_length: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            dw_memory_load: 0,
            ull_total_phys: 0,
            ull_avail_phys: 0,
            ull_total_page_file: 0,
            ull_avail_page_file: 0,
            ull_total_virtual: 0,
            ull_avail_virtual: 0,
            ull_avail_extended_virtual: 0,
        };
        unsafe {
            if GlobalMemoryStatusEx(&mut mem) != 0 {
                let avail_gb = mem.ull_avail_phys as f64 / 1024.0 / 1024.0 / 1024.0;
                let total_gb = mem.ull_total_phys as f64 / 1024.0 / 1024.0 / 1024.0;
                let load = mem.dw_memory_load;
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: "live.ram".to_string(),
                        control_name: "System Memory".to_string(),
                        breadcrumb_path: format!("System > Performance > {:.1} GB free / {:.1} GB total ({}% used)", avail_gb, total_gb, load),
                        launch_command: "taskmgr.exe".to_string(),
                        source: "LIVE".to_string(),
                        description: "Shows currently free physical RAM and memory load percentage.".to_string(),
                        synonyms: "ram memory physical usage performance".to_string(),
                    },
                    score: 2.0,
                });
            }
        }
    }

    // 4. Disk Space
    if q.contains("disk") || q.contains("storage") || q.contains("space") || q.contains("drive") || q.contains("free") {
        let mut free = 0u64;
        let mut total = 0u64;
        unsafe {
            if GetDiskFreeSpaceExW(std::ptr::null(), &mut free, &mut total, std::ptr::null_mut()) != 0 {
                let free_gb = free as f64 / 1024.0 / 1024.0 / 1024.0;
                let total_gb = total as f64 / 1024.0 / 1024.0 / 1024.0;
                let free_percent = if total > 0 { (free as f64 / total as f64) * 100.0 } else { 0.0 };
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: "live.disk".to_string(),
                        control_name: "Disk Space (C:)".to_string(),
                        breadcrumb_path: format!("System > Storage > {:.1} GB free of {:.1} GB ({:.1}% free)", free_gb, total_gb, free_percent),
                        launch_command: "ms-settings:storagesense".to_string(),
                        source: "LIVE".to_string(),
                        description: "Shows free space on your system partition (C: drive).".to_string(),
                        synonyms: "disk storage space free hard drive c".to_string(),
                    },
                    score: 2.0,
                });
            }
        }
    }

    results
}

fn scan_apps() -> Vec<AppInfo> {
    let mut apps = Vec::new();
    unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
        use windows::Win32::UI::Shell::{
            SHGetKnownFolderItem, FOLDERID_AppsFolder, KF_FLAG_DEFAULT, IShellItem, IEnumShellItems,
            BHID_EnumItems, SIGDN_NORMALDISPLAY, SIGDN_DESKTOPABSOLUTEPARSING
        };
        use windows::Win32::Foundation::HANDLE;

        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        
        let apps_folder: IShellItem = match SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, HANDLE::default()) {
            Ok(folder) => folder,
            Err(_) => return apps,
        };
        
        let enum_items: IEnumShellItems = match apps_folder.BindToHandler(None, &BHID_EnumItems) {
            Ok(e) => e,
            Err(_) => return apps,
        };
        
        let mut items = [None];
        let mut fetched = 0;
        while enum_items.Next(&mut items, Some(&mut fetched)).is_ok() && fetched == 1 {
            if let Some(item) = &items[0] {
                let display_name_ptr = match item.GetDisplayName(SIGDN_NORMALDISPLAY) {
                    Ok(ptr) => ptr,
                    Err(_) => continue,
                };
                let parsing_name_ptr = match item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING) {
                    Ok(ptr) => ptr,
                    Err(_) => {
                        windows::Win32::System::Com::CoTaskMemFree(Some(display_name_ptr.0 as *const _));
                        continue;
                    }
                };
                
                let mut len = 0;
                while *display_name_ptr.0.add(len) != 0 { len += 1; }
                let display_name = String::from_utf16_lossy(std::slice::from_raw_parts(display_name_ptr.0, len));
                
                let mut len = 0;
                while *parsing_name_ptr.0.add(len) != 0 { len += 1; }
                let parsing_name = String::from_utf16_lossy(std::slice::from_raw_parts(parsing_name_ptr.0, len));
                
                windows::Win32::System::Com::CoTaskMemFree(Some(display_name_ptr.0 as *const _));
                windows::Win32::System::Com::CoTaskMemFree(Some(parsing_name_ptr.0 as *const _));
                
                let display_name_lower = display_name.to_lowercase();
                if display_name_lower.contains("uninstall")
                    || display_name_lower.contains("help")
                    || display_name_lower.contains("documentation")
                    || display_name_lower.contains("readme")
                    || display_name_lower.contains("read me")
                    || display_name_lower.contains("release notes")
                    || display_name_lower.contains("whats new")
                    || display_name_lower.contains("what's new")
                    || display_name_lower.contains("license")
                    || display_name_lower.contains("changelog")
                    || display_name_lower.contains("website")
                    || display_name_lower.contains("manual")
                    || display_name_lower.contains("visit")
                    || display_name_lower.contains("about")
                {
                    continue;
                }

                let path_lower = parsing_name.to_lowercase();
                let is_document = path_lower.ends_with(".txt")
                    || path_lower.ends_with(".url")
                    || path_lower.ends_with(".chm")
                    || path_lower.ends_with(".hlp")
                    || path_lower.ends_with(".pdf")
                    || path_lower.ends_with(".html")
                    || path_lower.ends_with(".htm")
                    || path_lower.ends_with(".png")
                    || path_lower.ends_with(".jpg")
                    || path_lower.ends_with(".jpeg")
                    || path_lower.ends_with(".gif")
                    || path_lower.ends_with(".ico")
                    || path_lower.ends_with(".ini")
                    || path_lower.ends_with(".cfg")
                    || path_lower.ends_with(".xml")
                    || path_lower.ends_with(".json")
                    || path_lower.ends_with(".md")
                    || path_lower.ends_with(".rtf")
                    || path_lower.ends_with(".log")
                    || path_lower.ends_with(".doc")
                    || path_lower.ends_with(".docx")
                    || path_lower.ends_with(".xls")
                    || path_lower.ends_with(".xlsx")
                    || path_lower.ends_with(".ppt")
                    || path_lower.ends_with(".pptx")
                    || path_lower.ends_with(".zip")
                    || path_lower.ends_with(".rar")
                    || path_lower.ends_with(".7z");
                
                if is_document {
                    continue;
                }
                
                apps.push(AppInfo {
                    name: display_name,
                    path: parsing_name,
                });
            }
        }
    }
    
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps.dedup_by(|a, b| a.name == b.name);
    apps
}
