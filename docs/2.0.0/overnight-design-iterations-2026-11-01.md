# Native dock design iterations — 1 November 2026

This ledger distinguishes source review, build checks and actual native observations.
The reference is Luma's compact shared surface and the installed Apple design skill;
no paid artwork or application code was copied.

## Iteration 1: neutral material and compact widget panels

Changes through `7d5e561`:

- Replace the accent-dependent transient Desktop Acrylic backdrop with no system
  backdrop; Slint supplies the warm graphite material. This is translucent styling,
  not a claim of refractive Liquid Glass rendering.
- Use consistent warm-white focus and glyph treatment, smaller application artwork,
  concise widget previews and content-specific panel dimensions.
- Keep a native neutral volume slider with immediate pointer feedback and keyboard
  arrows; secondary clicks do not change volume. Coalesce queued volume writes.
- Account for owned popup focus, panel anchor retargeting and DPI-scaled panel gaps.
- Add opt-in debug inspection through `PROTON_DOCK_INSPECT=1`. Release builds ignore
  this flag. Inspection changes activation and Alt-Tab behavior, so it cannot prove
  release-mode focus behavior.

General source review found no outstanding high-risk issue. It caught the initial
Sound panel minimum-height mismatch and secondary-click handling; both were corrected.

### Native Computer Use observation

The parent agent captured the actual native application using the friendly app name
`ProtonSearch`. Catalog enumeration and executable-path selection had failed; those
failures did not establish that native capture was unavailable.

Observed in iteration 1: the warm material removed the blue appearance, but the Sound
panel clipped its close control/footer. The dock also showed excess empty width and
misaligned glyphs. Source/build success therefore did not establish visual acceptance.

## Iteration 2: component and native geometry agreement

Runtime correction `814d949` passes explicit logical width/height into Slint component
properties before native positioning. The UI must bind its window dimensions to those
properties; fixed 1100px dock and 404×460px panel bindings can disagree with native HWND
geometry and clip content. Widget artwork and compact control alignment are also being
corrected.

Status: UI integration, fresh build/tests and native recapture remain pending.
No visual pass is claimed for this iteration yet.

## Acceptance still open

- Native Sound, Focus, Customize and application drawer fit without clipped controls.
- Normal-mode activation/dismissal, rapid panel retargeting and interrupted transitions.
- Keyboard traversal, screen-reader slider operation, reduced motion and high contrast.
- 100%, 150% and 200% scaling, mixed monitors, fullscreen and Explorer restart.
- Taskbar restoration on graceful exit and renderer crash.
- Measured idle resources and frame pacing; no zero-lag or zero-work claim is made.

The application is stopped before edits/builds and opened only for explicitly
authorized native inspection. Existing profile data and search behavior are retained.
