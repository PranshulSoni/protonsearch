# P0 AUDIT — Clipboard does not capture images (ProtonSearch v2 / v10 handoff)

| Field | Value |
|---|---|
| Gate / task | G0 audit, task-30 (owner: sources-content / L2-A3 under L1-A) |
| Severity | **P0** (user-reported: clipboard history shows no images) |
| Status | **REPRODUCED on real Windows**; root cause identified; no product code changed (Gate 0 is read-only) |
| Repair | **FIXED in task-39 / G2** — see @BOOT_P0_REPAIR_EVIDENCE.md |
| Date | 2026-11-10 (Asia/Calcutta) |
| Host | Windows 11, AMD Ryzen AI 9 (10C/20T), 31 GB RAM |
| Evidence | @docs/qa/clipboard-audit/evidence/** (logs, decoded PNGs, runtime DBs, harness source) |
| Harness | @docs/qa/clipboard-audit/harness (standalone crate; not part of the shipped app) |

## TL;DR

1. The P0 is real and reproducible end-to-end through the production code path (TEXT copy → history row; real .NET IMAGE copy while the production listener runs → **no row, no cache file, no warning**).
2. Primary root cause: the bitmap decoder only accepts CF_DIB with BI_RGB (compression 0), 24/32 bpp. Real applications (.NET/Paint-style/OLE sources) put **BI_BITFIELDS (compression 3)** DIBs on the clipboard — the decoder rejects them and the listener then silently suppresses the event.
3. Secondary root causes: no CF_DIBV5/CF_BITMAP/registered-PNG/PNG/OLE fallbacks; the listener prefers text and never reads an image when text is present (mixed copies lose the image); a completely empty snapshot suppresses the listener callback, so unsupported images produce no callback/DB row/warning at all; Files/HTML/RTF formats are not captured; no thumbnails/dimensions/paste-back/retention.
4. Text capture works (control evidence below), which matches the user report: text history works, images never appear.

## Reproduction

### Environment / method
- GUI applications were **not** launched (task environment constraint: do not run GUI apps). Instead the audit drives the exact production modules the app runs:
  - @crates/ps-windows/src/imp/clipboard.rs — Win32 clipboard + listener
  - @crates/ps-sources/src/clipboard.rs — listener → DB/image-cache persistence (the same call `ps_sources::clipboard::start_listener(db, image_dir)` that @crates/ps-core/src/engine.rs:1077 registers)
  - a temp v2 database and image dir equivalent to the engine's (engine uses `data_dir/index.db` and `data_dir/clipboard-images`, engine.rs:1058/1076)
- Real external copies were produced from separate processes (Windows PowerShell 5.1 STA + System.Windows.Forms.Clipboard), so the listener observed genuine cross-process WM_CLIPBOARDUPDATE events.
- Harness sources (audit-only, outside crates/**): `harness/src/bin/format_audit.rs` (format negotiation matrix + raw headers), `harness/src/bin/db_audit.rs` (listener/DB/image-cache matrix, restart inspection), `harness/src/lib.rs` (synthetic Win32 clipboard payloads).

### Steps (commands reproduced in evidence logs)
1. Build harness once: `cd docs/qa/clipboard-audit/harness && cargo build --target-dir ../../../target/clipboard-audit`
2. Format matrix: `target/clipboard-audit/debug/format_audit.exe --evidence docs/qa/clipboard-audit/evidence/decoded` → @evidence/logs/format_matrix.log
3. End-to-end listener + DB: `target/clipboard-audit/debug/db_audit.exe --matrix --root docs/qa/clipboard-audit/evidence/runtime/matrix` → @evidence/logs/db_matrix.log
4. Restart persistence: `db_audit.exe --inspect --root .../matrix` (new process) → @evidence/logs/db_restart_inspect.log
5. Cross-process control: production listener running in a background process; external TEXT copy, then external real IMAGE copy (`powershell.exe -Sta -File evidence/scripts/set_clipboard_image.ps1`) → @evidence/logs/db_external_control.log
6. Direct seed path: set real .NET image, then `db_audit.exe --capture-external` → @evidence/logs/capture_dotnet_image.log

### Key observed results

**Cross-process control (production listener active):**
`BT__
READY: listener running for 16s; set the clipboard image now
observed count change: 0 -> 1                     <- external TEXT copy produced a row
listener: stopped
count: 1
row: id=1 category=Text pinned=false text="external clipboard text control 2026"
image_cache_files: 0                              <- external real IMAGE copy produced NOTHING
`BT__
The same listener process received the text update but never produced anything for the subsequent real image copy. This is the P0 exactly as the user experiences it.

**Direct seed path (`capture_current`):** real .NET image → `capture_current: None`, and the database file was not even created (no write attempted) — @evidence/logs/capture_dotnet_image.log.

**Why:** the real image clipboard from .NET exposes (raw dump, @evidence/logs/dotnet_setimage_observed.log):
`BT__
formats: DataObject, System.Drawing.Bitmap, CF_BITMAP, Ole Private Data, CF_DIB, CF_DIBV5
raw CF_DIB: header_size=40 320x200 bpp=32 compression=3 size_image=256000
raw CF_DIBV5: header_size=124 320x200 bpp=32 compression=3 masks=00ff0000/0000ff00/000000ff/00000000
get_image: None, get_image_png: None
`BT__
compression=3 is BI_BITFIELDS. The decoder gate at @crates/ps-windows/src/imp/clipboard.rs:146-151 explicitly rejects it:
`BT__
// Only uncompressed RGB/RGBA bitmaps are supported (screenshots, Snipping Tool).
if width <= 0 || height_field == 0 || compression != 0 { return None; }
if !matches!(bit_count, 24 | 32) { return None; }
`BT__

## Format negotiation table (synthetic input via raw Win32; real .NET input measured separately)

| # | Clipboard formats set by the source | Raw CF_DIB seen | `get_image` / `get_image_png` | Listener snapshot | DB row produced |
|---|---|---|---|---|---|
| 1 | CF_DIB 32bpp BI_RGB alpha=255 (bottom-up) | BI_RGB 32 | OK 64x48 | image OK | image row + PNG (expected) |
| 2 | CF_DIB 32bpp BI_RGB alpha=0 (GDI) | BI_RGB 32 | OK (alpha forced opaque) | image OK | image row (deduped if pixels repeat) |
| 3 | CF_DIB 24bpp BI_RGB | BI_RGB 24 | OK | image OK | deduped with #1 (same pixels) |
| 4 | CF_DIB 32bpp top-down (negative height) | BI_RGB 32 | OK | image OK | expected dedupe/row |
| 5 | CF_DIB 24bpp odd width 7 | BI_RGB 24 | OK 7x5 | image OK | expected |
| 6 | 8bpp indexed DIB | not CF_DIB in test (faked registered fmt) | **None** | **no callback** | **none** |
| 7 | CF_DIBV5 BI_BITFIELDS (modern apps) | synthesized CF_DIB compression=3 | **None** | **no callback** | **none** |
| 8 | CF_DIBV5 BI_RGB | synthesized CF_DIB BI_RGB | OK | image OK | expected |
| 9 | CF_BITMAP only | synthesized CF_DIB compression=3 | **None** | **no callback** | **none** |
| 10 | registered "PNG" only (browsers/some editors) | — (no CF_DIB) | **None** | **no callback** | **none** |
| 11 | registered "image/png" only | — | **None** | **no callback** | **none** |
| 12 | CF_DIB + CF_UNICODETEXT (mixed copy) | BI_RGB 32 | image OK, text OK | **text only, png=None** | **text row only — image lost** |
| 13 | CF_HDROP file list + text (Explorer-style) | — | None | text only | category Path from path text (Files not represented) |
| 14 | plain text | — | None | text OK | Text row (control) |
| 15 | empty clipboard | — | None | no callback | none |
| R | **real .NET Clipboard.SetImage** (OS-level real copy) | **compression=3 BI_BITFIELDS 320x200** | **None** | **no callback** | **none (P0)** |

Raw CF_BITMAP evidence (@evidence/logs/raw_bitmap_only.log): Windows synthesizes a CF_DIB for CF_BITMAP, but it is **compression=3 / 32bpp**, so the decoder rejects it too:
`BT__
raw CF_DIB (8): header_size=40 48x32 bpp=32 compression=3 size_image=6144
raw CF_DIBV5 (17): header_size=124 48x32 bpp=32 compression=3 masks=00ff0000/0000ff00/000000ff/00000000
get_image: None, get_image_png: None
`BT__

## End-to-end database / image-cache evidence
Produced by @evidence/logs/db_matrix.log (production listener + production persistence). Worked image example (real decoded PNG, 64x48):
![working CF_DIB capture](evidence/decoded/listener_dib32_bottomup_alpha255.png)

| Scenario | DB delta | Result |
|---|---|---|
| listener: CF_DIB 32/24 bit BI_RGB | +1 | `id=1 category=Image text="[image:91a5a1b1236a58a9]"` (64x48 PNG cached) |
| listener: CF_DIBV5 BI_BITFIELDS | 0 | **no row** (P0 path) |
| listener: CF_BITMAP only | 0 | **no row** |
| listener: registered PNG only | 0 | **no row** |
| listener: CF_DIB + text mixed | +1 | `id=3 category=Text "mixed image+text copy"` — **image lost** |
| listener: CF_HDROP + text | +1 | `id=4 category=Path "C:\Windows\win.ini"` — Files format not represented |
| listener: plain text | +1 | control row (Text) |
| capture_current: real .NET image | — | `None`, DB file not created |
| duplicate same DIB (copied twice) | +1 | one row (content-hash dedupe works) |
| rapid repeats (5 copies of same bitmap) | +1 | one row, one cached PNG |

Persistence and schema (@evidence/logs/db_schema_rows.log, @evidence/logs/db_restart_inspect.log):
- DB file survives process restart; 6 rows + 3 PNG files identical after reopening in a new process.
- Cached PNGs are valid and decode correctly: `png_ok: 91a5a1b1236a58a9.png 64x48 438 bytes`, `143da3305207e633.png 40x30 327`, `e7d256b1f92c369a.png 30x20 263`.
- Current schema is minimal: `clipboard_history(id, content, timestamp, source_app, is_image, pinned, ocr_text, image_path, category)` — **no width/height/thumbnail/format/content-hash columns**; image row text is a placeholder `[image:<hash>]`.
- Auto-increment ids jump (6 rows up to id=12) because `INSERT OR IGNORE` burns ids on de-duplicated copies; repeated copies of the same image do **not** refresh its timestamp (INSERT OR IGNORE path in @crates/ps-sources/src/clipboard.rs:128-190), so a re-copied image does not bubble to the top of history.

## Root-cause analysis (code references)

| ID | Root cause | Evidence | Code |
|---|---|---|---|
| R1 | **Bitmap decoder rejects BI_BITFIELDS.** Real .NET/Paint-style/WPF/Office DIBs use `biCompression=3`, which is rejected; only BI_RGB 24/32 bpp accepted. | real .NET dump + raw bitmap_only dump | @crates/ps-windows/src/imp/clipboard.rs:131-197 (gate :146-151) |
| R2 | **No CF_DIBV5 decoder.** CF_DIBV5 is never read directly; it only works when Windows synthesizes a BI_RGB CF_DIB (case #8), which is not guaranteed (case #7/#9 produce BITFIELDS). | table #7/#9 | get_image_inner reads only format 8: @crates/ps-windows/src/imp/clipboard.rs:113-129 |
| R3 | **No CF_BITMAP fallback.** CF_BITMAP-only copies expose a synthesized BITFIELDS DIB; a GetDIBits(BI_RGB) conversion would work but is not implemented. | raw bitmap_only dump | same file, no format 2 path |
| R4 | **No registered-PNG/PNG fallback.** Browsers/editors may expose only registered "PNG"/"image/png"; nothing reads it. | table #10/#11 | same file |
| R5 | **Mixed copies lose the image silently.** snapshot() only reads an image when `text.is_none()`; a CF_DIB+text copy records text only. | table #12; db_matrix id=3 | @crates/ps-windows/src/imp/clipboard.rs:216-229 |
| R6 | **Listener suppresses empty snapshots.** WM_CLIPBOARDUPDATE handler only calls the callback if text or png is Some, so unsupported images produce no callback, no DB row, no warning — the user sees an empty history rather than an error. | table #7/#9/#10/#11 show no listener callback | @crates/ps-windows/src/imp/clipboard.rs:360-372 |
| R7 | **Files/HTML/RTF not captured.** CF_HDROP is only seen via its synthesized text and classified Path; Files category cannot be represented. HTML/RTF are ignored entirely. | table #13 | snapshot(): @crates/ps-windows/src/imp/clipboard.rs:216-229 |
| R8 | **No metadata/thumbnail/paste-back.** No width/height/format/thumbnail columns; no API to put an image back on the clipboard; no image preview payload. | schema dump; grep finds no set_clipboard_image | @crates/ps-sources/src/clipboard.rs:32-45 (schema), @crates/ps-sources/src/lib.rs:45 (categories lack Files) |
| R9 | **Unbounded decode + blocking listener.** decode_dib allocates width*height*4 after only a payload-size check (no pixel cap), and the listener performs PNG encode + hash + file write + SQLite write inside the WM_CLIPBOARDUPDATE handler, so a large/slow copy blocks the listener message loop and rapid copies can be lost. Clipboard-open retries only ~120 ms total. | code review | ps-windows:131-230, :360-372; ps-sources:54-71 |
| R10 | **Two image cache locations.** Engine listener uses `<data_dir>/clipboard-images` (engine.rs:1076); `Sources::load` seeding uses `<data>/cache/clipboard` (@crates/ps-sources/src/lib.rs:242). Both may be created depending on code path. | code review | engine.rs:1076, ps-sources lib.rs:242 |
| R11 | **Clipboard history is off by default.** `PrivacySettings::default().clipboard_history == false` (@crates/ps-ipc/src/lib.rs:519); with defaults no listener starts and history is empty until the user opts in. This can mask/compound the P0 for fresh installs. | code + settings dump | ps-ipc lib.rs:519; engine.rs:1065-1090 |
| R12 | **OLE-only clipboard sources are invisible to plain Win32 reads.** Some OLE applications publish data only through the OLE data object; plain EnumClipboardFormats/GetClipboardData can return no useful formats. Not the primary cause here (real .NET image also exposed plain CF_DIB), but Gate 2 should read via `OleGetClipboard`/`IDataObject` as a fallback. | .NET DataObject file test showed only DataObject/Ole Private Data through plain Win32 | n/a (needs new OLE path) |

### User-visible failure chain
`BT__
copy image in Paint/Snipping Tool/browser
  -> WM_CLIPBOARDUPDATE reaches listener
  -> snapshot(): text None; get_image_png() -> GetClipboardData(CF_DIB)
  -> DIB has BI_BITFIELDS(compression=3)  [or only CF_BITMAP/CF_DIBV5/PNG]
  -> decode_dib() returns None
  -> snapshot { text: None, png: None }
  -> listener suppresses callback (no event reaches ps-sources/DB)
  -> clipboard_history gets no row / no PNG / no thumbnail
  -> launcher Clipboard card looks empty for images (text still works)
`BT__

## Gate 2 repair plan (no fixes implemented yet)

### P0 work items (blocking G2)
| # | Work item | Files (proposed owner) | Acceptance |
|---|---|---|---|
| A1 | Replace the single-format reader with a negotiation pipeline: registered PNG/`image/png` → CF_DIBV5 → CF_DIB (BI_RGB + BI_BITFIELDS, 32/24/8bpp palette, top-down/bottom-up, checked stride/size) → CF_BITMAP via GetDIBits(BI_RGB) → OLE IDataObject fallback. Bounded decode (max ~40 MP, max edge 8192, reject zero/overflow). | crates/ps-windows/src/imp/clipboard.rs (L2-A3) | Format matrix rows #1–#12 and real .NET SetImage produce a decoded image; unit fixtures for each format incl. BITFIELDS DIB bytes |
| A2 | Extend ClipboardSnapshot with negotiated representations: text, image (PNG+RGBA dims), files (CF_HDROP list), html, rtf, and the set of formats seen; pick best-image but **never** drop image when text exists; dedupe multiple formats into ONE history item. | ps-windows + ps-sources | Mixed CF_DIB+text copy produces one row with image persisted; Files copy classified as Files not Path |
| A3 | Never suppress listener callbacks; move decode/PNG/DB/file work off the WM_CLIPBOARDUPDATE thread into a bounded worker queue (drop-oldest policy, no UI/index blocking); keep retry when clipboard is locked; expose a warning on decode failure instead of silence. | ps-windows listener + ps-sources listener | Unsupported/corrupt clipboard write yields a diagnostic (warning/error row), not silence; rapid copies do not deadlock; app shutdown joins cleanly |
| A4 | DB migration (v2 clipboard schema): add width, height, thumbnail_path, format, source_hash, and Files category; store PNG atomically (temp+rename) with thumbnail (<=256px); dedupe by decoded-pixel hash while refreshing timestamp on re-copy; add quota/retention + clear-on-exit. | ps-sources clipboard + ps-ipc ClipboardEntry (contract change, L0 approval) | Re-copied image refreshes ordering; restart shows image + thumbnail; quota enforced; migration idempotent |
| A5 | Re-copy/paste-back: `ps_windows::set_clipboard_image(png)` (CF_DIB + CF_DIBV5 + registered PNG, alpha-aware), `ps_sources::clipboard` copy/pin/delete workflows; unsupported target → clear structured error (never fake success). | ps-windows + ps-sources (+ UI) | Paste into Paint/Snipping Tool-style target reproduces pixels (width/height round-trip); failure path returns error |
| A6 | Categories All/Text/Images/Links/Code/Files/Pinned wired to ClipCategory + Files addition; image preview thumbnails in both launcher modes; pin/retention for images. | ps-ipc + ps-sources + UI teams | Category tabs show correct counts; pinned image survives retention/restart |
| A7 | Privacy/performance hardening: keep default-off but make onboarding/Preferences explicit; pause, exclude sensitive apps (best effort), ignore identical updates, retention policy, secure wipe, no clipboard content in logs. | ps-sources + UI | Settings toggle starts/stops listener; no image content in logs; retention clears cached PNGs |

### Test plan (v10 §8 mandatory tests)
| v10 test | Gate 0 status | G2 acceptance |
|---|---|---|
| 1 Notepad text + recopy | text control passes (`external clipboard text control 2026` row) | text recopy from history |
| 2 VS Code code snippet + Code category | classifier exists; not re-verified here | Code category + multiline exact content |
| 3 Paint bitmap (NOT file path), thumbnail, restart, paste into Paint | **FAILS today** (real .NET image = same format family; no row) | image row, thumbnail, restart, pixel-identical paste |
| 4 Win+Shift+S Snipping Tool | **UNVERIFIED** (cannot automate the interactive overlay; format family approximated) | real snip → row/paste verification (manual E2E) |
| 5 Browser "Copy image" | **UNVERIFIED** (no browser automation run); registered-PNG/browser format sets covered synthetically and fail today | real Chrome/Edge copy-image E2E |
| 6 URL → Links category | classifier unit-tested (task-20) | UI shows Links |
| 7 Explorer file(s) → Files not image | today Path row only (no Files category) | Files category + no image misinterpretation |
| 8 HTML-rich copy classification/dedup | not captured today | HTML/RTF stored and classified; one row per copy |
| 9 large/high-DPI transparent, rapid copies, locked clipboard, restart during capture, retention, delete | dedupe/rapid/restart basics verified; bounds/locked/retention not implemented | bounded decode + retry + quota/retention tests |
| 10 no duplicates per one copy event | rapid-repeat dedupe works for supported DIBs; unsupported images produce zero rows (worse) | exactly one row per copy event across formats |

### Suggested sequencing for G2
1. A1+A2 (decoder negotiation + snapshot semantics) with format-fixture unit tests — unblocks visible image capture immediately.
2. A3 (listener worker + no silent drop) — prevents user-visible black holes and protects UI latency.
3. A4 (schema/thumbnail/quota) + A6 categories — unblocks V9 clipboard card (Images/thumbnails) and full history view.
4. A5 paste/recopy, then A7 privacy hardening.

### Risks / rollback
- Keep the existing BI_RGB path as a fallback branch during the rewrite; format fixtures pin behaviour.
- DB migration is additive columns only (no destructive change); keep a copy of user clipboard DB before migration in the app's migration flow.
- Listener worker queue must be bounded; on overwhelm drop oldest events and log a warning (never block the Win32 message loop).

## Evidence index
- Format + listener matrix: @evidence/logs/format_matrix.log
- Raw header dumps: @evidence/logs/raw_bitmap_only.log, @evidence/logs/raw_dibv5_bitfields.log, @evidence/logs/dotnet_setimage_observed.log
- End-to-end listener/DB matrix: @evidence/logs/db_matrix.log
- Restart persistence: @evidence/logs/db_restart_inspect.log, @evidence/logs/db_schema_rows.log
- Cross-process control (text row vs image none): @evidence/logs/db_external_control.log, @evidence/logs/db_external_listener.log
- Direct seed-path failure: @evidence/logs/capture_dotnet_image.log
- Real .NET image set script: @evidence/scripts/set_clipboard_image.ps1; OLE reader checks: @evidence/logs/ole_image_read.log
- Cached PNG validity: @evidence/logs/image_cache_png_validity.log; decoded evidence images: @evidence/decoded/*.png
- Runtime DBs + cached PNGs: @evidence/runtime/matrix/…, @evidence/runtime/external-control/…

## Limitations / unverified
- The GUI app was not launched (environment rule). Reproduction uses the exact production modules and call path (engine listener registration/DB/image dir); it is not a UI-level E2E. A real-app E2E with Paint/Snipping Tool/browser remains required for G2 acceptance.
- Real Paint / Snipping Tool / browser interactive copies were not automated (no GUI automation in this task); real clipboard format families from .NET (System.Windows.Forms.Clipboard.SetImage) and synthetic Win32 sets were used. Snipping Tool and browser copy-image are marked UNVERIFIED.
- The user's live v2 database at %APPDATA%\protonsearch\v2\index.db does not exist on this machine, and the E2E data dirs (.devdata*, .devdata-mig2) contain no clipboard_history table, so no real captured-image DB rows from the shipped app could be inspected; evidence comes from the production code path driven by the audit harness.