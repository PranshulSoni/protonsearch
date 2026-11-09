//! Windows.Media.Ocr implementation (WinRT).

use std::path::Path;

use anyhow::{anyhow, bail};
use windows::core::HSTRING;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Security::Cryptography::CryptographicBuffer;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};

use crate::{OcrDocument, MAX_IMAGE_PIXELS, MAX_LONG_EDGE};

thread_local! {
    static APARTMENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Initialise WinRT on this thread. Returns true when this call owns the
/// initialisation and must balance it with RoUninitialize.
fn ensure_apartment() -> bool {
    if APARTMENT.with(|cell| cell.get()) {
        return false;
    }
    match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        Ok(()) => {
            APARTMENT.with(|cell| cell.set(true));
            true
        }
        // RPC_E_CHANGED_MODE: the thread already lives in another apartment,
        // which is usable for WinRT activation.
        Err(_) => false,
    }
}

pub(crate) fn available() -> bool {
    let _ = ensure_apartment();
    OcrEngine::TryCreateFromUserProfileLanguages().is_ok() || fallback_engine().is_ok()
}

pub(crate) fn available_languages() -> Vec<String> {
    let _ = ensure_apartment();
    let mut languages = Vec::new();
    if let Ok(collection) = OcrEngine::AvailableRecognizerLanguages() {
        let size = collection.Size().unwrap_or(0);
        for index in 0..size {
            if let Ok(language) = collection.GetAt(index) {
                if let Ok(tag) = language.LanguageTag() {
                    languages.push(tag.to_string());
                }
            }
        }
    }
    languages
}

pub(crate) fn recognize(path: &Path, max_pixels: u64) -> anyhow::Result<Option<OcrDocument>> {
    let owns_apartment = ensure_apartment();
    let result = recognize_inner(path, max_pixels);
    if owns_apartment {
        unsafe { RoUninitialize() };
    }
    result
}

fn recognize_inner(path: &Path, max_pixels: u64) -> anyhow::Result<Option<OcrDocument>> {
    if !available() {
        return Ok(None);
    }
    let image = image::open(path)
        .map_err(|error| anyhow!("decoding image {}: {error}", path.display()))?;
    let rgba = image.to_rgba8();
    let (mut width, mut height) = rgba.dimensions();
    if width == 0 || height == 0 {
        bail!("image has zero dimensions: {}", path.display());
    }

    // Bounded decoding: clamp both the long edge and the total pixel count.
    let pixel_cap = max_pixels.clamp(1, MAX_IMAGE_PIXELS);
    let pixels = width as u64 * height as u64;
    let long_edge = width.max(height) as f64;
    let mut scale = 1.0f64;
    if long_edge > MAX_LONG_EDGE as f64 {
        scale = scale.min(MAX_LONG_EDGE as f64 / long_edge);
    }
    if pixels > pixel_cap {
        scale = scale.min((pixel_cap as f64 / pixels as f64).sqrt());
    }
    let resized;
    let rgba = if scale < 1.0 {
        let new_width = ((width as f64 * scale).round() as u32).max(1);
        let new_height = ((height as f64 * scale).round() as u32).max(1);
        resized = image::imageops::resize(
            &rgba,
            new_width,
            new_height,
            image::imageops::FilterType::Triangle,
        );
        width = new_width;
        height = new_height;
        &resized
    } else {
        &rgba
    };

    // Windows OCR needs BGRA8; alpha is forced opaque (screenshots).
    let mut bgra = Vec::with_capacity(width as usize * height as usize * 4);
    for pixel in rgba.pixels() {
        bgra.push(pixel[2]);
        bgra.push(pixel[1]);
        bgra.push(pixel[0]);
        bgra.push(255);
    }
    let buffer = CryptographicBuffer::CreateFromByteArray(&bgra)
        .map_err(|error| anyhow!("creating WinRT buffer: {error}"))?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        width as i32,
        height as i32,
    )
    .map_err(|error| anyhow!("creating SoftwareBitmap: {error}"))?;

    let engine = match OcrEngine::TryCreateFromUserProfileLanguages() {
        Ok(engine) => engine,
        Err(_) => fallback_engine()?,
    };
    let operation = engine
        .RecognizeAsync(&bitmap)
        .map_err(|error| anyhow!("starting OCR recognition: {error}"))?;
    let result = operation
        .get()
        .map_err(|error| anyhow!("Windows OCR failed: {error}"))?;
    let text = result
        .Text()
        .map(|text| text.to_string())
        .unwrap_or_default();
    let text = text.trim().to_string();
    let language = engine
        .RecognizerLanguage()
        .ok()
        .and_then(|language| language.LanguageTag().ok())
        .map(|tag| tag.to_string())
        .unwrap_or_default();
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(OcrDocument {
        text,
        language,
        confidence: 1.0,
        width,
        height,
    }))
}

fn fallback_engine() -> anyhow::Result<OcrEngine> {
    use windows::Globalization::Language;
    let language = Language::CreateLanguage(&HSTRING::from("en-US"))
        .map_err(|error| anyhow!("creating en-US language: {error}"))?;
    OcrEngine::TryCreateFromLanguage(&language)
        .map_err(|error| anyhow!("creating OCR engine: {error}"))
}
