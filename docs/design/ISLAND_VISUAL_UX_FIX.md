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

## 7. Review result — task-54 after set (2026-11-10)

**Verdict: PASS (notes closed after task-54 follow-up).** Reviewed the 33 after captures at 1150/860/430 over a white Notepad/text window plus the
before set. Opaque island surface inside the bbox (no desktop text/ghost bleed) verified; notifications 0/1/3 show the
18/1fr/30 grid, 52 px rows, reserved dismiss and fixed footer without overlap; Quick Settings renders volume 62 %/mute,
brightness 80 %, audio output, split Wi-Fi/Bluetooth tiles, drill-downs and a real “Open Windows settings” fallback with
no “Unsupported”; island search shows four filters, group labels/order, shared rows/icons and canonical empty copy.
Enter/exit CSS values are correct (180/120 ms transform-only, opacity constant 1); the 0 ms frame differs from final.
Notes: animation 90/180 ms and exit 60/120 ms screenshots are identical to endpoints due capture timing (add paused/clip
evidence); unsupported-brightness fallback is code-tested but not captured on this host (forced fixture requested);
Quick Settings’ Airplane/Night light/Energy saver compact buttons are real ms-settings actions but read like stateful
tiles — add an external/chevron cue or fold into fallback; split Wi-Fi/Bluetooth tiles use a separate On/Off pill plus
chevron rather than the spec's whole-left toggle. These are non-blocking polish/evidence items. Full G5 still has
pre-existing Media/real-Windows conditions.



### Follow-up closure (2026-11-10)

All five notes were addressed: Airplane/Night light/Energy saver now carry external chevrons + “Open Windows settings
for …” titles/aria; Wi-Fi/Bluetooth split-tile deviation recorded in VISUAL_AUDIT_ISLAND E9; audio-output row labelled
“Audio output” + device; paused enter 90 ms / exit 60 ms captures are real mid-frames distinct from endpoints and sample
opaque inside the bbox; forced-fixture brightness fallback capture shows reason + Open display settings. Manifest now 44
captures (8 before + 36 after); app suite 18 files / 242 tests green.

## 8. Task-62 native/compact UX binding (2026-11-10; supersedes task-56 where conflicting)

User feedback after native review: bleed/transparency, notification bars/scrollbar clutter, general visual clutter, wrong
compact-first expectation, Esc chain wrong, visible empty-search hint. This section is binding for the native surface and
is the acceptance contract for ui-launcher's next native after captures.

### 8.1 Compact-first and Esc chain

- **Alt+Space always summons the compact oval** (520x66, radius 999) at the saved anchor/monitor. Never restore the last
  expanded card, never auto-expand, never restore a drill-down. Every summon starts with active card = Search, query = "",
  scope = All, compact = true.
- **Compact oval anatomy:** brand mark 27, no placeholder, no helper text, no caret placeholder, no "Search files..."
  hint; 8 passive page dots (5 px, active 16x5) remain as the only secondary chrome. The input is still focused after show
  (190 ms max) so typing works immediately; caret is hidden until expansion. Accessible name "Search" and
  aria-expanded="false".
- **Typing or click expands:** any printable key expands Search and inserts the character; IME composition start expands;
  a pointer click anywhere on the oval except a page dot expands Search. No other card/drill-down opens from compact.
  Hover does not expand. Page dots switch to the selected card in expanded mode; they never open a drill-down.
- **Alt+Space while visible:** expanded -> compact (collapse, not hide); compact -> hide. This is toggle semantics and
  never directly hides an expanded card. Escape follows the separate chain below.
- **Escape chain (exact, no dialogs):**
  1. Drill-down/variant open -> parent expanded card (e.g. Wi-Fi list -> Quick Settings).
  2. Expanded card or expanded Search -> compact oval; clear query, reset scope to All, active card = Search.
  3. Compact oval -> hide and return focus to the previous app.
  Esc must not hide from the expanded state and must not clear a query as a separate step. Esc handles one step per press.
- Keyboard/ARIA: compact input combobox with aria-expanded=false; expanded combobox with aria-expanded=true,
  aria-controls results; polite live text "Search expanded" on click/typing, "Search collapsed" on Esc; after hide focus
  returns to the invoking window; no focus trap; Tab from the input reaches filters/results/preview in the documented order.
- Click outside hides only when the setting is enabled; it does not change the Esc chain.

### 8.2 No empty-search hint / empty states

- **Remove all empty-search hint copy.** Compact oval has no placeholder. Expanded Search with an empty query shows the
  search row + filter strip and a **blank results viewport**: no "Start typing...", no "Try a file name...", no suggestion
  rows, no fake recent/quick destinations, no count line. The card stays the fixed 460 px expanded size; blank space is
  intentional and must not be filled with marketing/helper text.
- **Zero results for a non-empty query:** one line only, "No matches for "{query}"." (11/600, ink-2), no secondary hint,
  no "Try a different spelling" line, no suggestions. The result count reads "0 matches". No fake rows.
- **Loading:** three flat 50 px skeleton rows, no shimmer, no hint text. **Error:** "Search is unavailable." + one Retry
  action; no hint copy. Both keep filters visible and do not resize the card.
- Accessibility: blank empty query is announced only as "Search, no query"; do not announce a hidden hint; zero results
  announces the one line politely; no empty-state live-region chatter on every keystroke.

### 8.3 Notifications anatomy — no bars, no scrollbar overlap

- Card 500x345 (>520) / 385 (<=520), radius 34; header then rows; **no footer bar, no row separators, no full-width
  horizontal rules**. Remove the bottom "Clear all" bar; the only action is a 28x28 header icon button at top-right
  (accessible name "Clear all notifications"), shown only when notifications exist and a real command is available.
- Header: title "Notifications" 15/700, subtitle "Recent activity · {n} unread" 11 ink-2; header block 56 high, padding
  18 20 10; no divider line; 8 px gap to list.
- List: rows 52 high, gap 6, padding 0 8 0 0 (8 px right inset); **no background, border or separator**; no visible
  scrollbar (hide native/webkit scrollbars; overlay scroll does not overlap content); max 3 full rows visible; more scrolls
  under a 8 px inset. Dismiss 30x30 reserved right column, icon 15; disabled dismiss is hidden rather than a dead ×.
- Row content: bell 18, title 12/600 one line ellipsis, body 10/1.4 max 2 lines, source/meta 10 ink-3 one line.
  No notification strip/bar/progress bar is drawn unless a real OS progress belongs to that notification; no fake unread
  bars.
- States: 0 = centred "All caught up." (no hint, no clear button, no bars); 1 = one row, header clear button; many = 3
  visible + hidden-scrollbar list; loading = 3 flat skeleton rows 52 without bars; error = "Notifications are unavailable."
  + reason/Retry if supported; unsupported source omitted, not "Unsupported".
- Copy: correct singular "1 unread"; no "Test local toast" in production; Clear all only with real bridge command; no
  footer bar/scrollbar collisions at 1150/860/430 or 125% scaling.

### 8.4 Quick Settings — usable, honest, reduced to one surface

Card 515x376 (>520) / 400 (<=520) / 483 (<=380); header 56, body padding 17 20 14; no footer/action bars.

1. Volume row 36: speaker icon 18, "Volume" 11, range flex, value 10 tabular; mute is a 32x32 icon button
   (aria-pressed, aria-label "Mute"/"Unmute") at the right, not a text button block.
2. Brightness row 36: sun icon 18, "Brightness", range, value 10; when unsupported replace the range with a single
   trailing text link "Open display settings" (11, accent) on the same row. No separate fallback row/box.
3. Audio output row 36: output icon 16, "Audio output" 11, current device 12 ellipsis, chevron 16; opens the output list.
4. Split tiles two columns, gap 9, height 56: tile radius 10, bg #171e23, border #34434a, on #223a49/#416178.
   Left area (flex 1, padding 8 10) is the toggle: Wi-Fi/Bluetooth icon 19 + label 11 + state 10 ("On"/"Off"); right
   chevron button 32 wide separated by 1 px #4b606c opens networks/devices. No duplicate On/Off pill, no extra badge.
   Toggle and chevron are separate accessible targets with names "Turn Wi-Fi on/off", "Show Wi-Fi networks", etc.
5. One quiet bottom action row 32: icon 14 + "Open Windows settings" 11/650, no border/box/background; opens
   ms-settings: display/network. This replaces Airplane/Night light/Energy saver pills and every dead/unsupported tile.
   Unavailable item: "{Control} can't be changed here." + "Open Windows settings."; no "Unsupported" word, no dead toggle.
- OS consent truth: Wi-Fi/BT scan denied -> inline note in the drill-down with "Open location settings" (real command), no
  fake networks; capability unavailable -> row/tile says available-via-Windows and the fallback link is the action.
- Fit 345 px inner: header 56 + volume 36 + brightness 36 + audio 36 + tiles 56 + gaps 8*4 + action 32 + body padding
  31 = 315 px; no clipping, no scroll at >520. <=380 may stack tiles vertically and scroll with hidden scrollbar.
- Loading: skeleton values + disabled controls with aria-busy; no shimmer. Setter failure: one quiet inline line
  "Couldn't change {control}." + fallback link. No fake values, no dead controls, no "Unsupported".

### 8.5 Motion — transform-first card switch

- **No native resize per frame.** Before every compact/expanded/card transition, measure the target geometry and call
  native setIslandGeometry exactly **once**; never during the animation, never per keystroke. Window background remains
  filled/opaque so there is no blank frame while the size is applied.
- Card switch (search -> card or card -> card), total 220 ms:
  1. Outgoing content: transform scale(.96) translateY(6px), opacity 1->0 over 120 ms ease-in; surface stays opaque.
  2. Swap content at 120 ms.
  3. Incoming content: transform scale(.985) translateY(8px) -> scale(1) translateY(0), opacity 0->1 over 180 ms
     cubic-bezier(.22,1,.36,1).
  No width/height/radius CSS animation is used for the switch; the native window jumps once at t=0 (or immediately after
  the outgoing frame is painted).
- Compact -> expanded Search: native window resized once to 550x460; content transform scale(.98) translateY(6px) ->
  1/0, 200 ms; no layout animation. Expanded -> compact: 140 ms reverse; native resize once at start.
- Enter/exit (summon/hide): 180 ms translateY(-8px) scale(.97) -> 0/1 on show; 120 ms translateY(-6px) scale(.985) on
  hide; **surface opacity constant 1**, no fade-through. Reduced motion: all transitions 0 ms, no transform; native
  resize still once before show.
- Acceptance frame targets (native video/60 fps): at 60/120/180/220 ms the surface is opaque with no desktop text/ghost;
  no blank/white frame; no frame with missing card; native geometry call count is 1 per transition; p95 frame build
  <=16.7 ms; no layout shift >1 px in card content; no visible scrollbar during switch. Record a native screen capture
  (not browser mock) to prove it.
- Dot/wheel/swipe/Ctrl card switching uses the same sequence; latest input wins; switch lock 350 ms remains; no native
  window resize during wheel scrolling.

### 8.6 Polish values (binding overrides)

| Token | Value |
|---|---|
| Surface radius | 34 expanded; 999 compact; 41 media |
| Border | 1 px #2e3538 solid; no inner double borders |
| Surface shadow | native only; CSS shadow may be 0 18px 44px rgba(20,32,40,.28) + 0 2px 8px rgba(27,32,32,.18); no inner shadow |
| Card padding | header 18 20 10; body 17 20 14 |
| Header | 56 high; title 15/700; subtitle 11 ink-2 margin-top 4 |
| Row density | notification 52; settings 36; tile 56; choose row 47; result row 50 |
| Gaps | 8 default; 6 notification list; 9 tile grid; 4 label/value |
| Type | card title 15/700; row title 12/600; body 10/1.4; meta 10; control label 11; value 10 tabular; button 11/650 |
| Icon sizes | header 23; row 18; controls 18-19; chevrons 16; dismiss 15 |
| Dots footer | 31 expanded / 38 compact; dot 5, active 16x5, gap 5; no glow/no fade |
| Chrome budget | max one hairline divider under a header; no footer bars, no full-width row rules, no extra action pills; quiet text links replace boxed fallbacks |
| Color | keep V9 graphite tokens; no rgba panel fills; only shadow/border/focus alpha |

### 8.7 Native opacity and acceptance — zero transparency inside the bbox

- **Mandatory:** inside the island surface bounding box every pixel is opaque (alpha 1). No CSS opacity <1 on the
  surface/inner/card shells, no rgba panel backgrounds, no backdrop-filter, no acrylic/mica, no DWM transparency inside
  the bbox. Alpha is allowed only for the outer shadow, 1 px borders and focus rings. Content may fade only over an
  opaque surface.
- Native window: transparent(true) is permitted **only in the shadow padding outside the surface**. Size/place the
  window before show; surface already painted at opacity 1; no white/desktop frame on first paint. If native
  transparency still produces bleed inside the bbox, use the **opaque-window fallback**: disable window transparency and
  the custom shadow pad, size the window exactly to the surface rect (no 112 px pad), enable native DWM shadow/rounded
  corners, keep the opaque CSS border/radius; zero transparency wins over the custom shadow.
- Native acceptance test (not browser mock): put a white Notepad/text document directly behind the island; capture at
  100%, 125%, 150% DPI, compact + expanded + notifications + Quick Settings. Inspect the island bbox pixels with an image
  editor: no underlying text/letter edges, no light halo, no alpha blend, no ghost card, no partial pixel rows at the
  rounded edges. Shadow may show outside the bbox only. Save PNGs plus a manifest under docs/qa/visual/live-shots with
  before/after pairs.
- Also test first show (0 ms), 90/180 ms show frames, 60/120 ms hide frames, one card switch, and reduced motion over the
  same text window; surface must be opaque at every frame.

### 8.8 Acceptance evidence for ui-launcher

Native before/after at 1150/860/430 plus 125%/150% DPI, over the white text window: compact oval (no hint), click-to-expand,
typing expands, Esc expanded->compact->hidden, no empty hint, notifications 0/1/many (no bars/scrollbar overlap), Quick
Settings idle + Wi-Fi/Bluetooth/audio drill-downs + honest fallback, card switch frames 60/120/180/220 ms, enter/exit
frames, reduced motion. Include a state manifest and the native resize count (must be 1 per transition). This section
invalidates any earlier acceptance claim based on browser-mock screenshots.

