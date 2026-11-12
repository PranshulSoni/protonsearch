# Gate 0 — Legacy v1.1.1 Baseline Audit

**Project:** ProtonSearch v2 "Project Phoenix"
**Branch:** `v2-phoenix` (HEAD `de0ef19`) == tag `v1.1.1-v2-baseline` (`git rev-parse` both = `de0ef198de9ed54f5e04d52135bdd747346dbd47`)
**Audit owner:** L1-D (rel-recon) / **Date:** 2026-11-09 / **Host:** user workstation (Win 11)

> Scope: read-only audit of the *legacy* app in `protonsearch/`. Nothing under `protonsearch/` was modified. The legacy GUI was **not** launched (workspace rule). Runtime behaviour claims that require the GUI are marked **UNVERIFIED**.

---

## 1. Environment (recorded for reproducibility)

| Item | Value | How obtained |
|---|---|---|
| OS | Microsoft Windows 11 Home Single Language, 10.0.**26300**, 64-bit | `Get-CimInstance Win32_OperatingSystem` |
| CPU | AMD Ryzen AI 9 465 w/ Radeon 880M — 10 cores / 20 logical | `Get-CimInstance Win32_Processor` |
| RAM | 31.2 GB | `Win32_ComputerSystem` |
| System drive | C: 439.3 GB used / 348.0 GB free (at audit time) | `Get-PSDrive C` |
| Rust | `rustc 1.99.0 (b940084d7 2026-09-28)` | `rustc --version` |
| Cargo | `cargo 1.99.0 (5f94df478 2026-08-27)` | `cargo --version` |
| Node | `v26.7.0` | `node --version` |
| Python | `3.14.7` | `python --version` |
| SQLite CLI | `3.53.4` (linked into rusqlite as bundled, independent of CLI) | `sqlite3 --version` |
| Host shell used by tooling | Windows PowerShell 5.1.26100.9818 (the `pwsh` tool spawns powershell.exe) | `$PSVersionTable.PSVersion` |
| git | `C:\Program Files\Git\cmd\git.exe` | `Get-Command git` |
| hermes | `C:\Users\Pranshul\AppData\Local\hermes\bin\hermes.exe` (present) | `Get-Command hermes` |

---

## 2. Build & test status (legacy crate `protonsearch/`)

### 2.1 Exact commands & results

```powershell
# from protonsearch/
cmd /c "cargo test --no-fail-fast > ..\.agents\notes\legacy-tests-clean.log 2>&1"
# EXIT_CODE=0
```

```text
test result: ok. 85 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s   (bin inspect-db)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s   (bin uninstall)
```

- Clean rerun: `.agents/notes/legacy-tests-clean.log` (11,850 B), exit code **0**, 3.25 s test time.
- Original baseline run: `.agents/baseline-old-tests.log` — identical result (85 passed / 0 failed / 1 ignored, 2.86 s test time, 57.51 s compile).
- Debug build (test profile) compiles in ~13 s warm / 57.5 s cold. The crate compiles clean apart from warnings; **no warnings-as-errors gate exists**.
- The ignored test is `search::tests::test_search_file` — comment in the test log says *"integration test: requires a real %APPDATA% index DB and the ONNX model"*. This is the only end-to-end-ish test and it is skipped.

### 2.2 Test distribution (86 functions, 85 run)

| File | `#[test]` count | Area |
|---|---:|---|
| `src/main.rs` | 41 | UI geometry, clipboard helpers, agent memory, icon caches, filters |
| `src/search.rs` | 24 | filters, memory events, clipboard/OCR matching, quick actions, catalog index |
| `src/hotkey.rs` | 6 | hotkey parsing/validation/round-trip |
| `src/markdown.rs` | 5 | markdown parser |
| `src/ai.rs` | 3 | key fallback, config resolution, HTML→text |
| `src/browser_indexer.rs` | 2 | browser timestamp normalisation, memory events |
| `src/indexer.rs` | 2 | ignored-folder normalisation |
| `src/settings.rs` | 2 | defaults |
| `src/settings_startup.rs` | 1 | startup command quoting |
| `src/git_indexer.rs`, `launcher.rs`, `settings_ui.rs`, `circle_to_search.rs`, `uninstall.rs`, `inspect_db.rs`, `applog.rs` | 0 | **no unit tests** |

Counts: `(Get-Content <file> -Raw | Select-String '#\[test\]')`; per-file test names in `.agents/baseline-old-tests.log`.

### 2.3 Compiler warnings (evidence of debt; not failures)

From the clean test log (15 warnings in the `ProtonSearch` bin target, 1 in `inspect-db`):

- `unnecessary unsafe block` — `src/inspect_db.rs:25`.
- `#![windows_subsystem]` attribute only allowed at crate root — `src/uninstall.rs:1` (silently ignored).
- Deprecated `PngEncoder::encode` — `src/circle_to_search.rs:347`.
- Dead code: `LocalAgentAction::OpenUrl` (`src/main.rs:4475`), `SearchEngine` fields `vecs`/`dim`/`anchor_categories` and struct `AnchorCategory` (`src/search.rs:605-640`), `search_local_files` (`src/search.rs:1464`), `get_conversational_results` (`src/search.rs:5313`), `mean_pool_norm` (`src/search.rs:5633`), all of `src/uninstall.rs`'s helpers + `main` when compiled as a test.
- Slint warnings: `ui/settings.slint:3` component doesn't inherit `Window`; padding on non-layout elements at `ui/settings.slint:1100`, `:1454`.

**Meaning:** the vector/embedding ranking path in `search.rs` is dead code (never called); the only live embedding use is the settings-catalog binary blob consumed in `SearchEngine::new` (`src/search.rs:651-682`). The README makes no semantic-search claim, so this is debt, not a false claim.

### 2.4 Binary / bundle sizes present on this host

| Artefact | Bytes | Notes |
|---|---:|---|
| `protonsearch/target/debug/ProtonSearch.exe` | 60,466,176 | debug build (may be stale: 2026-11-03) |
| `protonsearch/target/stages/baseline-2026-10-31/ProtonSearch.exe` | 64,763,904 | staged baseline binary, 2026-10-24 |
| release build (`target/release/ProtonSearch.exe`) | — | **absent** — never built during this audit |
| installer / MSIX / setup EXE | — | **absent** — `installer.iss` exists but no `setup/` output |

`protonsearch/Cargo.toml` is `[profile.release] opt-level=3, lto=true, codegen-units=1, strip=true` (`Cargo.toml:74-78`), so a release size was not measurable without a build.

### 2.5 Version/bookkeeping inconsistencies (baseline defects)

| Fact | Evidence |
|---|---|
| Crate version `1.1.0` | `protonsearch/Cargo.toml:3` |
| Tag is `v1.1.1-v2-baseline` but tree version is 1.1.0 | `git show v1.1.1-v2-baseline:protonsearch/Cargo.toml` |
| `installer.iss` AppVersion `1.1.0` | `protonsearch/installer.iss:3` |
| `update.json` version `1.1.0`, url points to `v1.1.0/protonsearchsetup.exe` | `update.json` (repo root) |
| winget manifests still `PranshulSoni.OmniSearch` `1.0.0` | `packaging/winget/.../OmniSearch/1.0.0/` |

The "v1.1.1" baseline is therefore a *content* tag, not a released binary version. v2 release numbering must not inherit this ambiguity.

### 2.6 Run status

**UNVERIFIED.** The legacy GUI was not launched (workspace rule: never run the launcher as a background process; user launches manually). No runtime smoke evidence exists for this audit. What *is* verified: the binary links and the test suite runs on this host; no window/hotkey/indexer end-to-end run was performed.

---

## 3. SQLite forensic summary (read-only)

**DB:** `C:\Users\Pranshul\AppData\Roaming\protonsearch\file_index.db`
- Size **149,372,928 B** (~142.4 MiB); SHA-256 `58249F011A045E74F02582F50722C80699E28BD6EDBE99073F567D386F361809` (`Get-FileHash`).
- Sidecars: `file_index.db-wal` **23,788,912 B**, `file_index.db-shm` 32,768 B (last written 2026-11-08).
- `PRAGMA quick_check` → `ok`; `journal_mode=wal`; page_size 4096; page_count 36,468; freelist 84. Schema user_version 0.
- Read-only access: `file:...?mode=ro&immutable=1` for schema dump, plain `mode=ro` (WAL-aware) for counts. Extraction scripts: `.agents/dump_schema.py` (L0) and `.agents/notes/forensic_dump.py` (this audit). No writes, no VACUUM, no checkpoint.

### 3.1 Tables and row counts (verified)

| Table | Rows | Notes |
|---|---:|---|
| `files` | **89,813** | 30,287 dirs, 59,526 files; 39,438 have empty extension (dirs / extensionless); 58,160 have size>0 |
| `files_fts` (FTS5) | **14,609** | content docs (`files_fts_content` also 14,609) |
| `browser_items` | **4,523** | brave_history 2,216; firefox_history 1,971; chrome_history 249; edge_bookmark 70; brave_bookmark 11; firefox_bookmark 5; chrome_bookmark 1 |
| `memory_events` | **5,286** | Browser/Visited Page 4,667; Git/Commit 619 |
| `git_repos` / `git_commits` / `git_branches` / `git_todos` | 15 / 516 / 23 / 132 | |
| `settings_catalog_fts` | **238** | SETTINGS 173; CONTROL 65 |
| `ai_settings` | **2** | keys `endpoint` (value len 43, URL) and `model` (len 22); **no `api_key` row** |
| `quicklinks` / `snippets` / `focus_categories` / `ignored_folders` | 4 / 1 / 1 / 2 | names: GitHub, Google, Rust Docs, YouTube / "Example Snippet" / "Deep Work" / node_modules, .angular |
| `clipboard_history` | **0** | schema includes `ocr_text TEXT` |
| `timeline_events` | **0** | schema exists; tracker code exists (see §4) |
| `indexer_state` | 2 | `is_indexing=1`, `progress=Scanning: 2000 files processed...` — **stale unclean-shutdown marker** |

Top file extensions: empty 39,438; `pyi` 7,528; `png` 6,533; `dll` 4,819; `cc` 3,356; `ts` 1,973; `md` 1,775; `py` 1,270; `pdf` 923 (full list in `.agents/notes/forensic_dump.py` output).

### 3.2 Schema notes relevant to migration

- `files(path PK, name, extension, modified, size, is_dir, last_seen_scan)` — path-keyed, **no file-id/inode**; renames lose identity (v2 must not copy this).
- `browser_items(url PK, title, source, visit_count, last_visit_time)` — capped to newest 5,000 history rows by `browser_indexer.rs:88-93`.
- `clipboard_history(id, content UNIQUE, timestamp, source_app, is_image, pinned, ocr_text)` — text/image path stored in `content`, OCR text separately.
- `memory_events(timestamp, source, event_type, title, detail, app_name, path, url)` + FTS5 content-sync table `memory_events_fts`.
- `timeline_events(timestamp, duration, app_name, window_title)` — separate from memory_events; empty here.
- AI config is a generic key/value table (see security register).
- `files_fts` is FTS5 with `path UNINDEXED, content`; orphan risk if files deleted outside the indexer (no triggers).
- No tables for: notes (files on disk), agents/agent chats (created lazily by `main.rs`), focus session state, steam games (live scan), open windows (live enumeration).

### 3.3 User data inventory under `%APPDATA%\protonsearch`

| Path | Size | Last write | Meaning |
|---|---:|---|---|
| `file_index.db` | 149,372,928 | 2026-11-08 | main index/DB (see above) |
| `file_index.db-wal` | 23,788,912 | 2026-11-08 | un-checkpointed WAL — migration must read via SQLite, not file copy of the .db alone |
| `file_index.db-shm` | 32,768 | 2026-11-08 | shared-memory index |
| `indexer.log` | 1,063,069 | 2026-11-08 | indexer diagnostics (paths/errors; no rotation except 1 MB truncation) |
| `git_indexer.log` | 27,073 | 2026-11-08 | git indexer diagnostics |
| `protondock-layout.json` (+ `.bak`) | 1,602 / 1,603 | 2026-11-08 | launcher/dock layout (protondock integration) |
| `notes/` | (dir) | 2026-11-07 | **empty** — no notes stored |
| `index.db` | **0** | 2026-11-07 | stale zero-byte file; not the real DB |
| `hermes_gateway.log` | absent | — | would be created by hermes gateway code |
| `panic.log`, `app_log.txt`, `clipboard_images/` | absent | — | created on demand |

No `settings.json` exists in AppData or beside the exe: settings are stored **in the exe directory** by preference (`settings.rs:192-224`) and no settings file was found next to the audited binary — i.e. defaults were in effect.

### 3.4 Read-only query latency (DB-only proxy, **not** UI latency)

20 iterations each against the WAL-aware read-only connection (`.agents/notes/db_bench.py`):

| Query | median | p95 |
|---|---:|---:|
| `files.name LIKE '%proton%'` | 6.56 ms | 6.93 ms |
| `files.name LIKE 'proton%'` | 4.56 ms | 4.70 ms |
| content FTS `files_fts MATCH 'snapshot'` | 0.03 ms | 0.07 ms |
| settings FTS `settings_catalog_fts MATCH 'night*'` | 0.02 ms | 0.03 ms |

This is SQLite time only; it does not include app startup, icon loading, ranking, or paint. The legacy code already maintains an in-memory filename index (`search.rs:643-645, 903-905, 996`) — the DB numbers are what that index avoids.

---

## 4. What is verified working vs claimed-only vs broken

Legend: **VERIFIED-CODE+TEST** (code path + unit test), **VERIFIED-CODE+DB** (code path + real data in the baseline DB), **CODE-ONLY** (code path, untested / no runtime evidence), **EMPTY/UNVERIFIED** (feature exists but no data or no run), **BROKEN** (defect found), **NOT-PRESENT** (claim/capability has no implementation).

| Feature | Status | Evidence |
|---|---|---|
| Global hotkey launcher | VERIFIED-CODE+TEST | `hotkey.rs:92-113` RegisterHotKey; handler `main.rs:1829`; 6 tests `hotkey.rs:314-399` |
| Frameless Win32 launcher UI | VERIFIED-CODE (no UI test) | `main.rs:1193-1200` `CreateWindowExW` `WS_POPUP`; painting `main.rs:7747`; 41 geometry/state unit tests in `main.rs` |
| Installed app launch (incl. UWP) | VERIFIED-CODE+TEST | `search.rs:6887-6902` IShellItem/AppsFolder enumeration; `main.rs:7327-7347` icons; test `test_enumerate_apps_folder` |
| File/folder filename index | VERIFIED-CODE+DB | `indexer.rs:210-260` writes `files`; in-memory index `search.rs:996`; 89,813 rows |
| Document content search (PDF/DOCX/text/code) | VERIFIED-CODE+DB+TEST | `indexer.rs:43-55,1008-1104` extraction (subprocess-isolated, 30 s watchdog); `files_fts` 14,609 docs; `search.rs:3442 search_with_fts` |
| Calculator / unit conversion | CODE-ONLY (no dedicated test found) | `search.rs:9113 try_calc`, `9957 try_unit_convert`; result source `CALC` |
| Native commands (lock/sleep/shutdown/restart/empty bin) | VERIFIED-CODE+TEST | `launcher.rs:216-690`; e.g. shutdown `launcher.rs:437`; test `quick_actions_expose_native_windows_shortcuts` |
| Clipboard history (text/images/pin/multi-select) | CODE-ONLY + EMPTY | Listener `main.rs:1291` AddClipboardFormatListener, handler `main.rs:1937-1999`; search `search.rs:2460`; **0 rows in DB**; Windows history import exists `main.rs:11704-11826`; many unit tests but no end-to-end proof on this host |
| Browser history/bookmarks | VERIFIED-CODE+DB+TEST | `browser_indexer.rs:16-96`; Chromium copy to temp `:245-322`, Firefox `:324-400`; 4,523 rows; 2 tests |
| Recent files | VERIFIED-CODE | `search.rs:6743 scan_recent_files` |
| Git repos/commits/TODOs | VERIFIED-CODE+DB (no tests) | `git_indexer.rs:69-95` 15-min loop, repo discovery `:11-49`, git CLI `:275-323`; 15/516/132 rows |
| Screenshot/clipboard OCR | VERIFIED-CODE+TEST + EMPTY | WinRT OCR `indexer.rs:1174-1290`; clipboard OCR `main.rs:11458`; OCR matching tests in log (`test_winrt_clipboard`, `clipboard_ocr_search_matches_compacted_text`, `run_clipboard_ocr_returns_job_result`); no clipboard rows in DB |
| Activity timeline | BROKEN (timeline) / WORKING (memory) | `timeline_events` **0 rows** while tracker `main.rs:11915-12036` writes it; `memory_events` 5,286 rows is what search reads (`search.rs:1493-1630,1711`) |
| Settings / Control Panel catalog | VERIFIED-CODE+DB+TEST | `search.rs:109 ensure_settings_catalog_fts`, `:911-956` search; 238 rows; test `settings_fts_returns_search_settings_in_fast_path` |
| Quicklinks & snippets | VERIFIED-CODE+DB | tables `search.rs:776-820`; search `search.rs:10591-10649`, keyword triggers `search.rs:10271-10301`; 4 / 1 rows |
| Web search / Google Lens | CODE-ONLY (0 tests) | `circle_to_search.rs:410-448` builds temp HTML and POSTs screenshot to lens.google.com; `main.rs:6537` web search action |
| Window controls (snap/move/resize/virtual desktops) | CODE-ONLY | `launcher.rs:752-830` (winvd), snap catalog `search.rs:8450-8545`; 0 tests |
| AI chat / Hermes agent | CODE-ONLY (external dependency) | `ai.rs` (~1,496 lines): ureq OpenAI-compatible client, Hermes gateway `ai.rs:548-622`, runs/approval `ai.rs:816-1070`; config in `ai_settings`; hermes binary present on host but **no AI key row** — AI path would ask for a key |
| Voice input | **NOT-PRESENT** | Cargo enables `Media_SpeechRecognition` (`Cargo.toml:48`) but `Select-String 'SpeechRecognizer|Media_Speech|voice'` over `src/*.rs` returns **no implementation**; README does not claim it |
| Notes (README claim) | CODE-ONLY + EMPTY | editor `main.rs:13041-13068`, notes dir `main.rs:13016-13021`; search `search.rs:10650`; AppData `notes/` is empty |
| Steam games (README claim) | **BROKEN/PARTIAL** | `search.rs:10481-10545` hardcodes only `C:\Program Files (x86)\Steam\steamapps`, `D:\SteamLibrary\steamapps`, `E:\SteamLibrary\steamapps`; does **not** parse `libraryfolders.vdf`; no DB table; 0 tests |
| Open windows (`switch:`/`window:`) | VERIFIED-CODE | `search.rs:10162-10269` EnumWindows + fuzzy title match |
| Focus categories | VERIFIED-CODE+DB | `search.rs:10547-10589`, schema `:822-841` (seeds one); launcher enforcement `launcher.rs:335-380` (loops `taskkill` every 3 s); 1 row |
| Settings app (Slint) | CODE-ONLY (UI) | `settings_ui.rs:120 run_settings_window`; 2,232-line `ui/settings.slint` (135 KB); separate process mode `main.rs:690-701` |
| Update checker | CODE-ONLY, **INSECURE** | `settings_ui.rs:669-900`; fetches `update.json` from raw.githubusercontent, no signature/checksum (see security register) |
| Uninstaller / migration | CODE-ONLY, destructive | `uninstall.rs:88-227`; `main.rs:733-742` rename-migration OmniSearch→ProtonSearch |
| Tray icon | CODE-ONLY | `main.rs:133-166` Shell_NotifyIcon, setup `main.rs:1209` |

### 4.1 Claims in README/.agents notes that are **not backed** by code+tests
- "Clipboard history … pin, multi-select, paste" — code and many unit tests exist, but baseline DB has **0 rows** and no end-to-end test; cannot be called verified-working on this host.
- "Notes" / "Steam games" — see table; notes has no data, Steam indexing is hardcoded-path only.
- "Activity timeline" — `timeline_events` empty; only `memory_events` works.
- "Hermes agent … autonomous tasks, execute approved commands" — code exists, but needs an external `hermes` binary + configured AI key; the DB has no key row. Not exercised in this audit.
- "Upgrading from OmniSearch … carry over automatically" — migration only renames `%APPDATA%\\omnisearch` → `protonsearch` **if the new dir does not exist** (`main.rs:738-739`); if a new empty dir exists first, data is not moved. Data-loss edge case.
- README performance claims (sub-millisecond FTS etc.) are consistent with DB-side numbers here but are **UNVERIFIED** end-to-end.

### 4.2 Broken / risky areas to fix-by-reimplementation (file:line)
1. **Updater trust**: no signature/checksum before running downloaded EXE — `settings_ui.rs:677-900`, `update.json`.
2. **Wi-Fi password action is non-functional / dangerous**: literal `PROFILE` placeholder in the netsh command — `launcher.rs:643-651`; catalog entry `search.rs:8349-8358` tells users it shows saved Wi-Fi passwords.
3. **Stale indexer state**: `is_indexing=1` left behind; no crash-recovery reset (`indexer.rs:632-647`, `settings_ui.rs:938-...`). After migration, v2 must treat the legacy DB as read-only and not trust this flag.
4. **Timeline tracker writes nothing**: 0 rows despite code — likely killed before first flush or silently failing; not migrated as "working".
5. **Clipboard history empty**: no runtime data to migrate; migration must handle zero-row table gracefully.
6. **Dead semantic-search code** (warnings above) — do not port.
7. **Version metadata drift** (Cargo 1.1.0 vs tag v1.1.1 vs update.json 1.1.0 vs winget 1.0.0).
8. **Uninstaller deletes `%APPDATA%\\protonsearch`** including the legacy index — `installer.iss:32-33`; `uninstall.rs:170-179`. v2 must never run the legacy uninstaller against v2 data; migration needs a rollback/export.
9. **Test fixtures left untracked** in the repo working dir: `protonsearch/test_db.db`, `test_ai_prefixes.db` (+ WAL/SHM) — produced by tests, gitignored (`.gitignore` `*.db`), but present; not source of truth.
10. **Single-instance mutex named `Local\\ProtonSearchInstanceMutex`** (session-local) — fine, but v2 must use its own name to avoid killing/attaching to legacy instance (`main.rs:646`).

---

## 5. Gate-0 exit criteria status

| Criterion | Status | Evidence |
|---|---|---|
| Baseline commit/tag identified; legacy tree unmodified | **MET** | HEAD == tag `v1.1.1-v2-baseline`; audit was read-only (`git status` shows no legacy modifications) |
| Legacy build compiles | **MET** | `cargo test` compile clean (warnings only), exit 0 |
| Legacy test suite result recorded | **MET** | 85 passed / 0 failed / 1 ignored, exit 0; logs in `.agents/notes/legacy-tests-clean.log` |
| DB forensic dump (tables/row counts) | **MET** | §3; scripts `.agents/dump_schema.py`, `.agents/notes/forensic_dump.py` |
| Feature parity matrix rows verified against source | **MET** (rows 1-19) + new rows added | `docs/implementation/FEATURE_PARITY_MATRIX.md` appendix |
| Security risk register with evidence | **MET** | `docs/security/SECURITY_RISK_REGISTER.md` |
| Competitive research with cited URLs | **MET** | `docs/research/RESEARCH.md` |
| CI skeleton for v2 workspace | **MET** | `.github/workflows/ci.yml` (not yet executed on GitHub; local `cargo fmt --check` currently fails on the WIP scaffold — expected) |
| Runtime smoke test of legacy app | **NOT DONE / UNVERIFIED** | GUI launch deliberately not performed; no screenshot/E2E evidence exists for this baseline |

**Gate-0 verdict: baseline evidence is sufficient to proceed to G1, with the explicit caveat that legacy *runtime* behaviour was not exercised on this host.** All "working" claims above are code/test/DB evidence, not UI evidence.

---

## 6. Reproduction commands (appendix)

```powershell
# repo state
git rev-parse HEAD; git rev-parse v1.1.1-v2-baseline; git status --short

# legacy tests (clean, exit code preserved)
cd protonsearch
cmd /c "cargo test --no-fail-fast > ..\.agents\notes\legacy-tests-clean.log 2>&1"; "EXIT_CODE=$LASTEXITCODE"
Select-String -Path ..\.agents\notes\legacy-tests-clean.log -Pattern 'test result:'

# DB forensics (read-only)
python .agents/dump_schema.py
python .agents/notes/forensic_dump.py
python .agents/notes/db_bench.py

# history scan for leaked keys (masked output only)
python .agents/notes/hist_key_scan2.py
```

*No secret values appear in this document. Any key material found is reported only as a masked prefix/length in the security register.*
