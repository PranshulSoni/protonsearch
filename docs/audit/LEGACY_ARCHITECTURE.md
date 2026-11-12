# Legacy Architecture — ProtonSearch v1.1.1 (read-only reference)

**Audience:** v2 implementers and reviewers. This is a map of the *existing* code, not a design proposal.
**Method:** static reading of `protonsearch/src/*.rs` + `protonsearch/ui/settings.slint`; test suite output; real SQLite DB. No GUI run.
**Rule:** the legacy app is evidence, not a template. Port behaviour and lessons, never the coupled code.

---

## 1. Scale and shape

| File | Lines | Bytes | Role |
|---|---:|---:|---|
| `src/main.rs` | 13,299 | 541,896 | entry, Win32 window/message loop, GDI painting, state, clipboard, icons, AI orchestration, preview, notes, imports/exports |
| `src/search.rs` | 10,701 | 427,956 | settings catalog data + embedding blob + `SearchEngine` + all search sources + calculator/unit conversion |
| `src/launcher.rs` | 1,626 | 62,584 | action dispatch: apps, URLs, shell/PowerShell system commands, window/virtual-desktop control |
| `src/ai.rs` | 1,496 | 54,861 | OpenAI-compatible client, key resolution, Hermes gateway lifecycle, runs/approvals |
| `src/indexer.rs` | 1,332 | 47,204 | file crawler, watcher, FTS writer, subprocess extraction, WinRT OCR |
| `src/settings_ui.rs` | 1,200 | 49,656 | Slint settings window, DB status polling, update checker/downloader |
| `src/circle_to_search.rs` | 500 | 16,049 | full-screen region picker + Google Lens upload via temp HTML |
| `src/browser_indexer.rs` | 462 | 16,742 | Chromium/Firefox history+bookmarks parsers |
| `src/markdown.rs` | 461 | 15,257 | markdown → render blocks/inlines for AI panel |
| `src/git_indexer.rs` | 455 | 15,332 | repo discovery, git CLI, TODO scan |
| `src/hotkey.rs` | 400 | 12,703 | hotkey parse/validate/register |
| `src/settings.rs` | 293 | 9,175 | settings JSON model + load/save |
| `src/inspect_db.rs` | 244 | 9,921 | dev-only DB inspection binary |
| `src/uninstall.rs` | 228 | 8,422 | cleanup binary |
| `src/settings_startup.rs` | 94 | 2,812 | HKCU Run registration |
| `src/applog.rs` | 40 | 1,448 | append-only diagnostic log |
| **total** | **~32,800** | **1,292,018** | 16 Rust files, 3 binaries, 1 Slint UI (2,232 lines) |

`protonsearch/Cargo.toml` declares three binaries: `ProtonSearch` (`src/main.rs`), `inspect-db`, `uninstall` — so "the app" is really one 13 KLOC entry binary plus two utility binaries in the same crate. Dependencies are broad: `windows 0.58` with 30+ feature groups (COM, Shell, GDI, DataExchange, Media_Ocr, Media_SpeechRecognition, ...), `slint 1.17`, `rusqlite` (bundled SQLite), `ureq` (blocking HTTP), `pdf-extract`, `docx-lite`, `winvd`, `image`, `notify`, `walkdir` (`Cargo.toml:18-81`).

---

## 2. Component / process map

```text
ProtonSearch.exe (one binary, three runtime modes)
├── main()  [main.rs:668]
│   ├── --extract-content <path>  → indexer::extract_content_subprocess  (throwaway child)  [main.rs:684-689]
│   ├── --settings                → settings_ui::run_settings_window     (Slint window, same exe) [main.rs:690-701]
│   └── normal launcher GUI       → run(first_settings_run)             [main.rs:809]
│
├── startup workers (spawned from run())
│   ├── search engine ×2 (fast + slow)      [main.rs:1345-1432]
│   ├── file indexer thread                 [indexer.rs:382-428]
│   ├── filesystem watcher                  [indexer.rs:264-360]
│   ├── browser indexer (10 min loop)       [browser_indexer.rs:16-28]
│   ├── git indexer (15 min loop)           [git_indexer.rs:69-95]
│   ├── timeline tracker (foreground window) [main.rs:11915]
│   ├── Windows clipboard-history import    [main.rs:1353-1365]
│   └── Hermes gateway launcher (optional)  [main.rs:1138; ai.rs:548]
│
├── Win32 message loop (custom WndProc)     [main.rs:809-1490, wnd proc 1495+]
│   ├── WM_HOTKEY → show/hide launcher      [main.rs:1829]
│   ├── WM_CLIPBOARDUPDATE → save clip      [main.rs:1937]
│   ├── WM_SEARCH_RESULTS → merge results   [main.rs:~2120-2360]
│   ├── WM_TIMER (animation/debounce/icon)  [main.rs:1592+ timers]
│   └── WM_DESTROY → GDI/client cleanup     [main.rs:3698, 3745]
│
├── launcher.rs  — executes a selected result's launch_command
├── ai.rs        — HTTP client + local hermes gateway control
└── persistent state: %APPDATA%\protonsearch\file_index.db (SQLite WAL)
```

### Entry / lifecycle facts
- Single instance enforced with a named mutex `Local\\ProtonSearchInstanceMutex`; second launch posts `WM_COMMAND(1)` to the existing window and exits (`main.rs:637-665`).
- OmniSearch→ProtonSearch migration: `std::fs::rename(%APPDATA%\omnisearch → %APPDATA%\protonsearch)` **only if the target does not exist** (`main.rs:733-742`).
- Panic hook writes message/location/backtrace to `%APPDATA%\protonsearch\panic.log` (`main.rs:744-781`).
- DPI awareness set per-monitor-v2; COM initialised apartment-threaded for the UI process (`main.rs:690-701, 711-718`); MTA in worker threads that need WinRT.
- Settings are JSON in the **exe directory first**, then `%APPDATA%`; save falls back if the exe dir is read-only (`settings.rs:192-268`).

---

## 3. Search pipeline

Two `SearchEngine` instances are built at startup: a **fast** one with the in-memory filename index and a **slow** one without it (`main.rs:1345-1348`; `search.rs:903-905`). Each runs on its own worker thread with its own channel; each coalesces queued requests to the newest query (`main.rs:1376-1423`). Results are boxed, sent as `WM_SEARCH_RESULTS` with `query_id` (+ bit 32 = final), and the UI drops stale query ids. There is **no cancellation token** — correctness relies on query_id filtering and the coalescing loop.

`SearchEngine::new` (`search.rs:651-909`) does DB/table setup, loads the settings-catalog embedding blob (`include_bytes! assets/catalog.bin`, read at `search.rs:651-682`), seeds quicklinks/snippets/focus/AI defaults, then `scan_apps()` + `scan_recent_files()` + (fast engine) `build_file_index()`.

### Source inventory (what can produce results)

| Source | Entry point | Data |
|---|---|---|
| Apps (desktop + UWP) | `search.rs:6887-7014` (`SHGetKnownFolderItem(FOLDERID_AppsFolder)`, `IShellItem`), `search.rs:6448-6460` | live enumeration + Start-Menu scan |
| Filename (in-memory) | `search.rs:996 build_file_index`, scan in `SearchEngine::search` | `files` table materialised as `FileRow` |
| Content FTS5 | `search.rs:3442 search_with_fts`, `files_fts MATCH` | `files_fts` (14,609 docs here) |
| Settings / Control Panel catalog | `search.rs:911-956` (FTS) over `settings_catalog_fts` | 238 embedded catalog rows, FTS5 |
| Clipboard | `search.rs:2460-2490` | `clipboard_history` (+ OCR text) |
| Recent files | `search.rs:6743-6830` | `%APPDATA%\Microsoft\Windows\Recent` |
| Browser history/bookmarks | `search.rs:2985-3100, 3100-3210` | `browser_items` |
| Git commits / branches / TODOs | `search.rs:3210-3442` | `git_*` tables |
| Memory/activity events | `search.rs:1711+` | `memory_events` + FTS |
| Timeline (window titles) | `search.rs:1493-1630` | `timeline_events` (empty in baseline) |
| Notes | `search.rs:10650-10700` | files under `%APPDATA%\protonsearch\notes` |
| Quicklinks / snippets | `search.rs:10591-10649`, keyword triggers `10271-10301` | DB tables |
| Games (Steam) | `search.rs:10481-10545` | hardcoded `steamapps` paths, `appmanifest_*.acf` |
| Open windows | `search.rs:10162-10269` | live `EnumWindows` + Levenshtein |
| Processes (kill) | `search.rs:10072-10159` | live `tasklist` CSV |
| Calculator / conversion | `search.rs:9113-10060` | in-process recursive-descent parser |
| Quick actions / commands / settings entries | `search.rs:7015-9045` | static `QuickAction`/catalog arrays |
| AI quick commands | `search.rs:4614-4670`, executed `main.rs:5550-5590` via `ai::run` | `ai ask/explain/translate/summarize/grammar/bugs` |
| Web/Lens + AI browser prefixes | `main.rs:6537`, `circle_to_search.rs`, `search.rs:4286-4322` | launches browser |

### Prefix/scope parsing
Prefixes are handled imperatively in `SearchEngine::search` by a chain of `starts_with` checks (`search.rs:4197-4284`): `switch:`/`window:`, `clip:`/`clipboard:`, `bookmarks:`, `history:`, `commits:`, `focus:`, `notes:`, `memory:`, `games:`, `todos:`, `file:`, `folder:`, `code:`, `img:`/`image:`/`screenshots:`/`ocr:`, `chats:`, `agentchats:`, `agents:`, plus AI prefixes and keyword-triggered quicklinks/snippets before that. Prefix removal also exists in `clean_prompt`/`peel_prefixes` for natural-language parsing (`search.rs:5356-5620`). This list is the compatibility target for v2 scope semantics.

### Ranking/filtering
`SearchResult { entry: CatalogEntry, score: f32 }` (`main.rs`/`search.rs:186-192`). UI adds relevance modifiers, filter pills, sort, section labels, "why it surfaced" tags (`main.rs:10407-10660`). Dedupe via launch command/path keys (`search.rs:262-302`). Re-ranking happens **in the UI** (`main.rs:10407-10540`), not in one deterministic engine — a coupling v2 must break.

---

## 4. Indexing subsystem

- **Crawl**: `run_indexer_folders_inner` (`indexer.rs:707-960`) walks scan folders; skips a long built-in ignore list + user `ignored_folders`; `files` rows + FTS content.
- **Priority phasing**: indexer thread waits 3 s then indexes Desktop/Downloads/Pictures/Documents; waits 15 s then full crawl of user profile/other drives (`indexer.rs:382-428`).
- **Watcher**: `notify` recommended watcher per scan folder; coalesces events and writes batched updates through a `Mutex<VecDeque<PendingUpdate>>` / MTX (`indexer.rs:264-360, 517-560`).
- **Content extraction**: plain text read capped at 50 KB (`indexer.rs:1106-1120`); PDF via `pdf-extract` and DOCX via `docx-lite` **in a child process** (`indexer.rs:1018-1104`) to survive parser stack overflows; 30 s watchdog, stdout pipe drained on a helper thread.
- **OCR**: WinRT `Windows.Media.Ocr` via `OcrEngine::TryCreateFromUserProfileLanguages` on decoded images (`indexer.rs:1174-1290`, `OcrEngine::RecognizeAsync` at 1273). No OCR bounding-box/offset storage.
- **State**: `indexer_state` key/value written for UI polling (`indexer.rs:632-647`); baseline left `is_indexing=1` stale.
- **Failure handling**: most DB writes ignore errors (`let _ =`), logging to `indexer.log`; no retry queue.

---

## 5. Browser and git indexers

**Browser** (`browser_indexer.rs`): discovers Chromium profiles under `%LOCALAPPDATA%\Google\Chrome\User Data`, Edge/Brave equivalents, and Firefox profile dirs; copies live SQLite files to `%TEMP%\<browser>_..._<timestamp>.db` to avoid locks; parses `urls`/`places`; inserts into `browser_items` and emits `memory_events`; prunes history to newest 5,000 rows. Ten-minute loop, 10 s initial delay. Temp cleanup via `TempFile` Drop (`:6-14`). Errors are printed and ignored; only `http(s)` URLs are inserted (`:300`). Firefox temp copy at `:324-335`.

**Git** (`git_indexer.rs`): recursive repo discovery to depth 8, pruning at first `.git` and a skip list (`:11-49`); executes `git branch --no-color` and `git log --max-count=100 --format=...` with `current_dir(repo)` and `CREATE_NO_WINDOW` (`:275-323`); scans repo files for TODO/FIXME with extension allowlist (`:325-360`); writes `git_*` + `memory_events`; 15-minute loop, 30 s initial delay. Git executable resolution: PATH first, then Program Files/LOCALAPPDATA paths, then bare `git` (`:229-273`).

---

## 6. Actions, launcher, AI

- `launcher.rs::launch(cmd)` (`:13-215`) is a string-dispatched command interpreter: file paths (`ShellExecuteW` "open"/"runas"), URLs, folders, and many special prefixes; error fallbacks open Explorer.
- `handle_action` (`:216-690`) spawns system tools: `explorer`, `control`, `taskkill`, `shutdown`, `rundll32`, `powershell`, plus window management via `winvd` (`:752-830`) and focus-session enforcement (`:335-380`).
- **AI**: blocking `ureq` OpenAI-compatible client (`ai.rs:1-3, 345-480`); key resolution order = DB `ai_settings.api_key` → env (`OPENCODE_API_KEY`, `DEEPSEEK_API_KEY`, `OPENSEARCH_AI_KEY`) → `%APPDATA%\protonsearch\opencode_key.txt` / `ai_key.txt` → build-time embedded keys → hardcoded constant (`ai.rs:139-264`). Hermes gateway: local `hermes` executable from `%LOCALAPPDATA%\hermes` or PATH, started with `--replace --accept-hooks` and toolsets enabled (`ai.rs:525-622`); runs API + streaming + approval resolution (`ai.rs:816-1070`); local fast-path executes `powershell -Command` directly from search text (`main.rs:4479-4545`).
- **Clipboard**: `AddClipboardFormatListener` + `WM_CLIPBOARDUPDATE`; stores text (max 500 unpinned) and images (BMP saved to `%APPDATA%\protonsearch\clipboard_images`) with OCR text (`main.rs:1291, 1937-1999, 11370-11450`); also imports the whole Windows clipboard history on startup (`main.rs:11704-11826`); global clipboard mutex `clipboard_lock()` (`main.rs:1710`).

---

## 7. UI

- **Launcher UI is hand-painted Win32 GDI**, not a UI toolkit: one `WNDCLASS` window with `WS_POPUP`, custom non-client/rounded region, double-buffered `paint()` (`main.rs:7747-10235`), GDI fonts/icons, animation timers, custom text layout/wrapping/markdown rendering. This is the single largest coupling in the app: state (`struct State`, `main.rs:317-460`), painting, input handling, search kick-off, and AI panel all live in one file/thread.
- **Settings is a Slint window** (`ui/settings.slint`, 2,232 lines; `settings_ui.rs`) launched in the same exe with `--settings`; it talks to the launcher through window messages (`notify_launcher_settings_changed`, `settings_ui.rs:52-64`) and to SQLite directly. This split is why some state is duplicated.
- **Circle-to-search** is a separate layered full-screen window for region selection (`circle_to_search.rs:75-110`), then builds a temp HTML form and POSTs the screenshot to Google Lens in the default browser (`:410-448`).
- **Icons**: async worker threads + `WM_ICON_LOADED` message; COM `IShellItemImageFactory`/PIDL extraction; HICON caches with eviction (`main.rs:7315-7570, 12041-12090`).

---

## 8. Installer / updater / uninstall

- `protonsearch/installer.iss`: Inno Setup, per-user install to `{localappdata}\Programs\ProtonSearch`, bundles `ProtonSearch.exe` + `uninstall.exe`, kills known process names during install, removes legacy OmniSearch/OpenSearch installs, and **deletes `{userappdata}\protonsearch` on uninstall** (`:32-33`).
- `src/uninstall.rs`: standalone cleanup binary; confirms with MessageBox, force-kills ProtonSearch/protonsearch/omnisearch/hermes, clears HKCU Run entries, invokes Inno silent uninstaller, deletes data dir(s) + install dirs + shortcuts, self-deletes from Temp.
- `src/settings_ui.rs:669-900`: update check fetches `https://raw.githubusercontent.com/PranshulSoni/protonsearch/main/update.json`, compares `version`, downloads `url` to `%TEMP%\protonsearch_update.exe`, then launches it `/SILENT /SUPPRESSMSGBOXES /NORESTART` detached after closing the launcher. No checksum/signature verification (see security register).
- `scripts/install.ps1`: downloads the latest GitHub release asset and runs it (no hash check). `scripts/update-sign-install.ps1`: signs a downloaded installer with `signtool` and verifies **after** signing (for maintainer release flow).
- `packaging/winget/`: stale OmniSearch 1.0.0 manifests.

---

## 9. Unsafe FFI inventory (why "unsafe" is everywhere)

Occurrences of the token `unsafe` per file (includes comments/attributes, so treat as an upper bound): `main.rs` 215, `launcher.rs` 30, `settings_ui.rs` 19, `search.rs` 16, `indexer.rs` 11, `circle_to_search.rs` 10, `hotkey.rs` 2, `inspect_db.rs` 2, `git_indexer.rs` 1, `uninstall.rs` 1.

Representative patterns / lifetimes:
- Raw `State` pointer stored in `GWLP_USERDATA` and dereferenced in every `WM_*` handler (imports `main.rs:1595`); comments acknowledge re-entrancy during `SetWindowPos` (`main.rs:4245, 4321`).
- Cross-thread UI notification by `Box::into_raw` + `PostMessageW` (`main.rs:1389-1396, 1413-1421`); receiver reconstructs with `Box::from_raw` (`main.rs:2129` for approvals).
- `SendHwnd` newtype to smuggle `HWND` across threads (`main.rs:460`).
- COM: `IShellItem`, `IShellItemImageFactory`, WinRT `StorageFile`/`BitmapDecoder`/`OcrEngine`, `Clipboard::GetHistoryItemsAsync` (main.rs:11704+, search.rs:6448-6470, indexer.rs:1174-1290).
- Win32 memory ownership: `SHGetKnownFolderPath` + manual `CoTaskMemFree` (`launcher.rs:693-706`), GDI font/icon destruction at `WM_DESTROY`/cache eviction (`main.rs:3698, 7465-7520`), `SetThreadPriority` in indexers.
- Raw Win32 sound/media APIs, `keybd_event`, `EnumWindows` callbacks with borrowed `Vec` pointers (`search.rs:10167-10217`, `launcher.rs:240-315`).

**Risk profile:** every one of these is a same-process safety obligation with no test coverage. v2 replaces this with a small `ps-windows` wrapper crate behind safe APIs (already scaffolded at `crates/ps-windows/src/lib.rs`).

---

## 10. Concurrency and locking map

| Lock/primitive | Where | Notes |
|---|---|---|
| Settings file `Mutex<()>` | `settings.rs:6-9`; used `load/save` | comments document re-entrancy hazard (`:227-245`) |
| AI DB connection `Mutex<ConnCache>` | `ai.rs:104-137` | pooled connection; comment warns not to hold across key lookup to avoid deadlock (`:139-142`) |
| Clipboard `Mutex<()>` | `main.rs:1710 clipboard_lock()` | serialises WM_CLIPBOARDUPDATE vs explicit copy/paste (`:11884`) |
| Indexer queues | `indexer.rs:574-631` (`Mutex<VecDeque<ExtractJob>>`, worker threads) | extraction concurrency bounded by job queue |
| Watcher state | `indexer.rs` `Mutex` around pending update buffer | batched flushes |
| UI state | single-threaded message loop | `State` is mutated only on UI thread |
| Search workers | `mpsc` per engine + `query_id` | last-write-wins coalescing; no cancellation |
| Timers | Win32 `SetTimer` + `WM_TIMER` | debounce, animation, icon batching |

No global lock hierarchy is documented. v2 must define one (SQLite write lock > index state > UI state) and keep blocking work off `ps-core`'s async path.

---

## 11. Blocking I/O (where the legacy app can stall)

- `ureq` is synchronous; all AI calls run on spawn threads but hold a worker for the full request (`ai.rs:345-480, 703-1070`).
- `git` CLI `.output()` in indexer thread (30 s + 15 min loop) and `tasklist` in `search_processes` (`search.rs:10080`) — the latter is called from a search path; it can add tens of ms and is exposed to tasklist weirdness.
- PDF/DOCX extraction is subprocess-isolated with 30 s watchdog (`indexer.rs:1052-1104`) — good pattern to keep.
- Update download/check on a spawn thread but writes a fixed temp file (`settings_ui.rs:735-841`).
- SQLite: every subsystem opens its own connection, sets `busy_timeout(5s)` + WAL (`search.rs:725-727`, `indexer.rs`, `browser_indexer.rs:31-33`, `ai.rs:94-101`). `settings_ui.rs` status thread uses 300 ms busy timeout to avoid UI stalls (`:920-923`).
- Icon/thumbnail loads happen on UI thread at paint time in some branches (e.g. `get_file_icon`/`load_shell_thumbnail` calls) — a known jank source in hand-painted UIs.

---

## 12. Coupling analysis (size alone is not the defect)

1. **One crate, zero internal boundaries.** `main.rs` calls `search`, `ai`, `indexer`, `launcher`, `settings`, `settings_ui`, `circle_to_search` directly; there are no traits/facades between UI and engines. Any engine signature change ripples through a 13 KLOC file. v2's crate split (`ps-core`/`ps-index`/`ps-sources`/`ps-windows`/`ps-ocr` + Tauri shell) is the fix.
2. **UI and search logic are interleaved.** Filter counts, ranking adjustments, dedupe, section labels and "why it surfaced" tags live in the UI (`main.rs:10407-10660`), while catalog data lives in `search.rs` (~9 KLOC of static data before the engine). v2 must put ranking in `ps-core` and keep the UI a pure renderer over `ps-ipc` DTOs.
3. **Testability is accidental.** 85 tests exist because the authors extracted pure helpers (geometry, parsing, timestamps) — but there is no seam for the real search path, indexer, browser copy, git calls, updater, or clipboard. v2's rule should be: sources are traits, file/DB IO behind `ps-sources`, and E2E tests run against fixtures, not the host's real AppData.
4. **Data model is path-keyed** (`files.path` PK) with no file identity; incremental update means delete+insert/rescan. Kensaku's `(device, inode)`/`file_id` identity is the better model for renames (see research doc).
5. **Static catalog blob** (`assets/catalog.bin` via `include_bytes!`) mixes data into the binary; updating 238 settings entries requires a rebuild. v2 should ship catalog as data with a version and FTS ingest step.
6. **No plugin/extension boundary** — every feature is compiled in; adding a source touches the monolith and the binary size. Compare Flow Launcher / PowerToys Command Palette extension models in the research doc.
7. **The good patterns worth preserving:** subprocess isolation for document parsers (with watchdog), WAL + busy_timeout everywhere, background low-priority indexer with priority folders, `TempFile` RAII for browser DB copies, CREATE_NO_WINDOW for all shell-outs, in-memory filename index for sub-10 ms queries, and the settings-catalog FTS idea.

---

## 13. Porting constraints for v2 (derived, not prescriptive)

1. Do not copy `main.rs`/`search.rs` structure; re-implement behaviour behind `ps-ipc` DTOs.
2. Preserve observable semantics: prefix names (`file:`, `folder:`, `code:`, `img:/ocr:`, `bookmarks:`, `history:`, `clip:`, `commits:`, `todos:`, `switch:/window:`, `focus:`, `notes:`, `memory:`, `games:`, `quicklink:`, `snippet:`, `agents:`, `agentchats:`), calculator syntax, settings-catalog entries, and hotkey combos.
3. Keep the legacy DB strictly read-only during migration; the legacy uninstaller would delete it (`installer.iss:32-33`).
4. Re-implement, don't port: updater trust, clipboard capture policy, AI key storage, focus-session process killing, Wi-Fi-password command.
5. Legacy risks to design out: dead vector-search code, blocking `ureq` in shared threads, tasklist in search path, fixed-name temp files, version metadata drift.

---

*All file:line references are to `protonsearch/src/*.rs` or `protonsearch/` at commit `de0ef19` unless stated. Claims requiring runtime observation are marked UNVERIFIED in `docs/audit/GATE0_BASELINE.md`.*
