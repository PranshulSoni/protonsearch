# Island visual/UX fix — binding spec (task-56)

**Status:** binding, 2026-11-10. **Owner:** L1-B. **Scope:** island surface, enter/exit motion, notifications, Quick
Settings, island search parity. **Supersedes conflicting older text in ISLAND_V9_SPEC.md §5/§6.** No code changes here.
**Trigger:** user rejection of the island's visual/UX quality: bleed/translucency artifacts, overlapping notifications,
Quick Settings showing "Unsupported", and search that does not match Normal Launcher.

## 1. Opaque island surfaces — no bleed, no translucency

- The island is a solid graphite object. Surface background **#0d1012**; border **1 px #2e3538**; every card, row, panel,
  tile and footer uses an **opaque solid token**: #0d1012, #101416, #171e23, #1c2931, #223a49, #202d35. No rgba/alpha
  panel backgrounds, no backdrop-filter, no acrylic/glass, no transparency over text.
- Alpha is permitted only for the outer shadow, 1 px borders/hairlines and focus rings.
- Do **not** animate surface opacity at any time (show, hide, card change, resize). Remove every opacity transition on
  the island surface, inner shell and card shells. Content-only fades are allowed only inside an already opaque surface.
- Add isolation:isolate, contain:paint and background-clip:padding-box to the island surface; keep the border solid so
  the rounded edge has no alpha fringe.
- Native window: transparent only in the 112 px shadow padding outside the surface. Size/place the window before showing;
  the surface must already be painted at full opacity. No white/desktop flash, no ghost frame, no translucent first frame.
- Acceptance over a light/text window (Notepad/white document behind): inside the island bounding box sample pixels must
  match the opaque surface/card colour; no desktop text, previous card or blended content may be visible. Only the outer
  shadow pad may expose the desktop.

## 2. Enter/exit animation — transform only, never fade

| Phase | Exact values |
|---|---|
| Show | **180 ms**; transform **translateY(-8px) scale(.97)** to **translateY(0) scale(1)**; opacity constant 1; easing **cubic-bezier(.22,1,.36,1)** |
| Hide | **120 ms**; transform **translateY(0) scale(1)** to **translateY(-6px) scale(.985)**; opacity constant 1; easing **cubic-bezier(.4,0,.2,1)** then native hide |
| Re-summon during exit | Cancel exit from the current transform; latest input wins; no queued/stale transition |
| Reduced motion | 0 ms, no transform; show/hide after the window is positioned; content instant |
| Focus | Search input focused after show completes (no later than 190 ms); focus must not be lost mid-animation |
| Native | Window size/position set before show; hide after exit completes or 120 ms fallback; no fade-through |

Card-to-card morph keeps the V9 550/500/450 ms family. Enter/exit is the separate 180/120 family above. If the current
implementation fades the surface in/out, remove that opacity transition entirely.

## 3. Notifications anatomy — fixed grid, no overlap

Card **500×345** (>520) / **385** (≤520), radius 34; header + divider + scrollable list + fixed footer; 31 px dots footer.

- Header: "Notifications" 16/700; subtitle "Recent activity · {n} unread" 11 ink-2 (singular "1 unread"; when 0:
  "Recent activity").
- Row grid: **18 px | minmax(0,1fr) | 30 px**, gap 10; row **52 px** fixed, padding 9 8, bottom rule 1 px #262b30.
  Bell icon 18; title 12/600 single-line ellipsis; body 10/1.4 ink-2 max 2 lines; source/meta 10 ink-3 single line.
  Dismiss is a **30×30** button in the reserved right column (icon 15), never overlapping text or footer; hover #293941.
- List viewport max-height **156 px** (independent scroll); footer action row below, margin-top 8, height 32. "Clear all"
  secondary button only when a real command exists. Test-only controls (e.g. "Test local toast") must be gated out of
  production. No absolutely positioned actions over rows.
- Loading: 3 flat skeleton rows at 52 px (no shimmer). Empty: centred "All caught up." / "New notifications will appear
  here." Error: "Notifications are unavailable." / reason / Retry if supported. Dismiss unavailable: hide it or show a
  clearly disabled button with title "Dismissing notifications isn't available in this build."; never a silent dead ×.
- 0/1/many: list scrolls, footer stays fixed, no row crosses the footer or dots. Copy uses correct singular/plural
  ("1 unread"), real titles/bodies/sources only; no fake counts.

## 4. Quick Settings — real controls + Windows settings fallback

Card **515×376** (>520) / **400** (≤520) / **483** (≤380); drill-down **515×366** / **418**. Header + divider, then:

1. **Volume row 40 px**: speaker icon 19 · label "Volume" 11 (label column 68) · range flex (height 16) · value 10
   tabular ("62%") · optional Mute button 28 high, padding 0 8, secondary, label "Mute"/"Unmute".
2. **Brightness row 40 px**: sun icon 19 · "Brightness" · range · "80%".
3. **Audio output row 40 px**: monitor/speaker icon 16 · "Audio output" 11 + current device 12 ellipsis · chevron 16;
   opens the output list (choose rows 47, real setter, 3 devices). Never fake a device.
4. **Split tiles** 2 columns gap 9; tile **61** high (58 ≤520, 50 ≤380), radius 10, bg #171e23 border #34434a; on
   #223a49 border #416178. Left toggle flex 1, padding 9 11, icon 19, label 11, status 10 On/Off. Right **chevron
   button 36 wide (27 ≤520)** with a 1 px #4b606c left border; separate accessible name "Show Wi-Fi networks" /
   "Show Bluetooth devices"; must not toggle. Tiles: Wi-Fi, Bluetooth.
5. **Windows fallback row**: full-width secondary button **36 px** high, settings icon 16, label "Open Windows settings"
   12/600, sub 10 "Network, sound and display." Opens ms-settings: through the shell open-settings action.

- Never render the word "Unsupported", a disabled blank tile or a dead toggle. Replace every genuinely unavailable
  setting (Airplane mode, energy saver, night light, casting) with the fallback row. If one control is unavailable, label
  it "{Control} can't be changed here." + "Open Windows settings."; the fallback button is always real.
- Fit at 376: header/divider 55 + rows 40+40+40 (gap 8) + tiles 61 + fallback 36 + padding 31 = 303 px inside the 345 px
  inner box. No clipping; body scroll only if ≤380 content must stack, with a visible thin scrollbar.
- Setter failure: quiet inline "Couldn't change {control}." / "Open Windows settings."; no dead slider, no fake value.
  Loading: skeleton values + disabled controls with aria-busy. Error: quiet state + fallback. Unsupported: fallback only.
- Wi-Fi/Bluetooth drill-down: rows 47, icon 18, title 12, meta 10, tail chevron; back 31; real connect action; loading,
  error and empty states ("No networks found." / "No devices found."); no fabricated networks/devices.
- Accessibility: ranges labelled with value text; split toggle and chevron separate names/states; fallback button real;
  Space toggles, arrows adjust sliders, chevron opens, Esc/back returns.

## 5. Island Search parity with Normal Launcher

Island expanded search is a **presentation variant of the same launcher search model**, not a second implementation.

| Aspect | Binding rule |
|---|---|
| Request | Same api.search request, debounce/epoch/cancel/ranking rules as Normal Launcher. |
| Filters | Same **All · Files · Folders · Apps** labels/order/scopes (no Actions chip; actions stay in ALL/prefixes). |
| Grouping/order | Use shared groupResults: APPLICATIONS · FILES · FOLDERS · DOCUMENTS · CLIPBOARD · WEB · CODE · ACTIONS; only non-empty; no client reordering. Visible group labels 10/700/+0.10em ink-3, block 24 (20 compact), inline 14; labels scroll with rows; card height stays the V9 formula. |
| Icons | Same shared ResultIcon source (real shell icons/extension/app identity); island may scale to 18 in its 34 tile but must not substitute generic glyphs for apps/folders. |
| Row data | Same title/subtitle/path/meta formatting, truncation, tooltip and accessible description; island row 50 px with 12/10 type is acceptable only as surface scaling. |
| Empty/copy | Same canonical copy: query zero "No matches for “{query}”." / "Try a different spelling, or a prefix such as file:, history:, or code:."; filter zero "No matches in {label}." / "Try another category, or press Esc to clear the filter."; loading 3 flat skeleton rows; error "Search is unavailable right now." + Retry. Never ship the blank 185 px zero state; show the compact visible 2-line state. |
| Actions | Same ActionSpec/Enter handling and failure copy; no fake counts. |
| Shared code | Extract one pure search model (request/scope/filter map, group order, row view-model, empty constants) consumed by both Normal Launcher and island; reuse ResultIcon and copy constants. Parity tests use one fixture and compare group order/labels/icons/empty copy. |

## 6. Evidence required over a light/text window

At 1150 and 430, with a visible white document/Notepad behind the island: compact and expanded search; notifications 0/1/3;
Quick Settings idle + Wi-Fi/Bluetooth drill-downs + fallback; enter at 0/90/180 ms; exit at 0/60/120 ms; one card switch;
reduced motion. Inside the island bounding box there must be no desktop text, previous-card ghost or opacity fade;
notifications must not overlap; Quick Settings must be usable and never say "Unsupported"; search must visibly match the
Normal Launcher order/labels/icons/empty copy. Before/after captures go in docs/qa/visual/live-shots with a manifest.
