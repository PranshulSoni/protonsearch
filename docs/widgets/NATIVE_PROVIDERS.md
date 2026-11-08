# Native widget providers

Native Rust is authoritative for every value below. Nothing is sampled from the
DOM, fabricated, or inferred from a provider that is not running. Provider reads
are read-only. Only explicit `widget_action` IDs can change media transport,
system settings, volume, the Recycle Bin, or open an allowlisted known folder.

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
  Bluetooth is sampled once per process: the windows 0.58 radio APIs crash when
  `GetRadiosAsync`/`State` are called repeatedly, so the first real observation
  is cached. A radio toggled after the first sample is not reflected until the
  app restarts; this is a documented native limitation, not a live feed.
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
    "uptimeSeconds": 18342,
    "connectivity": {
      "downloadBytesPerSecond": 12500.0,
      "uploadBytesPerSecond": 800.0,
      "interfaces": ["Wi-Fi"],
      "connectionType": "wifi",
      "wifi": {
        "connected": true,
        "ssid": "Home",
        "signalPercent": 82,
        "state": "connected"
      },
      "ethernet": { "connected": false, "adapter": null },
      "vpn": { "connected": false, "adapter": null },
      "bluetooth": { "available": true, "enabled": true, "name": "Bluetooth" }
    }
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
- `connectivity` is the network provider's entire `data` object, embedded without
  its envelope. It is `null` only when the network read itself fails; the system
  provider still stays `live` because the system metrics are independently real.
  Do not read connectivity from `system.status`.

Windows APIs: `GlobalMemoryStatusEx`, `GetSystemPowerStatus`, `GetSystemTimes`,
`GetTickCount64`, plus the network provider APIs listed above for `connectivity`.

## `provider_snapshot("media")`

```json
{
  "status": "live",
  "updatedAt": 1767225600000,
  "message": "",
  "data": {
    "title": "Track title",
    "artist": "Artist",
    "album": "Album",
    "albumArtist": "Album artist",
    "source": "Spotify.exe",
    "playing": true,
    "playbackStatus": "playing",
    "playbackRate": 1.0,
    "positionMs": 42000,
    "durationMs": 215000,
    "canToggle": true,
    "canNext": true,
    "canPrevious": true
  }
}
```

- Existing keys `title`, `artist`, `source`, `playing`, `canToggle`, `canNext`
  and `canPrevious` are preserved.
- `album` and `albumArtist` are `null` when the media properties do not expose
  them. `positionMs` and `durationMs` are `null` when the session has no timeline.
  Timeline values are converted from GSMTC 100 ns ticks; position is relative to
  the timeline start and duration is `end - start`.
- `playbackRate` is `null` when the session does not expose a rate. It is usually
  `1.0` for normal playback.
- `playbackStatus` is one of `closed`, `opened`, `changing`, `stopped`,
  `playing`, `paused` or `unknown`. `playing` remains the boolean used by the
  existing widget.
- `status` is `empty` with `data: null` when no session exists, and
  `permission-denied` when Windows rejects media-session access.
- No artwork is read. `TryGetMediaPropertiesAsync` is used only for text metadata.

Windows APIs: `GlobalSystemMediaTransportControlsSessionManager`,
`GlobalSystemMediaTransportControlsSession.TryGetMediaPropertiesAsync`,
`GetPlaybackInfo`, `GetTimelineProperties`, `PlaybackRate`, and the existing
transport control calls.

## `provider_snapshot("volume")`

```json
{
  "status": "live",
  "updatedAt": 1767225600000,
  "message": "",
  "data": {
    "level": 16,
    "muted": true,
    "device": "Speakers (Cirrus Logic XU (with APO Extensions))"
  }
}
```

- `level` is the default render endpoint's master scalar rounded and clamped to
  `0..=100`. `muted` is the endpoint mute flag. `device` is the endpoint friendly
  name, or `Default output device` if Windows exposes no name.
- `status` is `empty` with `data: null` and message `No audio output device`
  when no default render endpoint exists; `permission-denied` when access is
  denied; `unsupported` with a diagnostic message for other Core Audio failures.
- The read does not change volume or mute state.

Windows APIs: `IMMDeviceEnumerator::GetDefaultAudioEndpoint(eRender, eConsole)`,
`IMMDevice::Activate<IAudioEndpointVolume>`,
`IAudioEndpointVolume::GetMasterVolumeLevelScalar`, `GetMute`, and
`IMMDevice::OpenPropertyStore` for `PKEY_Device_FriendlyName`.

## `provider_snapshot("trash")`

```json
{
  "status": "live",
  "updatedAt": 1767225600000,
  "message": "",
  "data": {
    "itemCount": 0,
    "sizeBytes": 0,
    "lastEmptiedAt": null
  }
}
```

- `itemCount` and `sizeBytes` are aggregate Recycle Bin values across all drives.
  Negative native values are clamped to zero.
- `lastEmptiedAt` is always `null`. Windows does not provide a reliable
  last-emptied timestamp through `SHQueryRecycleBinW`; do not invent one.
- `status` is `live` when the query succeeds, `permission-denied` when Windows
  denies access, and `unsupported` with a diagnostic message for other shell
  failures.

Windows API: `SHQueryRecycleBinW` with a null root path.

## Actions

All of these are accepted only through `widget_action(action_id)`. They are not
provider reads and do not accept arbitrary paths, shells, or scripts.

| Action ID | Effect |
| --- | --- |
| `volume:up` | Raise the default output volume by 5 percentage points, clamped at 100. |
| `volume:down` | Lower the default output volume by 5 percentage points, clamped at 0. |
| `volume:mute` | Toggle the default output mute state. |
| `volume:set:<0..100>` | Set the master volume to the parsed integer, clamped to `0..=100`. |
| `trash:empty` | Empty the Recycle Bin with `SHERB_NOCONFIRMATION \| SHERB_NOPROGRESSUI \| SHERB_NOSOUND`. Irreversible; the UI should confirm first. |
| `folder:open:<folder-id>` | Open Explorer at an allowlisted known folder: `desktop`, `documents`, `downloads`, `pictures`, `screenshots`, `projects`. |

- A malformed `volume:set` value or unknown volume suffix fails with
  `Unsupported widget action` before touching the endpoint.
- `folder:open` resolves the ID through `SHGetKnownFolderPath` again. It rejects
  path strings, `..`, and unknown IDs. The resolved path is passed to the existing
  ShellExecuteExW opener; there is no shell command construction.
- `trash:empty` and volume actions are intentionally not invoked by the automated
  tests because they change user state. The provider reads are tested.

Windows APIs: `IAudioEndpointVolume::SetMasterVolumeLevelScalar`, `SetMute`,
`SHEmptyRecycleBinW`, and `ShellExecuteExW` through the existing `open_uri`.

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
  `storage:read` for storage, trash and known folders, `media:read` for media,
  `media:control` for media transport. Volume, trash-empty and folder-open remain
  behind the existing `widget_action` command and the widget layer's action
  allowlist; they introduce no new capability name.
- No administrator elevation, credentials, tokens, browser cookies, AI provider
  APIs, or network requests are used.
- WLAN/radio/folder failures return `unsupported`, `empty` or
  `permission-denied` with a message; they do not panic on headless hosts or VMs.
- Bluetooth never requests user consent. If the radio cannot be enumerated or its
  state cannot be read, `enabled` stays `false`. Its first successful sample is
  cached for the process lifetime because repeated radio reads crash under
  windows 0.58.
- Native provider and action calls are serialized by a process-wide lock. A slow
  media/volume query therefore delays another provider call briefly; it does not
  run concurrently with a write action.

## React follow-up

- Treat every nullable field as nullable; do not coerce `null` to `0`.
- Render battery from the system provider, not network.
- Do not show a temperature until a real source is added.
- Use `wifi.state == "unsupported"` to hide/disable Wi-Fi detail separately from
  `connected: false`.
- `connectionType == "vpn"` can be true while `wifi.connected` is false or while
  `ethernet.connected` is false; that is expected.
- `system.data.connectivity` is nullable and is the network `data` object, not a
  status. Read `connectionType`, `wifi`, `ethernet`, `vpn` and `bluetooth` from
  it; do not require `system.status == "live"` to mean connectivity is live.
- Media `album`, `albumArtist`, `playbackRate` and the timeline fields can be
  `null`. Use `playbackStatus` for a richer label and keep `playing` as the
  primary transport state. Do not render a zero-duration progress bar when
  `durationMs` is null or zero.
- Volume and trash return `data: null` for `empty`, `unsupported` or
  `permission-denied`; render the message and do not coerce the missing object.
- Send only the documented action IDs. Confirm `trash:empty` with the user before
  invoking it; it bypasses the Windows confirmation UI. Do not construct
  `folder:open` from a path, only from one of the six IDs.
- The files widget can call `invoke("known_folders_snapshot")` and read
  `data.folders` / `data.recent` after this native command is wired into the UI.

## Wi-Fi access denied (2026-11-08)

`network.wifi.accessDenied` is true when Windows withholds connection details for lack of precise-location consent; the dock then reports connected without a name. `settings:location` opens `ms-settings:privacy-location`. The same consent gates Wi-Fi scans and Geolocator.
