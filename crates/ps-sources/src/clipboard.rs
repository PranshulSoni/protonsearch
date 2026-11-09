//! Clipboard history capture, classification, persistence and search.
//! OWNER: sources-content.
//!
//! v2 schema (inside the shared v2 database):
//!   clipboard_history(id, content UNIQUE, timestamp, source_app, is_image,
//!     pinned, ocr_text, image_path, category, image_width, image_height,
//!     thumbnail_path, mime, format, image_bytes, alt_text)
//!
//! Gate 2 (task-39) behaviour:
//! - One history item per copy event. A copy exposing text + bitmap is stored
//!   as ONE image row (category Image) and the text alternative is kept in
//!   alt_text; images are never dropped because a text format is present.
//! - Image rows store the full PNG (content-hash named, atomic write), a bounded
//!   256 px PNG thumbnail, original dimensions, mime and source format.
//! - CF_HDROP file lists become category Files; HTML/RTF are stored with their
//!   text form and a format/mime marker.
//! - Re-copying identical content refreshes the timestamp so it bubbles to the
//!   top; rows are pruned by count and image-cache byte quota (pinned rows and
//!   their files are never pruned).
//! - copy_to_clipboard() restores text or a real bitmap (CF_DIB/DIBV5/PNG).
//!
//! Capture precedence (G1 contract, owned end-to-end by the engine + this module):
//!   MASTER GATE: privacy.clipboard_history == true AND clipboard.pause == false.
//!   The engine must not start the listener (and should ignore imported rows) when
//!   the master gate is closed. This module never bypasses that gate: it simply
//!   does not capture while no listener is running.
//!   FORMAT TOGGLES: clipboard.captureText and clipboard.captureImages are applied
//!   per snapshot by start_listener_with_options(). They only filter formats, not
//!   the master gate: captureImages=false drops image payloads (text/files/html/rtf
//!   still recorded), captureText=false drops text/html/rtf (images/files still
//!   recorded). File lists are not text and remain captured when the master gate is
//!   open. capture_current_with_options() applies the same filtering to the seed.
//!   Thumbnails satisfy the IPC contract: <=128 px longest edge, encoded PNG
//!   payload <=128 KiB, imageWidth/imageHeight <=8192, mime image/png|image/jpeg.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use super::{ClipCategory, ClipboardEntry};

const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
/// IPC contract: thumbnails are at most 128 px on the longest edge.
const THUMB_LONG_EDGE: u32 = 128;
/// IPC contract: thumbnail PNG payload must stay under 128 KiB.
const MAX_THUMBNAIL_BYTES: usize = 128 * 1024;
/// IPC contract: ResultMeta.imageWidth/imageHeight are <= 8192.
const MAX_DIMENSION: u32 = 8192;
/// IPC contract: only these image mimes may be exposed.
const ALLOWED_IMAGE_MIMES: &[&str] = &["image/png", "image/jpeg"];
const CLASSIFY_SAMPLE_BYTES: usize = 16 * 1024;
const MAX_BACKFILL_ROWS: usize = 10_000;
/// Retention: unpinned history is pruned to this many rows.
pub const MAX_HISTORY_ITEMS: u64 = 1000;
/// Retention: unpinned image files are pruned beyond this cache size.
pub const MAX_IMAGE_CACHE_BYTES: u64 = 256 * 1024 * 1024;

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
    category TEXT NOT NULL DEFAULT '',
    image_width INTEGER,
    image_height INTEGER,
    thumbnail_path TEXT,
    mime TEXT,
    format TEXT,
    image_bytes INTEGER,
    alt_text TEXT
);
CREATE INDEX IF NOT EXISTS idx_clipboard_timestamp
    ON clipboard_history(timestamp DESC);
";

/// Columns added after the original v2 clipboard schema; migrated additively.
const ADDED_COLUMNS: &[(&str, &str)] = &[
    ("category", "TEXT NOT NULL DEFAULT ''"),
    ("image_width", "INTEGER"),
    ("image_height", "INTEGER"),
    ("thumbnail_path", "TEXT"),
    ("mime", "TEXT"),
    ("format", "TEXT"),
    ("image_bytes", "INTEGER"),
    ("alt_text", "TEXT"),
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Per-format capture toggles (clipboard.captureText / clipboard.captureImages).
/// These are NOT the master gate: the engine decides whether to start the
/// listener at all using privacy.clipboard_history AND !clipboard.pause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardCaptureOptions {
    pub capture_text: bool,
    pub capture_images: bool,
}

impl Default for ClipboardCaptureOptions {
    fn default() -> Self {
        Self {
            capture_text: true,
            capture_images: true,
        }
    }
}

/// Start the Win32 clipboard listener with both format toggles on and seed the
/// current clipboard first. Only one listener may run per process.
pub fn start_listener(
    db_path: PathBuf,
    image_dir: PathBuf,
) -> Result<ps_windows::ClipboardListener> {
    start_listener_with_options(db_path, image_dir, ClipboardCaptureOptions::default())
}

/// Start the listener with explicit format toggles (master gate still belongs to
/// the engine: do not call this when privacy is off or clipboard capture is paused).
pub fn start_listener_with_options(
    db_path: PathBuf,
    image_dir: PathBuf,
    options: ClipboardCaptureOptions,
) -> Result<ps_windows::ClipboardListener> {
    let _ = capture_current_with_options(&db_path, &image_dir, options);
    let listener = ps_windows::ClipboardListener::start(move |snapshot| {
        let snapshot = filter_snapshot(snapshot, options);
        if snapshot.has_content() {
            let _ = record_snapshot(&db_path, &image_dir, &snapshot);
        }
    })?;
    Ok(listener)
}

/// Apply captureText/captureImages to one snapshot. File lists stay captured
/// (they are neither text nor image formats).
pub fn filter_snapshot(
    mut snapshot: ps_windows::ClipboardSnapshot,
    options: ClipboardCaptureOptions,
) -> ps_windows::ClipboardSnapshot {
    if !options.capture_images {
        snapshot.png = None;
        snapshot.width = None;
        snapshot.height = None;
        snapshot.image_format = None;
        snapshot.mime = None;
    }
    if !options.capture_text {
        snapshot.text = None;
        snapshot.html = None;
        snapshot.rtf = None;
        if snapshot.png.is_none() && snapshot.files.is_empty() {
            snapshot.formats.clear();
        }
    }
    snapshot
}

/// Read the CURRENT OS clipboard once (all representations) and persist it.
/// Returns the stored entry so callers can update the UI immediately.
pub fn capture_current(db_path: &Path, image_dir: &Path) -> Result<Option<ClipboardEntry>> {
    capture_current_with_options(db_path, image_dir, ClipboardCaptureOptions::default())
}

/// Seed variant that honours captureText/captureImages format toggles.
pub fn capture_current_with_options(
    db_path: &Path,
    image_dir: &Path,
    options: ClipboardCaptureOptions,
) -> Result<Option<ClipboardEntry>> {
    let snapshot = filter_snapshot(ps_windows::read_clipboard_snapshot(), options);
    if !snapshot.has_content() {
        return Ok(None);
    }
    let ids = record_snapshot(db_path, image_dir, &snapshot)?;
    if ids.is_empty() {
        return Ok(None);
    }
    Ok(list_recent(db_path, 10)
        .into_iter()
        .find(|entry| ids.contains(&entry.id)))
}

/// Persist a full clipboard snapshot as ONE history item.
///
/// Preference order: image (with text/files/html/rtf kept as alt_text), then
/// file list, then HTML, then RTF, then plain text. Returns the touched row ids.
pub fn record_snapshot(
    db_path: &Path,
    image_dir: &Path,
    snapshot: &ps_windows::ClipboardSnapshot,
) -> Result<Vec<i64>> {
    let source_app = snapshot.source_app.clone().unwrap_or_default();
    let alt_text = snapshot
        .text
        .clone()
        .or_else(|| snapshot.html.clone())
        .or_else(|| snapshot.rtf.clone())
        .or_else(|| {
            if snapshot.files.is_empty() {
                None
            } else {
                Some(snapshot.files.join("\r\n"))
            }
        })
        .filter(|text| !text.trim().is_empty());

    if let Some(png) = snapshot.png.as_deref() {
        let width = snapshot.width.unwrap_or(0);
        let height = snapshot.height.unwrap_or(0);
        let format = snapshot.image_format.clone().unwrap_or_else(|| "png".to_string());
        let id = insert_image(
            db_path,
            image_dir,
            png,
            width,
            height,
            &format,
            snapshot.mime.as_deref().unwrap_or("image/png"),
            &source_app,
            alt_text.as_deref(),
        )?;
        return Ok(id.map(|(id, _)| id).into_iter().collect());
    }

    if !snapshot.files.is_empty() {
        let content = snapshot.files.join("\r\n");
        let id = insert_item(
            db_path,
            image_dir,
            &content,
            &source_app,
            ClipCategory::Files,
            "hdrop",
            "text/x-file-list",
            None,
        )?;
        return Ok(id.map(|(id, _)| id).into_iter().collect());
    }

    if let Some(html) = snapshot.html.as_deref() {
        let text = strip_html(html);
        let content = if text.trim().is_empty() { html.to_string() } else { text };
        let category = classify(&content);
        let id = insert_item(db_path, image_dir, &content, &source_app, category, "html", "text/html", Some(html))?;
        return Ok(id.map(|(id, _)| id).into_iter().collect());
    }

    if let Some(rtf) = snapshot.rtf.as_deref() {
        let text = strip_rtf(rtf);
        let content = if text.trim().is_empty() { rtf.to_string() } else { text };
        let category = classify(&content);
        let id = insert_item(db_path, image_dir, &content, &source_app, category, "rtf", "application/rtf", Some(rtf))?;
        return Ok(id.map(|(id, _)| id).into_iter().collect());
    }

    if let Some(text) = snapshot.text.as_deref() {
        let category = classify(text);
        let id = insert_item(db_path, image_dir, text, &source_app, category, "text", "text/plain", None)?;
        return Ok(id.map(|(id, _)| id).into_iter().collect());
    }

    Ok(Vec::new())
}

/// Number of stored clipboard items (pinned and images included). 0 when the
/// database/table does not exist; never errors.
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

/// Insert a text item; consecutive identical text only refreshes the timestamp
/// (returns false). Empty/oversized text is ignored.
pub fn record(db_path: &Path, text: &str, source_app: &str) -> Result<bool> {
    if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
        return Ok(false);
    }
    let category = classify(text);
    let stored = insert_item(
        db_path,
        Path::new("."),
        text,
        source_app,
        category,
        "text",
        "text/plain",
        None,
    )?;
    Ok(stored.map(|(_, created)| created).unwrap_or(false))
}

/// Store a PNG clipboard image using only the encoded payload (legacy helper:
/// dimensions are decoded from the PNG). Prefer record_image_captured when the
/// capture pipeline already knows format/dimensions.
pub fn record_image(
    db_path: &Path,
    png_bytes: &[u8],
    source_app: &str,
    image_dir: &Path,
) -> Result<bool> {
    record_image_captured(db_path, png_bytes, 0, 0, "png", "image/png", source_app, image_dir)
}

/// Store a PNG clipboard image with explicit metadata from the capture pipeline.
pub fn record_image_captured(
    db_path: &Path,
    png_bytes: &[u8],
    width: u32,
    height: u32,
    format: &str,
    mime: &str,
    source_app: &str,
    image_dir: &Path,
) -> Result<bool> {
    let stored = insert_image(
        db_path, image_dir, png_bytes, width, height, format, mime, source_app, None,
    )?;
    Ok(stored.map(|(_, created)| created).unwrap_or(false))
}

/// Re-copy a stored history item back to the OS clipboard. Images are restored
/// as a real bitmap (CF_DIB + CF_DIBV5 + registered PNG + CF_BITMAP); text is
/// restored as CF_UNICODETEXT. Returns a structured error when the stored
/// payload is missing or the host cannot accept the format.
pub fn copy_to_clipboard(db_path: &Path, id: i64) -> Result<()> {
    let conn = open_or_create(db_path)?;
    let (content, is_image, image_path): (String, i64, Option<String>) = conn
        .query_row(
            "SELECT content, is_image, image_path FROM clipboard_history WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .with_context(|| format!("clipboard row {id} not found"))?;
    if is_image != 0 {
        let path = image_path
            .filter(|path| !path.is_empty())
            .ok_or_else(|| anyhow::anyhow!("clipboard image row {id} has no stored PNG path"))?;
        let bytes = std::fs::read(&path)
            .with_context(|| format!("reading stored clipboard image {path}"))?;
        ps_windows::set_clipboard_image(&bytes)?;
        Ok(())
    } else {
        ps_windows::set_clipboard_text(&content)?;
        Ok(())
    }
}

/// Search clipboard text (case-insensitive substring, pinned first then newest).
/// Images are excluded; an empty query lists recent items.
pub fn search(db_path: &Path, query: &str, limit: usize) -> Vec<ClipboardEntry> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return list_recent(db_path, limit);
    }
    let Ok((conn, flags)) = open_existing_with_flags(db_path) else {
        return Vec::new();
    };
    let pattern = format!("%{}%", escape_like(trimmed));
    let sql = format!(
        "SELECT {} FROM clipboard_history
         WHERE content LIKE ?1 ESCAPE '\\' AND is_image = 0
         ORDER BY pinned DESC, timestamp DESC, id DESC LIMIT ?2",
        select_columns(&flags)
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
    let Ok((conn, flags)) = open_existing_with_flags(db_path) else {
        return Vec::new();
    };
    let sql = format!(
        "SELECT {} FROM clipboard_history
         ORDER BY pinned DESC, timestamp DESC, id DESC LIMIT ?1",
        select_columns(&flags)
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
    let files: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT image_path, thumbnail_path FROM clipboard_history WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    let changed = conn.execute("DELETE FROM clipboard_history WHERE id = ?1", [id])?;
    if changed > 0 {
        remove_owned_files(files);
    }
    Ok(changed > 0)
}

/// Best-effort import of the legacy v1 clipboard_history table (read-only).
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
            "INSERT OR IGNORE INTO clipboard_history
                (content, timestamp, source_app, is_image, pinned)
             SELECT content, timestamp,
                    CASE WHEN source_app IS NULL OR source_app = '' THEN 'legacy' ELSE source_app END,
                    COALESCE(is_image, 0), COALESCE(pinned, 0)
             FROM legacy.clipboard_history
             WHERE content IS NOT NULL AND content != ''",
            [],
        )?;
        let _ = backfill_categories(&conn);
        let _ = backfill_image_meta(&conn);
        Ok(inserted)
    })();
    let _ = conn.execute_batch("DETACH DATABASE legacy");
    result
}

/// Deterministic content classifier used when a clipboard item is stored.
///
/// Rules (checked in order): URL (http/https/ftp/mailto, incl. www.) -> Link;
/// single-token email address -> Email; Windows drive/UNC or ./ relative path ->
/// Path; code-like content (code tokens, JSON, SQL, indented multi-line) -> Code;
/// otherwise Text. Images/Files are classified by the capture path.
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

// ---------------------------------------------------------------------------
// Storage internals
// ---------------------------------------------------------------------------

fn insert_item(
    db_path: &Path,
    image_dir: &Path,
    content: &str,
    source_app: &str,
    category: ClipCategory,
    format: &str,
    mime: &str,
    alt_text: Option<&str>,
) -> Result<Option<(i64, bool)>> {
    if content.trim().is_empty() || content.len() > MAX_TEXT_BYTES {
        return Ok(None);
    }
    let conn = open_or_create(db_path)?;
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM clipboard_history WHERE content = ?1",
            rusqlite::params![content],
            |row| row.get(0),
        )
        .ok();
    let timestamp = now_ms();
    let (id, created) = match existing {
        Some(id) => {
            conn.execute(
                "UPDATE clipboard_history
                    SET timestamp = ?1, category = ?2, mime = ?3, format = ?4,
                        alt_text = COALESCE(?5, alt_text)
                  WHERE id = ?6",
                rusqlite::params![timestamp, category.as_str(), mime, format, alt_text, id],
            )?;
            (id, false)
        }
        None => {
            conn.execute(
                "INSERT INTO clipboard_history
                    (content, timestamp, source_app, is_image, pinned, category, mime, format, alt_text)
                 VALUES(?1, ?2, ?3, 0, 0, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    content,
                    timestamp,
                    source_app,
                    category.as_str(),
                    mime,
                    format,
                    alt_text
                ],
            )?;
            (conn.last_insert_rowid(), true)
        }
    };
    let _ = enforce_retention(&conn, image_dir);
    Ok(Some((id, created)))
}

fn insert_image(
    db_path: &Path,
    image_dir: &Path,
    png_bytes: &[u8],
    width: u32,
    height: u32,
    format: &str,
    _declared_mime: &str,
    source_app: &str,
    alt_text: Option<&str>,
) -> Result<Option<(i64, bool)>> {
    if png_bytes.is_empty() || png_bytes.len() > MAX_IMAGE_BYTES {
        return Ok(None);
    }
    let stored = write_image_files(image_dir, png_bytes)?;
    let (width, height) = if width == 0 || height == 0 {
        (stored.width, stored.height)
    } else {
        (width, height)
    };
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        anyhow::bail!(
            "clipboard image dimensions exceed the IPC contract: {width}x{height} (max {MAX_DIMENSION})"
        );
    }
    if png_bytes.len() < 8 || !png_bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        anyhow::bail!("clipboard image payload is not a PNG");
    }
    // We always store a PNG re-encode of the captured bitmap, so the exposed
    // mime is always on the IPC allowlist (image/png). JPEG is accepted by the
    // contract but not produced by the current capture pipeline.
    let _ = ALLOWED_IMAGE_MIMES;
    let mime = "image/png";
    let conn = open_or_create(db_path)?;
    let content = format!("[image:{}]", stored.hash);
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM clipboard_history WHERE content = ?1",
            rusqlite::params![content],
            |row| row.get(0),
        )
        .ok();
    let timestamp = now_ms();
    let image_path = stored.image_path.to_string_lossy().to_string();
    let thumbnail_path = stored.thumbnail_path.to_string_lossy().to_string();
    let image_bytes = png_bytes.len() as i64;
    let (id, created) = match existing {
        Some(id) => {
            conn.execute(
                "UPDATE clipboard_history
                    SET timestamp = ?1, is_image = 1, category = ?2, image_path = ?3,
                        image_width = ?4, image_height = ?5, thumbnail_path = ?6,
                        mime = ?7, format = ?8, image_bytes = ?9,
                        alt_text = COALESCE(?10, alt_text)
                  WHERE id = ?11",
                rusqlite::params![
                    timestamp,
                    ClipCategory::Image.as_str(),
                    image_path,
                    width as i64,
                    height as i64,
                    thumbnail_path,
                    mime,
                    format,
                    image_bytes,
                    alt_text,
                    id
                ],
            )?;
            (id, false)
        }
        None => {
            conn.execute(
                "INSERT INTO clipboard_history
                    (content, timestamp, source_app, is_image, pinned, category,
                     image_path, image_width, image_height, thumbnail_path, mime, format,
                     image_bytes, alt_text)
                 VALUES(?1, ?2, ?3, 1, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    content,
                    timestamp,
                    source_app,
                    ClipCategory::Image.as_str(),
                    image_path,
                    width as i64,
                    height as i64,
                    thumbnail_path,
                    mime,
                    format,
                    image_bytes,
                    alt_text
                ],
            )?;
            (conn.last_insert_rowid(), true)
        }
    };
    let _ = enforce_retention(&conn, image_dir);
    Ok(Some((id, created)))
}

struct StoredImageFiles {
    hash: String,
    image_path: PathBuf,
    thumbnail_path: PathBuf,
    width: u32,
    height: u32,
}

fn write_image_files(image_dir: &Path, png_bytes: &[u8]) -> Result<StoredImageFiles> {
    std::fs::create_dir_all(image_dir)
        .with_context(|| format!("creating clipboard image dir {}", image_dir.display()))?;
    let hash = hex_sha256(png_bytes);
    let image_path = image_dir.join(format!("{hash}.png"));
    let thumbnail_path = image_dir.join(format!("thumb-{hash}.png"));
    if !image_path.is_file() {
        atomic_write(&image_path, png_bytes)?;
    }
    let decoded = image::load_from_memory(png_bytes).ok();
    let (width, height) = match &decoded {
        Some(image) => (image.width(), image.height()),
        None => (0, 0),
    };
    if width > 0 && height > 0 {
        if (width as u64) * (height as u64) > MAX_IMAGE_PIXELS {
            anyhow::bail!("clipboard image too large: {width}x{height}");
        }
        if !thumbnail_path.is_file() {
            let thumbnail = decoded
                .as_ref()
                .map(|image| make_thumbnail(image, THUMB_LONG_EDGE))
                .unwrap_or_default();
            if !thumbnail.is_empty() {
                atomic_write(&thumbnail_path, &thumbnail)?;
            }
        }
    }
    Ok(StoredImageFiles {
        hash,
        image_path,
        thumbnail_path,
        width,
        height,
    })
}

/// Encode a PNG thumbnail bounded by the IPC contract: longest edge <=128 px and
/// encoded payload <=128 KiB. Falls back to smaller edges if needed; returns an
/// empty Vec when no compliant thumbnail can be produced.
fn make_thumbnail(image: &image::DynamicImage, _max_edge: u32) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return Vec::new();
    }
    for edge in [THUMB_LONG_EDGE, 96, 64] {
        let scale = (edge as f64 / width as f64).min(edge as f64 / height as f64).min(1.0);
        let target_width = ((width as f64 * scale).round() as u32).max(1);
        let target_height = ((height as f64 * scale).round() as u32).max(1);
        let resized = if target_width == width && target_height == height {
            image.to_rgba8()
        } else {
            image::imageops::resize(
                &image.to_rgba8(),
                target_width,
                target_height,
                image::imageops::FilterType::Triangle,
            )
        };
        let mut out = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut out);
        if image::DynamicImage::ImageRgba8(resized)
            .write_to(&mut cursor, image::ImageOutputFormat::Png)
            .is_ok()
            && !out.is_empty()
            && out.len() <= MAX_THUMBNAIL_BYTES
        {
            return out;
        }
    }
    Vec::new()
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&temp, bytes)
        .with_context(|| format!("writing clipboard payload {}", temp.display()))?;
    std::fs::rename(&temp, path)
        .with_context(|| format!("committing clipboard payload {}", path.display()))?;
    Ok(())
}

fn remove_owned_files(files: Option<(Option<String>, Option<String>)>) {
    if let Some((image, thumbnail)) = files {
        for file in [image, thumbnail].into_iter().flatten() {
            if !file.is_empty() {
                let _ = std::fs::remove_file(file);
            }
        }
    }
}

/// Keep history bounded: prune old unpinned rows and image files over quota.
fn enforce_retention(conn: &rusqlite::Connection, image_dir: &Path) -> Result<()> {
    let mut deleted_files: Vec<String> = Vec::new();
    // 1) Row-count retention.
    {
        let mut statement = conn.prepare(
            "SELECT id, image_path, thumbnail_path FROM clipboard_history
             WHERE pinned = 0
             ORDER BY timestamp DESC, id DESC
             LIMIT -1 OFFSET ?1",
        )?;
        let rows = statement.query_map([MAX_HISTORY_ITEMS as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        let stale: Vec<(i64, Option<String>, Option<String>)> = rows.flatten().collect();
        for (id, image, thumbnail) in stale {
            conn.execute("DELETE FROM clipboard_history WHERE id = ?1", [id])?;
            deleted_files.extend(image.into_iter());
            deleted_files.extend(thumbnail.into_iter());
        }
    }
    // 2) Image-cache byte quota (oldest unpinned images first).
    let total: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(image_bytes), 0) FROM clipboard_history
             WHERE is_image = 1 AND pinned = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if total as u64 > MAX_IMAGE_CACHE_BYTES {
        let mut statement = conn.prepare(
            "SELECT id, image_path, thumbnail_path, image_bytes FROM clipboard_history
             WHERE is_image = 1 AND pinned = 0
             ORDER BY timestamp ASC, id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        })?;
        let mut remaining = total as u64;
        for row in rows.flatten() {
            if remaining <= MAX_IMAGE_CACHE_BYTES {
                break;
            }
            let (id, image, thumbnail, bytes) = row;
            conn.execute("DELETE FROM clipboard_history WHERE id = ?1", [id])?;
            remaining = remaining.saturating_sub(bytes.unwrap_or(0).max(0) as u64);
            deleted_files.extend(image.into_iter());
            deleted_files.extend(thumbnail.into_iter());
        }
    }
    for file in deleted_files {
        if !file.is_empty() {
            let _ = std::fs::remove_file(&file);
        }
    }
    let _ = image_dir;
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Read path (schema-tolerant for pre-migration databases)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
struct SchemaFlags {
    category: bool,
    image_width: bool,
    image_height: bool,
    thumbnail_path: bool,
    mime: bool,
    format: bool,
}

impl SchemaFlags {
    fn expr(self, enabled: bool, name: &'static str) -> &'static str {
        if enabled {
            name
        } else {
            "NULL"
        }
    }
}

fn select_columns(flags: &SchemaFlags) -> String {
    let category = flags.expr(flags.category, "category");
    let image_width = flags.expr(flags.image_width, "image_width");
    let image_height = flags.expr(flags.image_height, "image_height");
    let thumbnail = flags.expr(flags.thumbnail_path, "thumbnail_path");
    let mime = flags.expr(flags.mime, "mime");
    let format = flags.expr(flags.format, "format");
    format!(
        "id, content, timestamp, source_app, pinned, is_image,
         {category}, image_path, ocr_text,
         {image_width}, {image_height}, {thumbnail}, {mime}, {format}"
    )
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    let id: i64 = row.get(0)?;
    let content: String = row.get(1)?;
    let timestamp_ms: i64 = row.get(2)?;
    let source_app: String = row.get(3)?;
    let pinned = row.get::<_, i64>(4)? != 0;
    let is_image = row.get::<_, i64>(5)? != 0;
    let stored_category: String = row.get(6).unwrap_or_default();
    let image_path: Option<String> = row.get(7).unwrap_or_default();
    let ocr_text: Option<String> = row.get(8).unwrap_or_default();
    let width: Option<i64> = row.get(9).unwrap_or_default();
    let height: Option<i64> = row.get(10).unwrap_or_default();
    let thumbnail_path: Option<String> = row.get(11).unwrap_or_default();
    let mime: Option<String> = row.get(12).unwrap_or_default();
    let format: Option<String> = row.get(13).unwrap_or_default();
    let category = ClipCategory::from_db(&stored_category).unwrap_or_else(|| {
        if is_image {
            ClipCategory::Image
        } else if format.as_deref() == Some("hdrop") {
            ClipCategory::Files
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
        image_path: image_path.filter(|path| !path.is_empty()),
        width: width.and_then(|value| u32::try_from(value).ok()),
        height: height.and_then(|value| u32::try_from(value).ok()),
        thumbnail_path: thumbnail_path.filter(|path| !path.is_empty()),
        mime: mime.filter(|value| !value.is_empty()),
        format: format.filter(|value| !value.is_empty()),
        ocr_text: ocr_text.filter(|value| !value.is_empty()),
    })
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

fn open_existing_with_flags(db_path: &Path) -> Result<(rusqlite::Connection, SchemaFlags)> {
    let conn = open_existing(db_path)?;
    let flags = SchemaFlags {
        category: has_column(&conn, "category")?,
        image_width: has_column(&conn, "image_width")?,
        image_height: has_column(&conn, "image_height")?,
        thumbnail_path: has_column(&conn, "thumbnail_path")?,
        mime: has_column(&conn, "mime")?,
        format: has_column(&conn, "format")?,
    };
    Ok((conn, flags))
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

fn migrate(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)
        .context("migrating clipboard schema")?;
    let mut added = false;
    for (name, definition) in ADDED_COLUMNS {
        if !has_column(conn, name)? {
            conn.execute_batch(&format!(
                "ALTER TABLE clipboard_history ADD COLUMN {name} {definition}"
            ))
            .with_context(|| format!("adding clipboard_history.{name}"))?;
            added = true;
        }
    }
    if added {
        let _ = backfill_categories(conn);
        let _ = backfill_image_meta(conn);
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

fn backfill_categories(conn: &rusqlite::Connection) -> Result<usize> {
    let rows: Vec<(i64, String, i64, Option<String>)> = {
        let mut statement = conn.prepare(
            "SELECT id, content, is_image, format FROM clipboard_history
             WHERE category IS NULL OR category = '' LIMIT ?1",
        )?;
        let rows = statement.query_map([MAX_BACKFILL_ROWS as i64], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3).unwrap_or_default()))
        })?;
        rows.flatten().collect()
    };
    let mut updated = 0usize;
    for (id, content, is_image, format) in rows {
        let category = if is_image != 0 {
            ClipCategory::Image
        } else if format.as_deref() == Some("hdrop") {
            ClipCategory::Files
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

/// Backfill dimensions/thumbnails for image rows captured before task-39.
fn backfill_image_meta(conn: &rusqlite::Connection) -> Result<usize> {
    let rows: Vec<(i64, String)> = {
        let mut statement = conn.prepare(
            "SELECT id, image_path FROM clipboard_history
             WHERE is_image = 1 AND image_path IS NOT NULL AND image_path != ''
               AND (image_width IS NULL OR image_width = 0)
             LIMIT 200",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get(0)?, row.get::<_, String>(1)?))
        })?;
        rows.flatten().collect()
    };
    let mut updated = 0usize;
    for (id, path) in rows {
        let path_buf = PathBuf::from(&path);
        let Ok(bytes) = std::fs::read(&path_buf) else {
            continue;
        };
        if bytes.len() > MAX_IMAGE_BYTES {
            continue;
        }
        let Ok(image) = image::load_from_memory(&bytes) else {
            continue;
        };
        let width = image.width();
        let height = image.height();
        let thumb_path = match path_buf.parent() {
            Some(parent) => {
                let stem = path_buf
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_else(|| "image".to_string());
                parent.join(format!("thumb-{stem}.png"))
            }
            None => continue,
        };
        if !thumb_path.is_file() {
            let thumbnail = make_thumbnail(&image, THUMB_LONG_EDGE);
            if !thumbnail.is_empty() {
                let _ = atomic_write(&thumb_path, &thumbnail);
            }
        }
        updated += conn.execute(
            "UPDATE clipboard_history
                SET image_width = ?1, image_height = ?2, thumbnail_path = ?3,
                    mime = COALESCE(mime, 'image/png'), format = COALESCE(format, 'png'),
                    image_bytes = ?4
              WHERE id = ?5",
            rusqlite::params![
                width as i64,
                height as i64,
                thumb_path.to_string_lossy().to_string(),
                bytes.len() as i64,
                id
            ],
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
        ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '%' | '+' | '-' | '\'')
    });
    let valid_domain = domain
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'));
    valid_local && valid_domain
}

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
        return text.len() > 4 && text[2..].contains('\\');
    }
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    text.starts_with("\\\\?\\")
}

fn is_code(text: &str) -> bool {
    let lines = text.lines().count();
    let lower = text.to_ascii_lowercase();
    let trimmed = text.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        if trimmed.contains('"') && trimmed.contains(':') {
            return true;
        }
    }
    if lines <= 6 && lower.trim_start().starts_with("select ") && lower.contains(" from ") {
        return true;
    }
    if lower.trim_start().starts_with("insert into ") || lower.trim_start().starts_with("create table ") {
        return true;
    }
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
    if lines >= 2 && (text.contains('{') || text.contains('}')) && text.contains(';') {
        return true;
    }
    if lines >= 3 {
        let indented = text
            .lines()
            .filter(|line| line.starts_with("  ") || line.starts_with('\t'))
            .count();
        if indented >= 2
            && (text.contains('=') || text.contains('{') || text.contains("::") || text.contains("->"))
        {
            return true;
        }
    }
    false
}

fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_rtf(rtf: &str) -> String {
    let mut out = String::with_capacity(rtf.len());
    let mut chars = rtf.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                // Control word: skip letters and an optional numeric parameter.
                let mut next = chars.next();
                while matches!(next, Some(ch) if ch.is_ascii_alphabetic()) {
                    next = chars.next();
                }
                while matches!(next, Some(ch) if ch.is_ascii_digit() || ch == '-') {
                    next = chars.next();
                }
                if next == Some(' ') {
                } else if let Some(ch) = next {
                    if ch == '\\' || ch == '{' || ch == '}' {
                        out.push(ch);
                    }
                }
            }
            '{' | '}' => {}
            _ => out.push(ch),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
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
        let prose = "Dear team,\n  thanks for the update.\n  we will review tomorrow.\n";
        assert_eq!(classify(prose), ClipCategory::Text);
    }
}