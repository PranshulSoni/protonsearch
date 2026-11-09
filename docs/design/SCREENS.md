# ProtonSearch v2 — Screen Specifications

**Owner:** L1-B Product Design (`design-spec`). **Status:** binding for G3–G4. Companion to `DESIGN_SYSTEM.md` (tokens,
type, motion, components), `KEYBOARD_AND_A11Y.md` (keys, ARIA, focus), `REFERENCE_MAPPING.md` (reference → component).
**Reference of record:** `protonsearch-v2-ui-reference.html` (line citations below). **Scope:** only the surfaces the
master prompt names (§V, lines 163–199): Quick Launcher, Full Desktop App (Home, Search & Indexes, Activity,
Workflows, Preferences), Onboarding. No page/site, no dashboard filler.

All sizes are **logical CSS px**; Windows DPI multiplies physical pixels (§6). All copy below is exact, final copy —
implementers do not reword it. Samples data must be replaced by live `ps-ipc` DTOs (`SearchResult`, `EngineStatus`,
`Settings`, `ActivityItem`, `OnboardingState`; see `docs/implementation/WORK_BREAKDOWN.md` §4).

---

## 1. Windows, routes and data

| Surface | Window | Chrome | Route / shell |
|---|---|---|---|
| Quick Launcher | frameless, floating, 820 px wide (clamps to work area), 256–480 px tall | none: no title bar, no min/max/close, no tabs | separate launcher window (Tauri), route `launcher` |
| Desktop App | resizable standard window 1120×760 default, min 560×520, native Windows title bar "ProtonSearch" | native frame; in-window sidebar + workspace | app window, routes `/home`, `/search-indexes`, `/activity`, `/workflows`, `/preferences` |
| Onboarding | resizable window 900×620 default, min 560×520, native title bar "Set up ProtonSearch" | native frame; no app sidebar | app window route `/onboarding` (modal route, hides shell) |
| Confirm dialog | in-window modal, max 420 px | scrim + dialog | component |

**Launcher must never render** a title bar, window controls, tabs, "Ctrl+K" hint, kbd pill inside the search field,
fake status badge, or decorative metric cards (master prompt §V.A, lines 169–177). The brand mark is the first
element of the search row (reference:30, 136).

**Window behaviour (decisions):**

- Launcher appears on the monitor that contains the mouse cursor, horizontally centred; top =
  `workArea.top + round(workArea.height * 0.18)`, clamped ≥ 24 px from the work-area top.
- Show/hide is instant (0 ms); focus lands in the input before the first paint. No fade/scale. Hiding does not
  animate and clears the query after a 150 ms grace (so accidental double-tap of the hotkey reopens the prior query?
  **No**: show always starts empty, per master prompt "hidden/resumable launcher appears with input focused"; query is
  **not** preserved between summons unless the user is returning from the app in the same session — keep empty).
- Desktop app window size/position persisted; restore clamps to the current work area (monitor changes).
- Onboarding opened exactly once on first run (migration marks existing users complete, §4.9). Re-openable from
  Preferences and from the Home incomplete-setup banner.
- All windows: `background: var(--ps-surface)` set before first paint; `color-scheme` set from the resolved theme;
  no white flash (master prompt Gate 3).

**Master data mapping (display only — field names are L0's `ps-ipc` contract):**

| UI display | Source |
|---|---|
| result title / path / trailing meta / icon / action | `SearchResult` from `search` |
| match count, status word, progress, index counts | `EngineStatus` from `get_status` |
| recent items, activity rows | `get_activity` (`ActivityItem`) |
| roots, modes, exclusions, toggles, hotkey, theme, density, privacy, agent opt-in | `Settings` via `get_settings` / `update_settings` |
| setup step, name, roots, modes, shortcut, agent opt-in, skipped/completed | `OnboardingState` via `onboarding` (get/complete) |
| sources/health | `get_sources` |
| index control | `index_command` (pause/resume/rebuild) |

---

## 2. Quick Launcher (primary surface)

### 2.1 Structure and metrics

```text
┌──────────────────────────────────────────────────────────────┐  radius 14, 1px --ps-line-strong
│ [P 29px]  query input (21px)                     3 matches    │  search row: 72px, pad 14/20, gap 14
├──────────────────────────────────────────────────────────────┤  1px --ps-line
│ DOCUMENTS                                                    │  group label: 34px block, 10px caps
│ [icon] Architecture notes.docx                        Yesterday│  row: 58px, pad 8/12, radius 8
│ [icon] search-architecture.md                             Today│  ...
│ FOLDERS                                                      │
│ [icon] Architecture                                 24 items │
├──────────────────────────────────────────────────────────────┤  1px --ps-line
│ ● Index ready · Open ProtonSearch       [↑↓] Navigate [Enter] Open [Esc] Clear │  footer 44px
└──────────────────────────────────────────────────────────────┘
```

- Width `W = min(820, workArea.width − 32)`; min 480 px when DPI forces it. Preview side panel (§2.6) widens to
  `min(1180, workArea.width − 32)` only when `workArea.width ≥ 1200` (`--launcher-width-preview: 1180px` in the
  landed tokens).
- Height `H = clamp(256, 72 + 23 + Σ(34 + rowH·n), 480)` where 23 px = results padding 10 top + 13 bottom; `rowH` 58
  (comfortable) / 48 (compact); group label block 34 (30 compact). Height snaps to 2 px. Resize is instant and happens
  in the same frame as the content change (no animation, no white gap).
- Results max-height = `min(365, H − 72 − 44)`; scrollbar thin, thumb `--ps-scrollbar-thumb`.
- Header: brand mark (light or on-dark variant per theme, 29 px; 26 px at ≤700) + input + count.
- Empty-input minimum height is 256 px (search row 72 + results 140 + footer 44) — compact, not an empty stage.

### 2.2 Results model

- The UI receives a ranked list, groups by `SourceKind`, and renders only non-empty groups in **fixed order**:
  **APPLICATIONS · FILES · FOLDERS · DOCUMENTS · CLIPBOARD · WEB · CODE · ACTIONS** — except that an exact command /
  calculator match pins ACTIONS to the top (engine marks the action result as exact). Group labels are the literal
  uppercase strings above (they are content labels, rendered with CSS caps; localised).
- Row: 36 px icon slot · 12 px gap · title (13/680, ellipsis) · path (11 px `--ps-ink-2`, ellipsis, 4 px below) ·
  trailing meta 10–11 px `--ps-ink-3` (recency / size / item count / source). Path truncates at the end; title
  truncates with ellipsis; both keep a `title` tooltip and the accessible description carries the full path.
- Visible result cap 50; engine supplies the ranked list. Count text in the search row is the true count of the
  current result set ("3 matches", "1 match", "No matches", "Searching…").
- Selected row: fill `--ps-accent-soft` + 1 px `--ps-accent-border` + 2 px inset start bar `--ps-selected-bar`
  (accessibility correction, DESIGN_SYSTEM §3.3/§8.2). Hover on other rows: `#f0f3f0` (light) / `#242722` (dark).
- Selection is tracked by **stable result id + query epoch**, not raw index. New query text resets selection to the
  first row. Late tier results for the same query keep the selected id if present; otherwise clamp to
  `min(previousIndex, len−1)`. Enter always acts on the current selected id; if that id is not in the current epoch the
  action is dropped (master prompt §IV, "never let a stale query reselect the wrong item").
- Wrap: ArrowDown on the last row selects the first; ArrowUp on the first selects the last (reference:137).

### 2.3 States (launcher)

| State | Results area | Search row | Footer left |
|---|---|---|---|
| First query in flight | 3 skeleton rows at real 58 px pitch (title bar 40 % width 13 px tall, path 60 % width 10 px tall; flat grey, no shimmer) | count "Searching…" (`aria-live` polite, throttled 300 ms) | unchanged |
| Subsequent query in flight | previous rows stay rendered (no skeleton, no flicker); count "Searching…" | same | unchanged |
| Empty input, recents enabled and non-empty | group **RECENT** + up to 5 recent rows (`RecentRow`) | placeholder "Search files, apps, and actions…" | status |
| Empty input, no recents / recents off | empty state: "Start typing to search files, apps, and actions." + 11 px line "Try a file name, an app, or a command like “lock”." | placeholder | status |
| 1 match / many | grouped rows | count | status |
| 0 matches | "No matches for “{query}”." + 11 px "Try a different spelling, or a prefix such as file:, history:, or code:." | count "No matches" | status |
| Search error | inline block inside the results frame: "Search is unavailable right now." + text button "Retry" | count "Search unavailable" | dot negative + "Index unavailable" |
| Indexing (results still served) | normal results | normal | dot warning + "Indexing · 42 %" (real value; indeterminate → "Indexing…") |
| Index paused | normal results | normal | dot neutral + "Index paused" |
| Offline (network) | no special state — local search is unaffected; unavailable network roots are simply absent | normal | normal |
| Preview open | rows + side/drawer preview; selected row unchanged | normal | "Preview" hint |

### 2.4 Empty-input content rules

- Recent group rows use the same anatomy as result rows, source-agnostic (file, app, clipboard, search). Enter opens
  the item; the list is keyboard-navigable exactly like results.
- If recents are disabled by privacy settings, show the no-recents empty state — no explanation banner in the
  launcher (privacy is explained in Preferences).
- No fake "suggested commands", no tiles, no tips carousel.

### 2.5 Footer

- Left: 8 px status dot + status word + "·" + text button "Open ProtonSearch" (10 px/750, `#356d88`). Status is a
  polite live region; colour always paired with words. Truthful mapping: ready / indexing n % / paused / starting /
  unavailable. Never show "Index ready" before the engine reports ready.
- Right: at most three kbd hints, in priority order, contextual (reference:136 shows Navigate/Open/Clear):

| Context | Hints |
|---|---|
| rows exist, query empty | `[↑↓] Navigate` · `[Enter] Open` · `[Esc] Close` |
| rows exist, query typed | `[↑↓] Navigate` · `[Enter] Open` · `[Esc] Clear` |
| no rows, query typed | `[Esc] Clear` |
| preview open | `[Ctrl+Space] Close preview` · `[Esc] Close` (preview closes first) |
| multi-select active (clipboard) | `[Enter] Paste 3` · `[Esc] Clear selection` · `[Shift+Enter] Add` |
| copy-path feedback | left status becomes "Path copied" for 2 s (role=status) |

- Hints hide by priority (Copy/Paste → Preview → Navigate) below 900 px work width; never wrap above 900 px. Keep
  "Esc" semantics visible: Clear when a query exists, Close when empty (reference footer says "Esc Clear").
- "Open ProtonSearch" opens the app on Home with the current query pre-filled into the app search where applicable;
  it is a button, not a link (no browser semantics).

### 2.6 Preview (Ctrl+Space)

- Toggle only when a row is selected. Default **off**; Preferences may enable "Preview the selected result".
- Placement: side panel 360 px wide at `workArea.width ≥ 1200` (window grows to the 1180 px preview token, height
  unchanged); bottom drawer
  240 px tall below the results otherwise (window height grows to fit, cap at `min(640, workArea.height − 24)`).
- Panel anatomy: item title (13/680) · kind/type line (11 px) · path or URL (11 px, selectable) · preview body ·
  action row (Copy path, Open, Open containing folder — text buttons, only actions the ActionSpec declares).
- Preview bodies: code/text = first 20 lines in mono 12 px with the match highlighted; image = thumbnail max
  320×240 (no upscale, no letterbox background); PDF/Office = metadata card when rendering unavailable
  ("Preview not available — press Enter to open."); clipboard = full text or image; web = URL + source; app/action =
  summary card. Unknown kind: "No preview for this item."
- Loading: flat skeleton at the real content size (no shimmer); error: one sentence + Retry if retryable.
- Esc closes the preview first, then clears, then closes the window. Ctrl+Space toggles closed.
- Focus: the preview scroll region is focusable (Tab reaches it) and scrolls with ↑↓/PageUp/PageDown when focused;
  Tab returns to the input.

### 2.7 Multi-select (clipboard results only in v2)

- Enter multi-select with Ctrl+Click, or Shift+Enter on the selected row (keyboard path). Shift+↑/↓ extends the
  range; Ctrl+A selects all clipboard rows; Ctrl+Click toggles single rows.
- Rows show a 16 px checkbox marker at the start (row height unchanged); footer shows "3 selected" in the left
  status area and the multi-select hints. Enter pastes the selected items in list order (legacy clipboard
  multi-select behaviour); Esc clears the selection before other Esc semantics.
- Other sources declare multi-select in their `ActionSpec`; the UI hides the affordance when unsupported.

### 2.8 Context menu

- Open with the Menu key / Shift+F10 / right-click on a row; anchored to the row edge; focus moves into the menu,
  Esc returns focus to the row/input; Enter activates; ↑/↓ navigate; destructive items last in negative ink.
- Items by source (only those the ActionSpec declares; never invent actions):
  - File/folder: Open · Open containing folder · Open with… · Copy path (Ctrl+C) · Copy name · Rename… · Delete…
    (Rename/Delete require a confirmation dialog naming the item; Delete is destructive and never immediate).
  - App: Open · Copy name.
  - Action/command: Run.
  - Clipboard: Paste · Copy · Pin/Unpin · Remove from history (no confirm for a single item; "Clear clipboard
    history…" confirms with the count).
  - Web: Open · Copy URL.
- No menu on empty space. Menus close on outside click, Esc, Tab-out, or window hide.

### 2.9 Launcher keyboard (summary — full map in KEYBOARD_AND_A11Y.md §2)

↑/↓ navigate (wrap) · Enter open · Esc preview→clear→close · Tab/Shift+Tab move between input, preview (when open),
and the footer's "Open ProtonSearch" · Ctrl+Space preview toggle · Ctrl+C copy path (when the input has no text
selection) · Shift+F10/Menu open context menu · Shift+Enter multi-select · Home/End/PageUp/PageDown in the list.
No other global chords; the summon hotkey is configured in Preferences/onboarding.

### 2.10 Launcher motion

Show/hide 0 ms. Selection colour 110 ms. Footer status changes are instant (text). Preview content fades 160 ms +
6 px travel (reduced motion: instant). Window resize is instant (never animated). No scroll animation.

---

## 3. Full Desktop App

### 3.1 Shell

- Native window frame (deviation from the reference's card-on-stage; the app fills the window — master prompt §V.B).
- Layout: sidebar + workspace. Sidebar `--ps-sidebar`, 1 px `--ps-line` right border:
  - brand block: 29 px mark + "ProtonSearch" 12/750/`-0.035em` (reference:54);
  - label "WORKSPACE" (10/750/+0.12em, `--ps-ink-3`) (reference:54);
  - nav: Home, Search & indexes, Activity, Workflows, Preferences — 16 px icon in 19 px slot + 12/600 label,
    row 40 px, radius 7, hover `#e9eeea`, active `--ps-nav-active-bg` + `--ps-nav-active-ink` + weight 750, no left
    bar (reference:55-58); icons are real SVGs, not `⌂ ⌕ ◷ ↗ ⚙` placeholders;
  - userfoot pinned bottom: avatar 30 px (initial of optional name) + first name 11 px; without a name: neutral
    person glyph + "This PC" (never infer identity — master prompt §V.B); static, not a control.
- Workspace: padding 36/40 at ≥1100, 32/32 at 820–1099, 31/24 at 701–819, 31/18 at ≤700; content max-width 1040,
  left-aligned to the workspace grid (no centring).
- Skip link "Skip to main content" as the first focusable element; route change moves focus to the route h1
  (`tabindex="-1"`) and updates `document.title` ("Home — ProtonSearch").
- Sidebar widths: 220 (≥1100), 190 (820–1099), 56 icon rail (≤819, labels visually hidden but present for AT and
  shown as tooltips). At ≤700 the rail stays 56 and content stacks.

### 3.2 Home

**Question it answers:** "Is my index healthy and what can I do right now?"

| Element | Spec / exact copy |
|---|---|
| Eyebrow | "Your workspace / Home" (rendered uppercase tracked, 10 px) |
| H1 | Time-of-day greeting with optional name: `Good morning[, {name}].` — buckets 05:00–11:59 morning, 12:00–16:59 afternoon, 17:00–04:59 evening, local time via `Intl.DateTimeFormat`, punctuation final "." ; no name → "Good morning." (reference:140) |
| Lead | "Everything on your computer, within reach." (reference:140) |
| Big search | Button, 48 px tall, raised surface, 1 px border `#dce4db`, radius 8, 13 px placeholder-ink text "Find a file, open an app, or run an action…", search glyph left, ↗ right (reference:64, 140). Opens/summons the launcher. Accessible name = visible text. |
| Columns | ≥1000: `1.25fr / 0.9fr`, gap 38 (reference:67); 820–999: same with 32 gap; ≤819: single column, 24 gap |
| Recently accessed | h2 "Recently accessed" (11 caps 750) + up to 3 recent rows (icon 32, title 12/680, sub 11 `--ps-ink-2`, trailing arrow) + secondary button "Manage sources" (opens Search & indexes) (reference:69-71, 140) |
| Your index | h2 "Your index" + stat 34/680 (real total, tabular) + sub "items indexed on this PC" + 4 px progress with label "Content enrichment · 42 %" (only when enrichment is running; otherwise "Index up to date" 11 px) + metric rows: Documents, Applications, Images & OCR, Other files (real counts, right-aligned, tabular) (reference:72-74, 140) |
| Notice slot | Above the columns, only when real: paused indexing / index error / incomplete setup. One line + action, no banners with illustrations. |

**States**

- Loading: skeleton blocks at the real sizes (h1 line 41 px, lead 13 px, search 48 px, 3 recent rows 56 px, stat 34 px,
  4 metric rows 40 px); layout identical when data lands.
- Empty (fresh install, onboarding skipped): greeting + big search + "Nothing indexed yet." panel with "Set up
  indexing" (secondary → Search & indexes) and "Resume setup" (text → onboarding). Recently accessed shows "No
  recent items yet." — no fake rows.
- Incomplete setup: notice "Setup is {n} of 7 steps done." + "Resume setup".
- Error (status unavailable): the Your index block shows "Couldn't read the index status." + "Retry" text button;
  the rest of Home still works. Never a full-page error.
- Indexing: progress truthful; "Pause" text action next to the label.
- Offline: local Home unchanged; updater/agent blocks elsewhere may show offline copy.
- Dark: art-directed tokens; the stat and greeting keep the same hierarchy.

### 3.3 Search & indexes

**Question:** "What is indexed, how deep, and is it healthy?" h1 "Search & indexes"; lead "Choose what ProtonSearch
can find, without blocking the fast filename index." (reference:142)

Sections (vertical, 32–40 px apart, each with an 11 px caps h2; label column 450 + panel per reference settings
style at ≥1000, stacked below):

1. **LOCATIONS** — rows: root name (13/680) + path (11 px) · depth control (SegmentedControl: "Names only",
   "+ Content", "+ OCR") · status dot + word · item count (tabular) · overflow menu (Pause/Resume, Remove…).
   Buttons: "Add location…" (opens folder picker). Footer note: "Filename search works everywhere. Content and OCR
   run in bounded background jobs and never block the launcher."
   Empty: panel with "No locations selected." + "Add location…" primary + "Locations you add stay on this PC."
   Remove confirm: "Remove {folder} from the index? Results from this folder stop appearing. Your files are not
   changed." Cancel / Remove.
2. **INDEX STATUS** — progress bar (determinate with real % or indeterminate with phase text), "Last scan completed
   {localized relative time}", "Index size 214 MB", "Files 89,813", "Documents 14,609", "OCR documents 0".
   Actions: "Pause indexing" / "Resume indexing"; "Rebuild index…" (confirm: "Rebuild the index? Search stays
   available while filenames are re-scanned. This can take a few minutes and uses more CPU." Cancel / Rebuild).
3. **EXCLUSIONS** — list of excluded patterns with remove buttons; "Add pattern…" text field + button; note "These
   patterns are skipped for content and OCR." Empty: "No exclusions. Everything in your locations is indexed."
4. **SOURCES** — per-source rows (Files, Documents, OCR, Browser history & bookmarks, Clipboard, Git, Apps & actions):
   status dot + word, last updated, detail ("Re-indexing… 42 %"), Retry on error; error text is one sentence
   (e.g. "Couldn't read Chrome history — the browser profile is locked. Retry."). Empty: "No sources configured."
5. **CLOUD & REMOVABLE** (only when the engine reports such roots) — rows: root path, status ("Unavailable — network
   location disconnected"), Retry; note "Placeholders are indexed by name. Content is read only when available."

**States:** loading skeletons; 0 locations; indexing/paused/rebuilding; per-root error; overall error block with
Retry; offline network roots shown as unavailable, never as a global error; dark tokens; changes autosave with a
"Saved" stamp (2 s) next to the affected section title.

### 3.4 Activity

**Question:** "What did I do recently, and is it private?" h1 "Your activity"; lead "A local history of recent
searches, launches, and optional workflows." (reference:143)

- Privacy strip: "Stored on this PC. Pause or clear at any time." + Toggle "Pause recording" + text button
  "Clear history…".
- Filters (SegmentedControl): "All" · "Searches" · "Launches"; filters apply instantly and are reflected in the URL
  query (deep-linkable state).
- List: date group labels "Today", "Yesterday", then localized dates; rows: 32 px icon · title 12/680 · sub 11 px
  "Search · 09:41" / "Launched · 09:38" / "Clipboard · 09:32" · trailing quiet arrow. Row is one button (opens the
  item or repeats the search where applicable).
- Empty: "No activity yet." + "Searches and launches appear here while recording is on."
- Paused: notice "Activity recording is paused." + "Resume" text button; existing entries remain readable.
- Clear confirm: "Clear all activity? This removes {n} recorded items from this PC. It can't be undone."
  Cancel / Clear (negative).
- States: loading skeleton rows; read error inline with Retry; dark; filters with no matches → "No {searches} in this
  view." Keep the frame.

### 3.5 Workflows

**Question:** "What can run, and what is allowed to run?" h1 "Workflows"; lead "Native commands respond instantly;
the Python agent sidecar is optional and starts only when needed." (reference:144)

- **NATIVE COMMANDS** panel: rows "Window management", "Clipboard actions", "System actions" with status word
  ("Available"/"Unavailable") and one-line description. No buttons that fake actions; actions live in the launcher.
- **AGENT RUNTIME** panel: status row (dot + "Not set up" / "Not running" / "Running" / "Couldn't start"), copy
  "Agent workflows are optional. The runtime starts only when you run one, then exits when idle." Actions:
  "Set up agent…" (when not set up) / "Start" / "Stop"; provider row "No provider configured" + "Configure…".
  Off-state copy: "Agent workflows are off. ProtonSearch works fully without them." No permanent "Ask AI" affordance
  (master prompt §VI).
- **PERMISSIONS** panel: "Files & folders — read only", "Clipboard", "Shell commands", "Network" with state
  "Ask every time" (default); note "Writes, deletes, shell commands, and network access always ask first." Link
  "Review approvals" once an approvals log exists; otherwise omit the link rather than ship a dead control.
- Error: "The agent runtime couldn't start. Search is unaffected." + "Details" (opens log folder) + Retry.
- States: loading, off, starting, running, failed, offline (provider unreachable: "Couldn't reach the provider. Your
  keys and settings are unchanged."), dark.

### 3.6 Preferences

h1 "Make it yours"; lead "Customize behavior, appearance and privacy independently of search performance."
(reference:145)

Sections (label column + panel; one save model: **autosave with a "Saved" stamp**, no global Save; destructive
actions confirm):

1. **LAUNCHER SHORTCUT** — "Current shortcut" + kbd "Alt + Space" + "Change…" (capture dialog: "Press the shortcut
   you want." helper "Use at least one of Ctrl, Alt, Shift, or Win."; Esc cancels; conflict inline error "That
   shortcut is already used by another app. Try another."; "Reset to Alt + Space"). Saves immediately.
2. **APPEARANCE** — "Theme" segmented System / Light / Dark; "Density" segmented Comfortable / Compact; "Reduced
   motion" line "Follows your Windows animation setting."; live preview panel showing a mini launcher row in the
   selected theme/density (the reference's settings pages show values; the preview shows the result).
3. **PRIVACY & HISTORY** — toggles "Record activity" ("Search and launch history on this PC"), "Clipboard history";
   text buttons "Clear activity…" and "Clear clipboard history…" (confirm with counts); note "No telemetry. Nothing
   leaves your PC unless you opt into an agent provider."
4. **INDEXING** — toggles "Background content indexing", "Pause indexing on battery"; link "Index health" → Search &
   indexes; link "Rebuild index…".
5. **AGENT & MODELS** — toggle "Enable agent workflows" (default off); "Model cache 0 B" + "Manage…" (opens a panel
   that lists cached models and offers "Clear cache"); note "No model is downloaded without asking."
6. **UPDATES & DIAGNOSTICS** — version row ("ProtonSearch 2.0.0"); "Check for updates" button with result text
   ("Up to date" / "Update available" / "Can't check right now — you're offline."); "Open logs folder" button;
   "Export diagnostics…" only if a redacted export exists (else omit).
7. **ABOUT** — version, "Local-first. No account." and a licenses link.

**States:** loading skeleton; settings read error block "Couldn't load settings." Retry; hotkey conflict inline
`role="alert"`; updater offline notice; dark; disabled toggles (e.g. OCR off when no model) stay readable and
explain why in 11 px helper text.

---

## 4. Onboarding (first run)

### 4.1 Rules

- Seven steps; **skippable** and **resumable**. State persists after every step (`OnboardingState`); closing the
  window never loses progress. Existing users (legacy settings/index detected) are marked complete and **never see
  onboarding** (master prompt §V.C).
- Rail is position, not percent: numbers 1–7 with labels; current step filled `#295a70` white; completed steps accent
  ink; future steps muted. No fake progress bar.
- Typography: eyebrow "Getting started / 0{n} of 07"; h1 32/600; lead 13 px max 420 px; content; footer: left
  "Back" ghost + "Skip setup" text link, right primary. Rail hidden ≤700; eyebrow then carries the position.
- Primary labels: step 1 "Get started", steps 2–6 "Continue", step 7 "Start using ProtonSearch". Back is hidden on
  step 1. "Skip setup" is available on every step.
- Focus moves to the step h1 on change; Enter activates the primary; Esc is a no-op (prevents accidental loss);
  closing the window is safe.
- No account, no cloud sign-in, no silent downloads, no name inference (master prompt §V.C).

### 4.2 Step 01 — Welcome & privacy

- h1 "Find anything, without the friction."
- lead "A fast, private Windows search experience built around your work. We only need a few preferences to get
  started." (reference:148)
- Content: three rows (label + one line each, 12/680 + 11 px):
  "Local-first" — "Your index, history, and settings stay on this PC."
  "No account" — "Nothing to sign in to, and no telemetry."
  "You choose" — "Content, OCR, and agent workflows are opt-in."
- Primary "Get started"; "Skip setup" link.

### 4.3 Step 02 — Your name (optional)

- h1 "What should we call you?"
- lead "We use it only for the greeting in your local app. You can change or remove it later."
- Field label "FIRST NAME (OPTIONAL)"; input placeholder "Your first name"; helper "Leave blank if you'd rather not
  say."; max 40 chars, trimmed; no validation beyond length.
- Back / Continue.

### 4.4 Step 03 — Locations

- h1 "Choose where to search."
- lead "Pick folders to index. Filename search is instant; content and OCR enrich in the background."
- Choices (label + description + checkbox, reference:150): Documents — "Notes and documents"; Desktop — "Files close
  at hand"; Downloads — "Recent downloads"; Pictures — "Screenshots and images". Plus "Add another folder…"
  (secondary, folder picker; added roots appear in the list).
- Note: "You can change these later in Search & indexes."
- Defaults: the four suggested folders are listed **unchecked**; nothing is indexed without an explicit choice. If
  no location is selected, Continue is enabled (filenames simply have nothing to search) but the step shows the
  helper "Choose at least one folder to start with filenames." — never block a user who wants to skip.
- Back / Continue / Skip setup.

### 4.5 Step 04 — Search depth

- h1 "Decide how deep to index."
- lead "Basic filename search never needs AI or an account. Choose optional content search and OCR; model downloads
  remain opt-in." (reference:148)
- Content: for each selected location a row: name + SegmentedControl "Names only" · "+ Content" · "+ OCR"; below the
  list, the trade-off text for the highest selected mode (11 px, changing with selection):
  - Names only — "Instant. Smallest index. No file contents are read."
  - + Content — "Extracts text from documents, code, and text files in the background. Uses more CPU and disk."
  - + OCR — "Reads text in screenshots and images. Uses the most CPU; results arrive later."
- Note: "Measured estimates appear after the first scan. Nothing is uploaded."
- If no locations: empty state "No locations selected." + text button "Back to locations".
- Back / Continue / Skip setup.

### 4.6 Step 05 — Shortcut

- h1 "Set your shortcut."
- lead "Press it anywhere to summon ProtonSearch."
- Control: kbd display "Alt + Space" + secondary "Change shortcut"; capture dialog as in Preferences (§3.6.1).
- Helper: "Use at least one of Ctrl, Alt, Shift, or Win. If another app already uses it, we'll tell you."
- Conflict error inline (role=alert): "That shortcut is already used by another app. Try another."
- "Reset to Alt + Space" text button; note "You can change this later in Preferences."
- Default: Alt + Space (legacy default, `settings.rs:149-150`; conflict detection `hotkey.rs:67-88`).
- Back / Continue / Skip setup.

### 4.7 Step 06 — Agent & models (opt-in)

- h1 "Add an agent, only if you want one."
- lead "ProtonSearch works fully without AI. Agent workflows are optional, start on demand, and ask before they act."
- Content: checkbox row "Set up agent workflows later" · description "The runtime stays off until you run one.";
  note "No models are downloaded automatically. You choose a provider and a model in Preferences."; disclosure
  "What leaves this PC?" expanding to truthful copy: "Nothing leaves this PC unless you configure a provider and run
  an agent workflow. Then the prompt and the data you approve are sent to that provider."
- Default unchecked.
- Back / Continue / Skip setup.

### 4.8 Step 07 — Ready

- h1 "Ready when you are."
- lead "Filename search is available now. Content and OCR will fill in the background; you can change everything
  later." (reference:148)
- Summary panel (metric rows): "Included locations" {n}; "Filename search" "Always available"; "Document content"
  "Enabled"/"Off"; "OCR search" "Enabled"/"Off"; "Agent workflows" "Set up later"/"Off"; "Shortcut" "Alt + Space".
- Note 11 px: "Indexing starts after you continue. You can keep using search while it runs."
- Primary "Start using ProtonSearch" → app Home (focus h1). Secondary "Finish setup later" → app Home with the
  incomplete-setup banner; indexing still starts in the background if locations were chosen.
- No fake percentage on this step; real progress appears on Home.

### 4.9 Resumable / skippable / existing users

- Every step transition persists `OnboardingState` (step index + values). Closing/reopening resumes at the saved
  step with the eyebrow showing "Getting started / 03 of 07" and a quiet 11 px line "Setup saved. You can close this
  window and continue later."
- "Skip setup" → `onboarding.skipped = true`; Home shows "Finish setting up ProtonSearch" + "Resume setup"; indexing
  starts only with whatever locations/modes were chosen (possibly none).
- Legacy users: if a legacy settings/index is migrated, mark onboarding complete; never force first-run (master
  prompt §V.C).
- Re-entry: Preferences → "Run setup again" link in the About section (restarts at step 1 with existing values
  pre-filled); "Run setup again" is only shown when onboarding is complete.

---

## 5. State coverage matrix

### 5.1 Launcher

| State | Required visual | Copy | Notes |
|---|---|---|---|
| first summon | window + focused input, compact 256 px | placeholder | no flash, no animation |
| typing / in flight | previous rows or skeletons | "Searching…" | no flicker |
| empty input | RECENT group or empty prompt | §2.3 | recents respect privacy |
| 1 result | single row, count "1 match" | — | selection = row 0 |
| many results | groups, max 50, scroll | "{n} matches" | virtualize > 40 |
| selected | tint+border+2 px bar | — | instant, identity-stable |
| preview | side/drawer panel | — | Ctrl+Space; Esc closes |
| 0 matches | empty state | §2.3 exact copy | no illustration |
| loading first query | 3 skeletons | "Searching…" | flat bars |
| search error | inline error + Retry | "Search is unavailable right now." | footer "Index unavailable" |
| indexing | normal + footer progress | "Indexing · 42 %" | truthful |
| paused | normal + footer | "Index paused" | — |
| dark | all above | — | on-dark mark |
| high contrast | system colours | — | §12.4 DESIGN_SYSTEM |
| 125/150/200 % DPI | clamps to work area | — | §6 |

### 5.2 App + onboarding

| Screen | Loading | Empty | Error | Offline | Other |
|---|---|---|---|---|---|
| Home | skeleton | no index / no recents | status block error | n/a | indexing, incomplete setup |
| Search & indexes | skeleton | no locations | per-source/root errors + Retry | network root unavailable | indexing, paused, rebuild |
| Activity | skeleton rows | no activity | inline Retry | n/a | recording paused, filtered empty |
| Workflows | skeleton | agent off | runtime/provider error | provider unreachable | starting/running/stopped |
| Preferences | skeleton | n/a | settings read error | updater offline | hotkey conflict, disabled reasons |
| Onboarding | n/a | no locations (step 4) | hotkey conflict | n/a | resumed, skipped, complete |

---

## 6. Responsive and DPI

### 6.1 Breakpoints

| Width band | Launcher | Desktop app | Onboarding |
|---|---|---|---|
| ≥1100 (1000–1280 measured) | 820 wide; side preview only if ≥1200 | sidebar 220; workspace 36/40; 2-col | rail 240; content 44/43 |
| 820–1099 | 820 wide; preview bottom drawer | sidebar 190; workspace 32/32; 2-col; label column stacks | rail 240; content 32/32 |
| 701–819 | `min(820, wa−32)`; drawer | sidebar 56 rail; workspace 31/24; 1-col | rail hidden; content full |
| 560–700 (**700 breakpoint**) | `min(820, wa−32)`, search 61 px, input 16 px, row meta hidden, logo 26, footer may wrap | rail 56; workspace 31/18; single column; settings stack; recent meta hides | rail hidden; eyebrow "Step n of 7"; footer stacks if needed |
| 560 (test width) | min supported; hints hide | min supported | min supported |

At ≤700 exactly (reference:94-100): launcher search row 61 px, input 16 px, trailing meta hidden, results max 350,
logo 26; app sidebar becomes 56 px with hidden labels (keep them sr-only), workspace 31/18, columns 1, onboarding
rail hidden, setup content padding 32/22 with min-height 520.

### 6.2 DPI (125 / 150 / 200 %)

- CSS px stay constant; physical size multiplies. Launcher width/height clamp to the work area:
  `min(820, workAreaCss.width − 32)`, `min(480, workAreaCss.height − 48)`; results max = `H − 116`.
- At 200 % on a 1366×768 display the work area is ~640×350 CSS px: launcher = 608×302, search row 72, footer 44,
  results 186 px (~3 rows), preview becomes a drawer with 140 px content height. Nothing overlaps; hints hide.
- Icons are requested at 32/48/64/96 px source for 100/125/150/200 % (never upscale); hairlines stay 1 CSS px
  (1.25/1.5/2 device px, rendered crisply by the compositor); text remains vector.
- Test list: 560/820/1000/1280 × 125/150/200 % × light/dark × 1.0 text size; plus Windows text size 125 % if the
  WebView honours it — record actual result (do not assume).
- High contrast and reduced motion are independent switches; test with Black + Aquatic HC themes and reduced motion
  on.

---

## 7. Motion per surface (summary; tokens in DESIGN_SYSTEM §6)

| Surface | Motion | Reduced motion |
|---|---|---|
| Launcher show/hide | none | none |
| Result selection / hover | colour 110 ms | 0 ms |
| Footer/status text | none (text swap) | same |
| Preview | content fade 160 ms + 6 px | none |
| Context menu | fade 90 ms + scale 0.98→1 120 ms | none |
| Route change (app) | none | same |
| Onboarding steps | content fade 160 ms + 4 px | none |
| Progress | 240 ms interpolation; indeterminate 1200 ms loop | instant / static bar |
| Dialogs | scrim fade 120 ms + dialog scale 0.98→1 160 ms | none |

Nothing animates on keyboard-open, Enter, Esc, or arrow-key selection beyond the colour cross-fade. No layout
animation anywhere.

---

## 8. Keyboard-only journeys (acceptance scripts; keys in KEYBOARD_AND_A11Y.md)

1. **First run:** launch app → onboarding step 1 → Tab to "Get started" → Enter → type name → Enter (Continue) →
   Space toggles a location → Continue → set depth → capture shortcut with Alt+Space → Continue → leave agent off →
   Continue → Enter "Start using ProtonSearch" → Home focused.
2. **Summon & search:** press hotkey → type "arch" → ↓↓ → Enter → item opens; launcher hides.
3. **Clear semantics:** type query → Esc (query clears, window stays) → Esc (window hides).
4. **Preview:** type → ↓ → Ctrl+Space (preview opens) → Tab into preview → PageDown → Esc (preview closes) →
   Ctrl+Space (reopens) → Enter (opens item).
5. **Copy path:** type → ↓ → Ctrl+C → footer announces "Path copied".
6. **Context menu:** type → ↓ → Shift+F10 → ↓↓ → Enter (Open containing folder) → focus returns.
7. **App navigation:** Tab from workspace → sidebar nav → ↓/↑ between items → Enter → route h1 focused → skip link
   visible on first Tab from window start.
8. **Settings:** Preferences → Tab to theme segmented → arrow keys change theme → "Saved" stamp → Tab to hotkey
   "Change…" → Enter → press Ctrl+Alt+Space → Enter → conflict message if taken; Esc cancels capture.
9. **Recovery:** mock index error → Home shows inline Retry → Enter on Retry → status returns; launcher still works.
10. **Onboarding resume:** close setup at step 3 → reopen → step 3 with "Setup saved."; Skip setup → Home banner
    "Finish setting up ProtonSearch" → Enter resumes.

---

## 9. Copy deck (states and errors)

| Context | Exact copy |
|---|---|
| Launcher placeholder | "Search files, apps, and actions…" |
| Counts | "{n} matches" · "1 match" · "No matches" · "Searching…" |
| Launcher 0-match | "No matches for “{query}”." / "Try a different spelling, or a prefix such as file:, history:, or code:." |
| Launcher empty prompt | "Start typing to search files, apps, and actions." / "Try a file name, an app, or a command like “lock”." |
| Launcher error | "Search is unavailable right now." · "Retry" |
| Index status | "Index ready" · "Indexing · {n} %" · "Indexing…" · "Index paused" · "Index unavailable" |
| Footer link | "Open ProtonSearch" |
| Preview unavailable | "No preview for this item." |
| Home no index | "Nothing indexed yet." · "Set up indexing" · "Resume setup" |
| Home incomplete | "Setup is {n} of 7 steps done." · "Resume setup" |
| Search & indexes no roots | "No locations selected." · "Add location…" · "Locations you add stay on this PC." |
| Remove location confirm | "Remove {folder} from the index? Results from this folder stop appearing. Your files are not changed." |
| Rebuild confirm | "Rebuild the index? Search stays available while filenames are re-scanned. This can take a few minutes and uses more CPU." |
| Activity empty | "No activity yet." · "Searches and launches appear here while recording is on." |
| Activity paused | "Activity recording is paused." · "Resume" |
| Clear activity confirm | "Clear all activity? This removes {n} recorded items from this PC. It can't be undone." |
| Workflows agent off | "Agent workflows are off. ProtonSearch works fully without them." |
| Workflows runtime error | "The agent runtime couldn't start. Search is unaffected." · "Retry" |
| Preferences save | "Saved" (2 s stamp) |
| Hotkey conflict | "That shortcut is already used by another app. Try another." |
| Updates offline | "Can't check for updates right now — you're offline." |
| Onboarding saved | "Setup saved. You can close this window and continue later." |
| Onboarding skip | "Finish setting up ProtonSearch" |

## 10. Acceptance screenshots (must exist before G4 sign-off)

Launcher: first summon, empty input, 0 matches, 1 match, many+selected, preview (side and drawer), loading, index
error, indexing, dark, 560/820/1000/1280, 125/150/200 % DPI, high contrast, keyboard focus. App: Home (loaded,
loading, empty, incomplete, error), Search & indexes (locations, depth, health, rebuild confirm), Activity (list,
empty, paused), Workflows (off, running, error), Preferences (theme/density, hotkey capture, conflicts). Onboarding:
all 7 steps, resumed state, narrow (560), dark. Reference comparison sheet: reference HTML vs production for launcher,
Home, onboarding, light/dark (master prompt §XI.3).
