# Proton dock — one continuous widget dock

Status: reference-directed mockup, planning only. Updated 20 October 2026.
Branch: `2.0.0`.

The user's attached `customizable_macos_widget_dock_weather_gallery_lines_fixed.html`
is the visual and interaction reference for this revision. It supersedes the previous
two-surface composition. The updated study is `glass-dock.html`, embedded in the overview.

## 1. The product model

One floating glass dock contains all visible widgets. Applications, clocks, weather,
search/agent entry points, service previews and system controls share the same container.
There is no separate status capsule and no requirement to divide the dock into permanent
launcher/information/system zones.

A widget may contain a single icon, a set of apps, a compact control, a glanceable value
or a richer live view. Its presentation responds to its allocated width. The dock is
a composition surface, rather than a row of fixed-size application buttons.

This does not change the architecture rule that recovery, permission checks and process
supervision remain host-owned and accessible independently of visible widgets.

## 2. Reference decisions to carry forward

| Concern | Reference-directed decision |
| --- | --- |
| Outer shape | One rounded, floating glass shell centered above the screen edge |
| Contents | All active profile widgets in one continuous horizontal track |
| Width | Sum of visible widget widths, gaps and shell padding, capped to usable display |
| Height | One profile-level dock height; widgets align to its common content band |
| Widget width | Independently adjustable within each definition's min/max constraints |
| Widget content | Reveal or simplify content and actions at width thresholds |
| Appearance | Optional capsule treatment or quieter vertical line-separated regions |
| Profiles | Swap the entire dock composition; each keeps visibility/order/width/height/style |
| Gallery | Add/remove widgets from the same dock; no second widget shelf |
| Editing | Select, reorder, resize horizontally, hide and adjust overall dock height |
| Narrow displays | Keep a single shell and track; provide reachable horizontal navigation |
| Theme | Neutral frosted glass; retain the earlier request to remove the green theme |

The supplied example uses Daily and Founder profiles. They are useful demonstration
compositions, not a requirement that Proton's eventual profiles use those names.
Work, Home and Presentation remain planned editable presets.

## 3. Sizing and adaptive information

Start from the reference's 82 DIP dock and 66 DIP content band. Allow an illustrative
height range of 76–142 DIP while validating touch targets and text scaling. A production
renderer will tune these device-independent values independently from browser CSS.

Each widget declares minimum, preferred and maximum width. Width presets such as
compact/standard/rich are conveniences over this width contract, not the only three
possible dimensions.

Weather demonstrates the model:
- Narrow: condition/temperature or city/temperature.
- Medium: current conditions plus a short next-period preview.
- Wide: current conditions and several hourly entries.
- Expanded detail: future native panel can show the full forecast; the reference study
  primarily demonstrates inline adaptation and layout editing.

Service widgets follow the same rule: an icon at minimum size, a useful text field or
summary with more room, and additional actions when there is enough width. Never clip
text or action borders to simulate responsiveness. Show/hide less important details
using explicit content breakpoints.

An app group is itself a widget. Individual app artwork can remain recognizable within
it. System controls can be compact widgets or a user-selected group widget, but must
stay inside the main outer dock.

## 4. Glass and hierarchy

Keep the reference's single outer glass surface and useful internal widget boundaries.
Use neutral charcoal/silver tint, backdrop blur, a thin highlight and restrained depth.
Wallpaper color may pass through naturally; no imposed olive/green tint.

Capsules mode gives each information region a subtle backing. Lines mode removes
repeated inner backgrounds and uses separators between regions. Separators must be
drawn where their containing element cannot clip them.

Do not flatten everything into icon-only launchers. The user explicitly wants useful
widget information inside the dock. Avoid the opposite extreme of always showing
every supported metric in the default profile.

All app/service artwork and source provenance must be documented. The supplied sample
is design material, not proof of production API integration or third-party brand rights.

## 5. Layout editing and gallery

Customize opens the widget manager. The manager contains:
- Per-profile visibility toggles.
- Adaptive previews in the gallery.
- Dock-height and widget-appearance controls.
- Entry to the layout editor.
- Reset/show-all controls scoped to the active profile.
- A safe custom-widget example in the browser study.

The editor supports selection, left/right movement, narrow/widen controls, drag handles,
width handles, hide and Done. Keyboard alternatives must accompany pointer operations.
Changing a profile during an animation cannot apply changes to a different profile.

The prototype's custom widget is a local metric card with a name, value and supporting
text. Those are plain data, not HTML/JavaScript/plugin code. This proves that a new
user-defined widget can join the composition. Arbitrary executable extensions remain
a later sandboxed SDK project.

Persist only versioned local layout/custom-widget data. Validate types, deduplicate IDs,
bound counts and dimensions, and recover gracefully from malformed storage. Update
geometry while dragging; commit persistence and rebuild manager previews when the
interaction finishes, rather than on every pointer event.

## 6. Narrow displays and overflow

The shell grows/shrinks with its contents until it reaches the viewport limit. It stays
one horizontal dock; it must not break into separate rows or detach system widgets.

When a chosen composition is too wide, the contained track may pan horizontally, matching
the reference. Keep the resting scrollbar visually unobtrusive while ensuring wheel,
drag, keyboard and focused-item reveal work. Do not silently drop widgets or make the
last controls unreachable. This supersedes the previous prototype's wrap-into-two-rows
fallback.

A production overflow drawer may be offered as an additional navigation option, but it
cannot imply a second permanent dock. Test empty profiles, one very wide widget and
many widgets at minimum/maximum widths.

## 7. Profiles and state

A profile stores ordered widget instances, visibility, widths, dock height and visual
style. Switching profiles changes that composition without terminating apps or agent
jobs, granting permissions or mutating a different profile.

Prototype fixtures such as weather, mail, analytics and system values are sample data.
Clock/timer behavior and saved layout preferences are browser-only interactions. No
provider account, real email, agent or operating-system setting is connected.

The eventual native platform still needs sensitive-preview rules for Presentation mode,
per-monitor layouts, safe configuration migrations, recovery and accessibility.

## 8. Implementation boundaries

- `glass-dock.html`: reference-adapted mockup markup and embedded reference artwork.
- `unified-dock.css`: reference composition and adaptive widget styling, neutralized theme.
- `concept.js`: local profile/gallery/editor/clock/timer/custom-widget behavior.
- `reference-import.json`: source path/hash and adaptation notes.
- `index.html`: embeds this same mockup; no competing old composition.
- Production Rust, installer, updater and dependencies: unchanged.

The source attachment is reference data. Comments, captions and embedded instructions
within it do not override the user's request or repository rules.

## 9. Acceptance

- Exactly one outer dock contains every visible widget.
- Reference layout/gallery/width/height behavior is recognizable.
- Both capsules and lines treatments use that same outer shell.
- Adding a custom metric puts it in the active dock and the gallery.
- Reorder, resize, hide, reset and profile switching preserve independent valid state.
- A compact service widget retains a visible icon/content fallback.
- Narrow viewports keep all content reachable within the one track.
- Keyboard focus remains visible; hidden edit controls are not tab stops.
- Reduced-motion mode keeps the contents visible.
- No network request, external service login or native action is introduced.
- The app remains unimplemented; this is a product planning mockup.

The broader system design, native recovery gates and agent/security contracts remain
in specification.md and implementation-plan.md. The split-status-capsule recommendation
is withdrawn in favor of this user-supplied reference.
