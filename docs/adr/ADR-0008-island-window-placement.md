# ADR-0008: Dynamic Island native window and placement contract

- Status: **Accepted** (G1 post-review L0 decision, 2026-11-10)
- Owners: L0 (decision), L1-B (UI/spec), L1-A (shell), L1-E (review)
- References: V9.4 audit and binding V9 HTML; docs/design/ISLAND_V9_SPEC.md; ISLAND_SETTINGS_SPEC.md

## Context

The V9 island is a compact 520x66 surface with a layered shadow, nine anchor positions, per-monitor
placement and an expanded-card state that must not be clipped. It must behave like a real Windows
window: summonable by the global hotkey, topmost over normal apps, absent from the taskbar/Alt-Tab,
never off-screen or unreachable, and reapplied safely after settings changes. The G1 review found
keepInWorkArea=false could disable clamping, offsets were unbounded and island hide had no race guard.

## Decision

1. The native Tauri window is the V9 surface plus a transparent shadow pad of 96 logical px on each
   side (ISLAND_DEFAULT_WIDTH 520, HEIGHT 66, ISLAND_SHADOW_PAD 96; main.rs:28-33, 503-531). The
   WebView surface is exactly the card; the padding exists so the shadow is not clipped.
2. Placement: monitor strategy (primary/cursor/foreground/specific), nine anchors expressed as
   fractions of the monitor work area, offsets in DIPs scaled by the monitor scale factor, and an
   expanded-bounds policy (clamp or flip-then-clamp). The surface is ALWAYS clamped into the monitor
   (work area when keepInWorkArea=true, monitor bounds otherwise), and the surface size is clamped
   240-900 x 40-760 and additionally bounded by the monitor; an island can never be fully off-screen
   (main.rs:196-261, 1253-1261).
3. Offsets are validated server-side to +/-2000 before persistence (main.rs:605-635); invalid card
   orders are rejected with the other settings validation.
4. Visibility/lifecycle: hidden by default, focused(false), transparent, shadow(false), topmost,
   skip-taskbar, non-resizable; show/hide play the V9 motion driven by the UI; show increments a
   generation token and a pending hide only runs if the generation is unchanged, so re-summon can
   never be hidden mid-open (main.rs:269-320, 503-531).
5. hide_on_focus_loss is a settings-driven behavior; the hidden state emits island:hidden for the UI.
6. Monitor identity uses the display device name (e.g. \\.\DISPLAY1) plus a primary fallback. Hotplug
   healing (monitor removed/reordered, DPI change) is a documented follow-up: on show/settings apply
   the island falls back safely, but persisted monitor ids are not yet healed or rewritten.
7. The island surface size is reported by the UI through set_island_geometry, which clamps again and
   re-places the window without stealing focus.

## Consequences

- The island cannot trap the user off-screen under any settings combination.
- The shadow/motion contract is native-window based; V9 HTML interaction is still not evidence for
  Windows behavior and must be validated with real screenshots/video and frame pacing at G5/G8.
- Multi-monitor behavior is designed but only single-monitor evidence exists on this host; dual/
  mixed-DPI/disconnect tests remain blocked (V10_GATE0_AUDIT blockers) until hardware is available.
- Follow-up required before G6 acceptance: hotplug healing + explicit tests for monitor removal and
  DPI changes; do not claim these are solved.

## Alternatives considered

- Hosting the island in the launcher window (rejected: different geometry/lifecycle, animation
  conflicts).
- Electron-style always-on-top window with no clamp (rejected: unreachable risk).
- WebView-only CSS positioning inside a full-screen transparent window (rejected: taskbar/Alt-Tab/
  input and multi-monitor behavior problems).

## Verification

- Native constants/builders: main.rs:28-33, 196-320, 503-531, 1253-1261.
- Validation: main.rs:605-635.
- Settings/UI spec: docs/design/ISLAND_V9_SPEC.md, ISLAND_SETTINGS_SPEC.md.
- G5: real Windows screenshots/video for all 8 cards and states at 100/125/150/200% scaling,
  frame-time measurements, no-clipping checks; G6: placement editor + restart persistence tests.
