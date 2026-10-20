# Native dock implementation ledger

Started 20 October 2026 on `2.0.0`, after explicit user authorization to implement the
Windows application. The user additionally corrected the applications design: full-size
independent app icons, running indicators and window actions, rather than a miniature
apps tile. The HTML remains a design reference, not the application renderer.

## Baseline

`cargo test --locked --manifest-path protonsearch/Cargo.toml --bin ProtonSearch`:
87 passed, 1 failed of 88 tests before native implementation. The existing
`tests::test_antigravity_icons` requires the machine-specific `Google.Antigravity` icon;
that identifier returns a null icon on this machine. No launcher was run.

## Architecture decision

Retain the Win32 search controller and its existing search/indexing/hotkey behavior.
Launch a separate `--dock` Slint process, following the existing `--settings` pattern.
Each process owns exactly one event loop. Reuse the installed Slint 1.17 dependency;
promote its existing raw-window-handle 0.6 dependency for stable native HWND access.
Compile Settings and Dock through a shared Slint entry and Rust UI module.

Dock commands post bounded, predefined commands to the search controller. Native
providers return owned data/pixel buffers to the UI, never borrowed HWND/COM references
across worker threads. The controller supervises dock lifetime; dock exits if its
controller disappears. A separate native AppBar reserves an edge band; the visible
glass dock is centered within that reservation. Both clean up on shutdown.

The initial mode coexists with Explorer. No taskbar hiding, Explorer injection,
Winlogon modification or legacy data migration is enabled before recovery/tray/build
compatibility gates. Separate `dock-layout.json` stores only dock preferences.

References: [Slint Window native handle](https://docs.slint.dev/latest/docs/rust/slint/struct.Window),
[Microsoft AppBar negotiation](https://learn.microsoft.com/en-us/windows/win32/shell/abm-querypos).

## Implementation sequence

1. Tested layout/profile/pin/metric/timer model and isolated atomic persistence.
2. Native Slint dock and detail panels with independent application icons.
3. Windows app/window/icon, audio, power, network and AppBar adapters.
4. Controller startup/supervision, search/agent routes and dock runtime integration.
5. Functional first-party widgets, customization and persistent profiles.
6. Build, meaningful tests, independent Rust/security review and manual test checklist.
7. Continue through remaining approved features while five-hour allowance permits;
   stop before the requested 1% remaining threshold. Do not consume allowance artificially.

Live desktop verification requires the user to launch the executable manually because
the repository prohibits agents running the launcher in the background. Compilation,
unit tests and read-only API checks are permitted; unverified native behavior must be
reported as such. This ledger will distinguish implemented, tested and outstanding work.

## Native alpha delivered

The normal executable starts its supervised native Slint dock alongside existing search.
Use `ProtonSearch.exe --search-only` to keep the legacy launcher without the dock.
Do not start `--dock` directly: the child requires a validated controller.

Implemented:
- One glass dock with independent, full-size pinned/running application icons, hover
  enlargement, running indicators, real shell icons, window activation/selection/close.
- Installed applications drawer with filtering and profile-specific pin/unpin.
- Search and existing Agents entry points connected to the Win32 controller.
- Live Windows audio volume/mute, battery/charging, network status and local clock.
- Focus countdown with pause/reset, media key controls, Explorer and system settings routes.
- Work/Home/Presentation profiles, widget enable/add/resize/reorder, custom metric instances,
  dock height and desktop work-area reservation; atomic preferences with backups.
- Native AppBar negotiation, display/Explorer restart notifications, fullscreen hiding,
  supervised child restart/backoff, and graceful shutdown event before forced fallback.
- Embedded original SVG assets, cached native icons and background snapshots.

This remains a coexistence alpha, not a complete taskbar replacement. Explorer's taskbar
stays visible. Weather has no live forecast provider; music metadata is unavailable;
tray opens system settings rather than hosting third-party tray icons. Agents reuses
existing launcher behavior rather than adding a new agent scheduler. Custom metric values
are local user-entered data, not executable extension code. Marketplace, plugin sandbox,
full custom widget editor, multi-monitor dock instances, taskbar hiding/recovery gates,
accessibility/performance acceptance and release packaging remain outstanding.
The Start experience is an application drawer, not the complete planned customizable menu.
Exit shuts down the controller and dock; independently opened Settings windows may remain open.

## Manual acceptance checklist

Repository instructions prohibit launching the app automatically. No visual or live desktop
acceptance is claimed. After launching the built executable manually:

1. Confirm one dock, preserved search hotkey, readable icons and no focus stealing.
2. Pin/unpin and launch an application; activate/select/close its windows.
3. Change volume/mute, inspect battery/network, pause/reset Focus and test media controls.
4. Change profiles, resize/reorder/add/toggle widgets, exit and reopen to verify persistence.
5. Toggle work-area reservation; test maximized apps, fullscreen, DPI/display changes and
   Explorer restart. Exit via tray and confirm the reserved desktop band disappears.
6. Disable dock in Customize. To re-enable, close Proton and change `enabled` to `true` in
   `%APPDATA%/protonsearch/dock-layout.json`; preserve its backup if diagnosing a bad layout.
7. Check keyboard/screen reader access, mixed DPI, app launch variants and idle resources.

No installer or release version bump was performed; Cargo package version still reads1.1.0.

## Verification at handoff

Latest code build: `cargo build --locked --manifest-path protonsearch/Cargo.toml --bin ProtonSearch`
passed (debug executable at `protonsearch/target/debug/ProtonSearch.exe`).

`cargo test --locked --manifest-path protonsearch/Cargo.toml --lib --bin ProtonSearch -- --skip tests::test_antigravity_icons`
passed: library9/9 and binary91/91; one baseline machine-dependent icon test filtered.
Existing repository warnings remain. Earlier `cargo check --locked --all-targets` also
passed. Independent Rust and general integration reviews found no additional high
blockers after the graceful shutdown corrections; live desktop acceptance remains pending.

The five-hour allowance reached98% used during final verification. Work stops at the
requested remaining-allowance boundary without consuming a reset credit.
