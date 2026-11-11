//! Preview payload construction (Gate 2, task-37).
//!
//! Bounded, honest previews for every result family:
//! - text/code/markdown/json/log and structured documents (pdf/docx/pptx/xlsx/epub)
//!   are extracted with the existing ps-content extractors (512 KB extraction cap);
//! - images (files and clipboard rows) return a downscaled PNG data URL plus the
//!   indexed OCR text when available;
//! - binary/unsupported/folder rows return metadata plus an explicit
//!   unsupportedReason instead of blank content.
//!
//! The caller (Engine::preview) provides result resolution, caching and limits;
//! this module never touches search state and never runs extraction on the
//! search hot path.

use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use ps_ipc::{
    ActionSpec, PreviewField, PreviewPayload, PreviewRange, PreviewStatus, SearchResult, SourceKind,
};

use crate::db::EngineDb;

pub(crate) const RESULT_REGISTRY_CAP: usize = 512;

const PREVIEW_IMAGE_MAX_BYTES: u64 = 16 * 1024 * 1024;
const PREVIEW_IMAGE_MAX_EDGE: u32 = 640;
const PREVIEW_IMAGE_MAX_DECODE_PIXELS: u64 = 16 * 1024 * 1024;
const PREVIEW_TEXT_MAX_BYTES: u64 = 16 * 1024 * 1024;
const PREVIEW_SNIPPET_CHARS: usize = 240;
const PREVIEW_FIELD_MAX_CHARS: usize = 200;

/// Bounded registry of the most recent search results. Preview requests arrive
/// by resultId (the UI only knows ids), so the engine remembers the rows it
/// just returned. FIFO bounded; re-searching the same id refreshes it.
#[derive(Default)]
pub(crate) struct ResultRegistry {
    order: VecDeque<String>,
    map: HashMap<String, SearchResult>,
}

impl ResultRegistry {
    pub(crate) fn remember(&mut self, results: &[SearchResult]) {
        for result in results {
            if self.map.insert(result.id.clone(), result.clone()).is_none() {
                self.order.push_back(result.id.clone());
            }
        }
        while self.order.len() > RESULT_REGISTRY_CAP {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
    }

    pub(crate) fn get(&self, result_id: &str) -> Option<&SearchResult> {
        self.map.get(result_id)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

/// Build the preview payload for one resolved search result.
pub(crate) fn build_preview(
    result: &SearchResult,
    max_chars: usize,
    db: &EngineDb,
    clipboard_db: &Path,
) -> PreviewPayload {
    let mut payload = PreviewPayload {
        result_id: result.id.clone(),
        title: result.title.clone(),
        subtitle: result.subtitle.clone(),
        format: preview_format(result),
        text: None,
        snippet: result
            .meta
            .snippet
            .clone()
            .filter(|value| !value.trim().is_empty()),
        highlight_ranges: result
            .match_ranges
            .iter()
            .map(|range| PreviewRange {
                start: range.start,
                end: range.end,
            })
            .collect(),
        image_data_url: None,
        ocr_text: None,
        fields: preview_fields(result),
        unsupported_reason: None,
        status: PreviewStatus::Ok,
        truncated: false,
        error: None,
        image_mime: None,
    };
    match result.kind {
        SourceKind::File
        | SourceKind::Folder
        | SourceKind::Document
        | SourceKind::ContentMatch
        | SourceKind::OcrText
        | SourceKind::Recent => match result.path.as_deref() {
            Some(path) => {
                fill_file_preview(&mut payload, path, max_chars, db, result.kind == SourceKind::File)
            }
            None => {
                payload.unsupported_reason = Some("No file path is attached to this result.".to_string());
            }
        },
        SourceKind::Clipboard | SourceKind::Snippet => {
            fill_clipboard_preview(&mut payload, result, max_chars, clipboard_db);
        }
        SourceKind::BrowserHistory | SourceKind::Bookmark => {
            payload.format = "url".to_string();
            payload.text = Some(result.path.clone().unwrap_or_else(|| result.subtitle.clone()));
        }
        _ => {
            payload.format = preview_format(result);
            payload.text = Some(if result.subtitle.trim().is_empty() {
                result.title.clone()
            } else {
                result.subtitle.clone()
            });
        }
    }
    if payload.status == PreviewStatus::Ok && payload.unsupported_reason.is_some() {
        payload.status = PreviewStatus::Unsupported;
    }
    payload
}

fn fill_clipboard_preview(
    payload: &mut PreviewPayload,
    result: &SearchResult,
    max_chars: usize,
    clipboard_db: &Path,
) {
    payload.format = "clipboard".to_string();
    let is_image_row = result.meta.thumbnail_data_url.is_some()
        || result.meta.image_width.is_some()
        || result.meta.mime.as_deref() == Some("image/png")
        || result
            .path
            .as_deref()
            .map(|path| is_image_extension(extension_of(path).as_str()))
            .unwrap_or(false);
    if !is_image_row {
        if let Some(text) = clipboard_text(result) {
            let (text, truncated) = sanitize_preview_text(&text, max_chars);
            payload.text = Some(text);
            if truncated {
                payload.truncated = true;
                payload.fields.push(PreviewField {
                    label: "Note".into(),
                    value: "Preview truncated".into(),
                });
            }
            return;
        }
    }
    // Clipboard image row: the search payload normally already carries the
    // bounded thumbnail + dimensions. Fall back to the stored image/OCR row for
    // older payloads or after a restart.
    let mut image_source: Option<String> = None;
    let mut dimensions: Option<(u32, u32)> = match (result.meta.image_width, result.meta.image_height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => Some((width, height)),
        _ => None,
    };
    let mut ocr_text: Option<String> = None;
    if let Some(thumbnail) = result.meta.thumbnail_data_url.as_deref() {
        payload.image_data_url = Some(thumbnail.to_string());
        payload.image_mime = Some("image/png".to_string());
        payload.format = "image".to_string();
    } else if let Some(path) = result
        .path
        .as_deref()
        .filter(|path| is_image_extension(extension_of(path).as_str()))
    {
        image_source = Some(path.to_string());
    }
    if image_source.is_none() && payload.image_data_url.is_none() {
        if let Some((image_path, width, height, ocr)) = clipboard_image_info(clipboard_db, &result.id) {
            image_source = Some(image_path);
            if width > 0 && height > 0 {
                dimensions = Some((width, height));
            }
            ocr_text = ocr;
        }
    }
    if let Some(source) = image_source {
        match image_preview_data_url(Path::new(&source)) {
            Ok((data_url, width, height)) => {
                payload.image_data_url = Some(data_url);
                payload.image_mime = Some("image/png".to_string());
                payload.format = "image".to_string();
                if dimensions.is_none() {
                    dimensions = Some((width, height));
                }
            }
            Err(err) => {
                let reason = format!("Image could not be decoded: {err}");
                payload.error = Some(reason.clone());
                payload.status = PreviewStatus::Error;
                payload.unsupported_reason = Some(reason);
            }
        }
    }
    if let Some((width, height)) = dimensions {
        payload.fields.push(PreviewField {
            label: "Dimensions".into(),
            value: format!("{width} x {height}"),
        });
    }
    if let Some(mime) = result.meta.mime.as_deref() {
        payload.fields.push(PreviewField {
            label: "Type".into(),
            value: mime.to_string(),
        });
    }
    if ocr_text.is_none() {
        if let Some((_, _, _, ocr)) = clipboard_image_info(clipboard_db, &result.id) {
            ocr_text = ocr;
        }
    }
    if payload.ocr_text.is_none() {
        if let Some(ocr) = ocr_text {
            let (ocr, _) = sanitize_preview_text(&ocr, max_chars);
            if !ocr.trim().is_empty() {
                payload.ocr_text = Some(ocr);
            }
        }
    }
    if payload.image_data_url.is_none() && payload.text.is_none() && payload.unsupported_reason.is_none() {
        payload.unsupported_reason = Some("Clipboard item content is unavailable.".to_string());
    }
}

fn fill_file_preview(
    payload: &mut PreviewPayload,
    path: &str,
    max_chars: usize,
    db: &EngineDb,
    filename_result: bool,
) {
    let path_ref = Path::new(path);
    let extension = extension_of(path);
    let metadata = std::fs::metadata(path_ref).ok();
    if let Some(metadata) = &metadata {
        if !payload.fields.iter().any(|field| field.label == "Size") {
            payload.fields.push(PreviewField {
                label: "Size".into(),
                value: format_bytes_human(metadata.len()),
            });
        }
        if !payload.fields.iter().any(|field| field.label == "Modified") {
            if let Ok(modified) = metadata.modified() {
                let ms = modified
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_millis() as i64)
                    .unwrap_or(0);
                payload.fields.push(PreviewField {
                    label: "Modified".into(),
                    value: format_utc_ms(ms),
                });
            }
        }
    }
    if payload.format == "folder" {
        payload.unsupported_reason =
            Some("Folder preview is not available; open it in Explorer.".to_string());
        return;
    }
    if is_image_extension(&extension) {
        if let Some(metadata) = &metadata {
            if metadata.len() > PREVIEW_IMAGE_MAX_BYTES {
                payload.unsupported_reason = Some(format!(
                    "Image is too large to preview ({}, limit {}).",
                    format_bytes_human(metadata.len()),
                    format_bytes_human(PREVIEW_IMAGE_MAX_BYTES)
                ));
                return;
            }
        }
        match image_preview_data_url(path_ref) {
            Ok((data_url, width, height)) => {
                payload.image_data_url = Some(data_url);
                payload.image_mime = Some("image/png".to_string());
                payload.fields.push(PreviewField {
                    label: "Dimensions".into(),
                    value: format!("{width} x {height}"),
                });
            }
            Err(err) => {
                let reason = format!("Image could not be decoded: {err}");
                payload.error = Some(reason.clone());
                payload.status = PreviewStatus::Error;
                payload.unsupported_reason = Some(reason);
                return;
            }
        }
        if let Some(ocr) = db.ocr_text_for_path(path) {
            let (ocr, ocr_truncated) = sanitize_preview_text(&ocr, max_chars);
            if !ocr.trim().is_empty() {
                payload.ocr_text = Some(ocr);
                if ocr_truncated {
                    payload.truncated = true;
                }
            }
        }
        return;
    }
    if !ps_content::is_supported(path_ref) {
        payload.unsupported_reason = Some(if extension.is_empty() {
            "Preview is not available for this file type.".to_string()
        } else {
            format!("Preview is not available for .{extension} files.")
        });
        return;
    }
    if let Some(metadata) = &metadata {
        if metadata.len() > PREVIEW_TEXT_MAX_BYTES {
            payload.unsupported_reason = Some(format!(
                "File is too large for preview ({}, limit {}).",
                format_bytes_human(metadata.len()),
                format_bytes_human(PREVIEW_TEXT_MAX_BYTES)
            ));
            return;
        }
    }
    match ps_content::extract_text(path_ref, ps_content::DEFAULT_MAX_BYTES) {
        Ok(Some(document)) => {
            let (text, truncated) = sanitize_preview_text(&document.text, max_chars);
            if text.trim().is_empty() {
                if extension == "pdf" {
                    if let Some(ocr) = db.ocr_text_for_path(path) {
                        let (ocr, ocr_truncated) = sanitize_preview_text(&ocr, max_chars);
                        if !ocr.trim().is_empty() {
                            payload.ocr_text = Some(ocr);
                            payload.truncated = ocr_truncated;
                            return;
                        }
                    }
                    payload.unsupported_reason = Some(
                        "This PDF has no text layer (scanned document); OCR preview is not available yet."
                            .to_string(),
                    );
                } else {
                    payload.unsupported_reason =
                        Some("No extractable text was found in this file.".to_string());
                }
                return;
            }
            payload.fields.push(PreviewField {
                label: "Lines".into(),
                value: text.lines().count().to_string(),
            });
            payload.fields.push(PreviewField {
                label: "Words".into(),
                value: text.split_whitespace().count().to_string(),
            });
            if truncated || document.truncated {
                payload.truncated = true;
                payload.fields.push(PreviewField {
                    label: "Note".into(),
                    value: "Preview truncated".into(),
                });
            }
            if filename_result {
                payload.highlight_ranges.clear();
            }
            if payload.snippet.is_none() {
                payload.snippet = first_line_excerpt(&text);
                payload.highlight_ranges.clear();
            }
            payload.text = Some(text);
            if payload.ocr_text.is_none() {
                if let Some(ocr) = db.ocr_text_for_path(path) {
                    let (ocr, ocr_truncated) = sanitize_preview_text(&ocr, max_chars);
                    if !ocr.trim().is_empty() {
                        payload.ocr_text = Some(ocr);
                        if ocr_truncated {
                            payload.truncated = true;
                        }
                    }
                }
            }
        }
        Ok(None) => {
            payload.unsupported_reason = Some(format!("Preview is not available for .{extension} files."));
        }
        Err(err) => {
            let reason: String = format!("{err:#}")
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(PREVIEW_FIELD_MAX_CHARS)
                .collect();
            let message = if reason.is_empty() {
                "Preview extraction failed.".to_string()
            } else {
                format!("Preview extraction failed: {reason}")
            };
            payload.error = Some(message.clone());
            payload.status = PreviewStatus::Error;
            payload.unsupported_reason = Some(message);
        }
    }
}

fn preview_format(result: &SearchResult) -> String {
    if result.kind == SourceKind::Folder {
        return "folder".to_string();
    }
    let extension = result.path.as_deref().map(extension_of).unwrap_or_default();
    if !extension.is_empty() {
        return extension;
    }
    match result.kind {
        SourceKind::Application => "application",
        SourceKind::Clipboard => "clipboard",
        SourceKind::Snippet => "snippet",
        SourceKind::BrowserHistory => "browser-history",
        SourceKind::Bookmark => "bookmark",
        SourceKind::Action => "action",
        SourceKind::Command => "command",
        SourceKind::Calculator => "calculator",
        SourceKind::Quicklink => "quicklink",
        SourceKind::GitCommit => "git-commit",
        SourceKind::GitRepository => "git-repository",
        SourceKind::GitTodo => "git-todo",
        SourceKind::Setting => "setting",
        SourceKind::Window => "window",
        SourceKind::Agent => "agent",
        SourceKind::System => "system",
        SourceKind::Document => "document",
        SourceKind::ContentMatch => "content",
        SourceKind::OcrText => "ocr",
        SourceKind::Recent => "recent",
        SourceKind::File => "file",
        SourceKind::Folder => "folder",
    }
    .to_string()
}

fn preview_fields(result: &SearchResult) -> Vec<PreviewField> {
    let mut fields = Vec::new();
    if let Some(label) = result.meta.label.as_deref() {
        fields.push(PreviewField {
            label: "Kind".into(),
            value: label.to_string(),
        });
    }
    if let Some(size) = result.meta.size {
        fields.push(PreviewField {
            label: "Size".into(),
            value: format_bytes_human(size),
        });
    }
    if let Some(modified) = result.meta.modified {
        fields.push(PreviewField {
            label: "Modified".into(),
            value: format_utc_ms(modified),
        });
    }
    if let Some(count) = result.meta.item_count {
        fields.push(PreviewField {
            label: "Contents".into(),
            value: format!("{count} items"),
        });
    }
    if let Some(source) = result.meta.source_label.as_deref() {
        fields.push(PreviewField {
            label: "Source".into(),
            value: source.to_string(),
        });
    }
    fields
}

fn clipboard_text(result: &SearchResult) -> Option<String> {
    match &result.action {
        ActionSpec::CopyText { text } => Some(text.clone()),
        _ => {
            let title = result.title.trim();
            if title.is_empty() || title.starts_with("[image:") {
                None
            } else {
                Some(result.title.clone())
            }
        }
    }
}

/// Read the clipboard image row for a result id (id form "clip:<rowid>").
fn clipboard_image_info(db_path: &Path, result_id: &str) -> Option<(String, u32, u32, Option<String>)> {
    let id: i64 = result_id.strip_prefix("clip:")?.parse().ok()?;
    let conn =
        rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(1000));
    conn.query_row(
        "SELECT image_path, ocr_text FROM clipboard_history WHERE id = ?1 AND image_path IS NOT NULL",
        rusqlite::params![id],
        |row| {
            let image_path: String = row.get(0)?;
            let ocr_text: Option<String> = row.get(1)?;
            Ok((image_path, ocr_text))
        },
    )
    .ok()
    .map(|(path, ocr)| (path, 0, 0, ocr))
}

fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn is_image_extension(extension: &str) -> bool {
    matches!(
        extension,
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff"
    )
}

/// Downscale an image to a bounded PNG data URL. Never upscales.
fn image_preview_data_url(path: &Path) -> Result<(String, u32, u32), String> {
    use image::io::Reader as ImageReader;
    let reader = ImageReader::open(path)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|err| err.to_string())?;
    let (width, height) = reader.into_dimensions().map_err(|err| err.to_string())?;
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || width as u64 * height as u64 > PREVIEW_IMAGE_MAX_DECODE_PIXELS
    {
        return Err(format!(
            "image dimensions exceed preview decode limit ({width} x {height})"
        ));
    }
    let mut reader = ImageReader::open(path)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|err| err.to_string())?;
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(PREVIEW_IMAGE_MAX_DECODE_PIXELS * 4);
    reader.limits(limits);
    let image = reader.decode().map_err(|err| err.to_string())?;
    let (width, height) = (image.width(), image.height());
    let thumbnail = image.thumbnail(PREVIEW_IMAGE_MAX_EDGE, PREVIEW_IMAGE_MAX_EDGE);
    let mut png = Vec::new();
    thumbnail
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| err.to_string())?;
    Ok((
        format!("data:image/png;base64,{}", base64_encode(&png)),
        width,
        height,
    ))
}

/// Bounded cache for clipboard thumbnail data URLs. The island renders the
/// clipboard card from every keystroke-search; re-reading + base64-encoding the
/// same thumbnails per search is pure repeated work. Keyed by path + mtime + len.
struct CachedThumbnail {
    modified_ms: i64,
    len: u64,
    data_url: Option<String>,
}

const THUMBNAIL_CACHE_CAP: usize = 128;

fn thumbnail_cache() -> &'static Mutex<HashMap<String, CachedThumbnail>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedThumbnail>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
pub(crate) fn thumbnail_cache_len() -> usize {
    thumbnail_cache().lock().map(|cache| cache.len()).unwrap_or(0)
}

/// Build a bounded PNG data URL from an on-disk thumbnail (no decode needed:
/// ps-sources writes bounded PNG thumbnails for clipboard image rows).
pub(crate) fn thumbnail_data_url(path: &str) -> Option<String> {
    // Base64 expands by 4/3: cap the encoded data URL at 128 KB as required by
    // the preview contract.
    const MAX_THUMB_BYTES: u64 = 96 * 1024;
    const MAX_ENCODED_CHARS: usize = 128 * 1024;
    let metadata = std::fs::metadata(path).ok()?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);
    let len = metadata.len();
    if let Ok(cache) = thumbnail_cache().lock() {
        if let Some(cached) = cache.get(path) {
            if cached.modified_ms == modified_ms && cached.len == len {
                return cached.data_url.clone();
            }
        }
    }
    let data_url = if len == 0 || len > MAX_THUMB_BYTES {
        None
    } else {
        std::fs::read(path).ok().and_then(|bytes| {
            let encoded = base64_encode(&bytes);
            if encoded.len() > MAX_ENCODED_CHARS {
                None
            } else {
                Some(format!("data:image/png;base64,{encoded}"))
            }
        })
    };
    if let Ok(mut cache) = thumbnail_cache().lock() {
        if cache.len() >= THUMBNAIL_CACHE_CAP {
            cache.clear();
        }
        cache.insert(
            path.to_string(),
            CachedThumbnail {
                modified_ms,
                len,
                data_url: data_url.clone(),
            },
        );
    }
    data_url
}

/// Bounded data URL for an image row: prefer the pre-generated thumbnail, then
/// fall back to a downscaled render of the full image (still bounded).
pub(crate) fn image_row_data_url(thumbnail_path: Option<&str>, image_path: Option<&str>) -> Option<String> {
    if let Some(thumbnail) = thumbnail_path {
        if let Some(data_url) = thumbnail_data_url(thumbnail) {
            return Some(data_url);
        }
    }
    let image_path = image_path?;
    let (data_url, _, _) = image_preview_data_url(Path::new(image_path)).ok()?;
    // Keep search payloads small even when no thumbnail exists.
    if data_url.len() > 128 * 1024 {
        return None;
    }
    Some(data_url)
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Cap text to a character budget and strip control characters (keep newlines
/// and tabs). Returns (text, truncated).
fn sanitize_preview_text(raw: &str, max_chars: usize) -> (String, bool) {
    let mut out = String::with_capacity(raw.len().min(max_chars * 4));
    let mut count = 0usize;
    let mut truncated = false;
    for ch in raw.chars() {
        if ch == '\r' {
            continue;
        }
        if count >= max_chars {
            truncated = true;
            break;
        }
        let clean = if ch == '\n' || ch == '\t' || !ch.is_control() {
            ch
        } else {
            ' '
        };
        out.push(clean);
        count += 1;
    }
    (out.trim_end().to_string(), truncated)
}

fn first_line_excerpt(text: &str) -> Option<String> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    if line.is_empty() {
        None
    } else {
        Some(line.chars().take(PREVIEW_SNIPPET_CHARS).collect())
    }
}

fn format_bytes_human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Minimal UTC formatter for display fields (no chrono dependency).
fn format_utc_ms(ms: i64) -> String {
    let seconds = ms.div_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        seconds_of_day / 3600,
        (seconds_of_day % 3600) / 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_png_header_is_rejected_before_pixel_decode() {
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 0])))
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png[16..20].copy_from_slice(&5000u32.to_be_bytes());
        png[20..24].copy_from_slice(&5000u32.to_be_bytes());
        let crc = png[12..29].iter().fold(!0u32, |crc, byte| {
            (0..8).fold(crc ^ *byte as u32, |crc, _| {
                (crc >> 1) ^ (0xEDB88320 & (0u32.wrapping_sub(crc & 1)))
            })
        }) ^ !0u32;
        png[29..33].copy_from_slice(&crc.to_be_bytes());
        let path = std::env::temp_dir().join(format!(
            "ps-preview-{}-{}.png",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, png).unwrap();
        let error = image_preview_data_url(&path).unwrap_err();
        let _ = std::fs::remove_file(path);
        assert!(error.contains("5000 x 5000"), "{error}");
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn sanitize_strips_control_chars_and_caps() {
        let (text, truncated) = sanitize_preview_text("a\u{0001}b\r\nc", 100);
        assert_eq!(text, "a b\nc");
        assert!(!truncated);
        let (text, truncated) = sanitize_preview_text("abcdef", 3);
        assert_eq!(text, "abc");
        assert!(truncated);
    }

    #[test]
    fn utc_formatter_is_correct_for_epoch() {
        assert_eq!(format_utc_ms(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_utc_ms(1_700_000_000_000), "2023-11-14 22:13 UTC");
    }

    #[test]
    fn human_bytes_are_sane() {
        assert_eq!(format_bytes_human(0), "0 B");
        assert_eq!(format_bytes_human(1023), "1023 B");
        assert_eq!(format_bytes_human(1024), "1.0 KB");
        assert_eq!(format_bytes_human(1536), "1.5 KB");
    }
}
