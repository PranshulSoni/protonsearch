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
