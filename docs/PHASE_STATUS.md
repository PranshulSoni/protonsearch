# Phase status

Phase 1 is in progress; no phase gate has passed. Baseline commit `4c1dc2a4360dce59d91b18f3ea74d690ab963ceb`.

OCR scheduling, previews and Photos scope are under investigation or implementation. Model-pack management, extraction coverage, durable progress, benchmark corpus, migration and full Windows acceptance remain outstanding. Do not start Phase 2 until Phase 1 acceptance is supported by evidence.

Implemented changes under review: removal of OCR candidate starvation, a four-worker timeout cap, Photos search scope, bounded image decoding and nonblocking/fresh previews. A synthetic Windows OCR extraction test processed 100 images successfully. Desktop tests passed 260/260 before the final review fixes. These are not a Phase 1 pass.

Open blockers include downloaded local models and consent, durable stage-level counts, full mixed-document ingestion and provenance, scanned/embedded image OCR, real index corpus and benchmark, large-image downsampling, Photos browsing without query text, and native end-to-end preview verification. OCR roots on the current profile require explicit configuration; no settings were changed.

Onboarding and Preferences now describe PP-OCRv6, Indra and Tesseract, but they cannot yet be selected: runtime integration, installation and real recognition tests are incomplete. Windows OCR remains active. An opt-in PP-OCRv6 downloader exists behind a build feature with no app integration; its two-step activation is not crash-safe. A full OCR pass can take a long time on large libraries; adding a cap without a durable continuation would strand work.

Next: connect the selected engine to `index_ocr` without the Windows-only availability check or silent fallback; discover and verify local models and runtimes; expose consent, installation, progress and cancellation through the desktop bridge; package and test ONNX Runtime; handle Indra license acceptance and Tesseract language data; then enable available engines in both interfaces. Test actual recognition, restart and failure recovery before changing the fresh-install default to PP-OCRv6. The PP pack activation needs crash recovery before app integration.
