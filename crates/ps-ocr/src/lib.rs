//! Bounded OCR extraction pipeline. OWNER: sources-content.
//!
//! Strategy: Windows.Media.Ocr (native, no model download) for the default
//! path; the pipeline is decoupled from search, deduplicated by content hash,
//! bounded in decoded pixels/time, and OCR is NEVER run per query.
//!
//! Confidence note: Windows.Media.Ocr does not expose a per-result confidence
//! score. OcrDocument.confidence is therefore a boolean-capability signal
//! (1.0 = engine returned text, 0.0 = no text). Benchmarks must measure
//! character/word error rate against ground truth, not rely on this field.

use std::path::Path;
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail};
use sha2::{Digest, Sha256};

#[cfg(windows)]
mod platform;

pub mod store;

pub use store::{OcrHit, OcrIndex};

#[derive(Debug, Clone, PartialEq)]
pub struct OcrDocument {
    pub text: String,
    pub language: String,
    /// Heuristic flag, not a calibrated probability (see module docs).
    pub confidence: f32,
    pub width: u32,
    pub height: u32,
}

/// Supported image extensions for the OCR pipeline.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp", "tif", "tiff"];

/// Hard cap on the long edge of any image we send to the OCR engine.
pub const MAX_LONG_EDGE: u32 = 4096;
/// Hard cap on decoded pixels (16 MP).
pub const MAX_IMAGE_PIXELS: u64 = 16_000_000;
/// Per-image wall-clock budget.
pub const OCR_TIMEOUT: Duration = Duration::from_secs(20);

/// True when the current OS exposes a usable OCR engine.
pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(platform_available)
}

#[cfg(windows)]
fn platform_available() -> bool {
    platform::available()
}

#[cfg(not(windows))]
fn platform_available() -> bool {
    false
}

/// Recognizer languages installed on this machine (empty when unavailable).
pub fn available_languages() -> Vec<String> {
    #[cfg(windows)]
    {
        platform::available_languages()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// True when the extension is one the OCR pipeline accepts.
pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .map(|ext| {
            let ext = ext.to_string_lossy().to_ascii_lowercase();
            IMAGE_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

/// SHA-256 hex of raw bytes (used for OCR dedupe by content hash).
pub fn content_hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// SHA-256 hex of an image file (streamed, bounded memory).
pub fn content_hash_file(path: &Path) -> anyhow::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|error| anyhow!("opening {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Extract text from one image file. Bounded by decoded pixels and timeout.
/// Returns Ok(None) for unsupported extensions, empty results and when no OCR
/// engine is available on the machine.
pub fn extract_image(path: &Path, max_pixels: u64) -> anyhow::Result<Option<OcrDocument>> {
    extract_image_timed(path, max_pixels).map(|(document, _elapsed)| document)
}

/// Extract text and report per-image wall-clock timing (benchmark entry point).
pub fn extract_image_timed(
    path: &Path,
    max_pixels: u64,
) -> anyhow::Result<(Option<OcrDocument>, Duration)> {
    if !is_supported_image(path) {
        return Ok((None, Duration::ZERO));
    }
    if !path.is_file() {
        bail!("image not found: {}", path.display());
    }
    let started = Instant::now();
    let owned = path.to_path_buf();
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("ps-ocr-worker".into())
        .spawn(move || {
            let result = recognize_bounded(&owned, max_pixels);
            let _ = sender.send(result);
        })
        .map_err(|error| anyhow!("spawning OCR worker: {error}"))?;
    match receiver.recv_timeout(OCR_TIMEOUT) {
        Ok(result) => Ok((result?, started.elapsed())),
        Err(_) => Err(anyhow!(
            "OCR timed out after {:?} for {}",
            OCR_TIMEOUT,
            path.display()
        )),
    }
}

#[cfg(windows)]
fn recognize_bounded(path: &Path, max_pixels: u64) -> anyhow::Result<Option<OcrDocument>> {
    platform::recognize(path, max_pixels)
}

#[cfg(not(windows))]
fn recognize_bounded(_path: &Path, _max_pixels: u64) -> anyhow::Result<Option<OcrDocument>> {
    Ok(None)
}
