# REVIEW_3 — G4/G5 design spec-sync sign-off (task-47)

**Date:** 2026-11-10. **Owner:** L1-B UI/Motion/Product Design. **Scope:** design/RECON sign-off for G4 (desktop app,
onboarding, settings, migration UI) and G5 (previews/OCR/clipboard/browser/git/actions + Dynamic Island). No code changes.
**Purpose:** give rel-recon a single, current status after the Normal Launcher, preview and Dynamic Island
implementations caught up with the binding design specs.

## 1. Binding specs synced

| Spec | Change | Status |
|---|---|---|
| `SCREENS.md` §2.11 | Normal Launcher filter strip now four chips **ALL/FILES/FOLDERS/APPS**, Ctrl+1–4; all other scopes through prefixes/quick destinations; strict FILES/FOLDERS unchanged | synced to implemented v10 |
| `SCREENS.md` §2.4 | Empty state is the real **QUICK DESTINATIONS** listbox: 2×2 primary (Browser Bookmarks/History/Clipboard/Agents) + secondary Git history/Agent History, real counts/off/agent state, syntax hint, height 248/224, keyboard traversal | synced to implementation |
| `SCREENS.md` §2.6 | Preview is the real G2 IPC `PreviewPayload`: loading skeleton, text/code with highlight, image, OCR, fields, truncation, unsupported/error + Retry; 360 side/240 drawer; 160 ms/6 px panel | synced; error-detail/shimmer code gaps tracked below |
| `SCREENS.md` §2.12 | Clipboard grouped sections remain; trigger is now the `clipboard:` prefix/Clipboard History quick destination, not a chip | synced |
| `DESIGN_SYSTEM.md` | FilterStrip 4 chips/Ctrl+1–4; new QuickDestinations component/§8.2 rule; PreviewPanel real payload; task-47 decision | synced |
| `KEYBOARD_AND_A11Y.md` | Ctrl+1–4; quick-destination listbox/↑↓/Enter; preview real-payload ARIA; four-chip ARIA/acceptance | synced |
| `ISLAND_V9_SPEC.md` | shadow pad 112 (fits 107 px extent); expanded dots 14+5 layout with 24×30 hit; compact 11×37 documented a11y deviation; borderless input focus; Actions filter; clipboard thumbnails/subtitles; flat agent tasks; implementation status | synced to task-41 |
| `ISLAND_V9_GAP_LIST.md` | P0 blockers closed; remaining adapters/IPC/a11y/evidence gaps listed with owners | current |
| `REFERENCE_MAPPING.md` D23 | filter strip/deviation text updated to 4 chips + prefixes/quick destinations | synced |

## 2. Gate status

### G4 — desktop app, onboarding, settings, migration UI

**Design status: PASS WITH CONDITIONS.**

- App type/grid/quiet-state system is closed (REVIEW_2 F1–F9)
- Preferences → Launcher/Dynamic Island settings UI shipped by ui-app (task-40): primary launcher choice, placement
  editor with real monitors, 3×3 anchors/DIP offsets/ghost/Apply/Revert/Test, card manager with live dot preview,
  behavior fields; embedded-frame adaptation (~928 px at 1240) is allowed by ISLAND_SETTINGS_SPEC §9.
- Normal Launcher's four-chip strip, quick destinations and real preview are now specified exactly (this sync).
- **Conditions:** (a) preview error detail must not expose raw bridge text, and loading skeleton should be flat grey
  (currently pulse) — fix before release; (b) monitor policy (cursor/active) and per-monitor persistence remain IPC gaps
  and must stay explicit, no fake controls; (c) pin/undo/Normal-Launcher-specific options remain IPC gaps; (d) real
  Tauri overlay/listMonitors/DPI/taskbar visual evidence is still L0-owner (mock captures not native evidence);
  (e) refresh stale live-shots (island-shown/placeholder show the old letter tile).

### G5 — previews/OCR/clipboard/browser/git/actions + Dynamic Island

**Design status: PASS WITH CONDITIONS.**

- Island renderer v1 exists; geometry matches the V9 table; tokens/type/icons/dots/motion implemented; shadow pad 112;
  search/clipboard/browser/git real data; agents flat real tasks; honest disabled Media/Quick Settings/Notifications.
- Design-review deltas resolved in captures: dots V9 pitch with 24×30 hit, borderless input focus, placeholder/Actions,
  no search recency meta, clipboard counts/copy/thumbnails, V9 card subtitles, browser two-end context/chevrons.
- **Conditions:** (a) native Media/Quick Settings/Notifications adapters (task-35) still pending — cards must keep
  honest disabled states, never mock data; (b) agent task-tree/log and clipboard pinning IPC pending; (c) compact dot
  11×37 remains a documented WCAG target-size deviation (showDots opt-out + keyboard/wheel); (d) zero-results,
  reduced-motion and adapter drill-down captures still missing; (e) real Windows/DPI/frame-pacing visual evidence remains
  L0/task-31; (f) agent copy nit "1 TASKS"/"1 permissions" and visually ambiguous Pause+Resume pair to fix.

## 3. Evidence reviewed

- `docs/qa/visual/screenshots/island-v9/` — 20 refreshed captures (compact 5 widths; search/clipboard/agents at
  1150/520/335; media/controls/browser/git/notifications at 1150; clipboard-images) + `VISUAL_AUDIT_ISLAND.md` D2.
- `docs/qa/visual/live-shots/` — Tauri overlay captures: `island-live-compact` (current renderer + shadow),
  `island-card-1..7`, `island-clipboard-images`; `island-shown/placeholder` are stale (old tile) and must be
  refreshed or labelled.
- Reference authority remains `docs/qa/island-reference/` (136 captures) + `ISLAND_V9_SPEC.md`.

## 4. Re-sign recommendation for rel-recon

- **G4:** re-sign with the conditions above recorded. No design-owned blocker; Normal Launcher/quick destinations/preview
  specs are now implementation-ready and match shipped behaviour.
- **G5:** re-sign as **conditional** (renderer v1 accepted, gaps owner-tracked). Do not claim full V9 card fidelity until
  Media/Quick Settings/Notifications adapters exist and real Windows captures replace mock-only evidence.
- Re-sign must not treat the documented honest gaps as defects of this design pass; they are capability/evidence gaps in
  the owning tracks. If any condition regresses the visual/semantic contract (fake data, dead controls, hidden raw error,
  removed a11y alternative), reopen the design review.

## 5. Task-49 evidence update (2026-11-10)

**Additional evidence verified:** island-v9/zero-results-{1150,860,430}, reduced-motion-{compact,search}-1150,
expanded search/clipboard/agents at 860/430, refreshed media/controls/notifications, and
`docs/qa/visual/screenshots/island-v9/task49-spec-recheck.json` (surface boxes, dots pitch, focus, filters, clipboard,
agent rows, reducedMotion). Agent copy/action code is now pluralised and action-gated; Controls/Notifications are wired to
the shell controls API (volume 62 %, brightness 80 %, audio output, real notification list).

**Still open conditions (G5 stays conditional):**
- Controls card overflows at 516x376: bottom Wi-Fi/Bluetooth tiles are cut behind the dots; V9 structure (2x2 tiles +
  brightness/volume) must fit or audio-output must move to the Media drill-down.
- Zero-results copy is in the DOM but not visible at the canonical 185 px card; render a compact visible empty state.
- Notifications: dismiss is disabled pending a bridge command and a test-only “Test local toast” control appears; gate
  test controls out of production and give disabled dismiss a clear state.
- agents-1150/520/335 captures are stale (code fixed) and must be refreshed; stale live-shots remain L0/design.
- Media adapter + media/Wi-Fi/BT drilldowns still pending; compact 11x37 dot a11y deviation remains documented.
- Preview error detail/raw and pulse skeleton remain as previously recorded.

**Gate status unchanged:** G4 PASS WITH CONDITIONS; G5 PASS WITH CONDITIONS (evidence improved, new visual fixes required
before rel-recon can treat card states as fully accepted). re-sign may proceed for the rest; these are owner-tracked.

## 6. Task-56 user-rejection blocker (2026-11-10)

User rejected the island visual/UX quality: bleed/translucency, overlapping notifications, Quick Settings showing
"Unsupported", and search not matching Normal Launcher. New binding doc: `ISLAND_VISUAL_UX_FIX.md` (opaque surface,
enter/exit 180/120 ms transform-only, fixed-grid notifications, usable Quick Settings + Open Windows settings fallback,
island Search parity with Normal Launcher). ISLAND_V9_SPEC §12 records the override; SCREENS §2.13, DESIGN_SYSTEM and
KEYBOARD are synced.

**Task-56 visual/UX gate: PASS (reviewed 2026-11-10; notes closed by ui-launcher task-54 follow-up).** After set at 1150/860/430 over a white Notepad/text
window verifies: (a) fully opaque island surface — no desktop text/previous-card bleed inside the bbox at rest or during
the captured enter/exit frames; (b) notifications 0/1/3 use the fixed 18/1fr/30 grid with 52 px rows, reserved dismiss,
fixed footer and no overlap; (c) Quick Settings is usable — volume 62 %/mute, brightness 80 %, audio output, split
Wi-Fi/Bluetooth tiles with drill-downs, real "Open Windows settings" fallback, no "Unsupported"; (d) island search shows
the same four filters, shared group labels/order/icons and canonical empty copy as Normal Launcher. Enter/exit CSS values
(180 ms / 120 ms transform-only, opacity constant 1) are verified in code and by the 0 ms frames.

**Notes closed:** paused enter 90 ms / exit 60 ms captures now show real mid-frames distinct from endpoints (interior
sampled opaque #0d1012; no bleed); forced-fixture brightness-unsupported capture shows the reason + Open display settings
fallback; Quick Settings Airplane/Night light/Energy saver now carry external chevrons and “Open Windows settings for …”
labels, audio-output row is labelled, Wi-Fi/Bluetooth split-tile deviation is recorded in VISUAL_AUDIT_ISLAND E9.
Harness verified 18 files / 242 tests green. The only remaining task-56-era note is the recorded split-tile presentation
deviation (usable, no dead control).

**G5 overall remains CONDITIONAL on pre-existing evidence:** Media adapter and media/Wi-Fi/BT drill-down captures, and real
Windows/DPI/frame-pacing visual evidence (task-31/L0). These are not task-56 regressions. G4 status unchanged.

## 7. Task-62 native-signoff gate (2026-11-10)

User rejected the native island again: bleed/transparency, notification bars/scrollbar clutter, visual clutter, wrong
compact-first/Esc behavior, and empty-search hint. Binding added in `ISLAND_VISUAL_UX_FIX.md` §8 (also SCREENS §2.14,
ISLAND_V9_SPEC §13, KEYBOARD §13, GAP_LIST P0-62). No implementation is signed here.

**G5 status: BLOCKED for re-sign** until ui-launcher supplies native before/after captures over a white text window
(not browser mock) and the following pass review: (1) zero transparency inside the island bbox at 100/125/150% DPI; (2)
Alt+Space compact-only, typing/click expands, Esc drilldown->card->compact->hidden, no empty-search hint; (3) notifications
0/1/many with no bars/visible scrollbar overlap and header clear action; (4) reduced Quick Settings (volume/brightness/
audio + split Wi-Fi/Bluetooth + one quiet Open Windows settings link, no dead/unsupported control); (5) transform-first
card switch, native geometry call exactly once per transition (no per-frame resize), enter/exit 180/120 ms opaque frames.

**Current native before evidence (pre-fix) reviewed:** .bench/diag/island-after-click.png shows a click opening the Wi-Fi
drill-down instead of expanded Search; island-after-click-type.png shows the cluttered Quick Settings (fallback rows,
action pills, duplicate On/Off); v15-island-page.png shows the compact oval with placeholder hint. Exact deltas will be
listed when the native after set arrives; no sign-off yet.

