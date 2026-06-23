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
        
        // Initialize clipboard_history table
        let conn = Connection::open(&engine.db_path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // Add is_image column if it doesn't exist
        let _ = conn.execute("ALTER TABLE clipboard_history ADD COLUMN is_image INTEGER DEFAULT 0;", []);
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT UNIQUE,
                timestamp INTEGER NOT NULL,
                source_app TEXT NOT NULL,
                is_image INTEGER DEFAULT 0
            );",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS timeline_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                duration INTEGER NOT NULL,
                app_name TEXT NOT NULL,
                window_title TEXT NOT NULL
            );",
            [],
        )?;
        let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_timeline_timestamp ON timeline_events(timestamp);", []);

        let _ = engine.search("settings", 1);
        Ok(engine)
    }

fn get_path_score_modifier(full_path: &str) -> f32 {
    let path_lower = full_path.to_lowercase();
    
    // Penalize system/tool/hidden directories
    if path_lower.contains("\\node_modules\\") ||
       path_lower.contains("\\target\\") ||
       path_lower.contains("\\.git\\") ||
       path_lower.contains("\\appdata\\") ||
       path_lower.contains("\\.cargo\\") ||
       path_lower.contains("\\.rustup\\") ||
       path_lower.contains("\\.npm\\") ||
       path_lower.contains("\\.antigravity") ||
       path_lower.contains("\\.cursor\\") ||
       path_lower.contains("\\venv\\") ||
       path_lower.contains("\\.venv\\") ||
       path_lower.contains("\\__macosx\\") ||
       path_lower.contains("\\bin\\") ||
       path_lower.contains("\\obj\\") ||
       path_lower.contains("\\temp\\") ||
       path_lower.contains("\\tmp\\") {
        return -2.0; // Excluded from results
    }

    // Boost user's active/primary directories
    if path_lower.contains("\\desktop\\") ||
       path_lower.contains("\\documents\\") ||
       path_lower.contains("\\downloads\\") {
        return 1.5;
    }

    0.0
}

    fn query_everything(&self, query: &str, only_code: bool, max_results: usize) -> Option<Vec<SearchResult>> {
        use everything_ipc::wm::{EverythingClient, RequestFlags};
        
        let client = EverythingClient::new().ok()?;
        
        let code_exts = [
            "rs", "py", "js", "ts", "json", "html", "css",
            "c", "cpp", "h", "hpp", "cs", "go", "java", "kt", "sh", "bat",
            "ps1", "yaml", "yml", "toml", "ini", "sql", "xml"
        ];
        
        let full_query = if only_code {
            format!("{} ext:{}", query, code_exts.join(";"))
        } else {
            query.to_string()
        };
        
        let list = client
            .query_wait(&full_query)
            .request_flags(RequestFlags::FileName | RequestFlags::Path | RequestFlags::Size)
            .max_results(max_results as u32)
            .call()
            .ok()?;
            
        let mut results = Vec::new();
        for item in list.iter() {
            let filename = item.get_string(RequestFlags::FileName).unwrap_or_else(|| "Unknown".to_string());
            let path = item.get_string(RequestFlags::Path).unwrap_or_else(|| "Unknown".to_string());
            let size = item.get_size(RequestFlags::Size).unwrap_or(0);
            
            // Fix: Path::new("C:").join("file") gives "C:file" not "C:\file".
            // Always ensure a backslash separator between the parent path and filename.
            let full_path = if path.ends_with('\\') || path.ends_with('/') {
                format!("{}{}", path, filename)
            } else {
                format!("{}\\{}", path, filename)
            };
            
            let path_modifier = Self::get_path_score_modifier(&full_path);
            if path_modifier < -1.0 {
                continue; // Skip system/hidden/ignored files
            }

            let is_dir = std::path::Path::new(&full_path).is_dir();
            
            let ext = if is_dir {
                "folder".to_string()
            } else {
                std::path::Path::new(&filename)
                    .extension()
                    .map(|e| e.to_string_lossy().to_string().to_lowercase())
                    .unwrap_or_default()
            };
                
            let source = if is_dir {
                "FOLDER"
            } else if only_code || code_exts.contains(&ext.as_str()) {
                "CODE"
            } else {
                "FILE"
            };
            
            let q_lower = query.to_lowercase();
            let name_lower = filename.to_lowercase();
            let name_no_ext = if let Some(dot) = name_lower.rfind('.') { &name_lower[..dot] } else { &name_lower };
            
            let mut score = if name_lower == q_lower || name_no_ext == q_lower {
                3.0  // exact match
            } else if name_lower.starts_with(&q_lower) || name_no_ext.starts_with(&q_lower) {
                2.5  // prefix match
            } else if name_lower.contains(&q_lower) {
                1.8  // substring match
            } else {
                1.0  // fallback (Everything already knows it's relevant)
            };
            score += path_modifier;
            
            let breadcrumb = if source == "FOLDER" {
                format!("Folder > {}", full_path)
            } else {
                format!("{} > {}", if source == "CODE" { "Code" } else { "File" }, full_path)
            };
            
            let description = if source == "FOLDER" {
                "Local folder".to_string()
            } else {
                let size_str = if size < 1024 {
                    format!("{} B", size)
                } else if size < 1024 * 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else if size < 1024 * 1024 * 1024 {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
                };
                format!("Local {} file ({})", ext.to_uppercase(), size_str)
            };
            
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: format!("{}.{}", source.to_lowercase(), full_path),
                    control_name: filename.clone(),
                    breadcrumb_path: breadcrumb,
                    launch_command: full_path.clone(),
                    source: source.to_string(),
                    description,
                    synonyms: filename.to_lowercase(),
                },
                score,
            });
        }
        
        Some(results)
    }



    // with_fts_content: if false (general search), skips content-only matches — only filename hits shown.
    //                   if true  (file:/code: prefix), full content search is included.
    fn search_files_generic(&self, query: &str, only_code: bool, max_results: usize, with_fts_content: bool) -> Vec<SearchResult> {
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(_) => return Vec::new(),
        };

        let q_lower = query.to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();
        if q_words.is_empty() { return Vec::new(); }

        let code_exts = [
            "rs", "py", "js", "ts", "json", "html", "css",
            "c", "cpp", "h", "hpp", "cs", "go", "java", "kt", "sh", "bat",
            "ps1", "yaml", "yml", "toml", "ini", "sql", "xml"
        ];

        // Helper: score a filename against query
        let score_name = |name: &str| -> f32 {
            let name_lower = name.to_lowercase();
            let name_no_ext = name_lower.rfind('.').map(|d| &name_lower[..d]).unwrap_or(&name_lower);
            if name_lower == q_lower || name_no_ext == q_lower { return 3.0; }
            if name_lower.starts_with(&q_lower) || name_no_ext.starts_with(&q_lower) { return 2.5; }
            if name_lower.contains(&q_lower) { return 1.8; }
            let words: Vec<&str> = name_no_ext.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
            let matched = q_words.iter().filter(|w| words.contains(w)).count();
            if matched > 0 { 0.8 + 0.4 * (matched as f32 / q_words.len() as f32) } else { 0.0 }
        };

        let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut results = Vec::new();

        // ── 1. Metadata search via Everything (if running) ─────────────────
        if let Some(ev_results) = self.query_everything(query, only_code, max_results) {
            for r in ev_results {
                seen_paths.insert(r.entry.launch_command.clone());
                results.push(r);
            }
        }

        // ── 2. SQLite name LIKE fallback (when Everything unavailable) ─────
        if seen_paths.is_empty() {
            let name_query = format!("%{}%", q_lower);
            let query_str = if only_code {
                let placeholders: Vec<String> = code_exts.iter().map(|_| "?".to_string()).collect();
                format!(
                    "SELECT path, name, extension FROM files WHERE name LIKE ? AND extension IN ({}) LIMIT ?",
                    placeholders.join(",")
                )
            } else {
                "SELECT path, name, extension FROM files WHERE name LIKE ? LIMIT ?".to_string()
            };
            if let Ok(mut stmt) = conn.prepare(&query_str) {
                let mut params_vec: Vec<rusqlite::types::Value> = vec![
                    rusqlite::types::Value::Text(name_query),
                ];
                if only_code {
                    for ext in &code_exts { params_vec.push(rusqlite::types::Value::Text(ext.to_string())); }
                }
                params_vec.push(rusqlite::types::Value::Integer(max_results as i64));
                let params_ref = rusqlite::params_from_iter(params_vec.iter());
                if let Ok(rows) = stmt.query_map(params_ref, |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                }) {
                    for row in rows.filter_map(|r| r.ok()) {
                        let (path, name, ext) = row;
                        let path_modifier = Self::get_path_score_modifier(&path);
                        if path_modifier < -1.0 { continue; }
                        let mut score = score_name(&name);
                        if score <= 0.0 { continue; }
                        score += path_modifier;
                        let source = if ext == "folder" { "FOLDER" }
                            else if only_code || code_exts.contains(&ext.as_str()) { "CODE" }
                            else { "FILE" };
                        let breadcrumb = if source == "FOLDER" { format!("Folder > {}", path) }
                            else { format!("{} > {}", if source == "CODE" { "Code" } else { "File" }, path) };
                        let description = if source == "FOLDER" { "Local folder".to_string() }
                            else { format!("Local {} file", ext.to_uppercase()) };
                        seen_paths.insert(path.clone());
                        results.push(SearchResult {
                            entry: CatalogEntry {
                                id: format!("{}.{}", source.to_lowercase(), path),
                                control_name: name.clone(),
                                breadcrumb_path: breadcrumb,
                                launch_command: path,
                                source: source.to_string(),
                                description,
                                synonyms: name.to_lowercase(),
                            },
                            score,
                        });
                    }
                }
            }
        }

        // ── 3. FTS5 content search — only in dedicated prefix searches (file:/code:)
        // In general search (with_fts_content=false), we only boost already-found filename matches.
        if !with_fts_content {
            // Still boost score of metadata hits that also match content
            let clean_fts_query = q_words.join(" ");
            let fts_check = format!(
                "SELECT f.path FROM files f JOIN files_fts fts ON f.path = fts.path WHERE files_fts MATCH ? LIMIT 50"
            );
            if let Ok(mut stmt_fts) = conn.prepare(&fts_check) {
                if let Ok(rows) = stmt_fts.query_map([&clean_fts_query], |row| row.get::<_, String>(0)) {
                    for row in rows.filter_map(|r| r.ok()) {
                        if let Some(existing) = results.iter_mut().find(|r| r.entry.launch_command == row) {
                            existing.score += 0.5; // content match bonus on top of filename match
                        }
                    }
                }
            }
            return results;
        }
        let clean_fts_query = q_words.join(" ");
        let fts_query_str = if only_code {
            let placeholders: Vec<String> = code_exts.iter().map(|_| "?".to_string()).collect();
            format!(
                "SELECT f.path, f.name, f.extension, snippet(files_fts, 1, '', '', '...', 15) \
                 FROM files f \
                 JOIN files_fts fts ON f.path = fts.path \
                 WHERE files_fts MATCH ? AND f.extension IN ({}) LIMIT ?",
                placeholders.join(",")
            )
        } else {
            "SELECT f.path, f.name, f.extension, snippet(files_fts, 1, '', '', '...', 15) \
             FROM files f \
             JOIN files_fts fts ON f.path = fts.path \
             WHERE files_fts MATCH ? LIMIT ?".to_string()
        };
        if let Ok(mut stmt_fts) = conn.prepare(&fts_query_str) {
            let mut fts_params_vec: Vec<rusqlite::types::Value> = vec![
                rusqlite::types::Value::Text(clean_fts_query),
            ];
            if only_code {
                for ext in &code_exts { fts_params_vec.push(rusqlite::types::Value::Text(ext.to_string())); }
            }
            fts_params_vec.push(rusqlite::types::Value::Integer(max_results as i64));
            let fts_params_ref = rusqlite::params_from_iter(fts_params_vec.iter());
            if let Ok(rows) = stmt_fts.query_map(fts_params_ref, |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            }) {
                for row in rows.filter_map(|r| r.ok()) {
                    let (path, name, ext, snippet_raw) = row;
                    // Already shown via metadata — upgrade score if name also matches
                    if seen_paths.contains(&path) {
                        // Boost existing entry's score for content match
                        if let Some(existing) = results.iter_mut().find(|r| r.entry.launch_command == path) {
                            existing.score += 0.5; // content match bonus
                        }
                        continue;
                    }
                    let path_modifier = Self::get_path_score_modifier(&path);
                    if path_modifier < -1.0 { continue; }
                    let snippet = snippet_raw
                        .replace('\n', " ").replace('\r', " ").replace('\t', " ")
                        .split_whitespace().collect::<Vec<&str>>().join(" ");
                    let source = if only_code || code_exts.contains(&ext.as_str()) { "CODE" } else { "FILE" };
                    // Score: content-only matches intentionally score lower than filename matches.
                    // Base 0.8 + up to 1.5 name bonus keeps content matches below pure filename hits (score 1.8+).
                    let name_bonus = score_name(&name).min(1.5);
                    let mut score = 0.8 + name_bonus + path_modifier;
                    if score <= 0.0 { continue; }
                    let parent_dir = std::path::Path::new(&path)
                        .parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("");
                    let breadcrumb = if parent_dir.is_empty() {
                        format!("{} | {}", if source == "CODE" { "Code" } else { "File" }, snippet)
                    } else {
                        format!("{} > {} | {}", if source == "CODE" { "Code" } else { "File" }, parent_dir, snippet)
                    };
                    seen_paths.insert(path.clone());
                    results.push(SearchResult {
                        entry: CatalogEntry {
                            id: format!("{}.{}", source.to_lowercase(), path),
                            control_name: name.clone(),
                            breadcrumb_path: breadcrumb,
                            launch_command: path,
                            source: source.to_string(),
                            description: format!("Local {} file (content match)", ext.to_uppercase()),
                            synonyms: name.to_lowercase(),
                        },
                        score,
                    });
                }
            }
        }

        results
    }

    fn search_local_files(&self, query: &str) -> Vec<SearchResult> {
        // General search: filename matches only (no FTS content-only results)
        self.search_files_generic(query, false, 15, false)
    }

    pub fn db_path(&self) -> std::path::PathBuf {
        self.db_path.clone()
    }

    pub fn search_timeline(&self, start_time: i64, end_time: i64, keyword: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(_) => return results,
        };

        let select_query = if keyword.is_empty() {
            "SELECT timestamp, duration, app_name, window_title FROM timeline_events \
             WHERE timestamp >= ? AND timestamp <= ? \
             ORDER BY timestamp DESC LIMIT 50".to_string()
        } else {
            "SELECT timestamp, duration, app_name, window_title FROM timeline_events \
             WHERE timestamp >= ? AND timestamp <= ? AND (window_title LIKE ? OR app_name LIKE ?) \
             ORDER BY timestamp DESC LIMIT 50".to_string()
        };

        let mut stmt = match conn.prepare(&select_query) {
            Ok(s) => s,
            Err(_) => return results,
        };

        let rows: Vec<(i64, i64, String, String)> = if keyword.is_empty() {
            stmt.query_map([start_time, end_time], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default()
        } else {
            let like_pattern = format!("%{}%", keyword.to_lowercase());
            stmt.query_map(rusqlite::params![start_time, end_time, like_pattern, like_pattern], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default()
        };

        for (timestamp, duration, app_name, window_title) in rows {
            let time_str = format_timestamp_local(timestamp);
            let dur_str = if duration < 60 {
                format!("{}s", duration)
            } else {
                format!("{}m {}s", duration / 60, duration % 60)
            };

            let launch_command = if window_title.contains(":\\") || window_title.contains(":/") || window_title.contains("http://") || window_title.contains("https://") {
                extract_path_or_url(&window_title).unwrap_or_else(|| app_name.clone())
            } else {
                app_name.clone()
            };

            let display_app = std::path::Path::new(&app_name)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| app_name.clone());

            results.push(SearchResult {
                entry: CatalogEntry {
                    id: format!("timeline.{}", timestamp),
                    control_name: format!("{} ({})", window_title, display_app),
                    breadcrumb_path: format!("Timeline > {} ({})", time_str, dur_str),
                    launch_command,
                    source: "MEMORY".to_string(),
                    description: format!("Active app: {} at {}", display_app, time_str),
                    synonyms: format!("{} {} timeline memory", display_app.to_lowercase(), window_title.to_lowercase()),
                },
                score: 4.0,
            });
        }

        results
    }

    pub fn search_project(&self, project_keyword: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(_) => return results,
        };

        let kw_lower = project_keyword.to_lowercase();
        let kw_pattern = format!("%{}%", kw_lower);

        // 1. Find the Git repositories matching the keyword
        let mut stmt = match conn.prepare("SELECT name, path, head_branch FROM git_repos WHERE name LIKE ? OR path LIKE ?") {
            Ok(s) => s,
            Err(_) => return results,
        };
        let repos: Vec<(String, String, String)> = stmt.query_map([&kw_pattern, &kw_pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default();

        let mut project_paths = Vec::new();
        for (name, path, head) in &repos {
            project_paths.push(path.clone());
            
            // Add git repo itself as a result
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: format!("project.repo.{}", name),
                    control_name: format!("📁 Code Directory: {} (Git Repo)", name),
                    breadcrumb_path: format!("Project > Code > {} [branch: {}]", name, head),
                    launch_command: path.clone(),
                    source: "PROJECT".to_string(),
                    description: format!("Project repository located at {}", path),
                    synonyms: format!("{} project repo git code", name.to_lowercase()),
                },
                score: 9.5,
            });
        }

        // 2. Query matching recent files or folders
        let mut stmt = match conn.prepare("SELECT path, size, is_dir FROM files WHERE path LIKE ? LIMIT 20") {
            Ok(s) => s,
            Err(_) => return results,
        };
        let files: Vec<(String, i64, i32)> = stmt.query_map([&kw_pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i32>(2)?))
        }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default();

        for (path, size, is_dir) in files {
            let name = std::path::Path::new(&path).file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            
            let is_code = path.contains("\\target\\") || path.contains("\\node_modules\\");
            if is_code { continue; } // skip build targets

            let control_name = if is_dir == 1 {
                format!("📁 Folder: {}", name)
            } else {
                format!("📄 Document: {}", name)
            };

            results.push(SearchResult {
                entry: CatalogEntry {
                    id: format!("project.file.{}", path),
                    control_name,
                    breadcrumb_path: format!("Project > Files > {}", path),
                    launch_command: path.clone(),
                    source: "PROJECT".to_string(),
                    description: format!("Related file ({} bytes)", size),
                    synonyms: format!("{} project file", name.to_lowercase()),
                },
                score: 9.0,
            });
        }

        // 3. Query browser history / bookmarks matching the keyword
        let mut stmt = match conn.prepare("SELECT url, title, source FROM browser_items WHERE title LIKE ? OR url LIKE ? LIMIT 20") {
            Ok(s) => s,
            Err(_) => return results,
        };
        let urls: Vec<(String, String, String)> = stmt.query_map([&kw_pattern, &kw_pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default();

        for (url, title, src) in urls {
            let is_figma = url.contains("figma.com");
            let control_name = if is_figma {
                format!("🎨 Figma Design: {}", title)
            } else {
                format!("🔗 Link: {}", title)
            };

            results.push(SearchResult {
                entry: CatalogEntry {
                    id: format!("project.url.{}", url),
                    control_name,
                    breadcrumb_path: format!("Project > {} > {}", src.to_uppercase(), url),
                    launch_command: url,
                    source: "PROJECT".to_string(),
                    description: "Related browser link".to_string(),
                    synonyms: format!("{} project url link figma design", title.to_lowercase()),
                },
                score: 8.8,
            });
        }

        // 4. Query recent Git commits matching the keyword
        let mut stmt = match conn.prepare("SELECT repo_path, hash, message, timestamp FROM git_commits WHERE message LIKE ? ORDER BY timestamp DESC LIMIT 5") {
            Ok(s) => s,
            Err(_) => return results,
        };
        let commits: Vec<(String, String, String, i64)> = stmt.query_map([&kw_pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?))
        }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default();

        for (repo_path, hash, message, ts) in commits {
            let repo_name = std::path::Path::new(&repo_path).file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| "repo".to_string());

            results.push(SearchResult {
                entry: CatalogEntry {
                    id: format!("project.commit.{}", hash),
                    control_name: format!("💻 Commit: {} (in {})", message, repo_name),
                    breadcrumb_path: format!("Project > Commit > {} [hash: {}]", repo_name, &hash[..7.min(hash.len())]),
                    launch_command: repo_path.clone(),
                    source: "PROJECT".to_string(),
                    description: format!("Git commit at {}", format_timestamp_local(ts)),
                    synonyms: format!("{} project commit git", message.to_lowercase()),
                },
                score: 8.5,
            });
        }

        // 5. Query clipboard items matching the keyword
        let mut stmt = match conn.prepare("SELECT content, source_app, is_image FROM clipboard_history WHERE content LIKE ? LIMIT 5") {
            Ok(s) => s,
            Err(_) => return results,
        };
        let clips: Vec<(String, String, i32)> = stmt.query_map([&kw_pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?))
        }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default();

        for (content, app, is_image) in clips {
            let display_app = std::path::Path::new(&app)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| app.clone());

            if is_image == 1 {
                let filename = std::path::Path::new(&content).file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "image.bmp".to_string());
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("project.clip.{}", content),
                        control_name: format!("🎨 Screenshot (Copied from {})", display_app),
                        breadcrumb_path: format!("Project > Clipboard > Screenshot"),
                        launch_command: format!("copy_image:{}", content),
                        source: "PROJECT".to_string(),
                        description: format!("Project screenshot: {}", filename),
                        synonyms: "project image figma figma_screenshot clipboard".to_string(),
                    },
                    score: 8.0,
                });
            } else {
                let mut preview = content.replace("\r\n", " ").replace('\n', " ");
                if preview.len() > 100 {
                    preview.truncate(97);
                    preview.push_str("...");
                }
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("project.clip.{}", content),
                        control_name: format!("📋 Clipboard: {}", preview),
                        breadcrumb_path: format!("Project > Clipboard > Text"),
                        launch_command: format!("copy:{}", content),
                        source: "PROJECT".to_string(),
                        description: format!("Text copied from {}", display_app),
                        synonyms: format!("{} project clip", content.to_lowercase()),
                    },
                    score: 7.8,
                });
            }
        }

        // 6. Temporal proximity connections (Worked on simultaneously)
        let mut stmt = match conn.prepare("SELECT timestamp FROM timeline_events WHERE window_title LIKE ? OR app_name LIKE ?") {
            Ok(s) => s,
            Err(_) => return results,
        };
        let activity_timestamps: Vec<i64> = stmt.query_map([&kw_pattern, &kw_pattern], |row| {
            row.get::<_, i64>(0)
        }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default();

        if !activity_timestamps.is_empty() {
            let mut min_ts = i64::MAX;
            let mut max_ts = i64::MIN;
            for ts in activity_timestamps {
                if ts < min_ts { min_ts = ts; }
                if ts > max_ts { max_ts = ts; }
            }

            if max_ts >= min_ts {
                let mut stmt = match conn.prepare(
                    "SELECT url, title, source FROM browser_items \
                     WHERE ( \
                         CASE \
                             WHEN last_visit_time > 10000000000000000 THEN (last_visit_time / 1000000) - 11644473600 \
                             WHEN last_visit_time > 10000000000 THEN last_visit_time / 1000000 \
                             ELSE last_visit_time \
                         END \
                     ) >= ? - 600 AND ( \
                         CASE \
                             WHEN last_visit_time > 10000000000000000 THEN (last_visit_time / 1000000) - 11644473600 \
                             WHEN last_visit_time > 10000000000 THEN last_visit_time / 1000000 \
                             ELSE last_visit_time \
                         END \
                     ) <= ? + 600 \
                     AND url NOT IN (SELECT url FROM browser_items WHERE title LIKE ? OR url LIKE ?) \
                     LIMIT 5"
                ) {
                    Ok(s) => s,
                    Err(_) => return results,
                };
                let temp_urls: Vec<(String, String, String)> = stmt.query_map(
                    rusqlite::params![min_ts, max_ts, kw_pattern, kw_pattern],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                ).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default();

                for (url, title, src) in temp_urls {
                    results.push(SearchResult {
                        entry: CatalogEntry {
                            id: format!("project.temp_url.{}", url),
                            control_name: format!("🔗 Context Link: {}", title),
                            breadcrumb_path: format!("Project > Context > Browser > {}", url),
                            launch_command: url,
                            source: "PROJECT".to_string(),
                            description: "Opened during project work window".to_string(),
                            synonyms: format!("{} project context url link", title.to_lowercase()),
                        },
                        score: 7.5,
                    });
                }
            }
        }

        results
    }

    pub fn search_clipboard_history(&self, query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(_) => return results,
        };

        let q_lower = query.to_lowercase();
        
        let select_query = if q_lower.is_empty() {
            "SELECT content, source_app, timestamp, is_image FROM clipboard_history ORDER BY timestamp DESC LIMIT 50".to_string()
        } else {
            "SELECT content, source_app, timestamp, is_image FROM clipboard_history WHERE content LIKE ? OR source_app LIKE ? ORDER BY timestamp DESC LIMIT 50".to_string()
        };

        let mut stmt = match conn.prepare(&select_query) {
            Ok(s) => s,
            Err(_) => return results,
        };

        let rows: Vec<(String, String, i64, i32)> = if q_lower.is_empty() {
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i32>(3)?,
                ))
            }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default()
        } else {
            let like_pattern = format!("%{}%", q_lower);
            stmt.query_map([&like_pattern, &like_pattern], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i32>(3)?,
                ))
            }).map(|m| m.filter_map(|r| r.ok()).collect()).unwrap_or_default()
        };

        for (content, source_app, timestamp, is_image) in rows {
            let display_app = std::path::Path::new(&source_app)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| source_app.clone());

            if is_image == 1 {
                let filename = std::path::Path::new(&content)
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "image.bmp".to_string());

                let dims = get_bmp_dimensions(&content);
                let control_name = if let Some((w, h)) = dims {
                    format!("[Image] {}x{} (Copied from {})", w, h, display_app)
                } else {
                    format!("[Image] Copied from {}", display_app)
                };

                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("clip.{}", timestamp),
                        control_name,
                        breadcrumb_path: format!("Clipboard > {}", display_app),
                        launch_command: format!("copy_image:{}", content),
                        source: "CLIPBOARD".to_string(),
                        description: format!("Image history (Saved as {})", filename),
                        synonyms: format!("image {} clipboard copy", display_app.to_lowercase()),
                    },
                    score: 3.0,
                });
            } else {
                let mut desc = content.replace("\r\n", " ").replace('\n', " ");
                if desc.len() > 100 {
                    desc.truncate(97);
                    desc.push_str("...");
                }
                
                let display_name = content.replace("\r\n", " ").replace('\n', " ").replace('\t', " ");
                let display_name = if display_name.len() > 200 {
                    format!("{}...", &display_name[..197])
                } else {
                    display_name
                };

                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("clip.{}", timestamp),
                        control_name: display_name,
                        breadcrumb_path: format!("Clipboard > {}", display_app),
                        launch_command: format!("copy:{}", content),
                        source: "CLIPBOARD".to_string(),
                        description: desc,
                        synonyms: content.to_lowercase(),
                    },
                    score: 3.0,
                });
            }
        }

        results
    }

    pub fn search_files_only(&self, query: &str) -> Vec<SearchResult> {
        // Dedicated file: prefix — include full content search
        self.search_files_generic(query, false, 50, true)
    }

    pub fn search_code_only(&self, query: &str) -> Vec<SearchResult> {
        // Dedicated code: prefix — include full content search
        self.search_files_generic(query, true, 50, true)
    }

    pub fn search_bookmarks_only(&self, sub_query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(_) => return results,
        };

        let q_lower = sub_query.trim().to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();

        let rows = if q_lower.is_empty() {
            let mut stmt = match conn.prepare(
                "SELECT url, title, source, visit_count FROM browser_items 
                 WHERE source LIKE '%bookmark%' 
                 ORDER BY visit_count DESC LIMIT 100"
            ) {
                Ok(s) => s,
                Err(_) => return results,
            };
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            });
            match mapped {
                Ok(r) => r.collect::<Vec<_>>(),
                Err(_) => return results,
            }
        } else {
            let name_query = format!("%{}%", q_lower);
            let mut stmt = match conn.prepare(
                "SELECT url, title, source, visit_count FROM browser_items 
                 WHERE source LIKE '%bookmark%' AND (title LIKE ?1 OR url LIKE ?1)
                 ORDER BY visit_count DESC LIMIT 100"
            ) {
                Ok(s) => s,
                Err(_) => return results,
            };
            let mapped = stmt.query_map([&name_query], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            });
            match mapped {
                Ok(r) => r.collect::<Vec<_>>(),
                Err(_) => return results,
            }
        };

        for row in rows.into_iter().filter_map(|r| r.ok()) {
            let (url, title, source, visit_count) = row;
            let title_lower = title.to_lowercase();
            let url_lower = url.to_lowercase();

            let mut score = 1.0f32;
            if !q_lower.is_empty() {
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
                    } else {
                        score = 0.0;
                    }
                }
            } else {
                score = 1.0 + (visit_count as f32).min(100.0) / 100.0;
            }

            if score > 0.0 {
                let browser_name = if source.contains("chrome") {
                    "Chrome"
                } else if source.contains("edge") {
                    "Edge"
                } else if source.contains("brave") {
                    "Brave"
                } else if source.contains("firefox") {
                    "Firefox"
                } else {
                    "Browser"
                };
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("browser.{}", url),
                        control_name: title.clone(),
                        breadcrumb_path: format!("Browser > {}", url),
                        launch_command: url.clone(),
                        source: "BOOKMARK".to_string(),
                        description: format!("Bookmark from {}", browser_name),
                        synonyms: title.to_lowercase(),
                    },
                    score,
                });
            }
        }

        results.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn search_history_only(&self, sub_query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(_) => return results,
        };

        let q_lower = sub_query.trim().to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();

        let rows = if q_lower.is_empty() {
            let mut stmt = match conn.prepare(
                "SELECT url, title, source, last_visit_time FROM browser_items 
                 WHERE source LIKE '%history%' 
                 ORDER BY last_visit_time DESC LIMIT 100"
            ) {
                Ok(s) => s,
                Err(_) => return results,
            };
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            });
            match mapped {
                Ok(r) => r.collect::<Vec<_>>(),
                Err(_) => return results,
            }
        } else {
            let name_query = format!("%{}%", q_lower);
            let mut stmt = match conn.prepare(
                "SELECT url, title, source, last_visit_time FROM browser_items 
                 WHERE source LIKE '%history%' AND (title LIKE ?1 OR url LIKE ?1)
                 ORDER BY last_visit_time DESC LIMIT 100"
            ) {
                Ok(s) => s,
                Err(_) => return results,
            };
            let mapped = stmt.query_map([&name_query], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            });
            match mapped {
                Ok(r) => r.collect::<Vec<_>>(),
                Err(_) => return results,
            }
        };

        for row in rows.into_iter().filter_map(|r| r.ok()) {
            let (url, title, source, _last_visit_time) = row;
            let title_lower = title.to_lowercase();
            let url_lower = url.to_lowercase();

            let mut score = 1.0f32;
            if !q_lower.is_empty() {
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
                    } else {
                        score = 0.0;
                    }
                }
            } else {
                score = 1.0;
            }

            if score > 0.0 {
                let browser_name = if source.contains("chrome") {
                    "Chrome"
                } else if source.contains("edge") {
                    "Edge"
                } else if source.contains("brave") {
                    "Brave"
                } else if source.contains("firefox") {
                    "Firefox"
                } else {
                    "Browser"
                };
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("browser.{}", url),
                        control_name: title.clone(),
                        breadcrumb_path: format!("Browser > {}", url),
                        launch_command: url.clone(),
                        source: "HISTORY".to_string(),
                        description: format!("History from {}", browser_name),
                        synonyms: title.to_lowercase(),
                    },
                    score,
                });
            }
        }

        if !q_lower.is_empty() {
            results.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        }
        results
    }

    pub fn search_commits_only(&self, sub_query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(_) => return results,
        };

        let q_lower = sub_query.trim().to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();

        let rows = if q_lower.is_empty() {
            let mut stmt = match conn.prepare(
                "SELECT c.hash, c.author, c.date, c.message, r.name 
                 FROM git_commits c
                 JOIN git_repos r ON c.repo_id = r.id
                 ORDER BY c.date DESC LIMIT 100"
            ) {
                Ok(s) => s,
                Err(_) => return results,
            };
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            });
            match mapped {
                Ok(r) => r.collect::<Vec<_>>(),
                Err(_) => return results,
            }
        } else {
            let name_query = format!("%{}%", q_lower);
            let mut stmt = match conn.prepare(
                "SELECT c.hash, c.author, c.date, c.message, r.name 
                 FROM git_commits c
                 JOIN git_repos r ON c.repo_id = r.id
                 WHERE c.message LIKE ?1 OR c.author LIKE ?1 OR c.hash LIKE ?1
                 ORDER BY c.date DESC LIMIT 100"
            ) {
                Ok(s) => s,
                Err(_) => return results,
            };
            let mapped = stmt.query_map([&name_query], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            });
            match mapped {
                Ok(r) => r.collect::<Vec<_>>(),
                Err(_) => return results,
            }
        };

        let format_unix_time = |timestamp: i64| -> String {
            let days = timestamp / 86400;
            let years = 1970 + days / 365;
            let month = 1 + (days % 365) / 30;
            let day = 1 + (days % 365) % 30;
            format!("{:04}-{:02}-{:02}", years, month, day)
        };

        for row in rows.into_iter().filter_map(|r| r.ok()) {
            let (hash, author, date, message, repo_name) = row;
            let msg_lower = message.to_lowercase();
            let auth_lower = author.to_lowercase();
            
            let mut score = 1.0f32;
            if !q_lower.is_empty() {
                if msg_lower == q_lower || hash.to_lowercase() == q_lower {
                    score = 2.0;
                } else if msg_lower.starts_with(&q_lower) || hash.to_lowercase().starts_with(&q_lower) {
                    score = 1.6;
                } else if msg_lower.contains(&q_lower) || auth_lower.contains(&q_lower) {
                    score = 1.2;
                } else {
                    let matched = q_words.iter().filter(|w| msg_lower.contains(*w) || auth_lower.contains(*w)).count();
                    if matched > 0 {
                        score = 0.5 + 0.5 * (matched as f32 / q_words.len() as f32);
                    } else {
                        score = 0.0;
                    }
                }
            }

            if score > 0.0 {
                let date_str = format_unix_time(date);
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("git.commit.{}", hash),
                        control_name: format!("[{}] {}", repo_name, message),
                        breadcrumb_path: format!("Git > Commit > {} by {}", hash[..7.min(hash.len())].to_string(), author),
                        launch_command: format!("copy:{}", hash),
                        source: "COMMIT".to_string(),
                        description: format!("Commit on {} - {}", date_str, hash),
                        synonyms: format!("{} {} {}", message, author, hash),
                    },
                    score,
                });
            }
        }

        results.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn search_todos_only(&self, sub_query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                c
            }
            Err(_) => return results,
        };

        let q_lower = sub_query.trim().to_lowercase();
        let q_words: Vec<&str> = q_lower.split_whitespace().collect();

        let rows = if q_lower.is_empty() {
            let mut stmt = match conn.prepare(
                "SELECT t.file_path, t.line_number, t.todo_text, r.name 
                 FROM git_todos t
                 JOIN git_repos r ON t.repo_id = r.id
                 ORDER BY t.id DESC LIMIT 100"
            ) {
                Ok(s) => s,
                Err(_) => return results,
            };
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            });
            match mapped {
                Ok(r) => r.collect::<Vec<_>>(),
                Err(_) => return results,
            }
        } else {
            let name_query = format!("%{}%", q_lower);
            let mut stmt = match conn.prepare(
                "SELECT t.file_path, t.line_number, t.todo_text, r.name 
                 FROM git_todos t
                 JOIN git_repos r ON t.repo_id = r.id
                 WHERE t.todo_text LIKE ?1 OR t.file_path LIKE ?1
                 ORDER BY t.id DESC LIMIT 100"
            ) {
                Ok(s) => s,
                Err(_) => return results,
            };
            let mapped = stmt.query_map([&name_query], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            });
            match mapped {
                Ok(r) => r.collect::<Vec<_>>(),
                Err(_) => return results,
            }
        };

        for row in rows.into_iter().filter_map(|r| r.ok()) {
            let (file_path, line_number, todo_text, repo_name) = row;
            let todo_lower = todo_text.to_lowercase();
            let file_lower = file_path.to_lowercase();
            
            let mut score = 1.0f32;
            if !q_lower.is_empty() {
                if todo_lower == q_lower {
                    score = 2.0;
                } else if todo_lower.starts_with(&q_lower) {
                    score = 1.6;
                } else if todo_lower.contains(&q_lower) || file_lower.contains(&q_lower) {
                    score = 1.2;
                } else {
                    let matched = q_words.iter().filter(|w| todo_lower.contains(*w) || file_lower.contains(*w)).count();
                    if matched > 0 {
                        score = 0.5 + 0.5 * (matched as f32 / q_words.len() as f32);
                    } else {
                        score = 0.0;
                    }
                }
            }

            if score > 0.0 {
                let filename = std::path::Path::new(&file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&file_path)
                    .to_string();
                
                results.push(SearchResult {
                    entry: CatalogEntry {
                        id: format!("git.todo.{}.{}", file_path, line_number),
                        control_name: format!("[{}] {}", repo_name, todo_text),
                        breadcrumb_path: format!("Git > TODO > {}:L{}", filename, line_number),
                        launch_command: format!("vscode:{}:{}", file_path, line_number),
                        source: "TODO".to_string(),
                        description: format!("Line {}: {}", line_number, file_path),
                        synonyms: format!("{} {}", todo_text, file_path),
                    },
                    score,
                });
            }
        }

        results.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let q = query.trim();
        let q_lower_trimmed = q.to_lowercase();
        
        // Intercept temporal context queries (e.g. yesterday before lunch)
        if let Some((start_time, end_time, clean_q)) = parse_time_range(q) {
            return self.search_timeline(start_time, end_time, &clean_q);
        }

        if q.is_empty() {
            let mut results = Vec::new();
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: "folder.bookmarks".to_string(),
                    control_name: "Browser Bookmarks".to_string(),
                    breadcrumb_path: "Browser > Bookmarks".to_string(),
                    launch_command: "bookmarks:".to_string(),
                    source: "FOLDER".to_string(),
                    description: "Folder containing all browser bookmarks".to_string(),
                    synonyms: "bookmarks folders browser favs favorites stars".to_string(),
                },
                score: 5.0,
            });
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: "folder.history".to_string(),
                    control_name: "Browser History".to_string(),
                    breadcrumb_path: "Browser > History".to_string(),
                    launch_command: "history:".to_string(),
                    source: "FOLDER".to_string(),
                    description: "Folder containing all browser history".to_string(),
                    synonyms: "history folders browser recent urls websites web".to_string(),
                },
                score: 4.5,
            });
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: "folder.commits".to_string(),
                    control_name: "Git Commits".to_string(),
                    breadcrumb_path: "Git > Commits".to_string(),
                    launch_command: "commits:".to_string(),
                    source: "FOLDER".to_string(),
                    description: "Folder containing recent git commits".to_string(),
                    synonyms: "git commits codes changes hashes log repo history".to_string(),
                },
                score: 4.0,
            });
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: "folder.todos".to_string(),
                    control_name: "Git TODOs".to_string(),
                    breadcrumb_path: "Git > TODOs".to_string(),
                    launch_command: "todos:".to_string(),
                    source: "FOLDER".to_string(),
                    description: "Folder containing active TODO and FIXME comments".to_string(),
                    synonyms: "git todos tasks comment code fixme bug".to_string(),
                },
                score: 3.5,
            });
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: "folder.clipboard".to_string(),
                    control_name: "Clipboard History".to_string(),
                    breadcrumb_path: "Clipboard > History".to_string(),
                    launch_command: "clip:".to_string(),
                    source: "FOLDER".to_string(),
                    description: "Folder containing clipboard history".to_string(),
                    synonyms: "clipboard history copy paste clip".to_string(),
                },
                score: 3.4,
            });
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: "folder.files".to_string(),
                    control_name: "Local Files".to_string(),
                    breadcrumb_path: "Local > Files".to_string(),
                    launch_command: "file:".to_string(),
                    source: "FOLDER".to_string(),
                    description: "Folder containing indexed documents and files".to_string(),
                    synonyms: "files documents local downloads desktop search pdf docx".to_string(),
                },
                score: 3.3,
            });
            results.push(SearchResult {
                entry: CatalogEntry {
                    id: "folder.code".to_string(),
                    control_name: "Source Code".to_string(),
                    breadcrumb_path: "Local > Source Code".to_string(),
                    launch_command: "code:".to_string(),
                    source: "FOLDER".to_string(),
                    description: "Folder containing indexed source code files".to_string(),
                    synonyms: "code source rust python js cpp java develop program".to_string(),
                },
                score: 3.2,
            });
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

        if q_lower_trimmed == "recent" || q_lower_trimmed == "recents" {
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

        if q_lower_trimmed.starts_with("clipboard:") {
            let sub_query = q_lower_trimmed.strip_prefix("clipboard:").unwrap().trim();
            return self.search_clipboard_history(sub_query);
        }

        if q_lower_trimmed.starts_with("clip:") {
            let sub_query = q_lower_trimmed.strip_prefix("clip:").unwrap().trim();
            return self.search_clipboard_history(sub_query);
        }

        if q_lower_trimmed.starts_with("bookmarks:") {
            let sub_query = q_lower_trimmed.strip_prefix("bookmarks:").unwrap().trim();
            return self.search_bookmarks_only(sub_query);
        }

        if q_lower_trimmed.starts_with("history:") {
            let sub_query = q_lower_trimmed.strip_prefix("history:").unwrap().trim();
            return self.search_history_only(sub_query);
        }

        if q_lower_trimmed.starts_with("commits:") {
            let sub_query = q_lower_trimmed.strip_prefix("commits:").unwrap().trim();
            return self.search_commits_only(sub_query);
        }

        if q_lower_trimmed.starts_with("todos:") {
            let sub_query = q_lower_trimmed.strip_prefix("todos:").unwrap().trim();
            return self.search_todos_only(sub_query);
        }

        if q_lower_trimmed.starts_with("file:") {
            let sub_query = q_lower_trimmed.strip_prefix("file:").unwrap().trim();
            return self.search_files_only(sub_query);
        }

        if q_lower_trimmed.starts_with("code:") {
            let sub_query = q_lower_trimmed.strip_prefix("code:").unwrap().trim();
            return self.search_code_only(sub_query);
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
        file_matches.truncate(15); // Cap at 15 file results

        // ── Cross-Source Entity Linker ("Project Auto-Entity") ──────────────
        let mut matched_project_name = None;
        if q.len() >= 3 {
            if let Ok(conn) = Connection::open(&self.db_path) {
                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                
                // Check git repos first
                if let Ok(mut s) = conn.prepare("SELECT name FROM git_repos") {
                    let names: Vec<String> = s.query_map([], |row| row.get::<_, String>(0))
                        .map(|m| m.filter_map(|r| r.ok()).collect())
                        .unwrap_or_default();
                    for name in names {
                        let name_lc = name.to_lowercase();
                        if q_lower_trimmed == name_lc || q_lower_trimmed.contains(&name_lc) || name_lc.contains(&q_lower_trimmed) {
                            matched_project_name = Some(name);
                            break;
                        }
                    }
                }
                
                // If not found in git repos, check folder names in indexed files
                if matched_project_name.is_none() {
                    if let Ok(mut s) = conn.prepare("SELECT path FROM files WHERE is_dir = 1") {
                        let paths: Vec<String> = s.query_map([], |row| row.get::<_, String>(0))
                            .map(|m| m.filter_map(|r| r.ok()).collect())
                            .unwrap_or_default();
                        for path in paths {
                            if let Some(folder_name) = std::path::Path::new(&path).file_name() {
                                let folder_name_lc = folder_name.to_string_lossy().to_lowercase();
                                if !folder_name_lc.is_empty() && (q_lower_trimmed == folder_name_lc || q_lower_trimmed.contains(&folder_name_lc) || folder_name_lc.contains(&q_lower_trimmed)) {
                                    matched_project_name = Some(folder_name.to_string_lossy().into_owned());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut project_results = Vec::new();
        if let Some(ref project_name) = matched_project_name {
            let mut repo_path = String::new();
            if let Ok(conn) = Connection::open(&self.db_path) {
                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                if let Ok(mut s) = conn.prepare("SELECT path FROM git_repos WHERE name = ? LIMIT 1") {
                    if let Ok(p) = s.query_row([project_name], |row| row.get::<_, String>(0)) {
                        repo_path = p;
                    }
                }
            }

            let workspace_card = SearchResult {
                entry: CatalogEntry {
                    id: format!("project.workspace.{}", project_name),
                    control_name: format!("📁 PROJECT WORKSPACE: {}", project_name),
                    breadcrumb_path: format!("Project > Dashboard > {}", project_name),
                    launch_command: repo_path,
                    source: "PROJECT".to_string(),
                    description: format!("Active workspace for project '{}'", project_name),
                    synonyms: format!("{} project workspace dashboard card", project_name.to_lowercase()),
                },
                score: 10.0,
            };

            project_results.push(workspace_card);
            project_results.append(&mut self.search_project(project_name));
        }

        let mut merged = Vec::new();
        merged.append(&mut app_matches);
        merged.append(&mut recent_matches);
        merged.append(&mut file_matches);
        merged.append(&mut vec_results);
        merged.append(&mut project_results);
        merged.push(web_search.clone());
        merged.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        conv_results.append(&mut final_results);
        final_results = conv_results;
        final_results.append(&mut merged);
        
        // Deduplicate final_results by id or non-empty launch_command
        let mut unique_results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        let mut seen_launches = std::collections::HashSet::new();

        for r in final_results {
            let is_duplicate = seen_ids.contains(&r.entry.id) || 
                (!r.entry.launch_command.is_empty() && seen_launches.contains(&r.entry.launch_command));
            if !is_duplicate {
                seen_ids.insert(r.entry.id.clone());
                if !r.entry.launch_command.is_empty() {
                    seen_launches.insert(r.entry.launch_command.clone());
                }
                unique_results.push(r);
            }
        }
        final_results = unique_results;
        
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

    #[test]
    fn test_search_file() {
        let db_path = match std::env::var("APPDATA") {
            Ok(d) => {
                let path = std::path::PathBuf::from(d).join("opensearch-os");
                path.join("file_index.db")
            }
            Err(_) => std::path::PathBuf::from("file_index.db"),
        };
        let exe = std::env::current_exe().expect("failed to get current exe");
        let parent = exe.parent().expect("failed to get parent");
        let mut model_path = parent.join("model_int8.onnx");
        if !model_path.exists() {
            model_path = parent.parent().expect("failed to get grandparent").join("model_int8.onnx");
        }
        let mut engine = SearchEngine::new(&model_path, db_path).expect("Failed to initialize engine");
        
        println!("--- DIAGNOSTIC SEARCH TEST ---");
        let results = engine.search("resume", 10);
        println!("Combined search results for 'resume':");
        for (idx, r) in results.iter().enumerate() {
            println!("  [{}] ID: {}, Name: {}, Source: {}, Breadcrumb: {}, Score: {}", idx, r.entry.id, r.entry.control_name, r.entry.source, r.entry.breadcrumb_path, r.score);
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
    unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    }

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

fn get_bmp_dimensions(path: &str) -> Option<(i32, i32)> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut header = [0u8; 26];
    file.read_exact(&mut header).ok()?;
    if &header[0..2] != b"BM" { return None; }
    let width = i32::from_le_bytes(header[18..22].try_into().ok()?);
    let height = i32::from_le_bytes(header[22..26].try_into().ok()?);
    Some((width.abs(), height.abs()))
}

fn parse_time_range(query: &str) -> Option<(i64, i64, String)> {
    let q = query.to_lowercase();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    
    let local_time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    
    let mut time_zone_info = windows::Win32::System::Time::TIME_ZONE_INFORMATION::default();
    let _ = unsafe { windows::Win32::System::Time::GetTimeZoneInformation(&mut time_zone_info) };
    let bias_minutes = time_zone_info.Bias;
    
    let seconds_since_midnight = (local_time.wHour as i64 * 3600) + (local_time.wMinute as i64 * 60) + local_time.wSecond as i64;
    let today_start = now - seconds_since_midnight;
    let yesterday_start = today_start - 86400;
    
    let mut start_time = 0;
    let mut end_time = 0;
    let mut time_phrase = "";
    
    if q.contains("yesterday before lunch") {
        time_phrase = "yesterday before lunch";
        start_time = yesterday_start + 8 * 3600;
        end_time = yesterday_start + 12 * 3600;
    } else if q.contains("yesterday after lunch") {
        time_phrase = "yesterday after lunch";
        start_time = yesterday_start + 13 * 3600;
        end_time = yesterday_start + 17 * 3600;
    } else if q.contains("yesterday") {
        time_phrase = "yesterday";
        start_time = yesterday_start;
        end_time = today_start;
    } else if q.contains("before lunch") {
        time_phrase = "before lunch";
        start_time = today_start + 8 * 3600;
        end_time = today_start + 12 * 3600;
    } else if q.contains("after lunch") {
        time_phrase = "after lunch";
        start_time = today_start + 13 * 3600;
        end_time = today_start + 17 * 3600;
    } else if q.contains("before the meeting") {
        time_phrase = "before the meeting";
        start_time = today_start + 8 * 3600;
        end_time = today_start + 10 * 3600;
    } else if q.contains("this morning") {
        time_phrase = "this morning";
        start_time = today_start + 6 * 3600;
        end_time = today_start + 12 * 3600;
    } else if q.contains("this afternoon") {
        time_phrase = "this afternoon";
        start_time = today_start + 12 * 3600;
        end_time = today_start + 17 * 3600;
    } else if q.contains("today") {
        time_phrase = "today";
        start_time = today_start;
        end_time = now;
    } else if q.contains("last week") {
        time_phrase = "last week";
        start_time = today_start - 7 * 86400;
        end_time = now;
    } else if q.contains("last month") {
        time_phrase = "last month";
        start_time = today_start - 30 * 86400;
        end_time = now;
    }
    
    if !time_phrase.is_empty() {
        let clean_phrase = q.replace(time_phrase, "");
        let mut clean_query = clean_phrase.trim().to_string();
        
        for word in &["opened", "edited", "visited", "used", "the", "file", "code", "before", "after", "during", "i", "at"] {
            if clean_query.starts_with(word) {
                clean_query = clean_query.strip_prefix(word).unwrap().trim().to_string();
            }
            if clean_query.ends_with(word) {
                clean_query = clean_query.strip_suffix(word).unwrap().trim().to_string();
            }
        }
        
        return Some((start_time, end_time, clean_query));
    }
    
    None
}

fn format_timestamp_local(timestamp: i64) -> String {
    let filetime_val = (timestamp + 11644473600) * 10000000;
    let ft = windows::Win32::Foundation::FILETIME {
        dwLowDateTime: (filetime_val & 0xFFFFFFFF) as u32,
        dwHighDateTime: (filetime_val >> 32) as u32,
    };
    let mut local_ft = windows::Win32::Foundation::FILETIME::default();
    let mut st = windows::Win32::Foundation::SYSTEMTIME::default();
    unsafe {
        let _ = windows::Win32::Storage::FileSystem::FileTimeToLocalFileTime(&ft, &mut local_ft);
        let _ = windows::Win32::System::Time::FileTimeToSystemTime(&local_ft, &mut st);
    }
    
    let am_pm = if st.wHour >= 12 { "PM" } else { "AM" };
    let hour = if st.wHour == 0 {
        12
    } else if st.wHour > 12 {
        st.wHour - 12
    } else {
        st.wHour
    };
    
    format!("{:04}-{:02}-{:02} {:02}:{:02} {}", st.wYear, st.wMonth, st.wDay, hour, st.wMinute, am_pm)
}

fn extract_path_or_url(text: &str) -> Option<String> {
    if let Some(idx) = text.find(":\\") {
        if idx >= 1 {
            let start = idx - 1;
            let path_part = &text[start..];
            if let Some(sep_idx) = path_part.find(" - ") {
                return Some(path_part[..sep_idx].trim().to_string());
            }
            if let Some(sep_idx) = path_part.find(" | ") {
                return Some(path_part[..sep_idx].trim().to_string());
            }
            return Some(path_part.trim().to_string());
        }
    }
    if let Some(idx) = text.find(":/") {
        if idx >= 1 {
            let start = idx - 1;
            let path_part = &text[start..];
            if let Some(sep_idx) = path_part.find(" - ") {
                return Some(path_part[..sep_idx].trim().to_string());
            }
            if let Some(sep_idx) = path_part.find(" | ") {
                return Some(path_part[..sep_idx].trim().to_string());
            }
            return Some(path_part.trim().to_string());
        }
    }
    if let Some(idx) = text.find("http://") {
        let part = &text[idx..];
        if let Some(sep_idx) = part.find(' ') {
            return Some(part[..sep_idx].trim().to_string());
        }
        return Some(part.trim().to_string());
    }
    if let Some(idx) = text.find("https://") {
        let part = &text[idx..];
        if let Some(sep_idx) = part.find(' ') {
            return Some(part[..sep_idx].trim().to_string());
        }
        return Some(part.trim().to_string());
    }
    None
}
