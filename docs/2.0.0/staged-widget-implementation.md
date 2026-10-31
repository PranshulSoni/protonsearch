# Staged native widget implementation

Prepared on 31 October 2026 from the saved project knowledge graph. This is an
implementation sequence, not a claim that the stages have passed acceptance.

## Evidence and scope

The audit queried `graphify-out/graph.json` and read its node descriptions and
neighbourhoods. No application source was read for this planning audit. The graph
records source snapshot `4c359532`; source locations below are graph provenance,
not a fresh verification of the referenced code.

| Graph evidence | Current state | Implication |
| --- | --- | --- |
| `concept:profiles`, `Profile` — `native-implementation.md:L65`, `model.rs:L167` | Work/Home/Presentation retain widget visibility, order, widths, pins and geometry | Extend the existing model; do not rebuild profiles |
| `concept:native-firstparty` — `native-implementation.md:L63` | Audio/mute, battery, network, clock, focus timer, media keys and system routes exist | Test actual widgets individually before adding more |
| `concept:compact-layout`, `DockWindow` — `dock-redesign-2026-10-24.md:L29`, `dock.slint:L67` | Existing 82px shell, 60px content and 48px artwork contract | Treat these as a baseline to test, not proof that the design is good |
| `concept:controller-process`, `concept:local-search` — `native-implementation.md:L17`, `README.md:L40` | Existing Win32 search/index controller behind independent native dock | Keep the search engine and process boundary |
| `concept:media-metadata-pending`, `concept:tray-pending` — `dock-redesign-validation.md:L66` | Media metadata and third-party tray hosting are not implemented | Media keys and Settings links must not masquerade as complete widgets |
| `concept:weather-provider-plan` — `widgets.html:L44` | Weather provider is planned, including manual location, attribution, cache and offline states | Ship weather as a separate provider stage |
| `concept:manual-native-acceptance` — `dock-redesign-validation.md:L44` | Visual, DPI, fullscreen and recovery acceptance remains pending | Build/unit-test success does not close native acceptance |
| `concept:selected-tests` — `dock-redesign-validation.md:L13` | Historical result: 104 selected tests passed; Antigravity icon test excluded | Re-run appropriate checks for each changed build |

Luma Bar is a reference for a unified app/widget surface, compact live previews,
configurable composition and expanded controls. Its official README documents
multiple styles and sizes, drag placement and taskbar visibility controls. This
does not establish exact spacing, animation quality, performance, or absence of
profiles/search. Keep those claims unknown until verified. See
[`luma-baseline-research.md`](luma-baseline-research.md) for the evidence record.

Use the [requested Apple design skill](https://github.com/emilkowalski/skills/blob/main/skills/apple-design/SKILL.md)
as an interaction rulebook adapted to native Slint and Windows. Do not port its
CSS examples or add a web renderer. The installed skill's material and motion
guidance is a design input, not evidence that the current renderer supports every
effect or that a particular spring library is required.

## Delivery rule

Each stage produces a separately identifiable native build, short change list,
test checklist and known limits. Preserve the prior passing executable and saved
layout backup before the user tries a new build. Stop at the user acceptance gate
before advancing behavior or visual composition that depends on that feedback.
Research, documentation and graph maintenance may continue while feedback is pending.

The graph is the navigation and status authority. Before editing a graph-located
symbol, verify that exact source span and its callers; the graph cannot safely
replace checking current code. If it changed, refresh the structural graph and
re-read the affected Slint/document semantic fragment before planning the edit.
Record newly implemented behavior and acceptance results in the graph after each
stage. Never promote a planned node solely because a build passed.

## Stage 0 — evidence and acceptance baseline

**Deliverable:** Luma evidence record, Apple rules, graph-backed gap list and this plan.

- Save official reference URLs and capture dates. Separate text claims from observed visuals.
- Record application build identity, current profile and screen/DPI settings for comparison.
- Keep native Windows taskbar visible by default during design trials.
- Capture actual native dock and one expanded widget on the target machine. If capture
  tooling cannot target the tool window, use the user's observation; do not claim a visual pass.

**Gate:** The baseline clearly distinguishes implemented, planned and unverified behavior.
**Rollback:** Documentation/data only; no runtime change.

## Stage 1 — one visual and interaction contract

**Files located through graph:** `protonsearch/ui/dock.slint` (`DockWindow:L67`,
`GlassButton:L13`, `DockPanelWindow:L185`), `src/dock/mod.rs`
(`refresh:L265`, `position_panel:L153`, `update_rows:L70`).

**Small scope:** refine the shared surface and the existing audio/focus/clock previews.
Keep widget inventory, providers, profiles and search behavior unchanged.

- Use one quiet neutral glass shell, shared widget height, consistent app slots,
  bounded data widths and a restrained type/icon hierarchy. Avoid permanent card fills
  on every widget. Start with existing geometry; change measured tokens together.
- Show press feedback on pointer-down; commit actions on release. Keep cancellation possible.
- Open and close panels along the same anchored path. Repeated clicks may reverse the
  current transition; never disable input until a transition finishes.
- Keep artwork movement within fixed slots. Do not animate native window/work-area geometry
  for hover. Reuse stable models instead of recreating delegates during clock updates.
- Adapt Windows reduced-motion, transparency and high-contrast preferences separately.
  Use static/cross-fade alternatives, legible solid material and visible keyboard focus.
- Keep Windows system type and real high-resolution app artwork. Missing data uses an
  honest unavailable state, not placeholder values that resemble live telemetry.

**User checklist:** compare dock at rest, hover and press; open/close each of three
panels repeatedly; reverse a closing panel; use keyboard activation/Escape; try a
busy wallpaper and reduced-motion/high-contrast modes. Verify no overlap, clipping,
focus stealing or bottom-edge jump at 100%, 150% and 200% DPI where available.

**Checks:** Slint/Rust compile; targeted geometry/state tests only if logic changes;
native screenshots/observations. Do not create tests that merely mirror colour values.
**Gate:** User accepts the direction and names remaining visual issues before Stage 2.
**Rollback:** Previous executable; retain profile/layout data without migration.

## Stage 2 — core widgets, one group per test build

**Files:** `ui/dock.slint`, `src/dock/mod.rs` action/snapshot paths,
`src/dock/platform.rs` owned Windows providers; graph node `concept:owned-provider-snapshots`.

1. **Audio:** volume and mute reflect actual Windows state; slider feedback is immediate;
   failure is visible and the next snapshot reconciles optimistic state.
2. **Focus:** start, pause, resume, completion and reset use monotonic elapsed time;
   changing profiles must not silently invent or reset a running timer.
3. **Clock, power and network:** correct local time, AC/no-battery handling, connection
   changes, missing/denied provider state and appropriately named system routes.

Each group gets its own acceptance checkpoint. Test unavailable data and external
OS changes, not only a happy-path click. Keep slow provider work outside input/render paths.
**Gate:** User can complete each widget's advertised action without dead controls.
**Rollback:** Revert the failing group, not the accepted shell or unrelated widgets.

## Stage 3 — application and contextual widgets

- Validate pinned/running app identity, high-DPI artwork, launch/activate, multiple-window
  chooser, pin/unpin and close-with-unsaved-work behavior. Overflow must remain discoverable.
- Add media-session metadata as its own build. Retain media-key fallback and honest
  no-session states; do not display sample song titles or artwork.
- Add weather only in its own build: manual location, provider/license choice, attribution,
  cache age, loading/error/stale/offline states and bounded background refresh.

**Gate:** Accept application behavior first, then media, then weather independently.
**Rollback:** Disable/revert the new provider or contextual widget without changing layout schema.

## Stage 4 — profiles as a visible advantage

**Files:** graph-located `src/dock/model.rs` `Profile`, `DockConfig`, `LayoutStore`;
native profile selection/customization surfaces in `dock.slint` and `mod.rs`.

- First verify existing three-profile isolation across pins, order, width, visibility
  and restart. Add a clear current-profile indicator and predictable switching feedback.
- Only after parity passes, add profile create/rename/duplicate/delete and composition
  editing in separate small builds. Deleting the active/last profile needs defined fallback.
- Preserve unknown/corrupt/future layout handling, atomic save, backups and Unicode text.
  Do not migrate the search index or credentials with a dock profile.

**Gate:** User builds distinct work/home compositions and restarts without cross-profile leakage.
**Rollback:** Previous executable plus validated pre-migration layout backup.

## Stage 5 — search parity, then measured improvements

**Files:** existing `src/search.rs`, Win32 controller `src/main.rs`, dock command bridge
in `src/dock/mod.rs`; graph nodes `concept:search-engine-reuse` and `concept:generation-search`.

- Preserve hotkey, prefixes, indexed files/content, application launch and error/empty states.
- Measure representative cold/warm queries, query-to-first-result and query-to-final-result
  before promising efficiency. Compare on the same machine/index with a repeatable query set.
- Implement generation cancellation and incremental fast/slow result composition only where
  the graph and targeted source verification reveal a gap. Rapid typing must not show stale results.
- Keep ordinary search separate from explicit agent execution authority.

**Gate:** No search parity regression; measured changes improve the chosen bottleneck.
**Rollback:** Existing controller/search path remains available while new composition is disabled.

## Stage 6 — taskbar replacement release gate

Validate restart/crash/exit restoration, fullscreen, Explorer restart, DPI/display
changes, accessibility and idle/input resource use on real Windows. Treat taskbar
visibility and native tray hosting as separate capabilities. Preserve opt-in hiding
until native tray/input/security workflows and recovery pass acceptance.

Widget marketplace, arbitrary executable extensions and agent scheduler replacement
stay outside these stages. The graph marks those as planned work requiring their own
security and resource contracts, not prerequisites for a good first-party dock.

## Acceptance record for every stage

Record commit/build identity, changed widgets, machine/DPI, test commands/results,
native screenshots or user observations, pass/fail items, known gaps and rollback
artifact. Record measured performance separately from design targets. A failed gate
keeps the current stage open; it does not justify bundling later features into the fix.
