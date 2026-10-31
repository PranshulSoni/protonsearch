# Stage 1: native widget feedback and motion

Status: native changes committed in `af7fe7e`; compilation is being checked. User acceptance
is pending. This stage does not declare the dock design complete.

## Evidence and boundaries

Project analysis started with the saved graph, then used its source locations for the
specific Slint and runtime edits. Relevant nodes: `dockwindow`, `dockpanelwindow`,
`ui:glassbutton`, `ui:dockcallbacks`, `concept:compact-layout`,
`concept:anchored-popovers`, and `protonsearch_src_dock_mod_refresh`.
The graph identifies existing contracts; source verification is still required before edits.

[Luma Bar's official repository](https://github.com/Acadbek/lumabar) documents a shared
app/widget dock, configurable appearance, fullscreen avoidance and reversible Windows
taskbar hiding. The research agent inspected the official website preview: one continuous
glass surface, aligned app artwork, subtle separators, and wider information widgets.
Website preview pixels are not native dimensions or performance measurements.
Luma's Search widget exists. Profile support and comparative search efficiency are not
established by the examined sources. They must not be recorded as proven omissions.

[Apple design skill](https://github.com/emilkowalski/skills/blob/main/skills/apple-design/SKILL.md)
guides feedback, spatial consistency, material hierarchy, restrained motion and accessibility.
Its browser examples are principles to adapt, not CSS or JavaScript to insert into Slint.

## Fixed visual contract

Keep the existing 82 logical-pixel window, shared 60-pixel content band, 48-pixel app
artwork, 60-pixel icon slots and default 124-pixel data slots. These are Proton baseline
values, not measurements copied from Luma. Keep every widget on one baseline.
Hover must never resize the native window or move neighboring tiles.

Use one neutral charcoal glass surface. Resting tiles remain transparent; hover,
press, selection and keyboard focus provide distinct feedback. Stage 1 slightly deepens
the surface and softens its border. It does not implement Luma's proprietary refraction.
The existing Windows material remains the backdrop mechanism; unsupported appearance
must be judged on the native build, not inferred from the HTML prototype.

Retain Segoe UI Variable and native Shell application artwork. Keep 24–25-pixel system
glyphs. Secondary previews use 11-pixel text with stronger contrast. Keyboard focus uses
a visible two-pixel outline. These are design choices requiring native acceptance, not
claims of measured WCAG compliance or sharpness at every display scale.

## Widget and panel states

| Surface | Preview | Stage 1 check |
| --- | --- | --- |
| Applications | Independent app artwork and actual running dots | No tile reflow during hover; same hit area throughout press |
| Sound | Actual volume or muted state | Slider/mute keep current native callbacks; press feedback starts on pointer-down |
| Focus | Actual timer value and progress | Start/pause/reset remain reachable; no delayed stale reveal after close |
| Clock | Current time | Stable width and baseline as digits change |
| Weather | Unavailable state until a provider exists | No fake forecast and no dead location field |
| Other widgets | Existing actual state or explicit unavailable state | No additional provider or new widget is claimed by this stage |

Panels retain their existing trigger anchor and monitor clamping. They enter and leave
along the same vertical path. Inputs remain available during transitions. Rapidly reversing
a toggle cancels pending reveal work; the runtime no longer leaves an orphan reveal timer.
This stage does not add a gesture or spring engine and does not claim velocity-continuous
spring behavior. Drag-based gestures need a later isolated implementation and test gate.

## Accessibility and motion policy

Read Windows `SPI_GETCLIENTAREAANIMATION` at startup and once per existing heartbeat.
The query uses a native `BOOL`; failure selects static behavior. No new setting or dependency
is introduced. Both native windows receive the resulting reduced-motion property.

With animation enabled, existing short hover and panel transitions remain. Press colour
feedback is immediate rather than delayed by the previous background transition.
With animation disabled, app magnification/lift, glyph lift, progress interpolation and
panel slide/fade stop. Press colour/opacity and keyboard focus still communicate state.
Panel dismissal does not wait for an invisible animation.

Reduced transparency, high contrast, text scaling, keyboard traversal and screen-reader
labels remain explicit manual gates. Stage 1 does not claim those Windows settings are
newly wired. Native preference changes should be tested while the dock remains open.

## User acceptance gate

Use the native executable. Do not judge this stage from the browser mockup.

1. Compare resting, hover, pressed and keyboard-focused states on light and busy wallpaper.
2. Check app, Sound, Focus and Clock widgets share a baseline at 100%, 150% and 200% scale.
3. Press without releasing: feedback must already be visible. Drag away before release
   and check controls do not activate accidentally.
4. Open and close Sound and Focus repeatedly, including at least 20 rapid reversals.
   No late panel should appear after dismissal; no panel should flash at the default origin.
5. Turn Windows animation effects off, then on, while Proton runs. Within the next
   heartbeat, hover movement and panel transitions must follow the system preference.
6. Test keyboard activation, visible focus, Escape/close behavior, panel controls and
   profile switching. Record any missing route rather than calling the whole stage passed.
7. Confirm volume, timer, search hotkey, application actions and taskbar restoration still
   behave as before. Do not advance merely because the screenshot looks better.

Build/check evidence belongs in the implementation handoff. Actual visual, accessibility
and resource measurements must be recorded by the tester. No zero-lag or idle-CPU claim
is made here.

## Next stages and rollback

Stop after this increment for native feedback. Follow the separate staged implementation
plan for widget detail layouts, provider completeness, profile management, search integration
and recovery validation. Do not bundle those into Stage 1.

Rollback is the Stage 1 source commit, not deletion of saved profiles. This increment changes
no layout schema, saved data, taskbar ownership, provider contracts or search/indexing code.
