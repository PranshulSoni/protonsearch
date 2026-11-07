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
* **Rust Backend Module**:
  - Source file: `src-tauri/src/system_battery.rs`.
  - Native API: Win32 `GetSystemPowerStatus(&mut SYSTEM_POWER_STATUS)`.
    - `ACLineStatus`: `0` (Offline/Battery), `1` (Online/AC), `255` (Unknown).
    - `BatteryFlag`: High, Low, Critical, Charging, NoSystemBattery.
    - `BatteryLifePercent`: `0` to `100`, or `255` (Unknown).
    - `BatteryLifeTime`: Estimated seconds remaining.
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

- [ ] Connect `WeatherWidget.tsx` to `weather.rs` IPC command.
- [ ] Connect `BatteryWidget.tsx` to `system_battery.rs` Win32 listener.
- [ ] Implement `SiriOrb.tsx` WebGL / CSS radial gradient shader.
- [ ] Bind `RunningApps.tsx` to `src-tauri/src/running_apps.rs` with alphabetized default and manual override store.
- [ ] Add `InstalledAppsPicker.tsx` inside `WidgetManager.tsx` connecting to `shell:AppsFolder` enumeration.
