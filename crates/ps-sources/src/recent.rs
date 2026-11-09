//! Recently used files (Windows Recent folder + v2 launch history).
//! OWNER: sources-content.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::RecentEntry;

const MAX_RECENT: usize = 400;

pub fn scan(our_db: Option<&std::path::Path>) -> (Vec<RecentEntry>, Vec<String>) {
    let mut entries: Vec<RecentEntry> = Vec::new();
    let mut warnings = Vec::new();

    if let Some(recent_dir) = windows_recent_dir() {
        if recent_dir.is_dir() {
            match std::fs::read_dir(&recent_dir) {
                Ok(read_dir) => {
                    for entry in read_dir.take(MAX_RECENT * 4).flatten() {
                        if let Some(recent) = recent_entry_from_shortcut(&entry.path()) {
                            entries.push(recent);
                        }
                    }
                }
                Err(error) => warnings.push(format!("Windows Recent folder unreadable: {error}")),
            }
        }
    }

    if let Some(db) = our_db {
        match read_launch_history(db) {
            Ok(history) => entries.extend(history),
            Err(error) => warnings.push(error),
        }
    }

    // Dedupe by path, keeping the newest timestamp.
    let mut by_path: HashMap<String, RecentEntry> = HashMap::new();
    for entry in entries {
        match by_path.get(&entry.path) {
            Some(existing) if existing.last_used_ms >= entry.last_used_ms => {}
            _ => {
                by_path.insert(entry.path.clone(), entry);
            }
        }
    }
    let mut out: Vec<RecentEntry> = by_path.into_values().collect();
    out.sort_by(|a, b| b.last_used_ms.cmp(&a.last_used_ms));
    out.truncate(MAX_RECENT);
    (out, warnings)
}

fn recent_entry_from_shortcut(path: &Path) -> Option<RecentEntry> {
    if !path
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false)
    {
        return None;
    }
    let modified_ms = std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);
    let shortcut = ps_windows::resolve_shortcut(path).ok()?;
    let target = shortcut.target.filter(|target| target.is_file())?;
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| target.to_string_lossy().to_string());
    Some(RecentEntry {
        path: target.to_string_lossy().to_string(),
        name,
        last_used_ms: modified_ms,
    })
}

fn windows_recent_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| {
        PathBuf::from(base)
            .join("Microsoft")
            .join("Windows")
            .join("Recent")
    })
}

/// Read v2 launch history if the engine created the table. A missing table or
/// database is not an error: it simply means there is no v2 history yet.
fn read_launch_history(db: &Path) -> Result<Vec<RecentEntry>, String> {
    if !db.is_file() {
        return Ok(Vec::new());
    }
    let conn = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| format!("launch history unavailable: {error}"))?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
    let mut statement = match conn.prepare(
        "SELECT path, last_used_ms FROM launch_history ORDER BY last_used_ms DESC LIMIT ?1",
    ) {
        Ok(statement) => statement,
        // No launch_history table yet: not a warning-worthy condition.
        Err(_) => return Ok(Vec::new()),
    };
    let rows = statement
        .query_map([MAX_RECENT as i64], |row| {
            let path: String = row.get(0)?;
            let last_used_ms: i64 = row.get(1)?;
            Ok((path, last_used_ms))
        })
        .map_err(|error| format!("launch history query failed: {error}"))?;
    let mut entries = Vec::new();
    for row in rows.flatten() {
        let (path, last_used_ms) = row;
        if !Path::new(&path).exists() {
            continue;
        }
        let name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        entries.push(RecentEntry {
            path,
            name,
            last_used_ms,
        });
    }
    Ok(entries)
}
