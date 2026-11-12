# BOOT-P0 REPAIR EVIDENCE — Clipboard image capture (task-39 / G2)

| Field | Value |
|---|---|
| Task | task-39 (G2 clipboard images), owner sources-content / L2-A3 |
| Status | **BOOT-P0 PROVEN END-TO-END on real Windows** (before/after evidence below) |
| Date | 2026-11-10 (Asia/Calcutta) |
| Host | Windows 11, AMD Ryzen AI 9, 31 GB RAM |
| Prior audit | @CLIPBOARD_IMAGE_P0_REPORT.md (task-30 Gate 0) |
| Evidence dir | @evidence/logs/repair_*.log, @evidence/runtime/repair/**, @evidence/repair-decoded/** |
| Harness | @harness (standalone audit crate, docs-only, not shipped) |

## TL;DR

- The Gate 0 P0 path (real .NET/Paint-style CF_DIB/CF_DIBV5 **BI_BITFIELDS** images) is now captured, persisted and restored. Before: zero rows. After: one Image row with width/height/mime/format/thumbnail, a real PNG payload, restart persistence, and a working paste-back.
- All previously unsupported formats now decode through the negotiated pipeline: BI_BITFIELDS DIB and DIBV5, CF_BITMAP (via GetDIBits), registered PNG and image/png, 24/32/16 bpp and palette DIBs, top-down/bottom-up, alpha/no-alpha.
- Mixed text+image copies now produce **one** history item with the image kept (text retained as alt_text); listener never silently drops image events any more.
- Capture/decode/persist runs on a bounded worker thread (the WM_CLIPBOARDUPDATE handler only signals it), so the message loop no longer performs PNG encoding, file IO or SQLite writes.
- Contract compliance: thumbnails 128 px max / <=128 KiB payload, imageWidth/imageHeight <=8192, mime image/png (allowlisted), ClipboardEntry exposes image_path/width/height/thumbnail_path/mime/format (+ocr_text reserved).

## Format matrix: before -> after (production ps-windows API + listener)

| Source formats | Gate 0 (before) | After repair |
|---|---|---|
| CF_DIB BI_RGB 32bpp (alpha 0/255), 24bpp, top-down, odd width | OK | OK (24bpp colour-channel bug also fixed — unit-tested pixels) |
| CF_DIB / CF_DIBV5 **BI_BITFIELDS** (real .NET/Paint/Office style) | **None / no event** | **decoded** (checksum matches BI_RGB source) |
| CF_DIBV5 BI_RGB | OK only via synthesized BI_RGB CF_DIB | direct decoder, OK |
| CF_BITMAP only | **None / no event** | **decoded via GetDIBits** |
| registered PNG / image/png only | **None / no event** | **decoded** |
| 8bpp palette DIB | **None** | **decoded** (unit-tested palette indices/colours) |
| mixed CF_DIB + CF_UNICODETEXT | text only, **image lost** | text AND image in snapshot; **one** DB item, image kept |
| CF_HDROP file list | Path text row only | **category Files** (`hdrop`, text/x-file-list) |
| plain text / empty clipboard | OK / no-op | OK / no-op |

Evidence: @evidence/logs/repair_format_matrix.log (per-format get_image/get_image_png/listener_snapshot). The repaired 24bpp checksum now equals the 32bpp source checksum (same gradient fixture).

## Real Windows end-to-end reproduction (the exact Gate 0 failure)

Method: production listener running in a separate process (`db_audit --listen-external`), then a real `System.Windows.Forms.Clipboard.SetImage` copy from a separate STA PowerShell process (the same format family that failed in Gate 0: CF_DIB + CF_DIBV5 compression=3 BI_BITFIELDS, 320x200).

Before (Gate 0): `observed count change: 0 -> 1` for a text copy, then **no row at all** for the image copy.
After (repair) — @evidence/logs/repair_external_dotnet_listener.log:
`BT__
READY: listener running for 12s; set the clipboard image now
observed count change: 1 -> 2
row: id=2 category=Image pinned=false source="explorer" text="[image:5c4c4af7…]"
image_cache_files: 4   (full PNG + thumbnail for each of the two rows)
`BT__
Persisted row metadata (@evidence/logs/repair_db_schema.log):
`BT__
id=1 is_image=1 category=image
image_path=<data>/clipboard-images/5c4c4af7….png
image_width=320 image_height=200
thumbnail_path=<data>/clipboard-images/thumb-5c4c4af7….png
mime=image/png format=dibv5 image_bytes=6315
`BT__
Thumbnail contract verified: thumbnail 128x80 (<=128 px) and 3,671 bytes (<=128 KiB); full PNG 320x200, 6,315 bytes.
Restart persistence: a new process re-opened the same DB and listed both rows + files (@evidence/logs/repair_external_restart.log).
Seed path: `capture_current` on the same real clipboard returned `Some(id=1, category=Image)` and wrote the row (@evidence/logs/repair_capture_dotnet_image.log).
Paste-back: `paste_latest: row id=2 width=Some(320) height=Some(200) format=Some("dibv5") -> clipboard now carries 320x200 PNG` (@evidence/logs/repair_external_paste.log). The restore writes CF_DIB + CF_DIBV5 + registered PNG + CF_BITMAP, so Paint/Office/Snipping-style targets can paste the bitmap.

## Mixed formats and file lists
- Mixed copy: format-matrix listener snapshot is now `text=Some(...) png=Some(...)` (@evidence/logs/repair_format_matrix.log, dib_plus_text / listener_dib_plus_text) — the image is no longer dropped when text exists.
- Persistence for a mixed snapshot is one image item (integration test `mixed_image_and_text_snapshot_is_one_image_item`), with the text alternative kept in DB alt_text.
- CF_HDROP copies persist as category Files / format hdrop / mime text/x-file-list and remain searchable by file name (`files_snapshot_uses_files_category` + repair DB matrix row id=1).

## G1 contract compliance
- Master gate (engine-owned): privacy.clipboard_history AND !clipboard.pause; listener is only started when open. Format toggles: `start_listener_with_options(db, image_dir, ClipboardCaptureOptions{capture_text, capture_images})` filter snapshots per copy; `filter_snapshot` / `capture_current_with_options` are public. Documented at the top of crates/ps-sources/src/clipboard.rs.
- Thumbnails: <=128 px longest edge, <=128 KiB PNG payload, generated at capture time (falls back to 96/64 px if needed, never upscales).
- Exposed image fields: image_path, width, height, thumbnail_path, mime (image/png), format (png|dibv5|dib|bitmap|hdrop|html|rtf|text), ocr_text (reserved). All serde-defaulted; ClipCategory adds Files.
- Retention: unpinned history <=1000 rows and image cache <=256 MiB; pinned rows/files never pruned; delete removes the PNG+thumbnail. Re-copying identical pixels refreshes the timestamp.

## Tests (this host)
- `cargo test -p ps-windows`: 7 unit decoder tests + 11 integration tests = 18 passed / 0 failed (BI_BITFIELDS, DIBV5, palette, top-down, malformed/oversized rejection, colour-channel assertions, paste payload round-trip, listener restart).
- `cargo test -p ps-sources`: 18 unit + 31 integration = 49 passed / 0 failed (1 pre-existing ignored action-smoke), including image metadata/thumbnail/restart, paste round-trip, mixed one-item persistence, Files category, capture toggles, contract bounds, retention, delete cleanup.
- `cargo check -p ps-windows -p ps-sources -p ps-content -p ps-ocr --all-targets`: clean. `cargo check -p ps-core`: clean.

## Evidence artifacts
- Repaired format matrix: @evidence/logs/repair_format_matrix.log; decoded PNGs: @evidence/repair-decoded/
- Repaired DB matrix + paste + restart: @evidence/logs/repair_db_matrix.log, repair_paste.log, repair_restart_inspect.log
- Real .NET image: repair_external_dotnet_listener.log, repair_capture_dotnet_image.log, repair_external_paste.log, repair_external_restart.log, repair_db_schema.log
- Commands: harness build `cd docs/qa/clipboard-audit/harness && cargo build --target-dir <repo>/target/clipboard-audit`; run `db_audit --listen-external/--capture-external/--paste-latest/--inspect --root <dir>`; set image via @evidence/scripts/set_clipboard_image.ps1.

## Remaining / not yet verified
- OLE-only clipboard sources (data exposed only through IDataObject) are not read yet; plain Win32 formats cover the tested real cases. Add OleGetClipboard/IDataObject fallback in a follow-up.
- Real interactive Snipping Tool (Win+Shift+S), Paint and browser copy-image flows still need manual/automated GUI E2E; the format families are covered by synthetic Win32 sets and real .NET SetImage.
- Clipboard image OCR text is not populated yet (ocr_text reserved); UI rendering/thumbs is the island/desktop teams' task (engine maps the new fields).
- No benchmark numbers claimed here; numbers are functional evidence only.