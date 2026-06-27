use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "json", "html", "css", "c", "cpp", "h",
    "hpp", "cs", "go", "java", "kt", "sh", "bat", "ps1", "yaml", "yml", "toml", "ini", "sql",
    "xml", "rb", "php", "lua", "swift", "dart", "vue", "svelte", "csv", "tex", "rst", "adoc",
    "conf", "env",
];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif"];

fn is_indexable_content(ext: &str) -> bool {
    TEXT_EXTENSIONS.contains(&ext) || ext == "pdf" || ext == "docx" || IMAGE_EXTENSIONS.contains(&ext)
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
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(is_ignored_dir)
            .unwrap_or(false)
    })
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
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
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

/// Watch the user profile + fixed drives and index created/modified files instantly.
/// Falls back to the periodic crawl (safety net) for any events the OS drops.
pub fn start_watcher(db_path: PathBuf) {
    use notify::{EventKind, RecursiveMode, Watcher};
    thread::spawn(move || {
        // OCR (WinRT) needs COM initialized on this thread.
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
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
        for folder in get_scan_folders() {
            let _ = watcher.watch(&folder, RecursiveMode::Recursive);
        }
        let conn = match Connection::open(&db_path) {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        log_indexer("Watcher started");

        // Debounce: batch events in an ~800ms window and dedupe paths so a file being
        // written repeatedly (e.g. a download in progress) is only indexed once.
        let mut collect = |set: &mut std::collections::HashSet<PathBuf>, res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                if matches!(ev.kind, EventKind::Create(_) | EventKind::Modify(_)) {
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
            while let Ok(r) = rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())) {
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
    });
}

fn log_indexer(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let log_dir = match std::env::var("APPDATA") {
        Ok(d) => PathBuf::from(d).join("opensearch-os"),
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
        // Indexed within ~1 second of launch so common files are instantly searchable.
        thread::sleep(std::time::Duration::from_millis(500));
        log_indexer("Starting Phase 1 priority scan...");
        if let Err(e) = run_indexer_folders(&db_path_clone, get_priority_folders()) {
            log_indexer(&format!("Priority indexer error: {:?}", e));
            eprintln!("Priority indexer error: {:?}", e);
        }

        // ── Phase 2: Full crawl (entire user profile + other drives) ───────
        // Runs 10s after launch. (Skipped looping every 10 mins #7)
        thread::sleep(std::time::Duration::from_secs(10));
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

fn run_indexer_folders(db_path: &Path, folders: Vec<PathBuf>) -> anyhow::Result<()> {
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

    for folder in folders {
        log_indexer(&format!("Evaluating folder for index: {:?}", folder));
        if !folder.exists() {
            log_indexer(&format!("Folder does not exist, skipping: {:?}", folder));
            continue;
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

            let ext = if is_dir {
                "folder".to_string()
            } else {
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase()
            };

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let name = if name.is_empty() {
                path_str.clone()
            } else {
                name
            };

            if is_file && is_ignored_file(&name, &ext) {
                continue;
            }

            // seen_paths.insert(path_str.clone()); // skipped for memory (#8)

            let metadata = entry.metadata().ok();
            let modified = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let file_size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);

            let db_modified = {
                let mut stmt = conn
                    .prepare_cached("SELECT modified FROM files WHERE path = ?")
                    .unwrap();
                stmt.query_row([&path_str], |row| row.get::<_, i64>(0)).ok()
            };

            let is_text_or_doc = is_file
                && (TEXT_EXTENSIONS.contains(&ext.as_str()) || ext == "pdf" || ext == "docx");
            let is_image = is_file && IMAGE_EXTENSIONS.contains(&ext.as_str());
            let should_fts = is_text_or_doc || is_image;
            let needs_fts_check = should_fts && {
                let mut stmt = conn
                    .prepare_cached("SELECT 1 FROM files_fts WHERE path = ?")
                    .unwrap();
                !stmt.exists([&path_str]).unwrap_or(false)
            };

            if is_image {
                log_indexer(&format!(
                    "Found image in WalkDir: {} (modified={}, db_mod={:?}, needs_fts={})",
                    path_str, modified, db_modified, needs_fts_check
                ));
            }

            if db_modified.is_none() || db_modified.unwrap() != modified || needs_fts_check {
                let mut content = None;
                if is_file && should_fts {
                    if is_text_or_doc {
                        let is_pdf = ext == "pdf";
                        let is_docx = ext == "docx";

                        let extracted = if is_pdf {
                            safe_extract_pdf_text(path)
                        } else if is_docx {
                            safe_extract_docx_text(path)
                        } else {
                            read_text_file(path).ok()
                        };

                        content = Some(extracted.unwrap_or_default());

                        if is_pdf || is_docx {
                            thread::sleep(std::time::Duration::from_millis(50));
                        }
                    } else if is_image {
                        log_indexer(&format!("Extracting OCR text from image: {}", path_str));
                        let extracted = extract_ocr_text(path);
                        log_indexer(&format!(
                            "OCR finished for: {}. Text found: {:?}",
                            path_str, extracted
                        ));
                        content = Some(extracted.unwrap_or_default());
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                }

                pending_updates.push(PendingUpdate {
                    path: path_str,
                    name,
                    extension: ext,
                    modified,
                    size: file_size,
                    is_dir: if is_dir { 1 } else { 0 },
                    content,
                });

                if pending_updates.len() >= 1000 {
                    log_indexer("Flushing 1000 index updates to database...");
                    flush_updates(&mut conn, &mut pending_updates)?;
                }
            }

            // Yield CPU cycles after scanning every 1000 files
            file_count += 1;
            if file_count % 1000 == 0 {
                thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        log_indexer(&format!(
            "Finished WalkDir for {:?}: scanned {} total entries",
            folder, folder_file_count
        ));
    }

    log_indexer("Flushing remaining index updates to database...");
    flush_updates(&mut conn, &mut pending_updates)?;

    // Deleted files cleanup skipped to prevent memory bloat of seen_paths (#8)

    log_indexer("run_indexer_folders completed successfully");
    Ok(())
}

pub fn get_scan_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();

    let system_drive = std::env::var("SystemDrive")
        .unwrap_or_else(|_| "C:".to_string())
        .to_uppercase();

    // 1. Get the User Profile folder
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
            if drive_type == 3 {
                // 3 corresponds to DRIVE_FIXED in Win32
                folders.push(PathBuf::from(drive_path_str));
            }
        }
    }

    folders
}

fn safe_extract_pdf_text(path: &Path) -> Option<String> {
    let path_buf = path.to_path_buf();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        pdf_extract::extract_text(&path_buf)
    }));
    match result {
        Ok(Ok(text)) => {
            let mut truncated = text;
            truncated.truncate(50 * 1024);
            Some(truncated)
        }
        Ok(Err(e)) => {
            log_indexer(&format!("PDF extract error for {:?}: {:?}", path, e));
            None
        }
        Err(_) => {
            log_indexer(&format!("PDF extract PANICKED (caught) for {:?}", path));
            None
        }
    }
}

fn safe_extract_docx_text(path: &Path) -> Option<String> {
    let path_buf = path.to_path_buf();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        docx_lite::extract_text(&path_buf)
    }));
    match result {
        Ok(Ok(text)) => {
            let mut truncated = text;
            truncated.truncate(50 * 1024);
            Some(truncated)
        }
        Ok(Err(e)) => {
            log_indexer(&format!("DOCX extract error for {:?}: {:?}", path, e));
            None
        }
        Err(_) => {
            log_indexer(&format!("DOCX extract PANICKED (caught) for {:?}", path));
            None
        }
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
