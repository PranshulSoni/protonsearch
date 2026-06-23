use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::collections::HashMap;
use walkdir::WalkDir;
use rusqlite::{Connection, params};

pub fn start_indexer(db_path: PathBuf) {
    thread::spawn(move || {
        // Set low priority to run strictly in the background without affecting foreground apps
        unsafe {
            use windows::Win32::System::Threading::{SetThreadPriority, GetCurrentThread, THREAD_PRIORITY_BELOW_NORMAL};
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
        }
        
        // Initial delay to let the app start up completely lag-free
        thread::sleep(std::time::Duration::from_secs(5));
        loop {
            if let Err(e) = run_indexer(&db_path) {
                eprintln!("Indexer error: {:?}", e);
            }
            // Re-scan every 10 minutes
            thread::sleep(std::time::Duration::from_secs(600));
        }
    });
}

fn is_ignored_dir(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    if name_lower.starts_with('$') {
        return true;
    }
    match name_lower.as_str() {
        "node_modules" | "target" | "build" | "dist" | "venv" | ".venv" | ".git" |
        "appdata" | "obj" | "bin" | "out" | ".next" | ".nuxt" | ".cache" | "cache" |
        ".cargo" | ".rustup" | ".npm" | ".m2" | ".nuget" | "vendor" |
        "cmake-build-debug" | "cmake-build-release" | ".yarn" | "__pycache__" |
        ".idea" | ".vscode" | ".gradle" | ".metadata" | "system volume information" |
        "temp" | "tmp" => true,
        _ => false,
    }
}

fn is_ignored_file(name: &str, ext: &str) -> bool {
    if name.starts_with("~$") {
        return true;
    }
    match ext {
        "tmp" | "temp" | "log" | "pdb" | "obj" | "o" | "class" | "db-wal" | "db-shm" => true,
        _ => false,
    }
}

fn run_indexer(db_path: &Path) -> anyhow::Result<()> {
    let mut conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            extension TEXT NOT NULL,
            modified INTEGER NOT NULL
        );",
        [],
    )?;
    
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
            path UNINDEXED,
            content
        );",
        [],
    )?;

    let folders = get_scan_folders();
    let mut seen_paths = std::collections::HashSet::new();

    // Cache existing database file paths and modified times in memory to avoid query overhead
    let mut db_files = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT path, modified FROM files")?;
        let db_files_iter = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for item in db_files_iter {
            if let Ok((p, m)) = item {
                db_files.insert(p, m);
            }
        }
    }

    let mut file_count = 0;

    for folder in folders {
        if !folder.exists() { continue; }
        let walker = WalkDir::new(folder)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !is_ignored_dir(&name)
            });
            
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            let is_file = path.is_file();
            let is_dir = path.is_dir();
            if !is_file && !is_dir { continue; }
            
            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let ext = if is_dir {
                "folder".to_string()
            } else {
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase()
            };

            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let name = if name.is_empty() {
                path_str.clone()
            } else {
                name
            };

            if is_file && is_ignored_file(&name, &ext) { continue; }

            seen_paths.insert(path_str.clone());

            let modified = entry.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let db_modified = db_files.get(&path_str).copied();

            if db_modified.is_none() || db_modified.unwrap() != modified {
                conn.execute(
                    "INSERT OR REPLACE INTO files (path, name, extension, modified) VALUES (?, ?, ?, ?)",
                    params![path_str, name, ext, modified],
                )?;

                if is_file {
                    // Only perform content extraction for FTS5 on text documents and source code files
                    let text_extensions = [
                        "txt", "md", "rs", "py", "js", "ts", "json", "html", "css",
                        "c", "cpp", "h", "hpp", "cs", "go", "java", "kt", "sh", "bat",
                        "ps1", "yaml", "yml", "toml", "ini", "sql", "xml"
                    ];
                    let is_pdf = ext == "pdf";
                    let is_docx = ext == "docx";

                    if text_extensions.contains(&ext.as_str()) || is_pdf || is_docx {
                        let extracted = if is_pdf {
                            match pdf_extract::extract_text(path) {
                                Ok(text) => {
                                    let mut truncated = text;
                                    truncated.truncate(50 * 1024);
                                    Some(truncated)
                                }
                                Err(e) => {
                                    eprintln!("PDF extract failed for {:?}: {:?}", path, e);
                                    None
                                }
                            }
                        } else if is_docx {
                            match docx_lite::extract_text(path) {
                                Ok(text) => {
                                    let mut truncated = text;
                                    truncated.truncate(50 * 1024);
                                    Some(truncated)
                                }
                                Err(e) => {
                                    eprintln!("DOCX extract failed for {:?}: {:?}", path, e);
                                    None
                                }
                            }
                        } else {
                            read_text_file(path).ok()
                        };

                        if let Some(content) = extracted {
                            conn.execute("DELETE FROM files_fts WHERE path = ?", [&path_str])?;
                            conn.execute(
                                "INSERT INTO files_fts (path, content) VALUES (?, ?)",
                                params![path_str, content],
                            )?;
                        }

                        if is_pdf || is_docx {
                            thread::sleep(std::time::Duration::from_millis(50));
                        }
                    }
                }
            }

            // Yield CPU cycles after scanning every 100 files
            file_count += 1;
            if file_count % 100 == 0 {
                thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }

    // Clean up deleted files from the database in a single transaction
    let mut to_delete = Vec::new();
    for p_str in db_files.keys() {
        if !seen_paths.contains(p_str) {
            to_delete.push(p_str);
        }
    }

    if !to_delete.is_empty() {
        let tx = conn.transaction()?;
        for p_str in to_delete {
            tx.execute("DELETE FROM files WHERE path = ?", [&p_str])?;
            tx.execute("DELETE FROM files_fts WHERE path = ?", [&p_str])?;
        }
        tx.commit()?;
    }

    Ok(())
}

pub fn get_scan_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    
    let system_drive = std::env::var("SystemDrive")
        .unwrap_or_else(|_| "C:".to_string())
        .to_uppercase();

    // 1. Get the User Profile folder
    unsafe {
        use windows::Win32::UI::Shell::{SHGetKnownFolderPath, FOLDERID_Profile, KF_FLAG_DEFAULT};
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Com::CoTaskMemFree;
        
        let get_folder = |guid| -> Option<PathBuf> {
            let result = SHGetKnownFolderPath(guid, KF_FLAG_DEFAULT, HANDLE::default()).ok()?;
            let mut len = 0;
            while *result.0.add(len) != 0 { len += 1; }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(result.0, len));
            CoTaskMemFree(Some(result.0 as *const _));
            Some(PathBuf::from(s))
        };

        if let Some(p) = get_folder(&FOLDERID_Profile) {
            folders.push(p);
        }
    }

    if folders.is_empty() {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            folders.push(PathBuf::from(profile));
        }
    }

    // 2. Discover all other fixed drives and scan them from their roots
    for c in b'A'..=b'Z' {
        let drive_letter = c as char;
        let drive_path_str = format!("{}:\\", drive_letter);
        if drive_path_str.to_uppercase().starts_with(&system_drive) {
            continue;
        }
        let wide_path: Vec<u16> = drive_path_str.encode_utf16().chain(Some(0)).collect();
        unsafe {
            use windows::Win32::Storage::FileSystem::GetDriveTypeW;
            let drive_type = GetDriveTypeW(windows::core::PCWSTR(wide_path.as_ptr()));
            if drive_type == 3 { // 3 corresponds to DRIVE_FIXED in Win32
                folders.push(PathBuf::from(drive_path_str));
            }
        }
    }

    folders
}

fn read_text_file(path: &Path) -> std::io::Result<String> {
    use std::fs::File;
    use std::io::Read;
    
    let mut file = File::open(path)?;
    let mut buf = vec![0u8; 50 * 1024]; // Limit to 50KB
    let n = file.read(&mut buf)?;
    buf.truncate(n);
    
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
