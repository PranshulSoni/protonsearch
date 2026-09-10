# ProtonSearch Linux dependencies

## Required

* Rust/Cargo for building from source.
* `glibc` at runtime.
* `gtk4` for the graphical launcher window.
* A normal unprivileged user session.

## Optional Arch packages

| Package | Utility/service | Feature |
|---|---|---|
| `xdg-utils` | `xdg-open` | Open files and URLs |
| `wl-clipboard` | `wl-paste`, `wl-copy` | Wayland clipboard access |
| `cliphist` | `cliphist` | Persistent clipboard history search |
| `sqlite` | `sqlite3` | Browser history search |
| `tesseract` | `tesseract` | OCR search for images/screenshots |
| `poppler` | `pdftotext` | PDF content search |
| `grim` | `grim` | Hyprland screenshot capture |
| `slurp` | `slurp` | Screenshot area selection |
| `networkmanager` | `nmcli`, `NetworkManager.service` | Wi-Fi |
| `bluez-utils` | `bluetoothctl`, `bluetooth.service` | Bluetooth |
| `wireplumber` | `wpctl` | PipeWire audio |
| `brightnessctl` | `brightnessctl` | Backlight hardware |
| `playerctl` | `playerctl` | MPRIS media |
| `systemd` | `loginctl`, systemd-logind | Session/power |
| `upower` | `upower` or `/sys/class/power_supply` | Battery status |
| `power-profiles-daemon` | `powerprofilesctl` | Power profiles |
| `hyprland` | `hyprctl` and runtime IPC | Hyprland monitors/workspaces |

Use Arch’s normal package management manually, for example:

```text
sudo pacman -S xdg-utils wl-clipboard cliphist sqlite tesseract poppler networkmanager bluez-utils wireplumber brightnessctl playerctl power-profiles-daemon
```

ProtonSearch never runs this command, never invokes `sudo`, and never enables a service. Enable services through the user’s normal system administration workflow.

## Detection

Run:

```text
cargo run --manifest-path protonsearch-linux/Cargo.toml -- doctor
scripts/linux/check-dependencies.sh
```

Missing utilities produce `package_missing`; inactive providers produce `service_inactive`; missing Hyprland IPC produces a nonfatal configuration/desktop state.

## Desktop/session notes

The core search and `.desktop` discovery work in TTY, X11, Wayland, and Hyprland environments. `xdg-open` needs a graphical desktop. Hyprland-specific data requires `HYPRLAND_INSTANCE_SIGNATURE`, a valid session runtime, and a successful query response. No dotfile is edited.
