# ProtonSearch v2 — Bug Ledger (v10 Gate 0)

Owner: L1-E (rel-recon). Status: Gate-0 seed. Update rule: a bug row is updated only with evidence
(command output, log, screenshot, test ID). No fabricated reproductions. P0 items are the two bugs the
user explicitly reported in the v10 handoff; G0-A (task-29) and G0-D (task-30) are reproducing them.
Fields: ID, severity (P0/P1/P2/P3), status (OPEN / REPRODUCING / REPRODUCED / FIXED-UNVERIFIED /
VERIFIED / DEFERRED / CUT), symptom, expected, environment, reproduction steps, suspected area, owner,
reviewer, gate, evidence, acceptance criteria.

## 1. P0 bugs (user-reported; must be reproduced and fixed at Gate 2)

### BUG-P0-01 — File/text/document previews do not show
- Severity: P0. Status: REPRODUCING (G0-A, task-29, owner engine-core; no reproduction log accepted yet).
- Report: master prompt section 0 item 5 and section 7 ("file/text previews do not show currently"),
  handoff README. User-visible: selecting a search result yields no usable preview/snippet.
- Expected: selecting a result shows verified correct preview for TXT/code/Markdown/HTML/JSON/CSV/
  PDF/DOCX/image/OCR, with matched-term highlight, line/page info, explicit unsupported/error fallback,
  debounced/cancelled async loading, stale-response guards, bounded memory, no blank panel.
- Current evidence: v2 has a ResultPreview component and preview IPC fields, and launcher tests exercise
  the preview panel (apps/desktop/src/launcher/ResultPreview.tsx; launcherSurface.test.tsx "preview pane"),
  but the end-to-end production path is reported broken. ACCEPTANCE_REPORT v2 already notes
  "preview bodies limited" and preview work as a known gap. No accepted Gate-0 reproduction exists yet.
- Environment: Windows 11 build 26300, single 2880x1800 @ 120 Hz, effective 150% DPI, WebView2
  157.0.4341.42, release/debug build hash to be recorded per run.
- Reproduction plan (G0-A):
  1. Build/run the current app from a clean tree; keep a debug log.
  2. Index a controlled corpus (ASCII/UTF-8/BOM, Hindi/CJK/emoji, 0-byte, 1 MB, 100 MB text, CRLF/LF,
     invalid bytes, PDF text, scanned PDF, DOCX table, Markdown/code with matches, deleted/moved/locked
     file, encrypted PDF, unsupported binary).
  3. From the Normal Launcher, query each fixture, select the result, and record: IPC SearchResult
     payload (preview fields/snippet offsets), preview request/response, UI state, error/empty panel,
     console/WebView2 errors, and a screenshot.
  4. Repeat with preview independent of indexing (direct preview request) and via the Dynamic Island
     path once G5 exists; at Gate 0 island path is NOT AVAILABLE and must be marked blocked, not skipped.
  5. Compare engine content snippets with UI display; test rename/delete while selected; test huge and
     binary files for bounded memory/timeouts.
- Suspected areas: crates/ps-content extraction/snippet/offset code; crates/ps-ipc preview DTOs;
  apps/desktop/src-tauri preview commands; apps/desktop/src/launcher/ResultPreview.tsx and LauncherSurface
  state; stale request/cancellation logic in bridge.
- Acceptance: BUG_LEDGER evidence (before/after) + FEATURE_TEST_MATRIX preview rows green for both
  launcher presentations (island row blocked until G5, then must pass); no silent blank panel; security
  review for HTML/office/PDF preview (no script/macro/external fetch).
- Gate: G2 (must-fix before glamorous features). Reviewer: L0 + L1-E.

### BUG-P0-02 — Clipboard does not capture images
- Severity: P0. Status: REPRODUCING (G0-D, task-30, owner sources-content; no reproduction log accepted yet).
- Report: master prompt section 0 item 5 and section 8 ("clipboard does not capture images currently").
  User-visible: copying a bitmap (Paint/Snipping Tool/browser image) does not produce a clipboard image
  history item/thumbnail and cannot be pasted back through ProtonSearch.
- Expected: real Win32 clipboard image capture (CF_DIB, CF_DIBV5, CF_BITMAP when exposed, registered PNG
  or equivalent), dedupe one copy event into one item across formats, values classified into
  All/Text/Images/Links/Code/Files/Pinned, stable PNG + content hash + dimensions + thumbnail, atomic
  persistence across restart, real re-copy/paste into compatible apps, pinned/delete/clear/retention,
  privacy pause/exclusions, no UI blocking while decoding.
- Current evidence: v2 code contains CF_DIB decode/PNG encode and a clipboard listener
  (crates/ps-windows/src/imp/clipboard.rs: CF_DIB constant, get_image/get_image_png, decode_dib,
  capture_snapshot) and ps-sources image recording with PNG files and 32 MB cap
  (crates/ps-sources/src/clipboard.rs), yet the user reports images missing. This points to a
  wiring/format/dedup/persistence/UI issue, not necessarily absent decoding; G0-D must locate the broken
  link rather than assume. No accepted reproduction yet.
- Environment: same host; clipboard tests require real interactive apps (Paint, Snipping Tool, VS Code,
  browser) and a normal user session.
- Reproduction plan (G0-D):
  1. Ensure clipboard privacy capture is enabled for the test (it defaults OFF in v2).
  2. Copy text Notepad; copy code VS Code; copy URL; copy file(s) in Explorer; copy image via Paint,
     Snipping Tool, browser "Copy image"; copy HTML-rich content; include large/high-DPI transparent
     image and rapid sequential copies.
  3. After each event record: clipboard sequence/update event, formats exposed, decode result, DB row
     (is_image/image_path/ocr_text/category), PNG file existence/hash, UI row/thumbnail, and restart
     persistence; attempt paste-back into Paint/Word/VS Code.
  4. Test locked clipboard, empty clipboard, restart during capture, pinned retention, delete/corrupt
     record recovery, duplicate-notification dedupe.
  5. Dynamic Island clipboard card path is blocked until G5; Normal Launcher/history path tested now.
- Suspected areas: ps-windows clipboard listener/format negotiation; ps-sources record_image/open_or_create,
  dedupe/category; engine settings privacy gate; Tauri clipboard commands; launcher clipboard grouped
  sections UI (recently reworked by tasks 24/25).
- Acceptance: BUG_LEDGER before/after evidence with real pixels; FEATURE_TEST_MATRIX clipboard rows green;
  restart persistence; both launcher presentations (island blocked until G5, then must pass); privacy
  controls verified; unit mocks are not sufficient.
- Gate: G2 (must-fix). Reviewer: L0 + L1-E.

## 2. Other known issues (from existing evidence; not P0 for v10)

| ID | Sev | Status | Summary | Evidence / owner / gate |
|---|---|---|---|---|
| BUG-P1-01 | P1 | OPEN | v2 content retrieval p95 43 ms (HEAD 8692997) misses 30 ms target; legacy FTS baseline 6 ms | docs/performance/BENCHMARKS.md sections 4,7; owner L1-A/L1-C; G3 |
| BUG-P1-02 | P1 | OPEN | 1M filename cold process load 467-476 ms / cold load 442-448 ms (111 MB binary) threatens hotkey->visible target | BENCHMARKS section 3.3; G3/G4 |
| BUG-P1-03 | P1 | OPEN | Agent write-enabled runs: per-run write consent is UI-only; persisted allow_writes trusted by shell (PI_AGENT_SECURITY RR-02) | docs/security/PI_AGENT_SECURITY.md section 7; owner L0/L1-D; G6/G7 |
| BUG-P1-04 | P1 | OPEN | Workflows UI copy claims workspace confinement that does not exist (RR-01) | PI_AGENT_SECURITY section 7; owner L2-B/L1-E; G6 |
| BUG-P1-05 | P1 | OPEN | OCR text is not yet indexed in v2 search path (ps-ocr extraction exists; integration pending) | ACCEPTANCE_REPORT; BENCHMARKS section 7; owner L2-A; G3 |
| BUG-P1-06 | P1 | OPEN | Installer/updater unsigned; no signed manifest/Authenticode; S-02 legacy updater risk register still a release gate | docs/release/SECURITY.md; owner L1-E; G9 |
| BUG-P1-07 | P1 | OPEN | Island mode not implemented; no V9 fidelity evidence yet | tree has no island code; owner L1-B; G5 |
| BUG-P1-08 | P2 | OPEN | UI latency only a 4-query debug sample; hotkey->visible and release-build p95 not measured | BENCHMARKS section 8; owner L1-C; G8 |
| BUG-P1-09 | P2 | OPEN | Clipboard privacy: source-app exclusions/TTL/DPAPI hardening pending; capture defaults off | PI/v2 reviews; owner L2-A; G2/G9 |
| BUG-P1-10 | P2 | OPEN | First run of broad UI query returned 19 rows for a "zero" query (empty-state/relevance question) | latency-real-app.json note in BENCHMARKS section 8.4; owner L1-A/L1-C; G3 |
| BUG-P1-11 | P2 | BLOCKED (env) | No second monitor/mixed DPI/disconnect hardware for placement and dual-launcher tests | V10_GATE0_AUDIT blockers; owner L1-E; G8 |
| BUG-P1-12 | P2 | BLOCKED (credential) | No non-Pi agent/provider credentials; live model runs unverified | PI_AGENT_SECURITY; owner L1-D; G7 |

## 3. Ledger rules

1. P0/P1 defects need an owner, reviewer and gate; no defect may disappear without a CUT/DEFERRED
   rationale accepted by L0.
2. Reproduction must include the exact command/fixture/hash and expected vs observed; screenshots or
   logs are linked from the row.
3. Fixes are VERIFIED only after the independent reviewer re-runs the reproduction and the related
   FEATURE_TEST_MATRIX/CROSS_FEATURE_PAIRWISE rows pass.
4. G0-A/G0-D outputs are appended here with their evidence links; until then the P0 rows stay
   REPRODUCING and may not be treated as confirmed root causes.
