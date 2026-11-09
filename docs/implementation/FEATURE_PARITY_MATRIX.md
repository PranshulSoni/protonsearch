# ProtonSearch v2 — Feature Parity Matrix

Status: verified against legacy source by `rel-recon` at Gate 0 (commit `de0ef19`, tag `v1.1.1-v2-baseline`).
Every row below has file:line evidence in the "Legacy evidence" column and/or Appendix A. Rows marked **UNVERIFIED** require running the GUI on this host, which the Gate-0 rules forbade.
Status values: **LEGACY-WORKING** (verified by code + tests/DB), **LEGACY-PARTIAL** (code exists but data/feature incomplete), **LEGACY-BROKEN** (known defect), **NOT-PRESENT** (never existed), **PLANNED-V2** (v2 target, not started), **V2-WIP** (implemented/in progress, not E2E-accepted), **V2-PARTIAL** (subset implemented), **V2-DONE** (implemented and verified by code/test/measured evidence), **CUT** (with rationale).

**v2 status snapshot:** verified 2026-11-09 against commit 3906836/5f7522a plus working tree. V2-DONE requires a code path plus test/measured evidence; V2-WIP means code landed but no E2E acceptance yet. Evidence index in Appendix B.

| # | Capability | Legacy state (v1.1.1) | Legacy evidence (file:line) | v2 target | v2 status |
|---|---|---|---|---|---|
| 1 | Global hotkey launcher (Alt+Space) | **LEGACY-WORKING** (RegisterHotKey + validation; unit tests) | `hotkey.rs:92-113,314-399`; `main.rs:1439-1449,1829` | Preserved + configurable | V2-DONE |
| 2 | Frameless quick launcher UI | **LEGACY-WORKING** (custom Win32 GDI paint; UI runtime UNVERIFIED) | `main.rs:1193-1200` (WS_POPUP), `main.rs:7747` (paint), 41 geometry tests in `main.rs` | Redesigned per reference HTML (Tauri/React) | V2-WIP |
| 3 | Installed app search + launch (desktop + UWP/AppsFolder) | **LEGACY-WORKING** (code + test; DB-independent) | `search.rs:6887-7014` (SHGetKnownFolderItem AppsFolder `:6901-6902`), `main.rs:7327-7478` (icons); test `test_enumerate_apps_folder` | Preserved; icons first-class | V2-WIP |
| 4 | File/folder filename search | **LEGACY-WORKING** (SQLite `files` 89,813 rows + in-memory `FileRow` index) | `indexer.rs:210-260`; `search.rs:996 build_file_index`; DB §Gate-0 | In-memory compact index, sub-10ms warm | V2-DONE |
| 5 | Document content search (PDF/DOCX/text/code) | **LEGACY-WORKING** (FTS5, 14,609 docs; extraction subprocess-isolated) | `indexer.rs:43-55,744,1008-1104`; `search.rs:3442 search_with_fts` | FTS5 preserved + offsets for open-at-match | V2-WIP |
| 6 | Calculator / unit conversion | **LEGACY-WORKING** (in-process parser; no dedicated unit test found) | `search.rs:9113 try_calc`, `:9957 try_unit_convert` | Preserved | V2-DONE |
| 7 | Native commands (lock/sleep/shutdown/restart/empty bin) | **LEGACY-WORKING** (unit test for native shortcuts) | `launcher.rs:216-690` (shutdown `:437`, lock `:498`); test `quick_actions_expose_native_windows_shortcuts` | Preserved with confirmation for destructive | V2-DONE |
| 8 | Clipboard history (text/images, pin, multi-select, paste) | **LEGACY-PARTIAL / end-to-end UNVERIFIED** — code + many unit tests, but **0 rows** in baseline DB | `main.rs:1291,1937-1999,11370-11450`; `search.rs:2460-2490`; DB `clipboard_history` 0 | Preserved + rebuilt listener | V2-WIP |
| 9 | Browser history + bookmarks (Chromium/Firefox) | **LEGACY-WORKING** (4,523 rows; 2 unit tests) | `browser_indexer.rs:16-96,245-322,324-400`; DB by source §Gate-0 | Preserved + prefix scopes | V2-WIP |
| 10 | Recent files | **LEGACY-WORKING** (code; no unit test) | `search.rs:6743 scan_recent_files` | Preserved | V2-WIP |
| 11 | Git repos/commits/TODOs (+ open at line) | **LEGACY-WORKING** (15 repos / 516 commits / 132 TODOs; **no unit tests**) | `git_indexer.rs:69-95,275-323`; `main.rs:5530` (todos scope) | Preserved | V2-WIP |
| 12 | Screenshot/clipboard OCR search | **LEGACY-PARTIAL** — WinRT OCR code + unit tests, but no clipboard data at baseline | `indexer.rs:1174-1290`; `main.rs:11458-11490`; `search.rs:231-290` (match rules); tests in baseline log | Bounded OCR pipeline + FTS, no per-query OCR | V2-WIP |
| 13 | Activity timeline / memory events | **LEGACY-WORKING for `memory_events`; LEGACY-BROKEN for `timeline_events` (0 rows)** | memory: `search.rs:1493-1630,1711`; tracker: `main.rs:11915-12036` | Activity page (privacy-scoped) | V2-WIP |
| 14 | Settings & system settings catalog search | **LEGACY-WORKING** (238 FTS rows; unit test) | `search.rs:109,911-956`; test `settings_fts_returns_search_settings_in_fast_path` | Preserved | V2-WIP |
| 15 | Quicklinks & snippets | **LEGACY-WORKING** (4 / 1 rows; keyword triggers) | `search.rs:776-820,10271-10301,10591-10649` | Preserved | V2-WIP |
| 16 | Web search / Google Lens circle-to-search | **LEGACY-PARTIAL** — code exists, 0 tests; uploads screenshot to Google Lens via temp HTML | `circle_to_search.rs:410-448`; `main.rs:6537` | Preserved (explicit user action only; add egress consent) | PLANNED-V2 |
| 17 | Window controls (snap/move/resize/virtual desktops via winvd) | **LEGACY-WORKING (code only, 0 tests)** | `launcher.rs:752-830`; snap catalog `search.rs:8450-8545` | Preserved subset with safe API | PLANNED-V2 |
| 18 | AI agent runtime (legacy Hermes; Pi in v2) | **LEGACY-PARTIAL / UNVERIFIED end-to-end** — Hermes client + gateway code; external hermes binary present on host, but baseline DB has **no api_key row** | ai.rs:139-264,345-480,548-622,703-1070; settings_ui.rs:1093-1163; DB ai_settings §Gate-0 | **Hermes dropped; Pi coding-agent CLI is the default agent runtime** (read-only tools by default, writes/shell opt-in; Pydantic sidecar kept as optional fallback) | V2-WIP | docs/security/PI_AGENT_SECURITY.md; crates/ps-agent/src/pi/**; docs/agent/PI_INTEGRATION.md |
| 19 | Voice input | **NOT-PRESENT** — Cargo feature `Media_SpeechRecognition` enabled (`Cargo.toml:48`) but no implementation in `src/` | grep SpeechRecognizer|Media_Speech|voice → no matches in `protonsearch/src/*.rs` | Defer/cut unless evidence + real test | CUT (recommended) |
| 20 | Notes (create/edit/search) | **LEGACY-PARTIAL** — code exists, `notes/` dir empty (0 notes) | `main.rs:13016-13068`; `search.rs:10650-10700` | Preserved | PLANNED-V2 |
| 21 | Steam library games | **LEGACY-BROKEN/PARTIAL** — only 3 hardcoded `steamapps` paths; no `libraryfolders.vdf` parsing; 0 tests | `search.rs:10481-10545` | Preserved or CUT; if kept, parse `libraryfolders.vdf` | PLANNED-V2 |
| 22 | Open windows search (`switch:`/`window:`) | **LEGACY-WORKING (code only, 0 tests)** — live `EnumWindows` + fuzzy title match | `search.rs:10162-10269` | Preserved | PLANNED-V2 |
| 23 | Focus categories / focus sessions | **LEGACY-WORKING (code + 1 DB row)**; enforcement force-kills every 3 s (security risk S-08) | `search.rs:822-841,10547-10589`; `launcher.rs:335-380` | Preserved with safe UX; no wildcard kills | PLANNED-V2 |
| 24 | Settings app (Slint window) | **LEGACY-WORKING (code only; runtime UNVERIFIED)** — 2,232-line `settings.slint`; same exe `--settings` mode | `settings_ui.rs:120`; `main.rs:690-701` | React settings pages in app shell | V2-WIP |
| 25 | Tray icon / background residency | **LEGACY-WORKING (code only)** — `Shell_NotifyIconW` add/remove | `main.rs:133-166,1209,3745` | Preserved | V2-DONE |
| 26 | Inline file/image preview (Tab "Quick Look") | **LEGACY-WORKING (code only, 0 tests)** — separate popup window class + paint | `main.rs:1176-1190,1592-1690,10177` | Preview pane + open-at-match (G5) | V2-WIP |
| 27 | Screen color picker | **LEGACY-WORKING (code only, 0 tests)** — magnifier overlay, copies hex | `main.rs:4293-4404`; catalog `search.rs:9030-9041` | Preserved (optional plugin) | PLANNED-V2 |
| 28 | Device/system actions (Bluetooth, Wi-Fi toggle, media keys, volume, ipconfig, eject) | **LEGACY-WORKING (code only, 0 tests)** — PowerShell/cmd/native media keys | `launcher.rs:556-668,1407-1600` | Preserved subset with confirmations | V2-PARTIAL |
| 29 | Process search + force kill | **LEGACY-WORKING (code only)** — `tasklist` parsing + protected-process blocklist; kill action | `search.rs:10072-10159` | Preserved with confirmation | PLANNED-V2 |
| 30 | In-app update checker/downloader | **LEGACY-BROKEN (insecure)** — fetches mutable `update.json`, runs unverified EXE; version metadata drift | `settings_ui.rs:669-900`; `update.json`; security register S-02/S-17 | Signed manifest + Authenticode + hash (G7) | PLANNED-V2 (must redesign) |
| 31 | Uninstaller + OmniSearch→ProtonSearch migration | **LEGACY-WORKING but destructive** — data dir deleted on uninstall; migration only if target dir absent | `uninstall.rs:88-227`; `main.rs:733-742`; `installer.iss:32-33` | Non-destructive migration + rollback/export | V2-WIP |
| 32 | AI quick commands (ask/explain/translate/summarize/grammar/find bugs) | **LEGACY-WORKING (code only, no unit tests for prompts)** — prefix routing to `ai::run` | `search.rs:4614-4670`; `main.rs:5550-5590`; `ai.rs:1240-1300` | Keep with explicit AI consent | PLANNED-V2 |
| 33 | Dev/ops entry points (`inspect-db`, `--extract-content`, `--settings`) | **LEGACY-WORKING (code only)** — 3 binaries / 2 special argv modes | `Cargo.toml:6-16`; `main.rs:684-701` | ps-bench CLI replaces inspect-db | V2-PARTIAL |
| 34 | Quicklinks/snippets import & export | **LEGACY-WORKING (code only)** — JSON files expected on Desktop | `main.rs:13081-13299` | Preserved | PLANNED-V2 |
| 35 | Clipboard multi-select / combined paste / bulk cleanup | **LEGACY-WORKING (code only; end-to-end UNVERIFIED)** — selection ids, paste/delete paths | `main.rs:2726-3150`; test `clipboard_selection_matches_pinned_and_unpinned_ids` | Preserved | PLANNED-V2 |
| 36 | Windows clipboard-history import | **LEGACY-WORKING (code only)** — WinRT `Clipboard::GetHistoryItemsAsync`, text+bitmap import | `main.rs:11704-11826` | **Do not port as-is**; opt-in only (privacy S-07) | V2-PARTIAL |
| 37 | AI provider browser prefix (`chatgpt:`) | **LEGACY-WORKING (code only)** — builds provider URL with encoded prompt | `search.rs:4286-4322` | Preserved (explicit action) | PLANNED-V2 |
| 38 | Settings persistence (exe-dir-first JSON, AppData fallback) | **LEGACY-WORKING (code only)** — portable-first; corrupt file kept as .bak | `settings.rs:192-268` | Replaced by v2 `v2/settings.json` | V2-DONE |
| 39 | Multi-monitor / DPI-aware placement | **LEGACY-WORKING (code + geometry tests)** — per-monitor-v2 DPI, cursor-monitor placement | `main.rs:3997-4220,711`; tests `launcher_width_honors_setting_and_small_screens` etc. | Preserved | V2-DONE |

**Runtime substitution note (2026-11-09):** legacy Hermes agent integration (row 18) is dropped in v2. The installed Pi coding-agent CLI is the default agent runtime; Pi runs on demand with a read-only tool allowlist by default and explicit opt-in for writes/shell, and the embedded Pydantic AI sidecar is kept as an optional fallback. Security review and release gates: docs/security/PI_AGENT_SECURITY.md.

## User data that must survive (verified at baseline)

- `file_index.db` (149,372,928 B, SHA-256 `58249F01…61809`): files 89,813; files_fts 14,609; browser_items 4,523; memory_events 5,286; git_* 15 repos / 516 commits / 23 branches / 132 TODOs; quicklinks 4; snippets 1; settings_catalog_fts 238 (173 SETTINGS + 65 CONTROL); ai_settings 2 (keys: `endpoint`, `model` — **no `api_key` row**); ignored_folders 2; indexer_state 2 (stale `is_indexing=1`); focus_categories 1; clipboard_history **0**; timeline_events **0**.
- `file_index.db-wal` 23,788,912 B + `-shm` 32,768 B: **un-checkpointed WAL**; migration must read through SQLite (a plain copy of `.db` alone loses recent writes).
- `protondock-layout.json` (+`.bak`), `indexer.log` (1.06 MB), `git_indexer.log` (27 KB), `notes/` (empty), stale zero-byte `index.db`.
- No `settings.json` found next to the audited binary → defaults were in effect.
- Migration rules: open legacy DB `mode=ro` / copy + WAL; never run the legacy uninstaller (`installer.iss:32-33` deletes `{userappdata}\protonsearch`); export a rollback snapshot before importing; treat `is_indexing=1` as stale; never migrate key material without explicit user consent + OS secret storage.

## README / docs claims NOT backed by code + tests (audit result)

| Claim (source) | Reality |
|---|---|
| "A real clipboard workflow …" (README) | Code + unit tests exist, but baseline DB has 0 rows; no end-to-end evidence. **LEGACY-PARTIAL** until exercised in a real session. |
| "Activity timeline — unified, searchable history" (README) | `timeline_events` has 0 rows; only `memory_events` works. |
| "Games — installed Steam library games" (README) | Only 3 hardcoded `steamapps` directories; misses other libraries; no `libraryfolders.vdf` handling. |
| "Notes — quick local notes" (README) | Editor + search code exists; baseline notes directory empty. |
| "Upgrading from OmniSearch … carry over automatically" (README) | Rename-only migration runs only if `%APPDATA%\protonsearch` does not already exist (`main.rs:738-739`); a pre-existing empty target blocks migration. |
| Hermes "execute approved commands … control your PC" (README) | Requires external `hermes` binary (present on this host) + configured AI key (absent in baseline DB); not exercised. |
| "Runs from the Windows system tray" (README) | `Shell_NotifyIconW` code exists; runtime UNVERIFIED (GUI not launched). |
| ".agents/AGENTS.md: git discovery in 186ms" | No harness/benchmark in repo; **UNVERIFIED**. |
| ".agents/AGENTS.md: window 720px / 76px rows" | Defaults verified in `settings.rs:152-157`; visual appearance UNVERIFIED. |

## Appendix A — Additional evidence details

- **Tests used as evidence** (all from .agents/notes/legacy-tests-clean.log, 85 passed / 1 ignored): `hotkey::tests::*` (6), `search::tests::*` (24), `tests::*` in `main.rs` (41), `markdown::tests::*` (5), `ai::tests::*` (3), `browser_indexer::tests::*` (2), `indexer::tests::*` (2), `settings::tests::*` (2), `settings_startup::tests::*` (1). Ignored: `search::tests::test_search_file` (needs real index DB + ONNX model).
- **No unit tests** in: `git_indexer.rs`, `launcher.rs`, `settings_ui.rs`, `circle_to_search.rs`, `uninstall.rs`, `inspect_db.rs`, `applog.rs`.
- **Dead/unused code** (compiler warnings): `SearchEngine::vecs`/`dim`/`anchor_categories`, `search_local_files`, `get_conversational_results`, `mean_pool_norm` (`search.rs:605-640,1464,5313,5633`) — semantic/vector path not reachable; do not port.
- **Version drift**: `Cargo.toml:3` = 1.1.0; tag `v1.1.1-v2-baseline`; `installer.iss:3` = 1.1.0; `update.json` = 1.1.0; winget manifests = OmniSearch 1.0.0.
- **Evidence sources**: `docs/audit/GATE0_BASELINE.md`, `docs/audit/LEGACY_ARCHITECTURE.md`, `docs/security/SECURITY_RISK_REGISTER.md`, .agents/notes/forensic_dump.py output, and the legacy test log. Runtime claims remain UNVERIFIED until the v2 E2E suite runs.

## Appendix B — v2 status evidence (snapshot 2026-11-09)

These are the code/test/measurement artefacts backing the **v2 status** column. Paths are
repo-relative; line numbers move as agents edit, so file-level references are used.

| # | V2 status | Evidence |
|---|---|---|
| 1 | V2-DONE | `apps/desktop/src-tauri/src/main.rs` global-shortcut plugin + set_hotkey (:412-428, :580); settings hotkey persisted in ps-ipc Settings |
| 2 | V2-WIP | `apps/desktop/src/launcher/LauncherSurface.tsx`, `LauncherView.tsx`; real-app screenshots in docs/qa/visual/app-captures |
| 3 | V2-WIP | `crates/ps-sources/src/apps.rs` (scan/scan_with_icons); engine tier0 match_apps |
| 4 | V2-DONE | `crates/ps-index/src/{scan,persist,matching}.rs`; engine scan_into_index; docs/performance/results/filename-*-3906836-*.json (10k/100k/1m) |
| 5 | V2-WIP | `crates/ps-content/src/{extract,index,legacy}.rs`; docs/performance/results/content-v2-pscontent-*.json (bench, not UI accept) |
| 6 | V2-DONE | `crates/ps-sources/src/calc.rs` (:19 evaluate, :53 convert, :75 evaluate_anything; cfg(test)); engine tier0 |
| 7 | V2-DONE | `crates/ps-sources/src/actions.rs:68 run_native`; `crates/ps-windows/src/imp/native.rs`; destructive confirm handshake engine.rs:677-707 |
| 8 | V2-WIP | `crates/ps-sources/src/clipboard.rs`; `crates/ps-windows/src/imp/clipboard.rs`; privacy.clipboard_history default false (ps-ipc:516); capture UI pending |
| 9 | V2-WIP | `crates/ps-sources/src/browser.rs` (Chromium/Firefox scan, cache); no E2E acceptance run |
| 10 | V2-WIP | `crates/ps-sources/src/recent.rs` |
| 11 | V2-WIP | `crates/ps-sources/src/git.rs` (discover/scan repo) |
| 12 | V2-WIP | `crates/ps-ocr/src/{lib,platform,store}.rs`; engine index_ocr; OCR benchmark JSON exists, E2E accept pending |
| 13 | V2-WIP | `crates/ps-core/src/db.rs` activity table; Tauri get_activity; Activity view WIP |
| 14 | V2-WIP | `crates/ps-core/src/catalog.rs`; engine catalog::match_actions |
| 15 | V2-WIP | `crates/ps-sources/src/quicklinks.rs` |
| 18 | V2-WIP | Pi backend: crates/ps-agent/src/pi/**; docs/agent/PI_INTEGRATION.md; security review docs/security/PI_AGENT_SECURITY.md (must-fixes PI-SEC-01..06 pending; Tauri wiring pending) |
| 24 | V2-WIP | `apps/desktop/src/app/**` (DesktopApp, PreferencesView, activity/search pages) |
| 25 | V2-DONE | `apps/desktop/src-tauri/src/main.rs:112 build_tray` + tray menu actions |
| 26 | V2-WIP | `apps/desktop/src/launcher/ResultPreview.tsx` |
| 28 | V2-PARTIAL | lock/sleep/shutdown/restart/empty-bin/settings/taskmgr/terminal/dark-mode only (`ps-sources/src/actions.rs:68`); no Bluetooth/Wi-Fi/media-key actions |
| 31 | V2-WIP | `crates/ps-core/src/migration.rs` + `scripts/build-installer.ps1` (unsigned; installer not built yet) |
| 33 | V2-PARTIAL | `crates/ps-bench/**` (dev CLI); inspect-db/--extract-content equivalents not needed |
| 36 | V2-PARTIAL | `ps-sources/src/clipboard.rs:229 import_windows_history` exists but is opt-in/not wired |
| 38 | V2-DONE | `crates/ps-core/src/settings.rs` (atomic save) + v2 settings.json under PROTONSEARCH_DATA_DIR/v2 |
| 39 | V2-DONE | `apps/desktop/src-tauri/src/main.rs:48 place_launcher` multi-monitor placement |

**Status honesty notes:** V2-DONE means implemented and verified by code/tests/measured artifacts,
not release-accepted (that is G7). V2-WIP rows have code in the tree but no E2E/acceptance
evidence yet; treat them as in-progress in the progress ledger.
