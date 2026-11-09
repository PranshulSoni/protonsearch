# ADR-0009: Clipboard image storage and IPC contract

- Status: **Accepted** (G1 post-review L0 decision, 2026-11-10)
- Owners: L0 (decision), L2-A (sources implementation), L1-A (engine/IPC), L1-E (privacy review)
- References: docs/qa/clipboard-audit/CLIPBOARD_IMAGE_P0_REPORT.md (G0-D reproduction)

## Context

Clipboard image capture is a user-reported P0: text history works, images never appear. G0-D proved
the production listener decodes only CF_DIB with BI_RGB compression 0 / 24-32 bpp; real .NET/Paint/
OLE sources put BI_BITFIELDS (compression 3) on the clipboard, decoding returns None, and the
listener silently suppresses the event. There is also no fallback for CF_DIBV5/CF_BITMAP/registered PNG,
mixed text+image snapshots lose the image, and thumbnails/dimensions/paste-back/retention are absent.
Separately, the G1 review found two competing clipboard gates and no documented IPC size bounds.

## Decision

1. Capture formats and precedence: accept CF_DIB (all header/compression variants that can be decoded,
   including BI_BITFIELDS and top-down DIBs), CF_DIBV5, CF_BITMAP, registered PNG/OLE bitmap/PNG
   formats where feasible, with one logical copy event deduplicated into one history item. A mixed
   text+image copy stores both representations of the same item where the source provides both.
   Unsupported formats must be logged/warned, never silently dropped.
2. Single master gate: history is active only when privacy.clipboard_history AND !clipboard.pause.
   capture_text and capture_images are format toggles inside that gate (ps-ipc:809-853).
3. Storage: images are stored as PNG in the engine clipboard image directory together with a
   content hash, width, height, MIME, format and thumbnail path; a thumbnail data URL is produced for
   list results; DB migration is additive (columns added when missing). Retention (retention_days,
   max_items), exclude_apps and clear_on_exit are enforceable settings; secure wipe/clear semantics
   are part of this contract.
4. Bounds (enforced in the engine, documented on ClipboardSettings): thumbnail data URL <= 128 KiB,
   source dimensions <= 8192 px per side, MIME allowlist image/png and image/jpeg (plus whatever the
   thumbnail encoder emits under those rules), no multi-MB payloads in search results. Dimensions and
   MIME are validated before persistence and again before IPC. This enforcement lands with task-39
   and must be proven by tests; the ps-ipc doc comment already states the intent (ps-ipc:809-813).
5. IPC: SearchResult.meta carries thumbnailDataUrl, imageWidth, imageHeight and mime for clipboard
   image rows only (ps-ipc:252-281; engine populates at engine.rs:2298-2347). The UI may render the
   data URL directly (CSP img-src includes data:); it must not receive image paths or arbitrary
   filesystem URLs.
6. Paste-back: a copy action returns the original representation where possible (PNG/DIB) to the
   clipboard; if a source app only accepts bitmap, convert; report a clear error otherwise.
7. Privacy: capture remains opt-in (privacy.clipboard_history default false), exclusions are applied
   before storage, and no clipboard content is sent to an agent/provider without an explicit user
   action.

## Consequences

- The P0 root cause is addressed by format support, not by UI work; UI thumbnails are a consequence.
- Search result payloads stay bounded; a clipboard history query returning 50 image rows must not
  inflate the WebView by multiple MB (thumbnail cap).
- History growth is bounded by max_items/retention; secure wipe is a privacy requirement, not a
  nice-to-have.
- Format coverage must be verified with real applications (Paint, Snipping Tool, browser copy image,
  Office/.NET apps) at G2; synthetic-only evidence is insufficient.

## Alternatives considered

- Store full-size PNG data URLs in results (rejected: IPC/memory blowup).
- Leave decode per-format to the UI (rejected: security/privacy boundary and WebView memory).
- Add a second independent privacy toggle (rejected: ambiguous precedence; one master gate).

## Verification

- G0 evidence: docs/qa/clipboard-audit/CLIPBOARD_IMAGE_P0_REPORT.md (BI_BITFIELDS root cause,
  external cross-process reproduction, decoded PNGs, DB matrix).
- Code: crates/ps-windows/src/imp/clipboard.rs (format negotiation/decode), crates/ps-sources/src/
  clipboard.rs (DB/cache/categories/thumbnails), engine.rs:2298-2347 (IPC population), ps-ipc:252-281.
- G2 task-39 acceptance: real Windows copy/paste matrix, restart/re-paste, bounds tests, privacy
  tests, BUG_LEDGER before/after evidence.
