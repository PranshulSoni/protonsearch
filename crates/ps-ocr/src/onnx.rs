//! Local ONNX OCR adapters. Model assets are supplied by the caller; this
//! module does not download models or runtime binaries.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use image::DynamicImage;
use oar_ocr::indra::{IndraOcr, IndraOcrBuilder};
use oar_ocr::oarocr::{OAROCRBuilder, OAROCR};

/// A CPU ONNX OCR pipeline built from local model files.
pub enum OnnxOcr {
    /// PP-OCRv6 tiny, with the matching tiny dictionary.
    PpOcrV6Tiny(OAROCR),
    /// Indra OCR from a locally installed checkpoint.
    Indra(IndraOcr),
}

impl OnnxOcr {
    /// Loads PP-OCRv6 tiny assets from `model_dir`.
    pub fn pp_ocrv6_tiny(model_dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let model_dir = model_dir.as_ref();
        let detection = model_dir.join("pp-ocrv6_tiny_det.onnx");
        let recognition = model_dir.join("pp-ocrv6_tiny_rec.onnx");
        let dictionary = model_dir.join("ppocrv6_tiny_dict.txt");
        for path in [&detection, &recognition, &dictionary] {
            require_file(path)?;
        }
        let pipeline = OAROCRBuilder::new(detection, recognition, dictionary)
            .build()
            .context("loading local PP-OCRv6 tiny pipeline")?;
        Ok(Self::PpOcrV6Tiny(pipeline))
    }

    /// Loads Indra OCR assets from a local checkpoint directory.
    pub fn indra(model_dir: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let pipeline = IndraOcrBuilder::new(model_dir)
            .build()
            .context("loading local Indra OCR pipeline")?;
        Ok(Self::Indra(pipeline))
    }

    /// Recognizes text from an already decoded and size-bounded image.
    pub fn recognize(&self, image: DynamicImage) -> anyhow::Result<String> {
        let (width, height) = (image.width(), image.height());
        if width > crate::MAX_LONG_EDGE || height > crate::MAX_LONG_EDGE {
            bail!(
                "image exceeds OCR long-edge limit of {} pixels",
                crate::MAX_LONG_EDGE
            );
        }
        if u64::from(width) * u64::from(height) > crate::MAX_IMAGE_PIXELS {
            bail!(
                "image exceeds OCR pixel limit of {} pixels",
                crate::MAX_IMAGE_PIXELS
            );
        }
        let image = image.to_rgb8();
        let image = image25::RgbImage::from_raw(width, height, image.into_raw())
            .context("converting image for ONNX OCR")?;
        match self {
            Self::PpOcrV6Tiny(pipeline) => Ok(pipeline
                .predict(vec![image])?
                .into_iter()
                .next()
                .map(|result| result.concatenated_text("\n"))
                .unwrap_or_default()),
            Self::Indra(pipeline) => Ok(pipeline
                .predict(vec![image])?
                .into_iter()
                .next()
                .map(|page| page.markdown)
                .unwrap_or_default()),
        }
    }
}

use std::sync::Mutex;

static CACHED_PPOCR: Mutex<Option<(PathBuf, OnnxOcr)>> = Mutex::new(None);
static CACHED_INDRA: Mutex<Option<(PathBuf, OnnxOcr)>> = Mutex::new(None);

/// Recognizes text using a cached PP-OCRv6 tiny pipeline.
pub fn recognize_ppocrv6(model_dir: &Path, image: DynamicImage) -> anyhow::Result<String> {
    let mut guard = CACHED_PPOCR.lock().map_err(|e| anyhow::anyhow!("mutex poisoned: {e}"))?;
    if let Some((cached_dir, pipeline)) = guard.as_ref() {
        if cached_dir == model_dir {
            return pipeline.recognize(image);
        }
    }
    let pipeline = OnnxOcr::pp_ocrv6_tiny(model_dir)?;
    let result = pipeline.recognize(image);
    *guard = Some((model_dir.to_path_buf(), pipeline));
    result
}

/// Recognizes text using a cached Indra OCR pipeline.
pub fn recognize_indra(model_dir: &Path, image: DynamicImage) -> anyhow::Result<String> {
    let mut guard = CACHED_INDRA.lock().map_err(|e| anyhow::anyhow!("mutex poisoned: {e}"))?;
    if let Some((cached_dir, pipeline)) = guard.as_ref() {
        if cached_dir == model_dir {
            return pipeline.recognize(image);
        }
    }
    let pipeline = OnnxOcr::indra(model_dir)?;
    let result = pipeline.recognize(image);
    *guard = Some((model_dir.to_path_buf(), pipeline));
    result
}

fn require_file(path: &Path) -> anyhow::Result<()> {
    if !path.is_file() {
        bail!("required local OCR model asset not found: {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pp_ocrv6_requires_all_local_assets_without_downloading() {
        let dir = std::env::temp_dir().join(format!("ps-ocr-models-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let result = OnnxOcr::pp_ocrv6_tiny(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.is_err());
    }
}
