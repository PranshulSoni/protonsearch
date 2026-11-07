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
   - Preset profiles: `Daily`, `Work`, `Founder`, and `Focus`.
   - Native User Profile Creation: Top-shelf `+` button and modal allows users to instantiate custom profiles (e.g. `Coding`, `Gaming`, `Studio`) persisted in local storage.
   - Fluid Physical Morphing: Switching profiles animates dock rail width and widget positions dynamically via Apple spring curves (`cubic-bezier(0.16, 1, 0.3, 1)`).

---

### 1.2 Multi-Profile Architecture & Dynamic Rail Morphing

1. **Profile Persistence Contract**:
   - Default profiles stored statically in configuration.
   - Custom profiles serialized to local storage under key `protondock_custom_profiles` as `Record<string, string[]>`.
   - Dynamic profile tabs generated on runtime boot and synchronized with the dock rail.
2. **Dynamic Dock Width Morphing**:
   - Measuring previous width and layout coordinates before state mutation.
   - Applying target profile widget layout.
   - Measuring new width and running a smooth CSS spring transition on the dock rail container: `transition: width 380ms cubic-bezier(0.16, 1, 0.3, 1)`.
   - Sibling widgets glide horizontally into position via FLIP transforms (`translateX(dx)` -> `0`).
   - Entering widgets scale smoothly from `scale(0.85)` and `opacity: 0` to `scale(1.0)` and `opacity: 1`.
3. **In-Place Widget Resizing & Edit Mode Contract**:
   - **Pristine Normal Mode**: Zero hover navigation arrows, zero expand buttons, zero accidental dragging. Clicking any widget triggers launch or anchored popovers.
   - **Explicit Edit Mode Activation**: Clicking the top-shelf `Edit` pill button toggles edit mode and changes button label to `Done`.
   - **Direct Manipulation Reordering**: Slots become draggable (`draggable="true"` with `cursor: grab`) exclusively in edit mode.
   - **In-Place Resizing**: Clicking any widget or its top-right resize chip (`.dw-resize-chip`) cycles widths between `48px` (Square), `96px` (Compact), `140px` (Standard), and `168px` (Wide).
   - **Fluid Physical Spring Transitions**: Dynamic width shifts animate the outer dock rail and slide sibling widgets via FLIP spring physics (`cubic-bezier(0.16, 1, 0.3, 1)`).
   - **Height Invariant Guarantee**: Height strictly remains locked at `48px` across all sizing transitions.
   - **Layout Locking**: Clicking `Done` or pressing `Escape` exits edit mode and synchronizes preferences to local storage.

---


### 1.3 Three Native Themes Specification

ProtonDock and the Kensaku search engine support three native themes, toggled via the top-shelf theme switcher or keyboard shortcut. All tokens adapt automatically via CSS custom properties on the root `body[data-theme]`:

1. **Dark Theme (`data-theme="dark"`)**:
   - Background Surface: Deep slate charcoal (`rgba(14, 18, 26, 0.94)`).
   - Primary Text: Crisp white-silver (`#f1f5f9`), Secondary: (`#94a3b8`), Muted: (`#64748b`).
   - Borders: Subtle translucent border (`rgba(255, 255, 255, 0.10)`), strong (`rgba(255, 255, 255, 0.18)`).
   - Backdrop: Deep dark gradient (`#0b0f17` to `#05080c`) with 28px backdrop blur.
   - Contrast Ratio: Exceeds WCAG AAA standard (12.4:1 contrast on active labels).

2. **Light Theme (`data-theme="light"`)**:
   - Background Surface: Clean alabaster / paper frosted glass (`rgba(250, 252, 255, 0.96)`).
   - Primary Text: Deep slate (`#0f172a`), Secondary: (`#475569`), Muted: (`#64748b`).
   - Borders: Soft daylight borders (`rgba(0, 0, 0, 0.10)`), strong (`rgba(0, 0, 0, 0.18)`).
   - Backdrop: Warm natural daylight gradient (`#f1f5f9` to `#e2e8f0`) with 28px backdrop blur.
   - Contrast Ratio: 14.1:1 on text elements, zero glare, optimized for bright daylight working environments.

3. **Liquid Glass Theme (`data-theme="liquid-glass"`)**:
   - Background Surface: Ultra-translucent liquid glass (`rgba(255, 255, 255, 0.10)`).
   - Optical Backdrop Blur: High-refraction Tahoe blur (`backdrop-filter: blur(48px) saturate(220%)`).
   - Specular Highlights: Crisp top reflection (`inset 0 1.5px 0 0 rgba(255, 255, 255, 0.65)`).
   - Ambient Glow: Smooth volumetric drop-shadow (`0 18px 48px -12px rgba(0, 0, 0, 0.60)`).
   - Wallpapers: Supported over dynamic silk aura wave wallpaper backdrops with vivid refraction.

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
* **Design & Alignment Contract**:
  - **Discrete Standalone 48px Tiles**: Each active application renders as an independent 48px square tile directly on the dock rail. Grouping running apps into a wide capsule or nested container is strictly prohibited.
  - **Uniform Spacing & Height**: Exactly matches all other dock slots with locked 48px height (`--tile-h: 48px`), 48px width, and uniform 6px rail gaps (`var(--dock-gap: 6px)`).
  - **Running Dot Indicator**: A crisp white dot (`3.5px` diameter) is centered at `bottom: 3.5px` directly beneath the app icon.
  - **Alphabetical Default with Order Lock**: Running apps sort alphabetically (`A` to `Z`) on first launch; reordering by dragging in Edit Mode is permanently remembered. Launching or activating apps preserves the user manual sequence.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/running_apps.rs` / `protonsearch/src/running_apps.rs`.
  - Native API:
    - Enumerates top-level windows via `EnumWindows`.
    - Filters cloaked/invisible windows via `DwmGetWindowAttribute(DWMWA_CLOAKED)`.
    - Resolves executable PID via `GetWindowThreadProcessId`.
    - Extracts high-resolution raw icons via the Sharp Icon Pipeline (Section 2.8).
* **IPC Contract**:
  - Command: `invoke('get_running_apps')` -> returns `RunningApp[]`.
  - Command: `invoke('activate_app', { appId: string })` -> brings window to foreground via `SetForegroundWindow`.
  - Command: `invoke('save_app_order', { appIds: string[] })`.
  - Schema:
    ```typescript
    interface RunningApp {
      id: string;               // e.g. "code.exe"
      title: string;            // "Visual Studio Code"
      icon: string;             // Razor-sharp Base64 PNG data URI
      processId: number;
      active: boolean;          // Has focus
      orderIndex: number;
    }
    ```

---

### 2.7 Pinned Apps Widget & App Catalog Picker (`pinned-apps`)
* **Design & Alignment Contract**:
  - **Discrete Standalone 48px Tiles**: Each pinned application shortcut renders as an independent 48px square tile on the dock rail, perfectly aligned with running apps and system widgets.
  - **Apps Catalog Picker in Customize**: The Customize modal features an `Apps` catalog enumerating all installed desktop and Windows Store applications. Clicking any app pins it directly to the dock.
  - **Reordering & Unpinning**: Pinned apps can be dragged in Edit Mode to reorder or unpinned via the customizer modal.
* **Rust Backend Module**:
  - Source file: `src-tauri/src/installed_apps.rs` & `protonsearch/src/indexer.rs`.
  - Native API: COM `IShellItem` enumeration of `shell:AppsFolder` resolving clean target PIDs and Store AppIDs.

---

### 2.8 Windows Native Icon Extraction & Sharpness Architecture Contract

A common defect in Windows launchers is blurry, fuzzy application icons. This occurs when apps naively query Win32 `ExtractIconExW` or `SHGetFileInfoW(SHGFI_ICON)`, which extracts low-resolution 32x32 or 16x16 bitmaps that degrade severely when displayed on modern High-DPI screens (125%, 150%, 200% Windows scaling). Conversely, downsampling raw 256x256 icons using standard bilinear interpolation produces soft, blurry edges.

To guarantee razor-sharp, crystal-clear icons, ProtonDock enforces the following multi-stage icon pipeline:

1. **Small Icon Sizing Contract (Preventing Giant Bloated Icons)**:
   - Within each 48px square dock tile, the application icon must be **small**: `20px x 20px` to `22px x 22px` (never exceeding `24px x 24px`).
   - The icon must NEVER fill the 48px square. It is centered inside a `32px x 32px` glyph container with `13px` to `14px` of optical padding on all sides. This leaves balanced breathing room and ensures the `3.5px` white running dot sits neatly underneath without overlapping the icon graphic.

2. **Win32 & COM Jumbo Extraction (Never Use Low-Res Bitmaps)**:
   - Win32 Desktop Executables: Query the system image list via `SHGetImageList(SHIL_JUMBO, IID_IImageList, ...)` or extract the uncompressed 256x256 PNG resource from `RT_GROUP_ICON`.
   - Windows Store / UWP Packaged Apps: Resolve through `IShellItemImageFactory::GetImage` with flags `SIIGBF_RESIZETOFIT | SIIGBF_SCALEUP`, passing the exact device pixel dimensions computed from `GetDpiForWindow(hwnd)`.

3. **High-Quality Lanczos3 Resampling with Unsharp Mask (Rust Backend)**:
   - Resample high-resolution source bitmaps (256x256 or 128x128) down to target physical device pixels using the 3-lobed Lanczos filter (`image::imageops::FilterType::Lanczos3` in Rust, or Direct2D `D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC`).
   - Apply a micro unsharp mask filter (radius 0.5, amount 0.35) after resampling to restore micro-contrast and edge sharpness lost during fractional scaling.
   - Encode output strictly as uncompressed or losslessly compressed 32-bit PNG (`RGBA8888`) with pre-multiplied alpha. Never encode icons as JPEG or lossy WebP.

4. **Frontend Subpixel Crispness & Anti-Blur CSS Rules**:
   - Apply `image-rendering: -webkit-optimize-contrast;` and `image-rendering: crisp-edges;` to all icon images.
   - For SVG vector icons, enforce `shape-rendering="geometricPrecision"` and integer viewBox dimensions.
   - Enforce GPU subpixel snapping via `transform: translateZ(0)` on the icon container to prevent GPU bilinear texture filtering blur caused by subpixel fractional coordinates. (covers both Win32 desktop applications and modern UWP / MSIX Store apps).
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


### 2.17 Kensaku Native Desktop Search Engine Integration (`search`)

ProtonDock integrates the architecture and UI philosophy of **Kensaku** (`https://github.com/yjid/kensaku` by `yjid`), a fast, private, local-first search engine built in Rust. It indexes local files, full document text (FTS5), OCR inside screenshots, and codebases entirely on device without cloud dependencies.

* **Design & UI Layout Contract**:
  - **Dock Trigger**: 48px square tile featuring a glass magnifying lens glyph, triggered by click or global hotkey `Alt + Space` (or `Ctrl + Space`, or `/`).
  - **Spotlight Modal**: 680px width centered spotlight window with continuous rounded corners (border-radius: 20px), search header, active engine badge (`Kensaku Engine`), and in-process math evaluator.
  - **Scope Filter Pills**:
    - `All`: Unified search ranking across apps, files, FTS content, code, and OCR.
    - `Files`: Local files (PDF, DOCX, XLSX, TXT, images, archives).
    - `Content (FTS)`: Full-text content search matches with highlighted snippet previews.
    - `OCR (Screenshots)`: Screenshots and camera captures where text was recognized via OCR.
    - `Apps`: Installed Windows desktop and Store applications.
    - `Code`: Source code repositories and scripts (`.rs`, `.ts`, `.py`, `.html`, `.md`).
  - **Layout Mode Toggle (List vs Grid)**:
    - `List View`: Compact row layout with icon, file title, parent path, snippet, and category badge.
    - `Grid View`: 2-column continuous-corner card grid showing large thumbnails, detailed snippets, and metadata.
  - **Quick Look Preview Drawer (Spacebar Preview)**:
    - Pressing `Space` on any focused result toggles the Quick Look drawer.
    - Displays full filename, badge, path, match snippet (OCR extracted text or FTS highlighted match), and file metadata (size, last modified, indexing engine).
    - Action buttons: `Open File` (launches default handler), `Reveal in Explorer` (opens Windows Explorer focused on file), and `Copy Path` (copies absolute path to clipboard).

* **Rust Backend Engine Architecture**:
  - **Source Files**:
    - `protonsearch/src/indexer.rs`: SQLite FTS5 indexer and document text extraction.
    - `protonsearch/src/ocr.rs`: Windows Native OCR worker.
    - `protonsearch/src/semantic.rs`: Local embeddings and vector ranking.
    - `src-tauri/src/kensaku_search.rs`: Tauri v2 IPC command handlers.
  - **SQLite FTS5 Schema**:
    ```sql
    CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
      path UNINDEXED,
      filename,
      content,
      extracted_ocr,
      category,
      modified_at UNINDEXED,
      tokenize = 'unicode61 remove_diacritics 2'
    );
    ```
  - **Ranking Query**:
    ```sql
    SELECT path, filename, category, modified_at,
           snippet(files_fts, 2, '<mark>', '</mark>', '...', 16) AS content_snippet,
           snippet(files_fts, 3, '<mark>', '</mark>', '...', 16) AS ocr_snippet,
           bm25(files_fts, 10.0, 5.0, 2.0, 3.0) AS rank
    FROM files_fts
    WHERE files_fts MATCH ?
    ORDER BY rank
    LIMIT ? OFFSET ?;
    ```
  - **Windows Native OCR Pipeline**:
    - Uses Windows Runtime API `Windows.Media.Ocr.OcrEngine`.
    - Watches `%USERPROFILE%\Pictures\Screenshots` and camera roll directories.
    - On new screenshot detection, decodes image stream via `Windows.Graphics.Imaging.BitmapDecoder`, runs `OcrEngine.RecognizeAsync`, and inserts extracted text into `files_fts.extracted_ocr`.
  - **Local Semantic Vector Search**:
    - Offline embeddings using `fastembed-rs` (`all-MiniLM-L6-v2` ONNX model running on local CPU).
    - Stores 384-dimensional float embeddings in SQLite BLOB or vector virtual table.
    - Enables concept search (e.g. querying "monthly expenses" retrieves "Q3 budget invoice.xlsx").
  - **MCP Agent Server**:
    - Exposes a local JSON-RPC Model Context Protocol (MCP) server on port `127.0.0.1:41782` or stdio.
    - Tools provided to AI agents:
      - `search_files(query: string, scope?: string, limit?: number)`: Runs FTS5 and OCR queries.
      - `read_file_content(path: string, max_bytes?: number)`: Safely reads local file content.
      - `ocr_screenshot(path: string)`: Runs instant OCR on arbitrary image files.

* **IPC Contract**:
  - Commands:
    - `invoke('kensaku_search', { query: string, scope: string, limit: number, offset: number })` -> `KensakuSearchResponse`
    - `invoke('kensaku_quick_look', { path: string })` -> `KensakuQuickLookPayload`
    - `invoke('kensaku_index_status')` -> `KensakuIndexStats`
    - `invoke('open_path', { path: string })`
    - `invoke('reveal_in_explorer', { path: string })`
  - Schema:
    ```typescript
    interface KensakuSearchResult {
      title: string;
      path: string;
      category: 'apps' | 'files' | 'content' | 'ocr' | 'code';
      snippet: string | null;
      meta: string;
      modifiedAtMs: number;
      score: number;
    }

    interface KensakuSearchResponse {
      query: string;
      scope: string;
      totalMatches: number;
      results: KensakuSearchResult[];
      durationMs: number;
    }

    interface KensakuQuickLookPayload {
      path: string;
      filename: string;
      category: string;
      fileSizeBytes: number;
      modifiedAtMs: number;
      previewText: string;
      isOcr: boolean;
    }
    ```

---

## 3. Motion & Animation Standards (Apple Design System)

1. **Apple Two-Parameter Spring Dynamics**:
   - Damping Ratio: `1.0` (critically damped, no overshoot) for all layout and width morphing transitions.
   - Response Time: `0.38s` for rail expansion, `0.32s` for widget reordering.
   - CSS Spring Curve: `cubic-bezier(0.16, 1, 0.3, 1)` approximating native iOS fluid interface springs.
2. **Reorder & Resize FLIP Pipeline**:
   - Reordering or resizing widgets captures initial bounding client rects.
   - Inverts horizontal offset delta `dx = first.left - last.left`.
   - Animates to zero displacement using Web Animations API without layout jitter or vertical clipping.
3. **Dynamic Dock Rail Morphing**:
   - Changing profiles or expanding widget widths animates the outer dock container smoothly between computed widths.
   - Dock rail width is animated explicitly using `rail.style.transition = 'width 380ms cubic-bezier(0.16, 1, 0.3, 1)'` to prevent layout snapping.
4. **Instant Pointer-Down Response**:
   - Buttons and widget slots respond on pointer-down with `transform: scale(0.96)` over 100ms ease-out, providing physical directness.

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
- [ ] Connect multi-profile management to `profiles.rs` SQLite storage with native profile creation and dynamic switching.
- [ ] Implement Apple Design spring animation controller for fluid dock rail width morphing and FLIP transitions.
- [ ] Bind in-place widget resize controller with 48px square height invariant guarantee.
- [ ] Connect `SpotlightWindow.tsx` to `kensaku_search.rs` with FTS5, OCR, and semantic vector endpoints.
- [ ] Connect Spacebar Quick Look preview modal to `kensaku_quick_look` IPC command.
- [ ] Wire native theme switcher across Dark, Light, and Liquid Glass tokens with `localStorage` persistence.
- [ ] Bind List vs Grid layout toggle in Spotlight window with continuous-corner cards.
