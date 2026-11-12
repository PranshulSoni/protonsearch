# ADR-0012: Capability-gated native Windows adapters

- Status: **Accepted** (G1 post-review L0 decision, 2026-11-10)
- Owners: L0 (decision), L1-A (Win32/IPC), L2-A (adapters), L1-C/L1-E (perf/security review)
- References: master prompt sections on native integrations; task-35 (G7B)

## Context

The eight-card island needs real Windows data: media sessions/transport, audio output devices,
Wi-Fi, Bluetooth, brightness, volume, notifications and system state. Windows exposes these through
different API families (WinRT, COM, Core Audio, WLAN/Bluetooth APIs, power APIs) with varying
permissions, hardware support and failure modes. The product must never pretend an unsupported
control exists, must not silently change unrelated OS preferences, and must remain local-first.

## Decision

1. Capability-gated by design: every adapter produces a capability snapshot (supported/unsupported/
   permission-required/unavailable) in ps-ipc, and the UI renders disabled/redirect states honestly
   instead of fake toggles.
2. Milestone 1 (task-35 first slice): audio output endpoints + volume/mute, display brightness, and
   local/allowed notifications. Milestone 2: media sessions/transport (GSMTC), Wi-Fi, Bluetooth.
   Features whose APIs are not feasible with user-level rights are documented as gaps and may be CUT
   with L0 approval; never shipped as simulated success.
3. Implementation rules: allowlisted Win32/WinRT/COM APIs only, no shell strings, no powershell/cmd
   for OS actions; bounded calls with timeouts; all handles released safely; no silent preference
   changes outside the named control; no elevation requirement for baseline features.
4. IPC: capability/state DTOs live in ps-ipc; commands are request/response and return structured
   errors; device actions never block the WebView and are rate-limited/debounced.
5. Security/privacy: no network calls to third-party services for device state; notification access
   requires user consent; Bluetooth/Wi-Fi scans are bounded and do not persist SSIDs beyond the UI
   cache; the existing detect_agents PATH probing is bounded (2.5 s, 64 KiB, metachar-guarded) and
   documented as a residual same-user surface.
6. Test matrix: supported/unsupported hardware, permission-denied, device-disconnected, rapid
   toggles, sleep/resume, dual-action independence (e.g. Wi-Fi toggle vs chevron list), honesty of
   disabled controls; real Windows evidence required (no HTML simulation).

## Consequences

- The island can ship Quick Settings with honest partial coverage while the team expands adapters.
- Some controls will be redirect-to-Windows-settings rather than native actions; that is acceptable
  and must be labeled.
- Adapter work is the highest hardware-dependency area; single-monitor/no-Bluetooth variations on the
  host must be declared as test gaps rather than assumed.

## Alternatives considered

- One-shot all-device support (rejected: too broad, high dishonesty risk).
- Scripting OS settings via PowerShell (rejected: injection surface, fragile, slow).
- Third-party device libraries (rejected for v10; review licences/security if revisited).

## Verification

- task-35 acceptance: capability matrix on this host, real screenshots/E2E, honesty states, error
  paths, no shell usage, security review by L1-E, perf sanity by L1-C.
- G7B gate: media/audio/Wi-Fi/BT/brightness/volume/notifications rows in FEATURE_TEST_MATRIX pass or
  are explicitly CUT/deferred with L0 approval.
