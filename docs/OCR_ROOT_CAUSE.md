# OCR investigation — Phase 1 baseline

Baseline commit: `4c1dc2a4360dce59d91b18f3ea74d690ab963ceb`.

The displayed OCR document count comes from `COUNT(*)` on `ocr_documents`. Rows are written only for nonempty recognized text. The number is not the number of images discovered or attempted. Historical evidence does not establish why the reported value was three.

The current configuration has OCR enabled globally, but the four configured roots have two content-only and two names-only modes; none currently requests content plus OCR. Root paths are intentionally omitted here.

The baseline implementation selects the first 400 eligible images before checking whether they already have OCR data. A single background pass cannot reach later images, including when earlier images were already indexed or produced no text. The candidate cap was removed, with a separate hard limit on active OCR workers.

The current OCR engine uses Windows.Media.Ocr. It does not implement a downloadable model manager. Image dimensions and decoder memory are now limited before full decode; images over 16 MP are rejected rather than downsampled. A timed-out worker cannot be forcibly cancelled, but retains one of four global slots until it exits. Four permanently hung workers prevent new OCR work. Further work is required before Phase 1 acceptance.
