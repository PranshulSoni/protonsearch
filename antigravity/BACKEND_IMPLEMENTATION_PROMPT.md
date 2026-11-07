# Master Backend & Database Implementation Prompt: ProtonDock Full Wiring

> Authoritative Architecture Directive for Backend & Systems Implementation Agents  
> Target Repository: `protonsearch` (Windows 11 Native Dock & Kensaku Search Engine)  
> Technology Stack: Rust (Win32 API + WinRT + Direct2D) + Tauri v2 (IPC) + SQLite 3 WAL + React 19 Frontend  
> Authoritative Design Ground Truth: `dock-editable.html` (DOM, CSS, FLIP Springs, 48px Height Invariant)  
> Authoritative System Specs: `antigravity/spec.md` and `.design.md`  

---

## 1. Executive Mission & System Boundaries

You are tasked with the complete, production-grade backend and database wiring for **ProtonDock**. All frontend visuals, CSS layout invariants, Apple spring physics, popovers, and interactive widgets have already been prototyped and validated in `dock-editable.html`.

Your responsibility is to build the native Windows 11 subsystem hooks, SQLite database persistence layer, Tauri v2 IPC bridge, and settings controllers so that every single widget and configuration setting connects directly to real operating system APIs and local storage.

### Strict Workspace Constraints & Behavioral Rules
1. **Rule 4 (Never run launcher in background)**: Do NOT execute `cargo run`, `cargo run --release`, or `protonsearch.exe` as persistent background tasks in the agent workspace. Running the daemon locks the executable (`os error 5`) and blocks subsequent builds. Let the user launch the binary manually or run one-shot verification commands.
2. **Rule 7 (Kill before edit or build)**: Always run `Stop-Process -Name protonsearch -Force -ErrorAction SilentlyContinue` before rebuilding or editing binaries.
3. **Suppress Console Windows**: All background child process executions (git, CLI helpers, indexers) must configure `.creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) to prevent command windows from flashing on the user's screen.
4. **Win32 & GDI Safety**: Safely release COM pointers, device contexts (`ReleaseDC`), and font handles (`DeleteObject`) during shutdown and window destruction (`WM_DESTROY`).
5. **Anti-Slop Discipline**: Zero em dashes anywhere in code, markdown, comments, or UI copy. Use colons, parentheses, or commas. Clean, minimal, senior-engineer craftsmanship.

---

## 2. SQLite Database Architecture & Schema Migrations

All local state, user profiles, widget configurations, notes, and search indexes must reside in an authoritative SQLite database at `%APPDATA%\ProtonSearch\protondock.db`.

### 2.1 Connection Pragma Configuration
To prevent `database is locked` (error code 5) under multi-threaded concurrency (Tauri async commands + background indexer threads), every SQLite connection must initialize with:
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
PRAGMA foreign_keys = ON;
```

### 2.2 Relational Schema Definition

```sql
-- 1. Profiles Table
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    widget_order TEXT NOT NULL, -- JSON array of widget IDs: ["start","search",...]
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 2. Widget Settings Table
CREATE TABLE IF NOT EXISTS widget_settings (
    widget_id TEXT PRIMARY KEY,
    width INTEGER NOT NULL DEFAULT 48,
    enabled INTEGER NOT NULL DEFAULT 1,
    settings_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL
);

-- 3. Pinned Apps Catalog
CREATE TABLE IF NOT EXISTS pinned_apps (
    app_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    exec_path TEXT NOT NULL,
    app_user_model_id TEXT,
    icon_blob BLOB,
    order_index INTEGER NOT NULL DEFAULT 0,
    pinned_at INTEGER NOT NULL
);

-- 4. Quick Notes Storage
CREATE TABLE IF NOT EXISTS quick_notes (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    char_count INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 5. Kensaku Search FTS5 Virtual Table
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5 (
    path UNINDEXED,
    filename,
    extension,
    extracted_text,
    ocr_text,
    tokenize = 'unicode61'
);

-- 6. Clipboard History Cache
CREATE TABLE IF NOT EXISTS clipboard_history (
    id TEXT PRIMARY KEY,
    text_content TEXT NOT NULL,
    char_length INTEGER NOT NULL,
    source_app TEXT,
    created_at INTEGER NOT NULL
);
```

### 2.3 Seed Defaults Migration
Seed the initial database on first run with the four standard profiles matching `dock-editable.html`:
- `daily`: `["start","search","peripherals","system-stats","calendar","running-apps","music-player","weather","clock"]` (active)
- `work`: `["start","search","calendar","system-stats","running-apps","pinned-apps","weather","status-battery","wifi","bluetooth","notifications","quick-notes","clock"]`
- `founder`: `["start","search","claude","codex","system-stats","calendar","running-apps","pinned-apps","music-player","wifi","system-tray","clock"]`
- `focus`: `["start","search","calendar","system-stats","music-player","quick-notes","clock"]`

---

## 3. Exhaustive Widget-by-Widget Backend Wiring Contract

Every widget in `dock-editable.html` must be backed by a concrete Rust implementation file under `src-tauri/src/` or `protonsearch/src/`.

### 1. Windows 11 Start (`start`)
- **Action**: Clicking invokes Windows 11 Start menu directly at screen coordinates.
- **Backend File**: `src-tauri/src/system_actions.rs`
- **Native Implementation**: Win32 `SendInput` dispatching `VK_LWIN` key-down and key-up events, or `ShellExecuteW(NULL, L"open", L"shell:::{2559a1f8-21d7-11d4-bdaf-00c04f60b9f0}", NULL, NULL, SW_SHOWNORMAL)`.
- **IPC Command**: `invoke('trigger_start_menu')`

### 2. Kensaku Spotlight Search (`search`)
- **Action**: Alt+Space or clicking widget launches Kensaku spotlight search overlay.
- **Backend File**: `protonsearch/src/indexer.rs` and `src-tauri/src/kensaku_search.rs`
- **Native Implementation**:
  - Full-text search over `files_fts` with rank ordering (`bm25`).
  - In-process mathematical evaluation via high-speed recursive descent parser (evaluating equations, percentages, square roots, trigonometry).
  - Background OCR ingestion: Windows Media OCR API (`Windows.Media.Ocr.OcrEngine`) scanning `%USERPROFILE%\Pictures\Screenshots`.
  - Quick Look: Spacebar keydown queries first 2KB of text/metadata for instant preview drawer.
- **IPC Commands**:
  - `invoke('kensaku_query', { query: string, scope: string, limit: number })` -> `KensakuSearchResponse`
  - `invoke('kensaku_quick_look', { path: string })` -> `QuickLookPayload`
  - `invoke('open_file_path', { path: string })`
  - `invoke('reveal_in_explorer', { path: string })`

### 3. System Telemetry & Sparkline (`system-stats`)
- **Action**: Renders live CPU/RAM/SSD telemetry. Clicking directly on the dock toggles between smooth SVG sparkline curve and 3-tier horizontal bars.
- **Backend File**: `src-tauri/src/system_stats.rs`
- **Native Implementation**:
  - Crate: `sysinfo::System` with background sampling thread running every 2000ms.
  - Maintains rolling circular buffer of last 12 samples for SVG bezier curve generation.
  - Queries aggregate CPU utilization across all logical cores, active memory load, and primary physical disk I/O.
- **IPC Command**: `invoke('get_system_stats')` -> `{ cpu: number, ram: number, ssd: number, history: number[], peak: number }`
- **Event**: `system-stats-tick` emitted every 2000ms for zero-latency UI stream.

### 4. Apple Calendar Squircle Tile (`calendar`)
- **Action**: Apple-style crimson squircle tile synced live to OS date. Clicking opens anchored 7-day horizontal week strip popover with today highlighted and upcoming meetings.
- **Backend File**: `src-tauri/src/calendar_service.rs`
- **Native Implementation**:
  - Reads local system date via `chrono::Local::now()`.
  - Parses Windows Calendar store or local ICS files in `%APPDATA%\ProtonSearch\calendars\`.
- **IPC Commands**:
  - `invoke('get_calendar_today')` -> `{ day: string, date: string, monthYear: string }`
  - `invoke('get_week_schedule')` -> `WeekSchedulePayload`
  - `invoke('create_calendar_event', { title: string, startIso: string, durationMin: number })`

### 5. AirPods & Peripherals Battery (`peripherals`)
- **Action**: 48px square tile showing neon teal earbuds icon and percentage. Clicking opens multi-accessory battery HUD with left/right bud breakdown and ANC toggles.
- **Backend File**: `src-tauri/src/bluetooth_peripherals.rs`
- **Native Implementation**:
  - WinRT `Windows.Devices.Bluetooth.GenericAttributeProfile.GattDeviceService` reading Battery Service UUID `0x180F` (Characteristic `0x2A19`).
  - Enumerates connected Bluetooth Low Energy HID and Audio devices (AirPods Pro, Surface Earbuds, MX Master 3S, Keychron keyboards).
- **IPC Command**: `invoke('get_peripherals_telemetry')` -> `PeripheralsPayload`
- **IPC Command**: `invoke('set_anc_mode', { deviceId: string, mode: 'off' | 'transparency' | 'anc' })`

### 6. Discrete Running Applications (`running-apps`)
- **Action**: Displays running apps as discrete 48px square tiles with small 20px sharp icons, running indicator dots, and red Apple-style notification pill badges (`212`, `2`, `1`). Direct drag-and-drop reordering on the dock.
- **Backend File**: `src-tauri/src/running_apps.rs`
- **Native Implementation**:
  - Enumerates top-level desktop windows via Win32 `EnumWindows` and `GetWindowThreadProcessId`.
  - Filters out tooltips, hidden overlays, and background host processes (`WS_VISIBLE && !WS_EX_TOOLWINDOW`).
  - Jumbo Icon Extraction: Win32 `SHGetImageList(SHIL_JUMBO, ...)` downsampled via Lanczos3 filter to 20x20 device pixels with unsharp mask.
  - Notification Badges: WinRT `Windows.UI.Notifications.Management.UserNotificationListener` reading active notification count per AppUserModelID.
- **IPC Commands**:
  - `invoke('list_running_apps')` -> `RunningAppItem[]`
  - `invoke('activate_app_window', { processId: number, hwnd: number })`
  - `invoke('reorder_running_apps', { appIds: string[] })`
- **Event**: `running-apps-changed` emitted on window creation or exit via Win32 `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`.

### 7. Pinned Applications Catalog (`pinned-apps`)
- **Action**: Users pin apps from an installed application catalog. Each pinned app renders as a discrete 48px square tile.
- **Backend File**: `src-tauri/src/installed_apps.rs`
- **Native Implementation**:
  - Enumerates desktop `.lnk` shortcuts in Start Menu and modern Store apps in `shell:AppsFolder` via COM `IShellItem`.
  - Persists pinned selections in SQLite `pinned_apps` table.
- **IPC Commands**:
  - `invoke('list_installed_apps')` -> `InstalledApp[]`
  - `invoke('pin_app', { appId: string })`
  - `invoke('unpin_app', { appId: string })`
  - `invoke('launch_pinned_app', { appId: string })`

### 8. Now Playing Media Player (`music-player`)
- **Action**: 220px horizontal capsule (or 48px square). Displays album art thumbnail, track title, artist, and transport controls (`<<`, `||`/`▶`, `>>`).
- **Backend File**: `src-tauri/src/media_player.rs`
- **Native Implementation**:
  - Windows Global System Media Transport Controls (`GlobalSystemMediaTransportControlsSessionManager`) via WinRT `windows::Media::Control`.
  - Captures album art image stream, active playback state (Playing / Paused), and timeline.
  - Controls playback via `TryPlayAsync`, `TryPauseAsync`, `TrySkipNextAsync`, `TrySkipPreviousAsync`.
- **IPC Commands**:
  - `invoke('get_media_status')` -> `MediaStatusPayload`
  - `invoke('toggle_media_playback')`
  - `invoke('skip_media_next')`
  - `invoke('skip_media_prev')`
- **Event**: `media-status-changed` emitted when GSMTC session changes.

### 9. Rich Live Weather (`weather`)
- **Action**: 200px capsule displaying temperature, high/low, rain percentage, wind speed, and location selector. Clicking opens hourly forecast and air quality popover.
- **Backend File**: `src-tauri/src/weather_service.rs`
- **Native Implementation**:
  - Fetches Open-Meteo local forecasts cached in SQLite for 15 minutes.
  - Reverse geocoding for city name searches.
- **IPC Commands**:
  - `invoke('get_weather')` -> `WeatherPayload`
  - `invoke('set_weather_city', { city: string })` -> triggers geocoding and refresh.

### 10. Battery & Power Telemetry (`status-battery`)
- **Action**: 48px square battery tile with charge level fill and bolt glyph. Clicking opens HUD showing flow wattage (+48.2W charging / -16.4W draining), cycle count, health, voltage, and remaining time.
- **Backend File**: `src-tauri/src/system_battery.rs`
- **Native Implementation**:
  - Win32 `CallNtPowerInformation(SystemBatteryState, ...)` and `GetSystemPowerStatus`.
  - Calculates instantaneous flow wattage: `(Rate / 1000.0) W`.
- **IPC Command**: `invoke('get_battery_telemetry')` -> `BatteryTelemetryPayload`

### 11. AI Agent Widgets (`claude`, `codex`, `siri-ai`)
- **Action**: Dual-mode usage rings. Displays session quota percentage in resting state; transitions into revolving neon halo animation (`.is-working`) during active background execution.
- **Backend File**: `src-tauri/src/ai_agent_bridge.rs`
- **Native Implementation**:
  - Secure credential storage via Windows Data Protection API (DPAPI / `CryptProtectData`).
  - Queries usage quotas from Anthropic and OpenAI API telemetry endpoints.
  - Siri: Bridges to local Whisper STT engine or Windows Voice Recognition API.
- **IPC Commands**:
  - `invoke('save_ai_credentials', { provider: 'claude' | 'codex', apiKey: string })`
  - `invoke('get_ai_status', { provider: string })` -> `{ configured: boolean, quotaPct: number, isWorking: boolean }`
  - `invoke('toggle_agent_execution', { provider: string })`

### 12. Quick Notes Jot (`quick-notes`)
- **Action**: 48px tile with pencil/pad glyph. Clicking opens anchored card to jot quick notes.
- **Backend File**: `src-tauri/src/notes_service.rs`
- **Native Implementation**:
  - SQLite CRUD operations over `quick_notes` table.
- **IPC Commands**:
  - `invoke('list_notes')` -> `NoteItem[]`
  - `invoke('save_note', { content: string })` -> returns new `NoteItem`
  - `invoke('delete_note', { id: string })`

### 13. Connectivity & Tray Controls (`wifi`, `bluetooth`, `system-tray`, `clock`)
- **Wi-Fi**: Win32 `wlanapi.dll` (`WlanOpenHandle`, `WlanEnumInterfaces`, `WlanGetAvailableNetworkList`).
- **Bluetooth**: WinRT `BluetoothDevice` enumeration and pairing commands.
- **System Tray**: System notification tray icon enumeration.
- **Clock**: Synchronizes with local system clock, formats across `time`, `time-date`, and `analog`.

---

## 4. Multi-Profile & In-Place Direct Manipulation Wiring

### 4.1 Alt-Click (Option-Click) In-Place Morphing
- Ground truth from Ganella: Holding `Alt` while clicking any dock widget or app slot toggles it between its compact 48px square tile and its wide live widget capsule state (or vice-versa).
- Backend must save custom slot dimensions to `widget_settings.width` immediately upon toggle.

### 4.2 Dynamic Profile Morphing & Persistence
- When a user switches profiles (Daily, Work, Founder, Focus, or Custom) via the top-shelf tab bar or shortcut:
  1. Active profile ID is persisted to `profiles.is_active = 1`.
  2. Dock rail width morphs dynamically using Apple spring curves (`cubic-bezier(0.16, 1, 0.3, 1)`).
  3. Sibling slots glide horizontally using FLIP transforms.
- Adding custom profiles (`openNewProfileModal` in `dock-editable.html`) writes new rows to `profiles` table.

---

## 5. Verification & Acceptance Criteria for Implementation Agents

Before marking any backend or database task as complete, you must verify:
1. **Compilation**: `cargo check` and `cargo test` pass with zero compiler warnings or errors.
2. **Database Health**: `sqlite3 %APPDATA%\ProtonSearch\protondock.db "PRAGMA integrity_check;"` returns `ok`.
3. **Zero Window Flashing**: Launching any background child process does not flash a terminal or console window on Windows 11.
4. **48px Square Height Invariant**: No widget or sizing transition causes the dock rail or widget tiles to deviate from 48px height.
5. **Anti-Slop Compliance**: Grepping codebase for `\u2014` yields zero em dashes.
6. **Git Rule 6**: Run `git commit` and `git push origin dock` after every single verified change.
