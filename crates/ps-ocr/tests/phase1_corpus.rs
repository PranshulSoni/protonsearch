//! Phase 1 regression corpus: 100 generated, mixed-format variants of the
//! repository's synthetic OCR fixtures. No user documents are read.
use image::{imageops, DynamicImage, ImageFormat, Rgba, RgbaImage};
use ps_ocr::extract_image_timed;
use std::path::{Path, PathBuf};

const CORPUS_SIZE: usize = 100;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect()
}

#[test]
fn generated_corpus_extracts_expected_text() {
    if !ps_ocr::available() {
        eprintln!("UNVERIFIED: Windows OCR unavailable; 0/{CORPUS_SIZE} attempted");
        return;
    }

    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ocr-phase1-corpus");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let sources = [
        (fixture("ocr_sample.png"), "protons"),
        (fixture("ocr_numbers.png"), "1284"),
    ];
    let formats = [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Bmp];
    let mut corpus = Vec::with_capacity(CORPUS_SIZE);

    for (source_index, (source_path, expected)) in sources.iter().enumerate() {
        let source = image::open(source_path).expect("synthetic OCR fixture must decode");
        for variant in 0..(CORPUS_SIZE / sources.len()) {
            let scale = 0.96 + (variant % 5) as f32 * 0.02;
            let scaled = source.resize(
                (source.width() as f32 * scale).round() as u32,
                (source.height() as f32 * scale).round() as u32,
                imageops::FilterType::Lanczos3,
            );
            let pad_x = (variant % 7) as u32 + 2;
            let pad_y = (variant % 5) as u32 + 2;
            let mut canvas = RgbaImage::from_pixel(
                scaled.width() + pad_x * 2,
                scaled.height() + pad_y * 2,
                Rgba([255, 255, 255, 255]),
            );
            imageops::overlay(&mut canvas, &scaled.to_rgba8(), pad_x as i64, pad_y as i64);
            let format = formats[(variant + source_index) % formats.len()];
            let extension = match format {
                ImageFormat::Jpeg => "jpg",
                ImageFormat::Bmp => "bmp",
                _ => "png",
            };
            let path = dir.join(format!("sample-{source_index}-{variant}.{extension}"));
            DynamicImage::ImageRgba8(canvas)
                .save_with_format(&path, format)
                .expect("generated corpus image must save");
            corpus.push((path, *expected));
        }
    }

    let mut attempted = 0usize;
    let mut success = 0usize;
    let mut failed = Vec::new();
    for (path, expected) in &corpus {
        attempted += 1;
        match extract_image_timed(path, ps_ocr::MAX_IMAGE_PIXELS) {
            Ok((Some(document), _elapsed)) if normalize(&document.text).contains(expected) => {
                success += 1;
            }
            Ok((document, _elapsed)) => failed.push(format!(
                "{}: expected {expected:?}, got {:?}",
                path.file_name().unwrap().to_string_lossy(),
                document.map(|document| document.text)
            )),
            Err(error) => failed.push(format!("{}: {error}", path.display())),
        }
    }

    eprintln!(
        "Phase 1 OCR corpus: attempted={attempted}, success={success}, failed={}",
        failed.len()
    );
    assert_eq!(attempted, CORPUS_SIZE, "corpus must exercise 100 images");
    assert!(failed.is_empty(), "OCR corpus failures: {failed:?}");
}
