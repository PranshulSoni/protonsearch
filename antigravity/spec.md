# ProtonDock Technical Specification & Frontend-to-Backend Architecture

> Directory: `antigravity/spec.md`
> Status: Authoritative Architecture Specification
> Authors: Antigravity Team
> Target Platforms: Windows 11 (Win32 / Rust Backend + Tauri v2 + React 19 Frontend)

---

## 1. System Architecture & Sizing Invariants

### 1.1 Sizing Contract
1. **Locked Square-Type Height (`48px`)**:
   - All dock widgets strictly share the exact same height token: `--tile-h: 48px`.
   - Rail height is derived: `calc(var(--tile-h) + var(--dock-pad) * 2)` (48px + 12px = 60px).
   - No widget is ever permitted to exceed or reduce this 48px height.
2. **Horizontal Expansion**:
   - Base shape: Square (48px width x 48px height).
   - Expanded shape: Rectangle (customizable width from 48px up to 220px).
   - When expanding, the height stays locked at 48px; only width expands horizontally.
3. **Display Modes**:
   - `Capsules`: Discrete rounded glass squircle tiles with 6px gap (`--dock-gap: 6px`).
   - `Lines`: Seamless rail with 1px translucent vertical dividers between widgets.
4. **Dock Profiles**:
   - Top-shelf switcher supporting `Daily`, `Work`, and `Founder` modes.
   - Each profile retains independent widget ordering, enabled sets, and sizing preferences.

---

## 2. Core Widget Catalog & Backend IPC Mapping

### 2.1 Weather Widget (`weather`)
* **Design & Behavior**:
  - Square Mode (48px): Centered weather glyph with temperature readout (`24°`).
  - Rectangle Mode (120px): Weather icon on left, two-line stack on right: `24° Clear` / `H:28° L:19° · San Francisco`.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/weather.rs` / `protonsearch/src/weather.rs`.
  - Native API: Windows Location API (`ISensorManager` / `ILocationReport`) for coordinates, cached query to Open-Meteo API.
* **IPC Contract**:
  - Command: `invoke('get_weather')` -> returns `WeatherPayload`.
  - Schema:
    ```typescript
    interface WeatherPayload {
      temperature: number;      // Celsius
      weatherCode: number;      // WMO code (0: Clear, 1-3: Partly Cloudy, 51-67: Rain, etc.)
      condition: string;        // Human readable ("Clear", "Partly Cloudy")
      high: number;
      low: number;
      location: string;
      cachedAt: number;         // Unix timestamp ms
    }
    ```

---

### 2.2 Status / Battery Widget (`status-battery`)
* **Design & Behavior**:
  - Square Mode (48px): Pill battery outline with green/amber fill proportional to charge, charging lightning bolt indicator, and centered text (`88%`).
  - Rectangle Mode (124px): Battery icon on left + `88% Charging` (primary) / `Power: On AC (4h 12m)` (secondary).
  - **Anchored Popover HUD (Power Delivery Telemetry)**:
    - Live charge/drain rate in Watts: `+48.2W Fast Charge` or `-16.4W Draining`.
    - Voltage (`19.8 V`) and Current (`2.43 A`).
    - Battery Health (`96%`), Cycle Count (`142 cycles`), and Full Charge Capacity vs Design Capacity (`68.2 Wh / 71.0 Wh`).
    - Internal Pack Temperature (`31.4°C`).
    - Estimated Time to Full / Time Remaining (`42 min until full`).
    - Interactive simulation button to toggle between AC Fast Charge and On Battery drain.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/system_battery.rs`.
  - Native APIs:
    - Win32 `GetSystemPowerStatus(&mut SYSTEM_POWER_STATUS)`.
    - Native `CallNtPowerInformation(SystemBatteryState, ...)`:
      - Field `Rate`: signed integer reporting milliwatts (mW) entering or leaving the battery pack.
      - Wattage calculation: `watts = (Rate as f64) / 1000.0`. Positive for charging, negative for discharging.
    - WMI query to `SELECT DesignCapacity, FullChargeCapacity, CycleCount, EstimatedChargeRemaining, Voltage FROM Win32_Battery`.
* **IPC Contract**:
  - Command: `invoke('get_battery_status')` -> returns `BatteryPayload`.
  - Event: Listen on Win32 message `WM_POWERBROADCAST` to emit Tauri event `battery-changed`.
  - Schema:
    ```typescript
    interface BatteryPayload {
      percent: number;          // 0 - 100
      charging: boolean;        // true if AC connected or flag charging
      acConnected: boolean;
      timeRemainingSec: number | null; // null if on AC or calculating
      healthPercent: number;    // From WMI Win32_Battery or estimate
      wattageRate: number;      // Live power rate in Watts (+W charging, -W draining)
      voltageVolts: number;     // e.g. 19.8
      amperageAmps: number;     // e.g. 2.43
      cycleCount: number;       // e.g. 142
      fullCapacityWh: number;   // e.g. 68.2
      designCapacityWh: number; // e.g. 71.0
      temperatureC: number;     // e.g. 31.4
    }
    ```

---

### 2.3 Claude Usage Widget (`claude`)
* **Design & Behavior**:
  - Square Mode (48px): Warm orange asterisk glyph inside a circular SVG progress ring (`--ring-blue: #74b7ff`) depicting session quota.
  - Rectangle Mode (120px): Circular ring on left + `Claude 3.7` (primary) / `70% Session` (secondary).
* **Backend Mapping**:
  - Source file: `src-tauri/src/ai_usage.rs`.
  - Service: Reads Anthropic local token cache / OAuth credentials or configured API key stats.
* **IPC Contract**:
  - Command: `invoke('get_claude_usage')`.
  - Schema:
    ```typescript
    interface AIUsagePayload {
      provider: 'claude' | 'codex';
      usedPercent: number;      // 0 - 100
      tokensUsed: number;
      tokenLimit: number;
      resetTimeIso: string;
      active: boolean;
    }
    ```

---

### 2.4 Codex / Copilot Usage Widget (`codex`)
* **Design & Behavior**:
  - Square Mode (48px): Hexagonal code glyph inside a circular SVG progress ring.
  - Rectangle Mode (120px): Circular ring on left + `Codex AI` / `82% Quota`.
* **Backend Mapping**:
  - Source file: `src-tauri/src/ai_usage.rs`.
  - Service: Reads GitHub Copilot token telemetry from local VS Code config or OpenAI API endpoint.
* **IPC Contract**:
  - Command: `invoke('get_codex_usage')` -> returns `AIUsagePayload`.

---

### 2.5 Siri-Based AI Widget (`siri-ai`)
* **Design & Behavior**:
  - Square Mode (48px): Apple Intelligence / Siri-style fluid multi-color glowing orb. Animated gentle chromatic pulse (cyan `#60cdff`, magenta `#e056fd`, violet `#686de0`, and blue `#30336b`) with inner ripple.
  - Rectangle Mode (136px): Siri orb on left + `Siri AI` (primary) + `Listening...` / `Ready to assist` (secondary).
  - Interaction: Single click opens voice input overlay or local AI spotlight prompt; double click activates continuous microphone listening.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/voice_assistant.rs`.
  - Native API: Windows Speech Recognition / whisper.cpp local model bridge or Windows `winrt::Media::SpeechSynthesis`.
* **IPC Contract**:
  - Command: `invoke('trigger_siri_assistant', { mode: 'voice' | 'text' })`.
  - Event: Listen for `assistant-state-changed` (`'idle' | 'listening' | 'thinking' | 'speaking'`).
  - Schema:
    ```typescript
    interface AssistantStatePayload {
      state: 'idle' | 'listening' | 'thinking' | 'speaking';
      waveform: number[];       // Real-time audio amplitude bars
      transcript: string;
    }
    ```

---

### 2.6 Running Apps Widget (`running-apps`)
* **Design & Behavior**:
  - Shows active running applications with running indicator dots.
  - **Alphabetical Left-to-Right Default**: Items are initially sorted alphabetically by application name (`A` to `Z`).
  - **In-Dock Direct Reordering**: Users can reorder apps directly on the dock (via drag-and-drop or hover ‹ / › buttons) without opening the customize menu.
  - **Order Persistence**: Activating or launching an app does NOT reorder or jump the position; the user's manual sequence is strictly preserved.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/running_apps.rs`.
  - Native API:
    - Enumerates top-level windows via `EnumWindows`.
    - Filters invisible/cloaked windows via `DwmGetWindowAttribute(DWMWA_CLOAKED)`.
    - Resolves executable PID via `GetWindowThreadProcessId`.
    - Extracts high-res 48px/32px icons via Win32 `SHGetFileInfoW` or `ExtractIconExW`.
* **IPC Contract**:
  - Command: `invoke('get_running_apps')` -> returns `RunningApp[]`.
  - Command: `invoke('activate_app', { appId: string })` -> brings window to foreground via `SetForegroundWindow`.
  - Command: `invoke('save_app_order', { appIds: string[] })`.
  - Schema:
    ```typescript
    interface RunningApp {
      id: string;               // e.g. "code.exe"
      title: string;            // "Visual Studio Code"
      icon: string;             // Base64 PNG data URI or icon slug
      processId: number;
      active: boolean;          // Has focus
      orderIndex: number;
    }
    ```

---

### 2.7 Pinned Apps Widget & App Catalog Picker (`pinned-apps`)
* **Design & Behavior**:
  - Dedicated dock widget hosting user-pinned application shortcuts.
  - **Apps Catalog Picker in Customize**: The Customize modal contains an `Apps` catalog displaying all installed applications on the computer. Clicking any installed application pins it to the dock.
  - Pinned apps can be moved left/right directly on the dock or unpinned via right-click / customize menu.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/installed_apps.rs` & `protonsearch/src/indexer.rs`.
  - Native API: COM `IShellItem` enumeration of `shell:AppsFolder` (covers both Win32 desktop applications and modern UWP / MSIX Store apps).
* **IPC Contract**:
  - Command: `invoke('list_installed_apps')` -> returns `InstalledApp[]`.
  - Command: `invoke('pin_app', { appId: string })`.
  - Command: `invoke('unpin_app', { appId: string })`.
  - Command: `invoke('launch_app', { appId: string })` -> launches via `ShellExecuteW` or `shell:AppsFolder\<AppUserModelId>`.
  - Schema:
    ```typescript
    interface InstalledApp {
      id: string;
      name: string;
      icon: string;
      executablePath: string;
      appUserModelId: string | null;
      isPinned: boolean;
    }
    ```

---

### 2.8 Music Player Widget (`music-player`)
* **Design & Behavior**:
  - Square Mode (48px): Album art rounded tile with animated equalizer bars and play/pause toggle.
  - Rectangle Mode (150px): Album art on left + track title ("Night Drive") & artist ("Kavinsky") stack in the middle + playback controls (prev, play/pause, next) and progress indicator on the right.
  - Strictly locked to 48px square height.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/media_player.rs`.
  - Native API: Windows System Media Transport Controls (`GlobalSystemMediaTransportControlsSessionManager`) via WinRT `windows::Media::Control`.
    - Reads `MediaProperties` (Title, Artist, Album, Thumbnail stream).
    - Reads `PlaybackInfo` (PlaybackStatus: Playing, Paused, Stopped).
    - Controls playback via `TryPlayAsync`, `TryPauseAsync`, `TrySkipNextAsync`, `TrySkipPreviousAsync`.
* **IPC Contract**:
  - Command: `invoke('get_media_status')` -> returns `MediaPayload`.
  - Command: `invoke('toggle_media_playback')`.
  - Command: `invoke('media_next_track')`.
  - Command: `invoke('media_prev_track')`.
  - Event: `media-playback-updated` emitted on SMTC state change.
  - Schema:
    ```typescript
    interface MediaPayload {
      title: string;
      artist: string;
      album: string;
      isPlaying: boolean;
      positionSec: number;
      durationSec: number;
      thumbnailBase64: string | null;
    }
    ```

---

### 2.9 Take a Note Widget (`quick-notes`)
* **Design & Behavior**:
  - Square Mode (48px): Warm amber notepad glyph (`#f0bd6a`).
  - Rectangle Mode (120px): Notepad glyph on left + `Take a Note` / `Recent Notes`.
  - **Click Interaction (Floating Card Popup)**:
    - Clicking the widget opens an elevated Tahoe glass floating note card directly above the dock slot.
    - Features:
      - Clean scratchpad textarea with auto-focus.
      - "Save Note" action (saves to local database and adds to recent list).
      - Quick list of recent notes with relative timestamps.
      - Close button / dismiss on `Escape` or backdrop click.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/notes.rs`.
  - Storage: SQLite table `notes` in `%LOCALAPPDATA%/protondock/protondock.db` or individual Markdown files in `%APPDATA%/protondock/notes/`.
* **IPC Contract**:
  - Command: `invoke('get_recent_notes')` -> returns `NoteItem[]`.
  - Command: `invoke('save_note', { content: string, title?: string })` -> returns `NoteItem`.
  - Command: `invoke('delete_note', { id: string })`.
  - Schema:
    ```typescript
    interface NoteItem {
      id: string;
      title: string;
      content: string;
      createdAt: number;       // Unix timestamp ms
      updatedAt: number;
    }
    ```

---

### 2.10 ProtonSearch Launcher & In-Process Math Engine (`search`)
* **Design & Behavior**:
  - Square Mode (48px): Magnifying glass glyph with subtle highlight ring.
  - Shortcut: Triggered via clicking dock slot or pressing `Alt + Space` / `/`.
  - Spotlight Palette: Tahoe frosted glass spotlight window centered horizontally, elevated above wallpaper.
  - In-Process Math Parser:
    - Instant recursive descent evaluation for arithmetic, percentages (`15% of 340`), powers, roots (`sqrt(144)`), and trigonometric functions.
    - Renders calculation hero card directly above search results.
    - Hitting `Enter` copies calculated value to system clipboard and presents toast confirmation HUD.
* **Rust Backend Module**:
  - Source file: `protonsearch/src/calculator.rs` / `src-tauri/src/calculator.rs`.
  - Native Engine: In-process recursive descent tokenizer and AST evaluator with zero external cloud dependencies.
* **IPC Contract**:
  - Command: `invoke('calculate_expression', { expr: string })` -> returns `CalculationResult`.
  - Command: `invoke('search_query', { query: string, scope: string })` -> returns `SearchResult[]`.
  - Schema:
    ```typescript
    interface CalculationResult {
      expression: string;
      result: number;
      displayValue: string;
      valid: boolean;
    }
    ```

---

### 2.11 Wi-Fi Network Widget (`wifi`)
* **Design & Behavior**:
  - Square Mode (48px): Wi-Fi signal glyph with active connection strength indicator.
  - Rectangle Mode (124px): Wi-Fi icon + `Studio-5GHz` (primary) / `Wi-Fi 6` (secondary).
  - **Anchored Popover Menu**:
    - Radio power toggle switch (enable/disable Wi-Fi adapter).
    - Active Connection Card: SSID (`Studio-5GHz`), 802.11ax Wi-Fi 6 protocol, link speed (866 Mbps), RSSI (-52 dBm), and local IP address (`192.168.1.145`).
    - Available Networks List: Discovered SSIDs with signal bars and one-click "Connect" actions with toast feedback.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/wifi_wlan.rs`.
  - Native API: Windows Native Wifi API (`wlanapi.dll`):
    - `WlanOpenHandle`: Opens a client session with the WLAN service.
    - `WlanEnumInterfaces`: Enumerates all wireless network cards.
    - `WlanGetAvailableNetworkList`: Scans BSSID networks and retrieves signal quality percentages.
    - `WlanConnect` / `WlanDisconnect`: Controls network associations.
* **IPC Contract**:
  - Command: `invoke('get_wifi_status')` -> returns `WifiStatusPayload`.
  - Command: `invoke('toggle_wifi_radio', { enabled: boolean })`.
  - Command: `invoke('connect_wifi_network', { ssid: string, password?: string })`.
  - Schema:
    ```typescript
    interface WifiStatusPayload {
      enabled: boolean;
      connected: boolean;
      ssid: string | null;
      bssid: string | null;
      signalPercent: number;    // 0 - 100
      linkSpeedMbps: number;
      frequencyGhz: number;     // 2.4, 5.0, or 6.0
      ipv4Address: string | null;
      availableNetworks: Array<{
        ssid: string;
        signalPercent: number;
        security: string;       // "WPA3", "WPA2", "Open"
      }>;
    }
    ```

---

### 2.12 Bluetooth Devices Widget (`bluetooth`)
* **Design & Behavior**:
  - Square Mode (48px): Bluetooth glyph with active device badge indicator.
  - Rectangle Mode (120px): Bluetooth glyph + `Bluetooth` / `3 Devices`.
  - **Anchored Popover Menu**:
    - Bluetooth master radio toggle switch.
    - Connected Devices List: Real-time battery levels for connected peripherals (`AirPods Pro 85%`, `MX Master 3S 60%`, `Keychron K2 92%`).
    - Available Discovered Devices: One-click "Pair" action buttons with toast feedback.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/bluetooth_devices.rs`.
  - Native API: Windows Runtime Bluetooth APIs (`windows::Devices::Bluetooth`):
    - `BluetoothDevice::FromIdAsync` to obtain connected device references.
    - GATT Battery Service (`0x180F`) query on Battery Level Characteristic (`0x2A19`) to read peripheral battery percentages.
    - `DeviceWatcher` to discover nearby pairing advertisements.
* **IPC Contract**:
  - Command: `invoke('get_bluetooth_status')` -> returns `BluetoothPayload`.
  - Command: `invoke('pair_bluetooth_device', { deviceId: string })`.
  - Schema:
    ```typescript
    interface BluetoothPayload {
      radioEnabled: boolean;
      connectedDevices: Array<{
        id: string;
        name: string;
        category: 'audio' | 'mouse' | 'keyboard' | 'other';
        batteryPercent: number | null;
        signalRssi: number | null;
      }>;
      discoveredDevices: Array<{
        id: string;
        name: string;
        pairingStatus: string;
      }>;
    }
    ```

---

### 2.13 Notification Center Widget (`notifications`)
* **Design & Behavior**:
  - Square Mode (48px): Notification bell glyph with unread badge count (`3`).
  - Rectangle Mode (124px): Bell icon + `Notifications` / `3 New`.
  - **Anchored Popover Menu**:
    - Header with total unread count and "Clear All" action button.
    - Cards List: App name, relative arrival timestamp (`2m ago`), text body, and per-card dismiss button (`×`).
    - Dismissing individual items or clearing all updates the dock widget badge in real time.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/notifications.rs`.
  - Native API: WinRT User Notification Listener (`windows::UI::Notifications::Management::UserNotificationListener`):
    - `RequestAccessAsync`: Prompts user permissions for notification reading.
    - `GetNotificationsAsync(NotificationKinds::Toast)`: Reads system toast notifications queue.
    - `RemoveNotification`: Clears dismissed toast from Windows Action Center.
* **IPC Contract**:
  - Command: `invoke('get_unread_notifications')` -> returns `NotificationItemPayload[]`.
  - Command: `invoke('clear_all_notifications')`.
  - Command: `invoke('dismiss_notification', { id: string })`.
  - Event: `notification-arrived` emitted on incoming Windows toast.
  - Schema:
    ```typescript
    interface NotificationItemPayload {
      id: string;
      appName: string;
      title: string;
      body: string;
      timestampMs: number;
    }
    ```

---

### 2.14 Date & Time Customizer Widget (`clock`)
* **Design & Behavior**:
  - Square Mode (48px): Digital time format or authentic analog watch face.
  - Rectangle Mode (110px): Digital time + full day of week & date stack.
  - **Anchored Popover Menu**:
    - Large digital time with live seconds (`20:36:42`) ticking every second.
    - Full day and date display (`Sunday, November 8, 2026`).
    - Format switcher buttons:
      - `Time Only`: 48px square width digital clock.
      - `Time + Date`: 110px rectangle width digital clock with date sub-label.
      - `Analog Face`: 48px square width classic round watch dial with 12 ticks, dark hour/minute hands, and orange second hand.
    - Selecting any mode instantly updates the dock widget format and width without page reload.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/clock.rs`.
  - Native API: Win32 `GetLocalTime(&mut SYSTEMTIME)` / Rust `chrono` local time.
* **IPC Contract**:
  - Command: `invoke('get_system_time')` -> returns `SystemTimePayload`.
  - Schema:
    ```typescript
    interface SystemTimePayload {
      hours24: number;
      minutes: number;
      seconds: number;
      dayOfWeek: string;
      dateFormatted: string;
      timezone: string;
    }
    ```

---

### 2.15 System Tray Widget (`system-tray`)
* **Design & Behavior**:
  - Square Mode (48px): Up chevron icon (`^`) representing the Windows notification area tray.
  - **Anchored Popover Menu**:
    - Clean 4x2 grid of running background service icons (Windows Security, NVIDIA Settings, Docker Desktop, OneDrive, Steam, Discord, Proton VPN, Realtek Audio).
    - Clicking any tray application tile activates or restores that background window and shows an immediate toast confirmation HUD.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/system_tray_hook.rs`.
  - Native API:
    - Enumerates notify icons from `Shell_TrayWnd` -> `TrayNotifyWnd` -> `SysPager` -> `ToolbarWindow32`.
    - Reads icon tooltips, executable paths, and sends `WM_COMMAND` / `WM_LBUTTONDOWN` messages to trigger native context menus or activate background windows.
* **IPC Contract**:
  - Command: `invoke('get_tray_icons')` -> returns `TrayIconItem[]`.
  - Command: `invoke('activate_tray_icon', { iconId: string })`.
  - Schema:
    ```typescript
    interface TrayIconItem {
      id: string;
      title: string;
      processName: string;
      iconBase64: string;
    }
    ```

---

### 2.16 Anchored Popover System Contract
* **Dynamic Placement**:
  - All popover menus are rendered fixed in the viewport above the clicked dock widget slot.
  - Coordinates are computed relative to the slot bounding box:
    ```javascript
    popBottom = window.innerHeight - slotRect.top + 10;
    popLeft = Math.max(margin, Math.min(window.innerWidth - popWidth - margin, slotCenterX - popWidth / 2));
    caretX = Math.max(18, Math.min(popWidth - 18, slotCenterX - popLeft));
    ```
  - The caret indicator (`::after`) uses `--caret-x` to point directly at the clicked widget regardless of slot reordering.
* **Dismiss Triggers**:
  - Clicking outside the popover or outside the dock rail.
  - Pressing the `Escape` key.
  - Clicking the same widget slot again.
* **Backdrop**:
  - High-fidelity Tahoe frosted glass blur (`backdrop-filter: blur(40px) saturate(160%)`).
  - Dark glass surface (`rgba(18, 22, 30, 0.95)` with `1px solid rgba(255, 255, 255, 0.12)`).

---

## 3. Motion & Animation Standards

1. **Reorder FLIP Animations**:
   - Reordering triggers FLIP (First, Last, Invert, Play) transitions.
   - Transition duration: `220ms var(--ease)` where `--ease: cubic-bezier(0.16, 1, 0.3, 1)`.
   - Never causes layout jitter, scroll displacement, or vertical clipping.
2. **Sizing Transitions**:
   - Toggling between Square (48px) and Rectangle widths animates smoothly with `width: var(--speed-rail) var(--ease)`.
   - Dock rail dynamically expands and contracts around the modified slot.

---

## 4. Porting Checklist to Tauri v2 & React 19

- [ ] Connect `WeatherWidget.tsx` to `weather.rs` IPC command and anchored weather popover.
- [ ] Connect `BatteryWidget.tsx` to `system_battery.rs` with `CallNtPowerInformation` wattage telemetry HUD.
- [ ] Implement `WifiWidget.tsx` connecting to `wifi_wlan.rs` Win32 WLAN API.
- [ ] Implement `BluetoothWidget.tsx` connecting to `bluetooth_devices.rs` WinRT battery GATT service.
- [ ] Implement `NotificationsWidget.tsx` connecting to `notifications.rs` WinRT UserNotificationListener.
- [ ] Implement `ClockWidget.tsx` with dynamic format switcher (`time`, `time-date`, `analog`).
- [ ] Implement `SystemTrayWidget.tsx` connecting to `system_tray_hook.rs`.
- [ ] Implement `SiriRobot.tsx` vector UI with voice listener bridge.
- [ ] Bind `RunningApps.tsx` to `src-tauri/src/running_apps.rs` with discrete 48px square tiles, dot indicators, and in-dock reordering.
- [ ] Add `InstalledAppsPicker.tsx` inside `WidgetManager.tsx` connecting to `shell:AppsFolder` enumeration.
- [ ] Connect `MusicPlayerWidget.tsx` to `media_player.rs` Windows SMTC session manager.
- [ ] Connect `QuickNotesCard.tsx` to `notes.rs` SQLite storage.
- [ ] Connect `SpotlightWindow.tsx` to `calculator.rs` in-process math evaluator.
