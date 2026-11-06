# Final Launcher + Widgets Redesign and Native Port

Date: 2026-11-07
Status: In progress
Source of truth for design: `preview-tahoe-glass.html`
Target app: Tauri 2 + React 19 + TypeScript + existing Rust search engine

## User requirements (verbatim intent, normalized)

### Launcher
- Remove visible scrollbars from the launcher.
- Remove the Ctrl+K actions badge, Space preview badge, and Esc badge from the search bar.
- Remove the bottom shortcut deck (Select, Open, Reveal, Path, Actions, Preview, Close).
- Remove left/right split preview layout. Use one stable centered results column.
- Unified search only: top scopes `All`, `Apps`, `Files`, `Folders`, `OCR` (plus Git/Tools only if useful).
- Search bar is a plain input. Type to search, Enter to open. No shortcut theater.
- Remove demo chrome: header title, demo pills, "ProtonSearch has returned" style copy, footer match counters.
- Keep the minimalist Tahoe glass look. Quiet badges show item type via real widget icons: app, terminal, code (language-specific where possible), file, folder.
- Calculator stays as an inline hero result, per `docs/superpowers/specs/2026-11-07-minimalist-launcher-math-spec.md`.

### Dock and widgets
- Installed widgets manager must show a complete preview card of every widget, with enable/disable controls and size controls, not a bare toggle row.
- Customize widget screens must show the widget preview as cards and expose real per-widget settings.
- Profiles stay as-is in behavior, but every widget must be fully customizable.
- Spotify widget must work at the largest card size without breaking. Show real detail (track, artist, album art, progress, controls, device).
- Consistent sizing and spacing across all widgets.
- Add widgets:
  - Claude usage limit
  - ChatGPT usage limit
  - AI workspace agent widget (Claude Code style session/token/status widget)
  - Wi-Fi, Bluetooth, and connection status in the system/status widget
  - File and folder widgets
  - Weather consistency across sizes
- Redesign the weak "card" widget.
- Weather widget detail should be visible at all sizes where possible.
- Every widget in the store must be previewable as a card.
- Different item types must use distinct icons, not a generic "App" badge.

### Port and verification
- Port the finalized HTML/CSS/JS into the React + TypeScript UI.
- Keep the existing Rust search engine authoritative.
- Wire real Windows APIs through Tauri commands.
- Build as a Windows app.
- Verify the app actually launches and works (Computer Use + automated checks).
- Follow `rules.md`: kill `protonsearch.exe` before build, commit after edits, use graphify for analysis.

## Final design decisions

### Launcher
- Single centered column. Card width 660 px, radius 18, deep obsidian glass.
- Header: search icon, plain input, no shortcut badges, no top-right controls.
- Scopes: `All`, `Apps`, `Files`, `Folders`, `OCR`. Counts are allowed; shortcut labels are not.
- No footer. No right preview pane. No action deck. No demo header or toolbar.
- Results: 46 px rows, 30 px icon, 13 px title, 10.5 px detail. Type is conveyed by the icon, not a text badge.
- Icons: app logo or window tile for apps; terminal prompt for terminal; TS/TSX, C++, Rust, Python and generic code icons for code; document icon for files; folder icon for folders; branch/commit icons for git; gear for system; calculator for math; scan-text for OCR.
- Calculator remains a hero row above results, with expression and formatted result.
- Keyboard remains invisible but functional: Alt+Space toggle, Esc close, arrows select, Enter open, Ctrl+Enter secondary, Ctrl+C copy path, Tab cycles scopes.
- Scrollbars are hidden while content remains scrollable.
- Demo data now includes apps, code, files, folders, OCR images, git, system and math so every scope has content.

### Widgets
- Widget library: Windows Start, ProtonSearch, Running Apps, Spotify, Weather, System, Trash, Volume, Clock, Claude usage, ChatGPT usage, AI workspace agent, Recent Files, Quick Folders.
- Every widget has three sizes: Pill (compact), Tile (medium), Card (expanded). Card is a real content-rich layout, not a stretched pill.
- Widget manager is a card grid. Every card shows a live preview at the selected size, name, description, size segmented control, enable toggle, customize button and drag handle.
- Customize opens a focused subview with a large card preview and per-widget settings. Settings are specific to each widget, not a generic form.
- Profiles keep working. Presets include the new widgets and every profile change remains one state update.
- Store has preview cards with real install actions for optional widgets.
- All icons are inline SVG. No emoji.
- Card visual language: 16:10 glass card, category tint, 16 px padding, icon + name + status, one strong data focal point, supporting metrics, footer meta. No gradient headline text.
- Wi-Fi, Bluetooth, Ethernet, VPN and battery belong to the System widget.
- Spotify card shows album art, track, artist, elapsed/duration, progress, controls, volume and device. It must not overflow at Card size.
- Weather card shows current condition, temperature, high/low, feels like, wind, humidity, UV, visibility and hourly forecast.
- Claude and ChatGPT show usage windows, reset countdowns and plan labels. Values are demo in the preview and must be marked as sample; the native app uses configured/local data and never claims live numbers without a source.
- AI Workspace Agent shows workspace, branch, agent status, current task, session time, tokens and files changed.
- Recent Files and Quick Folders use file/folder icons and real Windows known folders in the app.
- Running Apps card shows app icons, active window, memory and window count.
- Trash card shows item count, reclaimable size and an empty action.
- Volume card shows output device, level slider and mute.
- Clock card shows large time, date, timezone and next event slot.
- No fake hardware toggles. Quick controls open the documented Windows settings pages in the app.

### Widget settings schema (preview)
- Spotify: showProgress, showControls, showDevice, compactProgress.
- Weather: location label, units C/F, showHourly, showDetails, forecast count.
- System: showCpu, showRam, showTemp, showNetwork, showBluetooth, showBattery.
- Claude/ChatGPT: plan label, showReset, showBreakdown, warnAt.
- AI Agent: workspace, showTokens, showFiles, showSession.
- Files: source folder, maxItems, showType.
- Folders: maxItems, showCounts.
- Clock: hour12, showSeconds, showDate, showTimezone.
- Apps: maxApps, showTitles, activeOnly.
- Each schema renders toggles, segmented choices, number ranges or text fields as appropriate.
- All settings persist through the existing layout `settings` object.

### Port mapping
- `preview-tahoe-glass.html` is the visual authority for launcher and widget cards.
- React launcher: rewrite `src/features/search/SpotlightWindow.tsx` and its CSS to the final single-column design. Keep Rust `search` requests, result IDs and execution authoritative. Add scope filtering on the result `type`/source and OCR routing.
- React dock: keep the existing provider/snapshot system and native commands. Replace the gallery with the card-grid manager and settings subview.
- React widgets: add Claude, ChatGPT, AI Agent, Files and Folders definitions. Extend System snapshot with connectivity. Keep all values live or explicitly disconnected.
- Native: add only the commands needed for connectivity, known folders, recent files, AI usage and agent status. No frontend shell execution, no credential access.
- Build gate: `npm test`, `npm run build`, `cargo test`, Windows Tauri build, launch, exercise search and every widget.

## Phases

1. Recon: map preview HTML, React/Tauri app, specs. Verify with written reports.
2. Finalize HTML preview:
   - Launcher minimalism.
   - Dock widget library, status/AI/file widgets, Spotify and card widget.
   - Installed/store manager previews and per-widget settings.
3. Port to React/TS:
   - SpotlightWindow minimal unified search + calculator.
   - DockShell widget cards and manager.
   - Settings window per-widget customization.
4. Rust/Windows APIs:
   - Status providers (Wi-Fi, Bluetooth), media, files/folders, AI usage.
   - Real commands and permissions.
5. Build and verify:
   - `npm test`, `npm run build`, `cargo check`, Tauri build.
   - Launch app, exercise launcher and widgets.
   - Record unverified gates in `docs/qa/STATUS.md`.
