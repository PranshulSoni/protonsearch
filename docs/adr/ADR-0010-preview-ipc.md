# ADR-0010: Preview IPC contract (resultId-only, bounded, cached)

- Status: **Accepted** (G1 post-review L0 decision, 2026-11-10)
- Owners: L0 (decision), L1-A (engine/command), L1-B/L2-B (UI), L1-E (security review)
- References: docs/qa/preview-audit/PREVIEW_P0_REPORT.md (G0-A reproduction); ADR-0006

## Context

File/text previews were a user-reported P0 because no preview pipeline existed: there was no preview
request/response in ps-ipc, no preview command in the Tauri shell and no preview code in ps-core. The
launcher could only show a snippet a content-FTS hit happened to carry, so filename results and OCR
results rendered "Preview not available". The G1 review defined the contract; G2 task-37 implements
the backend while task-38 wires the UI.

## Decision

1. Input is resultId-only: PreviewRequest { resultId, maxChars? } (ps-ipc:872-878). The UI must never
   send a filesystem path; the engine resolves resultId against its own recent search/result state
   and returns an honest Unsupported payload for unknown/expired ids (engine.rs:672-711). This keeps
   the path/authorization decision inside Rust.
2. Output is a bounded typed payload: PreviewPayload { resultId, title, subtitle, format, text?,
   snippet?, highlightRanges, imageDataUrl?, ocrText?, fields, status (ok/unsupported/error),
   truncated, error?, imageMime?, unsupportedReason? } (ps-ipc:880-934).
3. maxChars is optional and clamped by the engine to a min/default/max budget; text and snippet are
   capped and marked truncated. Highlight ranges are validated against the returned text length by
   the producer; image data URLs are bounded by the same clipboard/image limits (ADR-0009) and use an
   allowlisted MIME. OCR text is included only when already indexed (never a synchronous OCR call on
   the preview path).
4. The engine performs preview work synchronously but bounded (extraction caps, LRU cache keyed by
   resultId+maxChars), and the command must run it on a blocking worker, never on the search hot path;
   results are cached and reused (engine.rs:667-719, preview cache tests engine.rs:3443-3472).
5. Failure is explicit: unsupported formats/binary/encrypted files return status=unsupported with a
   reason, errors return status=error with a short safe message, and no silent blank panel is allowed.
6. The Tauri preview command and bridge method are added by task-37/task-38 (approved direction:
   invoke preview(PreviewRequest) -> PreviewPayload). Until the command lands, the UI keeps the
   current honest fallback; G2 acceptance re-checks the command, bounds and both launcher paths.
7. Both launcher presentations consume the same payload; parity is a G8 obligation, not a second
   backend.

## Consequences

- No arbitrary file read via preview IPC; the engine is the authorization point.
- A selected-result preview can be cached safely; stale result ids return honest unsupported states.
- Preview content must be sanitized for rendering; no HTML/script execution, no external resources.
- Real-format evidence (TXT/code/MD/HTML/JSON/CSV/PDF/DOCX/images/OCR/Unicode/large/binary/encrypted)
  and screenshots are required at G2; unit tests alone are insufficient.

## Alternatives considered

- Path-based preview request (rejected: turns IPC into arbitrary file read, bypasses index/exclusions).
- Streaming preview chunks (deferred: not needed for v10, adds lifecycle complexity).
- Rendering previews in Rust to images (deferred: excessive scope; text/metadata preview is enough).

## Verification

- G0-A: docs/qa/preview-audit/PREVIEW_P0_REPORT.md (pipeline absent, format matrix, payload evidence).
- Engine: engine.rs:667-719 (bounded cached preview, honest unknown-id payload), tests
  engine.rs:3337-3559; ps-ipc:872-934.
- G2 task-37/38: command + bridge acceptance, fixture excerpts/hashes, both-launcher screenshots,
  bounds/stale/cancel tests, security review of rendered HTML/PDF paths.
