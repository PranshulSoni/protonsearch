# Linux settings mapping

| Existing Windows setting or feature | Linux status |
|---|---|
| `%APPDATA%` settings/index location | Replaced by XDG config/data/state paths |
| Scan folders | Supported as Linux `search_roots` |
| Ignored folders | Supported as Linux `ignored_names` |
| Hidden files | Supported, off by default |
| Windows Start Menu and `.lnk` apps | Replaced by XDG `.desktop` entries |
| Launch at Windows startup/Registry Run | Deferred to XDG autostart/user-approved setup |
| Global `Alt+Space` Win32 hotkey | Deferred; Wayland has no universal grab |
| Windows taskbar hide/show | Unsupported cross-desktop |
| Win32 window position/monitor selection | Deferred to a Linux UI/compositor adapter |
| Windows themes/Segoe UI/wallpaper preview | Not exposed; Linux UI owns its theme |
| Windows Search/Indexing Options | Replaced by Linux search roots and exclusions |
| Windows Update, activation, Registry Editor | Not exposed |
| Wi-Fi settings/toggle | Supported through NetworkManager when available |
| Bluetooth settings/toggle | Supported through BlueZ when available |
| Sound/volume/mute | Supported through PipeWire/WirePlumber when available |
| Display/monitor information | Read-only Hyprland provider when confirmed |
| Brightness | Supported through `brightnessctl` when available |
| Battery and power profiles | Capability-detected; provider action support varies |
| Lock/suspend/reboot/logout/poweroff | Supported through systemd/logind with confirmation for destructive actions |
| Media controls | Supported through MPRIS/`playerctl` when available |
| Default apps/file manager/browser | `gio`/`xdg-open` path is supported |
| Windows clipboard history/OCR | Deferred; no universal Linux equivalent |
| Circle-to-Search/GDI capture | Deferred to portal-based implementation |
| Explorer/recycle-bin/process/window actions | Not exposed in this backend |
| AI/Hermes arbitrary command execution | Intentionally unsupported for security |
