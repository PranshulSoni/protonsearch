# Proton dock — revised glass design plan

Status: proposed visual direction, planning only. Prepared 20 October 2026.
Branch: `2.0.0`.

This replaces the visual direction of the green HTML concept. It supersedes the old
uniform tile treatment, persistent labels, palette and default composition. Existing
search, recovery, permissions, profile and Windows compatibility requirements remain.
The user authorized the rebuild. The interactive HTML study is now in glass-dock.html
and embedded in the overview. The study uses original illustrative app artwork.

## 1. Design intent

A restrained floating glass dock that feels at home on a modern desktop. The wallpaper
remains visible. App icons are recognizable. Live information earns its space. System
status is legible without dominating the dock.

The reference is the macOS Dock's restraint and hierarchy, not a literal copy of its
artwork or APIs. Proton adds resizable information widgets, search and agents while
keeping a calm default composition.

The first concept failed because each feature was put in a similarly heavy tile:
large outlines, permanent labels, unrelated Unicode glyphs, repeated little panels,
an oversized weather block and too many competing details. A green background with
some opacity is not the intended glass material. Changing the color alone cannot fix
the composition.

## 2. Composition options

| Direction | Benefit | Cost | Decision |
| --- | --- | --- | --- |
| Main glass dock + adjacent status capsule | Separates apps/live content from quiet system controls; creates breathing room | Two visible surfaces must feel related and remain keyboard-connected | Recommended default |
| One continuous glass dock | Most compact arrangement; straightforward alignment | Needs disciplined spacing so it does not become another crowded strip | Offer as an optional joined layout |
| Separate capsule for every widget | Strong individual module boundaries | Repeated glass edges, shadows and gaps make a cluttered row | Reject as default |

There should be two visible glass surfaces at most in the default resting state.
An expanded panel is temporary. Users can customize grouping; this is an initial
composition, not a restriction on which widgets they may install.

## 3. Recommended resting layout

```text
                    MAIN DOCK                                  STATUS
╭────────────────────────────────────────────────────────╮   ╭──────────────────╮
│ Start Search Agents  │  App App App App  │  ☀ 26°       │   │ Wi-Fi Sound      │
│                                          Pune         │   │ Battery   09:41  │
╰────────────────────────────────────────────────────────╯   ╰──────────────────╯
```

This is a semantic wireframe: words identify controls, not labels to display in the
finished dock. The status controls will occupy one horizontal row, not the two lines
used above to keep the diagram readable. Actual icons will be vector/native assets.

- Center the combined dock/status composition horizontally, 16 DIP above the bottom.
- Main dock: 72 DIP tall. Status capsule: 56 DIP tall; share the same vertical center.
- Start, Search and Agents are small system glyphs in generous hit areas.
- Four pinned applications show their actual app icons. Explorer is an app icon here;
  a richer folder widget can be added independently.
- Optional weather preview shows condition, temperature and a quiet city name.
- Wi-Fi, volume, battery and time sit in a separate low-emphasis status capsule.
- Native tray access lives in the status capsule's compact disclosure affordance.
  It remains reachable even if it is not a permanently labeled tile.
- Profile selection moves out of the everyday dock into the Start header/context menu.
  Presentation mode can show a subtle mode indicator. Editing has a dedicated entry.

The precise geometry is a mockup starting point, not a measured native layout result.
At normal text scale, target approximately 650–700 DIP for the main dock and 170–190 DIP
for the status group, separated by 20 DIP. Fit is decided from actual content widths.
Validate the default at 1280, 1440, 1920 and 2560 DIP work-area widths.

## 4. Glass material, with no green theme

Use neutral silver/charcoal glass with a subtle cool cast. Wallpaper colors can show
through naturally; the dock must not add a green tint or recolor app icons.

| Layer | Light glass starting treatment | Dark glass starting treatment |
| --- | --- | --- |
| Body tint | White at approximately 18–24% | Near-black blue-gray at approximately 28–38% |
| Blur | Approximately 24–32 CSS px in HTML study | Same range, tuned for legibility |
| Rim | One thin translucent white edge, brighter at top | Subtle white rim, no colored glow |
| Depth | Broad, soft shadow beneath the whole surface | Broad soft shadow, restrained opacity |
| Foreground | Near-black; sufficiently opaque underlay where required | Soft white; sufficiently opaque underlay where required |
| Hover | Small neutral fill behind hovered item only | Small neutral fill behind hovered item only |
| Focus | Clear visible ring, independent of hover | Clear visible ring, independent of hover |

These are art-direction ranges. Native blur does not use CSS units and must be tuned
separately. Do not translate a CSS blur number into an assumed Windows API parameter.

Glass needs actual background separation: blur, a controlled tint, a restrained edge
highlight and depth. Avoid cartoon refraction, exaggerated glossy streaks, neon halos,
thick double borders and reflections that compete with the contents.

For the browser concept, `backdrop-filter` can demonstrate blur of page content behind
the dock. It cannot demonstrate native blur of other desktop windows. Native Windows
compositing/backdrop behavior remains a renderer feasibility gate. Do not require WinUI
controls to obtain the desired visual appearance.

When transparency is disabled, high contrast is active, the renderer lacks blur support
or the performance budget is exceeded, use a deliberate solid neutral surface. Keep
geometry and hierarchy intact. No glass effect takes priority over readable controls.

## 5. Three visual families, one widget system

"Everything is a widget" defines extensibility and customization. It does not require
everything to look like a boxed dashboard card.

### A. Launchers

Start, Search, Agents, app icons and simple folder shortcuts.

- No permanent background tile around each item.
- App artwork: approximately 44 DIP inside a 48–52 DIP slot, with optical sizing.
- System glyphs: approximately 22–24 DIP, consistent vector style and stroke weight.
- Interactive area: at least 44 × 44 DIP; touch profile increases target sizes.
- Use the real icon for installed apps. Use purpose-designed SVG assets for Proton
  controls, not Unicode stand-ins such as a command symbol for all applications.
- Show name tooltip on hover or keyboard focus; keep accessible names always present.
- Running state: a small neutral dot. Active state: slightly stronger indicator.
- Badge only meaningful events, such as an agent needing approval. No permanent
  arbitrary "2" badge simply to make the concept look active.

### B. Status controls

Wi-Fi, audio, battery, native tray access and clock.

- A connected visual group, with compact icons and consistent spacing.
- Remove persistent "64%", "5h left" and date labels from the default resting view.
- Volume shows a speaker/mute icon; exact value appears in tooltip or opened control.
- Battery shows charge graphically; percentage is optional. Never invent time estimates.
- Time uses 13–14 DIP medium-weight tabular numerals. Date appears on focus/hover/panel.
- Clock opens calendar; audio opens its slider/devices; battery opens power details.
- Each action remains individually focusable. Shared chrome does not merge unrelated
  actions into one inaccessible hit area.
- Keep tray disclosure explicit and discoverable; do not hide access only in a gesture.

### C. Information widgets

Weather, media, next event, richer folders and agent summaries.

- Use one considered information widget in the default main dock, rather than displaying
  every possible metric. More widgets remain available through customization.
- Compact weather: condition icon + temperature, approximately 64–72 DIP wide.
- Standard weather: condition + temperature + city, approximately 112–128 DIP wide.
- Rich weather: today's range + one short next-period summary, approximately 176–200 DIP.
- These are content-driven widths; larger text gets more space or a simpler layout.
- Forecast charts, multi-day details, settings and provider attribution belong in the
  expanded panel. Rich inline weather remains available by deliberate user choice.
- Let the dock's common glass surface carry the widget. An individual backing is used
  only when data grouping or contrast requires it; avoid glass nested inside glass.

The launcher/status/information distinction is a visual presentation choice. Widgets
keep the shared definition, instance, configuration, data-provider and action contracts.

## 6. Spacing and typography

| Token | Initial value / rule |
| --- | --- |
| Main shell padding | 12 DIP horizontally; content centered vertically |
| App slot spacing | 8–10 DIP between hit areas |
| Group separation | Approximately 20 DIP; at most two subtle separators in main dock |
| Between main dock and status capsule | 20 DIP, reduced only when necessary |
| Shell radius | Main approximately 26 DIP; status approximately 28 DIP |
| App icon optical size | Approximately 44 DIP; preserve original artwork/aspect ratio |
| System icon size | Approximately 22–24 DIP |
| Widget temperature | Approximately 20–22 DIP, medium weight |
| Quiet widget label | Approximately 11–12 DIP, never artificially compressed |
| Main UI typeface | Segoe UI Variable for Windows-native readability |
| Tooltip | Compact neutral panel with normal sentence case |

Avoid serif headings, monospaced captions and marketing slogans inside the desktop
mockup. The next presentation should show an actual desktop/wallpaper and the dock
at believable scale. Explanations and controls belong outside that desktop canvas.

## 7. Expanded surfaces

- A panel appears above its own widget, approximately 10–12 DIP away, and clamps to
  the monitor's usable area. The status capsule is a valid anchor too.
- Share the same neutral material family, but use more opacity for text-heavy panels.
- Weather panel: current condition, high/low, short hourly row, then forecast. No
  repeated card around every number. Give the content a clear reading order.
- Audio panel: one dominant slider, mute action and output selector. App mixer later.
- Agents: compact status overview opens the workspace; detailed logs use a proper
  window rather than squeezing a terminal into the dock.
- Search opens a focused search surface. Start opens an organized app drawer. Their
  layouts serve their different tasks while sharing material/type/spacing tokens.
- One transient panel per dock. Retoggle/outside click/Escape dismisses immediately.
- Keyboard focus moves deliberately and returns to the invoker. Hover reveals a name,
  never launches a large panel unexpectedly.

## 8. Motion

- Hover: small lift (approximately 3–4 DIP) and gentle brightening over 120–160 ms.
- Panel: short 140–180 ms opacity/position transition; first useful content appears
  immediately. Closing must not wait for a long reverse animation.
- Magnification: optional and off initially; if enabled, constrain it to launcher icons
  and reserve visual room so adjacent information widgets do not reflow.
- Widget resizing happens in edit mode with a preview. Background data updates do not
  change the dock's width unexpectedly.
- No idle bouncing, shimmer, pulsing glass or animated wallpaper required by the dock.
- Reduced motion removes lift/scale/movement and keeps instant state transitions.

## 9. Customization without visual overload

Users keep the ability to choose widgets, sizes, ordering and profiles. The defaults
should demonstrate restraint rather than advertising every supported feature at once.

- Context menu / Start settings → Customize dock.
- Editing shows insertion slots, selected-widget outline, size choices and a side
  inspector. Resize/remove controls are not permanently attached to every widget.
- Save/Cancel and keyboard reorder/resize remain mandatory.
- Presets: Essential (icons + status), Balanced (recommended + one live widget),
  Information-rich (several intentionally sized widgets).
- Work/Home profiles can keep different compositions; Presentation hides private
  previews and uses a calmer composition.
- Default target: combined resting width stays below roughly 72% of the usable display.
  This is a composition goal, not permission to shrink text or pointer targets.
- On crowding: collapse only widgets explicitly configured for adaptive sizing, move
  unpinned low-priority items into an accessible overflow panel, then offer a denser
  composition. Preserve user-pinned sizes. Do not expose a horizontal scrollbar as the
  default desktop dock solution.
- All controls remain reachable at large text/DPI scales. If a custom layout cannot
  fit, show overflow and explain the constraint instead of silently removing controls.

## 10. Next visual study: exact deliverable

The next step is an isolated HTML design study, not a production Rust change. It should
replace the old illustration only after this direction is accepted.

1. Create `docs/2.0.0/glass-dock.html` and scoped design assets. No runtime dependencies.
2. Show a believable desktop with no oversized heading inside it. Use neutral/cool
   wallpaper studies; provide both bright and dark backgrounds to expose weak glass.
3. Show the recommended Balanced layout at realistic desktop scale. Use native app
   icons or reviewed, properly licensed assets and consistent custom system SVGs.
4. Include Essential and Information-rich compositions, plus a joined-layout option.
   Keep the recommendation visually primary; do not make three unrelated themes.
5. Show weather compact/standard/rich and an expanded weather panel; audio/status,
   focused Search, Start and agent-status states with the same visual language.
6. Demonstrate hover/focus, panel toggle/Escape, profile switching and edit preview with
   sample data only. Maintain explicit accessible names and predictable focus.
7. Verify no default scrollbar, no overlap, no cropped icons, and no unreadable glass
   at representative desktop widths and 100–200% zoom/text scales.
8. Present screenshots at normal desktop scale and close-up. Then restyle the planning
   portal's green chrome to matching neutral surfaces; the portal remains subordinate
   to the product concept.

## 11. Visual acceptance checklist

- [ ] No green tint, green primary accent, olive background or pale-green icon treatment.
- [ ] One coherent glass material; visible depth without heavy outlines or nested pills.
- [ ] App artwork is recognizable, sharp and optically aligned.
- [ ] System symbols share one deliberate icon style; no arbitrary Unicode substitutes.
- [ ] Labels and minor metrics are absent from the resting dock unless informative.
- [ ] Main actions, app icons, information widgets and status have clear hierarchy.
- [ ] Default weather feels integrated and quiet; richer sizes remain available.
- [ ] Native tray, Search, Start and essential status remain reachable.
- [ ] No horizontal scrollbar in the normal desktop resting composition.
- [ ] Light/dark/busy wallpapers remain readable, with a solid-material fallback.
- [ ] Hover and keyboard focus are equally clear; reduced motion is respected.
- [ ] Fullscreen, recovery and performance requirements from the system plan still apply.

## 12. Research references and limits

- [Apple: Use the Dock on Mac](https://support.apple.com/guide/mac-help/open-apps-from-the-dock-mh35859/mac)
  — app/icon actions and everyday Dock behavior.
- [Apple: Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
  — material hierarchy and accessibility adaptation.
- [Apple: Liquid Glass](https://developer.apple.com/documentation/technologyoverviews/liquid-glass)
  — reference for restrained material/color treatment, not an API available on Windows.
- [Microsoft: Acrylic](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic)
  — Windows material behavior and solid/high-contrast fallback.
- [Microsoft: Materials overview](https://learn.microsoft.com/en-us/windows/apps/develop/ui/materials)
  — distinctions between desktop backdrop materials and transparency fallback.

All dimensions, colors, composition rules and animation ranges above are proposed Proton
design choices. No native glass rendering, performance or accessibility result is claimed.
