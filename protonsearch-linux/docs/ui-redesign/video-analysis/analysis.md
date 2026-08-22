# Proton Search concept-launcher analysis

This document records the supplied frame-analysis report for the 103-second,
30 FPS concept clip (3,090 frames). The source report identified the following
distinct interaction states and transitions. The clip itself was not included
in the repository, so this is the authoritative analysis available to the
implementation branch; screenshot comparison will use locally captured Linux
states as they are implemented.

## Timeline and state inventory

| Time | State | Implementation implication |
| --- | --- | --- |
| 0–2s | Dashboard home | Empty search should be useful: time/date plus real Linux status cards and power/media actions. |
| 2–16s | Search with live contextual preview | Normal search becomes a master/detail surface: results on the left, selected-item preview on the right. |
| 10–15s | Source-code preview | Code previews need bounded reads, readable typography, and syntax-aware coloring where available. |
| 17–20s | Clipboard master/detail | Clipboard selection uses the same preview surface, including image/content rendering and deletion. |
| 20–23s | Suggestions | Suggestions remain keyboard navigable and use the same result-selection model. |
| 27–39s | Command workspace and speed-test tool | Commands should open readable embedded workspaces instead of exposing raw output. |
| 40–43s | Todo, Pomodoro, and system-information workspaces | These are mode changes within the launcher, not unrelated windows. |
| 47–56s | Application/process detail | Applications and processes use the same list/detail relationship and Linux metadata. |
| 60–90s | Agent conversation | Agent stays inside Proton Search with thinking, generation, stop, history, and error states. |
| 72–74s | Agent file attachment | `@file` opens an inline, keyboard-navigable file picker only when the Hermes path can safely accept context. |
| 92–103s | Adaptive local-file previews | Images, folders, text, and other files receive type-specific contextual previews. |

## Design principles extracted

1. One continuous application surface with intentional component surfaces only.
2. One search input drives universal search, command mode, and Agent entry.
3. Selection is singular for ordinary navigation and updates detail immediately.
4. The detail surface is lazy and bounded; it must not decode or read every result.
5. Narrow windows degrade to a list-first layout; wide windows use master/detail.
6. Contextual detail preview and configured Alt quick preview are separate flows.
7. Modes change contextual footer hints and preserve keyboard-first operation.
8. Dashboard values are real Linux data and refresh on demand or at a low cadence.
9. Dark and light themes use the same semantic tokens, not inverted ad-hoc colors.

## First milestone scope

This branch first delivers the shell that enables the rest of the concept:

- modular dashboard home with real local system data;
- responsive master/detail search surface using existing providers and preview
  loading;
- unified selection and contextual footer behavior;
- preserved filters, Alt quick preview, Agent entry, and existing actions;
- release-build and screenshot verification on the current Arch/Wayland host.

Command mini-apps, Agent attachments, full syntax highlighting, and cross-distro
testing remain follow-up milestones unless the existing Linux implementation
already provides them without destabilizing the shell.

## Linux-specific translation

macOS-only cards are mapped to Linux capabilities: `/sys/class/power_supply`
and UPower for battery, `/proc` and filesystem statistics for resource cards,
NetworkManager/Bluetooth tooling where available, XDG paths for files, desktop
entries for applications, and the installed Hermes gateway for Agent work.
Unavailable optional providers must present a useful unavailable state instead
of fake data or a raw JSON dump.
