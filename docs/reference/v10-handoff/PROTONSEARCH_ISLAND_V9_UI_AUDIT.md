# ProtonSearch Dynamic Island v9 — UI audit & handoff

**Baseline:** `protonsearch_dynamic_island_cards_v8.html` (provided in the conversation).  
**Revised prototype:** `protonsearch_dynamic_island_cards_v9_polished.html`.

## Preserve

- V8's hidden-by-default capsule, Alt+Space demonstration, scroll/swipe card switching, spring-like width/height transitions, dark compact visual language, all eight cards and their existing simulated interactions.
- The existing *normal launcher* is a separate presentation and has **not** been redesigned or replaced in this version.

## Changes

1. **Direct card selection:** Eight subtle pagination dots. They have meaningful accessible names, a distinct active indicator, and click-to-jump behavior. In compact Search they appear at the right edge of the pill; when expanded they occupy a dedicated 31px footer. Existing scrolling and swiping remain.
2. **Uniform SVG system:** Bounded inline SVGs, standardized stroke and transport icons, distinct text/code/image/link/file clipboard glyphs, proper headphone/speaker/monitor output glyphs, reduced-motion support.
3. **Media:** Consistent alignment of shuffle, previous, pause/play, next, repeat and audio output. Output button has a small down-chevron. Audio selection displays correct device icons.
4. **Quick Settings:** Wi-Fi/Bluetooth tile is divided into independent, semantically correct buttons: the left side toggles connectivity; the right chevron opens nearby connections. Larger, consistently aligned targets. Brightness and volume controls align to a shared grid.
5. **Clipboard:** Fixed large-icon sizing, code-snippet word wrapping, stable row heights and no partially visible row; narrow-screen category controls wrap onto multiple lines.
6. **Agents:** Consistent icon sizing, complete scrollable subagent list (newly spawned subagents do not disappear), visible status and aligned action controls.
7. **Browser, Git, Notifications:** Consistent leading icons, truncation/metadata alignment, dedicated notification-dismiss targets.
8. **Responsive:** Checked at 1150px, 430px and 335px viewports. The smallest viewport uses a shorter search placeholder and a single-column Quick Settings grid. Informational text wraps instead of overflowing.
9. **Rendering:** A versioned card-render token prevents older delayed transitions from overwriting a newer selected card.

## Verified in local Chromium

- All eight dots render, are clickable, and update `aria-current="page"`.
- Search query and filters work with simulated results.
- Media play/pause and output selection work in simulation.
- Clipboard Code category renders contained syntax-like text with no oversized graphic.
- Wi-Fi and Bluetooth chevrons each open their respective connection list without toggling the radio.
- Agent spawning adds visible rows; notifications render and dismiss.
- No JavaScript page errors during a desktop and narrow-screen interaction pass.
- SVG icons in inspected cards remain within their specified component sizes.

## Integration requirements for coding agent

This is an **interactive design prototype**, not a connected Windows application. Replace sample data and event handlers with the application's Rust/Tauri APIs, Windows media-session, device, and clipboard services, and the optional agent backend. Avoid treating browser-only Alt+Space handling as proof of a global Windows hotkey. Preserve the dedicated tool permissions and prompt for confirmation before destructive actions.

The component visual system and card interactions are references. Build a reusable React/TypeScript component system rather than embedding the prototype's monolithic JavaScript in production unchanged. Maintain one source registry and one Rust engine for Dynamic Island and Normal Launcher. Do not restore V5–V7's navigation rail or radial wheel.
