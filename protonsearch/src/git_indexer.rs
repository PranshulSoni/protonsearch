use crate::indexer::get_scan_folders;
use crate::search::{ensure_memory_events_schema, insert_memory_event};
use rusqlite::{params, Connection};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use walkdir::WalkDir;

/// Recursively find top-level git repos under `dir`, pruning when a repo is found.
/// This avoids descending into submodules or deeply nested build dirs.
fn find_repos_recursive(dir: &Path, found: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    // If this dir IS a git repo, add it and stop (don't recurse into submodules)
    if dir.join(".git").exists() {
        found.push(dir.to_path_buf());
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let is_dir = match entry.file_type() {
            Ok(ft) => {
                if ft.is_symlink() {
                    false
                } else {
                    ft.is_dir()
                }
            }
            Err(_) => false,
        };
        if !is_dir {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        // Skip heavy non-source directories
        match name.as_str() {
            "node_modules" | "target" | ".git" | "dist" | "build" | "venv" | ".venv"
            | "appdata" | "obj" | "out" | ".next" | ".nuxt" | ".cache" | "cache" => continue,
            _ => {}
        }
        find_repos_recursive(&entry.path(), found, depth + 1);
    }
}

fn log_git(msg: &str) {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let log_path = std::path::PathBuf::from(appdata)
            .join("protonsearch")
            .join("git_indexer.log");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            if f.metadata().map(|m| m.len() > 1024 * 1024).unwrap_or(false) {
                let _ = f.set_len(0);
            }
            let _ = writeln!(f, "{msg}");
        }
    }
}

pub fn start_git_indexer(db_path: PathBuf) {
    thread::spawn(move || {
        // Set low priority to run strictly in the background without affecting foreground apps
        unsafe {
            use windows::Win32::System::Threading::{
                GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
            };
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
        }
        log_git("[start_git_indexer] thread started");
        // Initial delay: 30s so the heavy file indexer, watcher, and search engine
        // all finish their initial I/O burst before git starts hitting the disk.
        thread::sleep(std::time::Duration::from_secs(30));
        loop {
            log_git("[run_git_indexer] starting run");
            if let Err(e) = run_git_indexer(&db_path) {
                let msg = format!("[run_git_indexer] ERROR: {:?}", e);
                log_git(&msg);
                eprintln!("{msg}");
            } else {
                log_git("[run_git_indexer] completed successfully");
            }
            // Re-scan every 15 minutes
            thread::sleep(std::time::Duration::from_secs(900));
        }
    });
}

fn run_git_indexer(db_path: &Path) -> anyhow::Result<()> {
    let mut conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode=WAL;")?;
    let _ = ensure_memory_events_schema(&conn);

    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS git_repos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT UNIQUE,
            name TEXT NOT NULL
        );",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS git_commits (
            hash TEXT PRIMARY KEY,
            repo_id INTEGER,
            author TEXT NOT NULL,
            date INTEGER NOT NULL,
            message TEXT NOT NULL,
            FOREIGN KEY(repo_id) REFERENCES git_repos(id) ON DELETE CASCADE
        );",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS git_branches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id INTEGER,
            name TEXT NOT NULL,
            is_head INTEGER NOT NULL,
            UNIQUE(repo_id, name),
            FOREIGN KEY(repo_id) REFERENCES git_repos(id) ON DELETE CASCADE
        );",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS git_todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id INTEGER,
            file_path TEXT NOT NULL,
            line_number INTEGER NOT NULL,
            todo_text TEXT NOT NULL,
            FOREIGN KEY(repo_id) REFERENCES git_repos(id) ON DELETE CASCADE
        );",
        [],
    )?;

    // Step 1: Find top-level git repositories (pruning at each repo to skip submodules)
    let folders = get_scan_folders();
    log_git(&format!("[step1] scanning {} folders", folders.len()));
    let mut found_repos = Vec::new();

    for folder in &folders {
        log_git(&format!("[step1] walking: {:?}", folder));
        if !folder.exists() {
            continue;
        }
        find_repos_recursive(folder, &mut found_repos, 0);
        log_git(&format!(
            "[step1] done walking {:?}, found {} repos so far",
            folder,
            found_repos.len()
        ));
    }

    log_git(&format!(
        "[step1] total repos to index: {}",
        found_repos.len()
    ));
    for r in &found_repos {
        log_git(&format!("[step1] repo: {:?}", r));
    }

    // Step 2: Index repositories
    let mut active_repo_ids = Vec::new();
    for repo_path in found_repos {
        let repo_path_str = match repo_path.to_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Repo")
            .to_string();

        let _ = conn.execute(
            "INSERT OR IGNORE INTO git_repos (path, name) VALUES (?, ?)",
            params![repo_path_str, repo_name],
        );

        let repo_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM git_repos WHERE path = ?",
                [&repo_path_str],
                |row| row.get(0),
            )
            .ok();

        if let Some(r_id) = repo_id {
            active_repo_ids.push(r_id);
            if let Err(e) = index_single_repo(&mut conn, r_id, &repo_path) {
                eprintln!("Error indexing repo {:?}: {:?}", repo_path, e);
            }
        }
    }

    // Step 3: Delete repos that no longer exist.
    // Collect rows first (explicit loop) to release the stmt borrow before writing.
    let stale_ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id, path FROM git_repos")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut ids = Vec::new();
        for row in rows.flatten() {
            let (r_id, r_path) = row;
            if !active_repo_ids.contains(&r_id) || !Path::new(&r_path).exists() {
                ids.push(r_id);
            }
        }
        ids
    };
    for r_id in stale_ids {
        let _ = conn.execute("DELETE FROM git_repos WHERE id = ?", [r_id]);
    }

    Ok(())
}

fn get_git_executable() -> std::ffi::OsString {
    // 1. Try PATH environment variable
    if let Ok(paths) = std::env::var("PATH") {
        for path in std::env::split_paths(&paths) {
            let git_path = path.join("git.exe");
            if git_path.exists() {
                return git_path.into_os_string();
            }
        }
    }
    // 2. Try standard Windows paths
    let standard_paths = [
        "C:\\Program Files\\Git\\cmd\\git.exe",
        "C:\\Program Files\\Git\\bin\\git.exe",
        "C:\\Program Files (x86)\\Git\\cmd\\git.exe",
        "C:\\Program Files (x86)\\Git\\bin\\git.exe",
    ];
    for p in &standard_paths {
        let path = std::path::Path::new(p);
        if path.exists() {
            return path.to_path_buf().into_os_string();
        }
    }
    // 3. Try AppData Local
    if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
        let user_git = std::path::Path::new(&appdata)
            .join("Programs")
            .join("Git")
            .join("cmd")
            .join("git.exe");
        if user_git.exists() {
            return user_git.into_os_string();
        }
        let user_git_bin = std::path::Path::new(&appdata)
            .join("Programs")
            .join("Git")
            .join("bin")
            .join("git.exe");
        if user_git_bin.exists() {
            return user_git_bin.into_os_string();
        }
    }
    // Fallback to "git"
    std::ffi::OsString::from("git")
}

fn index_single_repo(conn: &mut Connection, repo_id: i64, repo_path: &Path) -> anyhow::Result<()> {
    let git_exe = get_git_executable();

    // 1. Collect branches in memory
    let mut branches = Vec::new();
    let branch_output = Command::new(&git_exe)
        .args(["branch", "--no-color"])
        .current_dir(repo_path)
        .creation_flags(0x08000000)
        .output();

    if let Ok(out) = branch_output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (is_head, name) = if line.starts_with('*') {
                (1, line.strip_prefix('*').unwrap().trim().to_string())
            } else {
                (0, line.to_string())
            };
            branches.push((name, is_head));
        }
    }

    // 2. Collect recent commits in memory (up to 100)
    let mut commits = Vec::new();
    let log_output = Command::new(&git_exe)
        .args(["log", "--max-count=100", "--format=%H%x1F%an%x1F%at%x1F%s"])
        .current_dir(repo_path)
        .creation_flags(0x08000000)
        .output();

    if let Ok(out) = log_output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split('\x1F').collect();
            if parts.len() < 4 {
                continue;
            }
            let hash = parts[0].to_string();
            let author = parts[1].to_string();
            let date = parts[2].parse::<i64>().unwrap_or(0);
            let message = parts[3].to_string();
            commits.push((hash, author, date, message));
        }
    }

    // 3. Scan TODOs in memory
    let mut todos = Vec::new();
    let walker = WalkDir::new(repo_path).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy().to_lowercase();
        if name == "node_modules"
            || name == "target"
            || name == "build"
            || name == "dist"
            || name == "venv"
            || name == ".venv"
            || name == ".git"
            || name == "appdata"
            || name == "obj"
            || name == "bin"
            || name == "out"
            || name == ".next"
            || name == ".nuxt"
            || name == ".cache"
            || name == "cache"
        {
            return false;
        }
        if e.depth() > 0 && e.path().join(".git").exists() {
            return false;
        }
        true
    });

    let allowed_extensions = [
        "rs", "py", "js", "ts", "go", "cpp", "c", "h", "java", "kt", "cs", "md", "txt",
    ];

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !allowed_extensions.contains(&ext.as_str()) {
            continue;
        }

        // Skip files larger than 1MB
        if let Ok(meta) = entry.metadata() {
            if meta.len() > 1024 * 1024 {
                continue;
            }
        }

        let path_str = match path.to_str() {
            Some(s) => s.to_string(),
            None => continue,
        };

        if let Ok(content) = std::fs::read_to_string(path) {
            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if (trimmed.starts_with("//")
                    || trimmed.starts_with("#")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with("--")
                    || trimmed.starts_with("<!--"))
                    && (trimmed.contains("TODO")
                        || trimmed.contains("FIXME")
                        || trimmed.contains("BUG"))
                {
                    let line_number = (idx + 1) as i32;
                    todos.push((path_str.clone(), line_number, trimmed.to_string()));
                }
            }
        }
    }

    // 4. Perform database updates in a short-lived transaction
    let tx = conn.transaction()?;

    tx.execute("DELETE FROM git_branches WHERE repo_id = ?", [repo_id])?;
    for (name, is_head) in branches {
        tx.execute(
            "INSERT OR REPLACE INTO git_branches (repo_id, name, is_head) VALUES (?, ?, ?)",
            params![repo_id, name, is_head],
        )?;
    }

    tx.execute("DELETE FROM git_commits WHERE repo_id = ?", [repo_id])?;
    for (hash, author, date, message) in &commits {
        tx.execute(
            "INSERT OR REPLACE INTO git_commits (hash, repo_id, author, date, message) VALUES (?, ?, ?, ?, ?)",
            params![hash, repo_id, author, date, message],
        )?;
    }

    tx.execute("DELETE FROM git_todos WHERE repo_id = ?", [repo_id])?;
    for (path_str, line_number, todo_text) in todos {
        tx.execute(
            "INSERT INTO git_todos (repo_id, file_path, line_number, todo_text) VALUES (?, ?, ?, ?)",
            params![repo_id, path_str, line_number, todo_text],
        )?;
    }

    tx.commit()?;

    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown Repo");
    let repo_path_str = repo_path.to_string_lossy();
    for (hash, author, date, message) in commits {
        if date <= 0 {
            continue;
        }
        insert_memory_event(
            conn,
            date,
            "Git",
            "Commit",
            &format!("Commit: {}", message),
            &format!("{} by {} in {}", hash, author, repo_name),
            repo_name,
            Some(&repo_path_str),
            None,
        );
    }
    Ok(())
}
