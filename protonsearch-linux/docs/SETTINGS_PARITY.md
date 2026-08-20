# ProtonSearch settings parity

This matrix is the contract for the Linux settings app. Windows settings are
mapped to a Linux-native implementation where one exists. Windows-only APIs
are shown explicitly so the Linux UI does not present controls that cannot
work on the current desktop.

## Windows settings pages

| Windows setting or feature | Linux mapping | Status |
| --- | --- | --- |
| Run on startup | systemd user service, with XDG autostart fallback | Supported |
| Hide when launcher loses focus | GTK focus-out hide | Supported |
| Show taskbar icon | Desktop/tray integration; Wayland-dependent | Best effort |
| Window location and remembered position | Compositor-managed placement; no forced Wayland coordinates | Desktop-dependent |
| Window width and result height | GTK launcher dimensions | Supported |
| Search-bar height | GTK launcher styling | Supported |
| Query/result fonts | GTK/system font selection | Desktop-dependent |
| Dark, Light, and Nord themes | GTK theme mode | Supported |
| Wallpaper preview | Native desktop appearance settings | Not applicable |
| Alt+Space launcher shortcut | Hyprland/Sway adapter or desktop shortcut instructions | Desktop-dependent |
| Scan folders | XDG roots plus configured absolute search roots | Supported |
| Ignored folders | Ignored directory names | Supported |
| Hidden files | Include hidden entries | Supported |
| Placeholder visibility | Search entry placeholder | Supported |
| Calculator | Built-in calculator provider | Supported |
| Git commits | Bounded Git provider | Supported |
| Clipboard history and image preview | Wayland clipboard/cliphist provider | Supported when installed |
| OCR search | Deferred provider with an explicit Coming Soon result | Deferred |
| PDF search | Poppler `pdftotext` provider | Supported when installed |
| Browser history/bookmarks | Browser profile providers | Supported when profile is readable |
| Text expansions/snippets | Linux workflow/snippet provider | Partial |
| Circle to Search / color picker | Portal or desktop-specific tools | Optional/deferred |
| AI agent API settings | Hermes Agent desktop integration and session history when `hermes` is installed | Supported when installed |
| Indexed database/rebuild | Bounded on-demand search; roots and exclusions are authoritative | Different implementation |
| Windows Update/activation | Distribution/package manager workflow | Not applicable |

## Windows native command groups

| Windows group | Linux-native equivalent |
| --- | --- |
| Wi-Fi | NetworkManager (`nmcli`) and native network settings |
| Bluetooth | BlueZ (`bluetoothctl`) and Blueman/GNOME/KDE settings |
| Volume/mute | PipeWire/WirePlumber (`wpctl`) and native audio settings |
| Brightness | `brightnessctl` when hardware and permissions allow |
| Display | GNOME/KDE/XFCE settings or `wdisplays` |
| Power profile | `powerprofilesctl` |
| Lock/suspend/reboot/logout/poweroff | `loginctl`/`systemctl`, with explicit destructive confirmation |
| Media playback | MPRIS through `playerctl` |
| Open folders/files/URLs | XDG user directories and `gio`/`xdg-open` |
| Screenshot | `grim`/`slurp` or the desktop screenshot portal |
| Apps | XDG `.desktop` entries |
| Windows Explorer, Registry, Task Manager, recycle bin, Win32 tiling | No direct Linux equivalent; not exposed as working Linux settings |

## Implementation rules

1. A setting must be persisted, validated, and consumed by the runtime before
   it is shown as enabled.
2. Optional providers must expose readiness and a useful missing-dependency
   message instead of freezing or silently doing nothing.
3. Wayland compositor ownership stays with the compositor. ProtonSearch may
   install a documented adapter for Hyprland/Sway, but must not claim a
   universal global-hotkey API.
4. Destructive session actions always require an explicit confirmation at the
   point of activation; a preference may control whether the UI asks first,
   but it must not make the CLI unsafe.

## Installation and capability contract

The Arch/user installer checks optional provider executables and offers an
explicit `pacman` installation prompt. `--no-install-optional` is available for
minimal or audited installs, while `--install-optional` is available for
non-interactive release provisioning. The `doctor` command reports the actual
provider state for the current machine, including missing packages, inactive
services, unsupported hardware, and Hermes availability.

Linux intentionally does not download Hermes Agent or other third-party tools.
When `hermes` is on the user's `PATH`, Agents opens its desktop workspace and
Agent History reads the installed session list. This keeps credentials and
third-party updates under the user's control.
