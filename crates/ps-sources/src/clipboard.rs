//! Clipboard history capture, classification and search. OWNER: sources-content.
//!
//! v2 schema (inside the shared v2 database):
//!   clipboard_history(id INTEGER PK, content TEXT UNIQUE, timestamp INTEGER,
//!                     source_app TEXT, is_image INTEGER, pinned INTEGER,
//!                     ocr_text TEXT, image_path TEXT, category TEXT)
//!
//! Text items are captured by the Win32 clipboard listener (ps-windows); images
//! are stored as PNG files and deduplicated by content hash. Every row carries a
//! coarse category (link/email/path/code/image/text) used by the launcher.
//!
//! Windows 11 does NOT expose a documented, plain-text readable clipboard-history
//! store, so history is captured live from now on. The extracted-data folder
//! %LOCALAPPDATA%\Microsoft\Windows\Clipboard is not a history database (the old
//! code scanned it as UTF-8 text and always imported 0 items); that scan has been
//! removed. Legacy v1 ProtonSearch clipboard rows can still be imported once.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::{ClipCategory, ClipboardEntry};

const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
/// Classifiers look at the first 16 KiB only (cheap, deterministic).
const CLASSIFY_SAMPLE_BYTES: usize = 16 * 1024;
/// Backfill bound when migrating/importing rows.
const MAX_BACKFILL_ROWS: usize = 10_000;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS clipboard_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT UNIQUE,
    timestamp INTEGER NOT NULL,
    source_app TEXT NOT NULL DEFAULT '',
    is_image INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    ocr_text TEXT,
    image_path TEXT,
    category TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_clipboard_timestamp
    ON clipboard_history(timestamp DESC);
";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Start the Win32 clipboard listener and seed the current clipboard first.
/// Captured items are written to the v2 database; images are stored as PNG files
/// under image_dir. Only one listener may run per process.
pub fn start_listener(
    db_path: PathBuf,
    image_dir: PathBuf,
) -> Result<ps_windows::ClipboardListener> {
    // Seed immediately so a fresh history is never empty, even before the first
    // clipboard change arrives. Errors are non-fatal for listener startup.
    let _ = capture_current(&db_path, &image_dir);
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

/// Read the CURRENT OS clipboard once (text preferred, then image) and store it.
/// Returns the stored entry so callers can update the UI immediately.
pub fn capture_current(db_path: &Path, image_dir: &Path) -> Result<Option<ClipboardEntry>> {
    let source_app = ps_windows::foreground_process_name().unwrap_or_default();
    let content = if let Some(text) = ps_windows::get_clipboard_text() {
        if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
            None
        } else {
            let category = classify(&text);
            insert_text(db_path, &text, &source_app, category)?;
            Some(text)
        }
    } else if let Some(png) = ps_windows::get_clipboard_image_png() {
        if png.is_empty() || png.len() > MAX_IMAGE_BYTES {
            None
        } else {
            let (hash, _path) = write_image_file(image_dir, &png)?;
            let content = format!("[image:{hash}]");
            insert_image(db_path, &content, &source_app, &png_hash_path(image_dir, &hash))?;
            Some(content)
        }
    } else {
        None
    };
    match content {
        Some(content) => entry_by_content(db_path, &content),
        None => Ok(None),
    }
}

/// Number of stored clipboard items (pinned and images included). 0 when the
/// database/table does not exist yet; never errors.
pub fn count(db_path: &Path) -> u64 {
    let Ok(conn) = open_existing(db_path) else {
        return 0;
    };
    conn.query_row("SELECT COUNT(*) FROM clipboard_history", [], |row| {
        row.get::<_, i64>(0)
    })
    .map(|count| count.max(0) as u64)
    .unwrap_or(0)
}

/// Insert a captured clipboard item; consecutive identical text only refreshes
/// the timestamp (returns false). Empty/oversized text is ignored. The content
/// category is computed and stored automatically.
pub fn record(db_path: &Path, text: &str, source_app: &str) -> Result<bool> {
    if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
        return Ok(false);
    }
    let category = classify(text);
    Ok(insert_text(db_path, text, source_app, category)?.is_some())
}

/// Store a clipboard image as PNG (deduplicated by content hash), category Image.
pub fn record_image(
    db_path: &Path,
    png_bytes: &[u8],
    source_app: &str,
    image_dir: &Path,
) -> Result<bool> {
    if png_bytes.is_empty() || png_bytes.len() > MAX_IMAGE_BYTES {
        return Ok(false);
    }
    let (hash, image_path) = write_image_file(image_dir, png_bytes)?;
    let content = format!("[image:{hash}]");
    insert_image(db_path, &content, source_app, &image_path)
}

/// Deterministic content classifier used when a clipboard item is stored.
///
/// Rules (checked in order): URL (http/https/ftp/mailto, incl. www.) -> Link;
/// single-token email address -> Email; Windows drive/UNC or ./ relative path ->
/// Path; code-like content (code tokens, JSON, SQL, indented multi-line) -> Code;
/// otherwise Text. Image rows are classified separately in record_image.
pub fn classify(text: &str) -> ClipCategory {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ClipCategory::Text;
    }
    let sample = take_sample(trimmed);
    if is_link(sample) {
        return ClipCategory::Link;
    }
    if is_email(sample) {
        return ClipCategory::Email;
    }
    if is_path(sample) {
        return ClipCategory::Path;
    }
    if is_code(sample) {
        return ClipCategory::Code;
    }
    ClipCategory::Text
}

/// Search clipboard text (case-insensitive substring, pinned first then newest).
/// Images are excluded; an empty query lists recent items.
pub fn search(db_path: &Path, query: &str, limit: usize) -> Vec<ClipboardEntry> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return list_recent(db_path, limit);
    }
    let Ok((conn, has_category)) = open_existing_with_category(db_path) else {
        return Vec::new();
    };
    let pattern = format!("%{}%", escape_like(trimmed));
    let category_column = if has_category { "category" } else { "''" };
    let sql = format!(
        "SELECT id, content, timestamp, source_app, pinned, is_image, {category_column}
         FROM clipboard_history
         WHERE content LIKE ?1 ESCAPE '\\' AND is_image = 0
         ORDER BY pinned DESC, timestamp DESC, id DESC LIMIT ?2"
    );
    let mut statement = match conn.prepare(&sql) {
        Ok(statement) => statement,
        Err(_) => return Vec::new(),
    };
    let rows = statement.query_map(rusqlite::params![pattern, limit as i64], |row| {
        map_entry(row)
    });
    match rows {
        Ok(rows) => rows.flatten().collect(),
        Err(_) => Vec::new(),
    }
}

/// Newest clipboard items (pinned first, then newest).
pub fn list_recent(db_path: &Path, limit: usize) -> Vec<ClipboardEntry> {
    let Ok((conn, has_category)) = open_existing_with_category(db_path) else {
        return Vec::new();
    };
    let category_column = if has_category { "category" } else { "''" };
    let sql = format!(
        "SELECT id, content, timestamp, source_app, pinned, is_image, {category_column}
         FROM clipboard_history
         ORDER BY pinned DESC, timestamp DESC, id DESC LIMIT ?1"
    );
    let mut statement = match conn.prepare(&sql) {
        Ok(statement) => statement,
        Err(_) => return Vec::new(),
    };
    let rows = statement.query_map([limit as i64], |row| map_entry(row));
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
        // Imported rows have no category yet: classify them best-effort.
        let _ = backfill_categories(&conn);
        Ok(inserted)
    })();
    let _ = conn.execute_batch("DETACH DATABASE legacy");
    result
}

// ---------------------------------------------------------------------------
// Storage internals
// ---------------------------------------------------------------------------

fn insert_text(
    db_path: &Path,
    text: &str,
    source_app: &str,
    category: ClipCategory,
) -> Result<Option<i64>> {
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
        // Same consecutive item: refresh freshness/category, no new row.
        conn.execute(
            "UPDATE clipboard_history SET timestamp = ?1, category = ?2 WHERE content = ?3",
            rusqlite::params![timestamp, category.as_str(), text],
        )?;
        return Ok(None);
    }
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO clipboard_history
            (content, timestamp, source_app, is_image, pinned, category)
         VALUES(?1, ?2, ?3, 0, 0, ?4)",
        rusqlite::params![text, timestamp, source_app, category.as_str()],
    )?;
    if inserted > 0 {
        Ok(Some(conn.last_insert_rowid()))
    } else {
        // Content already exists (earlier copy, possibly pinned): refresh its
        // category/timestamp so it surfaces as recent.
        let _ = conn.execute(
            "UPDATE clipboard_history SET category = ?1 WHERE content = ?2 AND category = ''",
            rusqlite::params![category.as_str(), text],
        );
        Ok(None)
    }
}

fn insert_image(
    db_path: &Path,
    content: &str,
    source_app: &str,
    image_path: &Path,
) -> Result<bool> {
    let conn = open_or_create(db_path)?;
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO clipboard_history
            (content, timestamp, source_app, is_image, pinned, image_path, category)
         VALUES(?1, ?2, ?3, 1, 0, ?4, ?5)",
        rusqlite::params![
            content,
            now_ms(),
            source_app,
            image_path.to_string_lossy().to_string(),
            ClipCategory::Image.as_str()
        ],
    )?;
    if inserted == 0 {
        // Existing image row from an older build: upgrade its category.
        let _ = conn.execute(
            "UPDATE clipboard_history SET category = ?1 WHERE content = ?2 AND category = ''",
            rusqlite::params![ClipCategory::Image.as_str(), content],
        );
    }
    Ok(inserted > 0)
}

fn write_image_file(image_dir: &Path, png_bytes: &[u8]) -> Result<(String, PathBuf)> {
    let hash = format!("{:016x}", fnv1a(png_bytes));
    std::fs::create_dir_all(image_dir)
        .with_context(|| format!("creating clipboard image dir {}", image_dir.display()))?;
    let image_path = image_dir.join(format!("{hash}.png"));
    if !image_path.is_file() {
        std::fs::write(&image_path, png_bytes)
            .with_context(|| format!("writing clipboard image {}", image_path.display()))?;
    }
    Ok((hash, image_path))
}

fn png_hash_path(image_dir: &Path, hash: &str) -> PathBuf {
    image_dir.join(format!("{hash}.png"))
}

fn entry_by_content(db_path: &Path, content: &str) -> Result<Option<ClipboardEntry>> {
    let Ok((conn, has_category)) = open_existing_with_category(db_path) else {
        return Ok(None);
    };
    let category_column = if has_category { "category" } else { "''" };
    let sql = format!(
        "SELECT id, content, timestamp, source_app, pinned, is_image, {category_column}
         FROM clipboard_history WHERE content = ?1"
    );
    let mut statement = conn.prepare(&sql)?;
    let mut rows = statement.query([content])?;
    match rows.next()? {
        Some(row) => Ok(Some(map_entry(row)?)),
        None => Ok(None),
    }
}

/// Map a row selected as (id, content, timestamp, source_app, pinned, is_image,
/// category) into a ClipboardEntry; missing/empty categories are derived.
fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    let id: i64 = row.get(0)?;
    let content: String = row.get(1)?;
    let timestamp_ms: i64 = row.get(2)?;
    let source_app: String = row.get(3)?;
    let pinned = row.get::<_, i64>(4)? != 0;
    let is_image = row.get::<_, i64>(5)? != 0;
    let stored: String = row.get(6).unwrap_or_default();
    let category = ClipCategory::from_db(&stored).unwrap_or_else(|| {
        if is_image {
            ClipCategory::Image
        } else {
            classify(&content)
        }
    });
    Ok(ClipboardEntry {
        id,
        text: content,
        timestamp_ms,
        source_app,
        pinned,
        category,
    })
}

// ---------------------------------------------------------------------------
// Connection/schema helpers
// ---------------------------------------------------------------------------

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

fn open_existing_with_category(db_path: &Path) -> Result<(rusqlite::Connection, bool)> {
    let conn = open_existing(db_path)?;
    let has_category = has_column(&conn, "category")?;
    Ok((conn, has_category))
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
    migrate(&conn)?;
    Ok(conn)
}

/// Create the current schema and add the category column to older databases.
fn migrate(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)
        .context("migrating clipboard schema")?;
    if !has_column(conn, "category")? {
        conn.execute_batch(
            "ALTER TABLE clipboard_history ADD COLUMN category TEXT NOT NULL DEFAULT ''",
        )
        .context("adding clipboard_history.category column")?;
        let _ = backfill_categories(conn);
    }
    Ok(())
}

fn has_column(conn: &rusqlite::Connection, column: &str) -> Result<bool> {
    let mut statement = conn.prepare("PRAGMA table_info(clipboard_history)")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name.eq_ignore_ascii_case(column) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Classify rows that have no category yet (bounded, idempotent).
fn backfill_categories(conn: &rusqlite::Connection) -> Result<usize> {
    let rows: Vec<(i64, String, i64)> = {
        let mut statement = conn.prepare(
            "SELECT id, content, is_image FROM clipboard_history
             WHERE category IS NULL OR category = '' LIMIT ?1",
        )?;
        let rows = statement.query_map([MAX_BACKFILL_ROWS as i64], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.flatten().collect()
    };
    let mut updated = 0usize;
    for (id, content, is_image) in rows {
        let category = if is_image != 0 {
            ClipCategory::Image
        } else {
            classify(&content)
        };
        updated += conn.execute(
            "UPDATE clipboard_history SET category = ?1 WHERE id = ?2",
            rusqlite::params![category.as_str(), id],
        )?;
    }
    Ok(updated)
}

// ---------------------------------------------------------------------------
// Classification rules
// ---------------------------------------------------------------------------

fn take_sample(text: &str) -> &str {
    if text.len() <= CLASSIFY_SAMPLE_BYTES {
        return text;
    }
    let mut end = CLASSIFY_SAMPLE_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// URL: single token with a supported scheme (http/https/ftp/mailto) or www.
fn is_link(text: &str) -> bool {
    if text.contains(char::is_whitespace) || text.lines().count() > 1 {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    for scheme in ["http://", "https://", "ftp://", "mailto:"] {
        if lower.starts_with(scheme) && lower.len() > scheme.len() + 1 {
            return true;
        }
    }
    if let Some(rest) = lower.strip_prefix("www.") {
        return rest.len() > 3 && rest.contains('.');
    }
    false
}

/// Email: single token local@domain.tld with no whitespace.
fn is_email(text: &str) -> bool {
    if text.contains(char::is_whitespace) || text.lines().count() > 1 {
        return false;
    }
    if text.len() > 320 {
        return false;
    }
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    if local.is_empty()
        || domain.is_empty()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.contains("..")
    {
        return false;
    }
    let valid_local = local.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '%' | '+' | '-' | '\'' )
    });
    let valid_domain = domain
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'));
    valid_local && valid_domain
}

/// Path: Windows drive path, UNC path, or ./ ../ relative path (single line).
fn is_path(text: &str) -> bool {
    if text.lines().count() > 1 || text.len() > 4096 {
        return false;
    }
    let bytes = text.as_bytes();
    if text.starts_with("./")
        || text.starts_with(".\\")
        || text.starts_with("../")
        || text.starts_with("..\\")
    {
        return text.len() > 2;
    }
    if text.starts_with("\\\\") {
        // UNC: \\server\share
        return text.len() > 4 && text[2..].contains('\\');
    }
    // C:\... or C:/...
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    // Extended-length path prefix \\?\
    text.starts_with("\\\\?\\")
}

/// Code heuristic: code tokens, JSON/SQL shapes, or indented multi-line text.
fn is_code(text: &str) -> bool {
    let lines = text.lines().count();
    let lower = text.to_ascii_lowercase();
    // JSON-looking object/array (multi-line or compact).
    let trimmed = text.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        if trimmed.contains('"') && trimmed.contains(':') {
            return true;
        }
    }
    // SQL statements (common shapes, case-insensitive).
    let sql_tokens = [
        "select ", "insert into ", "update ", "delete from ", "create table ", "alter table ",
        "select\n", "from ", " where ", " values(",
    ];
    if lines <= 6 && lower.trim_start().starts_with("select ") && lower.contains(" from ") {
        return true;
    }
    if lower.trim_start().starts_with("insert into ") || lower.trim_start().starts_with("create table ") {
        return true;
    }
    // Shebang / XML / HTML document starts.
    if trimmed.starts_with("#!") || trimmed.starts_with("<?xml") || trimmed.starts_with("<!doctype") {
        return true;
    }
    let code_tokens: &[&str] = &[
        "fn ", "def ", "class ", "function ", "import ", "#include", "public static", "const ",
        "let mut ", "let ", "var ", "=>", "();", "</", "/>", "&&", "||", "::", "println!(",
        "console.log(", "system.out.", "def main(", "if (", "for (", "while (", "return ",
    ];
    let token_hits = code_tokens.iter().filter(|token| lower.contains(*token)).count();
    if token_hits >= 2 {
        return true;
    }
    // Braces/semicolons combined with a newline are strongly code-shaped.
    if lines >= 2 && (text.contains('{') || text.contains('}')) && text.contains(';') {
        return true;
    }
    // Indented multi-line block with assignment/operators.
    if lines >= 3 {
        let indented = text.lines().filter(|line| line.starts_with("  ") || line.starts_with('\t')).count();
        if indented >= 2
            && (text.contains('=') || text.contains('{') || text.contains("::") || text.contains("->"))
        {
            return true;
        }
    }
    let _ = sql_tokens;
    false
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

#[cfg(test)]
mod classifier_tests {
    use super::classify;
    use crate::ClipCategory;

    #[test]
    fn classifies_links() {
        for text in [
            "https://example.com",
            "https://example.com/path?q=1",
            "http://localhost:8080/x",
            "ftp://files.example.com/dir/file.txt",
            "mailto:someone@example.com",
            "www.example.com",
        ] {
            assert_eq!(classify(text), ClipCategory::Link, "{text:?}");
        }
    }

    #[test]
    fn classifies_emails() {
        for text in [
            "first.last@example.com",
            "a+b@sub.domain.co",
            "user_name@example.io",
        ] {
            assert_eq!(classify(text), ClipCategory::Email, "{text:?}");
        }
        assert_ne!(classify("not an @ sign"), ClipCategory::Email);
    }

    #[test]
    fn classifies_paths() {
        for text in [
            r"C:\Users\me\file.txt",
            r"C:\Program Files\Some App\app.exe",
            r"D:/data/report.csv",
            r"\\server\share\folder\file.doc",
            "./relative/file.rs",
            r".\relative\file.rs",
            "../up/file.txt",
        ] {
            assert_eq!(classify(text), ClipCategory::Path, "{text:?}");
        }
        assert_ne!(classify("word @ home"), ClipCategory::Path);
    }

    #[test]
    fn classifies_code() {
        for text in [
            "SELECT * FROM users WHERE id = 1;",
            "INSERT INTO t (a, b) VALUES (1, 2);",
            "{\n  \"name\": \"value\",\n  \"n\": 1\n}",
            "fn main() {\n    println!(\"hi\");\n}",
            "#include <stdio.h>\nint main(){return 0;}",
            "const x = () => 42;",
            "def process(items):\n    return [x for x in items]\n",
        ] {
            assert_eq!(classify(text), ClipCategory::Code, "{text:?}");
        }
    }

    #[test]
    fn classifies_plain_text() {
        for text in [
            "Hello world",
            "meeting notes for tomorrow at 10",
            "quarterly report 2026 final",
            "  ",
            "",
            "12345",
        ] {
            assert_eq!(classify(text), ClipCategory::Text, "{text:?}");
        }
    }

    #[test]
    fn classification_is_bounded_and_deterministic() {
        let huge = format!("https://example.com/{}", "a".repeat(64 * 1024));
        assert_eq!(classify(&huge), ClipCategory::Link);
        // Multiline prose with an indent or two is not code.
        let prose = "Dear team,\n  thanks for the update.\n  we will review tomorrow.\n";
        assert_eq!(classify(prose), ClipCategory::Text);
    }
}
