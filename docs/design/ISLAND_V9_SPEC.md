# ISLAND V9 — binding geometry, typography, motion and responsive spec

**Status:** FINAL reference extraction (task-28). **Owner:** L1-B UI/Motion/Product Design.
**Binding source:** `docs/reference/v10-handoff/protonsearch_dynamic_island_cards_v9_polished.html`. **Evidence:** 136
Playwright captures under `docs/qa/island-reference/` (24 states × 1150/860/520/430/335 + 6 settings captures),
`manifest.json` + per-state `*.metrics.json`. Regenerate: `docs/qa/island-reference/capture.mjs`.
**Rule:** V9 is binding. Prototype sample data/handlers are not production data; a browser interaction is not Windows
integration evidence.

## 1. Tokens

### 1.1 Colour (island-scoped; never the app's light/dark surfaces)

| Token | Value | Use |
|---|---|---|
| `--island-panel` | #0d1012 | island surface |
| `--island-border` | #2e3538 | 1 px edge |
| `--island-text` | #f6f7f8 | primary text |
| `--island-muted` | #9ba3ae | subtitle/placeholder |
| `--island-muted-2` | #7d878d / #8b9aa3 | context/meta |
| `--island-accent` | #98bbdb | restrained accent |
| `--island-dot-idle` | #657176 | inactive dot |
| `--island-dot-active` | #c7dce7 | active dot |
| `--island-footer` | #101416 | dots footer |
| `--island-footer-border` | #262c30 | footer hairline |
| `--island-row-hover` | #202d35 | hover/active row |
| `--island-row-border` | #334b5b | active row border |
| `--island-control-bg` | #171e23 | setting tile |
| `--island-control-border` | #34434a | tile edge |
| `--island-control-on` | #223a49 / #416178 | enabled tile |
| `--island-raised` | #1c2931 | card-top icon tile |
| `--island-icon` | #bad2df | section icon |
| `--island-track` | #27343d | progress track |
| `--island-track-fill` | #a6c7d8 | progress fill |
| `--island-focus` | #92bcd0 (dots), #9abedd (inputs) | focus ring |

### 1.2 Typography (Inter/Segoe UI Variable/Segoe UI/system-ui; base 13 px, tracking -0.015em)

| Element | Size / line-height | Weight | Colour |
|---|---|---|---|
| Search input | 16 / 1.2, -0.035em; 13 ≤520; 12 ≤380 | 400 | #f6f7f8; placeholder #9ba0a4 |
| Card h2 | 16 / 19.2, -0.04em; 15 ≤520 | 700 | #f6f7f8 |
| Card subtitle | 11 / 15.95 | 400 | #9ba7ae, margin-top 4 |
| Context caps | 10 / 1.2, +0.09em, uppercase | 760 | #aab4b8 |
| Result title | 12 | 400/700 | #e8ebee |
| Result sub | 10, margin-top 4 | 400 | #99a6ad |
| Result footer | 10 | 400 | #87979c; kbd #d2dce1 |
| Filter | 11; padding 7 8; min-height 29 | 400 | #a5adb0; chosen #e3f0f7 |
| Inline nav chip | 10; padding 7 9; min-height 29 | 400 | #9daab0; active #d4e8f4 |
| Clip title / sub | 11.5 / 10 | 700 / 400 | #eef1f3 / #8a9aa4 |
| Snippet | 10.5 / 1.55 mono | 400 | #b8cbd4 |
| Choose row title / meta | 12 / 10 | 570 / 400 | #e9edf1 / #8b9aa3 |
| Setting tile label / sub | 11 (10 ≤520) / 10 | 400 | #d6e0e3 / #a5b5be |
| Agent status | 9; padding 4 7; radius 5 | 400 | #b9cdd6 |
| Agent button | 11; min-height 33 | 400 | #d7e8ee |
| Notification title / body | 12 / 10 / 1.4 | 400 | #e9edf1 / #97a6ae |
| Song / artist | 19 / 11 | 400 | #f6f7f8 / #9eb7d1 |

Essential status text at 9 px must not ship unreadable in production: keep the visual badge but raise essential status or
pair word+shape; document the deviation. Dots are visual 5/4 px; expanded buttons keep the V9 **14×30 layout + 5 px gap**
with a centred invisible **24×30 hit pseudo-element**; compact keeps **11×37** (documented WCAG deviation, because 24 px
would overlap the input at 315 px). Keyboard/wheel/swipe alternatives and a showDots opt-out exist.

### 1.3 Radii, border, shadow

Radius: **34** default, **999** compact search, **41** media, 12 big icon tile, 10 setting tile, 8 row, 7 small
controls. Border 1 px `#2e3538`. Shadow union:
`0 26px 54px rgba(20,32,40,.25), 0 3px 8px rgba(27,32,32,.15)`. Shadow extent max is **107 px**; the native host now
uses `ISLAND_SHADOW_PAD = 112` (`src-tauri/src/main.rs:34`), so the shadow fits with 5 px headroom on every side.
Surface geometry is unchanged and the window adds the transparent margin.

### 1.4 Icons

Bounded inline SVG 0 0 24 24, `fill:none`, stroke currentColor, **stroke-width 1.85**, round caps/joins. Sizes:
**19** default, **23** card-top, **18** row/choice/clip, **20** transport (primary 24), **11** output chevron,
**16** expand/back, **15** notification dismiss, **14** agent button. Never stretch/upscale.

### 1.5 Page dots

| Metric | Expanded | Compact |
|---|---|---|
| Zone height | 31 px footer | 38 px overlay |
| Dot | 5 px; active 16×5 | 4 px; active 6×4 |
| Gap | 5 | 1 |
| Button | layout 14×30 (padding 9 3) + invisible 24×30 hit pseudo | 11×37, min-width 10, padding 11 0 (documented a11y deviation) |
| Nav | bottom 0, full width, bg #101416, top border #262c30 | top 14, right 15 (11 ≤520), transparent |
| Active colour | #c7dce7; idle #657176 | same |

Dots: 8 in card order, `aria-current="page"`, labels "Go to {card} card", hover scale 1.12, focus 2 px
`#92bcd0` offset -3. In compact, search row padding-right 124 (112 ≤520) and loupe hidden.

### 1.6 Paste-ready tokens (indented CSS block)

    :root {
      --island-panel:#0d1012; --island-border:#2e3538; --island-text:#f6f7f8;
      --island-muted:#9ba3ae; --island-accent:#98bbdb;
      --island-dot-idle:#657176; --island-dot-active:#c7dce7;
      --island-footer:#101416; --island-footer-border:#262c30;
      --island-row-hover:#202d35; --island-row-border:#334b5b;
      --island-control-bg:#171e23; --island-control-border:#34434a;
      --island-control-on:#223a49; --island-control-on-border:#416178;
      --island-raised:#1c2931; --island-icon:#bad2df;
      --island-track:#27343d; --island-track-fill:#a6c7d8;
      --island-focus:#92bcd0; --island-focus-input:#9abedd;
      --island-radius:34px; --island-radius-compact:999px; --island-radius-media:41px;
      --island-ease:cubic-bezier(.22,1,.36,1);
      --island-dur-morph:550ms; --island-dur-radius:500ms; --island-dur-transform:450ms;
      --island-dur-out:140ms; --island-dur-in:250ms; --island-dot-dur:300ms;
      --island-dots-h:31px; --island-dots-h-compact:38px;
    }

## 2. Geometry

Shell: reference stage height 590 (600 ≤520), island `top:22px`, `left:50%`, translateX(-50%).
Width `min(var(--w), calc(100% - 18px))`. Expanded cards reserve a **31 px footer**; inner height is
`calc(100% - 31px)`. Compact search: 520×66, radius 999, inner 100%. Base radius 34; media 41.

**Card dimensions (authoritative `cardDims()`).** Actual width = `min(nominal, stageInner - 18)`; captured widths at
520/430/335 are 500/410/315 for every card (stage clamp). Heights:

| Card id | Nominal W | H >520 | H ≤520 | H ≤380 |
|---|---|---|---|---|
| search idle | 520 | 66 | 66 | 66 |
| search expanded | 550 | `185 + 53×min(matches,5)` max 460 | same | same |
| media | 490 | 286 | 286 | 286 |
| media output | 490 | 342 | 365 | 365 |
| clipboard | 510 | 391 | 416 | 446 |
| agent | 520 | 411 | 445 | 445 |
| controls | 515 | 376 | 400 | 483 |
| controls Wi-Fi/BT | 515 | 366 | 418 | 418 |
| browser | 500 | 361 | 408 | 408 |
| git | 505 | 330 | 355 | 355 |
| notifications | 500 | 345 | 385 | 385 |

Search expanded observed: 0 results 185, 3 results 344, 5 results 450. Production keeps these visual targets but
derives/clamps from real content and the monitor work area; never overflow.

**Responsive.** Exactly two breakpoints: **520** and **380**.
≤520: stage 600; card body 15/15/12; h2 15; bigIcon 36; media body 16 18 12, top gap 11, cover 60, song 16; setting
tile 58, toggle 8, expand 27, label 10; search padding-left 12, compact right 112, dots right 11; result foot second
hint hidden; choice meta max 78 ellipsis.
≤380: body 14/13/11; inline nav wraps (row-gap 4, no scroll); setting grid 1 column gap 6, tile 50, toggle 7 12,
label 11; media body 14 13 10; transport gap 0 buttons min 32; search input 12. Width never exceeds stage/work area
−18; no horizontal page scroll.

## 3. Shared structure

Search row: height **65**, padding **0 18** (left 12 ≤520), gap 12 (9 ≤520); P mark 27×27; input flex, no chrome;
loupe 20, hidden compact. Expanded: `searchExpanded` height inner−65, border-top #24282b.
Filters: padding 7 10, gap 3, no wrap/scroll; chip padding 7 8 min-height 29 radius 6, chosen bg #263b49 ink #e3f0f7;
labels All/Files/Folders/Apps/Actions (prototype; production real scopes, no fake categories).
Results: padding 7 11, max-height 215; row min-height **50**, padding 8 10 gap 10 radius 8; icon 34 radius 7 bg
#253744, 18 glyph; title 12 bold ellipsis, sub 10 #99a6ad margin 4; active/hover #202d35 + border #334b5b.
Result foot: min-height 33, padding 8 15 9, border-top #2c3034, font 10; "{n} local results" + kbd hints (second
hidden ≤520). Empty prototype message "No matching sample results." — production must render a real empty state, and at the canonical 185 px card it must be a **compact visible 1–2 line state** inside the results area (title 11/600, hint 10, no 110 px min-height). The prototype's blank zero-results capture is a reference defect; never ship a blank state.
Card body: padding **17 22 14**, overflow-y auto, thin scrollbar #3c474a; card top min-height 42 gap 11; icon tile
40×40 radius 12 bg #1c2931, icon 23; h2 16/700; sub 11 margin-top 4; divider #262b30 margin 13. No card counter.

## 4. Cards

All cards share §3. Data below is V9 sample; production replaces with real sources and honest states.

**Search:** compact 520×66; expanded 550×185–460; filters/results/foot above. Sample 5 rows (Architecture notes.docx,
Documents/Nova/Product, Files; search-architecture.md, Projects/ProtonSearch, Files; Architecture, DIR,
Projects/Research, Folders; Visual Studio Code, APP; Open project workspace, RUN, Actions). Typing expands; filters;
↑/↓ moves; Enter acts. Real data/counts only.

**Media:** 490×286 (output 490×342, ≤520 365), radius 41. Body padding 20 23 15 column space-between gap 8. Top
min-height 70 gap 14: cover 73×73 radius 13 bg #294d6a with "NO." + ring; song 19/1.2, artist 11 #9eb7d1 margin 6;
sound bars 22×28, four 3 px bars heights 7/19/12/9 #a4c8da, 900 ms alternate, paused freezes. Seek margin 7 gap 10,
mono 11 tabular, range height 16. Transport min-height 44 gap 4; buttons height 42 flex 1 (primary 1.15), icons 20
(primary 24); shuffle/previous/play-pause/next/repeat/output; output min 49 + 11 chevron; active #a4c4e0; hover
#25292e. Output drill-down: back 31; 3 choose rows 47 (Laptop speakers / Wireless headphones / External display),
icon 18, title 12, meta 10, tail check/more; selecting returns. Real media/output APIs only.

**Clipboard:** 510×391 (416 ≤520, 446 ≤380). Head; divider margin 12 0 7; inline nav 7 tabs **All/Text/Images/Links/
Code/Files/Pinned**, gap 4, buttons min-height 29 padding 7 9 radius 7, active bg #1c303c border #304958 ink #d4e8f4;
no tab counts; wraps at ≤380. List max-height 144; clip row 48 fixed, padding 7 8 gap 10, icon 34, title 11.5/700, sub 10
neutral sentence-case "Type · {relative time}"; selected check. Image rows use a real 34 px `meta.thumbnailDataUrl`
thumbnail (object-cover) when present. Snippet max-height 95 min 48, margin-top 8, padding 10 12 radius 8, mono 10.5/1.55,
pre-wrap/break-anywhere, scroll. Pinned tab is **disabled honestly** until pinning IPC lands; no fake pin state. Real Win32
formats/thumbnails/previews.

**Agents:** 520×411 (445 ≤520). Head; divider margin 11; workspace row caps 10 "CURRENT WORKSPACE · n AGENTS" +
Subagent button 33 high padding 8 11 radius 7 icon 14; list max-height 150 scroll; rows 48 with agent icon 34, name 12,
task 10 ellipsis, status badge 9 (border #354c59, padding 4 7 radius 5); selected row like clip active. Actions margin-
top 10 gap 7: Pause/Resume + Details (icon 14); actions are gated by `task.controls` (a disabled action must read as
disabled); progress % right when determinate; track 4 bg #27343d fill #a6c7d8. Implemented v1: flat Pi tasks from
`api.agentTasks(50)` with real state/progress/permissions/log summary, no fabricated tree; hierarchy/agent-task-tree and
log IPC remain gaps. Pluralise counts ("1 TASK", "1 permission"); never show both Pause and Resume as enabled.

**Controls:** 515×376 (400 ≤520, 483 ≤380); drill-down 515×366 (418 ≤520). Real v1 data: Volume 62 %, Brightness 80 %, audio-output row + toggles via shell controls API. The fixed V9 height must show the 2×2 tile grid and brightness/volume without clipping; the audio-output row belongs to the Media output drill-down (or a dedicated Sound drill-down), not the first paint. Head; divider margin 12; grid 2 col gap 9;
tile 61 (58 ≤520, 50 ≤380) radius 10 bg #171e23 border #34434a; on bg #223a49 border #416178. Left toggle flex 1,
padding 9 11 (8 ≤520, 7 12 ≤380), gap 9, icon 19, label 11 (10 ≤520), sub 10 margin 5; right expand 36 (27 ≤520),
border-left #4b606c, chevron 16; only Wi-Fi/Bluetooth expand. Sliders margin-top 17 gap 12 min-height 25; icon 19,
label width 68, output width 34 tabular 10. Tiles Wi-Fi on, Bluetooth on, Airplane off, Energy saver off; brightness 70,
volume 50. Drill-down Wi-Fi/BT: head icon changes, back 31, rows 47 with icon 18 title 12 meta 10 tail more; 3 devices.
Split toggle vs chevron; real APIs/permissions; unsupported states honest.

**Browser:** 500×361 (408 ≤520). Head, divider, caps context BOOKMARKS/Saved pages and RECENT HISTORY/Local (10 caps
tracking); 2 bookmark rows + 1 history row; choose rows 47, icon 18, title 12, meta 10, tail more. Real adapters/open.

**Git:** 505×330 (355 ≤520). Head, divider, 3 commit rows (Optimize metadata search / 2 hours ago; Improve OCR
scheduling / Yesterday; Refine window animations / Monday) with git icon 18, title 12, meta 10, tail more. Real repo.

**Notifications:** 500×345 (385 ≤520). Real v1 list via shell notifications API. Head, divider; rows min 47, bell icon 18, title 12, body 10/1.4; dismiss
30×30 icon 15 radius 6 hover #293941. Dismiss may be disabled pending a bridge command — show the disabled reason, never a
silent dead ×; test-only controls (e.g. Test local toast) must be gated out of production. Sample 3; dismiss removes; 0
state centred "All caught up" margin-top 25. Real local + supported Windows sources only.

**Task-49 recheck findings (open):** zero-results copy invisible at 185 px; controls overflow cuts the Wi-Fi/Bluetooth
row; notifications test-only toast + disabled dismiss styling; agents-1150/520/335 captures stale. Fix before calling G5
fully accepted; G5 remains conditional.

## 5. Motion

| Motion | Value |
|---|---|
| island morph | width/height **550 ms**, radius **500 ms**, transform 450 ms, opacity 220 ms |
| easing | `cubic-bezier(.22, 1, .36, 1)` |
| close | opacity 200 ms; translate(-50%,-110px) scale(.88) 450 ms; visibility hidden after 450 |
| card out | opacity 140 ms, translateY(-11px) 190 ms |
| card in | keyframe 250 ms, translateY(11→0) + fade |
| render swap | 120 ms after out, then replace + in |
| page dot | background 200 ms, width 300 ms, transform 220 ms, hover scale 1.12 |
| sound bars | 900 ms alternate infinite; paused freezes |
| toast | 200 ms |
| guards | switch lock 350 ms; wheel 500 ms / 48 px; Ctrl+↑/↓ switches; focus search 190 ms after show |

Latest input wins; never queue stale transitions (paint-version guard). Inner scroll stays inside; only outer
wheel/swipe switches cards. Reduced motion: all transitions/animations 0; state changes instant; no layout jump.

## 6. Production derivation (hard-coded → derive)

| Prototype hard-code | Production requirement |
|---|---|
| Fixed card W/H | Keep visual target; derive from content, clamp to work area |
| Sample tabs/results/counts | Real sources and counts; no fake data |
| Fake media/duration/artwork | Real Windows media sessions and output endpoints |
| Clipboard sample rows + Pinned=Code alias | Real formats/thumbnails/categories/pinning; v1 uses real rows/thumbnailDataUrl and an honest disabled Pinned tab |
| Agent sample statuses/%/spawn | Real hierarchy/status/logs; indeterminate when unknown; v1 flat Pi tasks + honest disabled hierarchy/log controls |
| Simulated Wi-Fi/Bluetooth | Real APIs, permissions, unsupported states, split toggle/chevron |
| Browser/Git/notification samples | Real adapters, honest availability |
| Always 8 dots | Render enabled cards in saved order; dots reflect order |
| V9 stage bar/background | Not product chrome; only surface binds |
| CSS width/height animation | Keep visual timing; profile layout; latest input wins |
| Tiny dot hit areas | Expanded: 24×30 hit pseudo behind 14 px layout; compact: 11×37 documented deviation |
| Shadow clipped at stage | Native window transparent margin/native shadow; surface bbox unchanged |

## 7. Accessibility

Island region `aria-label="ProtonSearch island"`; card labelled by card title; dots nav "Jump to island card" with
per-card labels and `aria-current`; rows are options/buttons, active row announced; scrollable regions tabbable only
when actually scrollable; focus restored to invoking app on hide; Escape predictable (deep view → card → hide);
media/agent status polite live regions, no per-second chatter; reduced motion honoured including Windows setting;
targets ≥24×24 (dots need invisible padding); contrast: V9 muted #9ba3ae on #0d1012 ≈ 7.8:1, small 10 px text still
needs verification; agent 9 px status flagged. No focus traps in the non-modal island; keyboard: Tab/Shift+Tab, arrows,
Enter/Space, Esc, Ctrl+↑/↓, Home/End where applicable.

## 8. Acceptance checklist

- Captures match V9 at 1150/860/520/430/335 for all manifest states.
- Island boxes match §2 ±1 px at DPR 1 (or derived equivalent documented).
- Colours/radius/border/shadow/dot metrics match §1.
- Search, media, clipboard (all tabs), agent hierarchy, controls + Wi-Fi/BT/output drilldowns, browser, git,
  notifications 0/1/many, reduced motion all render.
- 550/500/450/220 ms motion and 120/250 card swap; latest-input-wins; reduced-motion instant.
- Inner scroll never switches cards; dots/wheel/swipe/Ctrl keys sync active state.
- No fake values; real data or honest empty/unsupported.
- Real Windows host: transparent no-flash window, shadow visible, placement clamp, DPI/taskbar/fullscreen safe.

## 9. Gap list

See `docs/design/ISLAND_V9_GAP_LIST.md` (updated post-task-41). Renderer v1 now exists in `apps/desktop/src/island/**`
with all eight cards, real search/clipboard/browser/git, flat agent tasks, dots/motion and the 112 px shadow pad; the
remaining gaps are native Media/Quick Settings/Notifications adapters (task-35), agent task-tree/log IPC, clipboard
pinning IPC, compact 11 px dot WCAG deviation, real Windows/DPI/frame-pacing evidence, and the copy nits in §10.

## 10. Implementation status — task-41 renderer v1 (2026-11-10)

**Landed:** V9 graphite surface/cards/dots, all card dimensions/radii from §2, tokens/type/icons, motion tokens and
reduced-motion swap; search real results/filters (All/Files/Folders/Apps/Actions) with borderless input and row-only
path; clipboard real categories/snippets/images with disabled Pinned; agents flat real Pi tasks with gated actions;
browser/git real scoped sources; Media/Quick Settings/Notifications honest disabled states; shadow pad 112.
**Design-review deltas resolved:** dots 14+5 pitch with 24×30 hit, input box removed, placeholder/Actions/recency, clipboard
counts/copy/thumbnail, V9 card subtitles, browser two-end context/chevron.
**Remaining honest gaps:** native Media/Quick Settings/Notifications adapters; agent task-tree/logs; clipboard pinning;
compact 11 px dot target (documented); zero-results/reduced-motion/drilldown capture evidence; real Windows/DPI/frame
pacing; agent copy pluralisation ("1 TASK"/"1 permission") and disabled-action visual state; preview error detail must be
sanitized (tracked in REVIEW_3).

### Task-49 evidence update (2026-11-10)

Zero-results (1150/860/430), reduced-motion (compact/search 1150), expanded 860/430 and refreshed
media/controls/notifications captures plus `task49-spec-recheck.json` now exist. Agent copy/action and clipboard
thumbnail deltas are resolved in code; Controls/Notifications now use the shell controls API. Open visual fixes: controls
card overflow (bottom tiles clipped), zero-results copy invisible at 185 px, notification test-action gating/disabled
dismiss state, and stale agents-1150/520/335 captures. Media adapter and media/Wi-Fi/BT drill-downs remain pending;
compact 11×37 dot target remains the documented accessibility deviation. Geometry is unchanged and passing.

## 12. Task-56 binding override — user-rejection fixes

The 2026-11-10 user review rejected the island's visual/UX quality (bleed/translucency, notifications overlap, Quick
Settings "Unsupported", search not matching Normal Launcher). The binding fix is `ISLAND_VISUAL_UX_FIX.md`; where it
conflicts with §5/§6 here (surface opacity, enter/exit, notifications, Quick Settings, island Search parity), the fix doc
wins. Summary: fully opaque surface (no opacity/fade), enter 180 ms / exit 120 ms transform-only, fixed-grid notifications
with non-overlapping dismiss/footer, usable Quick Settings (volume/brightness/audio + split Wi-Fi/Bluetooth + Windows
settings fallback instead of "Unsupported"), and island Search using the same four filters, group order, icons and empty
copy as Normal Launcher. G5 re-sign is blocked until the before/after evidence exists.

