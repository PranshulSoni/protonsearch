use anyhow::{bail, Result};
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::TensorRef};
use serde::Deserialize;
use tokenizers::Tokenizer;

const CATALOG: &[u8] = include_bytes!("../../assets/catalog.bin");
const MODEL: &[u8] = include_bytes!("../../assets/model/model_int8.onnx");
const TOKENIZER: &[u8] = include_bytes!("../../assets/model/tokenizer.json");

#[derive(Clone)]
pub struct CatalogEntry {
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
    #[allow(dead_code)]
    id: String,
    description: String,
    synonyms: String,
}

pub struct SearchEngine {
    vecs: Vec<f32>,
    meta: Vec<CatalogEntry>,
    n: usize,
    dim: usize,
    session: Session,
    tokenizer: Tokenizer,
}

impl SearchEngine {
    pub fn new() -> Result<Self> {
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
            .with_intra_threads(2)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .commit_from_memory(MODEL)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let tokenizer = Tokenizer::from_bytes(TOKENIZER)
            .map_err(|e| anyhow::anyhow!("tokenizer: {e}"))?;

        let mut engine = Self { vecs, meta, n, dim, session, tokenizer };
        let _ = engine.search("settings", 1);
        Ok(engine)
    }

    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let q = query.trim();
        if q.is_empty() { return vec![]; }

        let q_lower = q.to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();

        let qvec = match self.embed(q) {
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

                (i, sem_score + lex_score)
            })
            .collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scores.into_iter().take(top_k)
            .filter(|(_, s)| *s > 0.3)
            .map(|(i, score)| SearchResult { entry: self.meta[i].clone(), score })
            .collect()
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
        let mut engine = SearchEngine::new().expect("Failed to initialize engine");
        
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
}
