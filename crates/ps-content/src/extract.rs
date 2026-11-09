//! Bounded text extraction for the content index.
//!
//! Every parser is run with explicit limits: a maximum input file size, a
//! maximum extracted-text size, a per-entry decompression cap, and a wall-clock
//! timeout enforced by the caller. Malformed input must yield an error (which
//! the indexer records), never a panic or an unbounded allocation.

use std::io::Read;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context};

use crate::DocKind;

/// Default cap on extracted text per document (bytes).
pub const DEFAULT_MAX_BYTES: usize = 512 * 1024;
/// Default wall-clock budget per document extraction.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
/// Files larger than this are rejected outright rather than parsed.
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
/// Decompressed bytes read from one archive entry (zip-bomb bound).
const MAX_ENTRY_BYTES: usize = 8 * 1024 * 1024;
/// Total decompressed bytes read from one archive (zip-bomb bound).
const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;
/// Maximum number of archive entries inspected.
const MAX_ARCHIVE_ENTRIES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractOptions {
    /// Hard cap on the returned text, in bytes (UTF-8 safe truncation).
    pub max_bytes: usize,
    /// Wall-clock timeout for one document.
    pub timeout: Duration,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl ExtractOptions {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            ..Self::default()
        }
    }
}

/// Extracted text plus the detected family and truncation state.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedDocument {
    pub kind: DocKind,
    pub text: String,
    pub truncated: bool,
}

/// Map a file extension to a supported document family.
pub fn detect_kind(path: &Path) -> DocKind {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "pdf" => DocKind::Pdf,
        "docx" | "docm" | "dotx" => DocKind::Docx,
        "pptx" | "pptm" | "potx" => DocKind::Pptx,
        "xlsx" | "xlsm" | "xltx" => DocKind::Xlsx,
        "epub" => DocKind::Epub,
        "md" | "markdown" | "mdx" | "rst" => DocKind::Markdown,
        "json" | "jsonc" | "jsonl" | "ndjson" => DocKind::Json,
        // Plain text and configuration formats that we decode as text.
        "txt" | "text" | "log" | "nfo" | "csv" | "tsv" | "yaml" | "yml" | "toml" | "ini"
        | "cfg" | "conf" | "properties" | "env" | "xml" | "html" | "htm" | "css" | "scss"
        | "less" | "srt" | "vtt" => DocKind::Text,
        // Source code families.
        "rs" | "py" | "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "vue" | "svelte" | "go"
        | "java" | "kt" | "kts" | "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "cs" | "fs" | "rb"
        | "php" | "swift" | "m" | "mm" | "sh" | "bash" | "zsh" | "ps1" | "psm1" | "bat" | "cmd"
        | "sql" | "graphql" | "gql" | "proto" | "lua" | "r" | "pl" | "scala" | "dart" | "ex"
        | "exs" | "erl" | "hs" | "clj" | "cljs" | "zig" | "nim" | "asm" | "s" | "vb" | "vbs"
        | "cmake" | "gradle" | "makefile" | "dockerfile" | "tf" | "hcl" | "nix" | "sol" => {
            DocKind::Code
        }
        _ => DocKind::Other,
    }
}

/// True when the file extension is one we can extract text from.
pub fn is_supported(path: &Path) -> bool {
    detect_kind(path) != DocKind::Other
}

/// Extract text from a supported file. Returns `Ok(None)` for unsupported
/// formats; malformed supported formats return an error (never panic).
pub fn extract_text(path: &Path, max_bytes: usize) -> anyhow::Result<Option<ExtractedDocument>> {
    extract_text_with(path, ExtractOptions::new(max_bytes))
}

/// Extract text with explicit bounds. Parsers that can plausibly hang or panic
/// (PDF, zip-based formats) run on a short-lived worker thread with a timeout.
pub fn extract_text_with(
    path: &Path,
    options: ExtractOptions,
) -> anyhow::Result<Option<ExtractedDocument>> {
    let kind = detect_kind(path);
    if kind == DocKind::Other {
        return Ok(None);
    }
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("reading metadata for {}", path.display()))?;
    if !metadata.is_file() {
        return Ok(None);
    }
    if metadata.len() > MAX_INPUT_BYTES {
        bail!(
            "file too large for extraction: {} bytes (limit {})",
            metadata.len(),
            MAX_INPUT_BYTES
        );
    }
    let max_bytes = options.max_bytes.max(1);
    let timeout = options.timeout.max(Duration::from_millis(250));
    match kind {
        // Cheap, bounded plain-text path: run inline.
        DocKind::Text | DocKind::Code | DocKind::Markdown | DocKind::Json => {
            extract_plain(path, kind, max_bytes).map(Some)
        }
        // Structured formats: isolate parsing so a pathological file cannot
        // block the indexing pipeline indefinitely.
        _ => {
            install_quiet_panic_hook();
            let owned = path.to_path_buf();
            let (tx, rx) = mpsc::channel();
            let worker = std::thread::Builder::new()
                .name("ps-content-extract".into())
                .spawn(move || {
                    let result = catch_unwind_result(|| extract_structured(&owned, kind, max_bytes));
                    let _ = tx.send(result);
                })
                .context("spawning extraction worker")?;
            match rx.recv_timeout(timeout) {
                Ok(result) => {
                    let _ = worker.join();
                    result.map(Some)
                }
                Err(_) => Err(anyhow!(
                    "extraction timed out after {:?} for {}",
                    timeout,
                    path.display()
                )),
            }
        }
    }
}

/// Run a parser, converting panics into errors (malformed input safety).
///
/// Third-party parsers (notably pdf-extract) can panic on malformed input; the
/// panic is contained to the extraction worker thread and converted into a
/// normal error that the indexer records in content_errors.
fn catch_unwind_result<F>(f: F) -> anyhow::Result<ExtractedDocument>
where
    F: FnOnce() -> anyhow::Result<ExtractedDocument>,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => {
            let detail = panic_payload_message(&payload);
            Err(anyhow!("parser panicked on malformed input: {detail}"))
        }
    }
}

fn panic_payload_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// Suppress the default panic printout for the extraction worker threads.
///
/// A caught panic still runs the global panic hook, which would spam stderr (and
/// any attached log) for every malformed PDF. This hook is installed once and
/// delegates to the previously installed hook for every other thread, so the
/// application's own hook behaviour is preserved.
fn install_quiet_panic_hook() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let quiet = std::thread::current()
                .name()
                .map(|name| name.starts_with("ps-content-extract"))
                .unwrap_or(false);
            if !quiet {
                previous(info);
            }
        }));
    });
}

fn extract_structured(
    path: &Path,
    kind: DocKind,
    max_bytes: usize,
) -> anyhow::Result<ExtractedDocument> {
    match kind {
        DocKind::Pdf => extract_pdf(path, max_bytes),
        DocKind::Docx => extract_docx(path, max_bytes),
        DocKind::Pptx => extract_pptx(path, max_bytes),
        DocKind::Xlsx => extract_xlsx(path, max_bytes),
        DocKind::Epub => extract_epub(path, max_bytes),
        other => Ok(ExtractedDocument {
            kind: other,
            text: String::new(),
            truncated: false,
        }),
    }
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

fn extract_plain(
    path: &Path,
    kind: DocKind,
    max_bytes: usize,
) -> anyhow::Result<ExtractedDocument> {
    let metadata = std::fs::metadata(path)?;
    let file_len = metadata.len();
    // Read enough bytes to cover the cap even for UTF-16 input.
    let read_cap = max_bytes
        .saturating_mul(3)
        .saturating_add(4)
        .min(MAX_INPUT_BYTES as usize);
    let bytes = read_prefix(path, read_cap)?;
    let decoded = decode_text(&bytes);
    let (decoded, truncated_text) = truncate_utf8(decoded, max_bytes);
    let text = sanitize_text(&decoded);
    let truncated = truncated_text || file_len > bytes.len() as u64;
    Ok(ExtractedDocument {
        kind,
        text,
        truncated,
    })
}

fn read_prefix(path: &Path, cap: usize) -> anyhow::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let mut buffer = Vec::with_capacity(cap.min(64 * 1024));
    file.by_ref()
        .take(cap as u64)
        .read_to_end(&mut buffer)
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(buffer)
}

/// Decode text with BOM awareness (UTF-8, UTF-16 LE/BE) and a Windows-1252
/// fallback for legacy files.
pub fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return decode_utf16(&bytes[2..], true);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return decode_utf16(&bytes[2..], false);
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            decoded.into_owned()
        }
    }
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let unit = if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        units.push(unit);
    }
    String::from_utf16_lossy(&units)
}

/// Truncate to a UTF-8 boundary at or below `max_bytes`.
pub fn truncate_utf8(mut text: String, max_bytes: usize) -> (String, bool) {
    if text.len() <= max_bytes {
        return (text, false);
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    (text, true)
}

/// Replace NULs and stray control characters; keep newlines and tabs.
pub fn sanitize_text(text: &str) -> String {
    if !text.chars().any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
        && !text.contains('\r')
    {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    // keep a single newline for CRLF
                } else {
                    out.push('\n');
                }
            }
            '\n' | '\t' => out.push(ch),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// PDF
// ---------------------------------------------------------------------------

fn extract_pdf(path: &Path, max_bytes: usize) -> anyhow::Result<ExtractedDocument> {
    let bytes = std::fs::read(path).with_context(|| format!("reading pdf {}", path.display()))?;
    let raw = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| anyhow!("pdf extraction failed: {e}"))?;
    let text = sanitize_text(&raw);
    let (text, truncated) = truncate_utf8(text, max_bytes);
    Ok(ExtractedDocument {
        kind: DocKind::Pdf,
        text,
        truncated,
    })
}

// ---------------------------------------------------------------------------
// Zip-based office formats
// ---------------------------------------------------------------------------

type ZipArchive = zip::ZipArchive<std::io::Cursor<Vec<u8>>>;

fn open_zip(path: &Path) -> anyhow::Result<ZipArchive> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .with_context(|| format!("opening zip container {}", path.display()))
}

fn read_zip_entry_lenient(
    archive: &mut ZipArchive,
    name: &str,
    cap: usize,
) -> Option<Vec<u8>> {
    let mut entry = archive.by_name(name).ok()?;
    if entry.size() > cap as u64 {
        return None;
    }
    let mut buffer = Vec::with_capacity(entry.size().min(cap as u64) as usize);
    entry.by_ref().take(cap as u64).read_to_end(&mut buffer).ok()?;
    Some(buffer)
}

fn zip_names(archive: &mut ZipArchive) -> Vec<String> {
    let count = archive.len().min(MAX_ARCHIVE_ENTRIES);
    let mut names = Vec::with_capacity(count);
    for index in 0..count {
        if let Ok(entry) = archive.by_index(index) {
            names.push(entry.name().to_string());
        }
    }
    names
}

/// Decode XML character entities (text events carry raw/escaped content).
fn unescape_text(raw: &str) -> String {
    match quick_xml::escape::unescape(raw) {
        Ok(value) => value.into_owned(),
        Err(_) => raw.to_string(),
    }
}

fn extract_docx(path: &Path, max_bytes: usize) -> anyhow::Result<ExtractedDocument> {
    let mut archive = open_zip(path)?;
    let xml = read_zip_entry_lenient(&mut archive, "word/document.xml", MAX_ENTRY_BYTES)
        .ok_or_else(|| anyhow!("docx is missing word/document.xml"))?;
    let xml = String::from_utf8_lossy(&xml);
    let text = extract_xml_text(&xml, &["t"], &["p"], &["tab"], &["instrText", "delText"]);
    let text = sanitize_text(&text);
    let (text, truncated) = truncate_utf8(text, max_bytes);
    Ok(ExtractedDocument {
        kind: DocKind::Docx,
        text,
        truncated,
    })
}

fn extract_pptx(path: &Path, max_bytes: usize) -> anyhow::Result<ExtractedDocument> {
    let mut archive = open_zip(path)?;
    let mut slides: Vec<(u64, String)> = Vec::new();
    for name in zip_names(&mut archive) {
        let lower = name.to_ascii_lowercase();
        if !(lower.starts_with("ppt/slides/slide") && lower.ends_with(".xml")) {
            continue;
        }
        let index = lower
            .trim_start_matches("ppt/slides/slide")
            .trim_end_matches(".xml")
            .parse::<u64>()
            .unwrap_or(u64::MAX);
        slides.push((index, name));
    }
    slides.sort_by_key(|(index, _)| *index);
    let mut text = String::new();
    let mut total = 0usize;
    for (_, name) in slides {
        if total >= MAX_ARCHIVE_BYTES || text.len() >= max_bytes {
            break;
        }
        let Some(xml) = read_zip_entry_lenient(&mut archive, &name, MAX_ENTRY_BYTES) else {
            continue;
        };
        total += xml.len();
        let xml = String::from_utf8_lossy(&xml);
        let slide = extract_xml_text(&xml, &["t"], &["p"], &["tab"], &[]);
        text.push_str(&slide);
        if !text.ends_with('\n') {
            text.push('\n');
        }
    }
    if text.trim().is_empty() {
        bail!("pptx contains no extractable text");
    }
    let text = sanitize_text(&text);
    let (text, truncated) = truncate_utf8(text, max_bytes);
    Ok(ExtractedDocument {
        kind: DocKind::Pptx,
        text,
        truncated,
    })
}

fn extract_xlsx(path: &Path, max_bytes: usize) -> anyhow::Result<ExtractedDocument> {
    let mut archive = open_zip(path)?;
    let shared_strings = read_zip_entry_lenient(&mut archive, "xl/sharedStrings.xml", MAX_ENTRY_BYTES)
        .map(|xml| parse_shared_strings(&String::from_utf8_lossy(&xml)))
        .unwrap_or_default();
    let mut sheets: Vec<(u64, String)> = Vec::new();
    for name in zip_names(&mut archive) {
        let lower = name.to_ascii_lowercase();
        if lower.starts_with("xl/worksheets/sheet") && lower.ends_with(".xml") {
            let index = lower
                .trim_start_matches("xl/worksheets/sheet")
                .trim_end_matches(".xml")
                .parse::<u64>()
                .unwrap_or(u64::MAX);
            sheets.push((index, name));
        }
    }
    sheets.sort_by_key(|(index, _)| *index);
    let mut text = String::new();
    for (_, name) in sheets {
        if text.len() >= max_bytes {
            break;
        }
        let Some(xml) = read_zip_entry_lenient(&mut archive, &name, MAX_ENTRY_BYTES) else {
            continue;
        };
        let xml = String::from_utf8_lossy(&xml);
        let sheet_text = parse_xlsx_sheet(&xml, &shared_strings);
        if !sheet_text.trim().is_empty() {
            text.push_str(&sheet_text);
            if !text.ends_with('\n') {
                text.push('\n');
            }
        }
    }
    if text.trim().is_empty() {
        bail!("xlsx contains no extractable text");
    }
    let text = sanitize_text(&text);
    let (text, truncated) = truncate_utf8(text, max_bytes);
    Ok(ExtractedDocument {
        kind: DocKind::Xlsx,
        text,
        truncated,
    })
}

fn parse_shared_strings(xml: &str) -> Vec<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buffer = Vec::new();
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_t = false;
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                if event.local_name().as_ref() == "t" {
                    in_t = true;
                }
            }
            Ok(Event::Text(text)) => {
                if in_t {
                    current.push_str(&unescape_text(&text.into_inner()));
                }
            }
            Ok(Event::End(event)) => match event.local_name().as_ref() {
                "t" => in_t = false,
                "si" => {
                    strings.push(std::mem::take(&mut current));
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    strings
}

fn parse_xlsx_sheet(xml: &str, shared: &[String]) -> String {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buffer = Vec::new();
    let mut out = String::new();
    let mut row = String::new();
    let mut value = String::new();
    let mut in_value = false;
    let mut in_inline_text = false;
    let mut cell_is_shared = false;
    let mut cell_is_inline = false;
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => match event.local_name().as_ref() {
                "c" => {
                    cell_is_shared = false;
                    cell_is_inline = false;
                    for attribute in event.attributes().flatten() {
                        if attribute.key.local_name().as_ref() == "t" {
                            let raw = attribute.unescape_value().unwrap_or_default();
                            cell_is_shared = raw == "s";
                            cell_is_inline = raw == "inlineStr";
                        }
                    }
                }
                "v" => {
                    in_value = true;
                    value.clear();
                }
                "t" if cell_is_inline => in_inline_text = true,
                _ => {}
            },
            Ok(Event::Text(text)) => {
                if in_value || in_inline_text {
                    value.push_str(&unescape_text(&text.into_inner()));
                }
            }
            Ok(Event::End(event)) => match event.local_name().as_ref() {
                "v" => {
                    in_value = false;
                    let decoded = if cell_is_shared {
                        value
                            .trim()
                            .parse::<usize>()
                            .ok()
                            .and_then(|index| shared.get(index).cloned())
                            .unwrap_or_default()
                    } else {
                        value.clone()
                    };
                    if !decoded.trim().is_empty() {
                        if !row.is_empty() {
                            row.push('\t');
                        }
                        row.push_str(decoded.trim());
                    }
                }
                "t" => in_inline_text = false,
                "c" => {
                    if cell_is_inline && !value.trim().is_empty() {
                        if !row.is_empty() {
                            row.push('\t');
                        }
                        row.push_str(value.trim());
                    }
                    value.clear();
                }
                "row" => {
                    out.push_str(&row);
                    out.push('\n');
                    row.clear();
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    out
}

fn extract_epub(path: &Path, max_bytes: usize) -> anyhow::Result<ExtractedDocument> {
    let mut archive = open_zip(path)?;
    let mut entries: Vec<String> = zip_names(&mut archive)
        .into_iter()
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm")
        })
        .collect();
    entries.sort();
    let mut text = String::new();
    let mut total = 0usize;
    for name in entries {
        if text.len() >= max_bytes || total >= MAX_ARCHIVE_BYTES {
            break;
        }
        let Some(bytes) = read_zip_entry_lenient(&mut archive, &name, MAX_ENTRY_BYTES) else {
            continue;
        };
        total += bytes.len();
        let markup = String::from_utf8_lossy(&bytes);
        let fragment =
            extract_xml_text(&markup, &[], &["p", "div", "br"], &[], &["script", "style"]);
        text.push_str(&fragment);
        if !text.ends_with('\n') {
            text.push('\n');
        }
    }
    if text.trim().is_empty() {
        bail!("epub contains no extractable text");
    }
    let text = sanitize_text(&text);
    let (text, truncated) = truncate_utf8(text, max_bytes);
    Ok(ExtractedDocument {
        kind: DocKind::Epub,
        text,
        truncated,
    })
}

// ---------------------------------------------------------------------------
// XML text extraction (shared by DOCX/PPTX/EPUB)
// ---------------------------------------------------------------------------

/// Collect text nodes from an XML document.
///
/// - `text_tags` names whose inner text is captured (empty = capture all text).
/// - `newline_tags` append a newline when their element ends.
/// - `tab_tags` append a tab when their element ends.
/// - `skip_tags` subtrees that are ignored entirely.
fn extract_xml_text(
    xml: &str,
    text_tags: &[&str],
    newline_tags: &[&str],
    tab_tags: &[&str],
    skip_tags: &[&str],
) -> String {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buffer = Vec::new();
    let mut out = String::new();
    let mut depth: usize = 0;
    let mut skip_depth: usize = 0;
    let mut capture_depth: Option<usize> = None;
    let capture_all = text_tags.is_empty();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                depth += 1;
                let local = event.local_name();
                let name = local.as_ref();
                if skip_tags.iter().any(|tag| *tag == name) {
                    skip_depth += 1;
                } else if (capture_all || text_tags.iter().any(|tag| *tag == name))
                    && capture_depth.is_none()
                {
                    capture_depth = Some(depth);
                }
            }
            Ok(Event::Empty(event)) => {
                let local = event.local_name();
                let name = local.as_ref();
                if tab_tags.iter().any(|tag| *tag == name) {
                    out.push('\t');
                } else if newline_tags.iter().any(|tag| *tag == name) {
                    out.push('\n');
                }
            }
            Ok(Event::Text(text)) => {
                if skip_depth == 0 && capture_depth.is_some() {
                    out.push_str(&unescape_text(&text.into_inner()));
                }
            }
            Ok(Event::CData(data)) => {
                if skip_depth == 0 && capture_depth.is_some() {
                    out.push_str(&data.into_inner());
                }
            }
            Ok(Event::End(event)) => {
                let local = event.local_name();
                let name = local.as_ref();
                if skip_tags.iter().any(|tag| *tag == name) {
                    skip_depth = skip_depth.saturating_sub(1);
                } else {
                    if capture_depth == Some(depth) {
                        capture_depth = None;
                    }
                    if skip_depth == 0 {
                        if tab_tags.iter().any(|tag| *tag == name) {
                            out.push('\t');
                        } else if newline_tags.iter().any(|tag| *tag == name) {
                            out.push('\n');
                        }
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catch_unwind_result_converts_panics_to_errors() {
        let result = catch_unwind_result(|| panic!("pdf-extract simulated panic"));
        let error = result.expect_err("panic must become an error");
        assert!(
            error.to_string().contains("panicked"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn catch_unwind_result_passes_through_ok_values() {
        let document = ExtractedDocument {
            kind: DocKind::Text,
            text: "hello".to_string(),
            truncated: false,
        };
        let result = catch_unwind_result(|| Ok(document.clone())).unwrap();
        assert_eq!(result, document);
    }
}
