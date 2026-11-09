//! Bounded OCR extraction pipeline. OWNER: sources-content.
//!
//! Strategy: Windows.Media.Ocr (native, no model download, fast) for the
//! default path; the pipeline is decoupled from search, deduplicated by content
//! hash, bounded in decoded pixels/time, and results are stored as text in FTS5.
//! OCR is NEVER run per query.

use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct OcrDocument {
    pub text: String,
    pub language: String,
    pub confidence: f32,
    pub width: u32,
    pub height: u32,
}

/// Extract text from one image file. Bounded by pixel count and timeout.
pub fn extract_image(_path: &Path, _max_pixels: u64) -> anyhow::Result<Option<OcrDocument>> {
    Ok(None)
}

/// True when the current OS exposes a usable OCR engine.
pub fn available() -> bool {
    false
}

/// Supported image extensions for the OCR pipeline.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp", "tif", "tiff"];
