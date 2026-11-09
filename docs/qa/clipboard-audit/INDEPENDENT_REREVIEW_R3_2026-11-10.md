# Independent re-review - clipboard image P0 (R3 / task-46)

Reviewer: L1-E (rel-recon). Date: 2026-11-10. Snapshot: HEAD 93f84d1 + working tree. No code edits.
Purpose: independently verify the Gate-2 clipboard-image repair (R3) rather than trusting the owner's
report. Owner evidence: docs/qa/clipboard-audit/BOOT_P0_REPAIR_EVIDENCE.md.

## Commands and results

1. Independent test suite:
   cargo test -p ps-ipc -p ps-core -p ps-windows -p ps-sources
   -> exit 0; ps-ipc 17 passed; ps-core 48 passed; ps-windows 7 unit + 11 integration = 18 passed;
      ps-sources 18 unit + 31 integration = 49 passed, 1 pre-existing ignored action-smoke.
   Log: .agents/notes/g2-rereview-tests.log.
   Notable tests: ps-windows imp::clipboard::decode_tests (BI_BITFIELDS, DIBV5, RGB24/32, palette,
   top-down, malformed/oversized, paste round-trip); ps-sources clipboard (image stored as PNG,
   dedupe, thumbnail within IPC contract, oversized dimensions rejected, capture options vs master
   gate, image metadata survives restart and copies back, retention, delete cleanup).
2. Rebuilt the audit harness from the current tree:
   cd docs/qa/clipboard-audit/harness; cargo build --target-dir <repo>/target/clipboard-audit -> exit 0.
3. Real .NET clipboard image through the production listener:
   - started target/clipboard-audit/debug/db_audit.exe --listen-external --secs 20 --root
     docs/qa/clipboard-audit/evidence/runtime/rereview-external (root cleaned first);
   - from a separate STA PowerShell process ran docs/qa/clipboard-audit/evidence/scripts/
     set_clipboard_image.ps1 (System.Windows.Forms.Clipboard.SetImage of a 320x200 bitmap, the exact
     format family that failed at Gate 0);
   - listener output: "observed count change: 1 -> 2"; row category=Image, PNG 6,315 B,
     thumbnail 3,671 B (128x80), format=dibv5, imageWidth/Height=320x200.
   These bytes match the owner's BOOT evidence exactly (same PNG hash prefix 5c4c4af7...), which is
   independent corroboration of the repair.
4. Paste-back: db_audit --paste-latest -> "row id=2 width=320 height=200 format=dibv5; clipboard now
   carries 320x200 PNG (6315 bytes)".
5. Inspect: db_audit --inspect lists 2 clipboard rows + 4 image cache files (full PNG + thumb per row);
   DB and image files are confined to the supplied root.

## Bounds / gate / contract checks (code + tests)

- Master gate: privacy.clipboard_history AND !clipboard.pause; capture_text/capture_images only filter
  formats while the gate is open (ps-ipc ClipboardSettings doc; ps-sources tests
  clipboard_capture_options_filter_formats_not_master_gate; engine clipboard_listener_lifecycle test).
- Bounds: thumbnail <=128 px longest edge and <=128 KiB PNG; source dimensions <=8192; MIME image/png
  allowlist; IPC exposes only thumbnailDataUrl/imageWidth/imageHeight/mime (ResultMeta).
- Bounds tests: clipboard_thumbnail_is_within_ipc_contract, clipboard_rejects_oversized_image_dimensions.
- Storage: content-hash PNG + thumbnail files; retention (unpinned <=1000 rows, image cache <=256 MiB);
  delete removes PNG + thumbnail; recopy identical pixels refreshes timestamp.

## Caveats (honest)

- The synthetic "scoped listener matrix" run has harness sequencing/baseline artifacts for some
  scenarios (first scenario sees a pre-existing seeded row; dib32/dibv5/mixed deltas vary). Direct
  capture_current, the unit/integration tests and the real .NET external capture all pass; the
  synthetic artifacts are test-harness sequencing, not production defects. Do not cite the synthetic
  matrix deltas as pass/fail.
- Not independently re-run in this pass: full app-process GUI E2E with interactive Snipping Tool/
  Paint/browser copy-image flows; OLE-only sources (IDataObject) are not implemented yet; clipboard
  image OCR text is reserved. These remain G2 acceptance/follow-up items per BOOT section
  "Remaining / not yet verified".
