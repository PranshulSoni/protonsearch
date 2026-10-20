# Interactive Proton dock widget design

Approved direction: adaptive interactive widgets, 20 October 2026. The user approved
the proposed visual and interaction design with “Yes plz”. This document records that
approved scope; it does not add a further approval requirement to the authorized rebuild.

## Experience

Keep one floating glass shell. Use a coordinated set of original SVG icons with 1.7–2px
strokes, optical sizing, clear typography, neutral silver/charcoal glass and restrained
blue highlights. Remove Unicode pictograms and mixed-quality embedded service artwork.
Widgets have common height, independently adjustable widths, and compact/rich content
breakpoints. Subtle separators and quiet backing preserve a continuous dock appearance.

The default Daily composition emphasizes Search, apps, weather, music, focus, system
controls and clock; Agents remains accessible as a widget. Founder presents a second
composition with useful analytics. All existing widget types remain in the gallery.

## Demonstrated interactions

- Search filters local sample files/apps and opens a selected result preview.
- App drawer filters sample apps; activating one opens a window preview and updates
  running indicators. No actual OS process launches.
- Weather opens a forecast panel with hourly/daily switching and Celsius/Fahrenheit.
- Music supports play/pause, previous/next, seek and visibly advancing playback.
- Quick controls support volume, mute, audio output and simulated Wi-Fi network choice.
- Focus supports start/pause/reset and a useful timer display inside the dock.
- Agents accept a sample task, show staged progress, support cancellation and show a
  result. Clearly label execution as a demo; no model or filesystem action occurs.
- Clock opens a calendar. Additional gallery widgets show meaningful detail panels
  rather than dead surfaces. Custom metrics can be inspected/edited as plain text.

## Panels and state

Use one reusable widget-detail surface anchored above the initiating widget and clamped
to the viewport. Clicking the same trigger, Escape or outside closes it and restores
focus. Opening another replaces its contents. Profile switches and editing close it.
Actual sliders/buttons work inside tiles without also opening details or panning the dock.
Keep browser-only playback/focus/agent state separate from persisted layout state.
Respect reduced motion, keyboard control, narrow viewports and selected-widget reveal.

## Boundaries

`widget-library.js` defines icons and widget markup before `concept.js` initializes.
`concept.js` continues to own layout, profiles, gallery and persistence.
`widget-runtime.js` owns sample product interactions and the reusable panel.
`widget-polish.css` owns the new appearance. `glass-dock.html` loads these in order.
Use DOM textContent or escaped text for user input, finite bounded slider values,
cleared timer handles and guarded localStorage access. No external network resources,
accounts, native controls, arbitrary widget code or production changes.

## Validation

Inspect actual dock and detail panels at desktop and 320/375px widths. Verify search,
music/seek, weather units/tab, sound controls, app previews, focus and agent cancellation
through UI. Verify gallery, resize, reorder, profile persistence, panel focus restoration,
outside/Escape close, independent profile state and no page-level overflow. Run JS syntax,
local-link/duplicate-ID and diff checks, then independent code review. Save a preview image.
