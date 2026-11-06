# Native widget providers

Native Rust is authoritative for every value below. Nothing is sampled from the
DOM, fabricated, or inferred from a provider that is not running. Commands are
read-only except for the existing media and settings actions.

## Envelope

```json
{
  "status": "live",
  "data": {},
  "message": "",
  "updatedAt": 1767225600000
}
```

`updatedAt` is Unix epoch milliseconds. `message` is empty for healthy reads and a
short human-readable explanation otherwise. Statuses used by these providers:

| Status | Meaning |
| --- | --- |
| `live` | At least one real value was read and the provider is usable. |
| `loading` | A connection is present but throughput needs a second comparable sample. |
| `empty` | The API worked, but no matching hardware, session or folder exists. |
| `permission-denied` | Windows denied access to the necessary API or user data. |
| `unsupported` | The required API/radio is unavailable on this host or build. |

`error` is reserved for a command-level failure. Missing sensors are `null`, never
zero, and never labelled `live` on their own.

## `provider_snapshot("network")`

```json
{
  "status": "live",
  "updatedAt": 1767225600000,
  "message": "",
  "data": {
    "downloadBytesPerSecond": 12500.0,
    "uploadBytesPerSecond": 800.0,
    "interfaces": ["Wi-Fi"],
    "connectionType": "vpn",
    "wifi": {
      "connected": false,
      "ssid": null,
      "signalPercent": null,
      "state": "disconnected"
    },
    "ethernet": {
      "connected": false,
      "adapter": null
    },
    "vpn": {
      "connected": true,
      "adapter": "PANGP Virtual Ethernet Adapter Secure"
    },
    "bluetooth": {
      "available": true,
      "enabled": false,
      "name": "Bluetooth"
    }
  }
}
```

Existing keys are preserved: `downloadBytesPerSecond`, `uploadBytesPerSecond` and
`interfaces`.

- `connectionType` is the best active connection: `vpn`, then `wifi`, then
  `ethernet`, otherwise `none`.
- `wifi.connected` comes from the WLAN API, not from the NDIS adapter state. An
  enabled but unassociated Wi-Fi adapter is therefore `connected: false`.
- `wifi.state` is one of `connected`, `disconnected`, `associating`,
  `authenticating`, `discovering`, `ad_hoc`, `disconnecting`, `not_ready`,
  `unknown`, or `unsupported`. `unsupported` means the WLAN service/API could not
  be opened or no WLAN interface exists.
- `wifi.signalPercent` is the native WLAN signal quality clamped to `0..=100`,
  or `null` unless the interface is connected.
- `ethernet` only reports physical Ethernet adapters that are up. Hyper-V, WSL,
  Npcap, WFP and VPN virtual adapters are not Ethernet.
- `vpn` uses adapter type and name heuristics (`PPP`, VPN/WireGuard/OpenVPN/
  Tailscale/ZeroTier/TAP/WinTun/L2TP/IKEv2/SSTP/PPTP/GlobalProtect/PANGP/tunnel)
  and excludes Teredo, ISATAP and 6to4. It is a best-effort label, not a security
  guarantee.
- `bluetooth.available` is true only when a Bluetooth radio was enumerated.
  `enabled` is true only when the radio state is `On`; an unreadable state is
  reported as `false`, never guessed. `name` is `null` when Windows does not
  expose one. The provider does not call `RequestAccessAsync`, so it never shows
  a consent prompt; failure degrades to the same `available: false` shape.
- `interfaces` lists up hardware Ethernet/Wi-Fi adapter names used for throughput.
  An unassociated Wi-Fi adapter can still appear here; use `wifi.connected` and
  `connectionType` for connectivity.
- Throughput is `null` until two comparable adapter samples exist. Counters are
  keyed by adapter GUID, and resets/new adapters are ignored rather than producing
  negative or invented rates.
- `status` is `empty` when no recognized connection exists, `loading` when a
  connection exists but throughput is not yet comparable, and `live` otherwise.
  The empty message is `No active network connection`.

Windows APIs: `GetIfTable2`/`FreeMibTable` (IP Helper), `WlanOpenHandle`,
`WlanEnumInterfaces`, `WlanQueryInterface(wlan_intf_opcode_current_connection)`,
`WlanCloseHandle`, `WlanFreeMemory` (WLAN API), and
`Radio::GetRadiosAsync`/`Radio::Kind`/`Radio::State`/`Radio::Name`
(`Windows.Devices.Radios`). No `netsh` process is spawned.

## `provider_snapshot("system")`

```json
{
  "status": "live",
  "updatedAt": 1767225600000,
  "message": "",
  "data": {
    "cpu": 12.5,
    "ram": 47,
    "battery": 81,
    "batteryCharging": true,
    "temperatureC": null,
    "uptimeSeconds": 18342
  }
}
```

- `cpu` is a percentage from `GetSystemTimes`, or `null` on the first sample
  while the previous counters are established.
- `ram` remains `GlobalMemoryStatusEx().dwMemoryLoad`, a `0..=100` percentage.
- `battery` remains the Windows battery percentage, or `null` when no battery is
  reported (`BatteryLifePercent == 255`).
- `batteryCharging` is `true`/`false` from the charging flag, or `null` when the
  battery or flags are unknown or the host reports no battery.
- `temperatureC` is always `null`. There is no real, non-fabricated temperature
  source in this provider yet; do not render a number.
- `uptimeSeconds` is `GetTickCount64() / 1000`, i.e. milliseconds since boot
  truncated to whole seconds.

Windows APIs: `GlobalMemoryStatusEx`, `GetSystemPowerStatus`, `GetSystemTimes`,
`GetTickCount64`.

Storage remains the existing `{totalBytes, freeBytes, mountPoint}` shape from
`GetDiskFreeSpaceExW`.

## `known_folders_snapshot`

Registered Tauri command; the React layer does not call it yet.

```json
{
  "status": "live",
  "updatedAt": 1767225600000,
  "message": "",
  "data": {
    "folders": [
      { "id": "desktop", "name": "Desktop", "path": "C:\\Users\\me\\Desktop", "icon": "desktop" },
      { "id": "documents", "name": "Documents", "path": "C:\\Users\\me\\Documents", "icon": "documents" },
      { "id": "downloads", "name": "Downloads", "path": "C:\\Users\\me\\Downloads", "icon": "downloads" },
      { "id": "pictures", "name": "Pictures", "path": "C:\\Users\\me\\Pictures", "icon": "pictures" },
      { "id": "projects", "name": "Projects", "path": "C:\\Users\\me\\Documents\\Projects", "icon": "folder" }
    ],
    "recent": [
      {
        "name": "report.pdf",
        "path": "C:\\Users\\me\\Documents\\report.pdf",
        "kind": "pdf",
        "modifiedAt": 1767222000000
      }
    ]
  }
}
```

- Folder order: desktop, documents, downloads, pictures, screenshots (only if it
  exists), projects. `projects` is `Documents\Projects` when present, otherwise
  `%USERPROFILE%\Projects`; it is omitted when neither exists.
- Paths come from `SHGetKnownFolderPath` with `FOLDERID_*` and are freed with
  `CoTaskMemFree`. No user name is hardcoded. Only existing local absolute drive
  paths are returned; UNC/relative paths are rejected.
- `recent` contains at most 12 files from the returned known folders, scanned two
  directory levels deep with a per-root budget. Reparse points are skipped and not
  followed. It reads names, paths and modification times only: no file contents,
  no shell execution.
- `kind` is one of `image`, `pdf`, `document`, `spreadsheet`, `presentation`,
  `video`, `audio`, `archive`, `code`, `other`.
- `status` is `live` when at least one folder was resolved, `permission-denied`
  when Windows denied all known-folder lookups, and `empty` otherwise. `recent`
  may be an empty array even when folders are live.

Windows APIs: `SHGetKnownFolderPath`, `CoTaskMemFree`, `symlink_metadata` /
`FILE_ATTRIBUTE_REPARSE_POINT` for the recent listing.

## Permissions and host behaviour

- Required capabilities: `system:read` for system, `network:read` for network,
  `storage:read` for storage and known folders, `media:read` for media.
- No administrator elevation, credentials, tokens, browser cookies, AI provider
  APIs, or network requests are used.
- WLAN/radio/folder failures return `unsupported`, `empty` or
  `permission-denied` with a message; they do not panic on headless hosts or VMs.
- Bluetooth never requests user consent. If the radio cannot be enumerated or its
  state cannot be read, `enabled` stays `false`.

## React follow-up

- Treat every nullable field as nullable; do not coerce `null` to `0`.
- Render battery from the system provider, not network.
- Do not show a temperature until a real source is added.
- Use `wifi.state == "unsupported"` to hide/disable Wi-Fi detail separately from
  `connected: false`.
- `connectionType == "vpn"` can be true while `wifi.connected` is false or while
  `ethernet.connected` is false; that is expected.
- The files widget can call `invoke("known_folders_snapshot")` and read
  `data.folders` / `data.recent` after this native command is wired into the UI.
