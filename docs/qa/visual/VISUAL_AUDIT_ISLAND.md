# ISLAND V9 renderer — implementation audit

**Date:** 2026-11-10. **Task:** G5 / task-41 (owner ui-launcher). **Binding:**
`docs/design/ISLAND_V9_SPEC.md` + `ISLAND_V9_GAP_LIST.md` + `docs/qa/island-reference/` (136 captures).
**Scope shipped:** `apps/desktop/src/island/**` only. No demo stage/bar; only the island surface binds.

## 1. What shipped

- **Surface:** V9 graphite surface replacing the 520×66 placeholder. Compact search capsule (520×66, radius 999)
  and expanded cards with the 31 px page-dot footer (radii 34 default / 41 media). Tokens, type scale, icon sizes
  (24 viewBox, stroke 1.85), 1 px border and the two-layer shadow are island-scoped so app light/dark themes never
  bleed into the island.
- **Geometry contract:** `islandCardDims()` implements the §2 card table (width `min(nominal, stage−20)`, captured
  widths 500/410/315 at stage 520/430/335) and the expanded search height `185 + 53×min(matches,5)`. The renderer
  measures the SURFACE only and calls `setIslandGeometry(surfaceW, surfaceH)`; native `main.rs` already adds
  `ISLAND_SHADOW_PAD = 96` around it, so the shadow has transparent room.
- **Cards (8):**
  - Search — compact/expanded, filters All/Files/Folders/Apps, real `api.search` rows/count, ↑/↓ + Enter selection,
    per-row icon/metadata, honest pending/error/empty states, footer with real "{n} local results".
  - Clipboard — real `api.search` scope clipboard (rows filtered to clipboard kind), tabs All/Text/Images/Links/
    Code/Files/Pinned with real client classification, snippet panel, honest empty; Pinned is visibly disabled with a
    reason because pinning is not in the bridge yet.
  - Agents — real `agentStatus()` + `detectAgents()`; configured/not-configured states, available agents list.
    Task hierarchy/percentage/logs are not in the frozen bridge, so those controls are disabled with an explicit
    "appears once the agent task API lands" note (no fake statuses).
  - Browser — real scoped search rows split into bookmarks/saved pages and recent history caps.
  - Git — real scoped search rows for recent commits/repositories.
  - Media / Quick Settings / Notifications — capability-honest disabled cards with the adapter reason and
    "No mock data is shown." per the Lead's instruction until native adapters exist.
- **Dots:** enabled cards in saved order, expanded 31 px footer / compact overlay, aria-current + per-card
  "Go to {card} card" labels, hover scale, click jumps and syncs; hidden when `showDots === false`. Expanded
  buttons are a 24×30 hit target; compact buttons keep the reference 11×37 visual box (documented deviation: at
  315 px surface width a 24 px hit box would overlap the search input/dots, so compact uses vertical hit padding).
- **Interaction/motion:** card morph CSS tokens 550 ms width/height, 500 ms radius, 450 ms transform, 220 ms opacity,
  cubic-bezier(.22,1,.36,1); card out 140 ms → swap → in 250 ms; latest-token-wins guard; reduced motion instant.
  Wheel (48 px / 500 ms guard) and horizontal swipe switch cards on the outer surface only; inner scrollables
  consume their own wheel; Ctrl+↑/↓ switches; switch lock 350 ms; Esc = drilldown → expanded → compact search →
  hide (frontend close animation then `hideIsland`); `island:shown` resets to the configured default and refocuses
  search after 190 ms; `island:dismiss` plays the close path.
- **Branding:** official P mark at 27 px with island-scoped V9 blue/light facet; no hand-drawn letter tile.
- **Tests:** `src/island/island.test.tsx` covers the authoritative dimension table, expanded height, work-area clamp,
  saved-order dots, compact capsule + 8 dots/aria-current, focus-to-expand search with real filters/results, dot card
  switching, clipboard tabs incl. disabled Pinned, capability-honest media card, Ctrl+↑/↓ and wheel switching, Esc
  chain, and reduced-motion instant swaps. Total app suite: **16 files / 230 tests passed**; `npm run build` exit 0.

## 2. Screenshots (browser mock; capture-time clipboard/browser/git fixtures, no fake data in product code)

Directory: `docs/qa/visual/screenshots/island-v9/`

- Compact: `compact-1150/860/520/430/335.png` (520×66 → 315×66; dots overlay + search padding-right).
- Search expanded: `search-1150/520/335.png` (550/500/315 wide; 4 real results → 397 px, foot visible; filters,
  keyboard selection).
- Clipboard: `clipboard-1150/520/335.png` (510×391 / 500×416 / 315×446; 7 tabs, 48 px rows, mono snippet, real
  categories; tabs wrap at 335 per §2).
- Agents: `agents-1150/520/335.png` (honest not-configured state on this machine).
- Cards at 1150: `media-1150.png` (490×286, radius 41), `controls-1150.png` (515×376),
  `browser-1150.png` (500×361), `git-1150.png` (505×330), `notifications-1150.png` (500×345) — the
  adapter-less cards show explicit disabled states rather than V9 sample rows.
- Reference comparison: computed surface boxes match the manifest table exactly at DPR 1; element screenshots can
  read ±1 px from the 1 px border. Visual hierarchy (head/divider/context caps/rows/foot/dots), token palette, type
  scale, dot metrics and responsive breakpoints follow §1–§3. Pixel-identical comparison is intentionally limited for
  cards that must stay disabled until native adapters land.

## 3. Known gaps / follow-ups

1. Native Media, Quick Settings and Notifications adapters (task-35) — cards render honest disabled states, not V9
   sample controls/drilldowns (media output, Wi-Fi/BT drilldown, notifications 0/1/many).
2. Agent task hierarchy/session APIs ("agent_tasks"/"agent_task_tree") are not in the frozen bridge; the Agent card
   shows real runtime/agent detection and explicitly disabled actions. Wire when Lead exposes the APIs.
3. Clipboard image thumbnails/pinning depend on Gate-2 image clipboard IPC; Pinned tab is disabled honestly.
4. Search preview hookup is G2 (task-37/38); the island search card currently opens results only.
5. Native Windows visual run (real window/DPI/taskbar/fullscreen, 125/150/200%) and frame pacing are L0/task-31;
   this pass is Chromium/mock evidence. Compact dot hit area deviation documented above.
6. Motion was verified by CSS token inspection and the reduced-motion test; formal 550/500/450/220 ms timing
   evidence belongs to the performance harness (task-31).
