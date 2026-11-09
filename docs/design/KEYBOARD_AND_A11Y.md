# ProtonSearch v2 — Keyboard, Focus and Accessibility

**Owner:** L1-B Product Design (`design-spec`). **Status:** binding for G3–G4. Companion to `DESIGN_SYSTEM.md`
(tokens/motion/components) and `SCREENS.md` (states/copy). All bindings are Windows-first (Tauri 2 + WebView2);
"Meta" is never used. `Ctrl` = Control, `Win` = Windows key.

**Non-negotiables from the master prompt:** the launcher must be fully usable from the keyboard; selection changes
must not lag input; Enter must never act on a stale row; focus must be visible; reduced motion, high contrast and
high DPI must be honoured; footer hints are the only visible shortcut education (no "Ctrl+K" pill, no help text in
the search field) — `PROTONSEARCH_V2_MASTER_PROMPT_ALL_IN_ONE.md:169-199`, hard rule 4 at line 26–30.

---

## 1. Focus architecture

| Surface | Focus model | Trap | Return |
|---|---|---|---|
| Quick Launcher | dialog window; input owns focus; list uses `aria-activedescendant` (focus never leaves the input while navigating rows) | Tab cycles input ↔ preview scroll region (when open) ↔ preview actions (when present) ↔ footer "Open ProtonSearch" | On hide, OS returns focus to the previous app (Tauri must not steal it); on show, input is focused before paint |
| Context menu | roving focus inside `role="menu"` | Tab/Shift+Tab closes the menu | Esc/activation returns focus to the input with the row still selected |
| Confirm dialog | focus trapped in dialog; initial focus on **Cancel** (safe default) | Tab cycles dialog controls only | Esc/cancel returns focus to the triggering control |
| Desktop app | document order; sidebar nav is a roving-tabindex list; route h1 is a programmatic focus target | no trap | route changes move focus to the h1; back navigation returns to the previously focused nav item |
| Onboarding | step route; h1 focused on each step for announcement; Tab order per §6 | no trap (whole window is the surface); Esc is a no-op | window close persists state; reopening focuses the current step h1 |
| Preview panel | not modal; scroll region is tabbable | no trap | Esc closes and returns focus to the input |

Rules:

- **Focus is never stolen by async data.** A late result, status update, save stamp, or toast never moves focus.
- **Focus is visible on every interactive element** (2 px `--ps-focus` ring, offset 2; §8). Hover-only affordances
  must also exist on focus.
- **Show-on-summon:** the launcher's input is focused in the same frame the window becomes visible; no flash, no
  animation, and no dependence on a click. If the user Alt-Tabs away, the launcher hides after 150 ms of window blur,
  unless an in-window menu or dialog is open.
- **Hide must not commit anything.** Closing with a query never opens a result; state resets cleanly.

---

## 2. Quick Launcher — complete key map

Bindings below are the contract. "Selected row" means the option referenced by `aria-activedescendant`.

| Key | Context | Action | Notes |
|---|---|---|---|
| **Summon hotkey** (default **Alt+Space**, user-configurable) | anywhere | Show launcher with empty query and focused input; press again while visible → hide | Global; default preserved from legacy (`protonsearch/src/settings.rs:149-150`); conflict detection `hotkey.rs:67-88`. Never hard-code the chord in UI copy beyond the footer's "Alt + Space" where it is the configured value. |
| **Esc** | context menu open | Close menu, return focus to input, selection unchanged | first in the chain |
| **Esc** | preview open (no menu) | Close preview, keep query and selection | |
| **Esc** | multi-select active | Clear multi-selection, keep query/results | |
| **Esc** | query non-empty | Clear query and selection, keep window open, focus input | reference footer: "Esc Clear" (reference:136) |
| **Esc** | query empty | Hide launcher | "Esc Close" when empty |
| **↑ / ↓** | input focused, results exist | Move selection to previous/next row; wraps at both ends | reference:137; never moves the caret |
| **↑ / ↓** | input focused, no results | no-op | |
| **PageUp / PageDown** | input focused | Move selection one viewport (~6 rows) up/down; clamp (no wrap) | |
| **Ctrl+Home / Ctrl+End** | input focused | Select first / last row | Home/End alone stay caret keys in the input |
| **Home / End** | input focused | Move the text caret to start/end of the query (native behaviour) | important for editing paths/terms |
| **← / →** | input focused | Move the caret (native); never changes selection | |
| **Ctrl+← / →** | input focused | Move caret by word (native) | |
| **Enter** | results visible | Open the selected result via its `ActionSpec`; launcher hides **before** the action runs | must validate current query epoch + result id (§4); no-op if the selected id is stale/absent |
| **Enter** | 0 results / error | no-op | no fallback action, no "search the web" surprise |
| **Shift+Enter** | clipboard results | Toggle multi-select for the selected row; enter multi-select mode if not active | keyboard path for multi-select (§5) |
| **Shift+↑ / Shift+↓** | multi-select active | Extend the selection range | |
| **Ctrl+A** | multi-select active | Select all rows in the current list | does **not** select the query text while multi-select is active |
| **Ctrl+Space** | any row selected | Toggle preview for the selected row | no-op with no selection; closes preview first if open |
| **Ctrl+C** | input has **no text selection** and a row is selected | Copy the full path (files/folders), URL (web), or text (clipboard) of the selected row; footer announces "Path copied" | if input text is selected, native copy wins; if multi-select is active, copy all selected paths, one per line |
| **Ctrl+C** | input text selected | Native copy of the selection | |
| **Menu key / Shift+F10** | row selected | Open the row's context menu, focus first item | |
| **Tab** | launcher | Move focus: input → preview scroll region (if open) → preview action buttons (if any) → "Open ProtonSearch" → input | stops are never the result rows (combobox pattern) |
| **Shift+Tab** | launcher | Reverse of Tab | |
| **Typing / Backspace / Delete / Ctrl+A/C/X/V/Z** | input focused | Native text editing; typing always goes to the query | selection resets to row 0 on any query change |
| **Alt+F4** | launcher | OS close (hide) | native behaviour allowed |
| **Right-click** | on a row | Open context menu at the row, focus it | pointer equivalent of Shift+F10 |

Not bound (deliberately): Ctrl+K / Ctrl+P / Ctrl+J; F-keys; letter shortcuts while typing; any single-key global
shortcut. Shortcut education lives only in the footer and Preferences.

### 2.1 Esc state machine (implementation order)

```text
if (menuOpen)            closeMenu();            return
if (previewOpen)         closePreview();         return
if (multiSelectActive)   clearMultiSelect();     return
if (query.length > 0)    clearQuery();           return
                         hideLauncher()
```

Every branch keeps focus on the input except "hide". No branch triggers an action.

### 2.2 Caret vs list keys

Because the input mirrors a command palette, arrow-key ownership is split:

- `←/→` and `Home/End` edit the query; they never navigate the list.
- `↑/↓` navigate the list; the caret stays where it is, and typing continues to insert at the caret.
- `Ctrl+Home/End` jump the selection to the ends; `Ctrl+←/→` move by word.

### 2.3 Focus and the count/status regions

The match count and the footer status are **not** tab stops and never take focus. They announce changes through
live regions (§7). The footer "Open ProtonSearch" is a real button and is the only Tab stop outside the input (plus
preview when open).

---

## 3. Desktop app — key map

| Key | Context | Action |
|---|---|---|
| **Summon hotkey** | anywhere | Show launcher (same as §2) |
| **Tab / Shift+Tab** | app | Move through controls in document order; skip link is the first stop |
| **Enter / Space** | focused button, toggle, choice, link | Activate; Enter on a checkbox toggles, on a link navigates |
| **↑ / ↓** | sidebar nav focused | Move focus between the five nav items (roving tabindex); wraps |
| **Home / End** | sidebar nav focused | First / last nav item |
| **Enter / Space** | sidebar nav item focused | Navigate; focus moves to the route h1 |
| **↑ / ↓** | segmented control focused (theme, density, filters, depth) | Change the selected option immediately (radiogroup semantics) |
| **Home / End** | segmented control focused | First / last option |
| **Space** | switch/toggle focused | Toggle on/off; announces the new state |
| **Enter** | dialog default button | Activate the focused button (default focus is Cancel on destructive dialogs) |
| **Esc** | dialog open | Close/cancel, return focus to the trigger |
| **Esc** | hotkey capture open | Cancel capture, restore the previous shortcut |
| **Ctrl+Enter** | activity row list | Open the focused row (Enter also works) |
| **↑ / ↓** | activity list focused | Move the focused row (roving tabindex); Home/End first/last |
| **Tab** | hotkey capture control | Not a text field; Tab moves to the next control, capture stays armed until a chord/Esc |

No in-window navigation chords are invented for G3–G4 (no Ctrl+1…5); the sidebar and skip link are the navigation.
If a shortcut is added later it must be listed here first.

---

## 4. Stale-selection and typing protection (required behaviour)

The engine returns results asynchronously in tiers. The UI must make it impossible to open the wrong item:

1. Every keystroke increments `queryEpoch`; `search()` is called with a monotonic `requestId`.
2. Rendered results carry `{ epoch, requestId, items }`. Responses with an older epoch/requestId are discarded.
3. Selection is `{ resultId, index, epoch }`, not a raw index. On commit:
   - new epoch → select item 0 (or none);
   - same epoch → keep `resultId` if still present, else clamp `index` to `min(prev, len-1)`.
4. Enter reads the **committed** selection and re-validates that the resultId belongs to the current committed
   set and epoch. If not, it drops the action and announces "Result changed — choose again." (polite).
5. Ranking updates for the same epoch must not scroll the list unless the selected id moved out of view; the list
   is re-ordered only by the engine's deterministic merge, never by client-side score tweaks.
6. Status/indexing events never change the selection or close a menu.
7. Rapid ↑/↓ presses must not enqueue animation: selection paint is a CSS colour transition (110 ms), the state
   value changes synchronously, so 20 presses in 300 ms land on the correct row.

---

## 5. Multi-select (clipboard results only in v2)

Keyboard model:

- **Shift+Enter** toggles multi-select for the active row and switches the list to multi-select mode.
- **Shift+↑ / Shift+↓** extend the selection range; **Ctrl+A** selects every row in the list.
- **Enter** pastes all selected items in list order; **Esc** clears multi-select (first Esc in the chain).
- The active row keeps the normal selection styling; selected rows show a 16 px check marker and the footer
  announces "3 selected" (`aria-live` polite).

ARIA: when multi-select is active the listbox gets `aria-multiselectable="true"`; `aria-selected` marks the chosen
items; `aria-activedescendant` keeps marking the keyboard-active row. In single-select mode `aria-selected="true"`
marks the active row (standard combobox behaviour). Never both meanings on one row.

---

## 6. Onboarding — key map and focus order

| Key | Action |
|---|---|
| **Tab / Shift+Tab** | Move through the step controls in order: field/choices → Back (if shown) → Skip setup → primary |
| **Enter** | Activate the primary action (Continue / Get started / Start using ProtonSearch) when focus is on the primary; on the name field, Enter also continues |
| **Space** | Toggle a checkbox; activate the focused button |
| **↑ / ↓ / Home / End** | Move within a segmented control (depth modes) and change the value |
| **Esc** | No-op (deliberate: prevents accidental loss of setup); window close is safe and persists |
| **Typing** | In the optional name field; 40-char cap |
| **Shortcut capture** | "Change shortcut" enters capture mode; the next modified key chord is taken; Esc cancels; invalid/conflicting chords announce an error |

Focus enters each step on its `<h1 tabindex="-1">` (announced with the step position referenced via
`aria-describedby`), then Tab reaches the first control. The rail is not a set of links (positions are not
navigable); it is an ordered list with `aria-current="step"`.

---

## 7. ARIA and screen-reader semantics

### 7.1 Launcher

```text
<div role="dialog" aria-modal="true" aria-label="ProtonSearch" aria-keyshortcuts="ArrowUp ArrowDown Enter Escape Control+Space">
  <header> mark (role="img" alt "ProtonSearch") + <input> + count </header>
  <div id="ps-results" role="listbox" aria-label="Results" aria-activedescendant? no — on the input>
    <div role="group" aria-labelledby="ps-group-documents">
      <div id="ps-group-documents" role="presentation">DOCUMENTS</div>
      <div role="option" id="ps-opt-<stableId>" aria-selected="true" aria-describedby="ps-desc-<stableId>">
        title … <span id="ps-desc-<stableId>">Documents / Projects / Nova · Yesterday</span>
      </div>
    </div>
  </div>
  <footer> status (role="status") · Open ProtonSearch (button) · hints </footer>
</div>
```

| Element | Semantics | Requirements |
|---|---|---|
| Root | `role="dialog"`, `aria-modal="true"`, `aria-label="ProtonSearch"` | focus trap per §1; no focusable window chrome |
| Input | `role="combobox"`, `aria-expanded` ("true" when options exist, "false" when empty/0 results/error), `aria-controls="ps-results"`, `aria-autocomplete="list"`, `aria-activedescendant="ps-opt-<id>"` (only while a row is active), `aria-label="Search files, apps, and actions"`, `autocomplete="off"`, `spellcheck="false"` | the visible placeholder is **not** the label |
| Listbox | `role="listbox"`, `aria-label="Results"`; `aria-multiselectable="true"` only in multi-select mode | one listbox; virtualisation keeps option ids stable |
| Group | `role="group"` + `aria-labelledby` to the visible caps label | label element is `role="presentation"`, the group name carries it |
| Option | `role="option"`, id from the stable result id, `aria-selected` as in §5, `aria-describedby` pointing to path/meta text | accessible name = item title only; description = path + trailing meta; disabled option `aria-disabled="true"` with the reason in the description |
| Count | `role="status"` `aria-live="polite"` `aria-atomic="true"` | throttle to ≤1 update/300 ms; announce "No matches" once |
| Footer status | `role="status"` `aria-live="polite"` | announce state transitions only (ready→indexing→paused), not every percent tick |
| Hints | plain text, not live | kbd text is readable; the keycap glyphs are decorative spans (`aria-hidden="true"`) so a reader hears "Navigate", "Open", "Clear" |
| Empty/error/loading | `role="status"` (empty/loading), `role="alert"` (new error) | never inside the listbox as a fake option |
| Preview | `<aside aria-label="Preview">`; scroll region `tabindex="0"` when scrollable | toggling announces "Preview opened"/"Preview closed" (polite, one line); pressing buttons inside does not move the launcher's selection |
| Context menu | `role="menu"` + `role="menuitem"` (+ `aria-haspopup` on the launcher row is **not** used; the menu is opened by key/right-click) | focus moves into the menu; Tab closes; Esc returns to the input with the row selected |
| Buttons | real `<button>`; icon-only buttons get `aria-label`; "Open ProtonSearch" is a button (navigation within the app) | no `<div onClick>` |

### 7.2 Desktop app

| Element | Semantics |
|---|---|
| Skip link | first focusable, visually hidden until focused: "Skip to main content" |
| Sidebar | `<nav aria-label="Main">` + `<ul>`/`<li>` + buttons/links; active item `aria-current="page"`; roving tabindex |
| Brand block | mark `role="img"` `aria-label="ProtonSearch"`; wordmark is text |
| Userfoot | static text ("This PC" or the optional name), never a fake button |
| Route | `<main id="main">` with one `<h1 tabindex="-1">`; `document.title` = "{Route} — ProtonSearch"; on route change focus the h1 and announce the route once (polite) |
| Sections | `<section aria-labelledby="{id}">` + caps h2; settings rows use `<dl>` for label/value pairs |
| Metric rows | `<dl><div><dt>Documents</dt><dd>42,018</dd></div></dl>` with tabular numerals |
| Progress | `role="progressbar"` + `aria-valuemin="0"` `aria-valuemax="100"` `aria-valuenow="{n}"` `aria-valuetext="{n}% — {phase}"`; indeterminate: omit `aria-valuenow`, give `aria-valuetext` ("Extracting text") |
| Segmented control (theme/density/filters) | `role="radiogroup"` + `role="radio"` + `aria-checked` (theme/density/depth) OR `role="group"` + `aria-pressed` buttons for filters; never mix |
| Switch | `<button role="switch" aria-checked="true|false">` + visible label |
| Checkbox choice rows | native `<input type="checkbox">` inside the label; the whole row toggles |
| Fields | `<label for>`; helper text `aria-describedby`; errors `aria-invalid="true"` + error id in `aria-describedby`, message `role="alert"` |
| Hotkey display | `<kbd>Alt</kbd> + <kbd>Space</kbd>`; the capture control is a button with `aria-describedby` explaining capture mode; a live region announces "Press the shortcut you want." |
| Dialogs | `role="dialog" aria-modal="true" aria-labelledby`; initial focus Cancel on destructive; Esc cancels; focus returns to trigger |
| Save stamp | "Saved" in `role="status"` polite; does not steal focus |

### 7.3 Onboarding

- Root `<main>` with the step `<h1 tabindex="-1">` (announced on step change); eyebrow carries "Step {n} of 7" and is
  referenced by `aria-describedby` on the h1.
- Rail: `<nav aria-label="Setup steps"><ol>`, each item a step number + label; current item
  `aria-current="step"`; completed items are readable but not links.
- Fields/choices as §7.2; the mode segmented controls are radiogroups with visible labels.
- Errors: inline `role="alert"`, focus moves to the failing control on submit; never a modal for validation.
- "Skip setup" is a real link/button; it does not need confirmation.
- No account, no email validation, no password fields anywhere.

---

## 8. Focus-visible treatment

- `outline: 2px solid var(--ps-focus); outline-offset: 2px; border-radius: inherit;` on `:focus-visible` only
  (mouse clicks do not show the ring). Never `outline: none` without a replacement.
- On a selected row, the 2 px inset selection bar is **not** a focus indicator; when the input has focus and the row
  is active, the row shows its selected styling and the input shows the caret. The focus ring appears on the input
  itself (it is the focused element).
- Contrast: ring vs light surface 4.0:1, vs raised 4.12:1, vs selected fill 3.51:1; dark ring vs surface 7.16:1,
  vs ground 7.81:1 (WCAG 2.2 Focus Appearance requires ≥3:1 against adjacent colours and a ≥2 px perimeter).
- Focus must never be hidden behind the footer, a sticky element, or the preview: the launcher has no sticky
  overlays over focusable content; the preview scrolls, it does not cover the input.
- Forced colours: `outline-color: Highlight` (system), 2 px, offset 2.

---

## 9. Reduced motion, high contrast, DPI, target sizes

### 9.1 Reduced motion

`prefers-reduced-motion: reduce` (Windows "Animation effects" off):
- all durations collapse to ~0; no transform travel; selection/hover still change colour instantly (state must remain
  perceivable);
- indeterminate progress becomes a static 30 % bar with the phase text; determinate values update without
  interpolation;
- onboarding step content and preview content appear instantly;
- scrolling is immediate (`scroll-behavior: auto`), no parallax/scroll effects exist anyway;
- nothing is hidden: every state that animates also has a static appearance.

### 9.2 High contrast / forced colours

- Add nothing that relies on hue alone; status dots keep words, the selected row becomes `Highlight`/
  `HighlightText`, focus becomes `Highlight`.
- Shadows and custom backgrounds drop out; borders become `CanvasText`; the brand mark renders with `currentColor`
  (monochrome asset); file badges keep the extension text and lose the tint (border + text remain).
- Images/file icons: real icons are allowed to keep their colours (decorative, adjacent text names the item); if a
  forced-colours mode makes an icon unreadable it is decorative only and may be hidden (never the sole cue).
- Test the four default Windows HC themes (Black, White, Aquatic, Desert) plus `prefers-contrast: more` if the host
  maps it.

### 9.3 Target sizes (pointer)

| Control | Visual | Hit target |
|---|---|---|
| Launcher result row | 58 px | full row width × 58 (≥44) |
| Footer text buttons | 10 px text + 4 px padding | min 24×24 (WCAG 2.2 AA); aim 28 |
| Kbd hints | non-interactive | n/a |
| App nav row | 40 px | full row 40 (≥24) |
| Buttons | 32 px | 32×32 min; icon-only 32×32 |
| Checkboxes | 16 px box | whole choice row ≥44 px |
| Segmented options | 32×~28 | full segment |
| Context menu items | 30 px | full width × 30 |
| Step rail (non-interactive) | 25 px circle | n/a |
| Dialog buttons | 32–38 px | as drawn |
| Scrollbar | thin | OS default in app; launcher thumb ≥8 px wide hit area |

### 9.4 DPI / zoom

- CSS px are DPI-independent; Windows scaling 125/150/200 % must be tested on the launcher (clamped to work area)
  and the app (min 560×520 CSS px, so at 200 % on a 1366-wide laptop the app may exceed the work area — the window
  manager clamps; content stays usable via internal scrolling, no horizontal page scroll).
- Do not implement custom browser zoom in v2; rely on OS scaling. If L0 enables WebView2 zoom, it must be listed as a
  supported path here and re-verified at 200 %.
- At 200 % the launcher becomes 608×302 CSS px on a 1366×768 screen: search row 72, footer 44, results ~186 px
  (~3 rows). All keyboard bindings still work; hints hide; nothing clips.

---

## 10. Screen-reader test scripts

Test with **Narrator** (Windows 11) and **NVDA** (latest) at 100 % and 150 % DPI. "Announced" = what a reader should
produce; if a reader is chattier, fix the live-region throttling, not the markup.

### 10.1 Launcher

| Step | Action | Expected |
|---|---|---|
| 1 | Press summon hotkey | "ProtonSearch, dialog" + input label "Search files, apps, and actions, combo box, editable" |
| 2 | Type "arch" | one polite announcement of the result count ("3 matches"); no per-keystroke chatter |
| 3 | ArrowDown | active option announced: "Architecture notes.docx, option, selected, Documents / Projects / Nova · Yesterday" |
| 4 | ArrowDown again | next option announced; the group change announces "FOLDERS, group" |
| 5 | Ctrl+Space | "Preview opened" (polite); Tab reaches the preview region, announced "Preview, {title}, region" |
| 6 | Esc | "Preview closed" |
| 7 | Shift+F10 | "Actions, menu"; ArrowDown reads items; Esc returns to the input |
| 8 | Enter | launcher closes; focus returns to the previous application (no orphan focus) |
| 9 | Esc on empty query | window hides; no announcement required |

### 10.2 Desktop app

| Step | Expected |
|---|---|
| Tab from window start | "Skip to main content, link"; Enter jumps to main and focuses the route h1 |
| Sidebar | "Main, navigation"; items read "Home, button, current page" etc.; arrows move focus; Enter navigates and announces the new route heading |
| Home | h1 "Good afternoon."; stat announced as text with its label; progress "Content enrichment, progress bar, 42 percent" |
| Preferences toggle | "Record activity, switch, off"; Space → "on"; "Saved" announced politely |
| Destructive confirm | dialog announced with title; initial focus on Cancel; Esc returns focus to the trigger |
| Route change | focus lands on the h1; only one polite route announcement |

### 10.3 Onboarding

| Step | Expected |
|---|---|
| Step change | h1 announced, described "Step 3 of 7"; first control reachable by Tab |
| Error | inline error announced assertively once; focus moves to the field |
| Shortcut capture | button announces "Change shortcut, Alt plus Space"; after activation "Press the shortcut you want"; conflict announced assertively |

---

## 11. Verification checklist (blocking for G3/G4 sign-off)

1. Keyboard-only pass of all journeys in `SCREENS.md` §8 — no mouse needed.
2. Every focus stop has a visible focus ring at 100/125/150/200 % and in HC Black/White.
3. Focus trap tests: launcher Tab cycle, dialog Tab cycle, menu Tab-closes; focus never lands on a hidden element.
4. Stale-selection test: simulate tier-1 results arriving after a new keystroke (late request) — selection stays on
   the correct epoch; Enter never opens a stale id (unit test + manual).
5. Rapid-typing test: 10 keys in <500 ms, then Enter — the opend item matches the visible selection; no queued
   animation lag.
6. Multi-select keyboard-only test on clipboard rows: Shift+Enter, Shift+↓, Enter pastes N items, Esc clears.
7. Reduced-motion: all motion static; no state disappears; progress still truthful.
8. Forced colours: selected row Highlight, focus Highlight, status words present, brand mono.
9. Target-size audit (24 px minimum; 44 px rows).
10. SR scripts §10 with both Narrator and NVDA recorded (screen recording or transcript); no unannounced state
    changes on the critical path.
11. Contrast audit re-run with `.agents/notes/contrast_final.py`; any fails fixed in tokens, not components.

---

## 12. Known limitations and risks

1. **Narrator/NVDA are not in CI.** Scripts are manual; evidence must be a recording or transcript in `docs/qa/`.
2. **`aria-activedescendant` + virtualisation**: option ids must be stable and present in the DOM; windowing must
   keep the active option rendered (overscan ≥ 2 rows). If a virtualiser removes it, keyboard announcements break.
3. **Multi-select semantics**: §5 uses `aria-selected` for chosen items and `aria-activedescendant` for the active
   row; some older readers announce the active row as "not selected". Verify with the actual readers; the visual
   check marker and footer count remain the fallback cues.
4. **Live-region chatter** from counts/status is the most likely a11y defect in review; throttle hard and never
   announce per keystroke.
5. **Hotkey capture** is keyboard-only by nature; Users who cannot press modifier chords must be able to use the
   default/hotkey-free path (the app can always be opened from its shortcut/tray). Ensure the capture control can be
   skipped and onboarding never blocks on it.
6. **10 px tracked caps** at 100 % DPI are the smallest text; if a low-vision user increases Windows text size and the
   WebView does not scale it, the app falls short — record the measured behaviour at Windows text size 125–200 %.
