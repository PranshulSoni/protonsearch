//! Clipboard history capture and search. OWNER: sources-content.
//!
//! v2 schema (inside the shared v2 database):
//!   clipboard_history(id INTEGER PK, content TEXT UNIQUE, timestamp INTEGER,
//!                     source_app TEXT, is_image INTEGER, pinned INTEGER,
//!                     ocr_text TEXT, image_path TEXT)
//!
//! Text items are captured by the Win32 clipboard listener (ps-windows); images
//! are written as PNG files and deduplicated by content hash. OCR text can be
//! attached later by the OCR pipeline.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::ClipboardEntry;

const MAX_TEXT_BYTES: usize = 1024 * 1024;
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS clipboard_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT UNIQUE,
    timestamp INTEGER NOT NULL,
    source_app TEXT NOT NULL DEFAULT '',
    is_image INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    ocr_text TEXT,
    image_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_clipboard_timestamp
    ON clipboard_history(timestamp DESC);
";

/// Start the Win32 clipboard listener. Captured items are written to the v2
/// database; images are stored as PNG files under image_dir.
pub fn start_listener(
    db_path: PathBuf,
    image_dir: PathBuf,
) -> Result<ps_windows::ClipboardListener> {
    let listener = ps_windows::ClipboardListener::start(move |snapshot| {
        let source = snapshot.source_app.clone().unwrap_or_default();
        if let Some(text) = snapshot.text.as_deref() {
            let _ = record(&db_path, text, &source);
        }
        if let Some(png) = snapshot.png.as_deref() {
            let _ = record_image(&db_path, png, &source, &image_dir);
        }
    })?;
    Ok(listener)
}

/// Insert a captured clipboard item; consecutive identical text only refreshes
/// the timestamp (returns false). Empty/oversized text is ignored.
pub fn record(db_path: &Path, text: &str, source_app: &str) -> Result<bool> {
    if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
        return Ok(false);
    }
    let conn = open_or_create(db_path)?;
    let timestamp = now_ms();
    let previous: Option<String> = conn
        .query_row(
            "SELECT content FROM clipboard_history
             WHERE pinned = 0 AND is_image = 0
             ORDER BY timestamp DESC, id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();
    if previous.as_deref() == Some(text) {
        conn.execute(
            "UPDATE clipboard_history SET timestamp = ?1 WHERE content = ?2",
            rusqlite::params![timestamp, text],
        )?;
        return Ok(false);
    }
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO clipboard_history(content, timestamp, source_app, is_image, pinned)
         VALUES(?1, ?2, ?3, 0, 0)",
        rusqlite::params![text, timestamp, source_app],
    )?;
    Ok(inserted > 0)
}

/// Store a clipboard image as PNG (deduplicated by content hash).
pub fn record_image(
    db_path: &Path,
    png_bytes: &[u8],
    source_app: &str,
    image_dir: &Path,
) -> Result<bool> {
    if png_bytes.is_empty() || png_bytes.len() > 32 * 1024 * 1024 {
        return Ok(false);
    }
    let hash = format!("{:016x}", fnv1a(png_bytes));
    std::fs::create_dir_all(image_dir)
        .with_context(|| format!("creating clipboard image dir {}", image_dir.display()))?;
    let image_path = image_dir.join(format!("{hash}.png"));
    if !image_path.is_file() {
        std::fs::write(&image_path, png_bytes)
            .with_context(|| format!("writing clipboard image {}", image_path.display()))?;
    }
    let conn = open_or_create(db_path)?;
    let content = format!("[image:{hash}]");
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO clipboard_history(content, timestamp, source_app, is_image, pinned, image_path)
         VALUES(?1, ?2, ?3, 1, 0, ?4)",
        rusqlite::params![
            content,
            now_ms(),
            source_app,
            image_path.to_string_lossy().to_string()
        ],
    )?;
    Ok(inserted > 0)
}

/// Search clipboard text (case-insensitive substring, pinned first then newest).
pub fn search(db_path: &Path, query: &str, limit: usize) -> Vec<ClipboardEntry> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return list_recent(db_path, limit);
    }
    let Ok(conn) = open_existing(db_path) else {
        return Vec::new();
    };
    let pattern = format!("%{}%", escape_like(trimmed));
    let mut statement = match conn.prepare(
        "SELECT id, content, timestamp, source_app, pinned FROM clipboard_history
         WHERE content LIKE ?1 ESCAPE '\\' AND is_image = 0
         ORDER BY pinned DESC, timestamp DESC LIMIT ?2",
    ) {
        Ok(statement) => statement,
        Err(_) => return Vec::new(),
    };
    let rows = statement.query_map(rusqlite::params![pattern, limit as i64], |row| {
        Ok(ClipboardEntry {
            id: row.get(0)?,
            text: row.get(1)?,
            timestamp_ms: row.get(2)?,
            source_app: row.get(3)?,
            pinned: row.get::<_, i64>(4)? != 0,
        })
    });
    match rows {
        Ok(rows) => rows.flatten().collect(),
        Err(_) => Vec::new(),
    }
}

/// Newest clipboard items (pinned first).
pub fn list_recent(db_path: &Path, limit: usize) -> Vec<ClipboardEntry> {
    let Ok(conn) = open_existing(db_path) else {
        return Vec::new();
    };
    let mut statement = match conn.prepare(
        "SELECT id, content, timestamp, source_app, pinned FROM clipboard_history
         ORDER BY pinned DESC, timestamp DESC LIMIT ?1",
    ) {
        Ok(statement) => statement,
        Err(_) => return Vec::new(),
    };
    let rows = statement.query_map([limit as i64], |row| {
        Ok(ClipboardEntry {
            id: row.get(0)?,
            text: row.get(1)?,
            timestamp_ms: row.get(2)?,
            source_app: row.get(3)?,
            pinned: row.get::<_, i64>(4)? != 0,
        })
    });
    match rows {
        Ok(rows) => rows.flatten().collect(),
        Err(_) => Vec::new(),
    }
}

pub fn set_pinned(db_path: &Path, id: i64, pinned: bool) -> Result<bool> {
    let conn = open_or_create(db_path)?;
    let changed = conn.execute(
        "UPDATE clipboard_history SET pinned = ?1 WHERE id = ?2",
        rusqlite::params![if pinned { 1i64 } else { 0i64 }, id],
    )?;
    Ok(changed > 0)
}

pub fn delete(db_path: &Path, id: i64) -> Result<bool> {
    let conn = open_or_create(db_path)?;
    let changed = conn.execute("DELETE FROM clipboard_history WHERE id = ?1", [id])?;
    Ok(changed > 0)
}

/// Best-effort import of the legacy v1 clipboard_history table (read-only).
/// The legacy table was empty at baseline, so a zero result is normal.
pub fn import_legacy_clipboard(db_path: &Path, legacy_db: &Path) -> Result<usize> {
    if !legacy_db.is_file() {
        return Ok(0);
    }
    let conn = open_or_create(db_path)?;
    let uri = read_only_uri(legacy_db)?;
    conn.execute("ATTACH DATABASE ?1 AS legacy", rusqlite::params![uri])
        .with_context(|| format!("attaching legacy clipboard source {}", legacy_db.display()))?;
    let result = (|| -> Result<usize> {
        let has_table: i64 = conn.query_row(
            "SELECT COUNT(*) FROM legacy.sqlite_master WHERE type = 'table' AND name = 'clipboard_history'",
            [],
            |row| row.get(0),
        )?;
        if has_table == 0 {
            return Ok(0);
        }
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO clipboard_history(content, timestamp, source_app, is_image, pinned)
             SELECT content, timestamp,
                    CASE WHEN source_app IS NULL OR source_app = '' THEN 'legacy' ELSE source_app END,
                    COALESCE(is_image, 0), COALESCE(pinned, 0)
             FROM legacy.clipboard_history
             WHERE content IS NOT NULL AND content != ''",
            [],
        )?;
        Ok(inserted)
    })();
    let _ = conn.execute_batch("DETACH DATABASE legacy");
    let imported = result?;
    Ok(imported)
}

/// Best-effort import of Windows clipboard history files if the OS exposes a
/// readable pinned-items folder. Returns the number of text items imported.
pub fn import_windows_history(db_path: &Path) -> Result<usize> {
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return Ok(0);
    };
    let dir = PathBuf::from(local)
        .join("Microsoft")
        .join("Windows")
        .join("Clipboard");
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut imported = 0usize;
    let Ok(read_dir) = std::fs::read_dir(&dir) else {
        return Ok(0);
    };
    for entry in read_dir.flatten().take(500) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.len() == 0 || metadata.len() > 256 * 1024 {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let text = text.trim_end_matches(['\0', '\n', '\r']).to_string();
        if text.trim().is_empty() {
            continue;
        }
        if record(db_path, &text, "Windows Clipboard")? {
            imported += 1;
        }
    }
    Ok(imported)
}

fn open_existing(db_path: &Path) -> Result<rusqlite::Connection> {
    if !db_path.is_file() {
        anyhow::bail!("database does not exist: {}", db_path.display());
    }
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
    Ok(conn)
}

fn open_or_create(db_path: &Path) -> Result<rusqlite::Connection> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    let conn = rusqlite::Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    conn.execute_batch(SCHEMA)
        .context("migrating clipboard schema")?;
    Ok(conn)
}

fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn read_only_uri(path: &Path) -> Result<String> {
    let absolute = path
        .canonicalize()
        .with_context(|| format!("resolving {}", path.display()))?;
    let raw = absolute.to_string_lossy();
    let raw = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
    let text = raw.replace('\\', "/");
    let mut encoded = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            ' ' => encoded.push_str("%20"),
            '#' => encoded.push_str("%23"),
            '?' => encoded.push_str("%3F"),
            '%' => encoded.push_str("%25"),
            other => encoded.push(other),
        }
    }
    if encoded.starts_with("//") {
        Ok(format!("file:{encoded}?mode=ro"))
    } else {
        Ok(format!("file:///{encoded}?mode=ro"))
    }
}
