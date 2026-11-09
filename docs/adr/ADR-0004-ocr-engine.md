# ADR-0004: OCR engine — Windows.Media.Ocr first, others benchmarked

- Status: **Provisional** — native engine selected for v2 alpha; benchmark evidence required
- Owners: L1-C (OCR quality/perf), L2-C3 (implementation), L0

## Context

OCR is needed for screenshot and scanned-document search. It must be: local-only by default, no silent model download, bounded CPU/pixels/time, and it must index text asynchronously — never per query. Candidate engines: Windows.Media.Ocr (built into Windows, uses installed language packs), PaddleOCR/PP-OCR (larger model download, usually stronger on scene text), Tesseract (classic, language data files), plus vendor accelerators.

Legacy ProtonSearch already used Windows.Media.Ocr paths (WinRT) and had clipboard-image OCR, so the capability exists on the target OS without extra downloads.

## Decision

Use **Windows.Media.Ocr** for the v2 alpha pipeline:
- No model download, no license/redistribution burden, uses the user's installed language packs, low startup latency.
- Bounded pipeline: decode with the image crate, cap long edge/pixels, convert to SoftwareBitmap, run OCR with timeout, store text + language + confidence plus a content hash in FTS5.
- OCR runs in a background worker pool with pause/resume and is disabled by default; the UI reports availability truthfully ("OCR engine unavailable for this language pack") rather than a fake toggle.

The architecture keeps ps-ocr behind a small API (extract_image, available), so PaddleOCR/Tesseract can be added as an optional engine later without touching search.

## Consequences

- Recognition quality on low-quality scans/stylised UI text will be lower than PaddleOCR; this must be measured and reported, with the benchmark corpus and accuracy numbers, not asserted.
- Language coverage depends on installed Windows language packs; the report must list which languages were tested.
- CPU usage spikes during OCR indexing; bounded concurrency and battery-aware throttling are required.
- If measured accuracy on the fixed corpus is unacceptable, the ADR will be revisited with a PaddleOCR spike (size, startup, accuracy, license) before any download UI is built.
