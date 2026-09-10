# ProtonSearch Linux implementation summary

## New implementation

The additive `protonsearch-linux` Cargo package is an Arch/Linux-native GTK4 launcher and CLI. It is deliberately separate from the existing Win32 crate because the current product entry point unconditionally depends on Win32, COM, WinRT, registry APIs, and Windows paths.

Implemented modules:

* `xdg`: XDG base directories, `user-dirs.dirs`, safe default roots, mandatory system exclusions, and hidden-file policy.
* `search`: bounded case-insensitive file/directory search with no symlink following.
* `desktop`: XDG `.desktop` discovery, visibility filtering, localized names, `TryExec`, field-code parsing, and argv-only launch.
* `capabilities`: independent provider states and Arch package guidance.
* `system`: allowlisted executable lookup, three-second process timeout, bounded output, and safe `gio`/`xdg-open` use.
* `actions`: NetworkManager, BlueZ, PipeWire, brightness, MPRIS, and systemd/logind actions.
* `hyprland`: query-only JSON IPC and bounded, read-only config include discovery.
* `settings`: Linux-only JSON schema and catalogue.
* `gui`: GTK4 application launcher window with asynchronous search, keyboard navigation, hoverable rows, category scopes, resident toggle IPC, and editable settings UI.
* `calculator`: dependency-free expression parser with functions and percentage expressions.
* `providers`: applications, files/folders, settings, commands, recent files, browser bookmarks/history, Git commits, content/PDF/OCR search, clipboard history, snippets, notes, quicklinks, and Hyprland window switching.

## Architecture decisions

* Windows catalog data is not reused, so Windows settings cannot appear on Linux.
* Optional providers are capability-gated and never installed automatically.
* Wayland/Hyprland is detected rather than assumed; generic Linux remains usable without Hyprland.
* Dotfiles are read-only discovery inputs and are never changed.
* No arbitrary shell, PowerShell, `sudo`, package manager, or AI command execution exists in this Linux path.

## Tested capabilities

Unit tests cover XDG policy, hidden-file behavior, desktop parsing/visibility, Exec tokenization, shell-free executable lookup, Hyprland query allowlisting, case-insensitive search, and calculator expressions. The GTK4 launcher builds, stays open in the current Wayland session, and repeated invocations toggle through the per-user IPC socket. Cargo tests pass on the current Arch kernel/toolchain.

## Known limitations

Wayland does not expose one universal global-hotkey API, so the launcher uses a compositor-owned shortcut. Hyprland users add `bind = SUPER, SPACE, exec, protonsearch-linux gui`; other desktop environments can assign a shortcut to the installed desktop entry. Tray integration, a persistent filesystem index, screenshot-portal upload to visual search services, and desktop-specific window controls remain provider-dependent or deferred.
