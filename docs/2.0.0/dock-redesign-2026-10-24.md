# Native dock design reset — 24 October 2026

## Direction and evidence

Build one quiet glass dock with a consistent row of full-size application icons and compact interactive widgets. All items share one height and baseline; widgets may occupy multiple horizontal slots to expose useful information. The dock is a launcher and taskbar first. Expanded controls belong in an anchored popover rather than permanent dashboard cards.

The user's screenshot exposes four problems in the current native implementation: long generic labels consume the dock, every data item has a permanent card background, application artwork is inconsistent and blurred, and the row clips before essential controls. `DockWindow` currently selects any non-app item wider than 85 units, while model defaults give Search 136, Weather 184, and Focus/Clock 124 units. Reducing padding alone will not fix that hierarchy.

Official references, checked 24 October 2026:

- [Apple: Use the Dock on Mac](https://support.apple.com/guide/mac-help/open-apps-from-the-dock-mh35859/mac): direct app access, rearrangement, running apps, and context actions. Borrow the clear separation between artwork and running indicators.
- [Apple: Desktop & Dock settings](https://support.apple.com/en-ie/guide/mac-help/mchlp1119/27/mac/27): adjustable size and hover magnification. Keep magnification restrained to avoid disturbing adjacent controls.
- [Fedora Workstation](https://fedoraproject.org/workstation/): identifies GNOME as its desktop. [GNOME's official visual overview](https://help.gnome.org/gnome-help/shell-introduction.html) describes a dash of favorites/running apps, small running dots, a grid launcher, and window context actions. Its dash belongs to Activities overview; Proton deliberately keeps its own dock available on the desktop.

These references inform interaction and proportion. The precise dimensions below are Proton design decisions, not dimensions asserted by those projects.

## Alternatives and decision

1. **Uniform icon dock:** simplest and most compact, but loses the user's defining requirement for live information inside the dock.
2. **One-height modular dock — selected:** icons use one slot, useful data uses two slots, and deliberately expanded widgets may use three. Preserves the dock silhouette while supporting live widgets.
3. **Mixed-height dashboard:** exposes more content at rest, but repeats the clutter and large-card problem the user rejected.

## Geometry contract

All measurements are logical pixels. Convert to physical pixels exactly once at the native window boundary using the monitor DPI.

| Token | Default | Rule |
| --- | ---: | --- |
| Dock window height | 82 | Includes safe drawing inset; do not grow on hover |
| Shared content height | 60 | Every icon and widget uses this height |
| Item gap | 4 | One shared horizontal rhythm |
| Slot width | 60 | Icon actions and applications |
| Compact widget width | 124 | Two slots plus their gap |
| Expanded widget width | 188 | Three slots plus their gaps |
| Outer horizontal inset | 12 | Match both ends |
| Content top inset | 11 | Vertically centers 60 in 82 |
| App artwork | 48 | Hover maximum 54, fitted proportionally |
| System glyph | 24 | Vector, common optical weight |
| Dock radius | 22 | One outer silhouette |
| Hover radius | 14 | No permanent boxed surface for every item |
| Minimum screen edge clearance | 12 | Respect monitor bounds and reserved space |

Compute required width from the actual rendered item widths, gaps, and fixed customize/overflow controls. Never use a separate approximation that drifts from Slint's layout. Limit the entire dock to the monitor width minus 24 logical pixels. App icons have the same slot width as other icon actions; internal transparent artwork padding should not change layout.

Default row: Start, Search, pinned/running applications, a subtle separator, Weather, Focus, Audio, Clock, Agents, Customize. Start/Search/Agents/Customize are icon actions. Weather, Focus, and Clock are compact widgets. Audio defaults to an icon with an accessible current-value label and expands on click. Running applications appear once even if pinned and open.

Equal sizing means equal height, baseline, and icon rhythm. It does not force weather and clock into unreadable 60-pixel squares. Profile customization can choose compact or expanded widths, with the same height throughout.

## Glass and typography

Use a neutral charcoal glass surface, subtle white edge, and restrained shadow. No green tint, colored outline around the entire dock, or thick focus ring when merely hovered. A starting fallback gradient is `#242830D9` to `#171A20E8`; translucent paint is not a claim of real backdrop blur. Prefer a verified DWM backdrop when the renderer supports it; otherwise retain a readable neutral fallback. Never simulate glass by repeatedly capturing the desktop.

Use Segoe UI Variable for native consistency: 15-pixel semibold data, 10–11-pixel muted supporting text, 12-pixel tooltips. Clock may use 20-pixel tabular-looking digits. White primary text, softened secondary text, subtle cool focus accent. Keyboard focus has a clear 1–2-pixel inner outline; normal hover only adds a low-opacity fill. Remove repeated captions such as “Sound”, “Focus timer”, and “Local time” when icon and value already communicate the meaning.

## Widget presentation and behavior

| Item | Resting preview | Click / expanded state |
| --- | --- | --- |
| Applications | Individual 48-pixel app artwork, small running dot | Launch/activate; multiple windows use anchored chooser; context actions pin/unpin/close |
| Start | Consistent grid glyph | App drawer with searchable icon grid, stable selection |
| Search | Magnifier glyph | Existing Proton search window, intentionally positioned and focused |
| Agents | Consistent single glyph; small count only for actual work | Existing agent interface; do not invent simulated progress |
| Weather | Condition glyph, temperature, short city | Location setup, forecast when configured; unavailable shows a compact dash rather than fake temperature |
| Focus | Small timer glyph, 25:00, quiet progress line while running | Start/pause/reset; retain accurate monotonic countdown |
| Audio | Speaker glyph with optional compact percentage | Immediate slider and mute action; value reflects Windows state |
| Clock | Time with optional short date | Compact date/calendar surface or clearly labeled system date settings action |
| Network | Signal glyph; disconnected indication | Connection name and real status, Windows network settings action |
| Power | Battery glyph with charge indication | Percentage, charging, remaining estimate only if Windows provides it |
| Music | Small artwork only if genuine media artwork exists, short title | Actual playback actions; unavailable metadata remains honest |
| Custom metric | Explicit label and value, one consistent hierarchy | Edit label/value/detail without pretending to execute arbitrary plugins |
| System tray | Honest tray/system action glyph | Only advertise tray hosting once actual hosting exists |

Every widget supports ready, hovered, focused, pressed, expanded, loading, unavailable, and error states where applicable. Missing data preserves the slot instead of resizing the dock. Errors remain local and recoverable. No indefinite full-row shimmer or decorative animation.

## Popovers and motion

Only one dock-owned popover is visible at once. Clicking its owner again closes it. Escape and outside activation close it; interacting with its own controls must not dismiss it prematurely. Position above the owning item, offset 8 logical pixels from the dock, and clamp to the monitor work area. Fall back below only when above does not fit. Never show a visible window at the default origin before calculating final placement.

Use content-specific panel dimensions: sound about 300 by 180, focus 300 by 260, weather 340 by 340, and app/window chooser approximately 380 by 420. Long lists scroll internally. Generic 440 by 560 panels are reserved for the app drawer or editor, not every widget.

Hover: 120 milliseconds ease-out, up to 4-pixel app lift and 48-to-54 artwork magnification. Press: 80 milliseconds, subtle artwork reduction. Popover: 140 milliseconds opacity and up to 6-pixel translation. Animate content within the already positioned native window, not the HWND geometry every frame. No width animation that displaces neighbors or changes the reserved work area. Honor the Windows animation preference with immediate state changes when disabled.

## Overflow and performance

At narrow widths, keep Start, Search, Customize, and a visible overflow button reachable. Preserve pinned app ordering. Compact optional widget previews first, then place excess widgets/apps into an explicit overflow surface. A horizontal scroller is an acceptable interim implementation only if its scroll affordance is visible and keyboard users can reach off-screen items. Clipping with no indication is unacceptable.

Create, decode, and cache artwork outside pointer/animation paths. Request app artwork at the highest required physical size: `ceil(54 * dpi / 96)` minimum, with a larger native source preferred. Never repeatedly upscale a small window icon when Shell or executable artwork exists. Cache by app identity, source, and requested pixel size; bound memory and invalidate when sources change. Keep Windows providers off the Slint UI thread. Update only changed models; clock/focus need one-second updates, not full native reconstruction each animation frame. Change AppBar/window geometry only when layout, monitor, or DPI changes.

Performance targets are acceptance goals requiring measurement: no blocking provider or icon extraction on hover; no periodic layout jumps; first feedback on click within one rendered frame when the UI thread is idle; animations should fit the display frame budget. Do not claim zero lag without recording frame timing on the target machine.

## Native integration and acceptance

The Windows taskbar visibility toggle belongs in dock customization with a clear current state. Hiding Explorer's taskbar must be reversible on toggle, orderly exit, startup failure, and crash recovery; never terminate Explorer. Keep taskbar visibility management separate from the AppBar reservation so these two mechanisms cannot repeatedly resize each other.

Validate at 100%, 150%, and 200% DPI, mixed-DPI monitors, 1366×768 and 1920×1080, long app names, missing icons, no battery, disconnected network, fullscreen applications, Explorer restart, and repeated panel opens. Check each window starts at its final anchor without flashes. All dock items must remain accessible when overflowed. Native screenshots, rather than only HTML, are the review artifact for this work.

Implementation ownership remains split: native Slint presentation, Windows placement/taskbar recovery, runtime behavior/anchoring, and final integration/build verification. This document defines the common layout and behavior contract; it does not assert those implementation checks have already passed.
