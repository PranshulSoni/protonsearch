# ProtonDock backend specification (reconciled and phased)

Date: 2026-11-08. Branch: `dock`. Repo HEAD when written: `7727969`.
Status: proposed for user approval. Nothing in this document has been implemented or natively verified.
Audience: the orchestrator, the Sonnet high planner and reviewer, and Haiku implementers (1 to 3 files per task).
Supersedes: the backend parts of `antigravity/spec.md` (922 lines) and `antigravity/BACKEND_IMPLEMENTATION_PROMPT.md` (277 lines). Both files are unchanged and stay as design input. Where this document and those files disagree, this document wins; Section 2 lists every disagreement and the reason.
Builds on: `docs/widgets/CONTRACT.md`, `docs/widgets/NATIVE_PROVIDERS.md`, `docs/qa/dock-defects-2026-11-07.md` (peer session, five dock defects).
Companion: `docs/superpowers/specs/2026-11-08-dock-design-system.md` (UI port spec by a peer session, committed as `ad5640a`; section numbers below refer to that version). Where the two documents overlap (catalogue ids, layout v3, icon ladder, status states) this document adopts the UI document's decision and states the backend consequence. Conflicts and gaps between the two are listed in Section 2.9 as XD items.

Identifier prefixes used here: `BE-nn` backend work package, `BD-n` decision for the user, `XD-n` cross-document item, `E-n` evidence entry (Appendix A). The UI document uses `U*` and `D*`; they never collide with these.

## 1. Header and scope

### 1.1 Purpose

ProtonDock replaces the Windows taskbar with a customizable dock built only from widgets (the user adds, orders and sizes them). The Search widget opens a Spotlight-like launcher backed by the existing Rust ProtonSearch engine, which stays authoritative. The program goal is a proper native Windows dock. This document turns the two antigravity inputs into one reconciled backend plan that Haiku can implement phase by phase without judgment calls: canonical names, canonical storage, verified feasibility per widget, one IPC contract, a persistence design, security rules, an honesty register and ordered work packages.

### 1.2 In scope and out of scope

In scope: Rust code in `src-tauri/` and the engine crate `protonsearch/` where an engine API must be exposed; the IPC surface (Tauri commands, provider ids, widget actions, events); persistence and migration of what exists today; security and capability design; data honesty; backend work packages and the decisions they need.

Out of scope (owned elsewhere, never duplicated here): visuals, CSS, tokens, motion, glyphs, widget React components and layout geometry (UI document and the user-supplied HTML); the five dock defects (peer ledger `docs/qa/dock-defects-2026-11-07.md`, including the icon sharpness and jitter fixes); hiding or replacing Explorer's taskbar (only its backend consequences are recorded, Section 4.6, BD-9).

### 1.3 Authority order

1. `CLAUDE.md`, `AGENTS.md`, `rules.md` (repo rules; they outrank everything here).
2. `docs/widgets/CONTRACT.md` and `docs/widgets/NATIVE_PROVIDERS.md` (existing contracts; extended here, never weakened).
3. This document for every backend, IPC, provider, persistence and credential statement.
4. The UI document for catalogue ids, layout v3 shape, icon ladder and status-to-state mapping (adopted below).
5. `antigravity/*` (input only) and `dock-editable.html` (visual ground truth only; it is not a data source).

### 1.4 Evidence labels

Claims carry a label so reviewers know how far to trust them.

| Label | Meaning |
|---|---|
| [repo] | Read in this repository, with `file:line`. |
| [obs] | Observed on this machine on 2026-11-08 (Windows 11, `10.0.26300.0`) by a read-only diagnostic. No project binary was launched. Appendix A. |
| [doc] | Stated by a Microsoft Learn page or API reference. Appendix A has the URL. |
| [community] | Reported in a public issue or forum; not an official statement. |
| [crate] | Verified in the local `windows` 0.58.0, `rusqlite` 0.31.0 or `libsqlite3-sys` 0.28.0 source in the cargo registry. |
| [unverified] | Not verified here. Any package that depends on it starts with a spike (an `#[ignore]` test run once and recorded in `docs/qa/STATUS.md`). |

### 1.5 How this document was produced

Read in full: both antigravity files; `CLAUDE.md`, `AGENTS.md`, `rules.md`; `CONTRACT.md`, `NATIVE_PROVIDERS.md`; the peer UI document. Read by outline and line range: `src-tauri/Cargo.toml`, `tauri.conf.json`, `build.rs`, `capabilities/*`, `permissions/commands.toml`; `src-tauri/src/main.rs`, `widget_data.rs`, `running_apps.rs`, `ai_usage.rs`, `local_calendar.rs`, `dock_lifecycle.rs`, `clipboard_widget.rs`, `known_folders.rs`; `protonsearch/Cargo.toml`, `indexer.rs`, `search.rs`, `weather.rs`, `settings.rs`, `taskbar_recovery.rs`, `ai.rs` (key resolution only); `src/services/native.ts`, `src/widgets/sdk/{provider,types}.ts`, `src/widgets/final/{meta,useFinalWidgetData}.ts`, `src/components/dock/{layout,DockShell}.tsx`; `.agents/skills/rust-skills` (rule index). No build, test or application launch was run. Three read-only diagnostic scripts (window class enumeration; WinRT status reads; a Bluetooth property count plus a `netsh` presence check) and about two dozen web lookups produced the [obs], [doc] and [community] labels. Headroom was loaded but not used for compression: reads were bounded line ranges and re-sending them as arguments would have cost more than it saved.

### 1.6 Rules carried into every work package

1. The Rust search engine stays authoritative. No second search index, no new FTS table, no JavaScript index.
2. Never present fabricated, simulated or placeholder data as live (`CLAUDE.md`, `AGENTS.md`). A missing value is `null`; an unavailable source is an honest status.
3. Preserve existing user data. No destructive migration: archive before rewriting, keep unknown ids, never reset a profile.
4. The renderer never supplies a path, executable, process id, window handle, URI, SQL text or secret (the one exception is a secret entering a credential-save command exactly once, Section 7.2).
5. No listening sockets, no child console processes (an unavoidable child sets `CREATE_NO_WINDOW`, `0x08000000`), no administrator elevation, no package identity requirement.
6. Native work runs on `spawn_blocking`. No `std::sync::Mutex` guard is held across an `.await`. Locks are per domain; `NATIVE_LOCK` stays only where WinRT or Core Audio singletons need it (`widget_data.rs:45`). rust-skills: `async-spawn-blocking`, `async-no-lock-await`, `anti-lock-across-await`.
7. One command surface: extend `provider_snapshot` and `widget_action`; a bespoke command needs the justification in Section 5.4.
8. Tests never launch the app; `cargo run` and the executable are never started as background processes. Kill `protondock` and `protonsearch` before builds (`rules.md` rule 2).
9. Commit and push after every verified edit, explicit paths only (`rules.md` rule 1).
10. No em dashes, en dashes or emoji in code, comments, documents or UI copy.

### 1.7 Reading guide

Section 2 is the contradiction ledger. Section 3 decides the widget catalogue and the saved-layout migration. Section 4 audits every widget against the real Windows APIs. Section 5 is the one IPC contract. Section 6 is persistence. Section 7 is security. Section 8 is the data honesty register. Section 9 lists the work packages in dependency and risk order. Section 10 collects the decisions the user must make, each with a recommended default. Appendix A lists evidence; Appendix B lists the repo defects found while reconciling.

## 2. Contradiction ledger

Columns: `spec` is `antigravity/spec.md`; `prompt` is `antigravity/BACKEND_IMPLEMENTATION_PROMPT.md`; `repo` is the code today. The last column is the canonical choice and why.

### 2.1 IPC names and shapes

| # | Topic | spec | prompt | repo | Canonical choice and why |
|---|---|---|---|---|---|
| A1 | Start menu | `toggle_start_menu` (2.0), module `start_menu.rs` | `trigger_start_menu` (3.1), module `system_actions.rs` | `open_start` falls back to the Apps folder (`src-tauri/src/main.rs:446`), registered, permitted in `dock-commands`, typed as `native.openStart` | Keep `open_start`. Replace its body with an injected Win key tap (BE-14). Why: zero frontend or ACL change; the two new names would add commands for one verb. |
| A2 | System telemetry | `get_system_telemetry` returning `{cpu, ram, ssd, history, peak}` (2.15) | `get_system_stats` plus event `system-stats-tick` every 2000 ms (3.3) | `provider_snapshot("system")` returns `{cpu, ram, battery, batteryCharging, temperatureC, uptimeSeconds, connectivity}`; `provider_snapshot("storage")` returns `{totalBytes, freeBytes, mountPoint}` (`main.rs:724-783`) | Keep the providers. The SSD figure is system-drive used percent from `storage`. History and peak are a frontend ring buffer. No event. Why: the scheduler already polls only while visible; a native 2 s thread would keep polling while the dock is hidden. |
| A3 | Search | `search_query`, `calculate_expression` (2.10); `kensaku_search`, `kensaku_index_status` (2.17) | `kensaku_query` | `search({query, limit, requestId})`, `execute_search_result({resultId, actionId})` (`main.rs:174`, `238`) | Keep `search` and `execute_search_result`. Scope is expressed by the engine's existing prefixes (`file:`, `code:`, `notes:`), typed by the UI, not by a second argument. The calculator already returns `CALC` results (`protonsearch/src/search.rs:4771`) and Enter copies through `copy:`. Why: four names for one existing command. |
| A4 | Quick Look | `kensaku_quick_look({path})` | same | none | `search_preview({resultId})` (BE-41). Why: the renderer must not supply a path (CONTRACT.md). |
| A5 | Open, reveal, copy path | `open_path`, `reveal_in_explorer` (path args) | `open_file_path`, `reveal_in_explorer` (path args) | `execute_search_result` accepts only `actionId == "open"` (`main.rs:243`) | Extend `execute_search_result` with `actionId` in `open`, `reveal`, `copy-path` (BE-40). Why: result ids already identify the target natively. |
| A6 | Index status | `kensaku_index_status` | none | `INDEXING_PROGRESS` static (`indexer.rs:12`) | `search_index_status()` (BE-42), read-only. |
| A7 | Running apps | `get_running_apps`, `activate_app({appId})`, `save_app_order({appIds})` | `list_running_apps`, `activate_app_window({processId, hwnd})`, `reorder_running_apps`, event `running-apps-changed` via `SetWinEventHook` | `running_apps_snapshot`, `running_apps_activate({id})` with issued ids `running-N` and an identity recheck (`running_apps.rs:23-47`, `323-344`) | Keep the repo. Order is persisted by the frontend in layout settings, keyed by a native `key` (BE-13). `hwnd` and `processId` from the renderer are rejected. The event is replaced by 2 s visible polling. |
| A8 | Installed and pinned apps | `list_installed_apps`, `pin_app`, `unpin_app`, `launch_app` | same, but `launch_pinned_app` | none; the engine enumerates `shell:AppsFolder` privately (`search.rs:6995`, `pub(crate)`) | `installed_apps_snapshot`, `pinned_apps_snapshot({keys, iconPx})`, `widget_action("app:launch:<key>")`. The pin list lives in the layout as `pinnedApps` (UI document 5.4). BE-34 to BE-36. |
| A9 | Media | `get_media_status`, `toggle_media_playback`, `media_next_track`, `media_prev_track`, event `media-playback-updated`; payload `positionSec`, `durationSec`, `isPlaying`, `thumbnailBase64` | `skip_media_next`, `skip_media_prev`, event `media-status-changed` | `provider_snapshot("media")` with `positionMs`, `durationMs`, `playing`, no artwork; `widget_action("media:toggle", "media:next", "media:previous")` (`widget_data.rs:114-168`, `311-395`) | Keep the repo contract. Add `artworkId` to the snapshot and one command `media_artwork()` (BE-15, BE-16, BE-49). Why: a base64 image in every 2 s poll is wasteful and `artworkId` changes rarely. |
| A10 | Weather | `get_weather`, `set_weather_location({city, lat?, lon?})` | `get_weather`, `set_weather_city({city})` | `weather_snapshot({latitude?, longitude?})`; location lives in layout settings (`main.rs:657`, `protonsearch/src/weather.rs:24-55`) | Keep `weather_snapshot`. Add `weather_geocode({query})` and `weather_locate()` (BE-18, BE-19). No setter: location is a widget setting saved through `save_dock_settings`. |
| A11 | Battery | `get_battery_status`, event `battery-changed` from `WM_POWERBROADCAST` | `get_battery_telemetry` | percent and charging inside `system` only | `provider_snapshot("battery")` (BE-11). No event: the Tauri host owns no message window for `WM_POWERBROADCAST` and polling every 30 s is enough for a slow quantity. |
| A12 | AI usage | `get_claude_usage`, `get_codex_usage`, `save_ai_credentials({provider, apiKey})` | `get_ai_status({provider})` returning `{configured, quotaPct, isWorking}`, `toggle_agent_execution`, `save_ai_credentials` | `ai_usage_snapshot` reads local Codex rollout logs and Claude Code transcripts, no network, no credentials (`ai_usage.rs`, `main.rs:650`) | Keep `ai_usage_snapshot`. No credential command in v1 (4.4.14). `toggle_agent_execution` is dropped: a dock must not start or stop agents and the toggle is a simulation. |
| A13 | Notes | `get_recent_notes`, `save_note({content, title?})`, `delete_note` | `list_notes`, `save_note({content})`, `delete_note` | none. The engine already has `.txt` notes in `%APPDATA%\protonsearch\notes` (`search.rs:8462`) | `provider_snapshot("notes")`, `note_save({content})`, `widget_action("notes:delete:<id>")` over the engine's notes folder (BE-20). |
| A14 | Calendar | none in 2.x | `get_calendar_today`, `get_week_schedule`, `create_calendar_event` | `calendar_select_source`, `calendar_snapshot`, `calendar_forget_source`; read-only, token grants (`main.rs:673-712`) | Keep the repo; add week data (BE-39). `create_calendar_event` is dropped: CONTRACT.md states no calendar writes exist. |
| A15 | Bluetooth and peripherals | `get_bluetooth_status`, `pair_bluetooth_device({deviceId})` | `get_peripherals_telemetry`, `set_anc_mode({deviceId, mode})` | radio state inside `network.bluetooth`, sampled once per process (`widget_data.rs:637-673`) | `provider_snapshot("bluetooth")` for both widgets (BE-30). Pairing becomes `settings:bluetooth`. `set_anc_mode` is dropped (4.4.7). |
| A16 | Wi-Fi | `get_wifi_status`, `toggle_wifi_radio`, `connect_wifi_network({ssid, password?})` | none | `network.wifi` (`widget_data.rs:590-636`) | `provider_snapshot("wifi")`, `provider_snapshot("wifi-networks")`, `wifi:*` actions (BE-32, BE-33). No password path. |
| A17 | Notifications | `get_unread_notifications`, `clear_all_notifications`, `dismiss_notification`, event `notification-arrived` | none | none | `provider_snapshot("notifications")` (count only), `notifications_list_snapshot({showText})`, `notifications:*` actions. No push event (`NotificationChanged` fails unpackaged, E13). BE-37, BE-38. |
| A18 | Clock | `get_system_time` | none | `mapClock(now)` computed from `Date.now()` in `useFinalWidgetData.ts:198` | Dropped. The webview clock and the system clock are the same machine; `Intl` supplies zone and locale. |
| A19 | System tray | `get_tray_icons`, `activate_tray_icon({iconId})` | bullet only | none | Gated (4.4.16, BD-5). The specified enumeration path does not exist on this build. |
| A20 | Assistant | `trigger_siri_assistant`, `configure_assistant_model`, event `assistant-state-changed` | bridged inside `ai_agent_bridge.rs` | none | Gated (4.4.15, BD-6). |
| A21 | Layout and profiles | localStorage keys `protondock_custom_profiles`, `protondock_preview_state_v6`, `protondock_theme`, `protondock_notes_v1` (HTML) | SQLite `profiles` (`is_active`, `widget_order`) and `widget_settings` (`width`) | `load_dock_settings`, `save_dock_settings`, JSON v2 file `protondock-layout.json` (`main.rs:451-504`) | Layout JSON v3 (UI document 5.4). No SQLite tables for profiles or widget settings (Section 6). |
| A22 | Events | `battery-changed`, `media-playback-updated`, `notification-arrived`, `assistant-state-changed` | `system-stats-tick`, `running-apps-changed`, `media-status-changed` | `settings-updated`, `widget-card:show` only | No new events in v1. One reserved event `provider-changed` is specified in 5.7 for later use. |
| A23 | Search shortcut | `Alt + Space` or `/` (2.10), `Ctrl + Space` (2.17) | `Alt+Space` | global shortcut `Alt+Space` plus the configured hotkey; a registration failure aborts startup (`main.rs:1099-1102`, `?` inside `setup`) | Native owns shortcuts; the page registers none (UI document P9). Registration failure must not abort the app (BE-08). |

### 2.2 Storage locations and secrets

| # | Topic | spec | prompt | repo | Canonical choice and why |
|---|---|---|---|---|---|
| B1 | Dock settings | browser local storage | `protondock.db` tables | `%APPDATA%\protonsearch\protondock-layout.json`, atomic replace (`main.rs:454`, `472-500`) | Keep the file (v3). Why: small, atomic, versioned, already validated by `layout.ts` with its tests (`layout.test.ts`); no query need, so SQLite adds nothing. |
| B2 | Data root | `%LOCALAPPDATA%/protondock/` | `%APPDATA%\ProtonSearch\` | `data_dir()` returns `%APPDATA%\protonsearch` and migrates a legacy `omnisearch` folder (`main.rs:163-172`) | Existing user data stays under `data_dir()` (never move it). New machine-bound items (secrets, caches) use `%LOCALAPPDATA%\protondock\`. NTFS is case-insensitive, but code must call `data_dir()` and never hard-code a literal. |
| B3 | Credentials | `%LOCALAPPDATA%/protondock/credentials.json` (no protection stated) | Windows Data Protection API (DPAPI), `CryptProtectData`, no path | none for the dock. The engine stores an optional plaintext `ai_key.txt` and an `ai_settings` table (`protonsearch/src/ai.rs:17`) | File name `credentials.json` in `%LOCALAPPDATA%\protondock\`, every value DPAPI-protected, no plaintext on disk (6.8). Gated: no widget needs a stored key in v1. |
| B4 | Main database | `%LOCALAPPDATA%/protondock/protondock.db` for notes | `%APPDATA%\ProtonSearch\protondock.db` for everything | `%APPDATA%\protonsearch\file_index.db` (engine) | No `protondock.db` in v1 (6.2). The engine's `file_index.db` is untouched. If BD-2 later asks for it, 6.7 is the binding design. |
| B5 | Notes | SQLite `notes` or `%APPDATA%/protondock/notes/*.md` | table `quick_notes` | `.txt` files in `%APPDATA%\protonsearch\notes`, searched by `notes:` and the "Create Note" action (`search.rs:8462`, `quick_actions.rs:1849`) | Reuse the engine's notes folder (BE-20). Why: one notes system, instantly searchable, user-visible files. |
| B6 | Calendars | none | `%APPDATA%\ProtonSearch\calendars\` directory scan and Windows Calendar store | one picker-granted `.ics` file per profile, tokens in `calendar-sources.json` (`local_calendar.rs`) | Keep the repo model. The Windows Calendar store needs the restricted `appointmentsSystem` capability (E10), so it is blocked. |
| B7 | Search index | `files_fts` inside the new database | `files_fts` inside `protondock.db` | `files` plus `files_fts` in `file_index.db`, filled by the indexer and watcher (`indexer.rs:841-876`) | The engine's tables only (6.6). |
| B8 | Roaming versus local | not addressed | `%APPDATA%` (Roaming) for a WAL database | `%APPDATA%` for all data | Do not put WAL databases or secrets in Roaming. The only new files are secrets and caches, in `%LOCALAPPDATA%`. |

### 2.3 Schema columns

| # | Topic | spec | prompt | repo | Canonical choice and why |
|---|---|---|---|---|---|
| C1 | `files_fts` | `(path UNINDEXED, filename, content, extracted_ocr, category, modified_at UNINDEXED)`, tokenizer `unicode61 remove_diacritics 2` | `(path UNINDEXED, filename, extension, extracted_text, ocr_text)`, tokenizer `unicode61` | `(path UNINDEXED, content)` (`indexer.rs:872`), with `files(path PK, name, extension, modified, size, is_dir, last_seen_scan)` beside it | The repo definition. `CREATE VIRTUAL TABLE IF NOT EXISTS` with a different column list is a silent no-op, so a second definition would never apply and inserts would fail. Filename, extension and modified time already live in `files`; image OCR text is already merged into `content` (`indexer.rs:42-51`); category is derived from the extension at query time. |
| C2 | `clipboard_history` | none | `(id TEXT PK, text_content, char_length, source_app, created_at)` | `(id INTEGER AUTOINCREMENT, content TEXT UNIQUE, timestamp, source_app, is_image, pinned, ocr_text)` (`search.rs:742`) | The repo table. Ingestion is in the legacy host (`protonsearch/src/main.rs:1984`, `11388`); the Tauri host does not ingest (CONTRACT.md). Porting ingestion is gated (BE-65). |
| C3 | Notes record | `NoteItem {id, title, content, createdAt, updatedAt}` | table `quick_notes` without a title column | none | Files in the notes folder: title is the file name, times are file times, `id` is an opaque hash of the file name. |
| C4 | Pinned apps | none | table `pinned_apps` with `exec_path`, `icon_blob`, `order_index` | none | `pinnedApps: string[]` of native keys in the layout. No path and no icon blob is stored (a path would let the renderer influence launches; icons are re-rendered at the display's DPI). |
| C5 | Weather payload | `{status: 'unconfigured', 'ready', 'error', temperature, weatherCode, condition, high, low, location, cachedAt}` | `WeatherPayload` | envelope with `temperature`, `weatherCode`, `time`, `utcOffsetSeconds`, `hours[]`, plus apparent temperature, humidity, wind, UV, visibility (`weather.rs:87-185`) | The repo envelope. `condition` is derived from the WMO code in the frontend. |
| C6 | Media payload | `title, artist, album, isPlaying, positionSec, durationSec, thumbnailBase64` | `MediaStatusPayload` | `title, artist, album, albumArtist, source, playing, playbackStatus, playbackRate, positionMs, durationMs, canToggle, canNext, canPrevious` | The repo payload plus `artworkId`. |
| C7 | Battery payload | `percent, wattageRate (W), voltageVolts, amperageAmps, cycleCount, fullCapacityWh, designCapacityWh, temperatureC` | `BatteryTelemetryPayload` | none | New shape in 5.6 with integer milli-units (`mW`, `mWh`, `mV`), `null` for every unknown, no invented zero. |
| C8 | AI usage payload | `AIUsagePayload {usedPercent, tokenLimit, ...}` | `{configured, quotaPct, isWorking}` | `{chatgpt, claude, agent}` objects with `sessionPercent`, `weeklyPercent`, `resetIn`, `tokensUsed`, `plan` (`ai_usage.rs:269-406`) | The repo shape plus `working` (BE-12). Claude has no limit percentages locally (4.4.14). |
| C9 | Installed app record | `InstalledApp {executablePath, appUserModelId, ...}` | table with `exec_path` | none | The renderer receives only `{id (key), name, icon}`. Paths and app ids stay native. |
| C10 | Wi-Fi record | `bssid`, `availableNetworks[].ssid` | none | `ssid`, `signalPercent`, `state` | No `bssid` (it can geolocate a user). Networks carry an opaque `token`. |
| C11 | Pragmas | `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`, `foreign_keys=ON` | same | engine sets WAL and a 5 s busy timeout per connection | If a database is ever added: WAL, `synchronous=FULL` for user-authored data, `busy_timeout=5000`. `foreign_keys` is already on in the bundled build (`SQLITE_DEFAULT_FOREIGN_KEYS=1`, E22). |

### 2.4 Module and file names

| # | Topic | spec | prompt | repo | Canonical choice and why |
|---|---|---|---|---|---|
| D1 | Start | `start_menu.rs` | `system_actions.rs` | none | `src-tauri/src/start_menu.rs` (BE-14). |
| D2 | Weather | `src-tauri/src/weather.rs` or `protonsearch/src/weather.rs` | `weather_service.rs` | `protonsearch/src/weather.rs` | The engine crate file; geocoding is added there so it reuses `ureq` and the bounded-JSON helper. |
| D3 | AI usage | `ai_usage.rs` | `ai_agent_bridge.rs` | `src-tauri/src/ai_usage.rs` | `ai_usage.rs`. |
| D4 | Battery | `system_battery.rs` | `system_battery.rs` | none | `src-tauri/src/battery.rs` (shorter, matches provider id `battery`). |
| D5 | Notes | `notes.rs` | `notes_service.rs` | none | `src-tauri/src/notes.rs`. |
| D6 | Calendar | none | `calendar_service.rs` | `local_calendar.rs`, `calendar_ics.rs` | Keep the repo files. |
| D7 | Bluetooth | `bluetooth_devices.rs` | `bluetooth_peripherals.rs` | none | `src-tauri/src/bluetooth.rs`. |
| D8 | Media | `media_player.rs` | `media_player.rs` | `widget_data.rs::media()` | Keep `media()`; artwork goes in `src-tauri/src/media_art.rs`. |
| D9 | Wi-Fi | `wifi_wlan.rs` | none | helpers in `widget_data.rs` | `src-tauri/src/wifi.rs` for detail and actions; the network provider stays in `widget_data.rs`. |
| D10 | Search | `kensaku_search.rs`, `protonsearch/src/ocr.rs`, `protonsearch/src/semantic.rs` | `kensaku_search.rs` | search commands in `main.rs`; OCR lives in `indexer.rs:1298-1480`; embeddings were deliberately removed (`search.rs:4853`) | No `ocr.rs`, no `semantic.rs`, no `kensaku_search.rs`. Engine files stay as they are. |
| D11 | Others | `system_tray_hook.rs`, `voice_assistant.rs`, `clock.rs`, `profiles.rs`, `installed_apps.rs` | `installed_apps.rs`, `system_stats.rs` | none | Only `installed_apps.rs` is created (BE-34). The rest are dropped or gated. |

### 2.5 Document structure defects in the inputs

| # | Defect | Where | Resolution |
|---|---|---|---|
| E1 | Duplicate section numbers: 2.8 twice (spec lines 323 and 365), 2.15 twice (600 and 753), 2.16 twice (626 and 809), 2.17 twice (647 and 836) | `antigravity/spec.md` | This document numbers every section once. References to the inputs name the heading text and line, never the bare number. |
| E2 | The prompt cites "Rule 4 (never run launcher in background)", "Rule 7 (kill before edit or build)" and "Git Rule 6", but `rules.md` has three rules: commit and push after every edit, kill the app before build or edit, use the knowledge graph first | prompt 1 and 5 | Cite `rules.md` rule numbers as they exist. The "never run the launcher as a background process" rule is carried here as rule 8 of 1.6. |
| E3 | The spec's porting checklist names components (`WeatherWidget.tsx`, `BatteryWidget.tsx`, `SiriRobot.tsx`, `InstalledAppsPicker.tsx`) and modules (`profiles.rs`) that do not exist and do not match the repo layout (`src/widgets/final/`, `src/components/dock/`) | spec 4 | The UI document owns component work; this document owns only backend modules named in Section 9. |
| E4 | The prompt numbers its widget list 1 to 13 under a level-2 heading, so it cannot be referenced by section | prompt 3 | Referenced here by widget id. |

### 2.6 Widget ids, sizes and profiles

| # | Topic | spec and prompt | repo | Canonical choice and why |
|---|---|---|---|---|
| F1 | Widget ids | 19 ids: `start, search, running-apps, pinned-apps, system-stats, calendar, peripherals, music-player, weather, status-battery, wifi, bluetooth, notifications, quick-notes, claude, codex, siri-ai, system-tray, clock` | 14 ids in `src/widgets/final/meta.ts:10-25` and `WIDGET_CARD_IDS` (`main.rs:73`): `windows, search, apps, spotify, weather, system, claude, chatgpt, agent, files, folders, trash, volume, clock` | The UI document 5.3 catalogue, summarised and extended with backend facts in Section 3. |
| F2 | Assistant id | `siri-ai` | none | The UI document renames it `assistant` and reads the alias `siri-ai` (D3 there). The backend accepts both ids in its allowlist. |
| F3 | Sizes | 48 px square, widths 48, 96, 140, 168 (spec 1.2) and capsule widths 110, 120, 124, 136, 150, 196, 200, 220 per widget | `BASE_TILE 46`, height 32 to 64, width 32 to 200 (`src/components/dock/sizing.ts`), named sizes `compact`, `standard`, `wide` with widths 64 to 240 (`meta.ts`) | The UI document 4.3: height 48, steps 48, 96, 140, 168, 220 with an allowed subset per widget; `sizing.ts` is deleted there. The backend is size-agnostic and never reads or writes widths. |
| F4 | Profile defaults | the four lists in `dock-editable.html` (3975-3978) and prompt 2.3 differ from each other (prompt `founder` omits `weather`; the HTML `daily` has `peripherals`) | `PROFILE_DEFAULTS` in `layout.ts:47-52` uses old ids | Native code holds no defaults. The UI owns defaults from the user-supplied HTML. |
| F5 | Profile ids | free-form custom profiles in local storage | four fixed ids `daily, work, founder, focus` (`layout.ts:12`) and native validators that hard-code them (`weather.rs:16-22`, `local_calendar.rs:37-43`) | v3 allows custom profile ids (UI document 5.4). Native validators change to a grammar (BE-06, BE-07): `[a-z0-9][a-z0-9-]{0,23}`. |

### 2.7 Contract violations in the inputs

| # | Violation | Input | Rule broken | Resolution |
|---|---|---|---|---|
| G1 | The renderer supplies a path to read, open, reveal or OCR a file | spec 2.17 IPC, MCP tools `read_file_content(path)`, `ocr_screenshot(path)`; prompt 3.2 | CONTRACT.md: the renderer never supplies a path | Result ids (A4, A5). The MCP tools are excluded (4.5.3). |
| G2 | The renderer supplies `processId` and `hwnd` | prompt 3.6 `activate_app_window` | CONTRACT.md: never a process id or native window handle | Issued ids (A7). |
| G3 | A listening socket on `127.0.0.1:41782` | spec 2.17 MCP server | Security (7.1) | Excluded. |
| G4 | The renderer receives `executablePath` and `appUserModelId` | spec 2.7 `InstalledApp` | CONTRACT.md | Keys only (C9). |
| G5 | Calendar writes | prompt 3.4 `create_calendar_event` | CONTRACT.md: no calendar writes | Dropped. |
| G6 | A Wi-Fi password crosses IPC and the DOM | spec 2.11 `connect_wifi_network({password})` | Security (7.2) | No password path; the OS UI handles new secured networks (4.4.9). |
| G7 | Simulation controls | spec 2.2 charging simulation button; spec 2.16 "Working and Ready test states"; prompt `toggle_agent_execution` | `AGENTS.md`: never present fabricated metrics as live | Removed (Section 8). |
| G8 | Plaintext credential file implied | spec 1.4 | Security (7.2) | DPAPI values (B3). |
| G9 | Parallel Win32 event hooks and message windows in the Tauri host for battery, windows and notifications | spec 2.2, prompt 3.6 | Simplicity and the hidden-window rule (1.6 rule 6) | Polling with the visibility-aware scheduler; hooks only if measured need (5.7). |

### 2.8 Factual errors in the inputs (verified)

| # | Claim | Finding | Evidence |
|---|---|---|---|
| H1 | `ShellExecuteW(... "shell:::{2559a1f8-21d7-11d4-bdaf-00c04f60b9f0}")` opens the Start menu (prompt 3.1) | That CLSID is the Windows Search shell command, not Start. It would open Windows Search and compete with the dock's own Search widget. | E18 |
| H2 | `WM_SYSCOMMAND` with `SC_TASKLIST` posted to `Shell_TrayWnd` opens Start (spec 2.0) | A Windows 7 era mechanism; unverified on Windows 11, so it is not used. | [unverified] |
| H3 | WQL `SELECT DesignCapacity, FullChargeCapacity, CycleCount, EstimatedChargeRemaining, Voltage FROM Win32_Battery` (spec 2.2) | `Win32_Battery` has no `CycleCount` and no `Voltage` (it has `DesignVoltage`). The query is invalid. Cycle count is `BatteryCycleCount` in `root\wmi`. WMI is also not needed: a WinRT battery report supplies the capacities and charge rate. | E12, E4 |
| H4 | System tray icons are enumerated through `Shell_TrayWnd`, `TrayNotifyWnd`, `SysPager`, `ToolbarWindow32` (spec 2.15) | On this Windows 11 build `TrayNotifyWnd` exists but has no descendant windows: no `SysPager`, no `ToolbarWindow32`, no `NotifyIconOverflowWindow`. The path does not exist. | E2 |
| H5 | Per-app badge counts come from `UserNotificationListener` (spec 2.15 item 4, prompt 3.6) | The API returns toast notifications (`NotificationKinds::Toast` only), not app badge numbers. A count of toasts per app is the most it can give, and only if a spike passes. | E8, E13 |
| H6 | AirPods battery and ANC are read and controlled through the GATT Battery Service 0x180F (spec 2.12, 2.15 item 3) | AirPods are Classic audio devices that report battery over Apple's protocol; Windows exposes one level through a device property. Left, right and case levels need reverse-engineered BLE advertisements. ANC and transparency use Apple's AACP over Classic L2CAP PSM 0x1001, which has no public user-mode Windows API. | E5, E14 |
| H7 | `ISensorManager` or `ILocationReport` for location (spec 2.1) | The legacy sensor API; Microsoft's guidance is `Windows.Devices.Geolocation.Geolocator`. | E7 |
| H8 | The Anthropic "API key (sk-ant-...)" unlocks usage quota percentages (spec 2.3) | An ordinary API key has no usage endpoint. The usage and cost endpoints need an Admin key, organization accounts only, and return tokens and cost, not a quota percentage. | E15 |
| H9 | `sysinfo` with a 2000 ms background thread for CPU and RAM (spec 2.15, prompt 3.3) | The repo already reads CPU through `GetSystemTimes` and RAM through `GlobalMemoryStatusEx` with a visibility-aware scheduler; a thread would poll while hidden and a new crate adds nothing. | [repo] `main.rs:784-806` |
| H10 | `fastembed-rs` semantic search is part of the engine (spec 2.17) | The engine removed embeddings on purpose (`search.rs:4853`, comment "embeddings removed"). | [repo] |
| H11 | A Windows OCR watcher over `Pictures\Screenshots` must be written (spec 2.17, prompt 3.2) | The engine already indexes Desktop, Downloads, Pictures and Documents first, then the user profile, OCRs images with `Windows.Media.Ocr.OcrEngine`, and keeps a file watcher (`indexer.rs:435-480`, `1354-1480`). | [repo] |
| H12 | `SHIL_EXTRALARGE` is 48x48 and `SHIL_JUMBO` is a true 256x256 source (spec 2.8) | `SHIL_JUMBO` returns 256 even when the app has only 48 px frames, upscaling them; asking the shell for the exact display size picks a real frame. See 4.5.4. | [unverified], design reasoning |

### 2.9 Cross-document items (this document versus the UI document)

| # | Item | UI document says | This document says | Resolution needed |
|---|---|---|---|---|
| XD-1 | Status `unsupported` | 4.4 maps seven `WidgetStatus` values; `unsupported` is absent. `src/widgets/sdk/types.ts:3` has no `unsupported` either. | Native providers already return `unsupported` (`widget_data.rs:91-107`, NATIVE_PROVIDERS.md). | UI adds `unsupported` as the `unavailable` state with the message as tooltip (U0.3). New backend providers use `empty` for "no hardware" and `unsupported` only for "API missing on this build". |
| XD-2 | Icon request parameter | 4.5: the UI asks native for `px` from the ladder 32, 48, 64, 96, 128, 256 computed from the device pixel ratio; native returns a PNG at exactly that size, never upscaled. U1.4 changes `running_apps.rs` and `native.ts`. | The same parameter, named `iconPx`, is accepted by `installed_apps_snapshot` and `pinned_apps_snapshot`; native validates it against the ladder. | None. BE-13 must be sequenced with U1.4 because both edit `running_apps.rs`. |
| XD-3 | `chatgpt` versus `codex` | 5.3: `codex` is new; `chatgpt` stays as "More widgets" ("check `ai_usage.rs` before merging"). | Checked: `data.chatgpt` is built from the newest local Codex rollout log (`ai_usage.rs:269-293`), and `data.agent` from the same file. A `codex` tile and a `chatgpt` tile would show identical numbers. | Recommend reading `chatgpt` as an alias of `codex` (BD-1). `agent` stays as a separate "More widgets" item: it shows workspace and task, not quota. |
| XD-4 | Layout v3 | 5.4 `DockLayoutV3`: `pinnedApps`, `profileNames`, `profileOrder`, width steps, in-document `layoutV2Backup`, unknown ids kept in `unknown[]`. | Adopted. Backend consequences: native validators accept any profile id and layout versions 1, 2 and 3 (BE-06, BE-07); `pinnedApps` holds native keys (C4); the `isLayout` key whitelist also lists `layoutV2Backup`; Rust adds a one-generation `.bak` copy of the whole file (BE-04). | Add `layoutV2Backup` and the key-grammar rules to U1.2 acceptance. |
| XD-5 | Weather default | not mentioned | `meta.ts:264` ships `weatherEnabled: true` with latitude and longitude of Ahmedabad and `location: 'Ahmedabad'`, so every user without a saved override sees another city's weather as live. Defaults equal to the stored value are not persisted (`layout.ts:66-72`), so flipping the default silently changes existing users. | U1.2 must materialize the old weather defaults into explicit settings for existing items before flipping the default to unconfigured (BD-8). |
| XD-6 | Search sub-label shows "the registered hotkey or none" (3.4) | needs a native source | `AppSettings.global_hotkey` plus the registration result; none is exposed. | BE-08 adds `provider_snapshot("hotkey")`. |
| XD-7 | Taskbar | row 7: recommends an AppBar work-area reservation and hiding the system taskbar as an explicit setting (user decision) | Today the host never hides Explorer's taskbar (4.6). Start, tray and notification flows depend on it. | BD-9, BE-70 and BE-71 (gated). |
| XD-8 | `system-tray` state | 4.3: "always live" | Not achievable (H4). | UI gives the widget `unavailable` or removes it until BD-5 lands. |
| XD-9 | File overlaps between plans | U4.1 edits `main.rs` (`widget_card_position`); U1.4 edits `running_apps.rs`, `native.ts`; U1.2 edits `layout.ts` | BE-06, BE-08, BE-14 touch `main.rs`; BE-13 touches `running_apps.rs`; BE-05 documents `native.ts` additions | Sequence by file (Section 9.2). |
| XD-10 | Toast HUD | D9: do not port; use an `aria-live` region | Consistent: the spec's "Connect" toasts that claim success are removed (Section 8). | None. |
| XD-11 | Provider subscription | not covered | `useFinalWidgetData.ts:357-364` subscribes to `system`, `media`, `volume`, `trash`, `known-folders` and `running-apps` and starts the AI and weather hooks for every profile, whether or not the profile has a widget that reads them. | A new UI task, before BE-11, makes each subscription conditional on the profile's enabled widgets (5.7 item 1). Added to 9.3. |

### 2.10 Confirmed correct in the inputs

The following claims were checked and hold: GSMTC through `GlobalSystemMediaTransportControlsSessionManager` (already live in `widget_data.rs:114`); `CallNtPowerInformation(SystemBatteryState)` exists with a signed `Rate` in milliwatts (`windows` 0.58 has `SYSTEM_BATTERY_STATE`, [crate]); `WlanOpenHandle`, `WlanEnumInterfaces`, `WlanGetAvailableNetworkList`, `WlanConnect`, `WlanDisconnect` exist (already used or present in the crate); `UserNotificationListener.GetNotificationsAsync` and `RemoveNotification` exist; `Windows.Media.Ocr.OcrEngine` is the right OCR API and is already used; `IShellItemImageFactory::GetImage` exists and is the right API for AppsFolder items; Open-Meteo is already the weather source; bundled SQLite with FTS5 is already linked (E22); the ICS-based calendar is the realistic offline route; and the principle that user-tier widgets default to unconfigured is the same as the contract.

## 3. Widget catalogue decision

### 3.1 Decision

Adopt the UI document's catalogue (5.3 there): the 19 ids of `dock-editable.html` become the canonical set, the existing widgets keep working under their current ids, and nothing is dropped. The product owner's original vision includes volume, so `volume` stays a first-class optional widget. Saved layouts keep every id the user ever had: legacy ids are read forever as aliases, writers emit canonical ids, and an id that matches nothing is preserved in `unknown` and never deleted.

### 3.2 Catalogue table

Decision values: keep, rename (legacy id stays a read alias), new, defer, gated (needs a decision in Section 10).

| Canonical id | Legacy id | Decision | Backend source in v1 | Packages | Note |
|---|---|---|---|---|---|
| `start` | `windows` | rename | injected Win key tap | BE-14 | Tile is always live. |
| `search` | `search` | keep | engine search | BE-08, BE-40 to BE-42 | Extras are optional. |
| `running-apps` | `apps` | rename | engine window list and `running_apps.rs` | BE-13 | Snapshot is per window; the frontend groups by `key`. |
| `pinned-apps` | none | new | engine app enumeration plus shell icons | BE-34 to BE-36 | Pin list is `pinnedApps` in the layout. |
| `system-stats` | `system` | rename | `system` and `storage` providers | none | Battery, Wi-Fi, Bluetooth, temperature and uptime leave this tile for their own widgets. |
| `calendar` | none (the legacy SDK `calendar` is not mounted) | new | tile: local date; card: picker-granted ICS | BE-07, BE-39 | Windows Calendar store is blocked (4.4.6). |
| `peripherals` | none | new | `bluetooth` provider | BE-30 | One battery level per device; no ANC. |
| `music-player` | `spotify` | rename | `media` provider plus artwork | BE-15, BE-16 | Source-agnostic: any GSMTC session, not a Spotify brand. |
| `weather` | `weather` | keep | `weather_snapshot` | BE-18, BE-19 | Default location fix is XD-5. |
| `status-battery` | none | new | `battery` provider | BE-11 | |
| `wifi` | none | new | `wifi`, `wifi-networks` | BE-10, BE-32, BE-33 | Needs precise-location consent on current Windows 11 builds (4.3 item 2). |
| `bluetooth` | none | new | `bluetooth` provider | BE-30, BE-31 | |
| `notifications` | none | new, opt-in | `notifications`, `notifications_list_snapshot` | BE-37, BE-38 | Only after the spike passes. |
| `quick-notes` | none | new | engine notes folder | BE-20 | |
| `claude` | `claude` | keep | `ai_usage_snapshot` | BE-12 | Tokens only, no percentages. |
| `codex` | `chatgpt` (alias, BD-1) | new | `ai_usage_snapshot.data.chatgpt` and `data.agent.status` | BE-12 | Same Codex rollout source as `chatgpt`. |
| `assistant` | `siri-ai` | new, gated | none | none | BD-6; unconfigured tile only. |
| `system-tray` | none | new, gated | none in v1 | BE-63 or BE-64 | BD-5; the specified enumeration does not exist. |
| `clock` | `clock` | keep | none (browser clock) | none | |
| `agent` | `agent` | keep ("More widgets") | `ai_usage_snapshot.data.agent` | none | Workspace, task and session metrics from the same logs. |
| `files` | `files` | keep ("More widgets") | `known-folders` provider | none | |
| `folders` | `folders` | keep ("More widgets") | `known-folders` and `folder:open:<id>` | none | |
| `trash` | `trash` | keep ("More widgets") | `trash` provider and `trash:empty` | none | The UI confirms before the irreversible action. |
| `volume` | `volume` | keep ("More widgets", product owner vision) | `volume` provider and `volume:*` actions | none | Stays; the UI document gives it the generic tile until a design arrives. |
| clipboard | legacy SDK `clipboard` | defer | `clipboard_snapshot` exists | BE-65 | The engine table is only filled by the legacy host. |
| focus timer | legacy SDK `focus` | defer | none (browser timer) | none | Not mounted in the final dock. |
| other legacy SDK ids | `reference-widgets.json` (controls, network, media, disk, energy, time, analog, git, notes, downloads, launcher, activity, status, revenue, sales, followers, visitors, gmail, ai) | leave untouched | not mounted by `DockSurface` | none | Never mapped, never deleted. |

### 3.3 Read aliases and layout migration

Alias table (applied while reading any layout version, then de-duplicated keeping the first occurrence, with that item's size and settings): `windows` to `start`; `apps` to `running-apps`; `system` to `system-stats`; `spotify` to `music-player`; `siri-ai` to `assistant`; `chatgpt` to `codex` (only if BD-1 is accepted). The existing v1 map (`time` to `clock`, `status` to `system`, `media` to `spotify`, `protonsearch` to `search`, `layout.ts:40-45`) composes with it, so `status` ends as `system-stats`.

Rules (the layout shape is the UI document 5.4; these are the backend-relevant constraints):

1. Readers accept versions 1, 2 and 3. The writer emits version 3 only. A reader that meets a version it does not know shows defaults and blocks saving, which `DockShell.tsx:60-67` already does; this protects a v3 file from an older build.
2. Items whose id matches nothing (after aliasing) move to `unknown[]` as today. The UI document keeps this behaviour. Position, size and settings of such an item are lost, but its id is kept. BD-7 offers an optional stronger rule (keep the raw item in place); the default follows the UI document.
3. Settings are carried by key name. Keys the new widget does not declare are dropped by `settingsFor`, as today.
4. Before the first v3 write the original v2 JSON is stored once inside the document as `layoutV2Backup` (UI document 5.4) and never overwritten. Independently, the native save keeps one previous good copy as `protondock-layout.json.bak` (BE-04), which protects against a corrupt or truncated write of any version.
5. Calendar grants are untouched: `calendar-sources.json` is keyed by profile id and its tokens stay valid. Built-in profile ids do not change, and renaming a custom profile never changes its id (display names live in `profileNames`).
6. The weather widget's old defaults are materialized into explicit settings for any existing `weather` item before the default is flipped (XD-5, BD-8). Defaults equal to the stored value are not persisted (`layout.ts:66-72`), so skipping this step would change existing users silently.
7. `pinnedApps` stores native keys (`app-` and 16 hex digits). A key that no longer resolves is shown as missing; it is never deleted automatically.

### 3.4 Native allowlists that change

`WIDGET_CARD_IDS` (`main.rs:73`) is the Rust allowlist of widget card ids and must accept the new set or `open_widget_card` returns "Invalid widget card id" for every new widget. The exact new array (BE-06) is the 24 canonical ids `start, search, running-apps, pinned-apps, system-stats, calendar, peripherals, music-player, weather, status-battery, wifi, bluetooth, notifications, quick-notes, claude, codex, assistant, system-tray, clock, agent, files, folders, trash, volume` followed by the six legacy aliases `windows, apps, system, spotify, siri-ai, chatgpt`. `protonsearch/src/weather.rs:16-22` (`allowed_profile`) and `src-tauri/src/local_calendar.rs:37-43` (`profile_name`) hard-code profile ids and a layout version; both change to the grammar and the version set in 3.3 (BE-06, BE-07).

## 4. Per-widget feasibility audit

### 4.1 Method and verdicts

Each widget is checked against the actual API, the `windows` 0.58 crate that the repo already pins ([crate]: every feature name and function named below was found in the local crate source), the permission and identity model, and what the host does when the API is unavailable. Verdicts:

| Verdict | Meaning |
|---|---|
| Feasible | Works unpackaged and unelevated with documented or observed behaviour. |
| Feasible with caveat | Works with a named limit, a consent step, or an unverified step. A package marked SPIKE starts with an `#[ignore]` test whose result is recorded before any UI depends on it. |
| Blocked | No supported API, a restricted capability, a kernel driver, or a policy risk we will not take. |

### 4.2 Summary matrix

| Widget | Native source | Verdict | Deciding caveat |
|---|---|---|---|
| `start` | `SendInput` Win key tap | Feasible with caveat | Blocked silently by UIPI when an elevated window is foreground. |
| `search` | engine | Feasible | Live today; preview and actions are small additions. |
| `running-apps` | engine plus `running_apps.rs` | Feasible | Per-app badges are Blocked (toast counts at most). |
| `pinned-apps` | AppsFolder enumeration, `IShellItemImageFactory` | Feasible | Alpha handling of shell bitmaps is [unverified]. |
| `system-stats` | `GetSystemTimes`, `GlobalMemoryStatusEx`, `GetDiskFreeSpaceExW` | Feasible | SSD means used capacity, not I/O. |
| `calendar` | local date; picker-granted ICS | Feasible with caveat | One-off events only; Windows Calendar store Blocked. |
| `peripherals` | device property for battery | Feasible with caveat | One level per device; left/right/case needs reverse-engineered adverts; ANC Blocked. |
| `bluetooth` | device nodes; once-sampled radio state | Feasible with caveat | Radio state is stale after a toggle; toggle and pairing are not in v1. |
| `wifi` | WLAN API | Feasible with caveat | Needs precise-location consent on current Windows 11 builds; new secured networks and radio toggle Blocked. |
| `status-battery` | WinRT battery report | Feasible | Cycle count, voltage and temperature are device-dependent. |
| `notifications` | `UserNotificationListener` | Feasible with caveat (SPIKE) | Documented as a packaged capability; no change event unpackaged. |
| `quick-notes` | engine notes folder | Feasible | Plain files; filename sanitization is the risk. |
| `music-player` | GSMTC | Feasible | Artwork needs a decode step. |
| `claude` | local transcripts | Feasible with caveat | Tokens only; quota percentages Blocked. |
| `codex` | local rollout logs | Feasible | Real 5 hour and weekly percentages. |
| `assistant` | none | Blocked (undefined) | Product definition missing (BD-6). |
| `system-tray` | none in v1 | Blocked as specified | The specified enumeration path does not exist on this build. |
| `weather` | Open-Meteo | Feasible | City search and "use my location" are new. |
| `clock` | browser | Feasible | No native work. |
| `volume`, `trash`, `files`, `folders`, `agent` | existing providers | Feasible | Live today. |

### 4.3 Platform facts that apply to several widgets

1. Package identity. Microsoft lists features that need it (background tasks, push notifications, Windows AI APIs, others [doc E9]); notification listening and the Windows Calendar store depend on manifest capabilities (`userNotificationListener`, `appointmentsSystem`) [doc E8, E10]. ProtonDock is unpackaged (NSIS installer, `tauri.conf.json`). Getting identity would mean a sparse package with an external location plus code signing; that is a separate program decision and is not assumed anywhere in this document.
2. Location consent is shared. On current Windows 11 builds (Microsoft's page dates the change to an Insider release in October 2023 followed by a feature release; this machine is build 26300) the Wi-Fi scan and query APIs return `ERROR_ACCESS_DENIED` without precise-location consent [doc E7], and `Geolocator` needs the same consent. One missing toggle ("Let desktop apps access your location") therefore degrades Wi-Fi and "use my location" together. Both widgets must say so and offer `settings:location` (`ms-settings:privacy-location`).
3. No administrator rights are needed by any package in Sections 9 and 10, and none may be introduced.
4. Windows UIPI blocks injected input into higher-integrity windows without reporting failure; `SendInput` results are therefore never treated as proof that an effect happened.

### 4.4 Widget detail

#### 4.4.1 `start`

| Field | Detail |
|---|---|
| API | `SendInput` with two `INPUT` structures (`VK_LWIN` key down, `VK_LWIN` key up) in one call. Fallback: the existing `protonsearch::launcher::launch("shell:AppsFolder")`, which opens a folder window and is labelled a fallback in `main.rs:447`. |
| Crate and features | `windows` 0.58 `Win32_UI_Input_KeyboardAndMouse` (absent from `src-tauri/Cargo.toml`; present in the engine). |
| Permissions | None; no admin; no identity. UIPI applies (4.3 item 4). |
| Reliability | High while Explorer's shell runs. A Win key tap toggles Start, so a second click closes it. Start's position follows Explorer's taskbar (4.6). Not used when the user holds Alt, Ctrl or Shift (the UI's Alt-click morph path never calls it). |
| Rejected options | `shell:::{2559a1f8-21d7-11d4-bdaf-00c04f60b9f0}` is Windows Search (H1); `WM_SYSCOMMAND` `SC_TASKLIST` is unverified on Windows 11 (H2); `shell:AppsFolder` is not Start. |
| Honest fallback | If `SendInput` inserts fewer than two events, use the Apps folder fallback and return success only for the path that ran; never claim Start opened when it did not. |
| Model | Click only. No polling. Provider `none`. |
| Verdict | Feasible with caveat. Native check required (BE-14). |

#### 4.4.2 `search`

| Field | Detail |
|---|---|
| API | Existing: `search`, `execute_search_result`, `open_search`, `hide_search`, `fit_search`. Calculator, saved snippets, quicklinks, windows, notes and clipboard results already come from the engine. |
| Reliability | Live; the handoff records a real query and Enter launching Notepad (`docs/HANDOFF.md`). |
| Crate and permissions | No new crate or feature. No permission, no admin, no package identity. |
| Honest fallback | Engine not ready: `Err("Search engine not ready")`; the Spotlight shows its own error state. |
| Model | On demand; no polling. |
| Additions | `search_preview({resultId})` (BE-41), `reveal` and `copy-path` actions (BE-40), `search_index_status()` (BE-42), `provider_snapshot("hotkey")` and non-fatal hotkey registration (BE-08). |
| OCR and semantic | OCR is already indexed into `files_fts.content` (`indexer.rs:42-51`, `1354-1480`). Embeddings are deliberately removed (4.5.5). MCP is excluded (4.5.3). |
| Verdict | Feasible (live today). |

#### 4.4.3 `running-apps`

| Field | Detail |
|---|---|
| API | Window list from the engine (`search_windows("")`, launch command `window:<hwnd>`), identity and issued ids in `running_apps.rs` (hwnd, pid, process creation time, title, class; at most 12 windows), `SetForegroundWindow` on activation. |
| Crate and permissions | Already enabled: `Win32_UI_WindowsAndMessaging`, `Win32_System_Threading`, `Win32_Graphics_Gdi`. No admin, no identity. Processes that refuse limited query access may be omitted (CONTRACT.md). |
| Reliability | High. Identity is rechecked before activation. Icons follow the peer ledger item 2 (landed `a3d9e11`, native unverified). |
| Additions | `key` per item, stable across restarts, so the frontend can persist user order (BE-13). Poll every 2 s while visible (today 5 s). |
| Badges | Per-app red numbers are Blocked (H5). At most a count of toasts per application id after BE-38, labelled as notifications, hidden at zero. |
| Honest fallback | No windows: `empty`; the widget renders nothing (UI document 4.3). |
| Verdict | Feasible. |

#### 4.4.4 `pinned-apps`

| Field | Detail |
|---|---|
| API | Catalog: the engine's AppsFolder enumeration (`SHGetKnownFolderItem(FOLDERID_AppsFolder)`, `IEnumShellItems`, display name and parsing name), exposed by changing `installed_apps()` from `pub(crate)` to `pub` (`protonsearch/src/search.rs:6995`). Icons: `SHCreateItemFromParsingName` on the `shell:AppsFolder\<parsing name>` string, then `IShellItemImageFactory::GetImage`. Launch: `protonsearch::launcher::launch` with the same `shell:AppsFolder\<parsing name>` string the engine already issues (`search.rs:3575`). |
| Crate and features | Already present: `Win32_UI_Shell`, `Win32_System_Com`, `Win32_Graphics_Gdi`. `image` 0.24 with `png` already linked. |
| Threading | COM shell objects need a single-threaded apartment. The enumeration and icon rendering run on a short-lived dedicated thread (the engine's scan creates its own STA with a guard, `search.rs:7000-7020`). |
| Keys | The renderer sees `key = "app-" + 16 hex digits` of an FNV-1a 64-bit hash of the lowercase parsing name. Parsing names (paths, `{GUID}\...`, app user model ids) never leave Rust. `app:launch:<key>` resolves the key against the current catalog and fails with "App no longer installed" otherwise. |
| Reliability | High for enumeration (the engine has an enumeration test). Packaged-app icons come on their tile plate; the bitmap's alpha handling (premultiplied or not) must be observed natively before the un-premultiply step is trusted [unverified]. |
| Model | The catalog is cached natively for 5 minutes and is never polled. `pinned_apps_snapshot` runs when `pinnedApps` changes and when icons are re-requested for a new pixel ratio. No admin, no identity. |
| Running dot on pinned tiles | Not in v1. Pinned keys hash a parsing name while running keys hash an executable path, so they do not match. Matching needs the target path or app user model id of each item; deferred. |
| Verdict | Feasible. |

#### 4.4.5 `system-stats`

| Field | Detail |
|---|---|
| API | Existing: `GetSystemTimes` (CPU, first sample `null`), `GlobalMemoryStatusEx` (RAM), `GetDiskFreeSpaceExW` on `%SystemDrive%` through `storage`. |
| Rejected | The `sysinfo` crate and a 2000 ms thread (H9). Disk I/O activity (spec "primary physical disk I/O") is not provided; "SSD 43%" is used capacity. If activity is wanted later the route is `PdhAddCounterW` with `\PhysicalDisk(_Total)\% Disk Time` (`Win32_System_Performance` exists, [crate]); it is not recommended now. |
| History and peak | A frontend ring buffer of the last samples; resets with the webview. Temperature is always `null` (NATIVE_PROVIDERS.md). |
| Crate and permissions | No new crate or feature. No permission, no admin, no identity. |
| Model | `system` every 5000 ms and `storage` every 30000 ms while visible. |
| Verdict | Feasible (live today). |

#### 4.4.6 `calendar`

| Field | Detail |
|---|---|
| API | The tile is the browser's local date. The card uses the existing picker-granted `.ics` provider (`calendar_snapshot`, one-off events only, at most three events from at most 1000, source basename only). Week data (a seven-day strip and today's events) is added by BE-39. |
| Crate and permissions | `chrono` (already a dependency) for dates and the existing COM file picker (`IFileOpenDialog`, already used). No admin, no identity. |
| Model | `calendar_snapshot` every 30000 ms while visible (existing); the date tile needs no polling. Honest states: `disconnected` (no source), `empty`, `permission-denied` (grant missing for the active profile). |
| Blocked sources | Windows Calendar store: `AppointmentManager.RequestStoreAsync` requires the `appointmentsSystem` capability [doc E10], a restricted manifest capability. Classic Outlook COM and remote accounts (Graph, Google) are excluded; CONTRACT.md says no remote accounts and no writes exist. |
| Limits | No recurrence rules, no time-zone ids, no durations. Most real exports (Google, Outlook) contain recurring events, so the card will often show `empty` or an explicit error. BE-66 (gated) adds a recurrence subset. |
| Repo defect | `local_calendar.rs:64-79` accepts only layout version 1 and the profiles `daily` and `founder`; against today's version 2 file every calendar command fails with "Native dock profile could not be validated". BE-07 fixes this first. |
| Verdict | Tile Feasible. Events Feasible with caveat. Windows Calendar store Blocked. |

#### 4.4.7 `peripherals`

| Field | Detail |
|---|---|
| Sources | (a) The device property `{104EA319-6EE2-4701-BD47-8DDBF425BBE5}, 2` on present Bluetooth device nodes, the value Windows Settings shows [E19]; observed on 2 of 31 Bluetooth-class nodes on this machine [obs E5]. Read with `SetupDiGetClassDevsW(GUID_DEVCLASS_BLUETOOTH, DIGCF_PRESENT)` and `SetupDiGetDevicePropertyW`. (b) BLE advertisement manufacturer data, company id `0x004C`, Apple proximity pairing: left, right and case levels in 10 percent steps with charging flags [E14]. (c) GATT Battery Service `0x180F` on BLE peripherals (`GattDeviceService`). (d) Apple's AACP protocol over Classic L2CAP PSM `0x1001` for noise control. |
| Crate and features | (a) `Win32_Devices_DeviceAndDriverInstallation`, `Win32_Devices_Properties`. (b) `Devices_Bluetooth_Advertisement`, `Devices_Bluetooth` (gated). (c) `Devices_Bluetooth_GenericAttributeProfile` (not recommended). |
| Permissions | (a) none. (b) needs a BLE-capable radio; no consent prompt for desktop apps is documented [unverified]. (c) may be refused when another driver holds the service. (d) none exists. |
| Reliability | (a) High for devices whose stack publishes a level (not all do). (b) Fair: advertisements use rotating random addresses, so they cannot be tied to a paired device except by model and signal strength, and a scan also sees strangers' earbuds; Apple can change the format. (c) Device dependent and wakes the device. (d) Not possible: the community states a kernel-level driver is needed to open Classic L2CAP on Windows [E14]; Windows has no public user-mode BR/EDR L2CAP API. |
| Honest fallback | No device with a level: `empty` with reason "No device reports battery". Never a default or last-seen number labelled live. |
| Model | Poll `bluetooth` every 30 s (native cache 10 s). (b) only while the card is open, in 5 s scan windows. |
| Verdict | (a) Feasible. (b) Feasible with caveat, gated (BD-4). (c) not in v1. (d) Blocked; `set_anc_mode` and all ANC UI are dropped. |

#### 4.4.8 `bluetooth`

| Field | Detail |
|---|---|
| API | Radio state: the existing once-per-process `Radio::GetRadiosAsync` sample (`widget_data.rs:637-673`); repeated reads crash under windows 0.58 [repo]. Devices: 4.4.7 (a). Pairing: the existing `settings:bluetooth` action (`ms-settings:bluetooth`). |
| Crate and permissions | `Devices_Radios` (present) and the SetupAPI features of 4.4.7. No admin and no identity for reads; a toggle would need the `radios` capability. |
| Model | Poll `bluetooth` every 30 s with a 10 s native cache; the radio itself is sampled once per process. |
| Toggle | `Radio::SetStateAsync` requires the `radios` capability and a `RequestAccessAsync` flow [doc E11]; behaviour for unpackaged apps is [unverified] and the repeated-read crash is unexplained. Not in v1. |
| Reliability | The radio state does not follow a toggle made after the first sample (documented limitation). BE-31 is a spike that tries event-driven `StateChanged` on a dedicated thread to lift it. |
| Honest fallback | Radio unknown: `radio.enabled` is `null`. No radio: `empty`. |
| Verdict | Read Feasible with caveat. Toggle and in-app pairing are out of v1. |

#### 4.4.9 `wifi`

| Field | Detail |
|---|---|
| API | Existing: `WlanOpenHandle`, `WlanEnumInterfaces`, `WlanQueryInterface(current_connection)`. New: `WlanGetNetworkBssList` (centre frequency and RSSI of the connected BSS), `WlanGetAvailableNetworkList` (cached scan results with `has profile` and `connected` flags), `WlanScan`, `WlanConnect` (existing profile by name), `WlanDisconnect`; `GetAdaptersAddresses` for the IPv4 address; the `dot11PhyType` of the association for the Wi-Fi generation label. |
| Crate and features | `Win32_NetworkManagement_WiFi`, `Win32_NetworkManagement_IpHelper`, `Win32_Networking_WinSock` are already enabled. |
| Permissions | Precise-location consent is required on current Windows 11 builds for `WlanGetAvailableNetworkList`, `WlanGetNetworkBssList`, `WlanQueryInterface` and `WlanScan` [doc E7]. Consent is granted on this machine, so the denied branch could not be observed here [obs E6]. No admin. |
| Repo gap | `current_wifi_connection` returns `None` for every failure (`widget_data.rs:577-585`), so an access-denied query reads as `disconnected` even though `WlanEnumInterfaces` reports the interface connected. BE-10 reports connected with no name and sets `accessDenied`. |
| Rejected | `WlanSetProfile` with a passphrase: all-user profiles need elevation, a per-user profile pushes the secret through the renderer, and WPA3 or enterprise need more (G6). Radio toggle: `Radio::SetStateAsync` [doc E11] or `WlanSetInterface` with the radio-state opcode (permissions [unverified]); both out of v1. `bssid` is never returned (C10). |
| Honest fallback | `permission-denied` with action `settings:location`; no adapter: `unsupported`; networks needing a profile open `settings:wifi` (`ms-settings:network-wifi`, new fixed route). Connect results are never reported as success; the next poll shows the state. |
| Model | `wifi` every 10 s (native cache 3 s). `wifi-networks` only while the card is open, native cache 10 s. `wifi:scan` is rate limited to once per 10 s. |
| Verdict | Read Feasible with caveat. Connect to a known network Feasible with caveat. New secured network and radio toggle Blocked in v1 (BD-3). |

#### 4.4.10 `status-battery`

| Field | Detail |
|---|---|
| API | `Windows.Devices.Power.Battery::AggregateBattery().GetReport()` gives status (`NotPresent`, `Discharging`, `Idle`, `Charging`), `ChargeRateInMilliwatts` (signed), `DesignCapacityInMilliwattHours`, `FullChargeCapacityInMilliwattHours`, `RemainingCapacityInMilliwattHours`. Observed working from an unpackaged host here: status Idle, rate 0 mW, full 83002, design 83002, remaining 70152 mWh [obs E4]. `GetSystemPowerStatus` adds the AC line, percent and `BatteryLifeTime`. |
| Optional extras | Cycle count, voltage and temperature need the battery device IOCTLs (`IOCTL_BATTERY_QUERY_INFORMATION` with `BatteryInformation` and `BatteryTemperature`, `IOCTL_BATTERY_QUERY_STATUS`) through SetupAPI `GUID_DEVICE_BATTERY`; the constants exist in the crate. Many laptops return zero or an error; the value is then `null`. Optional package BE-21. |
| Rejected | WMI `Win32_Battery` (H3); `CallNtPowerInformation(SystemBatteryState)` is redundant and its `Rate` can be the unknown marker; `WM_POWERBROADCAST` needs a message window the host does not own. |
| Crate and features | `Devices_Power` and `System_Power` (add; `BatteryReport::Status` is gated by `System_Power` in the crate [crate]), `Win32_System_Power` (present). No admin, no capability. |
| Honest fallback | No battery: `empty`, message "No battery detected", `data: null`. Unknown rate or capacity: `null`. Health is full over design only when both are known; this machine reports full equal to design, so 100 percent here may be a firmware default. Time to full is an estimate derived natively and labelled as such. |
| Model | Poll 30 s, native cache 5 s. No event. |
| Verdict | Feasible. |

#### 4.4.11 `notifications`

| Field | Detail |
|---|---|
| API | `UserNotificationListener::Current()`, `GetAccessStatus()`, `GetNotificationsAsync(NotificationKinds::Toast)`, `RemoveNotification(id)`, `ClearNotifications()`. Only toast notifications are supported [doc E8]. |
| Crate and features | `UI_Notifications_Management`, `UI_Notifications`, `ApplicationModel` (add in BE-37; `UserNotification::AppInfo` is gated by `ApplicationModel` [crate]). |
| Identity | The documentation requires the `userNotificationListener` capability in the app manifest [doc E8], which belongs to packaged apps [doc E9]. Observed here: an unpackaged `powershell.exe` reads `GetAccessStatus()` as `Allowed` [obs E3]. Community report: in an unpackaged app `RequestAccessAsync` returns Allowed and `GetNotificationsAsync` works, while subscribing to `NotificationChanged` throws `0x80070490` [E13]. Conclusion: polling only, no official support, unverified in the Tauri host. BE-37 is a spike that must pass first. |
| Privacy | This reads every application's toast text, including messages and one-time codes. Controls: the widget is opt-in and absent from default profiles; the dock tile polls a count only; `notifications_list_snapshot({showText})` returns title and body only when `showText` is true (default false) and only while the card is open; title is cut to 80 characters and body to 200; nothing is stored or logged. |
| Badges | Per-app badge numbers are Blocked (H5). |
| Honest fallback | Access not allowed: `permission-denied` with the existing `settings:notifications` action. |
| Model | `notifications` (count) every 5 s with a native cache of 2 s; the list only while the card is open. |
| Verdict | Feasible with caveat (SPIKE). |

#### 4.4.12 `quick-notes`

| Field | Detail |
|---|---|
| API | Plain `std::fs` over the engine's notes folder `%APPDATA%\protonsearch\notes` (BE-20). The engine already lists these `.txt` files for the `notes:` prefix and the "Create Note" quick action. |
| Risk | File names come from user text: the sanitizer in BE-20 is the security-relevant code. |
| Crate and permissions | `std::fs` only. No admin, no identity. |
| Honest fallback | No notes: `empty` ("No notes yet"). Unwritable folder: "Could not save the note". |
| Model | On demand. After a save or delete the UI calls `refreshProvider('notes')`. |
| Verdict | Feasible. |

#### 4.4.13 `music-player`

| Field | Detail |
|---|---|
| API | Existing GSMTC provider with transport actions. Add `artworkId` and one `media_artwork()` command: `TryGetMediaPropertiesAsync().Thumbnail()`, `OpenReadAsync`, bounded read, decode, square crop, resize to 96 px, PNG. |
| Crate and features | `Storage_Streams` (add); `image` gains the `jpeg` feature because thumbnails are often JPEG (`Cargo.toml:14` has `png` only). |
| Permissions | None. GSMTC works from the unpackaged host (the media provider is live today). No admin. |
| Honest fallback | Nothing playing: `empty` ("Nothing playing"); no artwork: `null`, the UI draws its neutral glyph. |
| Model | Poll 2 s as today; the UI extrapolates position from `playbackRate`. |
| Verdict | Feasible. |

#### 4.4.14 `claude` and `codex`

| Source | Returns | Auth | Quota percent | Use |
|---|---|---|---|---|
| Codex rollout logs `~/.codex/sessions/**/rollout-*.jsonl`, `token_count.rate_limits.primary` and `.secondary` | used percent, reset time, plan type, cumulative tokens | none (local file read) | yes: the 5 hour and weekly windows as Codex sees them | Used today, keep (`ai_usage.rs:152-212`, `269-293`) |
| Claude Code transcripts `~/.claude/projects/**/*.jsonl`, assistant `usage` | per-message token counts | none | no | Used today; tokens only (`ai_usage.rs:313-364`) |
| Anthropic Usage and Cost API (`/v1/organizations/usage_report/messages`, `/v1/organizations/cost_report`) | tokens and dollar cost per time bucket | Admin key `sk-ant-admin...`, organization accounts only [E15] | no, spend only | Not in v1 |
| Anthropic response headers `anthropic-ratelimit-*` | per-minute API limits of a key | any key, requires a billed request | API limits, not subscription | Not used |
| `GET https://api.anthropic.com/api/oauth/usage` | the 5 hour and 7 day utilisation that Claude Code `/usage` shows | the OAuth token Claude Code stores | yes | Not used: undocumented, aggressively rate limited, and reading another application's OAuth token is a security and policy risk; reports say Anthropic blocked third-party use of Claude Code subscription tokens [E16] |
| OpenAI Usage and Costs API (`/v1/organization/usage/completions`, `/v1/organization/costs`) | tokens and cost | Admin key [E17] | no | Not used |

Consequences: `codex` is live today with real percentages and needs no key. `claude` can show tokens used only; its session and weekly percentages stay `null`. The "Connect account" setup card of the HTML has nothing to connect to in v1 and is replaced by a statement of what local sessions were found (UI work). "Working" (the halo) is real: the newest session file was modified within 300 seconds (`LIVE_WINDOW_SECONDS`, `ai_usage.rs:11`), added for Claude by BE-12. `parse_rollout` reads the whole newest log on every 60 s call; acceptable now, and a head-and-tail read is the upgrade if logs grow large (not scheduled). Permissions: none (local file reads, no network, no stored credential); no admin; no identity. Model: `ai_usage_snapshot` every 60000 ms while visible. Verdicts: `codex` Feasible; `claude` Feasible with caveat; percentages Blocked.

#### 4.4.15 `assistant` (`siri-ai`)

| Field | Detail |
|---|---|
| What exists | The engine crate enables `Media_SpeechRecognition` and has an OpenAI-compatible chat client (`protonsearch/src/ai.rs`); the Tauri search window still rejects AI actions with "requires the legacy ProtonSearch launcher" (`main.rs:316-321`). Whisper.cpp would add a native build and a model download of well over 100 MB. |
| Problem | The spec's assistant (microphone, speech to text, language model, speech) has no agreed product definition, and a waveform or transcript would be fabricated unless real capture exists. |
| Verdict | Blocked pending BD-6. The backend adds nothing. The UI keeps an unconfigured tile (UI document D3). Microphone access for desktop apps is governed by Settings, Privacy and security, Microphone. |

#### 4.4.16 `system-tray`

| Field | Detail |
|---|---|
| Observed | On this machine (Windows 11 build 10.0.26300) `Shell_TrayWnd` is visible and its descendants are `Windows.UI.Composition.DesktopWindowContentBridge`, `Windows.UI.Input.InputSite.WindowClass`, `Windows.UI.Core.CoreWindow` (hidden), `Start` (hidden), `TrayDummySearchControl`, `TrayNotifyWnd`, `ReBarWindow32`, `MSTaskSwWClass`, `MSTaskListWClass` (hidden). `TrayNotifyWnd` has no children: there is no `SysPager` and no `ToolbarWindow32`, and no `NotifyIconOverflowWindow` [obs E2]. The specified path is dead. |
| Registry history | `HKCU\Control Panel\NotifyIconSettings` has 70 subkeys here [obs E2]: every application that ever registered a notify icon, with its executable path and a promoted flag, not which icons are live. |
| Replacement shells | Projects that host the tray themselves report inconsistent behaviour on Windows 11 (missing icons, system icons that do not respond) [E20]. |
| Options | A: invoke Windows' own hidden-icons flyout through UI Automation (find the "Show hidden icons" button in the taskbar tree and call `Invoke`): real icons, tooltips and menus, no re-implementation; needs Explorer's taskbar and stable element names [unverified]; spike BE-63. B: a "Background apps" list from the registry cross-checked against running processes, labelled honestly as background apps, not a tray replica; BE-64. C: become the tray host (register the `Shell_TrayWnd` class and answer `NIM_*` messages): replacement-shell work, out of scope. D: send Win+B then Enter: Enter would reach the wrong application if Explorer did not take focus, so it is rejected. |
| Verdict | Blocked as specified. Recommended: remove from default profiles; if kept, option A after its spike, else option B (BD-5). |

#### 4.4.17 `weather`

| Field | Detail |
|---|---|
| API | Existing: Open-Meteo forecast through `ureq` with redirects disabled, 8 s total timeout, 64 KiB response bound and a 10 minute success cache (`weather.rs:186-213`, `215-256`). New: Open-Meteo geocoding (`https://geocoding-api.open-meteo.com/v1/search`) for city search, same client rules (BE-18); `Geolocator` for "use my location" (BE-19). Rain probability needs the extra hourly parameter `precipitation_probability`. Air quality is a different Open-Meteo host and is deferred. |
| Location | `Geolocator::RequestAccessAsync` must run in the user's context outside `C:\Windows\System32` [doc E7]; `LocationStatus` reads `NotInitialized` in an unpackaged host before a request [obs E3]. Coordinates are rounded to two decimals (about 1 km) before they are stored or returned; they are never logged. IP geolocation is rejected: it sends the user's address to a third party. |
| Crate and permissions | `Devices_Geolocation` (BE-02) for "use my location"; `ureq` is already in the engine. Location is a Windows consent (4.3 item 2). No admin, no identity. |
| Model | `weather_snapshot` every 600000 ms (native cache 10 minutes after a success, 1 minute after a failure); geocode and locate only on a click. |
| Honest fallback | No location configured: `disconnected` and the tile shows `--`. Network failure: `stale` with the last real data and a message (existing cache). The Ahmedabad default is fixed per XD-5. |
| Verdict | Feasible. |

#### 4.4.18 `clock`

No native API, crate, permission, admin or identity requirement. The browser clock is the system clock and a 1000 ms browser timer drives it; `Intl` supplies zone and locale (`useFinalWidgetData.ts:198`). The optional "next event" uses `calendar_snapshot`. There is no unavailable state. Verdict: Feasible.

#### 4.4.19 `volume`, `trash`, `files`, `folders`, `agent`

All five are live through existing providers: Core Audio (`IAudioEndpointVolume`), `SHQueryRecycleBinW`, known-folder resolution with a two-level recent scan, and the local session logs. Verdict: Feasible. `trash:empty` is irreversible; the UI must confirm, and BE-51 (optional) adds a native confirmation so a compromised renderer cannot trigger it silently.

### 4.5 Cross-cutting checks

#### 4.5.1 Start menu invocation

See 4.4.1. `SendInput` of a Win key tap is the only supported way to ask the shell to toggle Start. The alternatives in the inputs are wrong or unverified (H1, H2). The global search hotkey `Alt+Space` is the Windows window system menu and is the default of other launchers; registration can fail (BE-08).

#### 4.5.2 Windows location

`Geolocator` is the supported API; `ISensorManager` is legacy (H7). The access path and its coupling with Wi-Fi are in 4.3 item 2 and 4.4.17.

#### 4.5.3 MCP server on `127.0.0.1:41782`

Excluded. A loopback listener is reachable by every local process and, without Host and Origin checks, by web pages through DNS rebinding; the specified tools read arbitrary files by path and expose the index (G1, G3). If an AI client integration is ever wanted it must be a separate opt-in program started by that client over standard input and output, read-only, with an allowlist of roots and size caps. It is not part of the dock.

#### 4.5.4 Icon extraction at the display's pixel ratio

The UI document 4.5 owns the request ladder and the DOM rules (XD-2). Backend facts that decide the implementation of BE-35 and complement the peer ledger (item 2): (a) The running-apps path stays as landed in `a3d9e11`. (b) For AppsFolder items ask `IShellItemImageFactory::GetImage` for the exact requested `iconPx` with `SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK` and never `SIIGBF_SCALEUP`, which upscales small frames; if the returned bitmap is larger than requested, downscale with `image::imageops::resize` and `FilterType::Lanczos3`; if smaller, return it at its own size. (c) `SHIL_JUMBO` is not used: it returns 256 pixels even when the application ships only 48 pixel frames, which upscales them (H12). (d) The spec's unsharp mask (radius 0.5, amount 0.35) is unvalidated and is skipped; a `SHARPEN` constant stays `false` until a Computer Use comparison shows a visible gain. (e) Output is PNG RGBA8 with straight alpha (un-premultiply as `running_apps.rs:254-261` does). (f) GDI objects (`HDC`, `HBITMAP`, `HICON`) are owned by drop guards. (g) Results are cached per `(source key, px)` and cleared when the catalog refreshes.

#### 4.5.5 `fastembed` semantic search

Excluded. The engine removed embeddings deliberately for speed and size (`search.rs:4853`, comment "embeddings removed"); lexical ranking plus OCR content already covers the dock's needs. `fastembed` would add an ONNX runtime and a first-use model download. If concept search is wanted later it is an engine feature behind an opt-in setting with a measured relevance gap (the engine already has a hit-rate test harness), not a dock feature.

#### 4.5.6 Clipboard history

The engine owns `clipboard_history` (C2). The Tauri host reads it through `clipboard_snapshot` (previews hidden until the user enables them, at most three, bounded by the engine's 50-entry list). Ingestion exists only in the legacy host (`protonsearch/src/main.rs:1984`, `11388`, plus the import of Windows clipboard history). A clipboard widget in the final dock would show frozen history until ingestion is ported into the engine library (BE-65, gated, large, touches the legacy host).

#### 4.5.7 OCR and screenshots

Covered by the engine (H11). Nothing is built in the dock. The user-visible gap is only that an OCR text hit and a file-content hit share one `content` column, so the "OCR" filter pill can only be approximated by image extension at query time.

### 4.6 Taskbar replacement dependency

The program goal is to replace the Windows taskbar, but the Tauri host never hides Explorer's taskbar today: no `ShowWindow` or `SHAppBarMessage` call exists in `src-tauri/src` (`dock_lifecycle.rs:74` only reads class names, and a window-geometry test names the dock "centered above existing taskbar"). The legacy host did hide it (`show_taskbar` setting, `taskbar_shown_by_app`, `protonsearch/src/main.rs:4187`, `4390`) and left recovery code that the host still calls: `taskbar_recovery.rs`, the `--restore-taskbar` flag and the tray menu item "Restore Windows taskbar" (`main.rs:987-1000`, `1127-1144`).

Consequences for this plan: Start (4.4.1), the hidden-icons flyout (4.4.16, option A), toast flyouts and Win key shortcuts all rely on Explorer's shell. Hiding the taskbar window with `ShowWindow(SW_HIDE)` would break them; making it auto-hide (`SHAppBarMessage` with `ABM_SETSTATE` and `ABS_AUTOHIDE`, restorable) keeps the shell alive. The UI document (row 7) recommends reserving the work area with an AppBar (`ABM_NEW`, `ABM_QUERYPOS`, `ABM_SETPOS`) and hiding the system taskbar only as an explicit setting. Recommended default (BD-9): v1 leaves Explorer's taskbar visible and the dock above it; a later, user-approved step adds the AppBar (BE-70) and then the optional auto-hide with the existing lease and recovery path (BE-71). Both are gated and need native verification at 100, 125 and 150 percent.

## 5. Canonical IPC contract

### 5.1 Principles

1. Reads go through `provider_snapshot({providerId})`, which returns the envelope of 5.2. The three existing special routes (`clipboard_snapshot`, `running_apps_snapshot`, `known_folders_snapshot`) stay as they are; every new read provider is a new provider id, not a new command.
2. Writes go through `widget_action({actionId})` with the grammar of 5.5. A new action is a new grammar entry, not a new command.
3. A bespoke command is allowed only if it passes one of three tests: it needs a structured or free-text argument that cannot be a short opaque id; it returns a large or binary payload that must be fetched only on demand; or it must be callable by fewer windows than `provider_snapshot`. Section 5.4 states which test each one passes.
4. Opaque ids: every identifier the renderer sends back was issued by native code in an earlier snapshot and is looked up again in Rust. Paths, process ids, window handles, parsing names and Bluetooth addresses never reach the renderer.
5. Frontend metadata is not authorization (CONTRACT.md). Rust checks provider ids, action grammar, argument bounds and the calling window (capabilities, 5.8).
6. Every command returns `Result<T, String>`; the error string is a short sentence safe to show (5.9).
7. A new command is registered in exactly three places (`generate_handler!` in `main.rs`, `AppManifest::commands` in `build.rs`, a permission set in `permissions/commands.toml`), and a test enforces it (BE-03). Today `build.rs` omits five commands that `commands.toml` allows: `known_folders_snapshot`, `open_widget_card`, `hide_widget_card`, `widget_card_state`, `ai_usage_snapshot`.

### 5.2 Envelope and status vocabulary

`{status, data, updatedAt, message?}`; `updatedAt` is Unix milliseconds. Status values and their meaning for every provider:

| Status | Meaning | `data` |
|---|---|---|
| `live` | At least one real value was read. | present; unknown fields `null`, never `0` |
| `loading` | Needs a second comparable sample (for example throughput). | present or `null` |
| `empty` | The API worked and found no matching hardware, session or data. | `null` |
| `disconnected` | Not configured by the user (no weather location, no calendar source). | `null` |
| `permission-denied` | Windows consent or access is missing. | `null`, except where a partial real value is known (Wi-Fi connected without a name keeps `data`) |
| `unsupported` | The API is missing on this Windows build or host. | `null` |
| `stale` | Produced by the frontend scheduler when a refresh failed and an older real value exists. Native code never sends it. | last real data |
| `error` | Command-level failure (the IPC call threw). Produced by the scheduler. | last data or none |

The TypeScript `WidgetStatus` must include `unsupported` (XD-1).

### 5.3 Providers

Interval is the frontend subscription interval in milliseconds (never below 1000). Cache is the native time-to-live inside the provider, so a second window asking within it receives the cached envelope. "Today" providers are unchanged by this plan except where a package is named.

| Provider id | Route | `data` shape (5.6) | Statuses | Interval | Cache | Frontend permission label | Package |
|---|---|---|---|---|---|---|---|
| `system` | `provider_snapshot` | `{cpu, ram, battery, batteryCharging, temperatureC, uptimeSeconds, connectivity}` | live | 5000 | none | `system:read` | today |
| `storage` | `provider_snapshot` | `{totalBytes, freeBytes, mountPoint}` | live | 30000 | none | `storage:read` | today |
| `network` | `provider_snapshot` | existing, plus `wifi.accessDenied` | live, loading, empty | 5000 | none | `network:read` | BE-10 |
| `media` | `provider_snapshot` | existing plus `artworkId` | live, empty, permission-denied | 2000 | none | `media:read` | BE-16 |
| `volume` | `provider_snapshot` | `{level, muted, device}` | live, empty, permission-denied, unsupported | 5000 | none | `audio:read` | today |
| `trash` | `provider_snapshot` | `{itemCount, sizeBytes, lastEmptiedAt}` | live, permission-denied, unsupported | 60000 | none | `storage:read` | today |
| `known-folders` | `known_folders_snapshot` | `{folders, recent}` | live, empty, permission-denied | 60000 | none | `storage:read` | today |
| `clipboard` | `clipboard_snapshot` | `{availableCount, countIsLowerBound, items}` | live, empty | 30000 | none | `clipboard:read` | today |
| `running-apps` | `running_apps_snapshot` | `{items:[{id, key, title, icon, active}]}` | live, empty | 2000 (today 5000) | none | `running-apps:read` | BE-13 and U1.4 |
| `battery` | `provider_snapshot` | `BatteryData` | live, empty, unsupported | 30000 | 5 s | `battery:read` | BE-11 |
| `wifi` | `provider_snapshot` | `WifiData` | live, permission-denied, unsupported | 10000 | 3 s | `network:read` | BE-32 |
| `wifi-networks` | `provider_snapshot` | `WifiNetworksData` | live, empty, permission-denied, unsupported | 10000, only while the card is open | 10 s | `network:read` | BE-32 |
| `bluetooth` | `provider_snapshot` | `BluetoothData` | live, empty, unsupported | 30000 | 10 s | `bluetooth:read` | BE-30 |
| `notifications` | `provider_snapshot` | `NotificationsCountData` | live, permission-denied, unsupported | 5000 | 2 s | `notifications:read` | BE-38 |
| `notes` | `provider_snapshot` | `NotesData` | live, empty | on demand (after a save, `refreshProvider('notes')`) | none | `notes:read` | BE-20 |
| `hotkey` | `provider_snapshot` | `HotkeyData` | live | 60000 | none | none | BE-08 |

Reads that take arguments are commands (5.4): `weather_snapshot`, `ai_usage_snapshot` (today, no arguments, 60000), `calendar_snapshot` (today, 30000), `installed_apps_snapshot`, `pinned_apps_snapshot`, `notifications_list_snapshot`.

### 5.4 Commands

Existing commands that this plan does not change in name or arguments: `search`, `execute_search_result` (gains actions, 5.5), `open_search`, `hide_search`, `fit_search`, `fit_dock`, `open_start` (new body, BE-14), `open_settings`, `hide_settings`, `load_dock_settings`, `save_dock_settings` (hardened, BE-04), `provider_snapshot`, `known_folders_snapshot`, `open_widget_card`, `hide_widget_card`, `widget_card_state`, `widget_action`, `clipboard_snapshot`, `running_apps_snapshot` (gains `iconPx` in U1.4), `running_apps_activate`, `ai_usage_snapshot`, `weather_snapshot`, `calendar_select_source`, `calendar_snapshot`, `calendar_forget_source`.

New bespoke commands, with the test from 5.1 principle 3 that each passes:

| Command | Arguments | Returns | Errors | Windows | Test passed | Package |
|---|---|---|---|---|---|---|
| `media_artwork` | none | `MediaArtwork` or `null` | "Artwork unavailable" | dock, widget | on-demand binary payload; the UI calls it only when `artworkId` changes | BE-15, BE-16, BE-49 |
| `note_save` | `content`: 1 to 65536 UTF-8 bytes | `NoteSaved` | "Note is empty", "Note is too long", "Could not save the note" | widget | free-text argument | BE-20, BE-49 |
| `weather_geocode` | `query`: 1 to 80 characters after trimming | `GeocodeData` | "Enter at least 2 characters", "Place search is unavailable" | widget, settings | free-text argument plus outbound request | BE-18, BE-49 |
| `weather_locate` | none | `LocateResult` | none; denial is a result status | widget, settings | OS consent flow; location must not ride a polled provider | BE-19, BE-49 |
| `installed_apps_snapshot` | `query` (at most 80 characters), `limit` (1 to 60), `iconPx` | `Snapshot<InstalledAppsData>` | "Apps are not available yet" | settings | arguments and a large payload | BE-34, BE-35, BE-49 |
| `pinned_apps_snapshot` | `keys` (at most 24, each `app-` plus 16 hex digits), `iconPx` | `Snapshot<PinnedAppsData>` | "Invalid app key" | dock, widget, settings | arguments | BE-36, BE-49 |
| `notifications_list_snapshot` | `showText`: boolean | `Snapshot<NotificationsListData>` | none | widget | privacy-minimising argument; only the card holds text | BE-38, BE-49 |
| `search_preview` | `resultId` | `SearchPreview` | "Result expired; search again" | search | per-result bounded text | BE-41, BE-49 |
| `search_index_status` | none | `SearchIndexStatus` | none | search | engine static, not a widget provider | BE-42, BE-49 |

Gated, not in v1 (Section 10): `credential_status`, `credential_save`, `credential_delete` (settings window only, 6.8). Nothing else is added. In particular the following inputs are not commands: `get_*`, `set_*`, `toggle_*`, `kensaku_*`, `reveal_in_explorer`, `open_path`, `pair_bluetooth_device`, `connect_wifi_network`, `set_anc_mode`, `trigger_siri_assistant`.

### 5.5 `widget_action` grammar

Form `domain:verb` or `domain:verb:arg`. `arg` matches `[A-Za-z0-9._-]{1,64}` and is always an opaque token issued by a snapshot (or a fixed id for folders). The Rust side parses the string into an enum (rust-skills `api-parse-dont-validate`) and rejects anything else with "Unsupported widget action" before touching the OS. Unit tests are table driven and include malformed and injected inputs.

| Action | Effect | Source of `arg` | Package |
|---|---|---|---|
| `media:toggle`, `media:next`, `media:previous` | GSMTC transport | none | today |
| `volume:up`, `volume:down`, `volume:mute`, `volume:set:<0..100>` | default output endpoint | none | today |
| `trash:empty` | empties the Recycle Bin, irreversible | none | today (BE-51 optional native confirmation) |
| `folder:open:<id>` | opens a known folder | the six fixed ids | today |
| `settings:sound`, `settings:network`, `settings:bluetooth`, `settings:notifications` | opens a fixed Settings page | none | today |
| `settings:location` | opens `ms-settings:privacy-location` | none | BE-10 |
| `settings:wifi` | opens `ms-settings:network-wifi` | none | BE-33 |
| `wifi:scan` | `WlanScan`, at most once per 10 s | none | BE-33 |
| `wifi:connect:<token>` | `WlanConnect` to a saved profile only | `wifi-networks` token | BE-33 |
| `wifi:disconnect` | `WlanDisconnect` | none | BE-33 |
| `notifications:dismiss:<id>` | `RemoveNotification` | `notifications_list_snapshot` id | BE-38 |
| `notifications:clear` | `ClearNotifications` | none | BE-38 |
| `notes:delete:<id>` | deletes one note file | `notes` id | BE-20 |
| `app:launch:<key>` | launches a catalog app | `pinned_apps_snapshot` or `installed_apps_snapshot` key | BE-36 |

`execute_search_result({resultId, actionId})` accepts `actionId` in `open`, `reveal`, `copy-path` (BE-40); `reveal` and `copy-path` apply only to results whose stored command is an existing local path and otherwise return "This result has no file path".

### 5.6 TypeScript payload types

Keys are camelCase on the wire. `null` means unknown or unavailable and is never rendered as zero.

```ts
export type WidgetStatus = 'loading' | 'live' | 'empty' | 'stale' | 'disconnected'
  | 'permission-denied' | 'unsupported' | 'error';
export interface Snapshot<T = unknown> { status: WidgetStatus; data?: T | null; updatedAt: number; message?: string }

export type ProviderId =
  | 'none' | 'system' | 'storage' | 'network' | 'media' | 'clipboard' | 'running-apps'
  | 'volume' | 'trash' | 'known-folders'
  | 'battery' | 'wifi' | 'wifi-networks' | 'bluetooth' | 'notifications' | 'notes' | 'hotkey';

export type IconPx = 32 | 48 | 64 | 96 | 128 | 256;   // the UI ladder (UI document 4.5)

export interface BatteryData {
  state: 'charging' | 'discharging' | 'idle' | 'unknown';
  percent: number | null;                  // 0 to 100
  acConnected: boolean | null;
  chargeRateMilliwatts: number | null;     // signed, positive while charging
  remainingMilliwattHours: number | null;
  fullChargeMilliwattHours: number | null;
  designMilliwattHours: number | null;
  healthPercent: number | null;            // full over design, 0 to 100, null unless both are known
  secondsRemaining: number | null;         // discharging only, from Windows
  secondsToFull: number | null;            // charging only, an estimate
  cycleCount: number | null;               // BE-21 only
  voltageMillivolts: number | null;        // BE-21 only
  temperatureCelsius: number | null;       // BE-21 only
}

export interface WifiData {
  state: 'connected' | 'disconnected' | 'associating' | 'authenticating' | 'discovering'
    | 'ad_hoc' | 'disconnecting' | 'not_ready' | 'unsupported' | 'unknown';
  connected: boolean;
  accessDenied: boolean;                   // the OS withheld details for lack of location consent
  ssid: string | null;
  signalPercent: number | null;
  linkSpeedMbps: number | null;
  generation: 'wifi4' | 'wifi5' | 'wifi6' | 'wifi7' | 'legacy' | null;
  bandGhz: 2.4 | 5 | 6 | null;
  rssiDbm: number | null;
  ipv4: string | null;
}
export interface WifiNetworksData {
  networks: Array<{ token: string; ssid: string; signalPercent: number;
    secured: boolean; known: boolean; connected: boolean }>;   // at most 12, no BSSID
}

export interface BluetoothData {
  radio: { available: boolean; enabled: boolean | null; name: string | null };
  devices: Array<{ id: string; name: string; kind: 'audio' | 'input' | 'other';
    batteryPercent: number | null }>;      // id is 'bt-<n>', issued per snapshot
}

export interface NotificationsCountData { access: 'allowed' | 'denied' | 'unspecified'; count: number } // capped at 99
export interface NotificationsListData {
  access: 'allowed' | 'denied' | 'unspecified';
  items: Array<{ id: string; appName: string; title: string | null; body: string | null; timestampMs: number }>;
}                                          // at most 20; title and body are null unless showText is true

export interface NotesData {
  total: number;
  items: Array<{ id: string; title: string; preview: string; modifiedMs: number; chars: number }>; // at most 20
}
export interface NoteSaved { id: string; title: string }

export interface MediaArtwork { artworkId: string; dataUri: string }   // PNG, 96 by 96
// MediaData keeps every existing field and adds: artworkId: string | null

export interface RunningAppItem { id: string; key: string | null; title: string; icon: string | null; active: boolean }

export interface InstalledApp { id: string; name: string; icon: string | null }   // id is the app key
export interface InstalledAppsData { items: InstalledApp[]; total: number; truncated: boolean }
export interface PinnedAppsData { items: InstalledApp[]; missing: string[] }       // missing echoes unresolved keys

export interface GeocodePlace { name: string; admin1: string | null; country: string | null;
  latitude: number; longitude: number }
export interface GeocodeData { results: GeocodePlace[] }                           // at most 5
export interface LocateResult { status: 'ok' | 'permission-denied' | 'unavailable';
  latitude?: number; longitude?: number }                                          // two decimals

export interface HotkeyData { registered: string[]; failed: string[] }

export interface SearchPreview { kind: 'text' | 'none'; text: string | null;
  sizeBytes: number | null; modifiedMs: number | null }                            // text at most 2048 characters
export interface SearchIndexStatus { progress: string; fileCount: number | null }

// AiUsage: the existing { chatgpt, claude, agent } object; claude and chatgpt gain `working: boolean` (BE-12).

export type WidgetActionId =
  | 'media:toggle' | 'media:next' | 'media:previous'
  | 'volume:up' | 'volume:down' | 'volume:mute' | `volume:set:${number}`
  | 'trash:empty' | `folder:open:${string}`
  | 'settings:sound' | 'settings:network' | 'settings:bluetooth' | 'settings:notifications'
  | 'settings:location' | 'settings:wifi'
  | 'wifi:scan' | 'wifi:disconnect' | `wifi:connect:${string}`
  | 'notifications:clear' | `notifications:dismiss:${string}`
  | `notes:delete:${string}` | `app:launch:${string}`;
```

New wrappers in `src/services/native.ts` follow the existing `invoke<T>(name, args)` style: `mediaArtwork()`, `noteSave(content)`, `weatherGeocode(query)`, `weatherLocate()`, `installedApps(query, limit, iconPx)`, `pinnedApps(keys, iconPx)`, `notificationsList(showText)`, `searchPreview(resultId)`, `searchIndexStatus()`. They belong to the UI plan (one task next to U1.4, XD-9).

### 5.7 Polling, caching and event rules

1. Demand driven. A provider is subscribed only while a mounted, enabled widget reads it. Today `useFinalWidgetData` subscribes to `system`, `media`, `volume`, `trash`, `known-folders` and `running-apps` and runs the AI and weather hooks for every profile (`useFinalWidgetData.ts:357-364`); a profile without a trash widget still polls the Recycle Bin and scans known folders (XD-11). This must change before the new providers land.
2. Visibility aware. The scheduler skips a refresh while `document.hidden` and resumes on `visibilitychange` (`provider.ts:15`, `24`). Native code never starts its own polling thread.
3. Coalesced. One request in flight per provider (`pending`, `provider.ts:14-19`). Native providers keep a time-to-live cache (5.3) so a card window and the dock share one Windows call.
4. Stale on failure. A failed refresh keeps the last real data and sets `stale` (`provider.ts:18`). Native code never invents continuity.
5. Cadence floor of one second; defaults in 5.3. Popovers call `refreshProvider(id)` when they open and subscribe to detail providers (`wifi-networks`, the notifications list) only while open.
6. An action refreshes the provider it affects with `refreshProvider(id)` (`provider.ts:32-35`) instead of waiting for the next tick.
7. Locks. `NATIVE_LOCK` stays only for the WinRT radio, media and Core Audio calls it exists for (`widget_data.rs:42-45`). A long operation (a Bluetooth scan, a network call) must not hold it; it runs under its own lock or as a background job with a cached result. New provider modules own a private `static Mutex` for their cache. Provider bodies are synchronous and run inside `spawn_blocking`, so no guard crosses an `.await`.
8. Events are hints. One event is reserved: `provider-changed` with payload `{"providerId":"<id>"}`, emitted with `AppHandle::emit`; the frontend listens once and calls `refreshProvider(id)`. Events never carry data. It is not used in v1. Adopt it only for a measured latency problem (the candidate is running apps through `SetWinEventHook`). If implemented, the hook lives on a dedicated thread with its own message loop, the callback only sets an atomic flag, and the thread debounces 250 ms before emitting.
9. Window changes, battery changes, media changes and notification changes are therefore polled, not pushed. The inputs' event names are not implemented (A22).

### 5.8 Windows, capabilities and permission sets

| Window | Capability file | Permission sets today | After this plan |
|---|---|---|---|
| `dock` | `dock.json` (shared with settings) | `dock-commands`, `calendar-read`, event listen and unlisten | `dock-commands` gains `media_artwork`, `pinned_apps_snapshot`; windows list becomes `["dock"]` (BE-50) |
| `settings` | `dock.json`, `calendar-settings.json` | `dock-commands`, `calendar-read`, `calendar-settings` | new `settings.json` with `settings-commands`: the dock set plus `calendar-settings`, `installed_apps_snapshot`, `weather_geocode`, `weather_locate` (BE-50) |
| `widget` | `widget.json` | `widget-card-commands` | gains `note_save`, `weather_geocode`, `weather_locate`, `media_artwork`, `pinned_apps_snapshot`, `notifications_list_snapshot` |
| `search` | `search.json` | `search-commands` | gains `search_preview`, `search_index_status` |

`open_widget_card` accepts only ids on the Rust allowlist (3.4) and sizes `compact`, `standard`, `wide`; settings payloads are limited to 64 KiB (`WIDGET_SETTINGS_LIMIT`).

### 5.9 Error model

1. A provider condition is a status (5.2), not an error. A command error is an exceptional failure of the call.
2. An error string is a short English sentence that is safe to show. It contains no path, no secret, no user content (note text, notification text, network names) and no stack trace. Detail goes to `protonsearch::applog::log` without user content (rust-skills `obs-no-sensitive-data`).
3. Every string argument has a maximum length checked first (existing precedent: query 4096 characters, window id 128).
4. A `JoinError` from `spawn_blocking` is returned as "Native task failed". Production paths contain no `unwrap` (rust-skills `err-no-unwrap-prod`) and every `unsafe` block carries a `// SAFETY:` comment (`unsafe-safety-comment`).

## 6. Persistence design

### 6.1 What exists today

| Store | Path | Format | Writers and readers | Notes |
|---|---|---|---|---|
| Dock layout | `%APPDATA%\protonsearch\protondock-layout.json` | JSON, version 2 | Only `save_dock_settings`, under the `NativeState.persistence` mutex, temp file then `MoveFileExW(REPLACE_EXISTING + WRITE_THROUGH)`, 1 MiB cap, then emits `settings-updated` (`main.rs:451-504`). Read by the frontend and natively by `weather.rs` and `local_calendar.rs`. | Browser preview uses local storage key `protondock-layout`. |
| Calendar grants | `%APPDATA%\protonsearch\calendar-sources.json` | JSON, version 1, 64 KiB cap | `local_calendar.rs`: tokens to file paths per profile, atomic save, corrupt file fails closed and is preserved. | Layout items store only profile, token and basename. |
| Engine database | `%APPDATA%\protonsearch\file_index.db` with `-wal` and `-shm` | SQLite 3.45.0 (bundled), WAL | The engine, indexer, watcher, browser indexer and git indexer each open their own connection with WAL and a 5 s busy timeout. Tables include `files`, `files_fts`, `clipboard_history`, `memory_events` (plus FTS), `timeline_events`, `quicklinks`, `snippets`, `focus_categories`, `ai_settings`, `settings_catalog_fts`, `browser_items`, the `git_*` tables, `agent_memory`, `ai_chats`, `agents`, `ignored_folders`, `indexer_state`. | Owned by the engine. `rusqlite` 0.31 with `bundled` is already a dependency of the engine crate and therefore already linked into `protondock` (`src-tauri/Cargo.lock`). |
| Notes | `%APPDATA%\protonsearch\notes\*.txt` | UTF-8 text files | Created by the legacy "Create Note"; listed by `search_notes` (`search.rs:8462`). | One file per note, name is the title. |
| Engine settings | `settings.json` next to the executable if it exists, else `%APPDATA%\protonsearch\settings.json` (`protonsearch/src/settings.rs:192-223`) | JSON | Engine and legacy settings UI through `AppSettings::load()` and `save()` | Includes `global_hotkey`, `scan_folders`. |
| Taskbar lease | `%APPDATA%\protonsearch\dock-taskbar-lease.json` | JSON | Legacy host; recovery in `taskbar_recovery.rs` | Only read by recovery. |
| AI chat key | `%APPDATA%\protonsearch\ai_key.txt` | plaintext | Engine chat client (`ai.rs:17`) | Flagged in Appendix B; not touched here. |

### 6.2 Decisions

1. The layout stays a JSON file, now version 3 (UI document 5.4). It is the single source of truth for profiles, per-widget settings, `pinnedApps`, appearance and theme.
2. No `protondock.db` in v1. Every datum in the inputs has a home that already exists or is cheaper (table below). The database stays a documented, gated option (BD-2); if the user reverses this default, 6.7 is the binding design.
3. The engine database is untouched. The dock reads it only through engine APIs (`clipboard_snapshot`, a public `installed_apps()`) or, for BE-41 and BE-42 only, a read-only connection (6.6).
4. Notes are files in the engine's notes folder (6.4).
5. Secrets, if any are ever stored, are DPAPI-protected values in `%LOCALAPPDATA%\protondock\credentials.json` (6.8). Nothing in v1 needs one.
6. Caches (weather, icons, catalog, Bluetooth, notifications) live in memory only. Nothing else is persisted.
7. New files go under `%LOCALAPPDATA%\protondock\`; existing data is never moved.

How each table of the prompt is replaced:

| Prompt table | Replacement | Why |
|---|---|---|
| `profiles` (id, name, `widget_order`, `is_active`) | layout `profiles`, `profile`, `profileOrder`, `profileNames` | Same data, already atomic and validated; a table would duplicate it and need a sync with the JSON the weather and calendar code reads. |
| `widget_settings` (width, enabled, JSON) | layout item `width` and `settings` | Per-profile, travels with the profile; widths are frontend-only. |
| `pinned_apps` (path, icon blob, order) | layout `pinnedApps` of native keys | No path or icon is stored; icons are re-rendered at the display's pixel ratio. |
| `quick_notes` | `.txt` files in the notes folder | One notes system, searchable by `notes:`. |
| `files_fts` | the engine's `files_fts` | 6.6. |
| `clipboard_history` | the engine's `clipboard_history` | 6.6. |

### 6.3 Layout v3 and the native save path

The v3 shape and the size-to-width migration table are the UI document 5.4 and are not repeated. Native obligations (BE-04, BE-06, BE-07):

1. Size and shape: object only, at most 1 MiB, as today. Native does not interpret widths.
2. Previous-good copy. Before replacing the file, if the current file parses as a JSON object, copy it to `protondock-layout.json.bak` (replace). A corrupt current file is never copied over a good backup.
3. Load fallback. If the file exists but is unreadable or not valid JSON, return the `.bak` content when it is valid and log that recovery happened (the file itself stays untouched until the next save). If both are bad, return the existing error so the frontend shows defaults and blocks saving (`DockShell.tsx:60-77`).
4. Native readers accept layout versions 1, 2 and 3 and any profile id matching `[a-z0-9][a-z0-9-]{0,23}`: `weather.rs` (`allowed_profile`, `location`) and `local_calendar.rs` (`profile_name`, `active_profile`, and the `["daily","founder"]` loop in `grants`). Built-in profile ids are unchanged.
5. The `settings-updated` event and the mutex stay as they are.

### 6.4 Notes folder

Folder: `%APPDATA%\protonsearch\notes` (created if missing, as the engine does). One note is one `.txt` file. Rules (BE-20): title is the first non-empty line, trimmed, with `< > : " / \ | ? *` and control characters replaced by `_`, whitespace collapsed, trailing dots and spaces removed, at most 60 characters, `Note` if empty, a leading `_` for Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to `COM9`, `LPT1` to `LPT9`, with or without an extension), a ` (2)` to ` (99)` suffix for collisions, a timestamp suffix beyond that. Content is at most 64 KiB, must contain a non-space character, is written as UTF-8 without a byte order mark through a temporary file and a rename, and never overwrites an existing file. Listing reads only `*.txt` regular files (no directories, no reparse points), newest modification first, at most 20, with a preview of at most 120 characters read from at most 4 KiB. The note id is `n-` plus 16 hex digits of the FNV-1a 64 hash of the file name; deletion re-lists the folder and matches the id, so no path comes from the renderer.

### 6.5 Calendar grants

Unchanged in format and semantics: tokens stay valid for their profile until explicitly forgotten; there is no automatic pruning; the 64 KiB file bound still rejects additions without overwriting; legacy `{token, path}` records stay readable; a corrupt file fails closed and is preserved. The only change is the profile validation of 6.3 item 4. Grants of a deleted custom profile remain until forgotten.

### 6.6 What goes into `files_fts` and `clipboard_history`

Nothing. `files_fts(path UNINDEXED, content)` and `clipboard_history` already exist, are populated by the engine and are ranked by the engine. The inputs' extra columns are covered without a schema change: filename, extension and modified time are in `files`; image OCR text and document text share `content`; category is computed from the extension at query time; clipboard text, source, pin state and OCR text are already columns. The dock never creates, alters or writes either table. Where a dock feature needs engine data without a public API it uses, in order: an existing engine function; a new `pub` wrapper in the engine crate (BE-34 for apps); a read-only connection (`SQLITE_OPEN_READ_ONLY`, `PRAGMA query_only=ON`, `busy_timeout=2000`) used only by BE-41 (file preview text from `files_fts.content`) and BE-42 (`SELECT COUNT(*) FROM files`).

### 6.7 Conditional database `protondock.db` (only if BD-2 is accepted)

Not created by any package in the default plan. Binding design if it is:

| Item | Decision |
|---|---|
| Location | `%LOCALAPPDATA%\protondock\protondock.db` (non-roaming; never inside the engine's directory or database). |
| Crate | `rusqlite = { version = "0.31", features = ["bundled"] }`, the version the engine already uses. The lock file resolves `rusqlite` 0.31.0 and `libsqlite3-sys` 0.28.0, which bundles SQLite 3.45.0 with FTS5, JSON1, `SQLITE_THREADSAFE=1` and `SQLITE_DEFAULT_FOREIGN_KEYS=1` [crate, E22]. No `backup` feature: `VACUUM INTO` is used for backups. |
| Pragmas at open | `PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON;`. `FULL` rather than the prompt's `NORMAL` because the data is user-authored; the database is tiny so the cost is negligible. |
| Migrations | `const MIGRATIONS: &[&str]`, one entry per version. The runner reads `PRAGMA user_version`, refuses to open a database whose version is higher than `MIGRATIONS.len()` (message "Database is newer than this build", feature disabled, file untouched, never downgraded), and applies each missing entry inside `BEGIN IMMEDIATE` and sets `user_version` in the same transaction. |
| DDL v1 | Only if notes are moved into the database (BD-2): `CREATE TABLE notes (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL DEFAULT '', body TEXT NOT NULL, created_ms INTEGER NOT NULL, updated_ms INTEGER NOT NULL); CREATE INDEX notes_updated ON notes(updated_ms DESC);`. The renderer sees `n-<rowid>`. Body is at most 64 KiB. No other table. |
| Connections | One connection in a `Mutex<Connection>` held in `NativeState`; every call runs inside `spawn_blocking`; the guard is never held across an `.await`. No pool: the database is tiny and has no concurrent writer. |
| Integrity | `PRAGMA quick_check(1)` at open. On failure the file is renamed `protondock.db.corrupt-<unix ms>`, a fresh database is created, the provider reports `error` with a message, and the latest backup is offered for restore. |
| Backup | At most once per 24 hours when the file changed, `VACUUM INTO '<tmp>'` then rename over `protondock.db.bak` (one generation). Recovery order: database, backup, empty. |
| Tests | `migrations_apply_in_order_and_set_user_version`, `newer_database_is_refused_untouched`, `corrupt_database_is_renamed_and_recreated`, `backup_is_a_valid_database`, each on a temporary directory. |

### 6.8 Credential store (only when a gated package needs it)

| Item | Decision |
|---|---|
| File | `%LOCALAPPDATA%\protondock\credentials.json`: `{"version":1,"entries":{"<provider>":{"blob":"<base64>","savedAt":<ms>}}}`. The file name matches the spec; no value is plaintext. Provider ids are `[a-z0-9-]{1,32}` from a code allowlist. |
| Protection | `CryptProtectData` with the current-user scope, description `ProtonDock`, optional entropy `protondock.credentials.v1`, flag `CRYPTPROTECT_UI_FORBIDDEN`; `CryptUnprotectData` with the same entropy; output blobs freed with `LocalFree` (feature `Win32_Security_Cryptography`, present in the crate). Secret bytes are overwritten before drop (best effort; no `zeroize` crate is added). |
| API | `credentials::set(provider, secret)`, `get(provider)` (native only, never exposed), `delete(provider)`, `configured(provider)`. |
| Commands | `credential_save({provider, secret})`, `credential_delete({provider})`, `credential_status({provider})` returning `{configured}`; settings window only. The secret is never returned, echoed in an error, logged or kept in React state after submission; the input is `type="password"` and cleared on submit. |
| Atomic write | Same temporary file and `MoveFileExW` replace as the layout. |
| Threat model | Protects the file at rest and from other Windows users. It does not protect against another process running as the same user. The same is true of any DPAPI use without a user secret. |
| Existing plaintext | The engine's optional `ai_key.txt` (`ai.rs:17`) is out of scope and not migrated; see Appendix B. |

### 6.9 Backup and recovery summary

| Store | Backup | Recovery | What the user sees on failure |
|---|---|---|---|
| Layout | `protondock-layout.json.bak` (previous good), `layoutV2Backup` inside the v3 document | load falls back to `.bak`; the UI document's migration reads the in-document backup | Defaults with a message and saving blocked if both files are bad (existing). |
| Calendar grants | none (fail closed, file preserved) | the user selects the calendar again | "Select a calendar source for the active profile". |
| Notes | the files themselves | file system | Save errors as in 5.4. |
| Engine database | engine behaviour, untouched | engine behaviour | unchanged |
| `protondock.db` (conditional) | `protondock.db.bak` daily | 6.7 | `error` status with a message |
| Credentials (gated) | none | the user re-enters the key | status `configured: false` |

### 6.10 Lossless migration acceptance tests

These run without the app (frontend unit tests and Rust temporary-directory tests) and must pass before the catalogue ships:

1. A saved v2 layout with four profiles, sizes, settings, two unknown ids and a theme migrates to v3: every known item survives in order with the alias map applied, the unknown ids are in `unknown`, `layoutV2Backup` equals the original, and saving twice never overwrites the backup.
2. A v1 layout still migrates through v2 rules to v3.
3. A v3 file read by a reader that does not know version 3 returns defaults with saving blocked (existing behaviour, asserted).
4. A truncated or corrupt layout falls back to `.bak`; with both bad, the error path is taken and no file is modified.
5. An existing `weather` item with no stored settings keeps Ahmedabad weather after the default flips (explicit settings are materialized); a new weather item starts unconfigured.
6. A calendar grant for `daily` still returns events after migration to a v3 layout (BE-07), and a grant for a custom profile is rejected only while another profile is active.
7. `pinnedApps` with a key that no longer resolves is reported in `missing`, never dropped from the layout.
8. A note file with a reserved device name or an invalid character in its first line is saved under a safe name and listed; deleting an id that no file matches fails without touching the disk.

## 7. Security and safety

### 7.1 Trust boundary, network and processes

1. The renderer is first-party React in WebView2 under the CSP in `tauri.conf.json:9`: `default-src 'self'`, images from `self`, `asset:` and `data:`, and `connect-src ipc: http://ipc.localhost`. It can load no remote script and make no outbound request, so every network call lives in Rust. The renderer is still treated as untrusted input: Rust validates everything it sends (7.4).
2. No listening sockets: no `tauri-plugin-localhost`, no HTTP or WebSocket server, no MCP port (4.5.3). BE-52 fails the build if `TcpListener::bind`, `UdpSocket::bind`, `tauri_plugin_localhost` or `tiny_http` appears under `src-tauri/src`.
3. Outbound network allowlist for everything in this plan: `api.open-meteo.com` (forecast, existing) and `geocoding-api.open-meteo.com` (BE-18). Rules: HTTPS only, `redirects(0)`, 4 s connect, 4 s read and 8 s total timeouts, response bounds (64 KiB), no cookies, no credentials. The city text typed for geocoding leaves the machine and the UI copy must say so. A gated package that adds a host (Admin usage APIs, BD-13) must be listed here first. The engine's legacy network features (`ai.rs`, browser and git indexers) are outside this plan.
4. Child processes: providers spawn none (the existing network provider already states "No netsh process is spawned"). Any unavoidable console child sets `creation_flags(0x08000000)` (`CREATE_NO_WINDOW`). The one existing GUI spawn is `notepad.exe` for legacy note opening in `execute_search_result` (`main.rs:296-302`) and is on the BE-52 allowlist.
5. No administrator elevation and no package identity requirement anywhere (4.3).

### 7.2 Secrets and personal data

1. Credentials follow 6.8: DPAPI values, never returned to the renderer, never logged, never in React state after submit, never in an error string. Nothing in v1 stores one.
2. Personal data classes handled by this plan and their rules:

| Data | Rule |
|---|---|
| Notification text | Opt-in widget; the tile polls a count only; text crosses IPC only when `showText` is true and only for the open card; truncated; never stored or logged. |
| Wi-Fi names and addresses | No BSSID anywhere; the network list is fetched only while the card is open; SSID and IP are not logged. |
| Location | Fetched only on an explicit click; rounded to two decimals; never logged; stored only as the user's own widget setting. |
| Bluetooth | Names only; addresses and device instance paths never leave Rust; advertisement scanning (gated) discards other people's devices immediately. |
| Notes | Local files; not logged; previews at most 120 characters. |
| Clipboard | Existing rule: previews hidden until the user enables them, at most three. |
| AI session data | The `agent` object carries the last prompt text and workspace path (`ai_usage.rs:295-310`); the renderer must read it only while the agent widget is enabled (Appendix B). |

3. No analytics and no telemetry leave the machine.

### 7.3 Capabilities per window

Section 5.8 is binding. A command is added to a window's permission set only if that window's UI needs it; sensitive commands (`weather_locate`, any `credential_*`) are never in the dock window's set. BE-50 splits the shared dock and settings capability so the settings window cannot be reached through the dock's set and the reverse.

### 7.4 What the renderer may send

| Value | Accepted form | Rejected how |
|---|---|---|
| `providerId` | one of the ids in 5.3 | "Unknown provider" |
| `actionId` | grammar of 5.5, parsed to an enum | "Unsupported widget action" |
| item id, key, token | issued earlier by a snapshot and looked up again | "Window expired; try again", "App no longer installed", "Result expired; search again" |
| `query`, `content` | within the stated character or byte bound | specific message per 5.4 |
| `keys` | at most 24 strings matching `app-` plus 16 lowercase hex digits | "Invalid app key" |
| `iconPx` | 32, 48, 64, 96, 128 or 256 | "Unsupported icon size" |
| layout JSON | an object of at most 1 MiB | "Invalid layout" |

The renderer never sends a path, an executable, a process id, a window handle, a URI, SQL text, an SSID password or a Bluetooth address.

### 7.5 OS interaction rules

`ShellExecuteExW` receives only fixed `ms-settings:` routes, engine-issued commands, allowlisted known folders and parsed `http` or `https` quicklinks (existing `open_uri`). Input injection exists in one place only: `start_menu.rs` sending `VK_LWIN` (BE-14); BE-52 fails if `SendInput` appears elsewhere. `SetForegroundWindow` is called only in response to a user click on a running-app tile.

### 7.6 COM, WinRT and GDI lifetime rules

1. One WinRT apartment per thread: `RoInitialize(RO_INIT_MULTITHREADED)` once through a thread-local, never `RoUninitialize` (the existing `Apartment::initialize`, `widget_data.rs:52-67`).
2. Shell namespace objects (`IShellItem`, `IShellItemImageFactory`, `IFileOpenDialog`) need a single-threaded apartment: use a short-lived dedicated thread with `CoInitializeEx(COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)` and a drop guard that calls `CoUninitialize` (the pattern already in `local_calendar.rs` and `search.rs`). Never move a non-agile COM pointer across threads: pass owned Rust data instead.
3. Release rules, enforced by drop guards in all new code (not by manual cleanup at each return): `CoTaskMemFree` for shell strings; `WlanFreeMemory` for WLAN buffers (the existing `WlanHandle` already closes its handle on drop); `FreeMibTable`; `LocalFree` for DPAPI blobs; `DeleteObject`, `DeleteDC`, `ReleaseDC` and `DestroyIcon` for GDI objects; `SetupDiDestroyDeviceInfoList` for device sets; `CloseHandle` for handles. Existing manual cleanup in `running_apps.rs` is not refactored here.
4. A native window handle that crosses threads is stored as `isize` (as `Identity.hwnd` already is).
5. Every `unsafe` block has a `// SAFETY:` comment and is as small as possible (rust-skills `unsafe-safety-comment`, `unsafe-minimize-scope`).

### 7.7 Win32 message-thread rules

1. Providers never run on the Tauri main thread; they run inside `spawn_blocking`.
2. Anything that needs a message pump (WinEvent hooks, a clipboard listener, session notifications, an AppBar) runs on one dedicated named thread created once, with its own `GetMessageW` loop and, where required, a message-only window. It is stopped with `PostThreadMessageW(WM_QUIT)` on `RunEvent::Exit` and joined.
3. A hook or window procedure never blocks, never calls COM, WinRT or UI Automation, never takes `NATIVE_LOCK`, and only sets an atomic or pushes to a bounded channel.
4. An AppBar (BE-70) sends `ABM_REMOVE` on exit and relies on the existing `--restore-taskbar` recovery after a crash.

### 7.8 File system safety

Reads are bounded (`take(limit + 1)` as `bounded_json` does). Writes are a temporary file in the same directory followed by an atomic replace or a no-overwrite rename. User-chosen files reject reparse points (existing calendar rule). The notes folder deletes only a file whose resolved parent equals the folder and which is a regular file. No package writes outside `%APPDATA%\protonsearch`, `%LOCALAPPDATA%\protondock` or a temporary directory in tests.

### 7.9 Dependency budget

New crates in the default plan: none. New features: the `windows` features of BE-02 and the `image` `jpeg` feature. `rusqlite` is already linked through the engine and is added to `src-tauri` only if BD-2 is accepted (BE-68). No `sysinfo`, no `fastembed`, no WMI crate, no HTTP server crate.

### 7.10 Error model

The command error model is 5.9. In short: provider conditions are statuses, not errors; a command error is a short sentence that is safe to show and contains no path, secret or user content; every input is length-bounded before use; a `JoinError` becomes "Native task failed"; production code has no `unwrap`; detail is logged through `protonsearch::applog::log` without user content. Library-style modules (`layout_store`, `keys`, `notes`, `credentials`) return typed `Result<T, String>` values to the thin command wrappers rather than panicking (rust-skills `err-result-over-panic`, `err-no-unwrap-prod`).

## 8. Data honesty register

Every item below fabricates or simulates state in an input. The right column is the real behaviour or the honest empty state that replaces it. UI-only fabrications in `dock-editable.html` are catalogued by the UI document (B1 to B9, 3.2 there); this table adds the backend-visible ones and the repo's own. "Sample data" may exist only in the browser preview (`?preview=1`) with a visible "Sample" label and never in a native build (UI document U4.3).

| # | Source | Fabrication or simulation | Replacement | State shown | Package |
|---|---|---|---|---|---|
| 1 | spec 2.2; HTML | Charging versus draining simulation button | Removed. The tile shows only the battery report. | no battery: `empty` "No battery detected" | BE-11 |
| 2 | spec 2.2 | `+48.2W`, `19.8 V`, `2.43 A`, health 96 percent, 142 cycles, `68.2 Wh / 71.0 Wh`, 31.4 C | Real milliwatt rate and capacities; voltage, cycles and temperature only from BE-21, otherwise `--` | live with nulls | BE-11, BE-21 |
| 3 | spec 2.16 item 2; HTML | AI Working and Ready test toggle | `working` from session activity within 300 s | idle | BE-12 |
| 4 | spec 1.4, 2.3, 2.4; HTML | Invented quotas ("70% Session", "82% Quota"); "Save" that reports Connected without checking | `codex`: real percentages from local logs. `claude`: tokens only, percentages `--`. No key entry. | live, or `disconnected` when no sessions exist | BE-12 |
| 5 | spec 2.12, 2.16 item 4; HTML | AirPods 85 percent, MX Master 60, Keychron 92, case 34, left 88, right 92, ANC pills | Real devices that report a level; none: "No device reports battery". ANC removed. | `empty` or live | BE-30 |
| 6 | spec 2.11; HTML | `Studio-5GHz`, `Wi-Fi 6`, 866 Mbps, -52 dBm, `192.168.1.145`, a list of networks | Real WLAN data; consent missing: `permission-denied`; no adapter: `unsupported` | per 5.2 | BE-10, BE-32 |
| 7 | spec 2.11, 2.12 | "Connect" and "Pair" with a toast that claims success | Connect only to a saved profile and report nothing until the next poll shows it; pairing opens Settings | live state only | BE-33 |
| 8 | spec 2.1, 2.15; HTML 5666-5697 | San Francisco, 24 C, H28 L19, 19 percent rain, 11 km/h; setup that ignores the typed city | Open-Meteo for the geocoded or located place; unconfigured shows `--` | `disconnected` | BE-18, BE-19 |
| 9 | repo `meta.ts:264` | Default weather is Ahmedabad and enabled | Unconfigured default after old settings are materialized for existing items | `disconnected` | XD-5 |
| 10 | spec 2.13, 2.15 item 4; HTML | Three sample notifications; red badges 212, 2, 1 | Real toast count and list; badges only as toast counts and only if BD-14 | `permission-denied` or live | BE-38 |
| 11 | spec 2.15 | "8 Running" tray apps (Windows Security, NVIDIA, Docker, OneDrive, Steam, Discord, Proton VPN, Realtek) | Not implemented; if kept, a real list or `unavailable` | `unsupported` | BE-63, BE-64 |
| 12 | spec 2.8; HTML 3904-3906, 4614-4640 | Invented playlist, album gradients, "Nightcall by Kavinsky" | Real session, real artwork or a neutral glyph | `empty` "Nothing playing" | BE-15, BE-16 |
| 13 | spec 2.15; HTML 3780 | Static `FRI 16`, sample meetings and colour bars | Real local date; ICS events only; recurrence reported as unsupported | `disconnected` or `empty` | BE-07, BE-39 |
| 14 | spec 2.15; HTML 5532 | `cpu 18, ram 58, ssd 43, peak 36` and a random walk | Real values; CPU `null` until the second sample; SSD is used capacity; history only from real samples | live with nulls | none |
| 15 | HTML 3931-3936 | Chrome, Spotify, Terminal, VS Code with badge counts | Real windows; none: the widget renders nothing | `empty` | BE-13 |
| 16 | HTML 3939-3946 | Six hard-coded installed apps | Native AppsFolder enumeration | `empty` | BE-34 to BE-36 |
| 17 | HTML `protondock_notes_v1` | Notes in browser storage | Files in the notes folder | `empty` "No notes yet" | BE-20 |
| 18 | spec 2.5 | Waveform array and transcript for the assistant | No capture exists, so no waveform or transcript | unconfigured | BD-6 |
| 19 | prompt 3.11 | `toggle_agent_execution` | Dropped | none | none |
| 20 | prompt 3.4 | `create_calendar_event` | Dropped; read-only | none | none |
| 21 | spec 2.2 | "Power: On AC (4h 12m)" | Time remaining is `null` on AC | live with null | BE-11 |
| 22 | HTML 3741 | Search sub-label "Alt + Space" hard-coded | Registered hotkey from `hotkey`, or none | live | BE-08 |
| 23 | spec 2.17 | "Kensaku Engine" badge claiming OCR and semantic scopes | Only what the engine has: no semantic claim; the OCR pill is an image-extension filter | live | none |
| 24 | UI document 4.3 | `system-tray` "always live" | `unavailable` until BD-5 | `unsupported` | XD-8 |
| 25 | HTML | Analog clock hands frozen at fixed angles | Hands follow the browser clock (UI) | live | none |

## 9. Backend work packages

### 9.1 Conventions

1. Ids are `BE-nn`. Numbers are sparse on purpose and group by area: 0x hygiene, 1x honest reads, 2x notes and extras, 3x devices and apps, 4x search, 5x hardening, 6x gated, 7x taskbar.
2. Size: S is at most about 80 changed lines, M at most about 250, L is more than that and the planner splits it. Each package touches 1 to 3 files.
3. Provider modules are flat files in `src-tauri/src/`, declared in `providers.rs` as `#[path = "<name>.rs"] pub mod <name>;` and are pure: they do not `use crate::`. A provider returns `Reading { status: &'static str, data: serde_json::Value, message: String }` and `providers.rs` wraps it with `widget_data::envelope`. A pure module gets a test target `src-tauri/tests/<name>.rs` containing `#[path = "../src/<name>.rs"] mod <name>;` (the precedent is `src-tauri/tests/local_calendar.rs`), which runs its tests without compiling Tauri. A module that must call `crate::` keeps its tests inline.
4. Test kinds, none of which launches the app: pure unit tests; per-module test targets; read-only native smoke tests in the style of `native_readonly_provider_smoke`; and `#[ignore]` spikes run once by a person, with output recorded in `docs/qa/STATUS.md`. `cargo run` and the executable are never started as background processes. Tests that change user state (volume, trash, Wi-Fi connection, notifications) are never automated.
5. CU means native Computer Use verification on the real app. If the `orca` CLI is still unavailable (UI document 7), record the exact blocker in `docs/qa/STATUS.md` and leave the gate unverified.
6. Every package ends with: build gates (`npx tsc -b`, `npm test -- --reporter=dot`, `cargo test --manifest-path src-tauri/Cargo.toml`, with the pre-existing `local_calendar` access-denied failures excluded), an update of `docs/widgets/NATIVE_PROVIDERS.md` or `CONTRACT.md` where a contract changed (done by the integrator per phase, not by each package), then commit and push with explicit paths.

### 9.2 Order, dependencies and file overlaps

| Phase | Packages | Purpose |
|---|---|---|
| A. Hygiene | BE-01, BE-02, BE-03, BE-04, BE-06, BE-07, BE-08, BE-09 | Registry, features, ACL consistency, durable layout, validators that accept v3, non-fatal hotkey, stable keys. |
| B. Honest reads | BE-10 to BE-16, BE-18 to BE-20, BE-21 (optional) | Real battery, media artwork, AI working signal, Wi-Fi permission state, Start, weather search and location, notes, optional battery extras. |
| C. Devices and apps | BE-30 to BE-39 | Bluetooth, Wi-Fi detail and actions, installed and pinned apps with icons, notifications (after a spike), calendar week. |
| D. Search extras | BE-40 to BE-42 | Reveal, copy path, preview, index status (after the Spotlight UI work). |
| E. Wiring and hardening | BE-49, BE-50, BE-51, BE-52 | One command registration pass, capability split, trash confirmation, security guards. |
| F. Gated | BE-60 to BE-71 | Each needs the decision named in its card. |

Dependencies: BE-01 first. BE-02 and BE-09 follow BE-01. BE-11 to BE-20, BE-30 to BE-39 need BE-01 and BE-02 (and BE-09 where they use keys: BE-13, BE-20, BE-34, BE-36). BE-49 follows every module whose command it registers. BE-50 follows BE-49. BE-52 can run at any time. Gated packages follow their decision.

Serialize on shared files: `main.rs` editors in the order BE-04, BE-06, BE-08, BE-14, BE-40, BE-41, BE-42, BE-49 (and the UI plan's U4.1); `widget_data.rs` editors BE-10, BE-16, BE-32, BE-33, BE-51; `running_apps.rs` editors U1.4 then BE-13 (or merged); `local_calendar.rs` editors BE-07 then BE-39; `commands.toml` editors BE-03, BE-49, BE-50; `Cargo.toml` editors BE-02 first, then BE-37 and gated packages. Everything else is parallel-safe.

### 9.3 UI-plan obligations the packages depend on

| Obligation | UI task | Needed by |
|---|---|---|
| `WidgetStatus` gains `unsupported` and the state table maps it (XD-1) | U0.3 | BE-10 and the existing providers |
| `ProviderId`, `WidgetActionId` and the `native.ts` wrappers of 5.6 | one task next to U1.4 | every package that adds a provider or command |
| Layout v3, aliases, `pinnedApps`, `layoutV2Backup`, key whitelist, weather materialization (3.3, XD-4, XD-5) | U1.2 | BE-06, BE-07, BE-36 |
| Demand-driven provider subscription (5.7 item 1, XD-11) | new UI task before BE-11 | BE-11 and every later provider |
| Replace the Ahmedabad weather default only after materializing existing settings (XD-5, BD-8) | U1.2 | BE-18, BE-19 |
| `iconPx` ladder request for running apps, pinned apps and the picker (XD-2) | U1.4 | BE-13, BE-35, BE-36 |
| Tile and card states for `permission-denied` with remedy actions, no ANC, no toast HUD | U0.3, U4.x | BE-10, BE-30, BE-38 |
| Note, weather setup, Wi-Fi, Bluetooth and notification cards | U4.x | BE-20, BE-18, BE-19, BE-32, BE-30, BE-38 |
| No sample data on the native path (Section 8) | U4.3 | all |

### 9.4 Phase A: hygiene

**BE-01 providers-registry.** Size S. Parallel: first.
Goal: one registry for new providers and widget actions so later packages never edit `provider_snapshot_data` or `widget_data::action`.
Files: `src-tauri/src/providers.rs` (new), `src-tauri/src/main.rs`, `src-tauri/src/widget_data.rs`.
Change: `providers.rs` exposes `pub fn snapshot(id: &str) -> Result<Value, String>` (returns `Err("Unknown provider")` for any id without an arm) and `pub fn action(id: &str) -> Result<(), String>` (returns `Err("Unsupported widget action")`). `main.rs` declares `mod providers;` and, in `provider_snapshot_data`, replaces the last arm `_ => return Err("Unknown provider".into())` with `_ => return providers::snapshot(&provider_id)`. In `widget_data::action`, the final check that returns "Unsupported widget action" for non-media actions instead calls `drop(_guard); return crate::providers::action(action);` so the global lock is released before delegating. Match arms are one per line, alphabetical, so concurrent packages merge cleanly.
Tests: `unknown_provider_is_rejected`, `unknown_action_is_rejected` in `providers.rs`; the existing `rejects_unlisted_actions_and_providers` and the `provider_snapshot_data("shell")` test must still pass.
CU: no. Decision: none.

**BE-02 windows-features-batch.** Size S. Parallel: yes after BE-01.
Files: `src-tauri/Cargo.toml`.
Change: put the `windows` features one per line (a single long line makes every later package conflict) and add exactly `Devices_Power`, `System_Power`, `Storage_Streams`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_Devices_DeviceAndDriverInstallation`, `Win32_Devices_Properties`, `Devices_Geolocation`; change the `image` features to `["png", "jpeg"]`. Nothing else. Gated packages add their own features when they start (`UI_Notifications_Management`, `UI_Notifications` and `ApplicationModel` in BE-37, `Win32_Security_Cryptography` in BE-60, `Devices_Bluetooth` and `Devices_Bluetooth_Advertisement` in BE-62, `Win32_System_Ioctl` and `Win32_System_IO` in BE-21).
Tests: the orchestrator runs `cargo check --manifest-path src-tauri/Cargo.toml`. CU: no.

**BE-03 ipc-manifest-sync.** Size M. Parallel: yes.
Files: `src-tauri/build.rs`, `src-tauri/permissions/commands.toml`, `src-tauri/tests/ipc_manifest.rs` (new).
Change: add the five missing commands (`known_folders_snapshot`, `open_widget_card`, `hide_widget_card`, `widget_card_state`, `ai_usage_snapshot`) to the `AppManifest::commands` list. The test reads `main.rs` (the `generate_handler![...]` list), `build.rs` and `commands.toml` as text.
Tests: `every_handler_is_in_the_manifest`, `every_manifest_command_is_allowed_by_some_set`, `no_set_allows_an_unknown_command`.
CU: yes. After the change the dock must still open a widget card and the settings window must still open. If the app's ACL behaves differently, revert the `build.rs` edit and keep the test with the five commands as a documented exception.

**BE-04 layout-store.** Size M. Parallel: serialize on `main.rs` (first of the `main.rs` editors).
Files: `src-tauri/src/layout_store.rs` (new), `src-tauri/src/main.rs`, `src-tauri/tests/layout_store.rs` (new).
Change: move the file handling of `load_dock_settings` and `save_dock_settings` into `layout_store::load(dir: &Path) -> Result<Value, String>` and `save(dir: &Path, value: &Value) -> Result<(), String>` (same temporary file and `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`, same 1 MiB and object checks), and add the previous-good `.bak` copy and the load fallback of 6.3. The commands keep their names, arguments, mutex and the `settings-updated` emit; `data_dir()` stays in `main.rs` and is passed in.
Tests: `save_then_load_round_trips`, `object_and_size_limits_are_enforced`, `previous_good_copy_is_kept`, `corrupt_file_falls_back_to_backup`, `corrupt_file_never_replaces_a_good_backup`, `both_bad_returns_error_and_modifies_nothing`.
CU: no (frontend behaviour is unchanged).

**BE-05 ts-contract-types.** Not a backend package; it is the UI plan task listed in 9.3 (the types of 5.6 and the `native.ts` wrappers), numbered here so references resolve. Files: `src/widgets/sdk/types.ts`, `src/services/native.ts`. Owner: UI plan.

**BE-06 catalogue-allowlists.** Size S. Parallel: serialize on `main.rs`.
Files: `src-tauri/src/main.rs`, `protonsearch/src/weather.rs`.
Change: `WIDGET_CARD_IDS` becomes the exact array of 3.4 (`[&str; 30]`). In `weather.rs`, `allowed_profile(version, profile)` accepts version 1 with `daily` and `founder`, version 2 with the four built-ins, version 3 with any id matching `[a-z0-9][a-z0-9-]{0,23}`; `location` accepts versions 1, 2 and 3.
Tests: `widget_card_ids_accept_canonical_and_alias_ids_only` (`main.rs`); `weather_location_reads_version_three_and_custom_profiles`, `weather_rejects_invalid_profile_ids` (`weather.rs`).
CU: no. Decision: BD-1 only affects whether `chatgpt` is an alias; the array keeps it either way.

**BE-07 calendar-layout-compat.** Size M. Parallel: yes (serialize with BE-39 on the same file).
Files: `src-tauri/src/local_calendar.rs`.
Change: `profile_name` accepts the built-ins and the v3 grammar. `active_profile` reads the layout and handles versions 1, 2 and 3: version 1 keeps today's rule; versions 2 and 3 are active when `layout.profile` equals the requested profile and `layout.profiles` contains it; a missing layout keeps `daily` active; any other shape stays "Native dock profile could not be validated". `grants` validates every profile key present, not only `daily` and `founder`, with the existing record rules. The pre-existing access-denied failures of other tests in this module are unrelated and untouched.
Tests (inline, using the existing temporary-directory fixtures): `version_two_layout_activates_its_profile`, `version_three_custom_profile_is_valid`, `inactive_profile_is_denied_not_an_error`, `unknown_layout_version_is_rejected`, `grants_for_custom_profiles_are_validated`.
CU: yes when the calendar card exists (BE-39); until then no.

**BE-08 hotkey-resilience.** Size S. Parallel: serialize on `main.rs`.
Files: `src-tauri/src/main.rs`, `src-tauri/src/hotkey_status.rs` (new), `src-tauri/src/providers.rs` (declaration and arm).
Change: in `setup`, registering `Alt+Space` and the configured hotkey no longer uses `?`: each result is recorded (`registered` or `failed`) and a failure is logged, never fatal. `hotkey_status::read() -> Reading` returns `live` with `{registered, failed}` for `provider_snapshot("hotkey")`.
Tests (module): `records_success_and_failure_separately`, `snapshot_has_both_lists`.
CU: yes: with another program holding `Alt+Space` (PowerToys Run does by default) the dock must still start and the failure must be listed. If no such program is available the gate stays unverified.

**BE-09 stable-keys.** Size S. Parallel: yes after BE-01.
Files: `src-tauri/src/keys.rs` (new), `src-tauri/tests/keys.rs` (new), `src-tauri/src/providers.rs` (declaration `#[path = "keys.rs"] pub mod keys;`).
API: `pub fn fnv1a64(bytes: &[u8]) -> u64` (offset `0xcbf29ce484222325`, prime `0x100000001b3`); `pub fn key(prefix: &str, text: &str) -> String` returning `format!("{prefix}-{:016x}", fnv1a64(text.to_lowercase().as_bytes()))`; `pub fn is_key(prefix: &str, value: &str) -> bool` (the prefix, a dash, 16 lowercase hex digits).
Tests: `fnv1a64_matches_reference_vectors` (empty input is `0xcbf29ce484222325`, `"a"` is `0xaf63dc4c8601ec8c`), `key_lowercases_before_hashing`, `is_key_rejects_wrong_prefix_length_and_case`.
CU: no.

### 9.5 Phase B: honest reads

**BE-10 wifi-permission-state.** Size S. Parallel: serialize on `widget_data.rs`.
Files: `src-tauri/src/widget_data.rs`, `docs/widgets/NATIVE_PROVIDERS.md`.
Change: `current_wifi_connection` returns `Result<Option<WLAN_CONNECTION_ATTRIBUTES>, u32>` carrying the Win32 error code. In `read_wifi`, when the interface is connected and the query fails with code 5 (`ERROR_ACCESS_DENIED`), return connected with no name, no signal, state `connected` and `access_denied: true`; every other failure keeps today's behaviour. `network()` adds `"accessDenied"` to the `wifi` object (false otherwise). Add the fixed route `settings:location` (`ms-settings:privacy-location`) to `settings_uri`. Document the new field, the action, the consent coupling and the status note in `NATIVE_PROVIDERS.md`.
Tests: `access_denied_query_reports_connected_without_a_name` (pure helper `wifi_from_query`), `other_query_failures_stay_disconnected`, `settings_location_has_the_exact_route`.
CU: unverified gate. Observing the denied branch needs location consent switched off, a system setting this plan does not change; record the blocker in `docs/qa/STATUS.md`.

**BE-11 battery-provider.** Size M. Parallel: yes.
Files: `src-tauri/src/battery.rs` (new), `src-tauri/tests/battery.rs` (new), `src-tauri/src/providers.rs` (declaration and arm `"battery"`).
Behaviour of `read() -> Reading`:
1. `Battery::AggregateBattery()` then `GetReport()`. On error: status `unsupported`, message "Battery information is unavailable".
2. `report.Status()`: `NotPresent` gives status `empty`, `data` null, message "No battery detected". `Discharging`, `Idle` and `Charging` become the `state` strings; anything else is `unknown`.
3. `ChargeRateInMilliwatts`, `RemainingCapacityInMilliwattHours`, `FullChargeCapacityInMilliwattHours` and `DesignCapacityInMilliwattHours` are optional integers; a null reference is `None`.
4. `GetSystemPowerStatus` supplies `acConnected` (`ACLineStatus` 1 true, 0 false, 255 null), `percent` (`BatteryLifePercent`; 255 is null; if null and remaining and full are known, `round(remaining * 100 / full)` clamped to 0..=100) and `secondsRemaining` (`BatteryLifeTime`, `0xFFFFFFFF` is null, reported only while discharging).
5. `healthPercent` is `round(full * 100 / design)` clamped to 0..=100, null when either is null or design is 0.
6. `secondsToFull` is `(full - remaining) * 3600 / rate`, only while charging with a positive rate and `full > remaining`, clamped to 0..=86400.
7. `cycleCount`, `voltageMillivolts`, `temperatureCelsius` are null until BE-21. A private 5 second cache holds the last reading.
Tests: `health_is_none_without_design_capacity`, `health_is_clamped_to_one_hundred`, `seconds_to_full_requires_charging_and_positive_rate`, `percent_falls_back_to_capacities_only_when_power_status_is_unknown`, `battery_readonly_smoke` (passes with or without a battery: the status is `live`, `empty` or `unsupported`).
Reference values on the development machine for the native check [obs E4]: state idle, rate 0, full 83002, design 83002, remaining 70152 (milliwatt-hours).
CU: yes with the battery tile.

**BE-12 ai-working-signal.** Size S. Parallel: yes.
Files: `src-tauri/src/ai_usage.rs`.
Change: add `"working": bool` to the `claude` and `chatgpt` objects: true when the newest session file was modified within `LIVE_WINDOW_SECONDS` (300). `claude_data` already finds the newest file and reads its modification time; `chatgpt_data` receives the flag computed from the rollout file. No sessions: `false`. No new reads, no network, no credentials.
Tests: `claude_working_is_true_only_for_recent_transcripts`, `codex_working_matches_agent_live_status` (set times with `File::set_modified` in the existing temporary-home fixtures).
CU: no.

**BE-13 running-apps-key.** Size S. Parallel: serialize on `running_apps.rs` with U1.4.
Files: `src-tauri/src/running_apps.rs`.
Change: each item gains `"key"`: `crate::providers::keys::key("app", &lowercase_exe_path)` using the path from `QueryFullProcessImageNameW` (already used in `icon()`), or null when unavailable. The renderer persists user order by `key`. No other change (icon size belongs to U1.4).
Tests: `key_is_stable_for_the_same_path_and_case_insensitive`, `key_has_the_app_prefix_and_sixteen_hex_digits`, `missing_path_gives_null_key`.
CU: no. Depends on BE-09.

**BE-14 start-toggle.** Size M. Parallel: serialize on `main.rs`.
Files: `src-tauri/src/start_menu.rs` (new), `src-tauri/tests/start_menu.rs` (new), `src-tauri/src/main.rs`.
Change: `key_taps() -> [INPUT; 2]` builds two `INPUT_KEYBOARD` structures for `VK_LWIN` (first no flags, second `KEYEVENTF_KEYUP`, scan code 0, time 0, extra info 0). `toggle() -> Result<(), String>` calls `SendInput(&key_taps(), size_of::<INPUT>() as i32)` and returns `Err("Windows rejected the Start key press")` unless it inserted 2 events. In `main.rs`, `open_start` calls `start_menu::toggle()`; on `Err` it falls back to `protonsearch::launcher::launch("shell:AppsFolder")`, returns `Ok(())` only if a path actually ran, and logs which. The module sends no key other than `VK_LWIN`.
Tests: `key_taps_are_win_down_then_win_up`, `key_taps_are_one_batch_of_two_keyboard_inputs`.
CU: yes: clicking the Start tile opens the Start menu and a second click closes it; with an elevated window in the foreground record that nothing happens (UIPI) and that no success is claimed.

**BE-15 media-artwork-module.** Size M. Parallel: yes.
Files: `src-tauri/src/media_art.rs` (new), `src-tauri/tests/media_art.rs` (new), `src-tauri/src/providers.rs` (declaration).
Change: `square_png(bytes, px) -> Result<Vec<u8>, String>` decodes PNG or JPEG with the `image` crate, center-crops to a square, resizes with `FilterType::Lanczos3` and encodes PNG; it rejects input over 4 MiB and images over 4096 pixels on a side. `current() -> Result<Option<Artwork>, String>` (`Artwork { id, png }`) reads the current GSMTC session, `TryGetMediaPropertiesAsync().get()`, `Thumbnail()` (absent gives `Ok(None)`), `OpenReadAsync().get()`, reads at most 4 MiB through a `DataReader`, and returns a 96 px PNG with `id = key("art", "{title}|{artist}|{album}|{length}")`. `peek_id() -> Option<String>` returns the cached id without decoding when the track is unchanged. One cached entry; a private mutex; no `NATIVE_LOCK`.
Tests: `square_png_crops_and_resizes_a_wide_image`, `square_png_rejects_oversized_input`, `square_png_reads_png_and_jpeg`, `artwork_id_changes_with_the_track`.
CU: no (BE-16 and the UI show it).

**BE-16 media-artwork-id.** Size S. Parallel: serialize on `widget_data.rs`.
Files: `src-tauri/src/widget_data.rs`.
Change: `media()` adds `"artworkId"` from `crate::providers::media_art::peek_id()` (null when none). The `media_artwork` command is registered in BE-49.
Tests: extend the existing media smoke test to assert the key `artworkId` exists (null allowed).
CU: no.

**BE-18 weather-geocode.** Size M. Parallel: yes.
Files: `protonsearch/src/weather.rs`.
Change: `pub fn geocode(query: &str) -> Result<Value, String>`: trim; 2 to 80 characters else `Err("Enter at least 2 characters")`; request `https://geocoding-api.open-meteo.com/v1/search` with `name`, `count=5`, `language=en`, `format=json` through the same `ureq` agent rules as `fetch` (no redirects, 4 s connect, 4 s read, 8 s total) and `bounded_json` at 64 KiB. `parse_geocode` keeps at most 5 results, each with a non-empty name (cut to 80 characters), a finite latitude in -90..=90 and a finite longitude in -180..=180; `admin1` and `country` optional and cut to 80. Output `{"results":[{name, admin1, country, latitude, longitude}]}`. Any transport or parse failure maps to "Place search is unavailable" (never the raw error).
Tests: `geocode_rejects_short_and_long_queries`, `parse_geocode_bounds_results_and_rejects_invalid_coordinates`, `parse_geocode_drops_entries_without_a_name`.
CU: no (BE-49 and the UI later).

**BE-19 weather-locate.** Size M. Parallel: yes.
Files: `src-tauri/src/location.rs` (new), `src-tauri/tests/location.rs` (new), `src-tauri/src/providers.rs` (declaration).
Change: `locate() -> Result<Fix, LocateError>`: `Geolocator::new()`; `RequestAccessAsync().get()`: `Allowed` continues, `Denied` and `Unspecified` give `PermissionDenied`; `SetDesiredAccuracy(PositionAccuracy::Default)`; `GetGeopositionAsyncWithAgeAndTimeout` with a 5 minute maximum age (3000000000 ticks of 100 ns) and a 10 second timeout (100000000 ticks); coordinates are read from `Coordinate()?.Point()?.Position()?` (the `Latitude()` accessor is behind the crate's `deprecated` feature) and rounded to two decimals; any other failure is `Unavailable`. `to_result(Result<Fix, LocateError>) -> Value` builds `LocateResult` and includes no coordinates unless the status is `ok`. Nothing is logged.
Tests: `coordinates_round_to_two_decimals`, `denied_maps_to_permission_denied`, `unavailable_maps_to_unavailable`, `result_has_no_coordinates_unless_ok`.
CU: yes with location consent on; if it is off, record the blocker (shared with Wi-Fi, 4.3 item 2).

**BE-20 notes-folder.** Size M. Parallel: yes. Needs: BD-2 default (files); if BD-2 is accepted this package is replaced by BE-68.
Files: `src-tauri/src/notes.rs` (new), `src-tauri/tests/notes.rs` (new), `src-tauri/src/providers.rs` (declaration, provider arm `notes`, action prefix `notes:delete:`).
Change: implement 6.4. API: `default_dir() -> Option<PathBuf>`, `list(dir, limit) -> Vec<NoteRow>`, `save(dir, content) -> Result<NoteRow, String>`, `delete(dir, id) -> Result<(), String>`, `file_name_for(first_line, existing) -> String`, `valid_id(id) -> bool`. Errors: "Note is empty", "Note is too long", "Could not save the note", "Note not found". The id is `key("n", file_name)`. The `note_save` command is registered in BE-49.
Tests: `file_name_replaces_invalid_characters_and_reserved_names`, `title_uses_first_nonempty_line_and_truncates_to_sixty`, `collisions_get_numeric_suffixes`, `save_never_overwrites_an_existing_note`, `delete_rejects_unknown_ids`, `list_is_newest_first_bounded_and_text_only`, `empty_and_oversized_content_are_rejected`.
CU: yes after BE-49 and the card UI: save a note, find it with `notes:` in ProtonSearch, delete it.

**BE-21 battery-extras.** Size L, optional (after BE-11). Gate: only if the user wants the HUD rows.
Files: `src-tauri/src/battery.rs`, `src-tauri/tests/battery.rs`, `src-tauri/Cargo.toml` (features `Win32_System_Ioctl`, `Win32_System_IO`).
Change: through SetupAPI `GUID_DEVICE_BATTERY`, open the battery device and call `IOCTL_BATTERY_QUERY_TAG`, `IOCTL_BATTERY_QUERY_INFORMATION` (`BatteryInformation.CycleCount`, `BatteryTemperature` in tenths of a kelvin) and `IOCTL_BATTERY_QUERY_STATUS` (voltage in millivolts). A zero cycle count, a failed temperature query or an unknown voltage is `null`. Handles are closed by guards.
Tests: `kelvin_tenths_convert_to_celsius`, `zero_cycle_count_is_null`, `ioctl_failures_are_null_not_errors`.
CU: yes on a laptop.

### 9.6 Phase C: devices and apps

**BE-30 bluetooth-devices.** Size L (the planner splits enumeration from the provider arm). Parallel: yes.
Files: `src-tauri/src/bluetooth.rs` (new), `src-tauri/tests/bluetooth.rs` (new), `src-tauri/src/providers.rs` (declaration and arm `bluetooth`).
Change: `read_devices() -> Result<Vec<Device>, String>`: `SetupDiGetClassDevsW(Some(&GUID_DEVCLASS_BLUETOOTH), None, None, DIGCF_PRESENT)`, enumerate with `SetupDiEnumDeviceInfo`, read the friendly name (`DEVPKEY_Device_FriendlyName`) and the battery property `{104EA319-6EE2-4701-BD47-8DDBF425BBE5}, 2` with `SetupDiGetDevicePropertyW` (type `DEVPROP_TYPE_BYTE`, a value above 100 is null), keep nodes with a non-empty name that report a level, de-duplicate by name keeping the highest, and destroy the device set through a guard. `kind` from the lowercase name: `airpods`, `buds`, `headphone`, `headset`, `earbuds`, `speaker` give `audio`; `mouse`, `keyboard`, `trackpad`, `mx ` give `input`; else `other` (best-effort, said in the doc comment). Ids are `bt-1` to `bt-n` per snapshot. The provider arm composes `{radio, devices}` where `radio` is `data.bluetooth` taken from `widget_data::snapshot("network")` (no change to `widget_data.rs`). Status is `live` whenever the radio is known, even with zero devices; with no radio and no devices it is `empty` ("No Bluetooth radio").
Tests: `kind_from_name_classifies_common_devices`, `battery_byte_above_one_hundred_is_null`, `duplicates_keep_the_highest_level`; ignored `bluetooth_devices_spike` prints names and levels (the reference machine should list 2 devices, [obs E5]).
CU: yes with the peripherals tile.

**BE-31 bluetooth-radio-spike.** Size S. Parallel: yes. SPIKE.
Files: `src-tauri/tests/bluetooth_radio_spike.rs` (new, `#[ignore]`).
Change: call `Radio::GetRadiosAsync().get()` and read `Kind` and `State` 30 times on one thread and then on a second thread, and register `StateChanged` on a Bluetooth radio, noting which variants abort the process. A person runs it once and records the result in `docs/qa/STATUS.md`.
It decides whether the once-per-process cache at `widget_data.rs:641` can be replaced by repeated reads or an event. CU: no.

**BE-32 wifi-detail.** Size L (the planner splits `wifi` and `wifi-networks`). Parallel: serialize on `widget_data.rs`.
Files: `src-tauri/src/wifi.rs` (new), `src-tauri/src/providers.rs` (declaration and arms `wifi`, `wifi-networks`), `src-tauri/src/widget_data.rs` (`pub(crate)` on `open_wlan`, `wlan_interfaces`, `current_wifi_connection`, `WlanHandle`, `ssid_text`, `signal_percent`).
Change (`wifi`): from the connected interface's `WLAN_CONNECTION_ATTRIBUTES`: `linkSpeedMbps = ulRxRate / 1000`; `generation` from `dot11PhyType` using the crate constants (`dot11_phy_type_he` gives `wifi6`, `dot11_phy_type_eht` gives `wifi7`, `dot11_phy_type_vht` gives `wifi5`, `dot11_phy_type_ht` gives `wifi4`, older known types `legacy`, else null); `WlanGetNetworkBssList` for the connected SSID: the entry whose BSSID equals the association BSSID gives `rssiDbm` (`lRssi`) and `bandGhz` from `ulChCenterFrequency` in kilohertz (below 3000000 is 2.4, 4900000 to 5900000 is 5, 5925000 and above is 6); `ipv4` from `GetAdaptersAddresses(AF_INET)` matching the adapter name to the interface GUID, first unicast address. The BSSID is used only for matching and never serialized. `accessDenied` and the permission statuses follow 5.2 and BE-10.
Change (`wifi-networks`): `WlanGetAvailableNetworkList`, skip empty SSIDs, de-duplicate by SSID keeping the strongest, at most 12 sorted by signal; `secured` from `bSecurityEnabled`; `known` from the has-profile flag; `connected` from the connected flag. Tokens `wifi-1` to `wifi-n` are registered in a private `Mutex<HashMap<String, KnownNetwork>>` replaced on every snapshot (the profile name stays native). `WlanScan` is not called here.
Tests: `generation_labels_cover_known_phy_types`, `band_from_centre_frequency`, `rate_converts_kbps_to_mbps`, `networks_are_deduplicated_sorted_and_bounded`, `tokens_are_replaced_each_snapshot`.
CU: yes (the card shows real networks; remember the location-consent coupling).

**BE-33 wifi-actions.** Size M. Parallel: serialize on `widget_data.rs` and `wifi.rs`.
Files: `src-tauri/src/wifi.rs`, `src-tauri/src/providers.rs` (action arms), `src-tauri/src/widget_data.rs` (`settings:wifi` route).
Change: `wifi:scan` calls `WlanScan` for the interface at most once per 10 seconds (a private `Mutex<Instant>`; otherwise `Err("Scan was requested recently")`). `wifi:connect:<token>` resolves the token issued by the last `wifi-networks` snapshot, rejects unissued tokens and networks without a saved profile (`Err("Open Wi-Fi settings to join this network")`), then calls `WlanConnect` with `wlan_connection_mode_profile` and returns as soon as the request is accepted (it never reports the connection as established). `wifi:disconnect` calls `WlanDisconnect`. No profile is created and no password is accepted. `settings:wifi` opens `ms-settings:network-wifi`.
Tests: `scan_is_rate_limited`, `connect_rejects_unissued_tokens`, `connect_rejects_networks_without_a_profile`, `action_grammar_accepts_only_known_wifi_actions`, `settings_wifi_has_the_exact_route`.
CU: yes: disconnect and reconnect a saved network that the tester owns and record the result. Never automated.

**BE-34 installed-apps-api.** Size M. Parallel: yes (one engine line).
Files: `protonsearch/src/search.rs` (`pub(crate) fn installed_apps()` becomes `pub fn installed_apps()`, line 6995), `src-tauri/src/installed_apps.rs` (new), `src-tauri/src/providers.rs` (declaration).
Change: `catalog()` runs `protonsearch::search::installed_apps()` on a dedicated thread (the engine function initializes its own apartment), maps `AppInfo { name, path }` to `{key, name, parsing}` with `key("app", &path)`, drops empty names, de-duplicates by key, sorts by lowercase name and caches for 5 minutes in a private static. `resolve(key) -> Option<String>` returns the parsing name; `launch_target(key) -> Option<String>` returns `shell:AppsFolder\<parsing>` (the string the engine issues at `search.rs:3575`). `rows(query, limit) -> Vec<Row>` filters by case-insensitive substring and returns at most `limit` rows `{key, name}`; icons are attached in BE-35.
Tests: `keys_are_stable_unique_and_sorted_by_name`, `query_filters_case_insensitively_and_respects_limit`, `unknown_keys_do_not_resolve`, `catalog_cache_is_reused_within_five_minutes` (injected clock).
CU: no.

**BE-35 app-icons.** Size L. Parallel: yes.
Files: `src-tauri/src/app_icons.rs` (new), `src-tauri/tests/app_icons.rs` (new), `src-tauri/src/providers.rs` (declaration).
Change: `icon_png(parsing: &str, px: u32) -> Option<Vec<u8>>` (the caller validated `px` against the ladder): on a dedicated STA thread with a drop guard, `SHCreateItemFromParsingName` on `shell:AppsFolder\<parsing>` as an `IShellItemImageFactory`, `GetImage(SIZE { cx: px, cy: px }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)`, read the `HBITMAP` through `GetObjectW` and `GetDIBits` into BGRA, delete the bitmap through a guard, un-premultiply only if `is_premultiplied` says so, resize with Lanczos3 only when larger than `px` (never upscale), encode PNG RGBA8. Pure helpers: `bgra_to_rgba`, `unpremultiply`, `fit_size(source, px)`, `is_premultiplied(&[u8])` (every colour channel at most its alpha channel and at least one translucent pixel). Cache `(key, px)` to PNG bytes, cleared on catalog refresh. `SHARPEN: bool = false`.
Tests: `unpremultiply_restores_straight_alpha`, `fit_size_never_upscales`, `premultiplied_detection`, `bgra_to_rgba_swaps_channels`; ignored `app_icons_spike` renders the first AppsFolder item to a file in the temporary directory for visual inspection.
CU: yes: pinned tiles at 100, 125 and 150 percent beside the Windows taskbar icons, over a light and a dark rail (settles the alpha question).

**BE-36 pinned-apps-snapshot.** Size M. Parallel: serialize on `installed_apps.rs`.
Files: `src-tauri/src/installed_apps.rs`, `src-tauri/src/providers.rs` (action arm `app:launch:`).
Change: `pinned_rows(keys, icon_px) -> Value` validates every key with `keys::is_key("app", k)` (else `Err("Invalid app key")`), at most 24, order preserved, `icon_px` limited to 32, 48, 64, 96, 128, 256 (else `Err("Unsupported icon size")`), icons from BE-35 or null; returns `{items, missing}` and the envelope is `live` when at least one item resolves, otherwise `empty`. `app:launch:<key>` resolves with `launch_target` and calls `protonsearch::launcher::launch`; an unresolved key gives `Err("App no longer installed")`. The commands `installed_apps_snapshot` and `pinned_apps_snapshot` are registered in BE-49.
Tests: `invalid_keys_are_rejected`, `more_than_twenty_four_keys_are_rejected`, `missing_keys_are_reported_in_order`, `icon_px_is_limited_to_the_ladder`, `launch_rejects_unresolved_keys`.
CU: yes: pin from the Customizer, launch from the dock.

**BE-37 notifications-spike.** Size S. Parallel: yes (Cargo.toml: serialize). SPIKE.
Files: `src-tauri/tests/notifications_spike.rs` (new, `#[ignore]`), `src-tauri/Cargo.toml` (features `UI_Notifications_Management`, `UI_Notifications`, `ApplicationModel`).
Change: initialize WinRT, read `UserNotificationListener::Current()?.GetAccessStatus()`, call `GetNotificationsAsync(NotificationKinds::Toast).get()` and print only the status and the number of notifications (never any text), then attempt a `NotificationChanged` registration and print the error code (the community report expects `0x80070490`, [E13]).
Pass: the status is `Allowed` and the read returns without error from the unpackaged test process. Result recorded in `docs/qa/STATUS.md`. If it fails, BE-38 is cancelled and the widget keeps its `permission-denied` or `unsupported` state. CU: no.

**BE-38 notifications-provider.** Size M. Parallel: yes. Needs: BE-37 passed and BD-14 not required.
Files: `src-tauri/src/notifications.rs` (new), `src-tauri/src/providers.rs` (declaration, provider `notifications`, action arms), `src-tauri/tests/notifications.rs` (new, pure parts only).
Change: `count()` returns `{access, count}` (capped at 99). `list(show_text)` returns at most 20 items, newest first: `id` is `notif-<n>` registered in a private map to the native id for the current generation, `appName` from `AppInfo().DisplayInfo().DisplayName()`, `timestampMs` from `CreationTime`, `title` and `body` from the first and second text elements of the toast binding only when `show_text` is true (title cut to 80 characters, body to 200) and null otherwise. `notifications:dismiss:<id>` resolves the id and calls `RemoveNotification`; `notifications:clear` calls `ClearNotifications`. Access other than `Allowed` gives `permission-denied` with "Allow notification access in Windows settings". Nothing is stored or logged. The command `notifications_list_snapshot` is registered in BE-49.
Tests (pure helpers): `text_is_truncated_to_the_limits`, `list_hides_text_unless_requested`, `ids_are_replaced_each_generation`, `count_is_capped_at_ninety_nine`, `dismiss_rejects_unissued_ids`.
CU: yes, with the tester's explicit consent to read their own notifications, `showText` false by default.

**BE-39 calendar-week.** Size M. Parallel: serialize on `local_calendar.rs` after BE-07.
Files: `src-tauri/src/calendar_ics.rs`, `src-tauri/src/local_calendar.rs`.
Change: `calendar_snapshot` data gains `week: { start: "YYYY-MM-DD", days: [ { date, count } x 7 ] }` (the local week containing today, Monday first, counts of events overlapping each local day, capped at 99) and `today: [events]` (at most 10 events overlapping today, each bounded like the existing events). `events` stays at most three. Parsing limits and the explicit failures for recurrence and time zones are unchanged. Dates use `chrono::Local`.
Tests (inline): `week_counts_events_per_local_day`, `all_day_events_use_exclusive_end_dates`, `today_is_bounded_to_ten_events`, `week_starts_on_monday`.
CU: yes with the calendar card.

### 9.7 Phase D: search extras (after the Spotlight UI work, UI document U5)

**BE-40 search-actions.** Size M. Parallel: serialize on `main.rs`.
Files: `src-tauri/src/reveal.rs` (new), `src-tauri/tests/reveal.rs` (new), `src-tauri/src/main.rs`.
Change: `execute_search_result` accepts `actionId` of `open`, `reveal` and `copy-path`. `reveal::target_path(command: &str) -> Option<PathBuf>` returns the path only when the stored command is an absolute local path (no UNC, no relative, the rules of `known_folders::is_local_absolute_path`) that exists. `reveal::reveal(path)` uses `SHParseDisplayName` and `SHOpenFolderAndSelectItems` (no child process). `copy-path` calls the existing `copy_text`. A result without a path returns `Err("This result has no file path")`. The command string always comes from the engine's result map.
Tests: `target_path_accepts_only_existing_absolute_local_paths`, `target_path_rejects_urls_ms_settings_and_unc`.
CU: yes: reveal a file result in Explorer, copy its path.

**BE-41 search-preview.** Size M. Parallel: serialize on `main.rs`.
Files: `src-tauri/src/preview.rs` (new), `src-tauri/tests/preview.rs` (new), `src-tauri/src/main.rs` (command, registered in BE-49).
Change: `preview(path) -> SearchPreview`: for allowlisted text extensions (`txt`, `md`, `rs`, `ts`, `tsx`, `js`, `json`, `toml`, `yaml`, `yml`, `py`, `html`, `css`, `csv`, `log`, `ini`, `xml`) the first 2048 characters read from at most 8 KiB as lossy UTF-8; for other indexed files the first 2048 characters of `files_fts.content` for that path through a read-only connection (6.6); otherwise `{kind: "none"}` with size and time from file metadata. Directories and reparse points give `none`.
Tests: `text_extensions_are_allowlisted`, `preview_is_bounded_to_2048_characters`, `binary_extensions_give_none`, `missing_files_give_none`.
CU: yes with the Quick Look UI.

**BE-42 search-index-status.** Size S. Parallel: serialize on `main.rs`.
Files: `src-tauri/src/index_status.rs` (new), `src-tauri/src/main.rs`.
Change: `status() -> SearchIndexStatus`: `progress` is the `INDEXING_PROGRESS` text cut to 120 characters; `fileCount` is `SELECT COUNT(*) FROM files` through the read-only connection, cached 10 seconds, null on any error.
Tests (inline): `progress_text_is_truncated`, `file_count_is_null_when_the_database_is_missing`.
CU: no (shown in the Spotlight footer UI).

### 9.8 Phase E: wiring and hardening

**BE-49 register-bespoke-commands.** Size M (the planner may split by command). Parallel: no.
Files: `src-tauri/src/main.rs`, `src-tauri/build.rs`, `src-tauri/permissions/commands.toml`.
Change: after the modules exist, register in one change `media_artwork`, `note_save`, `weather_geocode`, `weather_locate`, `installed_apps_snapshot`, `pinned_apps_snapshot`, `notifications_list_snapshot` (if BE-38 landed), `search_preview` (if BE-41), `search_index_status` (if BE-42). Each is a thin async wrapper: validate arguments with the bounds of 5.4, `spawn_blocking`, call the module, map errors to the strings of 5.4. Add each to `generate_handler!`, the `build.rs` manifest list and the permission set named in 5.8 (until BE-50, `installed_apps_snapshot`, `weather_geocode` and `weather_locate` go in `dock-commands`, which the settings window shares).
Tests: pure validators `validate_geocode_query`, `validate_keys`, `validate_icon_px`; the BE-03 test must pass.
CU: yes, each wired feature through its UI.

**BE-50 capability-split.** Size M. Parallel: no.
Files: `src-tauri/capabilities/dock.json`, `src-tauri/capabilities/settings.json` (new), `src-tauri/permissions/commands.toml`.
Change: `dock.json` windows become `["dock"]`. `settings.json`: identifier `settings`, windows `["settings"]`, permissions `core:event:allow-listen`, `core:event:allow-unlisten`, `settings-commands`, `calendar-read`. `settings-commands` is the dock set plus `installed_apps_snapshot`, `weather_geocode` and `weather_locate` (moved out of `dock-commands`). `calendar-settings.json` stays.
Tests: extend the BE-03 test: every permission set named in a capability file exists; a table in the test lists the commands each window may call.
CU: yes: dock, settings (calendar picker, Customizer), widget card and search all still work.

**BE-51 trash-confirm.** Size S, optional (BD-10). Parallel: serialize on `widget_data.rs`.
Files: `src-tauri/src/widget_data.rs`.
Change: `trash:empty` shows a native `MessageBoxW` (Yes and No, `MB_ICONWARNING | MB_DEFBUTTON2`) before `SHEmptyRecycleBinW`; No returns `Err("Cancelled")`. `needs_confirmation(action) -> bool` is the testable part.
Tests: `only_trash_empty_needs_confirmation`. CU: yes.

**BE-52 security-guards.** Size S. Parallel: yes.
Files: `src-tauri/tests/security_guards.rs` (new).
Change: a test that reads every `src-tauri/src/*.rs` file as text and fails on: `TcpListener::bind`, `UdpSocket::bind`, `tauri_plugin_localhost`, `tiny_http`; the words `powershell`, `netsh`, `wmic` outside comments; any `Command::new(` whose line is not `notepad.exe` and is not followed within three lines by `creation_flags(0x08000000)`; any `SendInput` outside `start_menu.rs`; the characters U+2014 and U+2013 anywhere. The allowlist is a constant in the test.
Tests: `no_listening_sockets`, `no_shelling_out_to_console_tools`, `console_children_hide_their_window`, `input_injection_is_confined`, `no_em_or_en_dashes`. CU: no.

### 9.9 Phase F: gated and blocked packages

Each needs the named decision (Section 10) before work starts.

| Id | Goal | Files (1 to 3) | Needs | Size | Notes |
|---|---|---|---|---|---|
| BE-60 | DPAPI credential store (6.8) | `src-tauri/src/credentials.rs`, `src-tauri/tests/credentials.rs`, `src-tauri/Cargo.toml` | BD-13 | M | Tests: `protect_unprotect_round_trip_in_memory`, `entropy_mismatch_fails`, `file_never_contains_the_secret`, `provider_ids_are_allowlisted`. Commands `credential_*` are wired like BE-49. |
| BE-61 | Anthropic and OpenAI Admin usage (org spend only) | `src-tauri/src/admin_usage.rs`, `src-tauri/tests/admin_usage.rs` | BD-13 (recommended: never) | M | Adds two outbound hosts to 7.1 item 3. No quota percentage exists in these APIs. |
| BE-62 | AirPods left, right and case levels from advertisements | `src-tauri/src/airpods.rs`, `src-tauri/tests/airpods.rs`, `src-tauri/Cargo.toml` | BD-4 | L | `BluetoothLEAdvertisementWatcher` filtered to company `0x004C`, parse the proximity message nibbles, scan only while the card is open in 5 s windows, discard everything that is not the strongest signal above a threshold. Opt-in setting. |
| BE-63 | System tray: invoke Windows' hidden-icons flyout by UI Automation (spike first) | `src-tauri/tests/tray_uia_spike.rs`, then `src-tauri/src/tray.rs` | BD-5 | M | Feature `Win32_UI_Accessibility` (present). Needs Explorer's taskbar visible. |
| BE-64 | System tray: "Background apps" from `HKCU\Control Panel\NotifyIconSettings` cross-checked with running processes | `src-tauri/src/tray_apps.rs`, `src-tauri/tests/tray_apps.rs` | BD-5 | M | Feature `Win32_System_Registry` (present). Honest label, no tooltips, no menus. |
| BE-65 | Port clipboard ingestion into the engine library | `protonsearch/src/clipboard_listener.rs`, `protonsearch/src/lib.rs`, `src-tauri/src/main.rs` | BD-12 | L | Touches the legacy host (`protonsearch/src/main.rs:1984`, `11388`); message-only window thread under 7.7. |
| BE-66 | ICS recurrence and time-zone subset | `src-tauri/src/calendar_ics.rs`, `src-tauri/Cargo.toml` | BD-11 | L | RRULE `FREQ`, `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY`; `TZID` needs a time-zone source (a crate or Windows data); decide before work. |
| BE-67 | Assistant | none yet | BD-6 | L | Needs a product definition first. |
| BE-68 | `protondock.db` foundation and notes table (6.7) | `src-tauri/src/db.rs`, `src-tauri/tests/db.rs`, `src-tauri/Cargo.toml` | BD-2 | M | Replaces BE-20 if accepted. |
| BE-69 | Toast-count badges on running apps | `src-tauri/src/app_badges.rs`, `src-tauri/src/running_apps.rs` | BD-14, BE-38, BE-13 | M | Map a window's explicit application user model id (`SHGetPropertyStoreForWindow`) to the toast `AppUserModelId`; label as notifications. |
| BE-70 | AppBar work-area reservation | `src-tauri/src/appbar.rs`, `src-tauri/src/main.rs` | BD-9 | L | `SHAppBarMessage` `ABM_NEW`, `ABM_QUERYPOS`, `ABM_SETPOS`, `ABM_REMOVE`; dedicated message thread (7.7); native check at 100, 125 and 150 percent and with two monitors. |
| BE-71 | Optional Explorer taskbar auto-hide with the existing lease and recovery | `src-tauri/src/taskbar.rs`, `src-tauri/src/main.rs` | BD-9, BE-70 | M | `ABM_SETSTATE` with `ABS_AUTOHIDE`, restore on exit and through `--restore-taskbar`; never `ShowWindow(SW_HIDE)` (it breaks Start and the tray). |

## 10. Decisions needed from the user

Each decision has a recommended default so work can continue with the default if the user does not answer.

| Id | Decision | Recommended default | Why | If the answer differs | Packages |
|---|---|---|---|---|---|
| BD-1 | Read `chatgpt` as an alias of `codex` | Yes | Both are built from the same local Codex rollout log; two tiles would show identical numbers. `agent` stays separate. | Keep both; they duplicate each other. | BE-06, BE-12 |
| BD-2 | Create a SQLite `protondock.db` in v1 | No: notes are `.txt` files in the engine's notes folder; layout stays JSON | Every datum already has a home; a second store adds recovery paths and a second notes system. | Yes: BE-68 and the DDL of 6.7, and BE-20 is dropped. | BE-20, BE-68 |
| BD-3 | Wi-Fi scope | Read, saved-network connect and disconnect, Settings handoff; no new-network password entry, no radio toggle | A password through the renderer and an unverified radio toggle are the risky parts. | Radio toggle: run a spike on `Radio::SetStateAsync` or `WlanSetInterface` first. Password entry is not recommended. | BE-32, BE-33 |
| BD-4 | AirPods left, right and case levels from advertisements | No: one level per device only | Reverse-engineered format, random addresses, sees strangers' devices. | Yes: BE-62 as an opt-in setting. | BE-62 |
| BD-5 | System tray | Remove from default profiles; if wanted, try the Windows flyout by UI Automation (BE-63), else "Background apps" (BE-64) | The specified enumeration does not exist on this build and a tray replica is replacement-shell work. | Accept option C (host the tray): a separate program. | BE-63, BE-64 |
| BD-6 | Assistant | Unconfigured tile only; no backend | No product definition; a waveform or transcript would be fabricated. | Define the product first (speech to text, model, where it runs). | BE-67 |
| BD-7 | Unknown widget ids in saved layouts | Follow the UI document: keep ids in `unknown` | Matches the contract minimally and costs nothing. | Keep the raw item in place so position, size and settings survive: extra work in U1.2. | none |
| BD-8 | Weather default | Unconfigured by default, after materializing the old settings for existing items | A hard-coded city shown as live weather breaks the honesty rule. | Keep Ahmedabad as the default and label it as a default. | XD-5 |
| BD-9 | Taskbar replacement | v1 keeps Explorer's taskbar and the dock above it; later an AppBar, then optional auto-hide | Start, the hidden-icons flyout and toasts depend on Explorer's shell. | Hide it now: needs BE-70 and BE-71 and a native check at three DPIs and two monitors first. | BE-70, BE-71 |
| BD-10 | Native confirmation for `trash:empty` | Yes | The action is irreversible and bypasses Windows' own confirmation. | Rely on the UI confirmation only. | BE-51 |
| BD-11 | Calendar recurrence and time zones | Defer; keep the ICS subset and report limits honestly | Needs a time-zone source and RRULE expansion. | Build BE-66 first if real calendars are the goal. | BE-66 |
| BD-12 | Clipboard history widget | Defer until ingestion is ported | The engine table is only filled by the legacy host. | Port the listener (BE-65) before adding a widget. | BE-65 |
| BD-13 | AI usage credentials and Admin APIs | None: local logs only; Claude shows tokens; never use the undocumented OAuth endpoint | No API gives subscription quota percentages with a key; the OAuth route is a policy and security risk. | Org spend only: BE-60 and BE-61 with the outbound hosts listed in 7.1. | BE-60, BE-61 |
| BD-14 | Toast-count badges on running apps | No | The API gives toasts, not badge numbers; the counts would be a different, smaller number than the app's own badge. | Yes: BE-69 after BE-38. | BE-69 |

## Appendix A. Evidence log

Diagnostics run on 2026-11-08 on this machine (Windows 11 Home Single Language, `10.0.26300.0`). All were read-only and none launched a project binary; none read notification content, network names or device names.

| Id | Evidence |
|---|---|
| E1 | Repository reads, cited inline as `file:line`. |
| E2 | A PowerShell script using `FindWindow("Shell_TrayWnd")`, `EnumChildWindows`, `GetClassName` and `IsWindowVisible`: the taskbar is visible; descendants are `Windows.UI.Composition.DesktopWindowContentBridge`, `Windows.UI.Input.InputSite.WindowClass`, `Windows.UI.Core.CoreWindow` (hidden), `Start` (hidden), `TrayDummySearchControl`, `TrayNotifyWnd`, `ReBarWindow32`, `MSTaskSwWClass`, `MSTaskListWClass` (hidden). No `SysPager`, no `ToolbarWindow32`. `FindWindow("NotifyIconOverflowWindow")` and `FindWindow("TopLevelWindowForOverflowXamlIsland")` returned no window. `HKCU\Control Panel\NotifyIconSettings` has 70 subkeys. |
| E3 | From unpackaged `powershell.exe`: `UserNotificationListener.Current.GetAccessStatus()` returned `Allowed`; `Geolocator.LocationStatus` returned `NotInitialized`. |
| E4 | From unpackaged `powershell.exe`: `Battery.AggregateBattery.GetReport()` returned status Idle, charge rate 0 mW, full charge 83002 mWh, design 83002 mWh, remaining 70152 mWh. |
| E5 | `Get-PnpDevice -Class Bluetooth` returned 31 devices; 2 returned a value for `Get-PnpDeviceProperty -KeyName '{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2'`. Counts only. |
| E6 | `netsh wlan show interfaces` returned an SSID line and did not print the location-permission message (consent is granted here). Booleans only. |
| E7 | Microsoft Learn, "Changes to API behavior for Wi-Fi access and location": `https://learn.microsoft.com/en-us/windows/win32/nativewifi/wi-fi-access-location-changes`. Without precise-location consent `WlanGetAvailableNetworkList`, `WlanGetNetworkBssList`, `WlanQueryInterface` and `WlanScan` return `ERROR_ACCESS_DENIED`; `Geolocator.RequestAccessAsync` must run in the user's context outside `C:\Windows\System32`; redirect denied users to `ms-settings:privacy-location`. |
| E8 | Microsoft Learn, "Notification listener": `https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/notification-listener`. The User Notification Listener capability must be in the app manifest; access is requested with `RequestAccessAsync`; only app notifications are supported. |
| E9 | Microsoft Learn, "Modernize packaged desktop apps": `https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/modernize-packaged-apps`. Features that require package identity; apps packaged with external location also have identity. |
| E10 | Microsoft Learn, `AppointmentManager.RequestStoreAsync`: `https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.appointments.appointmentmanager.requeststoreasync`. Capability `appointmentsSystem`. |
| E11 | Microsoft Learn, `Radio.SetStateAsync`: `https://learn.microsoft.com/en-us/uwp/api/windows.devices.radios.radio.setstateasync`. Capability `radios`; call `RequestAccessAsync` first. |
| E12 | Microsoft Learn, `Win32_Battery`: `https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-battery`. Property list has `DesignCapacity`, `DesignVoltage`, `FullChargeCapacity`; no `CycleCount`, no `Voltage`. |
| E13 | `https://github.com/microsoft/WindowsAppSDK/issues/6172`: in an unpackaged app `RequestAccessAsync` returns Allowed and `GetNotificationsAsync` works, `NotificationChanged` throws `0x80070490`; a WNF workaround is mentioned. Community report. |
| E14 | `https://github.com/librepods-org/librepods/issues/293` ("You'd need a kernel-level driver to open l2cap sockets for AACP"); `https://github.com/brianpht/librepods-rs` (BLE advertisement decoding of model, battery and lid state, random addresses, noise control over L2CAP). Community reports. |
| E15 | Anthropic Usage and Cost API requires an Admin key and an organization: `https://www.toriihq.com/articles/how-to-monitor-spending-claude`, `https://www.reddit.com/r/ClaudeAI/comments/1mtsbb2/usage_and_cost_api_now_available/` ("The Admin API is unavailable for individual accounts"). |
| E16 | `https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor/issues/202` (undocumented `GET /api/oauth/usage`, same data as `/usage`), `https://github.com/anthropics/claude-code/issues/31021` and `/31637` (aggressive rate limiting), `https://news.ycombinator.com/item?id=46549823` (Anthropic blocked third-party use of Claude Code subscription tokens). Community reports; not legal advice. |
| E17 | OpenAI Usage API: `https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/completions` (Admin key). |
| E18 | Shell command lists: `https://winaero.com/windows-11-shell-commands-the-complete-list/` and `https://www.kapilarya.com/list-of-shell-commands-and-clsids-for-windows-10`: `shell:::{2559a1f8-21d7-11d4-bdaf-00c04f60b9f0}` is Windows Search. |
| E19 | `https://stackoverflow.com/questions/71736070/how-can-i-get-the-bluetooth-device-battery-percentage-using-powershell-on-window`: the property key `{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2` returns the battery percentage. |
| E20 | `https://github.com/dremin/RetroBar/issues/730` and `/737`: a replacement taskbar on Windows 11 shows inconsistent tray icons and unresponsive system icons. Community reports. |
| E21 | Local `windows` 0.58.0 source in the cargo registry: every feature name and function used in Sections 4 and 9 exists (`Devices_Power`, `System_Power`, `ApplicationModel`, `Storage_Streams`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_Devices_DeviceAndDriverInstallation`, `Win32_Devices_Properties`, `Devices_Geolocation`, `UI_Notifications_Management`, `UI_Notifications`, `Win32_Security_Cryptography`, `Devices_Bluetooth`, `Devices_Bluetooth_Advertisement`, `Devices_Bluetooth_GenericAttributeProfile`, `Win32_System_Ioctl`, `Win32_System_IO`, `Win32_UI_Accessibility`, `Win32_System_Registry`, `Win32_System_Performance`; `CallNtPowerInformation`, `SYSTEM_BATTERY_STATE`, `SendInput`, `VK_LWIN`, `CryptProtectData`, `CryptUnprotectData`, `Geolocator`, `UserNotificationListener`, `Battery::AggregateBattery`, `IShellItemImageFactory`, `SHGetImageList`, `SHAppBarMessage`, `WlanScan`, `WlanConnect`, `WlanGetNetworkBssList`, `WlanSetProfile`, `SetWinEventHook`, `IOCTL_BATTERY_QUERY_INFORMATION`). |
| E22 | `libsqlite3-sys` 0.28.0 `build.rs` enables `SQLITE_ENABLE_FTS5`, `SQLITE_ENABLE_JSON1`, `SQLITE_THREADSAFE=1`, `SQLITE_DEFAULT_FOREIGN_KEYS=1`; `sqlite3.h` is SQLite 3.45.0; `rusqlite` 0.31.0 `bundled` feature present. |

## Appendix B. Repo defects found while reconciling (not fixed by this document)

| # | Defect | Evidence | Where it is handled |
|---|---|---|---|
| 1 | Every calendar command fails against a version 2 layout ("Native dock profile could not be validated"); only profiles `daily` and `founder` are accepted | `local_calendar.rs:37-43`, `64-79`, `86` | BE-07 |
| 2 | A hotkey registration failure aborts startup, so another program holding `Alt+Space` stops the dock from starting | `main.rs:1099-1102` | BE-08 |
| 3 | `build.rs` omits five commands that `commands.toml` allows | `build.rs`, `permissions/commands.toml` | BE-03 |
| 4 | TypeScript `WidgetStatus` lacks `unsupported`, which Rust emits | `src/widgets/sdk/types.ts:3`, `widget_data.rs:91-107` | XD-1 |
| 5 | `useFinalWidgetData` subscribes to every provider for every profile | `useFinalWidgetData.ts:357-364` | XD-11 |
| 6 | An access-denied Wi-Fi query reads as `disconnected` | `widget_data.rs:577-585` | BE-10 |
| 7 | The weather default is a hard-coded city, enabled | `meta.ts:264` | XD-5, BD-8 |
| 8 | `unknown` drops position, size and settings and nothing renders it | `layout.ts:92-104` | BD-7 |
| 9 | Bluetooth radio state is sampled once per process because repeated reads crash under windows 0.58 | `widget_data.rs:637-673` | BE-31 |
| 10 | `agent` publishes the user's last prompt text and workspace path to the renderer; `parse_rollout` re-reads the whole newest log each minute | `ai_usage.rs:152-212`, `295-310` | privacy note in 7.2; no change scheduled |
| 11 | The engine reads an optional plaintext `ai_key.txt` and supports build-time embedded keys obfuscated with XOR | `protonsearch/src/ai.rs:17-52` | out of scope; flagged to the owner |
| 12 | `running_apps_snapshot` holds the shared engine mutex while it enumerates windows, so a 2 s poll could delay a search keystroke | `main.rs:619-635` | measure before adopting the 2 s cadence (4.4.3) |

