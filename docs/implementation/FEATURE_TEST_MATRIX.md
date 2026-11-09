# ProtonSearch v2 — Feature Test Matrix (v10 Gate 0)

**Owner:** L1-E (rel-recon). **Status:** Gate 0 seed. Every row is a claim to be verified, not a
finished result. Statuses here are honest snapshots at tag `pre-v10-handoff` / HEAD `0e4c9ec`.
**Columns:** Status before v10 = current tree behavior (evidence); Implemented behavior = required
v10 behavior; Test cases = planned IDs; Unit/Integration/Win E2E = evidence type expected; Screens =
visual evidence required; Permissions = OS/user capability needed; Regression = existing coverage
state; Reviewer = independent reviewer; Gate = first gate where the claim must be accepted.
A feature is DONE only when L0 accepts executable evidence (master prompt section 13).

## A. Launcher modes and shell

| ID | Feature | Status before v10 | Implemented behavior (target) | Test cases | Unit | Integration | Win E2E | Screens | Permissions | Regression | Reviewer | Gate |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| FT-L01 | Normal Launcher window | PARTIAL (v2 frameless launcher exists; centered work-area placement in Tauri command; no v10 acceptance) | centered on current monitor/work area, mixed DPI safe, no fullscreen, correct focus | L01-01..08 | geometry helpers | bridge mock + shell | yes | light/dark, 100-200% DPI, 4 sizes | none beyond user session | partial | L1-B | G4 |
| FT-L02 | Normal Launcher filters | NOT STARTED (visible All/Files/Folders/Apps filter row not implemented; sources exist behind scope field) | four visible filters + syntax/contextual access to other sources | L02-01..10 | filter reducer | scope contract | yes | each filter state + empty | none | none | L1-B | G4 |
| FT-L03 | Quick destinations (empty state) | NOT STARTED (v2 empty state is recent/suggested rows, not required destinations) | Bookmarks, History, Clipboard, Agents visible; Git + Agent History accessible; customizable recents | L03-01..08 | reducer | bridge mock | yes | 5 viewports | clipboard source opt-in | none | L1-B | G4 |
| FT-L04 | Dynamic Island mode | NOT STARTED (no island code in tree) | V9 hidden capsule, Alt+Space, compact Search, morphing cards, dots, scroll/swipe | L04-01..40 | component tests | IPC contract | yes | all 8 cards + states 100-200% | hotkey | none | L1-B | G5 |
| FT-L05 | Primary launcher scoping | PARTIAL (one launcher surface; no mode setting) | Settings/onboarding choice between Normal and Island without index rebuild | L05-01..05 | settings model | migration | yes | onboarding screens | none | none | L0 | G6 |
| FT-L06 | Global hotkey | DONE in v2 (Tauri global shortcut registered; conflict messages) | reliable RegisterHotKey, conflict-safe remap, restore focus | L06-01..05 | config tests | shell command | yes | conflict + success | none | partial | L1-E | G4 |
| FT-L07 | Tray / startup residency | PARTIAL in v2 (tray exists; startup setting exists) | tray, start-minimized, reopen behavior, single instance | L07-01..04 | config | shell | yes | tray menu | none | partial | L1-E | G4 |

## B. Search sources (must work in BOTH launchers by G8)

| ID | Feature | Status before v10 | Target behavior | Test cases | Unit | Integration | Win E2E | Screens | Permissions | Regression | Reviewer | Gate |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| FT-S01 | Files/folders search | PARTIAL (v2 ps-index works; 100k/1M targets met; v10 acceptance in both launchers pending) | real files/folders, stable IDs, ranking, no fake counts | S01-01..12 | ps-index suite | engine facade | yes | both launchers | user-readable paths | bench | L1-A | G3/G4/G5 |
| FT-S02 | Installed apps + UWP | PARTIAL (ps-sources apps implemented; launch path exists) | enumerate and launch desktop + packaged apps, real icons | S02-01..08 | ps-sources | engine | yes | app rows | none | partial | L2-A | G3 |
| FT-S03 | Full-text content | PARTIAL (FTS5 works; content p95 43 ms misses 30 ms target at HEAD 8692997) | indexed PDF/DOCX/code/MD/text, snippets + line numbers | S03-01..15 | ps-content | engine | yes | result rows | none | bench | L1-A/L1-C | G3 |
| FT-S04 | OCR search | PARTIAL/NOT STARTED (ps-windows OCR extracted; OCR FTS/search wiring not integrated at Gate 0) | indexed OCR text search, extraction measured separately from search | S04-01..12 | ps-ocr tests | engine | yes | image result + snippet | language packs | none | L2-A | G3 |
| FT-S05 | Actions / commands | PARTIAL (actions implemented with confirmations; v2 command surface limited) | safe action catalog, destructive confirmations, honest failure | S05-01..15 | unit | engine | yes | action rows | varies | partial | L1-E | G3/G4 |
| FT-S06 | Calculator / conversion | DONE in v2 (unit tests) | preserved parity | S06-01..06 | calc tests | engine | yes | result row | none | yes | L1-A | G3 |
| FT-S07 | Quicklinks/snippets | PARTIAL (data store/search exists; v10 UI/import-export not verified) | preserved sources, editable in v10 settings | S07-01..06 | store | engine | yes | rows | none | partial | L2-A | G3 |
| FT-S08 | Browser bookmarks/history | PARTIAL (ps-sources browser implemented; source adapter not E2E-verified at Gate 0) | real Chromium/Firefox profiles, dedupe, open action | S08-01..10 | parsers | engine | yes | rows | browser files opt-in | partial | L2-A | G3 |
| FT-S09 | Git commits/TODOs | PARTIAL (ps-sources git implemented; V9 Git card not started) | repo discovery, commits, TODOs, open-at-line | S09-01..08 | parsers | engine | yes | git card + rows | git installed | partial | L2-A | G3/G5 |
| FT-S10 | Clipboard text/link/code/files/pinned | PARTIAL (v2 listener + categorized rows implemented; privacy opt-in default off; v10 cards not started) | history with categories, pin/delete/copy/paste, filters, retention | S10-01..18 | clipboard suite | engine | yes | normal + island card | clipboard access | partial | L2-A | G2/G5 |
| FT-S11 | Clipboard images (P0-2) | BROKEN per user report; decode/record code exists but UI history reportedly missing — reproduction in progress (task-30) | real bitmap capture (CF_DIB/CF_DIBV5/CF_BITMAP/PNG), thumbnail, persistence, re-paste | S11-01..20 | decode tests | engine | yes | both launchers + restart | clipboard access | none | L2-A + L1-E | G2/G5 |
| FT-S12 | Agents / agent history | PARTIAL (Pi backend hardened; shell wired; Workflows write-consent gate open; V9 agent card not started) | real Pi runs, status/hierarchy/history, permission policy, honest controls | S12-01..14 | ps-agent 71 tests | shell/UI | yes | Workflows + card | opt-in runtime | partial | L1-D | G5/G7 |
| FT-S13 | Settings catalog search | PARTIAL (v2 settings search exists as FTS source) | v10 settings entries discoverable | S13-01..05 | FTS tests | engine | yes | rows | none | partial | L1-A | G4 |
| FT-S14 | Web search / lens / window controls / other legacy sources | NOT STARTED / CUT decisions documented in FEATURE_PARITY_MATRIX | explicit decision per legacy source, no silent removal | S14-01..06 | — | — | yes where kept | n/a | — | none | L1-E | G3/G4 |

## C. Preview pipeline (P0-1) — required formats

Every row must include: fixture path, expected excerpt/hash, matched-term highlight, unsupported/
error fallback, stale-cache behavior, and both launcher presentations by G8. Current state: v2
ResultPreview component and engine preview data exist, but the user reports previews do not show;
G0-A (task-29) is reproducing and auditing the full pipeline. **No row is accepted until the
pipeline reproduction is recorded in BUG_LEDGER.**

| ID | Format/condition | Required behavior | Test cases | Evidence expected | Reviewer | Gate |
|---|---|---|---|---|---|---|
| FT-P01 | TXT (ASCII/UTF-8/BOM/BOM-less) | actual lines, correct snippet around match | P01-01..08 | unit fixture + screenshot | L1-A | G2 |
| FT-P02 | Source code | syntax/text, match highlight, line/column jump | P02-01..10 | screenshot + IPC payload | L1-A | G2 |
| FT-P03 | Markdown/HTML | sanitized render, no script execution, text/metadata | P03-01..06 | security test + screenshot | L1-A/L1-E | G2 |
| FT-P04 | JSON/CSV | bounded formatting, no huge payload, metadata | P04-01..05 | unit + screenshot | L1-A | G2 |
| FT-P05 | PDF (selectable text) | page text + snippet + page number | P05-01..06 | fixture + E2E | L1-A | G2 |
| FT-P06 | PDF (scanned) | queued OCR once, progress, no keystroke re-OCR | P06-01..06 | OCR bench + E2E | L1-A/L2-A | G2 |
| FT-P07 | DOCX/Office | safe parse, excerpt, no macros/temp leaks | P07-01..06 | fixture + E2E | L1-A | G2 |
| FT-P08 | Images | real thumbnail + metadata (+ OCR snippet if indexed) | P08-01..06 | screenshot | L2-A | G2 |
| FT-P09 | Binary/unsupported/encrypted | explicit unsupported/error fallback, never blank | P09-01..06 | unit + screenshot | L1-A | G2 |
| FT-P10 | Unicode/Hindi/CJK/emoji | correct decode/render/highlight | P10-01..08 | fixtures + screenshot | L1-A | G2 |
| FT-P11 | 0-byte/1MB/100MB/locked | bounded memory, timeout/progress, honest failure | P11-01..08 | perf trace | L1-E | G2 |
| FT-P12 | Renamed/moved/deleted while selected | stale response guard, invalidation, error state | P12-01..06 | integration test | L1-A | G2 |
| FT-P13 | Both launchers presentation | same data, correct layout/density | P13-01..04 | screenshots | L1-B | G8 |

## D. Dynamic Island cards (V9) — G5

| ID | Card | Status before v10 | Required behavior | Test cases | Evidence | Reviewer | Gate |
|---|---|---|---|---|---|---|---|
| FT-I01 | Search card | NOT STARTED | real search capsule/results/filters/preview from shared core | I01-01..15 | screenshots + E2E | L1-B | G5 |
| FT-I02 | Media card | NOT STARTED | GSMTC session, metadata/artwork, transport, output device, honest capability | I02-01..12 | native E2E + screenshot | L1-A/L1-B | G5/G7 |
| FT-I03 | Clipboard card | NOT STARTED | real text/code/link/image/file rows, categories, pin/delete/copy, no giant SVG | I03-01..15 | E2E + screenshots | L2-A/L1-B | G5 |
| FT-I04 | Agents card | NOT STARTED | real Pi task state/hierarchy/logs/controls, honest pause/cancel semantics | I04-01..10 | E2E + screenshots | L1-D/L1-B | G5/G7 |
| FT-I05 | Quick Settings card | NOT STARTED | Wi-Fi/Bluetooth split primary+chevron drill-down, airplane/energy/hotspot/cast/night light, brightness/volume, capability gates | I05-01..20 | native E2E + screenshots | L1-A/L2-A/L1-B | G5/G7 |
| FT-I06 | Browser card | NOT STARTED | real bookmarks/history with source filter + open | I06-01..08 | E2E + screenshot | L2-A/L1-B | G5 |
| FT-I07 | Git card | NOT STARTED | commits/repo info + navigation | I07-01..08 | E2E + screenshot | L2-A/L1-B | G5 |
| FT-I08 | Notifications card | NOT STARTED | local notifications + supported sources, dismiss/clear/read, 0/1/many | I08-01..10 | E2E + screenshots | L1-A/L1-B | G5/G7 |
| FT-I09 | Pagination dots | NOT STARTED | 8 clickable dots, accessible names, active state, order sync, compact footer | I09-01..08 | screenshots + a11y test | L1-B | G5 |
| FT-I10 | Card switching motion | NOT STARTED | V9 width/height/corner/easing, wheel/swipe vs inner scroll, stale transition prevention, reduced motion | I10-01..12 | frame-time capture + video | L1-B/L1-C | G5/G8 |

## E. Settings / management app

| ID | Feature | Status before v10 | Required behavior | Test cases | Evidence | Reviewer | Gate |
|---|---|---|---|---|---|---|---|
| FT-SET01 | Home/greeting/onboarding | PARTIAL (v2 onboarding exists) | welcoming home, personalized greeting, primary launcher choice, no forced telemetry | SET01-01..08 | screenshots + E2E | L1-B | G6 |
| FT-SET02 | Indexing/source management | PARTIAL (v2 Indexes page exists) | roots/depth/exclusions, readiness per source, rebuild/repair, no fake global percent | SET02-01..10 | E2E + screenshot | L2-A | G6 |
| FT-SET03 | Appearance & motion | PARTIAL (v2 theme/density/motion exists) | light/dark/system, typography, reduced motion, V9 default preservation | SET03-01..08 | screenshots light/dark | L1-B | G6 |
| FT-SET04 | Launcher settings | PARTIAL (v2 hotkey/launch settings exist) | Normal vs Island, shortcuts, monitor strategy, width/height/density, quick destinations | SET04-01..10 | screenshots + E2E | L1-B | G6 |
| FT-SET05 | Island placement editor | NOT STARTED | graphical monitor(s), aspect/work area/taskbar, draggable V9 ghost, anchor presets, X/Y offsets, apply/cancel/reset, safe recovery, all 8 card shapes, keyboard + a11y | SET05-01..20 | screenshots + drag/keyboard E2E + persistence test | L1-B/L1-E | G6 |
| FT-SET06 | Card manager | NOT STARTED | ordered 8-card list, enable/pin/default, up/down, drag, live dots preview, reset/undo, persistence | SET06-01..15 | screenshots + E2E | L1-B | G6 |
| FT-SET07 | Clipboard management | PARTIAL (privacy gate exists; no full manager) | history view/search/filter/pin/delete/clear, retention, pause, secure wipe | SET07-01..12 | E2E | L2-A | G6/G7 |
| FT-SET08 | Agent management | PARTIAL (Workflows panel being finished) | runtime/provider config, permissions, approval policy, history, honest offline states | SET08-01..10 | E2E | L1-D | G7 |
| FT-SET09 | Permissions & privacy | PARTIAL (privacy toggles exist) | explicit consent, exclusions, no telemetry, clipboard/agent privacy | SET09-01..10 | E2E + screenshots | L1-E | G6 |
| FT-SET10 | Hotkeys / updates / troubleshooting | PARTIAL (hotkey + diagnostics exist; updater not implemented) | remap with conflict handling; signed updater or honest blocker; diagnostics export with redaction | SET10-01..08 | E2E | L1-E | G9 |
| FT-SET11 | Settings migration | PARTIAL (v2 settings migration exists) | versioned schema, atomic writes, rollback, migration tests, defaults preserved | SET11-01..08 | migration fixtures | L1-E | G6/G9 |

## F. Native Windows integrations (capability-gated)

| ID | Function | Status before v10 | Required behavior | Test cases | Evidence | Reviewer | Gate |
|---|---|---|---|---|---|---|---|
| FT-N01 | Media session / transport | NOT STARTED | real GSMTC sessions, metadata, supported actions only | N01-01..12 | native E2E | L1-A | G7 |
| FT-N02 | Audio output devices | NOT STARTED | enumerate/switch endpoints, honest unsupported | N02-01..06 | native E2E | L1-A | G7 |
| FT-N03 | Wi-Fi | NOT STARTED | adapters/networks/connect where supported; system-settings fallback | N03-01..08 | native E2E + screenshots | L1-A/L2-A | G7 |
| FT-N04 | Bluetooth | NOT STARTED | paired/discoverable devices, connect where exposed, pairing rights | N04-01..08 | native E2E | L1-A | G7 |
| FT-N05 | Airplane/energy/hotspot/cast/night light | NOT STARTED | identify actual supported API path; disabled/redirect otherwise | N05-01..10 | capability matrix test | L1-A | G7 |
| FT-N06 | Brightness/volume | NOT STARTED | per-monitor capability, sliders, hardware limits | N06-01..08 | native E2E | L1-A | G7 |
| FT-N07 | Notifications | NOT STARTED | local notifications by default; supported sources only; dismiss/clear | N07-01..08 | E2E | L1-A | G7 |
| FT-N08 | Hotkey/global capture | PARTIAL (v2 hotkey works) | no trap, conflict detection, restore previous | N08-01..05 | E2E | L1-E | G4 |

## G. Release / reliability

| ID | Area | Status before v10 | Required behavior | Evidence | Reviewer | Gate |
|---|---|---|---|---|---|---|
| FT-R01 | Installer/uninstaller | PARTIAL (unsigned NSIS built in v2; install/uninstall not v10-tested) | install/upgrade/uninstall cleanup, data preserved by default, rollback | E2E installer logs + hashes | L1-E | G9 |
| FT-R02 | Data migration/backup | PARTIAL (v2 legacy import verified read-only) | pre-upgrade backup, per-user data preserved, clipboard images migrated, rollback | migration fixtures + real-data copy | L1-E | G9 |
| FT-R03 | Crash recovery | NOT VERIFIED | DB/journal recovery, stale state reset, no silent data loss | fault injection | L1-E | G8/G9 |
| FT-R04 | CI / deterministic tests | PARTIAL (CI skeleton; runs locally) | fmt/clippy/tests/frontend/migration, no secrets, realistic failures | CI logs | L1-E | G8/G9 |
| FT-R05 | Security | PARTIAL (legacy risk register + v2/Pi reviews; S-01/S-02/RR-02 open) | secret rotation, signed update, agent consent, clipboard privacy, no telemetry | audit docs + scans | L1-E | G8/G9 |

## H. Execution rule

Rows above are planned acceptance criteria. At Gate 0 only environment/repository/data work and
documentation are complete; every code-behavior row remains unverified until its owner produces the
required evidence and L1-E/L0 re-runs it. Do not quote a row as passing because it appears here.


## G8 round 1 executed evidence (2026-11-10, task-50)

Statuses are an honest evidence snapshot for the first G8 sweep; the plan rows above remain the
acceptance checklist. Full details: docs/implementation/V10_G8_VERIFICATION_ROUND1.md.

| Matrix ID | Round-1 status | Evidence (short) |
|---|---|---|
| FT-L02 Normal Launcher filters | PASS (implemented, 4 visible filters + prefix access; design-spec micro-spec sync pending) | live screenshots docs/qa/g8-verify/e2e-launcher/, launcher tests |
| FT-L03 Quick destinations | PASS (implemented, honest off states) | real-launcher-empty.png; addendum C2 |
| FT-L04 Dynamic Island mode | PARTIAL (renderer/cards/dots/motion; native adapters + agent hierarchy + pinning incomplete) | island audits + live-shots |
| FT-L05 Primary launcher scoping | PARTIAL (settings/onboarding implemented; live mode-switch E2E not executed) | task-40 evidence |
| FT-S01 Files/folders | PARTIAL (live filename search works; full source matrix not executed) | e2e-drive latency + 10 screenshots |
| FT-S03 Full-text content | PARTIAL (fixtures search/preview work; official p95 target unconfirmed) | perf JSON, preview evidence |
| FT-S10 Clipboard text/categories | PARTIAL (image rows/categories visible live; text/pin/delete matrix not executed) | island Images tab + IPC rows |
| FT-S11 Clipboard images (P0-2) | PASS (live .NET capture, thumbnails, restart; paste-back UI + GUI matrix follow-ups) | g8 doc section 3.3 |
| FT-P01 TXT preview | PASS (API payload) | p0-1-preview-evidence.json |
| FT-P05 PDF text preview | PASS (API payload) | p0-1-preview-evidence.json |
| FT-P07 DOCX/Office preview | PASS (live launcher UI) | p0-1-preview-report.png |
| FT-P08 Image preview | PASS (payload + OCR text + highlight count; screenshot not re-captured) | p0-1-preview-evidence.json |
| FT-P09 Binary/unsupported | PASS (live launcher UI) | p0-1-preview-blob.png |
| FT-P02/P03/P04/P06/P10 | NOT EXECUTED this round | G8 sign-off item |
| FT-S12 Agents | BLOCKED/PARTIAL (no Pi credentials live; read-only alpha gate) | PI security review |


## G8 round 2 executed evidence on RC 4e9f589 (2026-11-10, task-53)

Full-binary suites are green on the RC tree (cargo workspace 306 passed / 0 failed; frontend 241
passed; see V10_G8_VERIFICATION_ROUND2.md). Live evidence on RC/dev-CDP build:

| Matrix ID | Round-2 status | Evidence |
|---|---|---|
| FT-P01 TXT preview | PASS (text/Unicode) | p0-1-round2 evidence |
| FT-P02 Source code | PASS (rs text preview) | p0-1-round2 evidence |
| FT-P03 Markdown/HTML | PARTIAL (MD text pass; HTML not executed) | p0-1-round2 evidence |
| FT-P04 JSON/CSV | PASS (JSON + CSV text preview) | p0-1-round2 evidence |
| FT-P05 PDF text | PASS (text layer) | p0-1-round2 evidence |
| FT-P06 PDF scanned | PARTIAL - honest unsupported "no text layer; OCR preview not available" | p0-1-round2 evidence |
| FT-P07 DOCX/Office | PASS (DOCX + XLSX text preview; UI screenshot) | p0-1-docx.png |
| FT-P08 Images | PASS (thumbnail data URL 26,746 chars + OCR 45 chars; UI screenshot) | p0-1-image.png |
| FT-P09 Binary/unsupported/encrypted | PASS (bin/xyz explicit unsupported; stale file explicit error) | p0-1 evidence + stale test |
| FT-P10 Unicode/highlight | PARTIAL/FAIL - Unicode text preserved but matched-term highlightRanges empty and UI marks 0 (condition open) | p0-1-token-evidence.json |
| FT-S11 Clipboard images | PASS (live .NET capture, thumbnail, restart persistence) | p0-2 evidence |
| FT-L02/L03 | PASS (live launcher filters/quick destinations) | e2e/launcher tests |
| FT-L04/L05, FT-S01/S03/S10/S12 | PARTIAL/blocked as in round 1 (native adapters, no Pi creds, full parity pending) | round-1 + gates |
