# Native capability audit

| Service | Existing implementation | Verified limit |
|---|---|---|
| Audio | Core Audio volume/mute/output | Hardware changes and roundtrips untested |
| Wi-Fi | WLAN saved-profile connect, cached networks, radio worker | No scan/disconnect/full credential flow verified |
| Bluetooth | Radio and paired devices; OS management fallback | Full profile connect/pairing not implemented |
| Brightness | Physical-monitor DDC/DXVA2 | Internal-panel WMI missing; setter may touch multiple monitors |
| Media | Hardcoded unsupported | No live session adapter |
| Notifications | Local application events | Universal Windows notification access not claimed |
| Power/hotspot/cast/night light | Capability records/settings URI allowlist | Requested native controls incomplete |
