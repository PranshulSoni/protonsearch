# ProtonSearch v2 — G8 Verification Round 1 (2026-11-10)

Owner: L1-E (rel-recon). Task: task-50. Snapshot: HEAD eb61b82 plus working tree. Reviewer of
record: L1-E; L0 is the acceptance authority.

## 1. Verdict

G8 PARTIAL — ROUND 1 COMPLETE, NOT SIGNED. The P0-1/P0-2 product paths are demonstrably working on
this host (frontend suite green; live P0 preview payloads and screenshots; live real-Windows clipboard
image capture with island thumbnails and restart persistence). G8 cannot be signed because: (a) the
full Rust workspace suite is currently blocked by a concurrent uncommitted task-35 change in
ps-windows; (b) the pairwise matrix is only partially executed; (c) native adapter cards are
honest-disabled; (d) island/launcher native DPI, High Contrast and frame-pacing evidence is missing;
(e) the paste-back-through-island-UI path and some preview formats were not exercised; and
(f) environment limits remain (single monitor, no signing certificate, no non-Pi credentials).

Snapshot details: HEAD eb61b82 (content retrieval optimization, preview UI wired, island deltas,
native controls IPC read-model) plus working tree; untracked task-35 files
crates/ps-windows/src/imp/{audio,brightness,winrt}.rs are mid-landing. App binary:
target/debug/ProtonSearch.exe (2026-11-10 02:44); frontend dist rebuilt by this review (npm run build
exit 0). Data profile: docs/qa/g8-verify/appdata (fixtures root, 11 filename entries, content+OCR
enabled, clipboard history enabled).

## 2. Suites

| Suite | Result | Evidence |
|---|---|---|
| Frontend unit tests | PASS - 18 files / 237 tests passed in 8.44 s | .agents/notes/g8-frontend-tests.log |
| Frontend production build | PASS - vite build exit 0 | .agents/notes/g8-frontend-build.log |
| Rust targeted (ps-ipc + ps-agent) | PASS - ps-ipc 17/17; ps-agent 75+5+16+4 = 100/100; 0 failed | .agents/notes/g8-partial-tests.log |
| Full Rust workspace | BLOCKED - cargo test --workspace exit 101: ps-windows fails to compile because task-35 has uncommitted mid-landing files (unresolved Capability/NativeCapabilities/RECT/LocalFree errors). Not a product regression; rerun after that work lands and the tree is green. | .agents/notes/g8-workspace-tests.log |
| Last full independent green baseline | PASS at 93f84d1: ps-ipc 17, ps-core 48, ps-windows 18, ps-sources 49 (1 pre-existing ignored) = 132 passed | docs/qa/clipboard-audit/INDEPENDENT_REREVIEW_R3_2026-11-10.md |
| Clipboard test contention | Handled - all Rust tests ran with no ProtonSearch.exe instance running (workspace rule); the app was launched only for live E2E afterwards | process checks in command log |

## 3. Real Windows E2E (live app, CDP-driven)

Method: debug app + vite preview server (localhost:5173, debug build devUrl), portable profile
docs/qa/g8-verify/appdata, WebView2 user data under docs/qa/g8-verify, CDP on 127.0.0.1:9223.
Screenshots/JSON: docs/qa/g8-verify/. Automation stability note: two long automated sessions lost the
CDP endpoint while the host process remained alive (no new ProtonSearch WER dump; only old
msedgewebview2 dumps from 17:39). Live interactions were therefore split into short sessions. A native
manual GUI pass is required before sign-off; this is a verification limitation, not yet a confirmed
product crash.

### 3.1 Launcher latency and baseline captures (e2e-drive.mjs, real app)

- 3 webview pages (island, main, launcher); engine state ready, 11 filename entries; portable profile.
- Input-to-visible result latency (real app, double rAF): architecture 19.0 ms, readme 2.7 ms,
  proton-serch 5.1 ms, zero-result 4.1 ms. Evidence: docs/qa/g8-verify/e2e-launcher/
  latency-real-app.json + real-launcher-{empty,architecture,recent-exact,typo,zero}.png and
  real-app-{home,search-indexes,activity,workflows,preferences}.png.

### 3.2 P0-1 previews (search result -> preview payload -> launcher UI)

API-level payloads through the live app (get_preview, resultId-only):

| Fixture | Result | Evidence |
|---|---|---|
| notes.txt | PASS - status=ok, format=txt, text 191 chars | p0-1-preview-evidence.json |
| report.pdf | PASS - status=ok, format=pdf, text 83 chars | p0-1-preview-evidence.json |
| diagram.png | PASS - status=ok, format=png, imageDataUrl 26,746 chars, OCR text 45 chars, 1 highlight | p0-1-preview-evidence.json |
| blob.bin | PASS - status=unsupported with explicit "Preview is not available for .bin files." | p0-1-preview-evidence.json |
| large.txt | PASS - status=ok, text capped and truncated=true | p0-1-preview-evidence.json |
| report.docx (UI) | PASS - real DOCX preview text (KOALA-DOCX-2026, fields Size/Modified/Lines/Words) rendered in the live launcher | docs/qa/g8-verify/p0-1-preview-report.png |
| blob.bin (UI) | PASS - explicit unsupported message rendered, no blank panel | docs/qa/g8-verify/p0-1-preview-blob.png |
| MD text (live UI, second session) | PASS - real markdown preview with truncation indicator and Open/Reveal/Copy path actions | docs/qa/visual/live-shots/launcher-preview-real2.png |

Not executed this round: code, Markdown/HTML, JSON/CSV previews; highlight of a matched term in body
text; Unicode/Hindi/CJK/emoji highlight; stale/rename/delete preview; image preview screenshot in both
launchers. Engine tests (ps-core 48 incl 8 preview tests) cover the payload logic; these are G8
sign-off items.

### 3.3 P0-2 clipboard images (real .NET SetImage -> history -> island Images tab -> restart)

Procedure: with the app running and clipboard history enabled, a separate STA PowerShell process ran
System.Windows.Forms.Clipboard.SetImage (320x200 bitmap via
docs/qa/clipboard-audit/evidence/scripts/set_clipboard_image.ps1); the island Clipboard card was opened
and the Images tab captured.

| Check | Result |
|---|---|
| Live capture | PASS - clipboard source count 3 after copy; new row clip:3 "Image 320 x 200"; full PNG 6,315 B + thumbnail 3,671 B (hash 5c4c4af7...) under docs/qa/g8-verify/appdata/v2/clipboard-images |
| IPC contract | PASS - search scope clipboard returns clip:3 with meta.thumbnailDataUrl 4,918 chars, imageWidth 320, imageHeight 200, mime image/png; thumbnails present for both image rows |
| Island Images tab | PASS - live screenshot shows Image 320 x 200 "just now" plus older image rows with real 34x34 thumbnail images (data:image/png) | docs/qa/g8-verify/p0-2-island-images.png |
| Restart persistence | PASS - after app restart with the same data dir clipboard count is still 3; same clip:3 row + 4,918-char thumbnail; island Images tab still renders the 320x200 entry | docs/qa/g8-verify/p0-2-island-images-after-restart.png |
| Paste-back through UI | NOT VERIFIED this round - clicking the island image row did not put an image back on the clipboard (clipboard stayed on the text marker); the row may need a different action/keyboard path. Harness-level paste-back is verified (R3: db_audit --paste-latest wrote 320x200 PNG; clipboard_image_metadata_survives_restart_and_copies_back test). | R3 evidence |

Not executed this round: interactive Paint/Snipping Tool/browser copy-image GUI matrix; privacy
off/paused negative tests through the live app; clear-on-exit; image OCR text in the UI (reserved).

### 3.4 Island/launcher surfaces

- Island renderer: 8 cards + dots + motion tokens verified earlier; live island screenshots and the
  fresh clipboard Images tab exist. Media/Quick Settings/Notifications remain honest-disabled
  (task-35 native adapters in progress); Agent card hierarchy and pinning gaps remain.
- Normal Launcher: live screenshots for empty/query/zero states; 4-filter model and quick destinations
  per Lead addendum; design-spec micro-spec sync and native DPI evidence still pending.

## 4. Feature matrix execution (round 1; full matrix stays planned)

| Matrix IDs | Status | Evidence |
|---|---|---|
| FT-L02 Normal Launcher filters | PASS (implemented; 4 visible filters + prefix access) | live screenshots, launcher tests; design-spec sync pending |
| FT-L03 Quick destinations | PASS (implemented, honest off states) | real-launcher-empty.png; addendum C2 |
| FT-L04 Dynamic Island mode | PARTIAL - renderer/cards/dots exist; native adapters/agent hierarchy incomplete | island live shots + audits |
| FT-L05 Primary launcher scoping | PARTIAL - settings model/UI implemented; live mode-switch E2E not re-run | settings audit/task-40 |
| FT-S01 Files/folders search | PARTIAL - live filename search works; full source matrix not executed | e2e-drive latency + screenshots |
| FT-S03 Full-text content | PARTIAL - fixtures search/preview work; official p95 target unconfirmed | preview evidence, perf JSON |
| FT-S10 Clipboard text/categories | PARTIAL - live image rows/categories visible; text/pin/delete matrix not executed | island Images tab + IPC rows |
| FT-S11 Clipboard images (P0-2) | PASS - live .NET capture, thumbnails, restart persistence; paste-back UI + GUI matrix follow-ups | section 3.3 |
| FT-P01 TXT preview | PASS | p0-1 evidence |
| FT-P05 PDF text preview | PASS | p0-1 evidence |
| FT-P07 DOCX/Office preview | PASS | p0-1-preview-report.png |
| FT-P08 Image preview | PASS (payload + OCR text + highlight count; screenshot not re-captured) | p0-1 evidence |
| FT-P09 Binary/unsupported fallback | PASS | p0-1-preview-blob.png |
| FT-P02/P03/P04/P06/P10 | NOT EXECUTED this round | G8 sign-off item |
| FT-S12 Agents | BLOCKED/PARTIAL - no Pi credentials for live run; read-only alpha gate closed | security review |

## 5. Pairwise matrix execution (round 1)

| Pair | Status | Evidence / gaps |
|---|---|---|
| P-03 clipboard image x Images category x restart x re-paste | PARTIAL PASS - capture/category/thumbnails/restart pass live; UI re-paste unverified; Paint/Snipping/browser sources and privacy-paused variants not executed | docs/qa/g8-verify/p0-2-* |
| P-07 search result x preview x file type | PARTIAL PASS - TXT/PDF/DOCX/PNG(+OCR)/binary executed live; code/MD/HTML/JSON/CSV and both-launcher parity not executed | p0-1 evidence |
| P-10 card order x dots x scroll x default x disabled x restart | PARTIAL - 8 cards + dot navigation verified in tests/screenshots; reorder/default/disabled/restart combination not executed | island live shots |
| P-13 settings Apply/Cancel x ghost preview x persistence x hotkey | PARTIAL - settings/placement UI implemented (task-40); full apply/cancel/reset/restart E2E not executed | task-40 screenshots |
| P-26 preview x both launchers | PARTIAL - normal launcher preview verified live; island search card has no preview hookup | island audit gap 4 |
| P-01/P-02 launcher/island x every source | PARTIAL - representative sources on both surfaces; full parity matrix not executed | live shots |
| P-06 clipboard paste x focused target, P-09 stale preview, P-17 agent lifecycle | NOT EXECUTED/BLOCKED (paste target GUI matrix, stale fixture, no Pi credentials) | — |
| P-11/P-12/P-14 placement/fullscreen/multi-monitor | BLOCKED/PARTIAL - single-monitor host | environment blocker |
| P-23 update/migration/rollback | NOT STARTED (G9) | — |

## 6. Gate verdicts after round 1

| Gate | Verdict | Notes |
|---|---|---|
| G1 | CLOSED (recommended) | unchanged from task-46 |
| G2 | PASS (conditional) - P0-1/P0-2 product paths demonstrated live; sign-off conditions below | full format matrix + both-launcher preview UI + app GUI clipboard re-paste matrix remain |
| G3 | CONDITIONAL | official quiet content run overall warm p95 41.20 ms (before 42.79 ms); targeted 28.66 ms A/B claim not yet reproduced in the official quiet artifact; formal rerun + reproducible report needed |
| G4 | CONDITIONAL | live launcher works; design-spec micro-spec sync + native DPI/High Contrast evidence needed |
| G5 | PARTIAL / NOT SIGNED | renderer exists; native adapters honest-disabled; pinning/agent hierarchy/native DPI/frame pacing pending |
| G6 | CONDITIONAL | settings/placement/card manager implemented; apply/cancel/reset/restart native E2E + DPI evidence pending |
| G7 | PARTIAL | Pi read-only hidden; native adapters in progress (task-35 currently breaking workspace compile); no live agent runs |
| G8 | PARTIAL (this round) | suites partially blocked, matrices partially executed, live E2E partial, environment gaps |
| G9 | NOT OPEN | signing/updater/installer evidence pending |

## 7. Remaining blockers / sign-off requirements for G8

1. Green tree: task-35 lands/commits; cargo test --workspace and frontend suites pass on the exact
   release-candidate build (record commit hash); rerun the matrix on that binary.
2. Preview acceptance: execute FT-P01..P10 in the launcher UI (TXT/code/MD/HTML/JSON/CSV/PDF
   text/scanned OCR/DOCX/image/binary) incl. matched-term highlight, Unicode/Hindi/CJK/emoji,
   stale/rename/delete; verify both launchers (island search card needs the preview hookup); screenshots
   for each; no blank panels.
3. Clipboard acceptance: full app GUI matrix (Paint, Snipping Tool, browser copy-image, Office),
   paste-back to Notepad/Paint/browser from the app, privacy off/paused/clear-on-exit, restart,
   retention; resolve/verify the island row paste-back path.
4. Pairwise execution: P-03, P-06, P-07, P-09, P-10, P-13, P-17, P-26 at minimum, with real Windows
   evidence and both-launcher parity where applicable; record pass/fail per row.
5. Island/launcher native visuals: 100/125/150/200% DPI, light/dark/High Contrast, 560 narrow,
   taskbar edges, multi-monitor when hardware exists; frame-pacing measurements; all 8 cards or
   explicit CUT decisions for Media/Controls/Notifications (task-35).
6. Agent/native: read-only Pi end-to-end with a real provider or explicit credential-unavailable gap;
   native adapter capability matrix; no fake data.
7. Performance: reproducible official quiet rerun of the content p95 target (28.66 ms A/B claim
   currently unconfirmed by the official quiet artifact) + launcher input latency + memory/idle.
8. Security/release: RR-02 write consent remains closed; unsigned artifact statement; migration/
   rollback drill; installer hashes; secret scan.
9. Environment: single monitor blocks dual/mixed-DPI/disconnect; no signing certificate; no non-Pi
   provider credentials; no clean VM/second machine.
10. Independent acceptance: L0 accepts; reviewer L1-E does not self-accept; at least one other lead
    re-runs a sample of the executed matrix and signs the evidence.

## 8. Evidence index

- docs/qa/g8-verify/: p0-1-preview-evidence.json, p0-1-preview-report.png, p0-1-preview-blob.png,
  p0-2-island-images.png, p0-2-island-images-after-restart.png, e2e-launcher/ (latency-real-app.json +
  10 live screenshots), raw driver scripts.
- docs/qa/visual/live-shots/: launcher-preview-real2.png (real MD preview), island live card shots.
- .agents/notes/: g8-frontend-tests.log, g8-frontend-build.log, g8-partial-tests.log,
  g8-workspace-tests.log (blocked), g2-rereview/g8 independent suite logs.
- Perf: docs/performance/results/v10-g3-content-quiet-official-eb61b82-...json (p95 41.20 ms overall).

Status: G8 round 1 PARTIAL; no release GO. Re-run after task-35 lands and after the sign-off items
above are executed on a green build.
