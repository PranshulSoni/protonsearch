# ISLAND SETTINGS — placement editor + card manager spec

**Status:** FINAL reference extraction (task-28). **Owner:** L1-B. **Binding reference:**
`docs/reference/v10-handoff/protonsearch_island_settings_placement_concept.html` (interactive placement/card-order
concept) plus the master prompt §6.1/§6.2. This is the spec for the full desktop app's **Preferences → Dynamic Island**
surface. It is UI-only in the reference; production must persist through the typed IPC `IslandSettings` and apply real
monitor/taskbar/DPI logic.
**Evidence:** `docs/qa/island-reference/settings/` captures at 1240 and 860 (placement default, cards & order, behavior,
bottom-right anchor, external monitor) with `*.metrics.json`; reference PNGs
`settings_placement_preview.png` and `settings_cards_1240.png`.

## 1. Route and frame

- Route: **Preferences → Dynamic Island** (entry under LAUNCHER/dynamic island; sibling "Normal Launcher" and
  "Privacy"). Page title "Make it yours."; eyebrow "PREFERENCES / LAUNCHER"; lead "Choose where the island lives, how it
  behaves and which cards you see. The actual island keeps its V9 appearance."
- Frame: max-width **1170 px**, margin **25 px auto**, border 1 px `#d1d9d2`, radius **15 px**, overflow hidden,
  background `#fbfcf9`, min-height **748 px**, shadow `0 22px 60px rgba(50,70,60,.075)`;
  grid **218 px sidebar + 1fr main**.
- Sidebar: width **218**, bg `#f5f6f2`, right border `#dce2db`, padding **26 px 13 px**, gap 20; brand row
  15/780/-0.05em with 27×29 mark, gap 10, padding 0 12; group labels 10/800/+0.11em `#9aa5a0` margin
  **21 10 8**; nav buttons 12, padding **11 12**, radius 8, colour `#6e7b76`, gap 10, icon 17; active
  bg `#e2e9e6`, colour `#263e4a`, weight 700; footer 10/1.6 `#8b9a94` padding 12, top border `#dce2db`.
- Main: padding **31 px 37 px 25 px**. Eyebrow 10/800/+0.11em `#8c9993`. h1 **31/650/-0.06em**, margin 10 0 5.
  Lead 12/1.65 `#87928d`, margin 0 0 26.
- Tabs: gap 5, bottom border `#dce2db`; buttons padding **12 16**, 12/690 `#75817d`; active colour
  `#244f65` + 2 px `#315f77` bottom border. Labels: **Placement · Cards & order · Behavior**.

## 2. Shared controls

- Section header: strong 13/700; caption 10 `#8a9792`; row margin **20 0 12**, flex space-between.
- Form label: 11/720 `#6d7974`, margin-bottom 8. Select/number input: full width, padding **10 11**, radius 7,
  border `#d4dcd5`, background white, font 12, colour `#334a51`.
- Switch row: padding **13 0**, bottom border `#e7ebe6`; label 12/620, sub 10 `#9aa59f` margin-top 5; checkbox
  **17×17**, accent `#38677d`.
- Buttons: padding **10 14**, radius 7, 11/720, white + border `#d7ded8` ink `#415a60`; primary bg `#254c62`,
  white, border same; hover brightness .97. Footer row: top border `#dce2db`, margin-top 28, padding-top 19,
  flex space-between; toast 11 `#477464`, min-height 16. Buttons: **Revert · Test placement · Apply changes**
  (primary).
- Focus: 2 px `#639bbb` ring, offset 2 for buttons/inputs/selects.

## 3. Placement editor

- Monitor area: bg `#e5eae5`, border 1 px `#d2dcd4`, radius 11, padding 22, min-height 296; top row "DISPLAY
  PREVIEW" 11/700 `#6f7f79` + segmented **Built-in / External** (track border `#cbd6cd` radius 8 padding 3
  bg `#dce4dd`; buttons 11, padding 6 10, radius 5; active white + `0 1px 3px rgba(20,44,36,.075)`).
- Screen: width `min(100%, 524px)`, aspect-ratio **1.85** built-in / **1.6** external, centred; border
  **7 px** `#333d3f`, outline 1 px `#6d7973`, radius 11, background `#d9e0da`, shadow
  `0 4px 0 #7c8780, 0 15px 23px rgba(36,51,38,.106)`; decorative ellipse; taskbar at bottom 2, centred, gap 5,
  four 9×9 radius-2 `#a3b3aa` tiles. Do not draw a fake OS wallpaper; keep the neutral preview.
- Ghost island: **154×30 px**, radius **40**, bg `#171f23`, border `#3c5157`, padding 0 10, gap 8, shadow
  `0 9px 20px rgba(22,38,38,.28)`; mark 15×16; label 9/0.83 "Search anything…"; status dot 7 `#86a5b5`; cursor
  grab/grabbing. It is positioned by percentage: left/top with translate(-50%,-50%).
- Anchors: 3×3 preset grid, 3 columns gap 6, margin-top 8; each **height 34**, radius 7, bg `#f5f7f3`, border
  `#d0d9d1`; active bg `#e1eaf0` border `#7393a5` ink `#2c5b72`; inner bar 15×7 radius 3 currentColor.
  Anchor percentages: top-left **12/12**, top-center **50/12**, top-right **88/12**, middle-left **12/48**,
  middle-center **50/48**, middle-right **88/48**, bottom-left **12/83**, bottom-center **50/83**, bottom-right
  **88/83**. Labels and titles use human anchor names; active anchor announced via `aria-pressed` (buttons).
- Monitor behavior select options: **Primary monitor · Monitor under cursor · Active window's monitor · Selected
  specific monitor**.
- Offsets: 2-col grid gap 8; Horizontal/Vertical number inputs min **-500**, max **500**, step 1; prototype derives
  offset from drag: `x = round((left% − 50) × 7)`, `y = round((top% − 12) × 5)` and clamps left 15–85%, top 10–87%.
  Production must store **DIPs** relative to the selected monitor anchor/work area (IPC `offsetX/offsetY`), not
  percentages; convert only in the editor view.
- Toggles: **Keep inside work area** (avoid taskbar/edges) and **Auto-flip expanded cards** (prevent tall cards from
  clipping), both default on, with subcopy.
- Footer hint: "Preview only · position is constrained inside the selected monitor."
- Interaction: pointer drag on ghost (pointer capture), keyboard arrows step **1%** per press on focused ghost
  (`role="slider"`, tabindex 0, label "Island position, drag or use arrow keys"); anchor click snaps; offset input
  updates position; monitor segmented updates aspect. **Test placement** shows a temporary real overlay with visible
  Revert (production); **Apply changes** persists; **Revert** restores committed values. Never trap the island
  off-screen; validate/clamp on apply and on monitor removal.
- Production mapping: `IslandPlacement` {monitor, anchor, offsetX, offsetY, keepInWorkArea, autoFlipExpanded}; native
  placement math already exists in `src-tauri/src/main.rs:140–190`; add monitor enumeration/work area/DPI via
  `list_monitors`; anchor enum uses centre names (Center/CenterLeft/...) not "middle".

## 4. Cards & order

- Header: "Card order" + **Reset to V9** button (secondary, 11).
- Help: 11/1.6 `#84928b` margin-top 16: "Move cards up or down, enable only those you use, and pick the default.
  Dots, wheel switching and swipe order follow this list."
- List: border `#e0e5df` radius 9 margin-top 17; rows **min-height 50**, padding **8 12**, gap 10, white,
  bottom rule `#ecf0eb`; disabled row opacity **.48**. Row: reorder column (two 13 px arrow buttons, gap 2,
  padding 1 5, ink `#798c87) · icon tile **30×30** radius 6 bg `#edf1ed` border `#e2e7e2`, glyph 15 ink
  `#58727a` · title 12/670 · DEFAULT tag (9/780, padding 4 7, radius 5, border `#dae5e1` bg `#f4f8f5`
  ink `#668073`) · **Set default** button (padding 6 8, 11) · enable checkbox 16 (accent `#376e88`; Search is
  required, disabled+checked).
- Card names/icons in V9 order: **Search · Media · Clipboard · Agents · Quick Settings · Browser · Git ·
  Notifications** (ids `search, media, clipboard, agent, controls, browser, git, bell`). No fake cards; only these
  eight.
- Dot preview: cardhelp + panel padding 13 bg `#e6ece7` border `#dce4db` radius 9 margin-top 18; mini island
  **height 70**, radius 17, bg `#1a2227` border `#34464f`, gap 18, spans 10; dots row gap 6 min-height 28;
  dot 6×6 radius 5 `#aabbb6`; active **22×6** `#3d6f84`. Shows "Default · {card}" and one dot per enabled card in
  order; updates immediately on reorder/hide.
- Rules: reorder does not reorder search sources; stable ids; persist ordered IDs + enabled flags separately;
  reject unknown/duplicate ids; default falls back to Search if hidden; Search always enabled; **Reset to V9** restores
  V9 order, all enabled, Search default; undo/apply production (not in prototype).
- Keyboard: each row's up/down buttons are real buttons with accessible labels "Move {card} up/down"; Set default
  named; checkbox labelled "Enable {card}"; add drag handle as an accessible move control (master §6.2); focus order
  follows list. Reordering must not steal focus from the moved row.
- Production fields: `cards` ordered array of {id, enabled}; `defaultCard`; optional pin (master says pin, IPC has
  no pin field yet — add or defer with a gap note); `showDots`, `skipDisabled`, `returnToSearch`,
  `hideOnFocusLoss`, `autoFlipExpanded`.

## 5. Behavior

Reference Behavior tab: three switch rows — **Dynamic Island enabled** ("Keep the alternate launcher available"),
**Return to Search on next opening** ("Otherwise remember last active card"), **Follow Windows reduced-motion
preference** ("Preserve V9 animation for standard motion"); select **Primary launcher when pressing Alt + Space**
(Dynamic Island / Normal Launcher). Production also exposes `showDots`, `skipDisabled`, `hideOnFocusLoss` and
card-manager behaviors from IPC; add them here (same switch-row metrics) rather than hiding them. Copy must be honest:
settings are visual; native storage/shortcuts/window placement are implemented by the shell.

## 6. Responsive

- ≤800: frame full-bleed (no margin, radius 0, min-height 100vh), grid **56 px rail + 1fr**; sidebar padding 16 6,
  nav centred, labels/group/sidefoot hidden, brand centred; main padding 25 17; controls one column; h1 27.
- ≤480: main padding 20 12; monitorarea padding 11; monitors wrap; h1 24; screen border 5 px; ghost **102×24**,
  ghost label 7; section header aligns top; tabs padding 11 10; card rows gap 7 padding 7; card title 11; DEFAULT
  tag hidden; screen hint wraps.
- High-DPI: CSS px scale; verify 125/150/200% against captures; no horizontal overflow.

## 7. States and errors

- No monitors / enumeration failed: quiet state in monitor area, "Couldn't read your displays." + Try again; never fake
  one monitor.
- Monitor disconnected: clamp to primary, keep user's preferred monitor id, show a quiet warning; do not lose settings.
- Invalid offsets: clamp to work area on apply; show helper "Position adjusted to stay inside the work area."
- Test placement: temporary overlay + Revert; auto-recover on lost monitor/out-of-bounds; never traps user.
- Apply: success toast/stamp "Saved"; failure quiet "Couldn't save that change." / "Your previous placement is still
  active." / Try again. Revert restores last committed snapshot.
- Persistence: atomic typed settings update; migration for old config; card ids/enum validation; default fallback;
  restart persistence; no dead controls.
- Accessibility: 2 px focus ring `#639bbb`; ghost is a slider with arrows; anchors 34 px targets;
  card rows keyboard operable; reduced-motion respects system; screen reader announces position via percentage/offset
  and active anchor; no colour-only state.

## 8. Paste-ready metrics tokens (indented CSS)

    :root {
      --set-frame-w:1170px; --set-sidebar-w:218px; --set-frame-radius:15px; --set-frame-min-h:748px;
      --set-main-pad:31px 37px 25px; --set-h1:31px; --set-lead:12px;
      --set-monitor-bg:#e5eae5; --set-monitor-border:#d2dcd4; --set-monitor-radius:11px;
      --set-screen-border:7px; --set-screen-radius:11px;
      --set-ghost-w:154px; --set-ghost-h:30px; --set-ghost-radius:40px;
      --set-anchor-h:34px; --set-cardrow-h:50px; --set-button-pad:10px 14px;
      --set-focus:#639bbb; --set-primary:#254c62; --set-ink:#24343a; --set-muted:#7c8988;
    }

## 9. Gap list vs the current app settings

- No Dynamic Island settings route/tab exists (App routes: Home/Indexes/Activity/Workflows/Preferences); add a
  Preferences subpage or new route with the concept's frame/tabs.
- Current Preferences uses 232/280 app grid and light/dark app tokens; the concept is a 1170 px/218 px frame with its
  own palette and 31 px h1. Keep the concept for this surface (or explicitly adapt to app grid; do not silently mix).
- Bridge has `IslandSettings`, `listMonitors`, show/hide/setIslandGeometry; native placement math exists — but no
  UI uses it. The placement preview/card manager screens are entirely missing.
- Concept's percentage/px offsets vs IPC DIPs and anchor naming must be reconciled (documented in §3).
- Concept lacks production fields (showDots, hideOnFocusLoss, skipDisabled, autoFlipExpanded, pin/undo); add them.
- Prototype does not persist, test the real overlay, or handle monitor removal; production must.
- No focus/keyboard/drag parity yet; add in implementation.

## 10. Acceptance checklist

- Frame/sidebar/header/tabs/monitorarea/screen/ghost metrics match §1–§3 within 1 px at 1240.
- 3×3 anchors, drag, arrow keys, offsets, monitor switching, taskbar/edge clamp, ghost preview.
- Test placement/Revert/Apply, persistence across restart, failure states.
- Card reorder/enable/default/reset, dot preview live, keyboard operable, Search always enabled.
- Behavior toggles and primary launcher mode persisted; no dead controls.
- Responsive ≤800/≤480, 125/150/200%, reduced motion, dark theme (if app applies; reference is light).
- No fake data, no dead controls, real monitor/DPI/taskbar logic.
