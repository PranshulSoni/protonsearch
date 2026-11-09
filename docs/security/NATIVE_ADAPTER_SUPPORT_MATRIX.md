# Native adapter support matrix — MILESTONE 1 FINAL (task-35)

| Field | Value |
|---|---|
| Status | **Milestone 1 final** — wired end-to-end by the shell/UI; task-35 closed as milestone-1 complete |
| Date | 2026-11-10 (Asia/Calcutta) |
| Host | Windows 11, AMD Ryzen AI 9 (10C/20T), 31 GB RAM, Cirrus Logic onboard audio |
| Evidence | @native-probe-host.log (regenerated capability matrix + live device data) |
| Code | crates/ps-windows: audio, brightness, capabilities, toast, winrt modules; crates/ps-sources: notifications store |

## Decision
Milestone 1 (capabilities + Core Audio volume/output + brightness + local notification store) is complete, verified and wired. Milestone 2 (GSMTC media transport, Wi-Fi network connect, Bluetooth device list/radio toggle) is **deferred to post-alpha**; it needs its own contract/IPC additions and real media/network/Bluetooth hardware verification that should not be rushed into this release. To keep the UI honest, the not-yet-shipped adapters now report `supported=false` with explicit reasons (they previously reported raw API availability, which could have produced non-functional controls).

## Capability matrix on this host (milestone-1 final)

| Capability | Supported | Host result / reason | State |
|---|---|---|---|
| audio_output | **yes** | 1 active render endpoint: `Speakers (Cirrus Logic XU (with APO Extensions))`, marked default | shipped |
| audio_volume | **yes** | master volume 0.88, muted=false; read/write round-trip with state restore | shipped |
| audio_output_switch | **yes** | IPolicyConfig interface created; SetDefaultEndpoint implemented | shipped (single endpoint on this host) |
| local_notifications | **yes** | SQLite local store: add/list/unread/mark_read/dismiss/clear | shipped |
| brightness | **no** | no monitor exposes user-level brightness control (external desktop monitor; Dxva2 per-monitor detection) | capability-gated in UI |
| toast_notifications | **no** | 0x80073D54: process has no package identity (unpackaged dev binary) | honest limitation; fix = AUMID shortcut at install |
| media_sessions | **no** | 'GSMTC media enumeration/transport controls are a post-alpha follow-up; capability probe exists but the control API is not shipped in this build' | milestone 2 (post-alpha) |
| wifi | **no** | 'Wi-Fi network list/connect is a post-alpha follow-up; the UI should open Windows network settings for now' | milestone 2 (post-alpha) |
| bluetooth | **no** | 'Bluetooth device list/radio toggle is a post-alpha follow-up; the UI should open Windows Bluetooth settings for now' | milestone 2 (post-alpha) |
| airplane_mode / energy_saver / hotspot / cast / night_light | no | explicit unsupported reasons | not planned for alpha |

## Shipped API surface (milestone 1)
`cargo
ps_windows::native_capabilities() -> NativeCapabilities   // serde; per-feature supported + reason
ps_windows::list_audio_outputs() / default_audio_output() / set_audio_output(id)
ps_windows::get_audio_volume() / set_audio_volume(f32) / get_audio_mute() / set_audio_mute(bool) / audio_state()
ps_windows::brightness_supported() / brightness_monitors() / get_brightness() / set_brightness(u8)
ps_windows::toast_supported() / show_toast(title, body)
ps_sources::notifications::{add, list, unread_count, mark_read, dismiss, clear, add_and_maybe_toast}
`
All user-level (no admin). Capability probes are bounded at 3 s each and side-effect free.

## Milestone 2 — post-alpha follow-up plan (explicitly deferred)

### (1) GSMTC media sessions/transport
- `GlobalSystemMediaTransportControlsSessionManager::RequestAsync().get()` (already probed) -> `GetSessions()`; per session: `GetPlaybackInfo()` (status + supported controls), `TryGetMediaPropertiesAsync()` (title/artist/album + thumbnail via `RandomAccessStreamReference`, bounded <=512 KiB, no network), `GetTimelineProperties()` for position/seek support; `TryTogglePlayPause` / `TrySkipNext` / `TrySkipPrevious`; shuffle/repeat/seek exposed only when playback-info reports support.
- New ps-ipc contract additions needed (L0): `MediaSessionInfo` {sessionId, sourceAppUserModelId, title, artist, album, playbackStatus, canPlayPause/canSkip, positionMs/durationMs, artwork (bounded bytes) } + `MediaAction` enum + transport result DTO.
- Tests: enumerate sessions/source apps; control calls guarded by playback status; artwork size bound; honest unsupported when no session/media apps. Requires a real playing media app for control verification.

### (2) Wi-Fi
- WlanAPI: `WlanOpenHandle` / `WlanEnumInterfaces` / `WlanGetAvailableNetworkList` (SSID, signal quality, security, profile name, connectable flag); radio state via WinRT `Radio` (RadioKind::WiFi, `SetStateAsync`) with `RadioAccessStatus` gating; `WlanConnect` only against an existing profile (`WlanGetProfileNames`), never prompts for keys; missing profile/permission -> structured error + 'open Windows network settings'.
- IPC additions needed: `WifiNetworkInfo` {ssid, signal, security, hasProfile, connected} + `WifiRadioState` + connect result DTO.
- Tests: network list on host; radio state read (no toggle during tests); connect path only when a saved profile exists, otherwise asserts structured error.

### (3) Bluetooth
- WinRT `Radio`: enumerate radios, Bluetooth on/off via `SetStateAsync` gated on `RadioAccessStatus`; paired devices via `DeviceInformation::FindAllAsync(BluetoothDevice::GetDeviceSelectorFromPairingState(true))` -> name/id/connected; pairing and connection remain OS UI; no fake pairing. Connect only if the platform exposes it for the device class, otherwise return unsupported.
- IPC additions needed: `BluetoothRadioState` + `BluetoothDeviceInfo` {id, name, connected, paired}.
- Tests: radio enumeration, device list; toggle only with explicit user action in manual E2E (never in automated tests to avoid changing the user's radios).

### Preconditions / risks for milestone 2
- WinRT async + COM apartment discipline on background threads; bounded calls with timeouts.
- New ps-ipc DTOs (L0-owned) and UI card wiring; cannot be verified without real media/network/Bluetooth hardware and a release-candidate pass.
- No admin required anywhere; no network access for artwork (local stream only).

## Verification (milestone 1 final)
- `cargo test -p ps-windows`: 7 unit + 15 integration = 22 passed / 0 failed (capability probe bounded + serde round-trip, audio enumeration, volume/mute restore, brightness gated round-trip, toast honesty).
- `cargo test -p ps-sources`: 18 unit + 3 notification-store tests + 31/32 sources tests passed / 0 failed (1 ignored action smoke).
- `cargo check -p ps-windows -p ps-sources -p ps-content -p ps-ocr --all-targets` clean.
- Host probe: `cargo run -p ps-windows --example native_probe` -> @native-probe-host.log.