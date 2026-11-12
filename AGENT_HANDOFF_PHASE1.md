# ProtonSearch Phase 1 handoff

## Goal

Complete Phase 1 OCR, indexing, search and preview work. The user specifically wants three usable OCR engines in onboarding and app settings:

- **PP-OCRv6**: the default for fresh installations once it works; approximately 6.1 MB of model files plus runtime.
- **Indra OCR**: an alternative for skewed and multilingual text; approximately 27 MB of model files. Present its license for acceptance before downloading.
- **Tesseract OCR**: a traditional alternative with separately installed language data; size varies by language.

Explain the differences and installation status clearly. Preserve existing users' Windows OCR settings and data. Do not represent an engine as usable until it is installed, integrated and tested.

## Starting point

- Workspace: `C:\Users\Pranshul\Documents\Projects\protonsearch`.
- Work is uncommitted. Preserve all existing changes; inspect `git status` before editing.
- Read `docs/PHASE_STATUS.md`, `docs/VERIFICATION_MATRIX.md`, `docs/OCR_ROOT_CAUSE.md`, `docs/PREVIEW_ROOT_CAUSE.md` and `docs/OCR_MODEL_RESEARCH.md`.
- The Phase 1 requirements are in `C:\Users\Pranshul\Downloads\00_START_HERE.md` and `C:\Users\Pranshul\Downloads\01_PHASE_1_OCR_INDEX_SEARCH.md`. Treat these as project requirements, subject to the user's direct instructions.
- The user requested Luna High implementation subagents, independent review and computer-use verification where available.

## Current state

Onboarding and Preferences describe the three engines, but disable them because they are not integrated. Windows OCR remains active. Settings and onboarding reject unavailable engine selections to avoid silently running the wrong engine.

Local ONNX and Tesseract adapters exist in `crates/ps-ocr/src/onnx.rs` and `tesseract.rs`. They are prototypes: indexing does not dispatch to them, and real recognition has not been verified. The ONNX adapter is behind the optional `onnx` feature; runtime packaging is unresolved.

`crates/ps-ocr/src/ppocr_pack.rs` contains a feature-gated, explicitly invoked PP-OCRv6 downloader with pinned checksums and bounded downloads. It is not connected to the app. Its two-step activation is not crash-safe, and it lacks app-level progress and cancellation. No external models were installed during this work.

OCR candidate starvation beyond the former 400-image cap was addressed. The current pass can take a long time for large libraries. Do not reinstate a simple cap: the worker has no guaranteed durable continuation, so it would strand later images and could report completion incorrectly.

Photos search scope, metadata backfill, preview changes and diagnostics were also developed. See the phase documents for limitations and evidence.

## Remaining work

1. Connect the selected OCR engine to actual indexing. Replace hard-coded availability with verified local runtime and model availability; avoid a Windows-only availability gate or silent fallback. Persist valid selections in settings and onboarding.
2. Finish the installation flow. Add explicit consent, progress, cancellation and recovery through the desktop bridge. Make PP pack activation crash-safe. Package and test the ONNX runtime. Present Indra's model license before download. Handle Tesseract executable and language data. Keep downloads opt-in.
3. Enable the three choices only when each is truly available. Make PP-OCRv6 the default for fresh installations only after successful integration; preserve existing Windows OCR configurations.
4. Test actual recognition for all three engines, including restart, unavailable assets, installation failure and recovery. Verify model provenance and licenses.
5. Finish Phase 1 acceptance: durable stage-level progress; mixed-document indexing corpus and benchmarks; scanned and embedded image OCR; large-image downsampling; Photos browsing without query text; migration and restart verification; and native end-to-end search and preview checks.
6. Build and test a new native release after integration. Verify onboarding, settings, tray and launcher behavior in the actual app. Do not claim the phase or release is complete without evidence.

## Verification so far

- The latest desktop test suite passed **265 tests**; the desktop production UI build passed.
- Core, IPC and OCR tests passed during integration. A synthetic Windows OCR extraction corpus recognized 100 generated images; it is not an end-to-end indexing corpus or an external-engine benchmark.
- The Preferences OCR selector was inspected in a browser preview using mock data. Updated native onboarding and settings were not verified end to end.
- The PP download component's targeted tests passed. Its ordinary error rollback was reviewed, but process interruption between activation renames remains a known risk.

## Coordination

An automatic continuation was scheduled in the original chat for approximately 3:34 PM India time on 11 November 2026. Disable or coordinate it if another agent takes over, to avoid overlapping edits.
