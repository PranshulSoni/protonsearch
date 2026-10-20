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
