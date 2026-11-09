# ISLAND V9 — gap list vs the current app

**Status:** task-28 deliverable (publish). **Reference:** `ISLAND_V9_SPEC.md` + `ISLAND_SETTINGS_SPEC.md` +
captures under `docs/qa/island-reference/`. No code was changed.
**Summary:** the current app has the **native island shell and settings types**, but only a placeholder island surface
and **no V9 card system or island settings UI**. Closing P0/P1 gaps is the implementation work for ui-launcher (island
renderer/cards) and ui-app (settings pages), with L0 owning the native window/placement/shadow contract.

## P0 — blocks the island implementation

| # | Gap | Current evidence | Required |
|---|---|---|---|
| 1 | Island is a placeholder only: letter tile + "Search…" | `apps/desktop/src/island/IslandView.tsx:41–49`; `island.css:16–46` | Build the V9 card renderer (8 cards, compact/expanded, drill-downs, page dots) against `ISLAND_V9_SPEC.md`. |
| 2 | No island design-token/component layer | `styles/tokens.css` has no island surface/motion/dot tokens; app tokens are light/dark green | Add island-scoped tokens from spec §1.6; island components must not inherit app light/dark surfaces. |
| 3 | Native window would clip the V9 shadow | `src-tauri/src/main.rs:410–434` builds island at exact 520×66 surface with shadow(false) | Add transparent shadow margin or native shadow; keep surface bbox; placement must account for the margin. |
| 4 | No placement editor UI | bridge `listMonitors/showIsland/hideIsland/setIslandGeometry` + placement math `main.rs:140–190` exist; no screen uses them | Implement `ISLAND_SETTINGS_SPEC.md` Placement: monitor canvas, anchors, offsets, taskbar/safe-area, ghost, Apply/Revert/Test. |
| 5 | No card manager UI | `IslandSettings.cards/defaultCard` exist in `bridge/types.ts:248–297`; no UI | Implement ordered/enable/default/reset list + live dot preview + persistence. |
| 6 | No card data/adapters wired | V9 reference is simulated; app has no media/controls/clipboard/agent card data paths | Real Windows adapters per master prompt; honest empty/unsupported; never fake V9 sample rows. |

## P1 — fidelity and interaction

| # | Gap | Current evidence | Required |
|---|---|---|---|
| 7 | No page dots / compact overlay architecture | placeholder has no dots; V9 dots expanded/compact metrics in spec §1.5 | Implement 8-dot nav, active sync, click jump, card-order reflection, compact right overlay/padding. |
| 8 | Radius/type/icon scale not present | app radius 12/14; island needs 34/41/999, type 16/11/10/9, icons 19/23/18 stroke 1.85 | Island-only scale; don't globally change app tokens. |
| 9 | Brand mark mismatch | placeholder uses a "P" tile; `assets/brand` is two-tone; V9 uses 3-path #102e55/#3173a8/#dbe6ed at 27×27 | Use authoritative V9/Original P mark; reconcile brand assets without redrawing. |
| 10 | Motion differs / no interrupted-transition model | current launcher uses 150/110 ms; island spec 550/500/450/220 + 120/250 card swap | Add island motion tokens and latest-input-wins guard; reduced motion instant; profile layout. |
| 11 | No wheel/swipe/inner-scroll card switching | no card carousel | Implement outer wheel/swipe/Ctrl+↑↓ card switch; inner scroll never switches; dot sync. |
| 12 | Placement semantics mismatch | concept uses percent/clamped drag + offsets (x−50)×7/(y−12)×5; IPC uses DIP offsets, center anchor names | Editor converts to DIPs/anchors; monitor IDs/DPI real; taskbar/fullscreen/edge clamp. |
| 13 | Settings fields incomplete vs IPC/master | concept lacks showDots, hideOnFocusLoss, skipDisabled, autoFlipExpanded, pin/undo/test-overlay | Add to settings page (same control language) and persist all IPC fields; document pin gap if IPC lacks it. |
| 14 | Accessibility gaps | agent status 9 px; supporting text 10 px; dots visual 4–5 px in 11–14 px buttons; no island focus model | Keep visual but: status legible, dot hit areas ≥24×24, focus rings, region/card labels, reduced motion, Esc/arrow model; verify contrast. |
| 15 | No island settings route in app | app routes only Home/Indexes/Activity/Workflows/Preferences | Add Preferences subpage/route with concept frame (1170/218) or documented app-grid adaptation. |

## P2 — polish / verification

| # | Gap | Required |
|---|---|---|
| 16 | V9 stage/chrome could be mistaken for product | Only the island surface is binding; do not ship the demo bar/stage/background. |
| 17 | Icon set stroke/size mismatch with app icons | Build bounded 24 viewBox icons at 19/23/18 with stroke 1.85; no stretch. |
| 18 | No reference-vs-production comparison harness | Use `docs/qa/island-reference/` captures + `capture.mjs` for per-state diffs; capture real Windows at 125/150/200%. |
| 19 | Peak/height growth and shadow at 335/DPI | Verify compact 315 px card, clipboard 446 / controls 483, no horizontal overflow, shadow not clipped. |
| 20 | Dark/light treatment of settings | Reference settings is light; app theme may be dark. Define dark tokens for the settings surface or lock it to app light tokens; island itself stays graphite. |

## Evidence & ownership

- Captures: `docs/qa/island-reference/v9/<width>/<state>.png` + `*.metrics.json`; settings under
  `docs/qa/island-reference/settings/`; manifest `docs/qa/island-reference/manifest.json` (136 images).
- Specs: `ISLAND_V9_SPEC.md` (geometry/type/motion/cards/gaps), `ISLAND_SETTINGS_SPEC.md` (placement + card manager).
- Suggested ownership: **ui-launcher** = island surface/cards/page dots/interaction; **ui-app** = Preferences →
  Dynamic Island settings pages; **L0** = native window/transparent shadow margin, monitor/DPI placement, typed
  persistence; **sources/agent owners** = real card data adapters. No code in this task.
