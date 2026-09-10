# ProtonSearch Arch Linux Migration Plan

## 1. Executive summary

The existing `protonsearch` crate is a Windows-only Rust/Slint launcher. Its entry point and most feature modules directly use Win32, COM, registry, `%APPDATA%`, PowerShell, `explorer.exe`, and Windows settings URIs. A Linux build cannot safely be obtained by conditional-compiling a few functions.

This migration therefore adds an isolated `protonsearch-linux` crate. The first milestone is intentionally a small, testable CLI/backend: XDG paths, bounded file search, `.desktop` discovery, safe launching, capability diagnostics, and narrow optional-provider actions. It never imports Windows catalog data and never edits user dotfiles. Existing files remain untouched; normal integration into the current product will require a later reviewed platform-selection change.

The implementation targets generic Linux first and adds Hyprland behavior only when a real Hyprland IPC endpoint is detected. Network, Bluetooth, audio, power, brightness, media, and compositor providers degrade independently when their package, daemon, hardware, or permissions are unavailable.

## 2. Repository architecture

* Rust 2021 application in `protonsearch/`.
* Slint UI and Win32 window/message loop in `protonsearch/src/main.rs` and `protonsearch/ui/settings.slint`.
* SQLite/FTS5 search and indexing in `search.rs`, `indexer.rs`, and related indexers.
* Windows settings/action catalog in `quick_actions.rs` and `search.rs`.
* Windows process, shell, monitor, audio, clipboard, power, and registry integration in `launcher.rs`, `hotkey.rs`, `settings_startup.rs`, `settings_ui.rs`, and `uninstall.rs`.
* Existing package/build path is Cargo plus Windows installer/WinGet metadata.
* Existing worktree contains unrelated modifications and untracked files; they are preserved.

## 3. Windows-specific inventory

| Area | Existing implementation | Linux status |
|---|---|---|
| Window/UI | Win32 window procedure, GDI painting, DWM, Slint settings | New Linux CLI/backend; GUI integration deferred |
| Global hotkey/tray | `RegisterHotKey`, tray messages, Win32 handles | Hyprland keybind documentation; compositor-specific integration later |
| Files/indexing | Known folders, `%APPDATA%`, COM shell APIs, Windows paths | XDG roots and bounded `walkdir` search |
| Applications | Start Menu, `.lnk`, Store apps, shell execution | XDG `.desktop` discovery and safe argv launch |
| Settings | `ms-settings:`, Control Panel, registry | Linux provider/action catalogue |
| Network/Bluetooth | Windows settings and shell commands | NetworkManager/BlueZ optional providers |
| Audio | WinRT/COM endpoints and media keys | PipeWire/WirePlumber (`wpctl`) and MPRIS (`playerctl`) |
| Display/window actions | Win32 monitor/window APIs and `winvd` | Read-only Hyprland IPC where available |
| Power/session | Win32 shutdown, suspend, lock and shell actions | `loginctl`, UPower, power-profiles-daemon |
| Startup/settings | Registry Run keys, `%APPDATA%`, executable directory | XDG config/data/state and optional user service guidance |
| Clipboard/OCR/browser/AI | Windows-specific APIs and paths | Deferred or provider-specific; not exposed as Windows results |

## 4. Feature dependency graph

```text
Linux CLI
├── XDG paths ── settings, search roots, application directories, logs
├── File search ── bounded traversal ── search results
├── Desktop entries ── parser ── safe application launch
├── Capability registry
│   ├── NetworkManager/nmcli ── Wi-Fi status/toggle
│   ├── BlueZ/bluetoothctl ── Bluetooth status/toggle
│   ├── PipeWire/wpctl ── audio status/volume/mute
│   ├── UPower/powerprofilesd/loginctl ── battery/profile/session actions
│   ├── brightnessctl ── brightness actions
│   ├── playerctl ── media actions
│   └── Hyprland IPC ── monitors/workspaces/config discovery
└── Linux settings catalogue ── only Linux providers and actions
```

## 5. Windows-to-Linux mapping matrix

| Windows source/behavior | Linux provider | Detection | Fallback |
|---|---|---|---|
| `%USERPROFILE%`, known folders | `$HOME`, `user-dirs.dirs` | `HOME` and XDG files | `$HOME` only |
| `%APPDATA%` settings/index | `$XDG_CONFIG_HOME`, `$XDG_DATA_HOME`, `$XDG_STATE_HOME` | XDG env/defaults | `~/.config`, `~/.local/share`, `~/.local/state` |
| `%TEMP%` | `$TMPDIR`/`/tmp` | standard library | OS temp directory |
| Start Menu/`.lnk` | `.desktop` entries | XDG data dirs | no app result |
| `explorer.exe` | `xdg-open` | executable lookup | clear launch error |
| Windows settings URIs | Linux action IDs | capability registry | unavailable result |
| Registry | provider config/XDG files | provider-specific | read-only unsupported |
| Windows Search index | bounded native traversal | configured roots | empty result with diagnostic |
| Task Manager/process action | `/proc`, systemd where safe | `/proc`/`loginctl` | unsupported |
| Wi-Fi settings | NetworkManager via `nmcli` | `nmcli` + service check | package/service guidance |
| Bluetooth settings | BlueZ via `bluetoothctl` | executable + service check | package/service guidance |
| Audio endpoints | PipeWire via `wpctl` | executable + command check | audio unavailable |
| Display settings | Hyprland query IPC | confirmed Hyprland socket | generic display info unavailable |
| Brightness | `brightnessctl` | executable + device result | unsupported hardware |
| Battery/power | UPower/sysfs/powerprofilesd/logind | commands/files | per-provider fallback |
| Media keys | MPRIS via `playerctl` | executable | media unavailable |
| Lock/suspend/reboot | `loginctl` | executable/session | explicit unsupported state |
| Default apps | XDG MIME associations/`xdg-open` | files/commands | desktop default launcher |
| Windows startup | XDG autostart/user service guidance | read-only path discovery | Hyprland keybind instructions |

Every external invocation uses a fixed executable name and an argument array. Search input is never placed in a shell command.

## 6. Linux platform architecture

`protonsearch-linux` is a standalone Cargo package. `src/lib.rs` exposes the modules; `src/main.rs` is a small CLI that is usable on a TTY, Wayland, X11, or a headless machine for diagnostics and search. Linux settings and actions are constructed only by the Linux crate, so Windows entries cannot leak into Linux output.

Capabilities use the states `available`, `package_missing`, `service_inactive`, `permission_denied`, `unsupported_desktop`, `unsupported_hardware`, `configuration_missing`, and `provider_error`. Detection is explicit (`doctor`) and cached per invocation; actions re-check their provider before changing state.

## 7. Proposed new directory structure

```text
protonsearch-linux/
  Cargo.toml
  src/{lib.rs,main.rs,actions.rs,capabilities.rs,desktop.rs,hyprland.rs,search.rs,settings.rs,system.rs,xdg.rs}
packaging/arch/PKGBUILD
packaging/arch/protonsearch-linux.desktop
scripts/linux/check-dependencies.sh
docs/linux/{ARCH_LINUX_MIGRATION_PLAN.md,IMPLEMENTATION_SUMMARY.md,DEPENDENCIES.md,SETTINGS_MAPPING.md,TEST_REPORT.md,EXISTING_FILE_INTEGRITY.md}
```

## 8. File-by-file implementation plan

* `xdg.rs`: XDG path resolution, user directories, safe root/exclusion policy.
* `search.rs`: bounded (depth and entry count), non-following symlink traversal and case-insensitive query matching.
* `desktop.rs`: `.desktop` parser, visibility filtering, field-code expansion, safe argv launching.
* `capabilities.rs`: command/service/session capability detection and diagnostics.
* `system.rs`: typed read-only provider queries, sysfs/UPower battery fallback, and bounded external process execution.
* `actions.rs`: allowlisted Wi-Fi, Bluetooth, audio, brightness, media, power, folder, and Hyprland actions.
* `hyprland.rs`: confirmed Hyprland IPC queries and read-only config include discovery only after confirmation.
* `settings.rs`: Linux-only JSON settings schema and catalogue.
* `main.rs`: `doctor`, `search`, `apps`, `launch`, `settings`, and `action` commands.
* `lib.rs`: public module boundary and result types.
* packaging/script files: build/install metadata and non-mutating dependency guidance.

## 9. Capability and dependency matrix

| Capability | Package/daemon | Required? | Missing behavior |
|---|---|---:|---|
| Search/XDG/apps | system utilities + filesystem | Yes | Core search still works with fewer roots |
| Open files/URLs | `xdg-utils` | Recommended | Explain missing `xdg-open` |
| Wi-Fi | `networkmanager` service | Optional | Status says package/service unavailable |
| Bluetooth | `bluez`, `bluez-utils` service | Optional | No Bluetooth results/actions |
| Audio | `pipewire`, `wireplumber`, `pipewire-pulse` | Optional | Audio provider unavailable |
| Power | `upower`, `power-profiles-daemon`, systemd-logind | Optional | Per-provider read-only fallback |
| Brightness | `brightnessctl` + device permissions | Optional | Hardware/permission diagnostic |
| Media | `playerctl` + MPRIS player | Optional | No active-player actions |
| Hyprland | `hyprland`, runtime socket | Optional | Generic Linux remains usable |

No package is installed automatically. `doctor` reports the exact Arch package and manual command.

## 10. File-search design

Defaults are `$HOME` plus valid XDG user directories and configured roots. The walker never defaults to `/`, never follows symlinks, and skips mandatory system-risk roots (`/proc`, `/sys`, `/dev`, `/run`, `/tmp`, `/var/tmp`, `/lost+found`) plus high-noise build/cache directories. Hidden files are not globally rejected; hidden-directory policy is explicit in settings. Inaccessible entries are skipped with a count, not a fatal error.

The initial implementation is direct traversal because it has no daemon dependency and can be tested on any Arch install. A future SQLite/FTS index can reuse the result model after measuring real workloads. `fd`, `locate`, Tracker, and AUR tools are not required.

## 11. Application-discovery design

The parser searches user entries before system entries across XDG data directories, honors `Hidden`, `NoDisplay`, `OnlyShowIn`, `NotShowIn`, `TryExec`, and `Type=Application`, and deduplicates by desktop ID. It parses `Name`, `GenericName`, `Comment`, `Keywords`, `Exec`, `Icon`, and `Terminal`. It does not invoke a shell or execute arbitrary desktop-file text. Field codes that require a file/URL are omitted when no object is supplied.

## 12. Linux settings catalogue

The catalogue contains: open settings/config directory, refresh/search roots, show hidden files, list applications, Wi-Fi status/toggle, Bluetooth status/toggle, audio volume/mute, brightness, battery/power profile, lock/suspend/reboot/logout with confirmation metadata, media controls, open default file manager/browser, Hyprland monitors/workspaces/config status, and capability diagnostics. Windows Update, Registry Editor, Control Panel, Explorer, Windows Search, Windows paths, and Windows terminology are excluded.

## 13. Wi-Fi architecture

NetworkManager is detected via `nmcli` and an optional service-state query. Status/toggle use fixed `nmcli` arguments. Passwords are never requested, logged, or placed in argv. Connecting to unknown networks is deferred until a UI can provide a secure interactive secret path; users can use NetworkManager’s normal UI.

## 14. Bluetooth architecture

BlueZ is detected through `bluetoothctl` and service/hardware state where visible. The initial safe actions are read-only status plus power toggle. Pairing, trust, and removal are deferred until a confirmation-capable UI and agent lifecycle are available; discovery is never left running indefinitely.

## 15. Audio and media architecture

`wpctl` is preferred for PipeWire/WirePlumber status, volume, and mute. `playerctl` is optional for MPRIS play/pause/next/previous. Human-readable output is used only as a provider fallback when no structured interface is available, and missing providers do not affect search.

## 16. Hyprland and dotfiles architecture

Hyprland is confirmed by `HYPRLAND_INSTANCE_SIGNATURE`, a valid XDG runtime socket, and a successful query-only IPC request; `XDG_CURRENT_DESKTOP=Hyprland` alone is insufficient. `hyprctl -j monitors`, `workspaces`, and `activeworkspace` are allowlisted query paths. Config discovery uses `HYPRLAND_CONFIG` or `$XDG_CONFIG_HOME/hypr`, resolves bounded `source` includes with cycle detection, and treats `exec*`, binds, plugins, and Lua as data. No dotfile is written or overwritten.

## 17. ProtonSearch settings migration

Windows settings are not path-translated. Linux settings use an independent JSON file under `$XDG_CONFIG_HOME/protonsearch/settings.json`, with data/state/cache under their corresponding XDG directories. Platform-neutral concepts retained are theme, search roots, ignored directories, hidden-file behavior, and startup preference. Win32 window position, registry startup, Segoe UI defaults, and Windows plugin switches are not imported.

## 18. Packaging plan

`packaging/arch/PKGBUILD` builds the standalone manifest and installs the binary, desktop entry, and documentation. The desktop entry starts the Linux CLI/launcher path without assuming Hyprland. Hyprland users can add their own keybind/autostart entry; ProtonSearch will not edit compositor configuration. A separate user service is unnecessary for the initial non-daemon CLI.

## 19. Security model

No `sh -c`, `bash -c`, `eval`, `sudo`, implicit package install, registry emulation, dotfile writes, or privilege escalation. Paths are canonicalized only when needed and validated as user-selected roots. Symlinks are not followed by default. External commands are allowlisted, use argv, have bounded output/time, and redact secrets. Destructive actions require an explicit `--confirm` path and are not exercised by tests.

## 20. Testing strategy

Unit tests cover XDG resolution, ignore policy, query matching, desktop parsing/field codes, visibility rules, safe action parsing, capability states, and Hyprland JSON parsing. Tests use temporary fixtures and harmless commands only. Integration checks run `cargo fmt --check`, `cargo test`, `cargo clippy`, the dependency checker, `doctor`, and missing-provider behavior. Hardware-changing actions are not invoked.

## 21. Risk register

* Desktop-file syntax and localization are broad; parser intentionally supports the common launch path and reports malformed entries.
* `nmcli`, `bluetoothctl`, and `wpctl` output varies by version; provider actions are narrow and optional.
* Hyprland IPC is compositor-specific and must not be presented as generic Wayland support.
* A standalone crate cannot replace the current Windows binary until a later platform-selection edit is reviewed.
* Direct traversal is correct but slower than a maintained index for very large homes; bounds and configurable roots limit impact.

## 22. Unsupported or deferred functionality

Windows-only OCR, WinRT, registry, Windows Store app discovery, taskbar/tray behavior, Win32 global hotkeys, window geometry actions, Explorer restart, Windows browser profile paths, Windows recent files, and PowerShell/AI native Windows tools are not exposed on Linux. Generic cross-desktop global hotkeys, pairing workflows, arbitrary display writes, tray integration, and full-text indexing remain deferred; the GTK4 launcher and Hyprland shortcut path are implemented.

## 23. Implementation sequence

1. Create the plan and baseline integrity evidence.
2. Add the isolated Linux crate and XDG/search/application core.
3. Add capability/provider queries and safe actions.
4. Add Hyprland read-only discovery and Linux settings.
5. Add Arch packaging/dependency guidance and the reproducible integrity verifier.
6. Run independent static, unit, and environment checks.
7. Produce summary, mapping, test, and integrity reports; run independent plan review.

## 24. Verification criteria

The Linux manifest builds on the current Arch host; unit tests pass; no Linux output contains Windows-only catalog entries; missing optional commands produce diagnostics rather than panics; desktop launch uses argv; search does not traverse mandatory system roots or follow links; Hyprland output is absent unless confirmed; existing files match the pre-work baseline.

## 25. Exact list of new files to be created

```text
protonsearch-linux/Cargo.toml
protonsearch-linux/src/lib.rs
protonsearch-linux/src/main.rs
protonsearch-linux/src/xdg.rs
protonsearch-linux/src/search.rs
protonsearch-linux/src/desktop.rs
protonsearch-linux/src/capabilities.rs
protonsearch-linux/src/system.rs
protonsearch-linux/src/actions.rs
protonsearch-linux/src/hyprland.rs
protonsearch-linux/src/settings.rs
scripts/linux/check-dependencies.sh
scripts/linux/verify-existing-files.sh
packaging/arch/PKGBUILD
packaging/arch/protonsearch-linux.desktop
docs/linux/IMPLEMENTATION_SUMMARY.md
docs/linux/DEPENDENCIES.md
docs/linux/SETTINGS_MAPPING.md
docs/linux/TEST_REPORT.md
docs/linux/EXISTING_FILE_INTEGRITY.md
```

`protonsearch-linux/Cargo.lock` may be generated by Cargo and is a new file if created; it is not a modification of the existing Windows lockfile.

## 26. Existing files that would ideally need later integration changes

The following existing files are intentionally not modified in this task: `protonsearch/Cargo.toml`, `protonsearch/src/main.rs`, `protonsearch/src/launcher.rs`, `protonsearch/src/search.rs`, `protonsearch/src/indexer.rs`, `protonsearch/src/settings.rs`, and `protonsearch/build.rs`. A future reviewed integration would add a platform-selected binary/workspace entry and connect the Linux result/action model to a Linux UI.

## 27. Proposed but unapplied patch strategy

No patch is needed to build or test the standalone Linux implementation. If product integration is later requested, create a new `patches/linux/platform-selection.diff` that only adds a Cargo workspace/member or target-specific binary selection and keeps the existing Windows path unchanged. Review and apply it separately; it is intentionally not generated or applied here.
