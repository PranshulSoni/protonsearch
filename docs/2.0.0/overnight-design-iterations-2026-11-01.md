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

UI integration landed in `e91959a` and `3d6e2ac`. The parent agent captured the actual
application in normal mode, without the debug inspection flag, through
`get_app_state(app: 'ProtonSearch')`.

Observed native results:

- Dock: 1060×82, centered glyphs and no blue appearance.
- Sound: 320×300, with close control, Mute and footer fully visible.
- Focus: 320×320; Start changed the timer to 24:59 and exposed Pause, and Reset
  restored 25:00.

These observations establish those specific normal-mode layout and timer interactions,
not full accessibility, frame-pacing or recovery acceptance. The earlier debug-only
capture limitation is no longer a blocker for native inspection.

## Iteration 3: issues found by native inspection

The iteration 2 captures exposed two further defects: the volume fill was centered
inside its track instead of left-anchored, and panel closing could briefly show generic
Proton/system-settings content after runtime panel state was cleared. Corrections are
in progress, alongside removal of the irrelevant System settings footer from Focus
and a more compact Focus panel.

Status: final source review, build/tests and native recapture remain pending. No pass
is claimed for these corrections until their final build is checked.

## Acceptance still open

- Native Sound, Focus, Customize and application drawer fit without clipped controls.
- Normal-mode activation/dismissal, rapid panel retargeting and interrupted transitions.
- Keyboard traversal, screen-reader slider operation, reduced motion and high contrast.
- 100%, 150% and 200% scaling, mixed monitors, fullscreen and Explorer restart.
- Taskbar restoration on graceful exit and renderer crash.
- Measured idle resources and frame pacing; no zero-lag or zero-work claim is made.

The application is stopped before edits/builds and opened only for explicitly
authorized native inspection. Existing profile data and search behavior are retained.

## Final overnight verification — 1 November 2026

Final application source: `eb866a9` (including runtime `2e47e9c` and app-label change
`9a11834`). The temporary debug inspection branch was removed. All observations below
used the normal dock window behavior and the installed Computer Use plugin.

### Native iteration results

- Iteration 3: captured Dock at 1023×82 logical pixels, Sound at 320×300 with the
  fill anchored to the rail's left edge, and Focus at 320×260 without its unrelated
  System settings footer. Close-transition accessibility state retained the departing
  Sound content rather than replacing it with generic Proton/system-settings content.
- Iteration 4: native Applications capture disproved the first list fix. The outer
  start-aligned layout still left a one-row viewport and a large empty area.
- Iteration 5: conditional stretch alignment fixed the shared list layout. Captured
  Applications at 440×500 with multiple visible rows, compact 48px Pin buttons,
  readable titles and human-facing Application/Pinned labels instead of launch IDs.
  Its Refresh action remains below the scrolling viewport.
- Customize: accessibility inspection reports 440×620 with the profile control,
  widget controls, multiple rows and Refresh inside the logical bounds. The screenshot
  provider cropped this larger surface. A DPI-aware Win32 read confirmed the actual
  770×1085 physical client at 168 DPI, consistent with 440×620 logical pixels.
  Do not treat the cropped capture as a complete Customize visual pass.

The reliable capture route is the Computer Use plugin's legacy friendly-name lookup,
`get_app_state(app: 'ProtonSearch')`. Catalog/path-based lookup omitted the dock.
Some accessibility click calls reported an error after performing the action; every
such result was re-observed before further input. No guessed window handles were used
for UI actions. Native captures were inspected in tool output; no screenshot files
were persisted. The app was stopped after the inspection session.

### Automated evidence

- Final native build passed in 58.87 seconds.
- Final selected tests passed: 10 library and 97 binary tests (107 total). The known
  baseline `tests::test_antigravity_icons` test was filtered out.
- Targeted Rust formatting passed for `dock/mod.rs` and `dock/platform.rs`.
- General and Rust source reviews found no outstanding HIGH or MEDIUM issue in the
  reviewed changes. Clippy was not rerun; the known main.rs literal diagnostics remain.
- Executable: `protonsearch/target/debug/ProtonSearch.exe`.
- SHA-256: `1ED10DA53D8FE01CA50F660599581A9184D3F9C388665EDBABAD570974EBFBE3`.

This delivery changes the dock/panel presentation and the observed interaction defects.
It preserves profiles, launch commands, search/indexing and existing provider behavior.
The neutral translucent material does not implement Luma's refraction shader. Live
weather, media metadata and a real third-party tray host remain outside this delivery.
The broader DPI, accessibility, recovery and performance gates above are still open;
no zero-lag or complete taskbar-replacement acceptance is claimed.
