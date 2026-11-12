# Native adapter support matrix — task-55 (milestone 2: Wi-Fi, Bluetooth, brightness, settings fallbacks)

| Field | Value |
|---|---|
| Status | **Implemented + verified on this host**; API ready for shell commands and ui-launcher cards |
| Date | 2026-11-10 (Asia/Calcutta) |
| Code | crates/ps-windows: imp/{wifi,bluetooth,radios,brightness,capabilities,settings_pages}.rs, src/lib.rs, examples/native_probe.rs, tests/windows_smoke.rs |
| Evidence | @native-probe-host.log (live capability + device data), @native-m2-tests.log (test-run evidence) |
| Constraints honoured | user-level (no admin), bounded calls, no OS-consent bypass, honest typed failures, no profile/key creation |

## What shipped
`text
Wi-Fi      ps_windows::wifi_radio_state() -> RadioStateInfo{available,enabled}
           ps_windows::set_wifi_radio(bool)
           ps_windows::list_wifi_networks() -> Vec<WifiNetworkInfo{ssid, signal(0-100), security,
               connected, has_profile, connectable}>
           ps_windows::connect_wifi(ssid) -> WifiConnectOutcome{ssid, connected, message}
           ps_windows::wlan_adapters() -> Vec<String>
Bluetooth ps_windows::bluetooth_radio_state() -> RadioStateInfo
           ps_windows::set_bluetooth_radio(bool)
           ps_windows::list_bluetooth_devices() -> Vec<BluetoothDeviceInfo{id,name,connected,paired}>
Brightness ps_windows::list_brightness_monitors() -> Vec<BrightnessMonitorInfo{index,name,current,
               minimum,maximum, method: dxva2|ddc_ci}>   (Dxva2 first, then DDC/CI VCP 0x10 fallback)
           ps_windows::brightness_supported() / brightness_monitors() / get_brightness() / set_brightness(0-100)
Settings  ps_windows::SettingsPage enum (wifi, bluetooth, display, sound, notifications,
               airplane_mode, mobile_hotspot, night_light, energy_saver, project,
               windows_update, privacy_location) with .uri() (ms-settings:*) and .label()
           ps_windows::open_settings_page(SettingsPage) -> Result<()>  (allow-listed URIs only)
Snapshot  ps_windows::controls_status() -> ControlsStatus{capabilities, wifi: Option<WifiState>,
               bluetooth: Option<BluetoothState>, brightness: Option<BrightnessState>}
           WifiState{radio, networks, error: Option<String>}
           BluetoothState{radio, devices, error: Option<String>}
           BrightnessState{supported, monitors}
Capability ps_windows::native_capabilities() -> NativeCapabilities (per feature: supported,
               reason, fallback_uri). fallback_uri is set for OS-owned features even when
               supported so the UI can offer an 'Open Windows settings' secondary action.
`

## Host probe results (real, this machine)

| Feature | Result | Evidence |
|---|---|---|
| Wi-Fi radio | available=true, enabled=true; adapter `MediaTek Wi-Fi 7 MT7925 Wireless LAN Card` | probe log |
| Wi-Fi network list | Windows **denied scan access** (location permission) -> honest error naming Settings > Privacy & security > Location + ms-settings:network-wifi fallback; UI can open @PrivacyLocation | probe log |
| Wi-Fi radio toggle | action reached Windows but returned RadioAccessStatus::DeniedByUser -> honest error + 'open Windows settings'; we deliberately do NOT bypass OS consent | test log |
| Wi-Fi connect-if-profile | missing-profile path verified: honest error, no profile/key creation; real saved-profile connect UNVERIFIED on this host | test log |
| Bluetooth radio | available=true, enabled=false (user has it off); toggle returned DeniedByUser -> honest error + ms-settings:bluetooth fallback | probe + test logs |
| Bluetooth paired devices | 2 devices enumerated with name/id/connected (Mivi SuperPods Immersio, Pranshul's iPhone) | probe log |
| Brightness | Dxva2 + DDC/CI implemented; this external monitor exposes neither -> supported=false with reason + ms-settings:display fallback | probe log |
| Settings fallbacks | wifi/bluetooth/display/sound/notifications/airplane/hotspot/night light/battery saver/project/windows update/location -> ms-settings:* | test `settings_fallbacks_are_allow_listed` |
| Controls snapshot | `controls_status()` bounded ~1.5 s, serialisable, carries per-section error strings | test log |

## Architecture note (important)
Repeated `Radio.GetRadiosAsync()` calls from fresh short-lived MTA threads crashed natively on Windows (access violation after the first enumeration). All WinRT radio/Bluetooth work now runs on ONE persistent MTA worker thread (`ps-winrt-radio`) with bounded request timeouts; results are plain data. This is why the adapter is stable across repeated probes/UI refreshes. Wi-Fi network lists use WlanAPI (WlanOpenHandle/WlanEnumInterfaces/WlanGetAvailableNetworkList/WlanGetProfileList/WlanConnect) with WlanFreeMemory. No admin rights are requested anywhere.

## Honest behaviour requirements for the UI (agreed semantics)
1. Never render a dead toggle: every unsupported capability carries `reason` + `fallback_uri`; render an 'Open Windows settings' button via `open_settings_page`.
2. Wi-Fi card: use `ControlsStatus.wifi.radio` for the radio toggle, `networks` for the list; when `wifi.error` is present show that text + a button to `SettingsPage::PrivacyLocation` (for the location-permission case) or `Wifi`.
3. Radio setter errors are trustworthy: surface the message (it already names the OS reason) and offer the fallback page instead of retrying silently.
4. connect_wifi only uses saved profiles; on error show message and open `SettingsPage::Wifi`. Never ask the user for a Wi-Fi key in ProtonSearch.
5. Brightness slider: bind to `list_brightness_monitors()[0].current`/`set_brightness`; if `brightness.supported==false` hide the slider and show fallback button.

## Suggested shell/UI mapping
`text
shell command                  -> ps-windows call
controls.status                -> controls_status()
wifi.radio.set(enabled)        -> set_wifi_radio(enabled)
wifi.networks.list             -> list_wifi_networks()
wifi.connect(ssid)             -> connect_wifi(ssid)
bluetooth.radio.set(enabled)   -> set_bluetooth_radio(enabled)
bluetooth.devices.list         -> list_bluetooth_devices()
brightness.list                -> list_brightness_monitors()
brightness.set(percent)        -> set_brightness(percent)
settings.open(page)            -> open_settings_page(SettingsPage)
capabilities.get               -> native_capabilities()
`

## Tests / verification
- `cargo test -p ps-windows`: 7 unit + 20 integration = **27 passed / 0 failed**, incl. capability probe boundedness/serde, settings allow-list, Wi-Fi honest-error paths, Bluetooth radio/device read paths, brightness method/fallback, controls snapshot bounded+serialisable.
- `cargo check -p ps-windows -p ps-sources -p ps-content -p ps-ocr --all-targets` clean.
- Host evidence: @native-probe-host.log, @native-m2-tests.log.

## Remaining honest limitations
- Wi-Fi network list requires Windows **location permission** for desktop apps on Win11; without it Windows returns access-denied and we surface that (no fake list). After the user enables it, the same API returns SSIDs. This is a Windows privacy requirement, not a ProtonSearch bug.
- OS radio consent can be denied per app (RadioAccessStatus::DeniedByUser); we honour it and route to Windows settings rather than bypassing it.
- Real connect to a saved profile and radio toggles that change state are not exercised in automated tests (would alter the user's connectivity); they are implemented and error-checked.
- Brightness on this host's external monitor exposes neither Dxva2 nor DDC/CI; the slider stays hidden with an honest reason + fallback.
- Media session transport controls are out of scope for task-55 (separate work item) and report supported=false with a reason.