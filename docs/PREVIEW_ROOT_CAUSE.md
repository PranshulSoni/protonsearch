# Preview investigation — Phase 1 baseline

Baseline commit: `4c1dc2a4360dce59d91b18f3ea74d690ab963ceb`.

Older captures record blank previews before the current preview IPC existed. Current code has a preview pipeline, but the Tauri command calls extraction synchronously. Scanned PDF handling returns unsupported before checking indexed OCR. Filename highlight offsets can be applied to unrelated extracted body text. The preview cache can also return stale data after a source changes.

Changes in progress move preview extraction to a blocking worker, retrieve indexed OCR for scanned PDFs, avoid applying filename offsets to body text, and refresh cached previews on close/reopen. Real Windows before/after interaction and format-specific regression evidence remain required.
