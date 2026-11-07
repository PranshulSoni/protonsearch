# ProtonDock design system and UI port spec

Date: 2026-11-08. Branch: `dock`. Repo HEAD when written: `7727969`. Status: draft for user review, nothing in this document has been natively verified.
Audience: the plan author (Sonnet) and the implementers (Haiku, 1 to 3 files per task). Every value below is meant to be copied, not interpreted. Where a value is a choice the user may veto, it carries a deviation id (V1 to V28, section 4.11) or a decision id (D1 to D10, section 1.5).

## 1. Header

### 1.1 Version pins

| Input | Git blob | Lines | State |
|---|---|---|---|
| `dock-editable.html` (design ground truth, analysed) | `2126c4990818e68d157e2207977702f5ab349d19` | 6388 (the brief said 6103; measured with `wc -l`) | Uncommitted worktree content when analysed. Stored unreachable in the object database by `git hash-object -w`; see 1.4. HEAD holds an older blob `a22da3160adb390aee66385f5a1f3dc35b5b3d27`. |
| `dock-editable.html` (worktree at commit time) | `0d8c471dfb3bc43559fbfc057ea9da765abae7cb` | 6597 | Drifted during this run. Lines 1 to 5830 are byte-identical to the pinned blob; the changes are in the search launcher script (pinned lines 5831 to 6385). |
| `.design.md` | `c6e04e5da160d80a808cb663fc16cd05015d0f1b` | 190 | Clean |
| `antigravity/spec.md` | `6132b4a561203bddded08fc64cfcd5f4c3b8eba4` | 922 | Clean |
| `antigravity/BACKEND_IMPLEMENTATION_PROMPT.md` | `b2222af75b71a1cb395615bd281eac2c476296a6` | 277 | Clean, not modified here |
| `docs/design-reference.md` | `648f3b0bcc1046ec5fb3ea294702f2a8f1745e06` | 40 | Superseded by this document |
| `docs/qa/dock-defects-2026-11-07.md` (peer ledger) | `74c93240f6d6c2607045174545aec4db1cb9a3b4` | 160 | Peer-owned. Built on, never duplicated. |

All HTML line numbers in this document refer to the pinned blob `2126c49908`. Landed commits relied on: `5011a06` to `16cf1c2` (old Phases 1 and 2), `fa60e4b` (jitter), `a3d9e11` (icon extraction). Both fixes are committed and natively unverified.

### 1.2 What this supersedes

Superseded (UI only): `antigravity/spec.md` 1.1 to 1.3 (sizing, profiles, themes), the "Visual Anatomy" bullets of 2.0 to 2.16, and section 3 (motion); the whole of `docs/design-reference.md`; Phases 1 and 2 and the sizing decisions of `docs/superpowers/plans/2026-11-07-macos-dock-plan.md` (global icon size 32 to 64, per-widget tile width and height overrides, 46px tiles).
Not superseded: every backend, IPC, provider and credential statement in `antigravity/*`, `docs/widgets/CONTRACT.md`, and `CLAUDE.md` (which outranks this document). `.design.md` stays the statement of motion intent; where its numbers differ from this document, section 3.6 lists the winner.

### 1.3 Precedence and citation keys

Order of authority: `CLAUDE.md` > this document > `dock-editable.html` (pinned) > `.design.md` > `antigravity/spec.md` UI sections.
Rule of thumb: where section 4.11 lists a deviation, the deviation wins unless the user vetoes it. Where it lists none, the HTML wins. Where the HTML is silent, this document specifies.

Citations in the findings use these keys (file:line; guidance documents, no launcher was run):

| Key | Source |
|---|---|
| CL-Q1 to CL-Q5 | `CLAUDE.md` dock quality bar items 1 (no jitter), 2 (crisp icons), 3 (motion preserved), 4 (restrained surfaces, solid Windows logo), 5 (glyph quality) |
| CL-NF | `CLAUDE.md`: never present fabricated metrics as live |
| UIS | `.agents/skills/UI_SKILLS.md` (19 surfaces, 20 logo and DPI, 21 motion, 22 slop bans) |
| CF | `impeccable/reference/craft-floor.md` |
| AN, LY, PO, AU, CR | `impeccable/reference/` animate, layout, polish, audit.native, critique (CR:409 visibility of status, CR:469 consistency, CR:385 visual noise, CR:389 inconsistent pattern) |
| IC, SY, CD, MO, CT | `taste-skill/references/` icons, system, cards, motion, copy-tells |

Where a skill rule targets marketing pages and conflicts with the product (MO:58 no overshoot is respected; MO:59 and MO:227 forbid animated width and scaling cards, which the HTML's magnification and FLIP require; CD:55 forbids glass, which CL-Q4 mandates), the product rule wins and the row says so.

### 1.4 Detecting HTML drift

1. `git hash-object --no-filters dock-editable.html` must print `2126c4990818e68d157e2207977702f5ab349d19` for the inventory ranges to be valid. `--no-filters` is required because `core.autocrlf=true` here.
2. The pinned bytes: `git cat-file -p 2126c4990818e68d157e2207977702f5ab349d19 > pinned.html`. The object is unreachable, so `git gc` prunes it after about two weeks. Make it permanent by committing a copy (work package U0.4).
3. If the hash differs: `git diff --no-index --stat pinned.html dock-editable.html`. Re-derive ranges with the anchor strings in section 2 (`grep -n "<anchor>"`), re-run the unstyled-class check (a class list from `class="..."` in lines 2953 to the end, minus classes appearing as `.name` in the style block), then re-triage section 4.11. Do not edit tasks that cite a changed range until the table is refreshed.
4. Facts a pin test can assert (work package U0.4): `--tile-h: 48px`, `--dock-gap: 6px`, `--dock-pad: 6px`, `--dock-radius: 18px`, `--mag-strength: 0.22`, `--mag-radius: 86px`, `--lift: -4px`, 19 ids in `DEFAULT_DOCK_ORDER`, the four `DEFAULT_PROFILES` lists, `cycleWidgetWidth` steps 48, 96, 140, 168, and zero `@media` rules in the style block.

### 1.5 Decisions needing the user (index)

| Id | Decision | Recommended default | Section |
|---|---|---|---|
| D1 | Liquid Glass recipe. The HTML's clear glass reads at 1.10:1 on a white desktop and a transparent WebView2 window cannot blur the desktop. | Smoked fill `rgba(22,24,28,0.66)`; optional later spike for DWM acrylic under the region | 3.3 S5, 4.2 |
| D2 | Calendar glyph colour. HTML and `.design.md` 9.3 use a crimson gradient card with a coloured shadow. | Flat solid `#e11d48`, no gradient, no coloured shadow | 3.2 B4 |
| D3 | Assistant widget. The HTML names it "Assistant" but the spec calls it Siri with an orb. | Keep the widget as `assistant` (alias `siri-ai` read on load), neutral robot outline, no orb | 3.2 B3 |
| D4 | Glyph size. HTML uses 13 to 34px glyphs. | Outline icons 24px, dials 28px, app icons 24px | 4.3, V1 |
| D5 | Dock edge and alignment. The HTML covers bottom and centre only. | Ship bottom and centre; keep `edge` and `align` types in `fit.ts` but do not expose them | 6 |
| D6 | Shelf. HTML shows five control groups permanently. | Keep all five (faithful) and keep the landed fade while the rail is hot; user may later move theme and style into the Customizer | 4.10 |
| D7 | React-only widgets (`files`, `folders`, `trash`, `volume`, `chatgpt`, `agent`) have no HTML design. | Keep them in the gallery under "More widgets" with the generic tile anatomy; no bespoke capsule | 5 |
| D8 | Width steps. | 48, 96, 140, 168, 220 with a per-widget allowed subset; no free slider | 4.3 |
| D9 | Toast HUD. It lives inside the hidden search card, so dock toasts are invisible. | Do not port; use a visually hidden `aria-live` region | 3.5 P5 |
| D10 | Widget Store tab ("Coming Soon" cards, one duplicating the shipped Calendar). | Do not port | 3.2 B9 |

## 2. Inventory of `dock-editable.html`

Structure of the pinned blob: style block 7 to 2951; body markup 2953 to 3709; one IIFE script 3710 to 6385. The rail is empty markup (`<nav class="dock-rail" id="dockRail">`, 3400); JS fills it with `rail.innerHTML` on every render (4427). State lives in `localStorage` keys `protondock_preview_state_v6`, `protondock_custom_profiles`, `protondock_theme`.

There are no JavaScript springs anywhere in the file: every "spring" is `--ease: cubic-bezier(0.16, 1, 0.3, 1)` (line 52) with a fixed duration, and FLIP uses `Element.animate` or inline transitions. "Anchor" is a string to `grep -n` when the range drifts. "Exists" names the React or Rust file that already ports it. Gap text is terse; findings are in section 3.

| # | Feature | HTML lines (anchor) | React port target | Exists today | Gap |
|---|---|---|---|---|---|
| 1 | Design tokens `:root` | 17-59 (`--tile-h`) | `src/design/dock-tokens.css` (new) | Partial: `dock-tiles.css:1-11` (46px, 24% radius, blue `--dock-focus`); `native.css:349-380` (sage theme, `--accent: var(--sage)`) | Tile 46 vs 48; radius 24% vs 12px; no type, radius, motion tokens; green accent |
| 2 | Three themes | 82-152 (`body[data-theme`), JS 5805-5830 (`function setTheme`) | tokens file plus `theme.ts` `applyTheme` | Partial: ids `light`, `dark`, `glass` | HTML id `liquid-glass` is React `glass`; light theme is broken inside tiles (S4) |
| 3 | Wallpaper and desktop hint | 154-203 (`.desktop-wallpaper`) | none | n/a | Preview chrome. Do not port. |
| 4 | Stage, cluster, shelf layout | 209-237, markup 2954-3000 (`.dock-stage`) | `DockSurface.tsx` (`.dock-final`, `.dock-final-shelf`) | Yes, shelf has profile pills and Customize only | Theme pill, style pill, Edit pill, "+" tab missing |
| 5 | Profile pill, tabs, "+" tab | 239-272, markup 2958-2967, JS 5037-5090 (`function switchProfile`) | `DockSurface.tsx` `.dock-final-pill`; `layout.ts` `PROFILE_IDS` | Yes, four fixed profiles | No "+" tab or custom profiles; no switch motion; `.profile-add-tab` has no CSS in the HTML |
| 6 | Style switch Capsules or Lines | 274-305, 489-511 (`data-display-style`) | `layout.appearances`, `dock-tiles.css:380-391` | Partial | No shelf control; HTML global, React per profile (keep per profile) |
| 7 | Theme pill | 308-343 | shelf (new) | UI missing | Tokens exist; control missing |
| 8 | Customize pill | 345-368 | `.dock-final-customize` | Yes (label "Customize Widgets") | HTML shows icon plus "Customize" |
| 9 | Edit pill and Edit mode | markup 2996 (`edit-pill`), chip 425-448, JS 5088-5134 (`function setEditMode`) | `useEditMode.ts` (new), `DockSurface.tsx` | No | `.edit-pill` has no CSS in the HTML; author in 4.10 |
| 10 | Dock rail | 370-392 (`.dock-rail`) | `.dock-final-rail`, `dock-tiles.css:31-75` | Yes | Padding 7 vs 6; glass via `::after` with `--mag-*`; height auto vs fixed 60; `transition: none` |
| 11 | Slot and widget surface | 398-487 (`.dock-slot`, `.dock-widget`) | `.dock-final-slot`, `.dock-tile`, `dock-tiles.css:77-146` | Yes | 46px; tint gradient; rest shadow; permanent `will-change` |
| 12 | Glyph box and glyph rendering | 524-555 (`.w-glyph-box`) | `DockWidget.tsx` switch, `glyphs.tsx`, `icons.tsx` | Partial | Sizes and strokes (S8) |
| 13 | Running dot | 556-569 (`.dock-dot`) | `dock-tiles.css:367-378` | Yes | Landed sits outside the tile; HTML inside, with glow |
| 14 | Per-widget glyph artwork | JS 4140-4241 (`function renderWidgetGlyph`), CSS 575-720, 835-904 | `DockWidget.tsx` (14 ids), `glyphs.tsx` (`Ring`, `ClockFace`) | Partial | 19 HTML ids vs 14 React ids (section 5) |
| 15 | System-stats capsule | CSS 725-833, JS 4075-4123, 4343-4360 (`isSystemStats`) | none (React `system` is a ring) | No | Missing |
| 16 | Music capsule and transport | CSS 906-973, 2402-2440, JS 4125-4139, 4361-4390 (`dock-music-expanded`) | none | No | Missing |
| 17 | Weather capsule | CSS 975-1017, JS 4391-4396 (`dock-weather-expanded`) | none | No | Missing |
| 18 | Rect text stack | CSS 1019-1047 (`.dw-stack`), JS 4397-4412 | none | No | Missing |
| 19 | Count badge, notification dot | 883-904 (`.app-notif-badge`), 1994-2010 (`.dock-badge-dot`) | `.dock-badge`, `dock-tiles.css:263-277` (trash only) | Partial | App badges missing |
| 20 | Tooltip | 1060-1083 (`.dock-tip`) | `.dock-tip`, `dock-tiles.css:338-365` | Yes, follows magnification | Single line; not theme-aware |
| 21 | Popover base and caret | CSS 1566-1702, JS 5219-5300 (`function openPopoverForWidget`) | `WidgetCardWindow.tsx`, `WidgetCard.tsx`; Rust `main.rs:98-135`, `524-596` | Yes, as a separate native window | No caret; anchored above the dock window top, not the tile |
| 22 | 12 popover bodies | markup 2998-3398 (`id="popover-`), CSS 1703-2398, JS 5302-5760 | `WidgetCard.tsx` (per React id) | Partial | Fabricated data (B1); ids differ |
| 23 | Pointer magnification | CSS 56-57, 472-486, JS 4777-4803 (`function magnify`) | `magnify.ts`, `dock-tiles.css:122-146` | Yes, other constants | Section 5 |
| 24 | FLIP reorder and enter | JS 4242-4272 (`function reorderWithFlip`), 4427-4474 | none | No (deleted; defects doc item 3) | Missing |
| 25 | Rail width morph | JS 4429-4442 (`rail.style.width`) | none | No | Missing; must not feed per-frame fit |
| 26 | Drag reorder | JS 4668-4770, CSS 450-457 (`is-dragging`) | `wm-grip` in the manager only | No on the dock | Missing |
| 27 | In-place resize cycle | JS 5113-5134 (`function cycleWidgetWidth`) | manager `SizeControl` (3 sizes) | No on the dock | Different step model |
| 28 | Alt-click morph | JS 4483-4497 (`e.altKey`) | none | No | Missing |
| 29 | Customizer modal (Gallery, Apps, Store) | CSS 1085-1565, markup 3405-3545, JS 4804-5030 | `WidgetManager.tsx` (separate native window) | Yes, other shape | Apps catalog missing; Store is placeholder |
| 30 | New-profile modal | markup 3550-3592 (`profile-create-card`), JS 5128-5218 | none | No | Unstyled in the HTML (S10) |
| 31 | Toast HUD | CSS 2493-2528, markup 3596-3600, JS 5795-5803 | none | No | Do not port (D9) |
| 32 | Spotlight UI (Kensaku) | CSS 2446-2951, markup 3593-3709, JS 5846-6335 (pinned) | `SpotlightWindow.tsx` (613 lines) | Partial | No Quick Look drawer, list/grid toggle, calculator hero card, engine footer |
| 33 | Keyboard handling | JS 4843-4848, 6340-6380 (`Global Keyboard shortcuts`) | `WidgetCardWindow.tsx` (Escape) | Partial | Rail keyboard model absent (4.9) |
| 34 | State persistence | JS 4003-4040 (`protondock_preview_state_v6`) | `layout.ts` v2 | Yes | Schema v3 (section 5) |
| 35 | Reduced motion | JS 4775-4780 only; zero `@media` in CSS | `dock-tiles.css:393-418` | Yes in CSS | Extend (4.8) |

## 3. Design review of the HTML

### 3.1 Method

Static audit of the pinned blob: grep counts over the style block and script, a class-versus-rule cross check, and WCAG 2 contrast computed on composited backgrounds (dark tile = rail `rgba(14,17,22,.95)` over `#0d1017` plus white .05, giving `rgb(26,29,34)`; light tile = rail `rgba(244,247,252,.96)` over `#e2e8f0` plus black .045, giving `rgb(232,235,241)`; popover `rgba(18,22,30,.95)`). Contrast figures are computed assumptions, not native measurements. Severity: Blocker (breaches `CLAUDE.md` or makes a theme unusable), High, Med, Low.

### 3.2 Breaches of `CLAUDE.md`

| Id | Finding and evidence | Sev | Concrete fix | Rule |
|---|---|---|---|---|
| B1 | Fabricated values shown as live. System stats is a random walk (`Math.random` at 5532, interval 5557). Weather setup ignores the typed city and always writes 24 C, H28 L19, 48 %, 11 km/h, UV 3, AQI 28 (5666-5697); the popover shows an hourly row (3040-3047) beside the `--` temperature header. Claude and Codex "Save" write 70 % and "Connected" without checking the key (5713-5760). A music click invents "Nightcall by Kavinsky" (4614-4640), and previous and next cycle a three-track invented playlist (3904-3906, 4125-4139). `WIDGETS` ships AirPods 34 %, Wi-Fi "Studio-5GHz", battery 88 % "On AC (4h 12m)" (3785-3860). Battery popover rows (3050-3074). The analog clock hands are hard-coded and never move: `renderAnalogClockSvg` 4051-4073, refreshed only for the two text modes (4142-4148, 5483-5503) while the default mode is analog (3925). | Blocker | Every value comes from a provider snapshot. Missing data renders a state from 4.4 (never a plausible number). Sample data exists only in the browser preview (`?preview=1`), and any tile showing it carries `data-sample="true"` with a visible "Sample" corner label; native builds never import it (work package U4.3). | CL-NF, CD:305-309, CT "Invented specifics (ban)" |
| B2 | Controls that simulate state: battery "Toggle Charging / Draining Simulation" (3074, handler 5311); an agent click flips Working and Ready (4605-4612; spec 2.16 calls these "test states"); a music click flips local `isPlaying` (4614-4640). | Blocker | Delete. Actions send real commands (for example SMTC play and pause). The UI shows only observed state. | CL-NF, UIS:22 |
| B3 | Assistant orb: `radial-gradient(#60cdff,#e056fd,#686de0,#30336b)`, purple glow `0 0 12px rgba(160,100,255,.55)`, endless hue rotation (575-601). The CSS is dead (JS draws a robot outline, 4176-4187) but spec 2.5 says "idle orb". | High | Delete the CSS. The glyph is the robot outline in `currentColor`. Title "Assistant". (D3) | CL-Q4, UIS:22, CF:10, CF:34 |
| B4 | Coloured gradient sub-surfaces: calendar card `#e11d48 to #be123c` with a crimson shadow (840-841); invented playlist art gradients (3904-3906). | Med | Calendar flat `#e11d48`, no shadow (D2). Album art is the real thumbnail or a neutral music glyph on `--tile-bg`. | CL-Q4, UIS:19, CF:10 |
| B5 | Hue sprawl inside tiles: green-tinted badge (747-760) and popover badge (1634-1638); amber glyph strokes on weather (4214, 4388, 987) and notes (4203); green earbuds (874); Spotify green EQ (673) and play hover (2436); blue ring (705, 5717); bars yellow, rose, white (831-833); sparkline green plus cyan (4094-4106). Roughly 14 non-neutral hues, six of them blues. | High | Colour roles (4.2): neutral glyphs; only `ok`, `warn`, `bad`, the Windows logo and one clock accent are coloured. Ring and bars are neutral and turn `warn` or `bad` past a threshold. | CL-Q4, SY:68-77, UIS:22 |
| B6 | Emoji and symbol glyphs standing in for icons: water-drop and wind emoji plus U+25BE, U+2191, U+2193 in the weather capsule (4392-4393); `&times;` close buttons (3162-3172, 3416, 3555, 5607). | Med | Draw 12px SVG glyphs (drop, wind, chevron, arrows, close) in the icon set. | CF:40, IC:55 |
| B7 | Drop shadow on every tile at rest, `0 3px 8px rgba(0,0,0,.25)` (469); notification and count badges carry shadows or glows (899, 2008). | Low | Rest: inset highlight only. Hover: lift shadow only on hover (4.1). Badges: no glow. | UIS:22, CD:28-30, MO:79 |
| B9 | Placeholder content: the Apps catalog hard-codes six apps (3931-3950); the Widget Store shows six "Coming Soon" cards, one duplicating the shipped Calendar (3461-3545). | Med | Catalog comes from native enumeration. Drop the Store tab (D10). | CD:305-309 |

The HTML Windows logo is compliant (solid rects, `fill="var(--win-blue)"`, 3733) and is recorded here only to prevent regression; the landed `icons.tsx:8-15` four equal rects is the defect (defects doc item 5).

### 3.3 System-level inconsistencies

| Id | Finding and evidence | Sev | Concrete fix | Rule |
|---|---|---|---|---|
| S1 | Type: 23 distinct sizes from 7.5 to 38px in half-pixel steps; 31 `font-size` declarations are under 10px (7.5 x1, 8 x7, 8.5 x2, 9 x7, 9.5 x14; for example 757, 781, 814, 852, 877, 890); eight weights including 550, 650, 850. | High | Eight-step scale and weights 400, 500, 600, 700 (4.1 snap table). Nothing under 10px. | SY:62-64, PO:61, CR:385 |
| S2 | Radii: 21 distinct values (999px x17, 8px x14, 50% x14, 6px x10, 2px x7, 7px x6, 9px x5, 4px x5, 5px x3, plus one-offs 10, 14, 16, 18, 20). | Med | Radius ladder (4.1); one-offs snap to the nearest rung. | SY:90-93 |
| S3 | Colour has no accent token: six blues (`#60cdff #74b7ff #0a84ff #38bdf8 #93c5fd #0070df`), three greens, five reds or roses; `--dock-shadow` is defined per theme (102, 125, 149) and never used by `.dock-rail` (381-385). | High | Role tokens (4.2). | SY:68-77, CR:469 |
| S4 | Light theme is broken inside tiles. Only one light rule exists beyond the token blocks (341) while the style block hard-codes 53 `#fff` and 180 `rgba(255,255,255,...)`. `--glyph` (43) is not themed. Computed on the light tile: white text 1.19:1, default glyph 1.27:1, `--text-3` 2.15:1, running dot 1.19:1. Popovers (1574), tooltips (1060-1070) and the modal (1108-1112) stay dark. Dark theme `--text-3` is 3.50:1 on the tile and 3.75:1 in popovers. | Blocker (Light) | Semantic tokens per theme (4.2); raise dark `--text-3` to `#8a93a0` (5.44:1) and light to `#5b6678` (4.86:1). | CF:9, AU:36-37, PO:66 |
| S5 | Liquid Glass: `--rail-top .14`, `--tile-bg .10` white over the desktop; computed white text on the tile over a white desktop 1.10:1. All four windows are `transparent:true` with no acrylic or Mica call (`tauri.conf.json:5-8`), so `backdrop-filter` has no desktop to sample (to be confirmed natively). Outer rail shadows fall outside the `SetWindowRgn` region and are clipped (`window_geometry.rs`). | High | Smoked recipe (4.2, D1); no outer rail shadow. | CF:9, CF:34 |
| S6 | No `@media` rule in the style block at all. `prefers-reduced-motion` is checked only in JS and only gates magnification (4775-4780). `.design.md` section 8 promises reduced motion and reduced transparency. | High | 4.8. | CL-Q3, AN:77, MO:209-218 |
| S7 | Nine width values: edit cycle 48, 96, 140, 168 (5113-5121); customizer presets 48, 96, 130, 180 (4887-4890); toolbar "Expand Rectangles" 124 (3441); dead `toggleWidgetWidth` 48 or 124 (4762-4768); defaults 196, 200, 220; slider 48 to 220 step 2 (4893). One row shows presets, slider and value badge together. | High | Five steps (4.3). | CR:469, SY:62 |
| S8 | Glyph scale: 13, 16, 17, 18, 20, 22, 26, 28, 30, 34px across widgets (3812-3891, 4140-4241, 548-720, 836-863) against `.design.md` 3.2 (20 to 22, never over 24). Stroke widths 0.9 to 2.2 (1.8 x29, 2 x24, 1.6 x7); effective stroke after scaling 1.19 to 1.67px. | High | Glyph grid (4.3), stroke rule `36 / size` (4.5). | CF:40, PO:68, IC:55 |
| S9 | Hit targets: music buttons 20x20 (955), play 22 (2421), resize chip 15x15 (434-435), `.dpop-btn-text` 11px text with 2px 4px padding (1655), shelf tabs about 22px high (257-258). | Med | Minimum 24x24 CSS px for any pointer target (chip 20px visible inside a 24px target). | AU:15, LY:55 |
| S10 | 25 real markup classes have no CSS rule: `edit-pill`, `profile-add-tab`, `profile-create-card`, `pcc-btn pcc-close-btn pcc-field pcc-footer pcc-header pcc-input pcc-label pcc-preset-opt pcc-presets pcc-sub pcc-title`, `dpop-btn-primary`, `ps-res-icon ps-res-info ps-res-snippet ps-res-sub ps-res-title`, plus hook classes `dock-notes-glyph dock-slot-app dock-slot-empty-pin dock-slot-pinned`. Search rows use `ps-res-*` (6241-6246) while the rules are named `ps-item-*` (2746-2790). Dead: `.siri-orb-*` (575-601), `toggleWidgetWidth`. 54 inline `style="..."` (for example 3024-3025, 3280, 3300). The new-profile modal reuses `.ps-spotlight-backdrop` (3551). | High | Author the missing controls in 4.10; delete dead code; no inline styles. | PO:81-83, CR:389 |
| S11 | Magnification: falloff measured from the slot centre, so a 200px tile at peak 1.22 grows 22px per side against a 6px gap and overlaps neighbours; `getBoundingClientRect` for every slot on every `pointermove` without a frame throttle (4777-4790); `will-change: transform` is permanent (479), which can fix the raster scale at rest (blur at peak; native check required). | Med | Rect tiles do not scale; frame-batched; `will-change` only while hot (4.8). | AN:71, CL-Q2 |
| S12 | Popover anchored to the slot rect, not the magnified tile: 10px above the slot top while the peak tile top is 14.6px above it (5245, repeated at 5293; 4.6px overlap). No flip, no focus move, no `aria-expanded` (zero occurrences), outside click only, Escape priority differs between handlers (4844 versus 6357). | Med | 4.7, 4.9. | PO:74, SY:206-208 |
| S13 | Rail overflow: `max-width: calc(100vw - 20px)` with `overflow: visible` and no rule when widgets exceed the width (Work lists 13 widgets plus one tile per running app). | Med | Section 6 row 9. | LY:51 |
| S14 | Tooltip anchored to the slot overlaps the magnified tile (1060-1083). Landed `dock-tiles.css:353` already compensates. | Low | Keep the landed approach (4.6). | CL-Q5 |
| S15 | Legibility from 32 to 96px: the HTML has no scaling rule and several glyphs are CSS shapes that do not scale as one drawing: the battery is a bordered box with a 1.5px border and a 2x4px nub (610-641), the EQ is three 2px divs (670-682), the running dot a 3.5px circle with glow (557-569). At 2x (96px) the 1.5px border is 3px beside 2.0px icon strokes; at 32px the 8px and 7.5px texts are about 5px high. | High | All glyph art is SVG on a 24 or 28 grid and scales uniformly; stroke rule `36 / size`; no text under 10px; acceptance is the 32, 48, 64, 96 glyph sheet in section 7. | CL-Q5, CF:40, PO:68 |

### 3.4 Widget by widget

| Widget | Evidence | Findings | Sev | Fix |
|---|---|---|---|---|
| start | `iconSvg` 3733, click shows a toast stub (4641-4646) | Solid and compliant; 20px; sub-label hard-coded "Windows 11" | Low | 24px flag, `--win-logo`; tooltip "Start" |
| search | 3741, sub "Alt + Space" | Hard-coded hotkey; Alt+Space is the Windows system menu, the backend owns the shortcut | Low | Sub-label shows the registered hotkey or none |
| running-apps, pinned-apps | placeholders 18px stroke 1.8 (3844, 3853); hard-coded 20px coloured SVG look-alikes of VS Code, Chrome, Spotify, Slack, Figma (3931-3950); badge 14px, 8px text, white on `#ff3b30` (884-904) 3.55:1; dot glow (566) | Look-alike brand marks, tiny badge text, glow | Med | Real icons from native at DPR (4.5); badge 16px, 10px text, `#d92d20` (4.83:1); dot without glow |
| calendar | card 34px (836-863), weekday 8px/800 tracked caps, gradient plus coloured shadow, static `FRI 16` (3780) refreshed by `updateLiveCalendar` (5509) | Larger than the 32px glyph box; unreadable weekday | Med | 28px flat card, weekday 10px/700, date 15px/700 (4.3) |
| weather | glyph 18px amber plus 10px temp (643-656, 4211-4217); capsule row 2 at 8px with emoji (975-1017, 4392-4393); fabricated data (B1) | Illegible row 2; tint | High | Section 4.3 table; SVG glyphs; neutral glyph; `--` when unconfigured |
| system-stats | 22px badge with 8px text (747-760), 7.5 and 8px labels (810-817), sparkline SVG text 5.5 to 7.5px (4106-4112), bars in three colours, `id="sparkGrad"` duplicated per instance (4094), random walk | Illegible; hue sprawl | High | Remove all sparkline SVG text (numbers move to the tooltip); 10px labels; one neutral bar colour; per-instance gradient id |
| peripherals | 18px green icon plus 9.5px percent (865-881); spec promises per-bud levels and ANC pills (2.15, 2.16) | Standard GATT battery gives one level per device; ANC control is not available | Med | One level per device; no ANC UI |
| status-battery | CSS box 22x12 (610-641), fill always green (630), no low state | Wrong colour when discharging or low | Med | SVG battery; fill `--ok` only while charging, neutral otherwise, `--warn` at 20 % or less, `--bad` at 10 % or less |
| wifi, bluetooth, notifications, system-tray | sizes 18, 17, 18, 16 at stroke 2, 2, 2, 2.2 (3863-3891) | Four sizes, three strokes | Low | All 24px at the stroke rule |
| notifications | badge dot 13px, 8px text, red glow (1994-2008) | Glow, tiny text | Med | Count badge spec (4.6) |
| quick-notes | amber stroke (4203) | Category tint | Low | Neutral glyph |
| clock | analog 28px dial with orange second hand (4051-4073); digital 13px/700 inline styles (4142-4147); frozen hands (B1) | Frozen; inline styles | Med | Live hands from `now` (landed `ClockFace` does this); tokens |
| music-player | idle 26px disc with green border (2410-2420); playing 22px look-alike Spotify disc (4196); art gradients (3904-3906); title 11/700, artist 8.5px (2402-2440, 906-973); buttons 20px; play hover green (2436) | Look-alike logo, fake art, small targets | High | Idle: neutral note glyph; playing: the media session's real icon or art; 24px buttons; neutral hover |
| claude, codex | ring 28px r13 stroke 2.4 (effective 2px) with 13px glyph (3812, 3825, 4226-4240); unconfigured state exists (3803-3830: track `.15`, glyph `.45`); working halo `ringSpin` with `drop-shadow(0 0 6px ...)` (2184-2190) | The unconfigured pattern is the model for 4.4; glow halo is decoration | Med | Keep the unconfigured look; working = rotating neutral arc, no glow |
| assistant | robot outline 22px stroke 1.8, hard-coded `#ffffff` (4176-4187); click shows a toast stub | Breaks in Light theme | Low | `currentColor`, 24px |

### 3.5 Shelf, popover, modal, search

| Id | Finding and evidence | Sev | Fix |
|---|---|---|---|
| P1 | Edit pill, "+" tab, new-profile card and `dpop-btn-primary` have no CSS (2996, 2967, 3552-3592, inline 3024-3025). | High | Author them (4.10). |
| P2 | Shelf pills use three fixed backgrounds `rgba(16,19,26,.88)` (246, 281, 353) while only the theme pill uses `--shelf-pill-bg` (315); text 10 and 10.5px; heights 29px and auto. They stay dark in Light. | Med | One pill recipe (4.10). |
| P3 | `.dock-popover` hard-codes `rgba(18,22,30,.95)` (1574) although `--popover-bg` is defined (95, 118, 141). | Med | Use the token. |
| P4 | Popover section titles are uppercase, tracked, 10px, `--text-3` (1647-1654, 3.75:1); badges 9.5px; link buttons blue 11px. | Med | Sentence case 11px/600 `--text-2`; badges 10px. SY:58-61. |
| P5 | The toast lives inside the hidden search card (markup 3596-3600 within 3594; ancestor opacity 0 at 2446-2465), so dock toasts never show unless search is open. | Med | Drop (D9); `aria-live` region. |
| P6 | Customizer row shows three width controls and order buttons (4887-4893, 1350-1440). | Med | One segmented control of allowed steps; order by drag or keyboard. |
| P7 | Radii 20 (modal), 16 (popover), 18 (rail, search card), gradient fill on the modal (1112). | Low | Radius ladder; modal solid `--modal-bg`. |
| P8 | `role="dialog" aria-modal="true"` on the customizer (3408) without a focus trap or focus return. | Med | Trap and return focus (4.9). |
| P9 | Search shortcuts `Alt+Space`, `Ctrl+Space`, `/` are bound in the page (6340-6356). | Low | Native owns global shortcuts; no page-level `/`. |
| P10 | Copy: specification language leaks into the UI. Running and pinned apps show the sub-label "Discrete 48px Square Tiles" (WIDGETS, 3841 and 3850); the customizer subtitle says "Locked 48px square height for all widgets..." (3414); title-case labels "Connect Account", "Configure Assistant" (3832), "Take a Note" (3248); "Toggle Charging / Draining Simulation" (3074). | Med | Sub-labels state facts ("3 running", "2 pinned"); customizer subtitle "Choose which widgets appear and how wide they are."; sentence case; controls name their action. SY:58, CF:16, CT. |

### 3.6 Drift between documents

| Item | `.design.md` | `antigravity/spec.md` | HTML | Winner |
|---|---|---|---|---|
| Rail radius | 20px | none | 18px (24) | 18px |
| Glyph size | 20 to 22px, max 24, in a 32px box | none | 13 to 34px | 24 icons, 28 dials in a 32 box (V1) |
| Width steps | 48, 96, 140, 168 | same cycle plus 48 to 220 | nine values (S7) | 48, 96, 140, 168, 220 |
| Popover motion | 0.30s, `(0.25,1,0.35,1)`, damping 0.82, anchored origin | 200ms spring | 180ms opacity, 200ms `(0.16,1,0.3,1)`, centre origin | HTML timing plus anchored origin (V12) |
| Pointer-down | `scale(0.96)` 100ms | same | not implemented | Add (V11) |
| FLIP and profile durations | 0.32, 0.35, 0.40s | 0.38 and 0.32s | 220, 380, 320ms | HTML |
| Reduced motion and transparency | specified | none | none | 4.8 |
| Dark text roles | none | `#f1f5f9 #94a3b8 #64748b` | `#f0f3f6 #9ea5b0 #6b7280` | HTML, `--text-3` raised (V5) |
| Focus ring | none | none | white .55, 2px, offset 2 or 3 | Neutral ring (V10); the old plan used `#0a84ff` |
| Badge size | none | 14px | 14px | 16px (V15); landed is 18px |
| Tile size | 48px | 48px | 48px | 48px; landed 46px is wrong |

## 4. The improved design system

Default is a faithful port. Everything that differs from the HTML is a numbered deviation in 4.11. Rules cited: see 1.3. Naming: this document uses the HTML class names (`.dock-slot` is the landed `.dock-cell`, `.dock-widget` is the landed `.dock-tile`, `.dock-final-slot` is the wrapper); implementers keep the landed names.

### 4.1 Tokens

Create `src/design/dock-tokens.css`, imported last in `src/main.tsx` (after `dock-tiles.css`, so its values win), and delete the `:root` block at `dock-tiles.css:1-11`. Copy as written:

```css
:root {
  /* geometry: height is locked, never animated (.design.md 3.1) */
  --tile-h: 48px;  --tile-w: 48px;
  --dock-gap: 6px; --dock-pad: 6px;
  --rail-h: calc(var(--tile-h) + var(--dock-pad) * 2);       /* 60px */
  --dock-bottom: 16px; --shelf-gap: 9px; --shelf-h: 28px;
  /* radius ladder */
  --r-bar: 2px; --r-xs: 4px; --r-sm: 6px; --r-md: 8px; --r-tile: 12px;
  --r-pop: 16px; --r-rail: 18px; --r-modal: 20px; --r-pill: 999px;
  /* glyph grid inside the 48px tile (4.3) */
  --glyph-box: 32px; --glyph-icon: 24px; --glyph-dial: 28px; --glyph-stack: 18px; --glyph-mini: 12px;
  --hit-min: 24px;
  /* pointer behaviour copied from the HTML (56-58) */
  --mag-strength: 0.22; --mag-radius: 86px; --lift: -4px;
  /* popover and tooltip */
  --pop-w: 330px; --pop-gap: 10px; --caret: 10px; --tip-gap: 10px;
  /* type: Segoe UI Variable resolves from the HTML's stack on Windows */
  --font: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  --font-display: "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif;
  --fs-10: 10px; --fs-11: 11px; --fs-12: 12px; --fs-13: 13px;
  --fs-14: 14px; --fs-16: 16px; --fs-20: 20px; --fs-32: 32px;
  --fw-regular: 400; --fw-medium: 500; --fw-semibold: 600; --fw-bold: 700;
  /* motion (4.8) */
  --ease-spring: cubic-bezier(0.16, 1, 0.3, 1);
  --dur-mag: 120ms; --dur-hover: 140ms; --dur-press: 100ms; --dur-reorder: 220ms;
  --dur-flip: 380ms; --dur-enter: 320ms; --dur-pop: 200ms; --dur-fade: 180ms;
  --dur-tip: 120ms; --dur-pill: 130ms;
}
```

Type roles (size / line height / weight): micro 10/12/600 (sub-labels, badge numerals, popover badges); small 11/14/600 (tooltip, shelf pills, tile titles in 96px tiles); body 12/16/400 or 600 (popover body, rect tile title 600); label 13/16/600 (popover and dialog labels, list titles); numeric 14/16/700 tabular (tile readouts); title 16/20/700 (dialog and popover headings, weather temperature in capsules); headline 20/24/700 (popover headline numerals); display 32/36/700 tabular (popover big numerals, `--font-display`). Numbers always use `font-variant-numeric: tabular-nums`. Labels are sentence case; the only uppercase text is the three-letter weekday inside the calendar glyph and `kbd` caps (SY:58).

Snap table for porting existing CSS (old value to token):

| Property | Mapping |
|---|---|
| font-size | 7.5, 8, 8.5, 9, 9.5, 10 to 10px (content that no longer fits is removed, never shrunk); 10.5, 11, 11.5 to 11; 12, 12.5 to 12; 13, 13.5 to 13; 14 to 14; 15, 15.5, 16, 17 to 16; 18, 19 to 20; 32, 34, 38 to 32 |
| font-weight | 500 and 550 to 500; 600 and 650 to 600; 700, 800, 850 to 700 |
| radius | 1 to 3 to `--r-bar`; 4, 5 to `--r-xs`; 6, 7 to `--r-sm`; 8, 9, 10 to `--r-md`; 12 to `--r-tile`; 14, 16 to `--r-pop`; 18 to `--r-rail`; 20 to `--r-modal`; 50% and 999px unchanged |

### 4.2 Colour roles per theme

Roles replace every literal colour. Only `ok`, `warn`, `bad`, `--win-logo`, `--clock-accent`, `--bad-fill` and the calendar glyph fill are coloured (SY:68-77, UIS:22). Dark and light values copy the HTML where it had one (27-29, 82-126); changed values are marked and justified in 4.11. Selectors: `body[data-theme="dark"]` (default), `"light"`, `"glass"` (the React id; the HTML's `liquid-glass` is the same theme).

| Role | Dark | Light | Glass (Liquid Glass) |
|---|---|---|---|
| `--rail-fill` | `linear-gradient(180deg, rgba(22,26,33,.94), rgba(14,17,22,.95) 60%, rgba(10,12,16,.96))` | `linear-gradient(180deg, rgba(250,252,255,.96), rgba(244,247,252,.96) 60%, rgba(236,241,249,.97))` | `rgba(22,24,28,.66)` (changed, V6) |
| `--rail-border` | `rgba(255,255,255,.085)` | `rgba(0,0,0,.10)` | `rgba(255,255,255,.32)` |
| `--rail-edge` (inset only) | `inset 0 1px 0 rgba(255,255,255,.08), inset 0 -1px 0 rgba(0,0,0,.4)` | `inset 0 1px 0 rgba(255,255,255,.9)` | `inset 0 1.5px 0 rgba(255,255,255,.65), inset 0 0 0 1px rgba(255,255,255,.22)` |
| `--rail-ring` (solid colour behind badges) | `rgb(14,17,22)` | `rgb(243,246,252)` | `rgb(40,42,46)` |
| `--tile-bg` | `rgba(255,255,255,.05)` | `rgba(0,0,0,.045)` | `rgba(255,255,255,.08)` (changed) |
| `--tile-border` | `rgba(255,255,255,.075)` | `rgba(0,0,0,.07)` | `rgba(255,255,255,.22)` |
| `--tile-hover-bg` | `rgba(255,255,255,.10)` | `rgba(0,0,0,.08)` | `rgba(255,255,255,.16)` (changed) |
| `--tile-hover-border` | `rgba(255,255,255,.20)` | `rgba(0,0,0,.16)` | `rgba(255,255,255,.48)` |
| `--tile-edge` (inset highlight) | `inset 0 1px 0 rgba(255,255,255,.10)` | `inset 0 1px 0 rgba(255,255,255,.7)` | `inset 0 1px 0 rgba(255,255,255,.30)` |
| `--tile-lift-shadow` (hover only) | `0 10px 22px rgba(0,0,0,.38)` | `0 8px 18px rgba(15,23,42,.16)` | `0 10px 22px rgba(0,0,0,.34)` |
| `--text` | `#f0f3f6` | `#0f172a` | `#ffffff` |
| `--text-2` | `#9ea5b0` | `#475569` | `rgba(255,255,255,.96)` |
| `--text-3` (non-essential text only) | `#8a93a0` (changed from `#6b7280`) | `#5b6678` (changed from `#94a3b8`) | `rgba(255,255,255,.80)` |
| `--glyph` | `#cbd3dc` | `#334155` (new) | `#ffffff` |
| `--win-logo` | `#60cdff` | `#0067c0` | `#60cdff` |
| `--ok` / `--warn` / `--bad` | `#34d399` / `#f0bd6a` / `#f87171` | `#047857` / `#9a3412` / `#b91c1c` | same as dark |
| `--bad-fill` (count badge) | `#d92d20` | `#d92d20` | `#d92d20` |
| `--clock-accent` | `#f59c35` | `#c2410c` | `#f59c35` |
| `--dial-face` / `--dial-ink` (clock face) | `#f0f3f6` / `#14161a` | `#ffffff` / `#14161a` (plus 1px ring `rgba(0,0,0,.18)`) | `#ffffff` / `#14161a` |
| `--shelf-pill-bg` | `rgba(16,19,26,.88)` | `rgba(245,248,254,.92)` | `rgba(22,24,28,.66)` |
| `--active-pill-bg` | `rgba(255,255,255,.12)` | `rgba(0,0,0,.09)` | `rgba(255,255,255,.22)` |
| `--popover-bg` / `--modal-bg` | `rgba(18,22,30,.95)` / `rgba(18,22,30,.96)` | `rgba(254,255,255,.98)` / `rgba(253,254,255,.98)` | `rgba(22,24,28,.90)` / `rgba(22,24,28,.92)` |
| `--tip-bg` / `--tip-text` | `rgba(14,16,22,.96)` / `#ffffff` | `rgba(255,255,255,.98)` / `#0f172a` | `rgba(22,24,28,.90)` / `#ffffff` |
| `--line` | `rgba(255,255,255,.12)` | `rgba(0,0,0,.12)` | `rgba(255,255,255,.28)` |
| `--scrim` | `rgba(0,0,0,.60)` | `rgba(15,23,42,.35)` | `rgba(0,0,0,.60)` |
| `--focus-ring` | `rgba(255,255,255,.85)` | `#0f172a` | `rgba(255,255,255,.95)` |
| `--inverse-bg` / `--inverse-text` | `#f0f3f6` / `#0f172a` | `#0f172a` / `#ffffff` | `#ffffff` / `#0f172a` |

Computed contrast (assumptions in 3.1): dark `--text-3` 5.44:1 on the tile and 5.83:1 in a popover; light `--text-3` 4.86:1, `--glyph` 8.67:1, `--ok` 4.59:1, `--warn` 6.12:1, `--bad` 5.42:1, `--win-logo` 4.75:1; white on `--bad-fill` 4.83:1; white on the calendar fill `#e11d48` 4.70:1; glass over a pure white desktop: primary 4.76:1, secondary 4.52:1, tertiary 3.71:1 (over mid grey 128: primary 8.73:1).

Glass recipe, as the HTML wants it and as the native window can deliver it: the rail is `background: var(--rail-fill)`, `border: 1px solid var(--rail-border)`, `box-shadow: var(--rail-edge)`, `border-radius: var(--r-rail)`, plus `backdrop-filter: blur(28px) saturate(145%)` (keep for forward compatibility only: the windows are transparent with no acrylic, so nothing is sampled behind them and contrast must never depend on blur). No outer drop shadow on the rail or shelf pills: the window region clips it. Tiles carry only `--tile-edge` at rest (V7).

### 4.3 Tile anatomy, glyph grid, widths

Slot (`button.dock-slot`, `data-shape`): `height: var(--tile-h)`, `width: var(--w)`, `border-radius: var(--r-tile)`, `--w` is the width step in px; `data-shape="square"` when `--w` is 48, otherwise `"rect"` (HTML threshold `width > 52`, 4343). Surface (`.dock-widget`, inside the slot): `border: 1px solid var(--tile-border)`, `background: var(--tile-bg)`, `box-shadow: var(--tile-edge)`, `overflow: hidden`, `transform: translateY(var(--lift-y, 0)) scale(var(--mag, 1))`, `transform-origin: bottom center`. Hover (slot): `--lift-y: var(--lift)`, background and border to the hover tokens, shadow `var(--tile-edge), var(--tile-lift-shadow)`. Siblings of the surface, never clipped by it: tooltip, running dot, state dot, count badge, resize chip. Stacking, low to high: surface, glyph, text, dot, badge, chip, tooltip. Lines style (`data-appearance="lines"`): HTML 489-511 unchanged (no gap, transparent tiles, 1px divider inset 9px top and bottom, hover `rgba(255,255,255,.05)`), divider colour `var(--line)`.

Layouts. Square: the 32px glyph box is centred (8px inset). Rect: padding-left 8px, 32px glyph box, gap 6px, text stack (title 12/16/600 `--text`, sub 10/12/400 `--text-2`, one line each with ellipsis), padding-right 10px. Capsules (weather, system stats, music at their widest steps) use the per-widget layouts below.

Glyph classes (all centred on the tile centre; stacks are centred as a block; a play triangle shifts +1px right for optical balance):

| Class | Size | Use | Rules |
|---|---|---|---|
| icon | 24x24, viewBox 24 | outline widget icons | `stroke-width` 1.5 (the rule is `36 / displayPx`), round caps and joins, `currentColor`, `shape-rendering: geometricPrecision`; solid fills only for the Windows flag, battery fill, bolt, play |
| dial | 28x28 | clock, ring, calendar card | bounding box 28, centred |
| stack | icon 18 plus 2px gap plus one 10/12/700 line, total 32 | weather, peripherals, CPU in square mode | stroke 2.0 on the 18px icon |
| app | 24x24 raster | running and pinned apps | 4.5 |
| mini | 12x12 | capsule rows, popovers | stroke 3.0 |

Width steps: 48, 96, 140, 168, 220. A width outside a widget's allowed set is rejected and replaced by its default (never clamped, no free slider). Edit-mode cycle and the customizer control walk the widget's allowed steps in ascending order and wrap.

| Widget (id) | Default | Allowed | At 48 | Wider steps | Not live (4.4) |
|---|---|---|---|---|---|
| start | 48 | 48 | Windows flag 24px, solid `--win-logo` | n/a | always live |
| search | 48 | 48, 140 | magnifier icon | 140: icon, "Search" 12/600, sub = registered hotkey or none | always live |
| running-apps | 48 each | 48 | one tile per window, app icon 24, dot, count badge | n/a | none: collapses to nothing (placeholder only in edit mode, V27) |
| pinned-apps | 48 each | 48 | pinned app icon, dot only when running | n/a | none: dashed placeholder only in edit mode (HTML 4313-4322) |
| calendar | 48 | 48, 140 | dial: 28px flat `#e11d48` squircle radius 8, weekday 10/10/700 caps white, date 15/15/700 white | 140: card, "Friday" 12/600, "16 November" 10/400 | system date, always live |
| weather | 220 | 48, 96, 140, 220 | stack: weather icon 18 plus temperature 10/12/700 | 96: icon 24, temp 14/700; 140: icon, temp 16/700, condition 10; 220: icon 24, temp 16/700, row 1 "Clear sky, H 28 L 19" 11/600, row 2 mini drop 12 plus "19 %", mini wind 12 plus "11 km/h", city 10/400, mini chevron | unconfigured: icon plus `--`, tooltip "Set location" |
| system-stats | 220 | 48, 96, 220 | stack: label "CPU" 10/12/600 `--text-2` over value 14/16/700 | 96: 24px ring plus value; 220: left block (label, value 14/700, sub "RAM 58 % / SSD 43 %" 10/400) and right 86x30 sparkline or three bars (labels 10/600, tracks 4px, one neutral fill, `--warn` at 80 %, `--bad` at 95 %), click toggles view; no text inside the sparkline SVG (V22) | unavailable: `--` |
| peripherals | 48 | 48, 140 | stack: earbuds icon plus percent 10/700 | 140: icon, device name 12/600, "Battery 34 %" 10/400 | no device with a level: `--`, tooltip "No device reports battery" |
| status-battery | 48 | 48, 96, 140 | icon: battery outline 24x12 with fill (`--ok` while charging, `--text` otherwise, `--warn` at 20 % or less, `--bad` at 10 % or less), bolt when charging | 96: plus "88%" 14/700; 140: plus "Charging" or "On battery" and time 10/400 | no battery: plug icon, state unavailable, "No battery detected" |
| wifi | 48 | 48, 140 | icon with arcs filled by signal | 140: SSID 12/600, "Connected" 10/400 | off or no adapter: arcs dimmed, slash |
| bluetooth | 48 | 48, 140 | icon | 140: "Bluetooth", "2 devices" | radio off: dimmed |
| notifications | 48 | 48, 140 | bell, count badge | 140: "Notifications", "3 unread" | access denied: bell with slash, state `permission-denied` |
| quick-notes | 48 | 48, 140 | note icon, neutral | 140: "Quick note", first line of the latest note | no notes: "No notes yet" |
| system-tray | 48 | 48 | grid icon | n/a | always live |
| clock | 48 | 48, 96, 140 | dial 28: disc `--dial-face`, 12 ticks and hour and minute hands `--dial-ink`, hands rotate from `now`, second hand and pin `--clock-accent` | 96: digital time 14/700; 140: time 12/700 plus "Friday 16" 10/400 | always live; mode (analog, time, time and date) is a widget setting |
| music-player | 168 | 48, 168, 220 | idle: music note icon; playing: the session's real icon or art 24, three 2px EQ bars (`--text`, animated only while playing) | 168: art 32, title 11/600, artist 10/400, play or pause 24px button; 220: plus previous and next 24px buttons | nothing playing: note icon, tooltip "Nothing playing" |
| claude | 48 | 48, 140 | dial: ring 28 (r 12, stroke 2.4), glyph 14 inside; neutral fill, `--warn` past the widget's `warnAt` | 140: "Claude", "70 % session" or "Connect account" | unconfigured: track `rgba(255,255,255,.15)` only, glyph .45 (HTML 3803-3830) |
| codex | 48 | 48, 140 | same as claude | same | same |
| assistant | 48 | 48 | robot outline icon (HTML 4176-4187) in `currentColor` | n/a | unconfigured: dimmed, tooltip "Set up assistant" |
| files, folders, trash, volume, chatgpt, agent | per React meta | 48, 140 | generic: icon plus optional stack (D7) | generic rect | per 4.4 |

### 4.4 Status visuals

Every widget renders exactly one state, derived from `Snapshot.status` (`src/widgets/sdk/types.ts:3`) and widget setup. Dimming applies to glyph and text, never to the tile border. The indicator is a 6px dot at top-left, inset 5px, so it never collides with the count badge at top-right.

| UI state | `WidgetStatus` | Glyph and value | Indicator | Tooltip line 2 | Accessible label suffix |
|---|---|---|---|---|---|
| live | `live` | full opacity, real value | none | optional "Updated 14:02" | value |
| loading | `loading` (rendered only after 400ms) | glyph opacity .55, 1.2s opacity pulse .55 to .85, value `--` | none | "Loading" | busy |
| unconfigured | `disconnected` | glyph .55, value `--` or none, tile border `1px dashed var(--tile-border)` | none | what to do: "Connect account", "Set location", "Set up assistant" | "needs setup" |
| unavailable | `empty` | glyph .40, value `--` | none | reason: "No battery detected", "Nothing playing", "Bluetooth is off" | "unavailable" |
| permission needed | `permission-denied` | glyph .55, value `--` | `--warn` solid dot | "Allow access in Windows settings" | "access needed" |
| stale | `stale` | last value at opacity .70 | `--warn` hollow dot (1.5px ring) | "Updated 12 min ago" | "out of date" |
| error | `error` | last value or `--` | `--bad` solid dot | "Could not refresh. Click to retry." | "error" |

Copy states facts or actions; it never says "coming soon" and never shows a plausible number (CL-NF, CR:409, PO:42). A missing value is `--`.

### 4.5 Icon rendering at device pixel ratio

- SVG glyphs: integer CSS sizes from 4.3, `viewBox="0 0 24 24"`, `stroke-width = 36 / displayPx` (24px gives 1.5, 18px gives 2.0, 12px gives 3.0; the effective stroke is always 1.5px), `shape-rendering: geometricPrecision`. Remove the `transform: translateZ(0)` and `image-rendering` rules the HTML puts on glyphs (533-545). Icons come from one drawn set in `src/widgets/final/icons.tsx` (CF:40, IC:55, PO:68).
- Raster app icons: CSS box 24x24. The UI asks native for `px = ladder(ceil(24 * 1.22 * devicePixelRatio))` where `ladder` is the smallest of 32, 48, 64, 96, 128, 256 that is not below the value (1.0 gives 32, 1.25 gives 48, 1.5 gives 48, 1.75 gives 64, 2.0 gives 64, 2.5 gives 96, 3.0 gives 96). The 1.22 is the magnification peak, so a hovered tile is never upscaled. Native returns a PNG at exactly that size, downsampled from the 256px source with Lanczos3 (`.design.md` 3.2) and never upscaled. If the best source is smaller than requested, native returns it at its own size and the UI draws it at `natural / dpr` CSS px, never stretched. Cache key `${id}@${px}`. Follow-up from the defects doc: when the chosen HICON is under 64px, prefer `PrivateExtractIconsW(exe, 0, 256, 256)` except for `ApplicationFrameHost.exe`.
- Markup: `<img width="24" height="24" decoding="async" draggable="false" alt="">` with `image-rendering: auto`. Never `crisp-edges` or `-webkit-optimize-contrast` on bitmaps (they alias downscaled icons).
- Re-request icons when `devicePixelRatio` changes (a `matchMedia("(resolution: <dpr>dppx)")` listener re-armed after each change) and when the dock window moves to another monitor.
- No permanent `will-change: transform` on tiles (it can pin the raster scale at 1.0 and blur the magnified peak). Set `will-change: transform` only on `.dock-final[data-hot="true"] .dock-tile`.
- Sizes keep even insets (24px in 48 gives 12, 28 gives 10), the running dot is 4px, and text line heights are even, so content lands on device pixels at 100, 125, 150, 175 and 200 percent. This is a design rule; the native check is in section 7.

### 4.6 Badges, dots, tooltip

| Part | Spec |
|---|---|
| Count badge | Sibling of the surface. `position:absolute; top:-3px; right:-3px; min-width:16px; height:16px; padding:0 4px; border-radius:8px; background:var(--bad-fill); color:#fff; font:700 10px/16px var(--font); font-variant-numeric:tabular-nums; border:1.5px solid var(--rail-ring)`. Text `99+` above 99. No shadow, no glow. Hidden at 0. Replaces `.app-notif-badge` (884-904) and `.dock-badge-dot` (1994-2008). |
| Running dot | Sibling of the surface. `left:50%; bottom:3px; width:4px; height:4px; margin-left:-2px; border-radius:50%; background:var(--text)`; no shadow; `pointer-events:none`; opacity transition `var(--dur-tip)`. Inside the tile, as in the HTML (557-569); stays put while the surface scales about its bottom edge. |
| State dot | 4.4: 6px, top-left, inset 5px. |
| Tooltip | Sibling of the surface. `position:absolute; left:50%; bottom:calc(100% + var(--tip-gap)); padding:4px 9px; border-radius:var(--r-md); background:var(--tip-bg); color:var(--tip-text); border:1px solid var(--line); box-shadow:0 4px 12px rgba(0,0,0,.30)`. Line 1: `600 11px/14px`. Optional line 2 (state or hint, 4.4): `400 10px/14px` at 72 % of the text colour. Each line one row with ellipsis, `max-width: min(240px, railWidth - 12px)`. The tip is clamped inside the rail's x-extent through a `--tip-shift` set on hover, because the native window region clips anything outside it and the region must not widen on hover. Position compensates for magnification and lift: `transform: translate(-50%, calc(-1 * var(--tile-h) * (var(--mag, 1) - 1) + var(--lift-y, 0px)))` (the landed `dock-tiles.css:353` idea). Appears on `:hover` and `:focus-visible` of the slot with opacity and a 4px rise over `var(--dur-tip)` ease-out. `aria-hidden` (the slot's label carries the text). |

### 4.7 Popover (card) anatomy and anchoring

The React app shows cards in the separate native `widget` window (`WidgetCardWindow.tsx`, Rust `open_widget_card`), not in the dock page. The visual contract below is for that window's root; the placement contract is Rust.

Anatomy: width `var(--pop-w)` (330px, as the HTML), height to content, body scrolls past 380px, window max 440px. Surface: `background: var(--popover-bg); border: 1px solid var(--line); border-radius: var(--r-pop); box-shadow: var(--tile-edge)`; no outer shadow (the window clips it). Header: padding `10px 14px`, bottom border `var(--line)`, 16px neutral glyph, title `600 12px/16px`, optional badge pill `600 10px/12px` on `--tile-bg` (a status badge colours its text with `ok`, `warn` or `bad`, never its fill). Body: padding `12px 14px`, gap 10px, section titles `600 11px/14px` `--text-2` in sentence case. Footer actions: buttons 32px high, `--r-md`, `600 12px`, primary = `--inverse-bg` on `--inverse-text`, secondary = `--tile-bg` with `--tile-border`, disabled opacity .4, no blue. Close: 24x24 drawn X at the header end, `aria-label="Close <title> card"`. Caret: 10px square rotated 45 degrees at the bottom edge, `left: var(--caret-x)`, same fill and border as the surface.

Placement for a bottom dock (physical pixels; replaces `desired_y = dock_position.1 - card_h - 10 * scale` at `main.rs:128`):
- `slotTop = dock.outer_position.y + anchor.y * scale`, where `anchor` is the slot rect (not the tile) in dock CSS pixels, sent at activation. The React code already passes the slot (`activate(item, event.currentTarget)`).
- `y = clamp(slotTop - cardH - 10 * scale, work.top, work.bottom - cardH)`.
- `x = clamp(centerX - cardW / 2, work.left + 12 * scale, work.right - cardW - 12 * scale)` with `centerX = dock.x + (anchor.x + anchor.width / 2) * scale`.
- `caretX = clamp(centerX - x, 18 * scale, cardW - 18 * scale) / scale`, added to the `widget-card:show` payload; the card sets `--caret-x` in CSS pixels.
- While a card is open its slot has `data-open="true"` and that tile rests at scale 1 and lift 0 (`.dock-slot[data-open="true"] .dock-widget { transform: none }`; landed names `.dock-cell` and `.dock-tile`), so the 10px gap is exact and the tile does not poke through the caret (V12). Other tiles keep magnifying.

Motion: `transform-origin: var(--caret-x) 100%`; from `opacity:0; transform: translateY(8px) scale(.97)`; opacity `var(--dur-fade)` ease, transform `var(--dur-pop) var(--ease-spring)`. No exit animation (the HTML has none; instant hide). Dismiss: Escape, window blur (existing `defer_widget_card_hide`), the close button, or activating the same slot again. Focus: the window takes focus on show; focus returns to the opening slot on hide. The slot has `aria-haspopup="dialog"` and `aria-expanded` mirrored from `widget-card:show` and a new `widget-card:hide` event.

### 4.8 Motion tokens and reduced motion

Springs in this product are the bezier below with the stated duration; there is no JS spring and no curve that overshoots (MO:58). Damping ratio 1.0 is the intent for all layout motion; the card's 0.82 in `.design.md` is approximated by the same curve.

| Motion | Duration and easing | Property | Source |
|---|---|---|---|
| Pointer magnification | `--dur-mag` 120ms ease-out | `transform` via `--mag` | HTML 475 |
| Tile hover surface | `--dur-hover` 140ms ease-out | background, border, shadow | HTML 476-478 |
| Hover lift | with magnification, `--lift` -4px | `transform` | HTML 58, 472 |
| Pointer-down | `--dur-press` 100ms ease-out, `scale(.96)`, release 120ms | `transform` on `:active` | `.design.md` 2 (V11) |
| Drag reorder FLIP | `--dur-reorder` 220ms `--ease-spring` | slot `translateX` | HTML 4263 |
| Profile switch, resize, Alt-click morph | `--dur-flip` 380ms `--ease-spring` | slot `translateX` (WAAPI), rail glass extents | HTML 4439, 4452-4460 |
| Entering slot | `--dur-enter` 320ms `--ease-spring` | opacity 0 to 1, scale .85 to 1 | HTML 4461-4469 |
| Leaving slot | none (removed at once) | n/a | HTML (no exit) |
| Card and dialog | opacity `--dur-fade` 180ms ease, transform `--dur-pop` 200ms `--ease-spring`, from `translateY(8px) scale(.97)` (dialog 12px) | opacity, transform | HTML 1119-1121, 1586-1588 |
| Backdrop | 180ms ease | opacity | HTML 1101 |
| Tooltip | `--dur-tip` 120ms ease-out | opacity, 4px rise | HTML 1076 |
| Shelf pills | `--dur-pill` 130ms ease-out | color, background | HTML 261 |
| EQ bars | 650ms alternate ease-in-out, delays 0, -220, -440ms | bar height 2px to 6px (the only height animation, inside a fixed 6px box) | HTML 675-678 |
| Working arc (claude, codex) | 1.4s linear infinite | rotate, no glow | `.design.md` 10 (glow removed, B5) |
| Loading pulse | 1.2s ease-in-out infinite | glyph opacity | 4.4 |

Rules. FLIP uses `Element.animate` on transforms only; the first rect comes from `getBoundingClientRect()` (an in-flight animation is therefore the new start, so motion is interruptible) and any running animation on the element is cancelled first. The rail does not animate `width`: the rail lays out at its final width at once and the glass surface animates its left and right extents through `--glass-l` and `--glass-r` (`transition: left, right var(--dur-flip) var(--ease-spring)`). The native window is fitted twice per morph, `fit(union(first, last))` at the start and `fit(last)` 400ms later, and the `ResizeObserver` fit path is skipped while `<body data-morphing>` is set (this keeps the per-frame IPC loop that `dock-defects` item 3 warns about from returning). Tile height never changes layout; magnification is a transform and the single permitted visual exception to the 48px lock.

`@media (prefers-reduced-motion: reduce)`: magnification, lift and pointer-down scale off; FLIP, rail morph and entering replaced by a single 120ms opacity fade of the rail contents; card and dialog fade only (120ms, no transform); tooltip fades only; EQ bars static at 6px; working arc static at a quarter arc; loading pulse static at .55; hover colour and border transitions stay (state feedback, AN:77). `@media (prefers-reduced-transparency: reduce)`: `--rail-fill`, `--popover-bg`, `--modal-bg`, `--tip-bg`, `--shelf-pill-bg` become opaque (`#0c1017` dark, `#ffffff` light, `#1b1d21` glass), `backdrop-filter: none`, and `--line` borders become 1.5px. Reduced motion is the only motion off-switch (CL-Q3).

### 4.9 Focus, keyboard, accessibility

- Focus ring on every interactive control: `outline: 2px solid var(--focus-ring); outline-offset: 2px`, drawn on the tile surface for slots, never animated, 16.9:1 on the dark tile and 16.5:1 on the light rail (SY:206-208). This replaces the HTML's white .55 and the old plan's blue `#0a84ff` (V10).
- Roles and names: the rail is `role="toolbar" aria-label="Dock widgets" aria-orientation="horizontal"`; each slot `aria-label` is "<title>, <live value or state suffix from 4.4>" and includes badge counts ("3 notifications"). One tab stop (roving tabindex).
- Keys on the rail: Left and Right move focus, Home and End jump, Enter or Space activates (open card or action), Shift+Enter toggles the in-place morph (Alt-click equivalent, V24), the Menu key or Shift+F10 opens the card (right-click equivalent). In edit mode: Enter cycles the width, Alt+Left and Alt+Right move the widget, Escape leaves edit mode. Escape priority (one handler): new-profile dialog, customizer, card, edit mode.
- Dialogs (customizer, new profile): `role="dialog" aria-modal="true"`, focus trapped, first field focused, focus returned to the opener on close.
- One visually hidden `role="status" aria-live="polite"` region announces profile switch, edit mode on and off, "moved to position 3 of 9", and "resized to 96 pixels" (replaces toasts, D9).
- Every pointer target is at least 24x24 CSS px (S9); tab order equals visual order; no `tabindex` above 0; every icon-only button has an `aria-label` (SY:199-200).

### 4.10 Controls the HTML leaves undefined

Authored here because the pinned CSS has no rules for them (S10, P1).

- Shelf order, left to right (HTML 2958-3000): profiles, theme, style, edit, customize; 8px between groups; 9px between shelf and rail; shelf height 28px.
- Shelf pill (all five groups): `display:flex; align-items:center; gap:2px; padding:3px; height:var(--shelf-h); border-radius:var(--r-pill); background:var(--shelf-pill-bg); border:1px solid var(--rail-border); backdrop-filter:blur(20px) saturate(140%)`. Tab inside: `height:22px; padding:0 11px; font:600 var(--fs-11)/22px var(--font); color:var(--text-2); border-radius:var(--r-pill); transition: color, background var(--dur-pill) ease-out`. Hover colour `--text`. Active: `color:var(--text); background:var(--active-pill-bg); box-shadow: inset 0 0 0 1px var(--tile-border)`. Theme tabs: Dark, Light, Liquid Glass with 10px icons. Style tabs: Capsules, Lines.
- "+" tab (inside the profile pill): 22x22 round button, 10px plus icon (stroke 3.0), `aria-label="Add profile"`, same hover as tabs; opens the new-profile dialog.
- Edit pill and Customize pill: height 28px, `padding:0 12px; gap:6px`, 10px icon, label `600 11px`. Edit has `aria-pressed`; pressed shows "Done" with an 11px check and `background:var(--active-pill-bg); color:var(--text)`.
- Edit-mode affordances: resize chip is a 16px circle in a 24px hit area at `top:-8px; right:-8px`, `background:var(--inverse-bg); color:var(--inverse-text)`, 10px glyph, `box-shadow:0 1px 3px rgba(0,0,0,.4)`, hover `scale(1.1)` 120ms, `aria-label="Resize <title>, <n> pixels wide"`. Dragged tile opacity .35; drop target `scale(.96)`; cursor `grab` and `grabbing`.
- New-profile dialog: backdrop `var(--scrim)` plus `blur(8px)` fading over 180ms; card 360px wide, `--r-pop`, `--modal-bg`, border `--line`, padding 16px, motion from `translateY(12px) scale(.97)` 200ms spring. Header: title `700 16px/20px`, 24px close. Sub-line `400 12px/16px --text-2`: "Create an isolated dock layout." Name field: label `600 12px`, input 36px high, `--r-md`, border `--tile-border`, focus ring. "Start from" radio group, four 36px rows: Current dock, Daily, Work, Focus. Footer buttons right-aligned: Cancel (secondary), Create (primary, disabled until valid). Validation: 1 to 24 characters, unique ignoring case; error text 11px `--bad` under the field. Enter submits, Escape cancels. Do not reuse `.ps-spotlight-backdrop`.
- Customizer (`WidgetManager`) changes: the width control is one segmented control of the widget's allowed steps (labels "48", "96", "140", "168", "220") replacing presets, slider and value badge; remove "Expand Rectangles", keep "All square" and "Reset defaults" (resets each widget to its default step); drop the Store tab (D10); the Apps tab lists installed apps from native; ordering by the existing drag grip with Alt+Up and Alt+Down.

### 4.11 Deviations from `dock-editable.html`

Each row stands unless the user replies with its veto line ("veto Vn"), which restores the HTML behaviour for that row.

| Id | Deviation | Why | Veto |
|---|---|---|---|
| V1 | Outline icons 24px, dials 28px (calendar 34 to 28), app icons 24px, in a 32px glyph box; HTML uses 13 to 34px | S8 inconsistency; `.design.md` 3.2 cap of 24; even insets for device-pixel alignment (D4) | Keep per-widget HTML sizes |
| V2 | Eight-step type scale, weights 400 to 700, nothing under 10px | S1 | Keep HTML sizes |
| V3 | Radius ladder | S2 | Keep HTML radii |
| V4 | Colour roles: neutral glyphs, neutral ring and bars, no amber or green glyph tints, single clock accent | B5, S3, CL-Q4 | Per widget, restore its HTML colour |
| V5 | Dark `--text-3` `#6b7280` to `#8a93a0`; light tokens completed (`--glyph`, `--text-3`) | S4 contrast and broken Light theme | None offered (accessibility) |
| V6 | Liquid Glass rail `rgba(22,24,28,.66)`, tiles .08, instead of .04 to .14 clear glass (D1) | S5, 1.10:1 on a white desktop; no backdrop to blur | Restore HTML glass and accept it only reads on dark wallpapers |
| V7 | No outer rail or shelf shadow; no rest shadow on tiles (hover shadow kept) | B7; region clips outer shadows | Restore shadows |
| V8 | Rect tiles do not scale on magnification (they still lift and take hover); frame-batched reads | S11 overlap with neighbours | Scale all tiles |
| V9 | `will-change: transform` only while hot | S11, CL-Q2 | Permanent `will-change` |
| V10 | Neutral focus ring `--focus-ring` | HTML ring is invisible in Light; old plan used blue | Blue `#0a84ff` or white .55 |
| V11 | Pointer-down `scale(.96)` 100ms added | `.design.md` and spec require it; HTML lacks it | Omit |
| V12 | Card transform-origin at the caret; anchor tile rests while its card is open; native placement from the slot top plus caret (replaces "above the window top") | S12, 4.7 | HTML origin; place above window top |
| V13 | Theme-aware tooltip with optional second status line | S4, 4.4 | Single dark line |
| V14 | Running dot `--text`, no glow, `bottom:3px`, 4px (landed outside placement dropped) | Light theme 1.19:1 | Landed outside dot |
| V15 | Count badge 16px, 10px text, `--bad-fill` `#d92d20`, ring border | 14px/8px text and 3.55:1 fail | HTML badge |
| V16 | Five width steps with per-widget allowed sets; weather and stats default 220 (HTML 200, 196); no slider (D8) | S7 | Keep the slider and HTML defaults |
| V17 | Authored Edit, "+" and New-profile controls and one shelf pill recipe (4.10) | S10, P1, P2 | n/a (no HTML to restore) |
| V18 | No toast HUD; `aria-live` region instead (D9) | P5 | Port toasts into a visible host |
| V19 | No Widget Store tab (D10) | B9 | Keep the tab |
| V20 | Drawn SVG glyphs instead of emoji and symbol characters | B6 | Keep characters |
| V21 | Assistant: id `assistant` (alias `siri-ai`), neutral robot outline, no orb (D3) | B3 | Restore Siri naming and orb |
| V22 | No text inside the sparkline SVG; numbers in the tooltip | S1, illegible 5.5 to 7.5px | Keep SVG text |
| V23 | 24px minimum targets (music buttons 24px, chip 24px target) | S9 | Keep 20px |
| V24 | Roving-tabindex toolbar, Shift+Enter morph, Menu key opens the card | 4.9; HTML has none | Tab through every slot |
| V25 | Shelf fades while the rail is hot (landed behaviour kept; D6) | Tooltip and magnified tiles occupy the shelf zone | Shelf always visible |
| V26 | Calendar card flat `#e11d48`, no gradient, no coloured shadow (D2) | B4, CL-Q4 | Restore gradient and shadow |
| V27 | Empty running-apps and pinned-apps collapse in normal mode; dashed placeholder only in edit mode (HTML 4313-4322) | Landed always shows a dim placeholder | Keep placeholder |
| V28 | App icons are native rasters at DPR, never the HTML's hand-drawn brand look-alikes (3931-3950) | IC:3-5, CL-Q2 | None offered |

## 5. Reconciliation with code already on this branch

Verdicts: KEEP, ADAPT (exact change given), DELETE. Nothing here redoes `fa60e4b` (jitter) or `a3d9e11` (icon extraction); where a change touches the same lines the note says what must stay.

### 5.1 Magnification: HTML versus `magnify.ts`

| | HTML (4777-4803, tokens 56-57) | Landed `magnify.ts` and `fit.ts` |
|---|---|---|
| Falloff | `1 + 0.22 * (1 - d / 86)^2`, zero beyond 86px | `1 + 0.55 * (1 + cos(pi * d / 150)) / 2`, zero beyond 150px |
| Peak scale | 1.22 | 1.55 (`MAG_RISE_PCT 55`) |
| Neighbour at 54px (tile plus gap) | 1.03 | about 1.40 |
| Neighbours | not moved; the 6px gap absorbs the 5.3px growth of a 48px tile | spread by `--shift`; rail glass widened by `--mag-before` and `--mag-after` |
| Transform | `translateY(--lift-y) scale(--mag)` about the bottom centre, 120ms ease-out, lift -4px on hover | `translate(--shift) scale(--mag)` about the bottom centre, 140ms spring |
| Wide tiles | scaled (overlaps neighbours, S11) | scaled |
| Cost | per-slot rect read per event, unthrottled | frame-batched (keep) |

Decision: the HTML's numbers win (V8 adds the wide-tile exception). Consequence for the jitter fix: with no neighbour spreading the rail never grows sideways on hover, so the horizontal hit-region allowance becomes a constant and only the vertical headroom changes with `hot`; this removes the `-40px` region jump documented in the defects doc (hot path) at its source.

### 5.2 Files

| Item | Verdict | Exactly how |
|---|---|---|
| `src/components/dock/sizing.ts` and `sizing.test.ts` | DELETE | Its model (global icon size 32 to 64, per-widget `tileWidth` and `tileHeight`, `BASE_TILE 46`) conflicts with the locked 48px height. `sizeOf(_item)` at `DockSurface.tsx:32` ignores the item and returns `DEFAULT_TILE`, and `DockItem` has no `tileWidth` or `tileHeight`, so no saved data depends on it. Replace with `widthSteps.ts` exporting `TILE_H = 48`, `WIDTH_STEPS = [48, 96, 140, 168, 220]`, the allowed and default step per widget id from 4.3, `isAllowedStep(id, n)`, `nextStep(id, current)` (ascending, wraps), `shapeOf(width)` (`48` is `square`, else `rect`) and `slotStyle(width)` returning `{'--w': '<n>px'}`. Remove `--dock-glyph-scale` and its rule `dock-tiles.css:153-158`. |
| `src/components/dock/fit.ts` | ADAPT | `MAG_RISE_PCT 55` becomes `MAG_STRENGTH = 0.22`, `MAG_MAX = 1.22` (still exported for `magnify.ts`). `magHeadroom = ceil(MAG_STRENGTH * 48) + 4 + 10 + 38 = 63` (peak growth 11, lift 4, tooltip gap 10, two-line tooltip 38; the landed value is 64) and the top extent is `max(shelf height 28 + shelf gap 9, headroom)`. `magAlong` becomes the constant `ALONG_PAD = 12` (end-tile growth 5.3 plus margin) applied to both sides and never to `hot`. In `dockFit`, `hot` changes only `r.y` and `r.height` (`r.y -= headroom; r.height += headroom`); delete the `along` terms from the hot branches (`fit.ts:25-26`) and from `width`. Shelf region radius 15 becomes 14, rail 18 stays. Keep `edge` and `align` parameters and their tests (D5). Update `fit.test.ts`. |
| `src/components/dock/magnify.ts` and test | ADAPT | `MAG_RADIUS = 86`; `magnifyScale(d) = d >= 86 ? 1 : 1 + 0.22 * (1 - d / 86) ** 2`; delete `magnifyLayout` shifts and totals and the `--mag-before`, `--mag-after`, `--shift` writes; write only `--mag`, and only for cells whose shape is `square` (skip `rect`, V8). Keep the rAF batching, the `data-dragging` guard and the reduced-motion early return. Update `magnify.test.tsx` (peak 1.22 at d 0, 1.03 at d 54, 1 at d 86, rect cells untouched). |
| `src/components/dock/DockGroup.tsx` | KEEP, ADAPT | Keep one 48px square tile per app (HTML 4295-4311, `.design.md` 3.1). Change `AppGlyph size={32}` to 24 with the DPR request (4.5); add the count badge and the dot per 4.6; tooltip gets the optional second line; `aria-label` includes badge count; the empty group renders nothing in normal mode and a dashed 48px placeholder only in edit mode (V27) instead of the always-visible `is-off` tile. |
| `src/widgets/final/DockWidget.tsx` glyph tiles | ADAPT | Replace the `switch` with a registry `glyphs/index.ts` keyed by widget id (one file per widget family, work package U1.3). Remove literal sizes 22, 20, 16 in favour of the glyph classes of 4.3; derive label and `data-state` from `uiState` (4.4); keep `settingsFor`, data hooks and `preview` handling. |
| `src/design/dock-tiles.css` | ADAPT | (a) Move the `:root` block to the tokens file; delete `--dock-pop` and the blue `--dock-focus`. (b) `.dock-tile` background becomes `var(--tile-bg)`, border `var(--tile-border)`, shadow `var(--tile-edge)` only, size `var(--w) x var(--tile-h)`, radius `var(--r-tile)`; delete the tint gradient (lines 134-138) and `color: var(--fw-tint)`. (c) `.dock-final-rail` padding 7 becomes `var(--dock-pad)`, height `var(--rail-h)`; the glass moves onto the rail itself using the roles of 4.2; delete the `::after` extents and the `--mag-*` transitions (lines 52-75). (d) `.dock-tip`, `.dock-dot`, `.dock-badge` per 4.6. (e) Delete permanent `will-change`. (f) Extend the reduced-motion block per 4.8. (g) Keep the tooltip magnification compensation (line 353). |
| `src/widgets/final/glyphs.tsx` | ADAPT | `Ring`: render 28px, `r = 12`, stroke 2.4, neutral fill `var(--text)` (warn colour past `warnAt` via `--warn`), new `working` prop (rotating quarter arc, 1.4s linear, no glow). `ClockFace`: 28px, 12 ticks (HTML 4056-4069), colours from `--dial-face` and `--dial-ink`, second hand and pin `--clock-accent`; keep the time maths (it is correct, unlike the HTML). |
| `src/components/dock/DockSurface.tsx` hot state | ADAPT | Keep `heat` and `relax` and the `lastFit` dedupe and the `clientWidth`, `clientHeight` sizing from `fa60e4b`. `hot` now drives only the vertical headroom region and the shelf fade (V25). Add the `data-morphing` guard and the two-phase fit of 4.8. Replace `sizeOf` with the width step. Slots keep `key={item.id}` (stable keys let FLIP move shared widgets). If the peer's smallest fix for defect 3 lands first (`key` by profile plus `dock-slot-in`), work package U2.3 reverts the profile key and deletes `dock-slot-in`. |
| `src/components/dock/layout.ts` | ADAPT | Schema v3 (5.4). |
| `src/design/native.css:349-380` | ADAPT, then DELETE if dead | The sage theme (`--accent: var(--sage)`) is a second token system and a green accent. Confirm the `.dock-stage .dock-shell` rules are dead (the live dock uses `.dock-final`), then delete them; keep only what the settings window still uses. |
| `src/components/settings/WidgetManager.tsx` | ADAPT | `SizeControl` (compact, standard, wide) becomes the allowed-steps segmented control (4.10); preview strip reuses the tile components. |
| `src/features/widget-card/WidgetCardWindow.tsx`, `main.rs:98-135, 524-596` | ADAPT | Placement and caret per 4.7; keep the Escape handler and the allowlists (`valid_widget_card_id`). |
| `src/features/search/SpotlightWindow.tsx` | KEEP | Gaps against the HTML are optional work package U5. |
| `src/widgets/final/icons.tsx:8-29` | ADAPT | Windows glyph becomes the HTML flag geometry (3733) solid in `currentColor`; Apps glyph stays visually distinct. |

### 5.3 Widget id catalogue

The HTML has 19 widgets, React has 14, six overlap by meaning.

| HTML id | React id | Action |
|---|---|---|
| start | windows | rename, read alias `windows` |
| search | search | keep |
| running-apps | apps | rename, read alias `apps`; keep settings `activeOnly`, `maxApps` |
| pinned-apps, calendar, peripherals, status-battery, wifi, bluetooth, notifications, quick-notes, system-tray | none | new |
| weather, clock, claude | same | keep |
| system-stats | system | rename, read alias `system` |
| music-player | spotify | rename, read alias `spotify` |
| codex | none (`chatgpt` is a different provider) | new; keep `chatgpt` (check `ai_usage.rs` before merging) |
| assistant (`siri-ai`) | none | new, read alias `siri-ai` |
| none | agent, files, folders, trash, volume, chatgpt | keep as "More widgets" (D7), generic anatomy, restyled to tokens |

### 5.4 Saved layout schema: v2 today, v3 needed

v2 (`layout.ts:30-37`): `version 2`, `profile: ProfileId` (four literals), `profiles: Record<ProfileId, DockItem[]>`, `unknown`, `appearances?`, `theme?`; `DockItem { id, size?: compact | standard | wide, settings? }`. The new design needs custom profiles, one width from five steps, and pinned apps.

```ts
interface DockLayoutV3 {
  version: 3;
  profile: string;                              // active id; built-ins daily, work, founder, focus
  profiles: Record<string, DockItem[]>;         // built-ins always present
  profileNames?: Record<string, string>;        // custom display names (built-ins use PROFILE_LABELS)
  profileOrder?: string[];                      // tab order; default built-ins then creation order
  appearances?: Record<string, 'capsules' | 'lines'>;
  theme?: 'light' | 'dark' | 'glass';
  pinnedApps?: string[];                        // stable app ids, global as in the HTML
  unknown: string[];                            // unchanged
}
interface DockItem { id: WidgetId; width?: 48 | 96 | 140 | 168 | 220; settings?: Record<string, unknown> }
```

Rules. The reader accepts v1, v2 and v3; the writer emits v3 only. Before the first v3 write, store the original v2 JSON once as `layoutV2Backup` in the same settings document (never overwritten). `size` is read for migration and never written. Migration of `size` to `width` (then snapped to the nearest allowed step, ties to the lower):

| Widget | compact | standard | wide |
|---|---|---|---|
| weather | 96 | 140 | 220 |
| system-stats | 96 | 220 | 220 |
| music-player | 48 | 168 | 220 |
| all others | 96 | 140 | 168 |

Items without `size` take the widget default. Items whose id is not in the catalogue move to `unknown` (existing behaviour); nothing is deleted. Legacy ids map through 5.3. Update the strict key whitelist in `isLayout` (`layout.ts:206`) to include `profileNames`, `profileOrder`, `pinnedApps`. `PROFILE_IDS` becomes `BUILTIN_PROFILE_IDS`; `isProfileId` checks `layout.profiles` keys. The dock-level edge and alignment are not stored (D5). Edit mode is not stored.

## 6. Dock-level behaviour beyond the HTML

| # | Topic | HTML covers? | Recommended default | User decision? |
|---|---|---|---|---|
| 1 | Dock edge (bottom, left, right) | Bottom only (`.dock-stage { bottom: 16px }`, 209-220) | Bottom in v1. `fit.ts` keeps the `edge` type but nothing exposes it. A vertical dock needs a rule for 48px-high tiles of mixed width (a sidebar of left-aligned rows) that no design exists for. | Yes (D5) |
| 2 | Dock alignment along the edge | Centred (`align-items: center`, 217) | Centred | Yes (D5) |
| 3 | Widget alignment inside the rail | Left to right in saved order, no spacer | Same; no spacer or separator widget in v1 | Yes |
| 4 | Widgets placed anywhere (Start in the middle) | Yes: any order by edit-mode drag (4668-4770) and the customizer order buttons; Start is first only by default (3975) | No constraints; Start is an ordinary widget; the Daily default keeps it first | No |
| 5 | Native window fit | No (browser) | Window = union of shelf and rail plus margins: width `max(shelf, rail) + 2 * 12`, height `BOTTOM_INSET 14 + rail 60 + top extent (headroom 63 from 5.2 or the shelf stack 37, whichever is larger) + TOP_MARGIN 8`; bottom of the rail 16px above the work-area bottom (HTML 213) | No |
| 6 | Hit regions | No | Only two rounded regions receive input: shelf (radius 14) and rail (radius 18). `hot` adds a strip of `headroom` above the rail over the rail's own x-extent; it never changes x or width, so tooltips are clamped inside that extent (4.6). Everything else in the window passes clicks through (`SetWindowRgn`). Regions never change while idle. | No |
| 7 | Taskbar replacement and work area | No | Out of UI scope; recommend reserving the work area with an AppBar so maximised windows stop above the dock and hiding the system taskbar as an explicit setting | Yes |
| 8 | Overflow when widgets exceed the work width (Work profile plus running apps) | No (`overflow: visible`, S13) | Render-time only: step the widest widgets down one step at a time (220, 168, 140, 96, 48) until the rail fits `workWidth - 32`; if all are 48 and it still overflows, running-app tiles beyond the limit collapse into one "+N" tile that opens a card. Nothing is saved. | Yes |
| 9 | Multi-monitor | No | Primary monitor; icons and fit re-request on DPR change | Yes |
| 10 | Auto-hide, fullscreen | No | Always on top; hidden while a fullscreen window is foreground (check `dock_lifecycle.rs`); no auto-hide in v1 | Yes |
| 11 | Active window indicator | No (one dot for all running apps) | Running = 4px dot; foreground app = 12x4 pill, same colour | Yes |
| 12 | Right-click | No | Opens the widget card (landed behaviour); no jump lists in v1 | No |

## 7. Quality-bar mapping and native acceptance

Native checks use Computer Use on the real app. Blocker on 2026-11-08 (defects doc): the `computer-use` skill needs the `orca` CLI and it is not on PATH. If it still is not, record the exact blocker in `docs/qa/STATUS.md` and leave the gate unverified; never mark it verified without an observation. Browser captures do not count.

| `CLAUDE.md` item and defect | Already landed (verify, do not redo) | UI work packages | Native acceptance checks |
|---|---|---|---|
| CL-Q1 no jitter, defect 1 | `fa60e4b`: dedupe of identical fit requests, sizing from `clientWidth` and `clientHeight`, `place` sets size and position only when changed | U2.1 (constant horizontal allowance, `hot` only vertical), U2.5 (two-phase fit), U1.2b (fixed 60px rail) | (a) Idle 60s, one screenshot every 2s: rail left and right edge pixel columns identical in all 30. (b) Ten hover enter and leave cycles on tile 3: edge columns never move. (c) Daily to Focus and back five times: three consecutive screenshots 1s apart equal after the settle. (d) `fit_dock` log: 0 calls idle, at most 2 per hover cycle, exactly 2 per morph. (e) Repeat at 100, 125, 150 percent. Tests from the defects doc must still pass (rail mock `scrollHeight` 86 keeps height 146; no second `fit_dock` after `resize`). |
| CL-Q2 crisp icons, defect 2 | `a3d9e11`: large-first HICON order, exe fallback `PrivateExtractIconsW`, natural-size PNG | U1.4 (DPR request, 24px box), U2.2 (`will-change` only while hot), Rust follow-up for sources under 64px | DevTools `naturalWidth >= ceil(24 * 1.22 * dpr)` at 100, 125, 150; Computer Use zoom of Notepad's tile beside the Windows taskbar icon at the same scale, plus a hover-peak shot; compare edge sharpness; record screenshots. |
| CL-Q3 motion preserved, defect 3 | none (animation was deleted in `e6727e0`, `8e9029c`, `ded8910`) | Peer's smallest fix (entrance animation, profile key) then U2.3 (FLIP), U2.4 (lift, press), U2.5 (rail morph), U2.6 (reduced motion) | Profile switch: `getAnimations().length > 0` within 50ms (Vitest with a mocked `animate`) and a six-shot burst over 500ms showing intermediate widths; hover lift visible; with Windows animations off (reduced motion) the same switch shows no intermediate widths. |
| CL-Q4 restrained surfaces and solid Windows logo, defect 4 | none | U1.1 (neutral tile), U0.2 (flag), U1.3.* (no tinted sub-surfaces), U4.2 (card surfaces; rows already listed in the defects doc, do not redo them) | Sample the tile background of all 19 widgets in Dark: channel spread 6 or less, calendar fill the only coloured sub-surface; Windows logo pixels fully opaque over a bright desktop; Light and Glass screenshots over a white and a black desktop. |
| CL-Q5 glyph quality, defect 5 | none | U0.2, U1.3.a to U1.3.h | Glyph sheet at tile sizes 32, 48, 64 and 96 CSS px (Customizer preview strip scaled) compared side by side with `preview-tahoe-glass.html`; checklist: effective stroke 1.5px, optical centre within 1px of the tile centre, no clipped glyph, Windows and Apps glyphs clearly different. |

## 8. Ordered UI work packages (seeds)

Seeds only: the plan author writes the exact task specs. Size is in Haiku tasks of 1 to 3 files. "Parallel" means safe to run beside the other packages of the same phase (disjoint files). Gates for every package: `npx tsc -b`, `npm test -- --reporter=dot`, plus the acceptance below.

| Id | Goal | Files | Depends on | Parallel | Size | Acceptance |
|---|---|---|---|---|---|---|
| U0.1 | Tokens file, import last, remove old `:root` | `src/design/dock-tokens.css` (new), `src/main.tsx`, `src/design/dock-tiles.css` | none | no (all depend) | 2 | `dock-tokens.test.ts` reads the CSS and asserts every token name and value in 4.1 and the three theme blocks in 4.2 |
| U0.2 | Icon stroke rule and Windows flag | `src/widgets/final/icons.tsx`, `src/widgets/final/glyphs.test.tsx` | U0.1 | yes (with U0.3, U0.4) | 2 | Every `Icon` has `viewBox 24` and `strokeWidth = 36 / size`; Windows glyph is solid `currentColor` with no stroke; Apps glyph differs |
| U0.3 | UI state model | `src/widgets/final/state.ts` (new), `src/widgets/final/types.ts`, `src/design/dock-tiles.css` | U0.1 | yes | 2 | Mapping test for every row of 4.4; CSS has a rule per `data-state` |
| U0.4 | Pin the HTML | `docs/design/dock-editable.2126c49.html` (copy), `scripts/html-pin.test.mjs` | none | yes | 1 | `node scripts/html-pin.test.mjs` asserts the blob hash and the 1.4 facts (plain Node, not Vitest) |
| U1.1 | Neutral 48px tile, rail geometry | `src/design/dock-tiles.css` | U0.1 | no (serial on this file) | 2 | No tint gradient on `.dock-tile`; 48px, 12px radius; rail 60px; CSS-text assertions; native CL-Q4 sampling |
| U1.2 | Width steps and schema v3 | `src/components/dock/widthSteps.ts` (new), `src/components/dock/layout.ts`, delete `sizing.ts` and `sizing.test.ts` | U0.1 | no (`layout.ts`) | 3 | Migration tests v1 and v2 to v3 (table 5.4), unknown ids preserved, backup written once, invalid width rejected, `isLayout` whitelist |
| U1.2b | Wire widths into the surface | `src/components/dock/DockSurface.tsx`, `src/design/dock-tiles.css` | U1.1, U1.2 | no | 2 | Slot style `--w`, `data-shape` square or rect; existing DockSurface tests updated |
| U1.3.0 | Glyph registry split | `src/widgets/final/glyphs/index.ts` (new), `src/widgets/final/DockWidget.tsx` | U0.2, U0.3 | no (first of U1.3) | 2 | `final-widgets.test.tsx` still passes; registry has one entry per widget id |
| U1.3.a to U1.3.h | One glyph family each: a outline (start, search, assistant, system-tray, wifi, bluetooth, quick-notes); b dials (clock, calendar); c state (battery, peripherals, notifications); d weather; e system-stats; f music; g claude and codex rings; h apps | `src/widgets/final/glyphs/<family>.tsx` plus test each (b and g also `glyphs.tsx`; h also `DockGroup.tsx`) | U1.3.0 | yes (disjoint files) | 2 each | Per 4.3: sizes, classes, `currentColor`, no literal hue except 4.2 roles; clock hands follow `now`; sparkline has no SVG text; no emoji (test enforces) |
| U1.4 | App icons at DPR | `DockGroup.tsx`, `src/widgets/final/primitives.tsx` (`AppGlyph`), `src/services/native.ts`, `src-tauri/src/running_apps.rs` | U0.1 | no | 4 | Ladder test for dpr 1, 1.25, 1.5, 1.75, 2 gives 32, 48, 48, 64, 64; Rust test: PNG IHDR equals the requested size and no upscaling; native CL-Q2 |
| U1.5 | Badge, dot, state dot, tooltip | `dock-tiles.css`, `DockGroup.tsx`, `DockSurface.tsx` | U1.1, U0.3 | no | 2 | Rules of 4.6 present; count text `99+`; tooltip transform compensates `--mag` and `--lift-y` |
| U2.1 | Fit constants, no horizontal hot change | `fit.ts`, `fit.test.ts` | U1.1 | yes | 1 | Headroom 63 at 48px; region x and width equal hot and idle; dedupe untouched |
| U2.2 | Magnification to the HTML numbers | `magnify.ts`, `magnify.test.tsx`, `dock-tiles.css` | U2.1 | no | 2 | `scale(0)` 1.22, `scale(54)` 1.03, `scale(86)` 1; rect cells untouched; no `--shift` |
| U2.3 | FLIP and entering animation | `src/components/dock/useFlip.ts` (new), `DockSurface.tsx` | U1.2b | no | 3 | With mocked `animate`: reorder (220ms), profile switch (380ms), enter (320ms, opacity and scale .85); previous animation cancelled; none under reduced motion; profile key reverted |
| U2.4 | Hover lift and pointer-down | `dock-tiles.css` | U1.1 | yes | 1 | Lift -4px on hover, `scale(.96)` on `:active`, durations per 4.8 |
| U2.5 | Rail morph with two-phase fit | `DockSurface.tsx`, `fit.ts`, `dock-tiles.css` | U2.3, U2.1 | no | 3 | During a morph `fitDock` is called exactly twice (union then last) and never by the observer; `data-morphing` set and cleared |
| U2.6 | Reduced motion and transparency | `dock-tiles.css`, `dock-tokens.css` | U2.2 to U2.5 | no | 1 | Both media blocks present with every override of 4.8 |
| U2.7 | Edit mode, drag reorder, width cycle, morph | `src/components/dock/useEditMode.ts` (new), `DockSurface.tsx`, `dock-tiles.css` | U2.3, U1.2b | no | 4 | Cycle walks the allowed steps; Alt-click and Shift+Enter toggle 48 and default; drag reorder persists through `moveWidget`; Escape priority |
| U2.8 | Rail keyboard model, live region | `src/components/dock/useRailKeys.ts` (new), `DockSurface.tsx` | U1.2b | no | 2 | Roving tabindex, Home and End, Enter, Shift+Enter, Menu key; live region text for switch, edit, move, resize |
| U3.1 | Shelf pills (theme, style, Edit, +, Customize) | `DockSurface.tsx`, `src/design/dock-final.css` | U0.1, U2.7 | no | 3 | One pill recipe (4.10); `aria-pressed`; Edit toggles Done |
| U3.2 | New-profile dialog | `src/components/dock/NewProfileDialog.tsx` (new), `layout.ts` (add profile action) | U1.2 | yes | 3 | Validation 1 to 24 chars, unique; focus trap and return; Enter and Escape |
| U3.3 | Customizer width control and Apps tab | `src/components/settings/WidgetManager.tsx`, `src/design/dock-final.css` | U1.2 | yes | 3 | Segmented control of allowed steps; no slider; Store tab absent |
| U4.1 | Card placement and caret | `src-tauri/src/main.rs` (`widget_card_position`, `caret_x` in the payload), `src/features/widget-card/WidgetCardWindow.tsx`, `src/design/dock-final.css` | none | yes | 3 | Rust unit tests: centre, left and right clamp, `y` from slot top; TS test sets `--caret-x`; `data-open` rests the tile |
| U4.2 | Card anatomy and surfaces | `dock-final.css`, `src/widgets/final/final-widgets.css` (rows already in the defects doc), `WidgetCard.tsx` | U0.1 | no (shared CSS with the peer) | 3 | 4.7 anatomy; no tinted fills; sentence-case section titles |
| U4.3 | Remove fabricated data paths | `src/widgets/final/sample.ts`, `WidgetCard.tsx`, `DockWidget.tsx` | U0.3 | no | 2 | Native path never reads `sampleFor`; preview tiles carry `data-sample="true"` and a "Sample" label; no simulate controls remain |
| U5 (optional) | Spotlight parity: Quick Look drawer, list or grid, calculator hero, engine footer | `src/features/search/SpotlightWindow.tsx`, its CSS | U0.1 | yes | 3 | Behaviour matches HTML 5953-6040 at the pinned blob; Escape and arrow keys intact; Rust engine stays authoritative |
| U6 | Native verification | `docs/qa/STATUS.md`, `docs/qa/` captures | U1 to U4 | no | 1 | Section 7 checks recorded with screenshots, or the exact blocker; nothing marked verified without an observation |

Order: U0 (U0.1 first), then U1.1 and U1.2, then U1.2b, U1.3.*, U1.4, U1.5; then U2.1 to U2.8; U3 and U4 can start after U1.2; U5 last; U6 closes each phase. Kill `protondock` and `protonsearch` before builds; launch from a temp copy (WebView2 `0x800700AA`); never commit unrelated worktree changes.

