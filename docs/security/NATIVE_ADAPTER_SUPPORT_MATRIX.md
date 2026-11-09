# Native adapter support matrix — host evidence (task-35 milestone 1)

| Field | Value |
|---|---|
| Milestone | task-35 milestone 1: capability probes, Core Audio volume/output, brightness, local notification store |
| Date | 2026-11-10 (Asia/Calcutta) |
| Host | Windows 11, AMD Ryzen AI 9 (10C/20T), 31 GB RAM, Cirrus Logic onboard audio |
| Evidence | @native-probe-host.log (raw JSON capability matrix + live device data) |
| API | crates/ps-windows (+ crates/ps-sources::notifications) — user-level, no admin, bounded probes (3 s per capability) |

## Capability matrix on this host

| Capability | Supported | Host result / reason | Milestone status |
|---|---|---|---|
| media_sessions (GSMTC) | **yes** | GSMTC session manager created via WinRT | probe done; enumeration/transport controls in milestone 2 |
| audio_output (enumerate render endpoints) | **yes** | 1 active device: 
`Speakers (Cirrus Logic XU (with APO Extensions))`, default | done |
| audio_volume (master volume/mute) | **yes** | read/write verified: volume 0.88, muted=false; round-trip test restores state | done |
| audio_output_switch (default device) | **yes** | IPolicyConfig interface created; SetDefaultEndpoint implemented (not exercised on this host because only one endpoint exists) | done |
| wifi (WLAN API) | **yes** | WlanOpenHandle + WlanEnumInterfaces succeed (adapter present) | probe done; SSID/connect in milestone 2 |
| bluetooth (WinRT radio) | **yes** | Radio::GetRadiosAsync returns a Bluetooth radio | probe done; device list/toggle in milestone 2 |
| brightness | **no** | no monitor exposes user-level brightness control (external desktop monitor; Dxva2 path) | honest disabled state; per-monitor support detection implemented |
| local_notifications (store) | **yes** | SQLite-backed local store (add/list/read/dismiss/clear) | done |
| toast_notifications (Windows toasts) | **no** | error 0x80073D54: process has no package identity (unpackaged dev binary) | best-effort API implemented; see limitation below |
| airplane_mode / energy_saver / hotspot / cast / night_light | no (not implemented) | explicit unsupported reasons returned to UI | planned later / may remain unsupported honestly |

## API surface (milestone 1)
`cargo
ps_windows::native_capabilities() -> NativeCapabilities   // per-feature supported + reason
ps_windows::list_audio_outputs() -> Result<Vec<AudioOutputDevice>>
ps_windows::default_audio_output() -> Result<AudioOutputDevice>
ps_windows::set_audio_output(device_id: &str) -> Result<()>
ps_windows::get_audio_volume() / set_audio_volume(f32) / get_audio_mute() / set_audio_mute(bool) / audio_state()
ps_windows::brightness_supported() -> bool; brightness_monitors() -> Vec<String>; get_brightness(); set_brightness(u8)
ps_windows::toast_supported() -> bool; show_toast(title, body) -> Result<()>
ps_sources::notifications::{add, list, unread_count, mark_read, dismiss, clear, add_and_maybe_toast}
`

## Honest limitations / follow-ups
- **Windows toasts need app identity.** An unpackaged binary (current dev build) cannot create a toast notifier (0x80073D54). Fix path: register a Start Menu shortcut carrying an AppUserModelID (IShellLink + PKEY_AppUserModel_ID) or a package identity at install time, then toast_notifications becomes supported. The local store always works and remains the source of truth.
- **Brightness is per-monitor and hardware dependent.** Dxva2 works for internal laptop panels that expose brightness; this desktop monitor does not. The capability reason is shown to the UI instead of a fake slider. DDC/CI fallback for external monitors is optional future work.
- **Output switching uses IPolicyConfig**, a stable but undocumented Windows interface (the same one the OS sound UI uses). If a future build removes it the capability probe reports unsupported and the UI should open Windows sound settings.
- No admin privileges are requested anywhere; all probes are bounded (3 s) and side-effect free.
- Media/Wi-Fi/Bluetooth adapters currently expose **capability probes only**; enumeration/transport/connect operations are milestone 2 work.

## Test evidence
- `cargo test -p ps-windows`: 7 unit + 15 integration = 22 passed / 0 failed, including native capability probe (bounded + serde round-trip), audio device enumeration, volume/mute restore round-trip, brightness round-trip guarded by capability, toast honesty.
- `cargo test -p ps-sources`: 18 unit + 3 notifications tests + 31/32 sources tests passed / 0 failed (1 ignored action smoke), including store round-trip, read/dismiss/clear counts, input validation/truncation, toast fallback honesty.
- Host probe command: `cargo run -p ps-windows --example native_probe` (raw output in @native-probe-host.log).