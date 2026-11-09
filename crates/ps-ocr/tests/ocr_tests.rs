//! Real OCR pipeline tests: capability probe, generated-PNG text recall,
//! corrupt inputs, pixel bounds and storage/search.

use std::path::{Path, PathBuf};

use ps_ocr::{extract_image, extract_image_timed, OcrIndex, OcrDocument};

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

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
fn available_reports_real_capability() {
    let available = ps_ocr::available();
    let languages = ps_ocr::available_languages();
    eprintln!("Windows.Media.Ocr available: {available}; languages: {languages:?}");
    // The value must be stable for the process and truthful about the host.
    assert_eq!(available, ps_ocr::available());
    if available {
        assert!(
            !languages.is_empty(),
            "an available OCR engine must report at least one language"
        );
    } else {
        eprintln!("UNVERIFIED: Windows.Media.Ocr is not available on this host");
    }
}

#[test]
fn extracts_known_text_from_generated_png() {
    if !ps_ocr::available() {
        eprintln!("UNVERIFIED: Windows OCR unavailable; skipping real-text test");
        return;
    }
    let path = fixture("ocr_sample.png");
    assert!(path.is_file(), "missing fixture: {}", path.display());
    let document = extract_image(&path, 16_000_000)
        .expect("OCR must not error on a clean PNG")
        .expect("OCR must find text in the fixture");
    let normalized = normalize(&document.text);
    eprintln!(
        "OCR text: {:?} (language={}, {}x{})",
        document.text, document.language, document.width, document.height
    );
    let expected = ["protons", "search", "ocr", "test", "2026"];
    let recall = expected
        .iter()
        .filter(|word| normalized.contains(&word.to_string()))
        .count() as f32
        / expected.len() as f32;
    eprintln!("word recall against expected tokens: {recall:.2}");
    assert!(
        recall >= 0.75,
        "OCR recall too low ({recall:.2}) for text {:?}",
        document.text
    );
    assert!(
        normalized.contains("protons") && normalized.contains("2026"),
        "expected product name and year in {:?}",
        document.text
    );
}

#[test]
fn extracts_numbers_from_generated_png() {
    if !ps_ocr::available() {
        eprintln!("UNVERIFIED: Windows OCR unavailable; skipping numbers test");
        return;
    }
    let path = fixture("ocr_numbers.png");
    let document = extract_image(&path, 16_000_000).unwrap().expect("text");
    let normalized = normalize(&document.text);
    eprintln!("numbers fixture OCR: {:?}", document.text);
    assert!(normalized.contains("1284"), "expected 1284 in {:?}", document.text);
}

#[test]
fn corrupt_image_errors_without_panicking() {
    let dir = workdir("ocr-corrupt");
    let path = dir.join("broken.png");
    std::fs::write(&path, b"this is not a png at all").unwrap();
    match extract_image(&path, 16_000_000) {
        Ok(None) => {}
        Ok(Some(document)) => panic!("garbage decoded as text: {document:?}"),
        Err(error) => assert!(!error.to_string().is_empty()),
    }
}

#[test]
fn unsupported_extension_is_skipped() {
    let dir = workdir("ocr-unsupported");
    let path = dir.join("notes.txt");
    std::fs::write(&path, b"hello").unwrap();
    assert_eq!(extract_image(&path, 16_000_000).unwrap(), None);
}

#[test]
fn pixel_bounds_are_enforced() {
    let dir = workdir("ocr-bounds");
    let path = dir.join("huge.png");
    // 6000x200 white image: must be downscaled to the long-edge/pixel caps.
    let image = image::RgbaImage::from_pixel(6000, 200, image::Rgba([255, 255, 255, 255]));
    image.save_with_format(&path, image::ImageFormat::Png).unwrap();
    let result = extract_image(&path, 500_000);
    match result {
        Ok(Some(document)) => {
            assert!(
                document.width as u64 * document.height as u64 <= 500_000,
                "OCR input exceeded pixel cap: {}x{}",
                document.width,
                document.height
            );
            assert!(document.width <= ps_ocr::MAX_LONG_EDGE);
        }
        Ok(None) => {}
        Err(error) => panic!("bounded OCR must not fail on a valid large image: {error}"),
    }
}

#[test]
fn timed_api_reports_elapsed() {
    let dir = workdir("ocr-timed");
    let path = dir.join("small.png");
    let image = image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 255, 255, 255]));
    image.save_with_format(&path, image::ImageFormat::Png).unwrap();
    let (_document, elapsed) = extract_image_timed(&path, 1_000_000).unwrap();
    assert!(elapsed.as_millis() < 60_000, "unexpected duration {elapsed:?}");
}

#[test]
fn ocr_storage_dedupes_by_hash_and_searches() {
    let dir = workdir("ocr-storage");
    let index = OcrIndex::open(&dir.join("index.db")).unwrap();
    let document = OcrDocument {
        text: "ProtonSearch screenshot note about quarterly budget".to_string(),
        language: "en-US".to_string(),
        confidence: 1.0,
        width: 640,
        height: 480,
    };
    assert!(index
        .store("C:\\shots\\budget.png", "hash-a", &document)
        .unwrap());
    assert!(
        !index
            .store("C:\\shots\\budget.png", "hash-a", &document)
            .unwrap(),
        "same content hash must be deduplicated"
    );
    let hits = index.search("quarterly", 10).unwrap();
    assert_eq!(hits.len(), 1, "hits={hits:?}");
    assert_eq!(hits[0].lang, "en-US");
    let (documents, bytes) = index.stats();
    assert_eq!(documents, 1);
    assert!(bytes > 0);
    assert!(index.search("'); DROP TABLE ocr_documents; --", 10).is_err() || index.stats().0 == 1);
    index.remove("C:\\shots\\budget.png").unwrap();
    assert!(index.search("quarterly", 10).unwrap().is_empty());
}

#[test]
fn ocr_query_sanitisation_quotes_terms() {
    assert_eq!(ps_ocr::store::sanitize_fts_query("alpha beta"), "\"alpha\" \"beta\"");
    assert!(ps_ocr::store::sanitize_fts_query("((( )))").is_empty());
}
