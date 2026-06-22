use anyhow::{bail, Result};
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::TensorRef};
use serde::Deserialize;
use tokenizers::Tokenizer;
use std::sync::atomic::{AtomicBool, Ordering};
use rusqlite::Connection;

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

#[derive(Clone)]
pub struct RecentFileInfo {
    pub name: String,
    pub path: String,  // resolved target path
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
    recent_files: Vec<RecentFileInfo>,
    db_path: std::path::PathBuf,
}

impl SearchEngine {
    pub fn new(model_path: &std::path::Path, db_path: std::path::PathBuf) -> Result<Self> {
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

        let mut engine = Self { vecs, meta, n, dim, session, tokenizer, anchor_categories: vec![], apps: vec![], recent_files: vec![], db_path };
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
        engine.recent_files = scan_recent_files();
        let _ = engine.search("settings", 1);
        Ok(engine)
    }

    fn search_local_files(&self, query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(_) => return results,
        };

        let q_lower = query.to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();
        if q_words.is_empty() { return results; }

        let name_query = format!("%{}%", q_lower);
        let mut stmt = match conn.prepare("SELECT path, name, extension, modified FROM files WHERE name LIKE ? LIMIT 15") {
            Ok(s) => s,
            Err(_) => return results,
        };

        let metadata_rows = stmt.query_map([&name_query], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        });

        if let Ok(rows) = metadata_rows {
            for row in rows.filter_map(|r| r.ok()) {
                let (path, name, ext, _modified) = row;
                
                let name_lower = name.to_lowercase();
                let name_no_ext = if let Some(dot) = name_lower.rfind('.') {
                    &name_lower[..dot]
                } else {
                    &name_lower
                };
                
                let mut score = 0.0f32;
                if name_lower == q_lower || name_no_ext == q_lower {
                    score = 2.5;
                } else if name_lower.starts_with(&q_lower) || name_no_ext.starts_with(&q_lower) {
                    score = 2.0;
                } else if name_lower.contains(&q_lower) {
                    score = 1.5;
                } else {
                    let name_words: Vec<&str> = name_no_ext.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
                    let matched = q_words.iter().filter(|w| name_words.contains(w)).count();
                    if matched > 0 {
                        score = 0.8 + 0.4 * (matched as f32 / q_words.len() as f32);
                    }
                }

                if score > 0.0 {
                    results.push(SearchResult {
                        entry: CatalogEntry {
                            id: format!("file.{}", path),
                            control_name: name.clone(),
                            breadcrumb_path: format!("File > {}", path),
                            launch_command: path.clone(),
                            source: "FILE".to_string(),
                            description: format!("Local {} file", ext.to_uppercase()),
                            synonyms: name.to_lowercase(),
                        },
                        score,
                    });
                }
            }
        }

        let clean_fts_query = q_words.join(" ");
        let mut stmt_fts = match conn.prepare(
            "SELECT f.path, f.name, f.extension, f.modified 
             FROM files f 
             JOIN files_fts fts ON f.path = fts.path 
             WHERE files_fts MATCH ? LIMIT 15"
        ) {
            Ok(s) => s,
            Err(_) => return results,
        };

        let fts_rows = stmt_fts.query_map([&clean_fts_query], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        });

        if let Ok(rows) = fts_rows {
            for row in rows.filter_map(|r| r.ok()) {
                let (path, name, ext, _modified) = row;
                
                if results.iter().any(|r| r.entry.launch_command == path) {
                    continue;
                }

                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("file.{}", path),
                        control_name: name.clone(),
                        breadcrumb_path: format!("File > {}", path),
                        launch_command: path.clone(),
                        source: "FILE".to_string(),
                        description: format!("Local {} file (matches content)", ext.to_uppercase()),
                        synonyms: name.to_lowercase(),
                    },
                    score: 1.0,
                });
            }
        }

        results
    }

    fn search_browser_items(&self, query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(_) => return results,
        };

        let q_lower = query.to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();
        if q_words.is_empty() { return results; }

        let name_query = format!("%{}%", q_lower);
        let mut stmt = match conn.prepare(
            "SELECT url, title, source, visit_count FROM browser_items 
             WHERE title LIKE ? OR url LIKE ? 
             ORDER BY visit_count DESC LIMIT 25"
        ) {
            Ok(s) => s,
            Err(_) => return results,
        };

        let rows = stmt.query_map([&name_query, &name_query], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        });

        if let Ok(rows) = rows {
            for row in rows.filter_map(|r| r.ok()) {
                let (url, title, source, visit_count) = row;
                
                let title_lower = title.to_lowercase();
                let url_lower = url.to_lowercase();
                
                let mut score = 0.0f32;
                
                if title_lower == q_lower || url_lower == q_lower {
                    score = 2.0;
                } else if title_lower.starts_with(&q_lower) || url_lower.starts_with(&q_lower) {
                    score = 1.6;
                } else if title_lower.contains(&q_lower) || url_lower.contains(&q_lower) {
                    score = 1.2;
                } else {
                    let matched = q_words.iter().filter(|w| title_lower.contains(*w) || url_lower.contains(*w)).count();
                    if matched > 0 {
                        score = 0.5 + 0.5 * (matched as f32 / q_words.len() as f32);
                    }
                }

                // Boost bookmarks
                if source.contains("bookmark") {
                    score += 0.4;
                }

                if score > 0.0 {
                    results.push(SearchResult {
                        entry: CatalogEntry {
                            id: format!("browser.{}", url),
                            control_name: title.clone(),
                            breadcrumb_path: format!("Browser > {}", url),
                            launch_command: url.clone(),
                            source: "BROWSER".to_string(),
                            description: format!("Bookmark/History from {}", if source.contains("chrome") { "Chrome" } else { "Edge" }),
                            synonyms: title.to_lowercase(),
                        },
                        score,
                    });
                }
            }
        }

        results
    }

    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let q = query.trim();
        if q.is_empty() || q.to_lowercase() == "recent" || q.to_lowercase() == "recents" {
            let mut results = Vec::new();
            for rf in self.recent_files.iter().take(top_k.min(20)) {
                let ext = rf.name.rsplit('.').next().unwrap_or("").to_uppercase();
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("recent.{}", rf.path),
                        control_name: rf.name.clone(),
                        breadcrumb_path: format!("Recent > {}", rf.path),
                        launch_command: rf.path.clone(),
                        source: "RECENT".to_string(),
                        description: format!("Recently opened {} file", ext),
                        synonyms: rf.name.to_lowercase(),
                    },
                    score: 3.0,
                });
            }
            return results;
        }

        // ── Calculator: inject instantly if query is a math expression ──────
        let calc_result: Option<SearchResult> = try_calc(q).map(|val| {
            let display = if val.fract() == 0.0 && val.abs() < 1e15 {
                format!("{}", val as i64)
            } else {
                // Up to 10 significant digits, strip trailing zeros
                let s = format!("{:.10}", val);
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            };
            SearchResult {
                entry: CatalogEntry {
                    id: "calc".to_string(),
                    control_name: format!("{} = {}", q, display),
                    breadcrumb_path: format!("Calculator > Press Enter to copy  {}", display),
                    launch_command: format!("copy:{}", display),
                    source: "CALC".to_string(),
                    description: format!("Math result: {}", display),
                    synonyms: String::new(),
                },
                score: 10.0,
            }
        });

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
                    let fr_name = fr.entry.control_name.to_lowercase();
                    let vr_name = vr.entry.control_name.to_lowercase();
                    fr_name == vr_name
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

        // Recent files matching
        let mut recent_matches = Vec::new();
        for rf in &self.recent_files {
            let name_lower = rf.name.to_lowercase();
            // Strip extension for matching (e.g. "report.pdf" → "report")
            let name_no_ext = if let Some(dot) = name_lower.rfind('.') {
                &name_lower[..dot]
            } else {
                &name_lower
            };
            let mut score = 0.0f32;
            if name_lower == q_lower || name_no_ext == q_lower {
                score = 2.8;
            } else if name_lower.starts_with(&q_lower) || name_no_ext.starts_with(&q_lower) {
                score = 2.3;
            } else if name_lower.contains(&q_lower) {
                score = 1.7;
            } else {
                let name_words: Vec<&str> = name_no_ext.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
                let mut matched = 0;
                for qw in &q_words {
                    if name_words.contains(qw) { matched += 1; }
                }
                if matched > 0 && !q_words.is_empty() {
                    let ratio = matched as f32 / q_words.len() as f32;
                    if ratio >= 0.5 { score = 0.6 + 0.4 * ratio; }
                }
            }
            if score > 0.0 {
                let ext = rf.name.rsplit('.').next().unwrap_or("").to_uppercase();
                recent_matches.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("recent.{}", rf.path),
                        control_name: rf.name.clone(),
                        breadcrumb_path: format!("Recent > {}", rf.path),
                        launch_command: rf.path.clone(),
                        source: "RECENT".to_string(),
                        description: format!("Recently opened {} file", ext),
                        synonyms: rf.name.to_lowercase(),
                    },
                    score,
                });
            }
        }
        recent_matches.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        recent_matches.truncate(5); // Cap at 5 recent file results

        let encoded_query = url_encode(q);
        let web_search = SearchResult {
            entry: CatalogEntry {
                id: "web_search".to_string(),
                control_name: format!("Search Google for \"{}\"", q),
                breadcrumb_path: "Web > Google Search > Open in default browser".to_string(),
                launch_command: format!("https://www.google.com/search?q={}", encoded_query),
                source: "web".to_string(),
                description: format!("Opens default browser and searches Google for '{}'.", q),
                synonyms: "google search web internet online".to_string(),
            },
            score: 1.1,
        };

        let mut file_matches = self.search_local_files(q);
        file_matches.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        file_matches.truncate(10); // Cap at 10 file results

        let mut browser_matches = self.search_browser_items(q);
        browser_matches.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        browser_matches.truncate(15); // Cap at 15 browser results

        let mut merged = Vec::new();
        merged.append(&mut app_matches);
        merged.append(&mut recent_matches);
        merged.append(&mut file_matches);
        merged.append(&mut browser_matches);
        merged.append(&mut vec_results);
        merged.push(web_search.clone());
        merged.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        conv_results.append(&mut final_results);
        final_results = conv_results;
        final_results.append(&mut merged);
        
        final_results.truncate(top_k);

        // Quick system actions: match against query
        let mut action_matches = get_quick_actions(q);
        for am in &action_matches {
            final_results.retain(|r| r.entry.control_name.to_lowercase() != am.entry.control_name.to_lowercase());
        }
        action_matches.append(&mut final_results);
        final_results = action_matches;

        // Prepend calc result if we got one
        if let Some(calc) = calc_result {
            final_results.insert(0, calc);
        }

        // Ensure web_search is always in the list as a fallback
        if !final_results.iter().any(|r| r.entry.id == "web_search") {
            final_results.push(web_search);
        }

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
        let mut engine = SearchEngine::new(&model_path, std::path::PathBuf::from("test_db.db")).expect("Failed to initialize engine");
        
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

// ── Recent Files ────────────────────────────────────────────────────────────
fn scan_recent_files() -> Vec<RecentFileInfo> {
    let mut results = Vec::new();

    // %APPDATA%\Microsoft\Windows\Recent
    let recent_dir = match std::env::var("APPDATA") {
        Ok(d) => std::path::PathBuf::from(d).join("Microsoft\\Windows\\Recent"),
        Err(_) => return results,
    };

    let entries = match std::fs::read_dir(&recent_dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    let mut file_entries: Vec<(std::path::PathBuf, std::time::SystemTime)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("lnk"))
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let path = e.path();
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((path, modified))
        })
        .collect();

    // Sort by modification time, newest first
    file_entries.sort_by(|a, b| b.1.cmp(&a.1));

    // Take at most 200 recent items (the list can be large)
    file_entries.truncate(200);

    for (lnk_path, _) in file_entries {
        // Resolve the .lnk target
        if let Some(target) = resolve_lnk_path(&lnk_path) {
            // Skip system folders, executables, etc. — keep documents
            let path_lower = target.to_lowercase();
            let is_useful = path_lower.ends_with(".pdf")
                || path_lower.ends_with(".docx")
                || path_lower.ends_with(".doc")
                || path_lower.ends_with(".xlsx")
                || path_lower.ends_with(".xls")
                || path_lower.ends_with(".pptx")
                || path_lower.ends_with(".ppt")
                || path_lower.ends_with(".txt")
                || path_lower.ends_with(".md")
                || path_lower.ends_with(".png")
                || path_lower.ends_with(".jpg")
                || path_lower.ends_with(".jpeg")
                || path_lower.ends_with(".mp4")
                || path_lower.ends_with(".mp3")
                || path_lower.ends_with(".zip")
                || path_lower.ends_with(".rs")
                || path_lower.ends_with(".py")
                || path_lower.ends_with(".js")
                || path_lower.ends_with(".ts")
                || path_lower.ends_with(".json")
                || path_lower.ends_with(".html")
                || path_lower.ends_with(".css");

            if !is_useful { continue; }

            // Get the file name from the target path
            let name = std::path::Path::new(&target)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&target)
                .to_string();

            results.push(RecentFileInfo { name, path: target });
        }
    }

    results
}

fn resolve_lnk_path(lnk_path: &std::path::Path) -> Option<String> {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, IPersistFile, STGM_READ};
    use windows::Win32::UI::Shell::{ShellLink, IShellLinkW, SLGP_UNCPRIORITY};
    use windows::core::{PCWSTR, Interface};

    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = link.cast().ok()?;
        let path_wide: Vec<u16> = lnk_path.to_str()?.encode_utf16().chain(std::iter::once(0)).collect();
        persist.Load(PCWSTR(path_wide.as_ptr()), STGM_READ).ok()?;
        let mut buffer = [0u16; 260];
        link.GetPath(&mut buffer, std::ptr::null_mut(), SLGP_UNCPRIORITY.0 as u32).ok()?;
        let target = String::from_utf16_lossy(&buffer);
        let trimmed = target.trim_matches('\0').trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    }
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

// ── Quick System Actions ────────────────────────────────────────────────────
// Each entry: (trigger_phrases, name, breadcrumb, launch_command, description)
struct QuickAction {
    triggers: &'static [&'static str],
    name: &'static str,
    breadcrumb: &'static str,
    launch_command: &'static str,
    description: &'static str,
}

static QUICK_ACTIONS: &[QuickAction] = &[
    QuickAction {
        triggers: &["lock", "lock screen", "lock pc", "lock computer"],
        name: "Lock Screen",
        breadcrumb: "System > Security > Lock this PC immediately",
        launch_command: "action:lock",
        description: "Lock the screen immediately.",
    },
    QuickAction {
        triggers: &["shutdown", "shut down", "power off", "turn off computer", "turn off pc"],
        name: "Shut Down",
        breadcrumb: "System > Power > Shut down this computer",
        launch_command: "action:shutdown",
        description: "Shut down the computer.",
    },
    QuickAction {
        triggers: &["restart", "reboot", "restart computer", "restart pc"],
        name: "Restart",
        breadcrumb: "System > Power > Restart this computer",
        launch_command: "action:restart",
        description: "Restart the computer.",
    },
    QuickAction {
        triggers: &["sleep", "hibernate", "sleep computer", "sleep pc"],
        name: "Sleep",
        breadcrumb: "System > Power > Put computer to sleep",
        launch_command: "action:sleep",
        description: "Put the computer to sleep.",
    },
    QuickAction {
        triggers: &["empty recycle bin", "clear recycle bin", "empty trash", "recycle bin"],
        name: "Empty Recycle Bin",
        breadcrumb: "System > Storage > Empty the Recycle Bin",
        launch_command: "action:recycle",
        description: "Permanently delete all items in the Recycle Bin.",
    },
    QuickAction {
        triggers: &["open downloads", "downloads folder", "my downloads"],
        name: "Open Downloads",
        breadcrumb: "File System > User > Downloads",
        launch_command: "action:folder:downloads",
        description: "Open the Downloads folder.",
    },
    QuickAction {
        triggers: &["open desktop", "desktop folder", "go to desktop"],
        name: "Open Desktop",
        breadcrumb: "File System > User > Desktop",
        launch_command: "action:folder:desktop",
        description: "Open the Desktop folder.",
    },
    QuickAction {
        triggers: &["open documents", "documents folder", "my documents"],
        name: "Open Documents",
        breadcrumb: "File System > User > Documents",
        launch_command: "action:folder:documents",
        description: "Open the Documents folder.",
    },
    QuickAction {
        triggers: &["open pictures", "pictures folder", "my pictures", "photos folder"],
        name: "Open Pictures",
        breadcrumb: "File System > User > Pictures",
        launch_command: "action:folder:pictures",
        description: "Open the Pictures folder.",
    },
    QuickAction {
        triggers: &["open music", "music folder", "my music"],
        name: "Open Music",
        breadcrumb: "File System > User > Music",
        launch_command: "action:folder:music",
        description: "Open the Music folder.",
    },
    QuickAction {
        triggers: &["open videos", "videos folder", "my videos"],
        name: "Open Videos",
        breadcrumb: "File System > User > Videos",
        launch_command: "action:folder:videos",
        description: "Open the Videos folder.",
    },
    QuickAction {
        triggers: &["open temp", "temp folder", "temporary files", "open tmp"],
        name: "Open Temp Folder",
        breadcrumb: "System > Temp > %TEMP% folder",
        launch_command: "action:folder:temp",
        description: "Open the Windows temporary files folder.",
    },
    QuickAction {
        triggers: &["open startup", "startup folder", "startup programs folder"],
        name: "Open Startup Folder",
        breadcrumb: "System > Startup > Shell startup programs",
        launch_command: "shell:startup",
        description: "Open the user startup programs folder.",
    },
    QuickAction {
        triggers: &["flush dns", "clear dns", "reset dns", "dns cache"],
        name: "Flush DNS Cache",
        breadcrumb: "Network > DNS > Flush resolver cache",
        launch_command: "action:flushdns",
        description: "Clear the DNS resolver cache.",
    },
    QuickAction {
        triggers: &["open task manager", "task manager", "taskmgr"],
        name: "Open Task Manager",
        breadcrumb: "System > Performance > Task Manager",
        launch_command: "taskmgr.exe",
        description: "Open Task Manager.",
    },
    QuickAction {
        triggers: &["open registry", "registry editor", "regedit"],
        name: "Open Registry Editor",
        breadcrumb: "System > Advanced > Registry Editor",
        launch_command: "regedit.exe",
        description: "Open the Windows Registry Editor.",
    },
    QuickAction {
        triggers: &["open environment variables", "environment variables", "env variables", "path variable"],
        name: "Environment Variables",
        breadcrumb: "System > Advanced System Settings > Environment Variables",
        launch_command: "action:envvars",
        description: "Open Environment Variables settings.",
    },
    QuickAction {
        triggers: &["clear clipboard", "empty clipboard", "reset clipboard"],
        name: "Clear Clipboard",
        breadcrumb: "System > Clipboard > Clear clipboard contents",
        launch_command: "action:clearclip",
        description: "Clear all contents from the clipboard.",
    },
    QuickAction {
        triggers: &["open hosts file", "hosts file", "edit hosts"],
        name: "Open Hosts File",
        breadcrumb: "Network > DNS > Edit hosts file",
        launch_command: "action:hosts",
        description: "Open the system hosts file in Notepad.",
    },
];

fn get_quick_actions(query: &str) -> Vec<SearchResult> {
    let q = query.trim().to_lowercase();
    if q.len() < 2 { return vec![]; }

    let mut matches = Vec::new();
    for action in QUICK_ACTIONS {
        let mut best_score = 0.0f32;
        for &trigger in action.triggers {
            let t = trigger.to_lowercase();
            let score = if t == q {
                3.5
            } else if t.starts_with(&q) {
                3.0
            } else if q.starts_with(&t) {
                2.8
            } else if t.contains(&q) {
                2.5
            } else if q.contains(&t) {
                2.3
            } else {
                // Word-level match
                let t_words: Vec<&str> = t.split_whitespace().collect();
                let q_words: Vec<&str> = q.split_whitespace().collect();
                let matched = q_words.iter().filter(|w| t_words.contains(w)).count();
                if matched > 0 {
                    let ratio = matched as f32 / q_words.len().max(1) as f32;
                    if ratio >= 0.5 { 1.5 + ratio } else { 0.0 }
                } else {
                    0.0
                }
            };
            if score > best_score { best_score = score; }
        }
        if best_score > 0.0 {
            matches.push(SearchResult {
                entry: CatalogEntry {
                    id: format!("action.{}", action.name.to_lowercase().replace(' ', "_")),
                    control_name: action.name.to_string(),
                    breadcrumb_path: action.breadcrumb.to_string(),
                    launch_command: action.launch_command.to_string(),
                    source: "ACTION".to_string(),
                    description: action.description.to_string(),
                    synonyms: action.triggers.join("|"),
                },
                score: best_score,
            });
        }
    }
    matches.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    matches
}

// ── Calculator: recursive-descent expression parser ────────────────────────
// Supports: +, -, *, /, ^, %, parentheses, unary minus
// Named functions: sqrt, abs, round, floor, ceil, sin, cos, tan, log, ln
// Special form: "N% of M" → N/100 * M
pub fn try_calc(input: &str) -> Option<f64> {
    let s = input.trim();
    // Must contain at least one digit to be a math expression
    if !s.chars().any(|c| c.is_ascii_digit()) { return None; }

    // Handle "X% of Y" shorthand
    let s = if let Some(pct_of) = try_pct_of(s) {
        return Some(pct_of);
    } else {
        s.to_string()
    };

    let tokens = tokenize(&s)?;
    let mut pos = 0usize;
    let result = parse_expr(&tokens, &mut pos)?;
    // Consume any trailing whitespace tokens
    while pos < tokens.len() {
        if tokens[pos] != Token::EOF { return None; }
        pos += 1;
    }
    if result.is_nan() || result.is_infinite() { return None; }
    Some(result)
}

fn try_pct_of(s: &str) -> Option<f64> {
    // Match "N% of M" case-insensitively
    let lower = s.to_lowercase();
    let idx = lower.find("% of ")?;
    let pct_str = s[..idx].trim();
    let rest_str = s[idx + 5..].trim();
    let pct: f64 = pct_str.parse().ok()?;
    let base: f64 = rest_str.parse().ok()?;
    Some(pct / 100.0 * base)
}

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Num(f64),
    Plus, Minus, Star, Slash, Caret, Percent,
    LParen, RParen,
    Ident(String),
    EOF,
}

fn tokenize(s: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = s.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' => { i += 1; }
            '+' => { tokens.push(Token::Plus);    i += 1; }
            '-' => { tokens.push(Token::Minus);   i += 1; }
            '*' | '×' => { tokens.push(Token::Star);  i += 1; }
            '/' | '÷' => { tokens.push(Token::Slash); i += 1; }
            '^' => { tokens.push(Token::Caret);   i += 1; }
            '%' => { tokens.push(Token::Percent); i += 1; }
            '(' => { tokens.push(Token::LParen);  i += 1; }
            ')' => { tokens.push(Token::RParen);  i += 1; }
            ',' => { i += 1; } // ignore comma separators
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') { i += 1; }
                let num_str: String = chars[start..i].iter().collect();
                let n: f64 = num_str.parse().ok()?;
                tokens.push(Token::Num(n));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') { i += 1; }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(word.to_lowercase()));
            }
            _ => return None, // unknown character → not a math expression
        }
    }
    tokens.push(Token::EOF);
    Some(tokens)
}

// expr = term (('+' | '-') term)*
fn parse_expr(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let mut left = parse_term(tokens, pos)?;
    loop {
        match tokens.get(*pos) {
            Some(Token::Plus)  => { *pos += 1; left += parse_term(tokens, pos)?; }
            Some(Token::Minus) => { *pos += 1; left -= parse_term(tokens, pos)?; }
            _ => break,
        }
    }
    Some(left)
}

// term = power (('*' | '/' | '%') power)*
fn parse_term(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let mut left = parse_power(tokens, pos)?;
    loop {
        match tokens.get(*pos) {
            Some(Token::Star)    => { *pos += 1; left *= parse_power(tokens, pos)?; }
            Some(Token::Slash)   => { *pos += 1; let r = parse_power(tokens, pos)?; if r == 0.0 { return None; } left /= r; }
            Some(Token::Percent) => {
                // Check if next token is 'of' (handled earlier) or treat as modulo
                *pos += 1;
                match tokens.get(*pos) {
                    Some(Token::Ident(w)) if w == "of" => {
                        *pos += 1;
                        let base = parse_power(tokens, pos)?;
                        left = left / 100.0 * base;
                    }
                    _ => {
                        // treat as percentage of the next value if present, else /100
                        left = left / 100.0;
                    }
                }
            }
            _ => break,
        }
    }
    Some(left)
}

// power = unary ('^' power)?  (right-associative)
fn parse_power(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    let base = parse_unary(tokens, pos)?;
    if matches!(tokens.get(*pos), Some(Token::Caret)) {
        *pos += 1;
        let exp = parse_power(tokens, pos)?;
        Some(base.powf(exp))
    } else {
        Some(base)
    }
}

// unary = '-' unary | primary
fn parse_unary(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    if matches!(tokens.get(*pos), Some(Token::Minus)) {
        *pos += 1;
        Some(-parse_unary(tokens, pos)?)
    } else {
        parse_primary(tokens, pos)
    }
}

// primary = number | ident '(' expr ')' | '(' expr ')'
fn parse_primary(tokens: &[Token], pos: &mut usize) -> Option<f64> {
    match tokens.get(*pos)?.clone() {
        Token::Num(n) => { *pos += 1; Some(n) }
        Token::LParen => {
            *pos += 1;
            let val = parse_expr(tokens, pos)?;
            if tokens.get(*pos) == Some(&Token::RParen) { *pos += 1; }
            Some(val)
        }
        Token::Ident(name) => {
            *pos += 1;
            // Named functions expect a parenthesised argument
            if tokens.get(*pos) == Some(&Token::LParen) {
                *pos += 1;
                let arg = parse_expr(tokens, pos)?;
                if tokens.get(*pos) == Some(&Token::RParen) { *pos += 1; }
                match name.as_str() {
                    "sqrt" => Some(arg.sqrt()),
                    "abs"  => Some(arg.abs()),
                    "round"=> Some(arg.round()),
                    "floor"=> Some(arg.floor()),
                    "ceil" => Some(arg.ceil()),
                    "sin"  => Some(arg.to_radians().sin()),
                    "cos"  => Some(arg.to_radians().cos()),
                    "tan"  => Some(arg.to_radians().tan()),
                    "log"  => Some(arg.log10()),
                    "ln"   => Some(arg.ln()),
                    _ => None,
                }
            } else {
                // Named constants
                match name.as_str() {
                    "pi" | "π" => Some(std::f64::consts::PI),
                    "e"        => Some(std::f64::consts::E),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}
