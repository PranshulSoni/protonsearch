use notify::Watcher;
use once_cell::sync::Lazy;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub static IS_INDEXING: AtomicBool = AtomicBool::new(false);
pub static INDEXING_PROGRESS: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Idle".to_string()));
pub static LAST_INDEX_TIME: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("Never".to_string()));

const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "json", "html", "css", "c", "cpp", "h",
    "hpp", "cs", "go", "java", "kt", "sh", "bat", "ps1", "yaml", "yml", "toml", "ini", "sql",
    "xml", "rb", "php", "lua", "swift", "dart", "vue", "svelte", "csv", "tex", "rst", "adoc",
    "conf", "env",
];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp"];

fn is_indexable_content(ext: &str) -> bool {
    TEXT_EXTENSIONS.contains(&ext)
        || ext == "pdf"
        || ext == "docx"
        || IMAGE_EXTENSIONS.contains(&ext)
}

/// Extract searchable text for a file (document text or image OCR), or None.
fn extract_content(path: &Path, ext: &str) -> Option<String> {
    if TEXT_EXTENSIONS.contains(&ext) {
        read_text_file(path).ok()
    } else if ext == "pdf" {
        safe_extract_pdf_text(path)
    } else if ext == "docx" {
        safe_extract_docx_text(path)
    } else if IMAGE_EXTENSIONS.contains(&ext) {
        extract_ocr_text(path)
    } else {
        None
    }
}

/// True if any path component is an ignored directory (node_modules, .git, appdata, temp…).
fn path_in_ignored_dir(path: &Path) -> bool {
    path.components()
        .any(|c| c.as_os_str().to_str().map(is_ignored_dir).unwrap_or(false))
}

/// Index a single file immediately: upsert name/meta, plus content/OCR for indexable types.
/// Used by the filesystem watcher so new files are searchable in milliseconds.
fn index_one_file(conn: &Connection, path: &Path) {
    let meta = match std::fs::metadata(path) {
        Ok(m) if m.is_file() => m,
        _ => return,
    };
    let path_str = match path.to_str() {
        Some(s) => s.to_string(),
        None => return,
    };
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if name.is_empty() || is_ignored_file(&name, &ext) {
        return;
    }
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let size = meta.len() as i64;

    let _ = conn.execute(
        "INSERT OR REPLACE INTO files (path, name, extension, modified, size, is_dir) VALUES (?,?,?,?,?,0)",
        params![path_str, name, ext, modified, size],
    );
    if is_indexable_content(&ext) {
        let content = extract_content(path, &ext).unwrap_or_default();
        let _ = conn.execute("DELETE FROM files_fts WHERE path = ?", [&path_str]);
        let _ = conn.execute(
            "INSERT INTO files_fts (path, content) VALUES (?, ?)",
            params![path_str, content],
        );
    }
}

static WATCHER: Lazy<Mutex<Option<notify::RecommendedWatcher>>> = Lazy::new(|| Mutex::new(None));

/// Watch the user profile + fixed drives and index created/modified files instantly.
/// Falls back to the periodic crawl (safety net) for any events the OS drops.
pub fn start_watcher(db_path: PathBuf) {
    let is_initial = WATCHER.lock().unwrap().is_none();

    let mut g = WATCHER.lock().unwrap();
    // Drop the old watcher first to release all OS watches and close mpsc channels
    *g = None;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = match notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    }) {
        Ok(w) => w,
        Err(e) => {
            log_indexer(&format!("Watcher init failed: {e}"));
            return;
        }
    };

    let folders = get_scan_folders();
    for folder in &folders {
        log_indexer(&format!("Watcher watching folder: {:?}", folder));
        let _ = watcher.watch(folder, notify::RecursiveMode::Recursive);
    }

    let db_path_clone = db_path.clone();
    thread::spawn(move || {
        if is_initial {
            // Wait 15s for the initial indexer I/O burst to settle before registering
            // recursive filesystem watches.
            thread::sleep(std::time::Duration::from_secs(15));
        }

        // OCR (WinRT) needs COM initialized on this thread.
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };

        let conn = match Connection::open(&db_path_clone) {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        log_indexer("Watcher started");

        let mut collect = |set: &mut std::collections::HashSet<PathBuf>,
                           res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                if matches!(
                    ev.kind,
                    notify::EventKind::Create(_) | notify::EventKind::Modify(_)
                ) {
                    for p in ev.paths {
                        set.insert(p);
                    }
                }
            }
        };

        loop {
            let first = match rx.recv() {
                Ok(r) => r,
                Err(_) => break,
            };
            let mut paths = std::collections::HashSet::new();
            collect(&mut paths, first);
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(800);
            while let Ok(r) =
                rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            {
                collect(&mut paths, r);
                if paths.len() > 2000 {
                    break;
                }
            }
            for p in paths {
                if !path_in_ignored_dir(&p) {
                    index_one_file(&conn, &p);
                }
            }
        }
        log_indexer("Watcher thread exited");
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    });

    *g = Some(watcher);
}

fn log_indexer(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let log_dir = match std::env::var("APPDATA") {
        Ok(d) => PathBuf::from(d).join("omnisearch"),
        Err(_) => PathBuf::from("."),
    };
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("indexer.log");
    if let Ok(meta) = std::fs::metadata(&log_path) {
        if meta.len() > 1024 * 1024 {
            let _ = std::fs::remove_file(&log_path);
        }
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(file, "{}", msg);
    }
}

pub fn start_indexer(db_path: PathBuf) {
    let db_path_clone = db_path.clone();
    thread::spawn(move || {
        log_indexer("Indexer thread started");
        // Initialize COM for WinRT OCR
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };

        // Set low priority so indexing never slows down foreground apps
        unsafe {
            use windows::Win32::System::Threading::{
                GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
            };
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
        }

        // ── Phase 1: Priority folders (Desktop, Downloads, Pictures, Documents) ──────
        // Delay 3s so the launcher window is fully up and the settings app can open
        // without being immediately hit by heavy first-launch DB writes.
        thread::sleep(std::time::Duration::from_secs(3));
        log_indexer("Starting Phase 1 priority scan...");
        if let Err(e) = run_indexer_folders(&db_path_clone, get_priority_folders()) {
            log_indexer(&format!("Priority indexer error: {:?}", e));
            eprintln!("Priority indexer error: {:?}", e);
        }

        // ── Phase 2: Full crawl (entire user profile + other drives) ───────
        // Runs 15s after launch to further reduce first-open contention.
        thread::sleep(std::time::Duration::from_secs(15));
        log_indexer("Starting Phase 2 full crawl...");
        if let Err(e) = run_indexer_folders(&db_path_clone, get_scan_folders()) {
            log_indexer(&format!("Indexer error: {:?}", e));
            eprintln!("Indexer error: {:?}", e);
        }
        log_indexer("Phase 2 crawl finished.");
    });
}

/// Returns Desktop, Downloads, Pictures, Documents — fast to scan, highest value to user.
fn get_priority_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    unsafe {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Com::CoTaskMemFree;
        use windows::Win32::UI::Shell::{
            FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Pictures,
            SHGetKnownFolderPath, KF_FLAG_DEFAULT,
        };

        let get_folder = |guid| -> Option<PathBuf> {
            let result = SHGetKnownFolderPath(guid, KF_FLAG_DEFAULT, HANDLE::default()).ok()?;
            let mut len = 0;
            while *result.0.add(len) != 0 {
                len += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(result.0, len));
            CoTaskMemFree(Some(result.0 as *const _));
            Some(PathBuf::from(s))
        };

        // Put Documents last so it doesn't block Desktop/Downloads/Pictures from being indexed immediately
        for guid in [
            &FOLDERID_Desktop,
            &FOLDERID_Downloads,
            &FOLDERID_Pictures,
            &FOLDERID_Documents,
        ] {
            if let Some(p) = get_folder(guid) {
                folders.push(p);
            }
        }
    }
    folders
}

fn is_ignored_dir(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    if name_lower.starts_with('$') {
        return true;
    }
    match name_lower.as_str() {
        "node_modules"
        | "target"
        | "build"
        | "dist"
        | "venv"
        | ".venv"
        | ".git"
        | "appdata"
        | "obj"
        | "bin"
        | "out"
        | ".next"
        | ".nuxt"
        | ".cache"
        | "cache"
        | ".cargo"
        | ".rustup"
        | ".npm"
        | ".m2"
        | ".nuget"
        | "vendor"
        | "cmake-build-debug"
        | "cmake-build-release"
        | ".yarn"
        | "__pycache__"
        | ".idea"
        | ".vscode"
        | ".gradle"
        | ".metadata"
        | "system volume information"
        | "temp"
        | "tmp" => true,
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

struct PendingUpdate {
    path: String,
    name: String,
    extension: String,
    modified: i64,
    size: i64,
    is_dir: i64,
    content: Option<String>,
}

fn flush_updates(conn: &mut Connection, updates: &mut Vec<PendingUpdate>) -> anyhow::Result<()> {
    if updates.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    {
        let mut insert_file_stmt = tx.prepare(
            "INSERT OR REPLACE INTO files (path, name, extension, modified, size, is_dir) VALUES (?, ?, ?, ?, ?, ?)"
        )?;
        let mut delete_fts_stmt = tx.prepare("DELETE FROM files_fts WHERE path = ?")?;
        let mut insert_fts_stmt =
            tx.prepare("INSERT INTO files_fts (path, content) VALUES (?, ?)")?;

        for update in updates.drain(..) {
            // Clone path before moving into params! so FTS statements can use it afterwards
            let path_clone = update.path.clone();
            insert_file_stmt.execute(params![
                update.path,
                update.name,
                update.extension,
                update.modified,
                update.size,
                update.is_dir
            ])?;

            if let Some(content) = update.content {
                delete_fts_stmt.execute([&path_clone])?;
                insert_fts_stmt.execute(params![path_clone, content])?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

struct ExtractJob {
    path: PathBuf,
    name: String,
    ext: String,
    modified: i64,
    size: i64,
}

/// Extract document text + image OCR for the given files in parallel (the slow part of
/// indexing). Returns a receiver of completed PendingUpdates so the caller can flush them
/// to SQLite incrementally (bounded result channel keeps memory in check). Workers run at
/// below-normal priority so a fast first-pass index still yields to the foreground.
fn spawn_extractors(jobs: Vec<ExtractJob>) -> std::sync::mpsc::Receiver<PendingUpdate> {
    use std::sync::mpsc;

    let n_workers = std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1).min(4))
        .unwrap_or(2);

    let (job_tx, job_rx) = mpsc::channel::<ExtractJob>(); // unbounded; jobs are tiny (paths/meta)
    let (res_tx, res_rx) = mpsc::channel::<PendingUpdate>(); // unbounded → no deadlock risk

    for _ in 0..n_workers {
        let job_rx = job_rx.clone();
        let res_tx = res_tx.clone();
        thread::spawn(move || {
            // OCR (WinRT) needs COM; below-normal so we don't starve the user.
            let _ = unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED,
                )
            };
            unsafe {
                use windows::Win32::System::Threading::{
                    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
                };
                let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
            }
            loop {
                let job = match job_rx.recv() {
                    Ok(j) => j,
                    Err(_) => break, // feeder dropped → all jobs done
                };
                let content = extract_content(&job.path, &job.ext).unwrap_or_default();
                let _ = res_tx.send(PendingUpdate {
                    path: job.path.to_string_lossy().into_owned(),
                    name: job.name,
                    extension: job.ext,
                    modified: job.modified,
                    size: job.size,
                    is_dir: 0,
                    content: Some(content),
                });
            }
            // CoUninitialize on thread exit to balance CoInitializeEx
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        });
    }
    drop(res_tx); // workers hold the only senders now → res_rx ends when they finish

    // Feed jobs from a separate thread so sending never blocks the result drain.
    thread::spawn(move || {
        for job in jobs {
            if job_tx.send(job).is_err() {
                break;
            }
        }
    });

    res_rx
}

fn save_indexer_state_to_db(db_path: &Path, key: &str, value: &str) {
    if let Ok(conn) = Connection::open(db_path) {
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS indexer_state (
                key TEXT PRIMARY KEY,
                value TEXT
            );",
            [],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO indexer_state (key, value) VALUES (?, ?);",
            [key, value],
        );
    }
}

pub fn run_indexer_folders_force(db_path: &Path, folders: Vec<PathBuf>) -> anyhow::Result<()> {
    if let Ok(mut g) = INDEXING_PROGRESS.lock() {
        *g = "Forced indexing of added folder...".to_string();
        save_indexer_state_to_db(db_path, "progress", &g);
    }
    let res = run_indexer_folders_inner(db_path, folders);
    if let Ok(mut g) = INDEXING_PROGRESS.lock() {
        *g = "Idle".to_string();
        save_indexer_state_to_db(db_path, "progress", &g);
    }
    res
}

pub fn run_indexer_folders(db_path: &Path, folders: Vec<PathBuf>) -> anyhow::Result<()> {
    if IS_INDEXING.swap(true, Ordering::SeqCst) {
        log_indexer("Indexer already running, skipping overlapping run.");
        return Ok(());
    }
    save_indexer_state_to_db(db_path, "is_indexing", "1");

    if let Ok(mut g) = INDEXING_PROGRESS.lock() {
        *g = "Starting scan...".to_string();
        save_indexer_state_to_db(db_path, "progress", &g);
    }
    let res = run_indexer_folders_inner(db_path, folders);
    IS_INDEXING.store(false, Ordering::SeqCst);
    save_indexer_state_to_db(db_path, "is_indexing", "0");
    if let Ok(mut g) = INDEXING_PROGRESS.lock() {
        *g = "Idle".to_string();
        save_indexer_state_to_db(db_path, "progress", &g);
    }
    if res.is_ok() {
        if let Ok(mut g) = LAST_INDEX_TIME.lock() {
            unsafe {
                let st = windows::Win32::System::SystemInformation::GetLocalTime();
                *g = format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
                );
            }
            save_indexer_state_to_db(db_path, "last_index_time", &g);
        }
    }
    res
}

fn run_indexer_folders_inner(db_path: &Path, folders: Vec<PathBuf>) -> anyhow::Result<()> {
    log_indexer(&format!(
        "run_indexer_folders started with folders: {:?}",
        folders
    ));
    let mut conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            extension TEXT NOT NULL,
            modified INTEGER NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            is_dir INTEGER NOT NULL DEFAULT 0
        );",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_files_name ON files(name);",
        [],
    )?;

    // Migrate existing databases that may lack the new columns
    let _ = conn.execute(
        "ALTER TABLE files ADD COLUMN size INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE files ADD COLUMN is_dir INTEGER NOT NULL DEFAULT 0",
        [],
    );

    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
            path UNINDEXED,
            content
        );",
        [],
    )?;
    // Removed unbounded memory caching of db_files, fts_paths, seen_paths (#8)
    // They grew infinitely on large drives and caused memory leaks.
    let mut file_count = 0;
    let mut pending_updates = Vec::new();
    let mut extract_jobs: Vec<ExtractJob> = Vec::new();

    for folder in folders {
        log_indexer(&format!("Evaluating folder for index: {:?}", folder));
        if !folder.exists() {
            log_indexer(&format!("Folder does not exist, skipping: {:?}", folder));
            continue;
        }
        if let Ok(mut g) = INDEXING_PROGRESS.lock() {
            *g = format!("Scanning: {}", folder.to_string_lossy());
            save_indexer_state_to_db(db_path, "progress", &g);
        }
        log_indexer(&format!("Folder exists, starting WalkDir: {:?}", folder));
        let walker = WalkDir::new(&folder).into_iter().filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !e.file_type().is_dir() || !is_ignored_dir(&name)
        });

        let mut folder_file_count = 0;
        for entry in walker.filter_map(|e| e.ok()) {
            folder_file_count += 1;
            let path = entry.path();
            let is_file = path.is_file();
            let is_dir = path.is_dir();
            if !is_file && !is_dir {
                continue;
            }

            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let file_size = if is_file {
                entry.metadata().map(|m| m.len()).unwrap_or(0) as i64
            } else {
                0
            };

            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let should_fts = is_indexable_content(&ext);

            // Fast path: query metadata DB to see if changed before doing FTS read
            let db_modified: Option<i64> = {
                let mut stmt = conn.prepare_cached("SELECT modified FROM files WHERE path = ?")?;
                stmt.query_row([&path_str], |r| r.get(0)).ok()
            };

            // Also check FTS table if it should have content
            let needs_fts_check = if should_fts && db_modified.is_some() {
                let mut stmt_fts =
                    conn.prepare_cached("SELECT rowid FROM files_fts WHERE path = ?")?;
                stmt_fts.query_row([&path_str], |_| Ok(())).is_err()
            } else {
                false
            };

            if db_modified.is_none() || db_modified.unwrap() != modified || needs_fts_check {
                if is_file && should_fts {
                    // Defer the slow content/OCR extraction to the parallel pool below.
                    extract_jobs.push(ExtractJob {
                        path: path.to_path_buf(),
                        name,
                        ext,
                        modified,
                        size: file_size,
                    });
                } else {
                    // Folders / non-indexable files: just the name+meta row, no extraction.
                    pending_updates.push(PendingUpdate {
                        path: path_str,
                        name,
                        extension: ext,
                        modified,
                        size: file_size,
                        is_dir: if is_dir { 1 } else { 0 },
                        content: None,
                    });
                    if pending_updates.len() >= 1000 {
                        flush_updates(&mut conn, &mut pending_updates)?;
                    }
                }
            }

            file_count += 1;
            if file_count % 1000 == 0 {
                if let Ok(mut g) = INDEXING_PROGRESS.lock() {
                    *g = format!("Scanning: {} files processed...", file_count);
                    save_indexer_state_to_db(db_path, "progress", &g);
                }
            }
        }
        log_indexer(&format!(
            "Finished WalkDir for {:?}: scanned {} total entries",
            folder, folder_file_count
        ));
    }

    log_indexer("Flushing name/meta updates to database...");
    flush_updates(&mut conn, &mut pending_updates)?;

    // Extract document text + image OCR for changed files in parallel (the slow part),
    // flushing results as they stream back so memory stays bounded.
    if !extract_jobs.is_empty() {
        log_indexer(&format!(
            "Extracting content/OCR for {} files in parallel...",
            extract_jobs.len()
        ));
        let total_jobs = extract_jobs.len();
        let mut processed_jobs = 0;
        for update in spawn_extractors(extract_jobs) {
            processed_jobs += 1;
            if processed_jobs % 10 == 0 || processed_jobs == total_jobs {
                if let Ok(mut g) = INDEXING_PROGRESS.lock() {
                    *g = format!(
                        "Extracting: {}/{} files (OCR/Text)...",
                        processed_jobs, total_jobs
                    );
                    save_indexer_state_to_db(db_path, "progress", &g);
                }
            }
            pending_updates.push(update);
            if pending_updates.len() >= 500 {
                flush_updates(&mut conn, &mut pending_updates)?;
            }
        }
        flush_updates(&mut conn, &mut pending_updates)?;
    }

    // Deleted files cleanup skipped to prevent memory bloat of seen_paths (#8)

    log_indexer("run_indexer_folders completed successfully");
    Ok(())
}

pub fn get_scan_folders() -> Vec<PathBuf> {
    let app_settings = crate::settings::AppSettings::load();
    if !app_settings.scan_folders.is_empty() {
        return app_settings
            .scan_folders
            .iter()
            .map(PathBuf::from)
            .collect();
    }
    get_default_scan_folders()
}

pub fn get_default_scan_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();

    // Only scan the user's home profile directory by default.
    // ProgramFiles and other drive roots are excluded to prevent massive startup I/O.
    unsafe {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Com::CoTaskMemFree;
        use windows::Win32::UI::Shell::{FOLDERID_Profile, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

        let get_folder = |guid| -> Option<PathBuf> {
            let result = SHGetKnownFolderPath(guid, KF_FLAG_DEFAULT, HANDLE::default()).ok()?;
            let mut len = 0;
            while *result.0.add(len) != 0 {
                len += 1;
            }
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

    folders
}

fn safe_extract_pdf_text(path: &Path) -> Option<String> {
    extract_via_subprocess(path, "PDF")
}

fn safe_extract_docx_text(path: &Path) -> Option<String> {
    extract_via_subprocess(path, "DOCX")
}

/// Child-process entry (`--extract-content <path>`): print the document's extracted text to
/// stdout, then exit. pdf_extract/docx_lite can stack-overflow on malformed files, and a
/// stack overflow is an abort that `catch_unwind` cannot catch — so we run them here, in a
/// throwaway process. If it overflows, this child dies and the parent just skips the content.
pub fn extract_content_subprocess(path_str: &str) {
    let path = Path::new(path_str);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let text = match ext.as_str() {
        "pdf" => pdf_extract::extract_text(path).ok(),
        "docx" => docx_lite::extract_text(path).ok(),
        _ => None,
    };
    if let Some(mut t) = text {
        t.truncate(50 * 1024);
        use std::io::Write;
        let _ = std::io::stdout().write_all(t.as_bytes());
        let _ = std::io::stdout().flush();
    }
}

/// Extract document text in a child process so a parser stack overflow can't crash the
/// indexer. Drains stdout on a helper thread (no pipe deadlock) and kills a child that runs
/// too long. Returns None on spawn failure / timeout / child crash / empty output — the file
/// is still indexed by name, just without full-text content.
/// ponytail: one process spawn per PDF/DOCX — fine for the low-priority background crawl.
/// If it ever dominates indexing time, switch to a single persistent extractor process.
fn extract_via_subprocess(path: &Path, kind: &str) -> Option<String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    let exe = std::env::current_exe().ok()?;
    let mut child = Command::new(exe)
        .arg("--extract-content")
        .arg(path)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Drain stdout on a helper thread so a large document can't deadlock the pipe.
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    // Watchdog: kill a child that runs too long (hang or pathological input).
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {
                if start.elapsed() > std::time::Duration::from_secs(30) {
                    let _ = child.kill();
                    log_indexer(&format!("{kind} extract TIMEOUT for {:?}", path));
                    break child.wait().ok();
                }
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            Err(_) => {
                let _ = child.kill();
                break child.wait().ok();
            }
        }
    };
    let text = reader.join().unwrap_or_default();
    if text.is_empty() {
        if !status.map(|s| s.success()).unwrap_or(false) {
            log_indexer(&format!(
                "{kind} extract produced no content (child crashed?) for {:?}",
                path
            ));
        }
        None
    } else {
        Some(text)
    }
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

fn extract_ocr_text(path: &Path) -> Option<String> {
    use windows::core::HSTRING;
    use windows::Graphics::Imaging::{
        BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, SoftwareBitmap,
    };
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::StorageFile;

    let path_str = path.to_str()?;
    let path_wide = HSTRING::from(path_str);

    let file = match StorageFile::GetFileFromPathAsync(&path_wide)
        .ok()
        .and_then(|async_op| async_op.get().ok())
    {
        Some(f) => f,
        None => {
            log_indexer(&format!(
                "OCR: Failed to get StorageFile for {:?}",
                path_str
            ));
            return None;
        }
    };

    let stream = match file
        .OpenAsync(windows::Storage::FileAccessMode::Read)
        .ok()
        .and_then(|async_op| async_op.get().ok())
    {
        Some(s) => s,
        None => {
            log_indexer(&format!("OCR: Failed to open stream for {:?}", path_str));
            return None;
        }
    };

    let decoder = match BitmapDecoder::CreateAsync(&stream)
        .ok()
        .and_then(|async_op| async_op.get().ok())
    {
        Some(d) => {
            // Enforce max image size for OCR to prevent OOM (#9)
            if d.PixelWidth().unwrap_or(9999) > 4000 || d.PixelHeight().unwrap_or(9999) > 4000 {
                return None;
            }
            d
        }
        None => return None,
    };

    // Get raw decoded bitmap (any pixel format)
    let raw_bitmap = match decoder
        .GetSoftwareBitmapAsync()
        .ok()
        .and_then(|async_op| async_op.get().ok())
    {
        Some(b) => b,
        None => {
            log_indexer(&format!(
                "OCR: Failed to get SoftwareBitmap for {:?}",
                path_str
            ));
            return None;
        }
    };

    // WinRT OCR requires Bgra8 + Premultiplied alpha — convert if needed
    let software_bitmap = {
        let fmt_ok = raw_bitmap.BitmapPixelFormat().ok() == Some(BitmapPixelFormat::Bgra8);
        let alpha_ok = raw_bitmap.BitmapAlphaMode().ok() == Some(BitmapAlphaMode::Premultiplied);
        if fmt_ok && alpha_ok {
            raw_bitmap
        } else {
            match SoftwareBitmap::ConvertWithAlpha(
                &raw_bitmap,
                BitmapPixelFormat::Bgra8,
                BitmapAlphaMode::Premultiplied,
            ) {
                Ok(converted) => converted,
                Err(e) => {
                    log_indexer(&format!(
                        "OCR: Bitmap conversion failed for {:?}: {:?}",
                        path_str, e
                    ));
                    return None;
                }
            }
        }
    };

    let ocr_engine = match OcrEngine::TryCreateFromUserProfileLanguages() {
        Ok(engine) => engine,
        Err(e) => {
            log_indexer(&format!("OCR: Failed to create OcrEngine: {:?}", e));
            return None;
        }
    };

    let ocr_result = match ocr_engine.RecognizeAsync(&software_bitmap).ok().and_then(
        |async_op: windows::Foundation::IAsyncOperation<windows::Media::Ocr::OcrResult>| {
            async_op.get().ok()
        },
    ) {
        Some(res) => res,
        None => {
            log_indexer(&format!("OCR: RecognizeAsync failed for {:?}", path_str));
            return None;
        }
    };

    let text = match ocr_result.Text() {
        Ok(t) => t.to_string(),
        Err(e) => {
            log_indexer(&format!("OCR: Failed to get text from result: {:?}", e));
            return None;
        }
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        log_indexer(&format!("OCR: No text found in {:?}", path_str));
        None
    } else {
        log_indexer(&format!(
            "OCR: Successfully extracted {} chars from {:?}",
            trimmed.len(),
            path_str
        ));
        Some(trimmed.to_string())
    }
}
