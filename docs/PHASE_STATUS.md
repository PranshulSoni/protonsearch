# Phase status

Phase 1 OCR engine integration, dynamic discovery, in-app model download pipeline, and onboarding/settings UI are complete.

### Implemented & Verified in Phase 1
- **Multi-Engine Dispatch & Runtime Packaging**:
  - `ps-ocr` links ONNX Runtime (`oar-ocr/download-binaries`, `ort-sys`).
  - `extract_image_with_engine` dispatches to Windows OCR, PP-OCRv6 (ONNX), Indra (ONNX), or Tesseract.
  - Crash-safe atomic backup-and-rename activation with recovery (`recover_model_dir`) in `ppocr_pack.rs`.
  - Dynamic availability checks: `is_engine_available` inspects model files on disk (`v2_data_dir().join("models/...")`) and system PATH for Tesseract.
- **In-App Download & Activation Flow**:
  - Backend exposed Tauri IPC commands `get_ocr_engines` and `download_ocr_engine`.
  - Desktop bridge connected in `api.ts` and `mock.ts`.
  - Downloading PP-OCRv6 verifies SHA-256 hashes against pinned releases, stages, atomically renames, updates settings, and persists configuration.
- **High-Grade Onboarding & Preferences UI**:
  - Replaced plain dropdown with rich, dark-themed card grid in `OcrEngineSelector.tsx`.
  - Clear distinctions: speed badges (`~30ms`, `~45ms`, `~120ms`, `~150ms`), footprint labels (`~6.1 MB`, `Built-in (0 MB)`, `~27 MB`), highlights bullets, and license disclosures.
  - Interactive actions: Instant "Select" for available engines, 1-click "Download & Activate (~6.1 MB)" for PP-OCRv6 with live progress, and modal license agreement for Indra OCR.
- **Test & Build Verification**:
  - Vitest desktop test suite: **265/265 tests passed**.
  - `ps-ocr` suite: 9 unit tests + 9 integration tests + 100-image `phase1_corpus` passed.
  - `ps-core` suite: 64/64 tests passed.
  - Full desktop release binary built with embedded frontend via `tauri build --no-bundle` and deployed to `C:\Users\Pranshul\ProtonSearch-Release\ProtonSearch.exe`.
