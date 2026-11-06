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
