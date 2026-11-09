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
| Quick Launcher | frameless, floating; card 820 px wide (preview 1180), window = card + 56 px shadow pad; card 296–544 px tall (window 352–600) | none: no title bar, no min/max/close, no tabs | separate launcher window (Tauri), route `launcher` |
| Desktop App | resizable standard window 1120×760 default, min 560×480, native Windows title bar "ProtonSearch" | native frame; in-window sidebar + workspace | app window, routes `/home`, `/search-indexes`, `/activity`, `/workflows`, `/preferences` |
| Onboarding | resizable window 900×620 default, min 560×480, native title bar "Set up ProtonSearch" | native frame; no app sidebar | app window route `/onboarding` (modal route, hides shell) |
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
| match count, status word, indexed counts | `EngineStatus` from `get_status` |
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
│ [ALL] [FILES] [FOLDERS] [APPS]                                 │  filter strip 40px, chips 26px, gap 6
├──────────────────────────────────────────────────────────────┤
│ DOCUMENTS                                                    │  group label: 34px block, 10px caps
│ [icon] Architecture notes.docx                        Yesterday│  row: 58px, pad 8/12, radius 8
│ [icon] search-architecture.md                             Today│  ...
│ FOLDERS                                                      │
│ [icon] Architecture                                 24 items │
├──────────────────────────────────────────────────────────────┤  1px --ps-line
│ ● Index ready · Open ProtonSearch       [↑↓] Navigate [Enter] Open [Esc] Clear │  footer 44px
└──────────────────────────────────────────────────────────────┘
```

- **Native window** = card + 56 px (`--ps-launcher-shadow-pad: 28px` each side, transparent). Card width
  `W = min(820, workArea.width − 80)` (24 px screen margin + 56 px pad); preview card
  `W = min(1180, workArea.width − 80)` when `workArea.width ≥ 1200`; window width = `W + 56` (876 regular /
  1236 full preview). _Do not let the card touch the window edge: the shadow is clipped there._
- **Card height** `H = clamp(296, 72 + 44 + 40 + 23 + Σ(34 + rowH·n), 544)` where 44 px is the footer, **40 px the
  filter strip**, 23 px the results padding (10 top + 13 bottom); `rowH` 58 (comfortable) / 48 (compact); group label
  block 34 (30 compact); strip 40 (36 compact). Window height = `H + 56`. Height snaps to 2 px; resize is instant and
  happens in the same frame as the content change (never animated).
- Results max-height = `min(365, H − 156)`; clipboard sections (§2.12) live inside the results viewport and add no
  separate row or height; scrollbar thin, thumb `--ps-scrollbar-thumb`. The filter strip lives inside the results column,
  so the preview panel keeps the full body height.
- Header: brand mark (light or on-dark variant per theme, 29 px; 26 px at ≤700) + input + count.
- Empty-input minimum card height is 296 px (search 72 + strip 40 + results 140 + footer 44) — compact, not an empty
  stage. The 28 px window pad is not click-through: a press there dismisses the launcher.

### 2.2 Results model

- The UI receives a ranked list, groups by `SourceKind`, and renders only non-empty groups in **fixed order**:
  **APPLICATIONS · FILES · FOLDERS · DOCUMENTS · CLIPBOARD · WEB · CODE · ACTIONS**. Group labels are the literal
  uppercase strings above (content labels, rendered with CSS caps; localised). The exact-command / calculator ACTIONS
  pin is **CUT for v2.0 alpha** (§10): ACTIONS always renders last, even when a command or expression matches exactly.
  Filter chips (§2.11) expose only **ALL/FILES/FOLDERS/APPS**; every other source (clipboard, web, documents/OCR, git,
  actions) stays in **ALL** and is reachable with query prefixes or the empty-state quick destinations (§2.4). Chips are
  exact: FILES never shows folder rows and FOLDERS never shows file rows (§2.11). When the clipboard scope is active via
  its `clipboard:` prefix (or the Clipboard History quick destination), clipboard rows are grouped into category
  sections (§2.12); in ALL they stay in the single CLIPBOARD group.
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
| Indexing (results still served) | normal results | normal | dot warning + "Indexing" (alpha: the IPC has no percentage — §10) |
| Index paused | normal results | normal | dot neutral + "Index paused" |
| Offline (network) | no special state — local search is unaffected; unavailable network roots are simply absent | normal | normal |
| Preview open | rows + side/drawer preview; selected row unchanged | normal | "Preview" hint |
| Filter active | one of the four chips is selected; results are scoped; FILES/FOLDERS are exact object types; all other scopes require a typed prefix/quick destination | count reflects the scoped set | unchanged |
| Empty input, no recents | QUICK DESTINATIONS panel replaces the empty prompt (see §2.4); no fake rows | placeholder normal | status |
| Preview loading/ready | preview pane shows skeleton, then the real IPC payload (text/image/OCR/fields/highlight/truncated); unsupported/error are explicit | normal | "Preview" hint |

Filter-chip states, copy and keyboard behaviour are in §2.11. Chips stay interactive while a search is in flight; an empty scoped result uses "No matches in {label}." + "Try another category, or press Esc to clear the filter."

### 2.4 Empty-input content rules

**Quick destinations (v10).** When the query is empty, the scope is ALL, there are no recents/results (or recents are
disabled), and no request is pending/erroring, the results area shows **QUICK DESTINATIONS** instead of an empty prompt.
It is a listbox of real destinations, not fake suggested commands. Counts/state come only from `EngineStatus`; never
invent data.

- Heading: "QUICK DESTINATIONS" — 10 px/750, +0.12em, `--ps-ink-3`; padding 4 14 8 (inside the results column).
- Primary grid: **2 columns**, gap **8**, inline padding 12; four destinations in this order: **Browser Bookmarks,
  Browser History, Clipboard History, Agents**. Each tile: min-height **54**, padding **9 12**, 1 px line border, radius
  **9**, raised surface; selected = `--ps-accent-soft` fill + `--ps-accent-border`; label 13/680, subtitle 10/ink-2
  single-line ellipsis; off state subtitle ink-3.
- Secondary row: **Git history** and **Agent History**, quiet pills (flex wrap, gap 6, padding 8 12 0; button
  padding 5 9, radius 6, transparent, row layout); label 13/680, subtitle 10.
- Subtitle copy (exact templates): Browser Bookmarks "Search saved bookmarks" / off "Off — enable in Preferences";
  Browser History and Clipboard History real count ("{n} items" longform; "No items indexed yet"; "Search this source"
  fallback; off copy); Agents "Agent runtime running" / "stopped" / "needs attention" / "No agent configured";
  Git history same count/off pattern; Agent History "Open Workflows history".
- Actions: Bookmarks/History/Clipboard/Git **write their prefix into the query** (`bookmarks:`, `history:`,
  `clipboard:`, `git:`) and run the search; Agents/Agent History open the main app Workflows history. An off
  destination opens the main app (Preferences) instead of faking a search.
- Syntax hint below the list: 10/`--ps-ink-3`, margin 8 14 0, line-height 1.4:
  "Prefixes: bookmarks:, history:, clipboard:, git:, ocr:, content:, calc:".
- Keyboard: the destinations are listbox options in DOM order (2×2 primary, then secondary); ↑/↓ from the input move
  through them, Enter activates; hover selects; no extra Tab stops. The listbox is labelled "Quick destinations"; the
  secondary group is "More destinations". The result-count live region does not update for this static state.
- Geometry: quick destinations reserve **248 px** (comfortable) / **224 px** (compact) of extra content height; the
  launcher window grows within the normal max/height clamp. At ≤700 the grid becomes one column and heading/secondary/
  hint use 8 px inline padding.
- Recents still win when present: empty input + recent items renders the RECENT group (not quick destinations). When
  recents are disabled, quick destinations render; no privacy explanation banner.

- No fake "suggested commands", no tiles, no tips carousel beyond these real destinations.

### 2.5 Footer

- Left: 8 px status dot + status word + "·" + text button "Open ProtonSearch" (10 px/750, `#356d88`). Status is a
  polite live region; colour always paired with words. Truthful mapping: ready / indexing / paused / starting / unavailable (no
  percentage in alpha — §10). Never show "Index ready" before the engine reports ready.
- Right: at most three kbd hints, in priority order, contextual (reference:136 shows Navigate/Open/Clear):

| Context | Hints |
|---|---|
| rows exist, query empty | `[↑↓] Navigate` · `[Enter] Open` · `[Esc] Close` |
| rows exist, query typed | `[↑↓] Navigate` · `[Enter] Open` · `[Esc] Clear` |
| no rows, query typed | `[Esc] Clear` |
| preview open | `[Ctrl+Space] Close preview` · `[Esc] Close` (preview closes first) |
| copy-path feedback | left status becomes "Path copied" for 2 s (role=status) |

- Multi-select / Paste-N hints are **CUT for v2.0 alpha** (§10); the footer never shows a Paste-N hint. Hints hide by
  priority (Copy → Preview → Navigate) below 900 px work width; never wrap above 900 px. Keep
  "Esc" semantics visible: Clear when a query exists, Close when empty (reference footer says "Esc Clear").
- "Open ProtonSearch" opens the app on Home with the current query pre-filled into the app search where applicable;
  it is a button, not a link (no browser semantics).

### 2.6 Preview (Ctrl+Space) — real IPC payload (G2, task-47)

- Toggle only when a row is selected. Default **off**; Preferences may enable "Preview the selected result".
- Placement (unchanged): side panel **360 px** at `workArea.width >= 1200` (card 1180, window 1236, height unchanged);
  bottom drawer **240 px** tall otherwise (window grows, cap min(640, workArea.height−24)). Panel animates in 160 ms,
  opacity + 6 px, reduced motion instant; the window resize never animates.
- Head: result icon + title 13/680 + format/source line 11 ink-2 + path/URL 11 selectable/ellipsis.
- The panel now renders the real engine `PreviewPayload`; no placeholder body:
  - **Loading:** flat 3-line skeleton at the real body size (no shimmer, no layout shift); late/pending requests are
    token-guarded and debounced 120 ms.
  - **Text/code:** `payload.text ?? payload.snippet` in mono **11/1.55**, max-height **300**, pre-wrap/break-anywhere,
    with real `highlightRanges` rendered as `mark` (accent-soft, 3 px radius). Label is "{FORMAT} preview" or
    "Preview". Truncation: "Showing the first {n} characters. Press Enter to open the full item."
  - **Image:** real `imageDataUrl` in a bordered wrap (padding 8, radius 8, surface-soft); image max-width 100% /
    max-height 300, object-contain, radius 6, alt "Preview of {title}".
  - **OCR:** separate "Text found in image" section under a dashed top border; mono 11/1.55, max-height 300.
  - **Metadata:** `payload.fields` rendered as a definition list; path/URL stays selectable.
  - **Unsupported:** `payload.unsupportedReason` or "No preview for this item."; no fake render, no dead controls.
  - **Error:** "Preview is unavailable right now." + quiet detail + **Retry** (reloads that result); error detail must be
    sanitized (no raw exception text/paths). **Known code gap:** the current preview error detail can surface the
    bridge message; fix before release (recorded in REVIEW_3/G4-G5 sync).
  - **Empty body (status ok):** "No preview body for this item."
  - **No selection:** "Select a result to preview it."
- Action row: only actions the engine's `ActionSpec` declares (Copy path, Open, Open containing folder, etc.), max one
  row; preview actions do not move the launcher's active row.
- Loading skeleton must be flat grey (no gradient/pulse); current implementation uses a subtle pulse gradient — flagged
  for correction in the G4/G5 sync, not part of the binding visual.
- Esc closes the preview first, then clears, then hides. Ctrl+Space toggles closed. Focus: scroll region tabbable,
  ↑↓/PageUp/PageDown scroll when focused; Tab returns to the input.

### 2.7 Multi-select — **CUT for v2.0 alpha**

Clipboard multi-select and Paste-N are postponed for the alpha, not merely unbuilt. Rationale: the feature needs a
selection model layered over the active-row model, a text-paste aggregation path, and IPC/action support the alpha
contract does not carry; shipping a stub (dead checkboxes, a disabled Paste-N) would break the truthful-UI rule.
Alpha behaviour: every clipboard result is a single item — Enter pastes it, the context menu offers Paste/Copy/Pin/
Remove, the footer never shows a selection count, and Shift+Enter / Shift+↑↓ / Ctrl+A are unbound. Re-entry point and
post-alpha acceptance criteria are recorded in §10.

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

↑/↓ navigate (wrap) · Enter open · Esc menu→preview→clear filter→clear query→close · Tab/Shift+Tab move between input,
filter strip (selected chip), preview (when open) and the footer's "Open ProtonSearch" · ←/→ change filter when the
strip has focus · Ctrl+Space preview toggle · Ctrl+C copy path (when the input has no text selection) · Shift+F10/Menu
open context menu · Home/End/PageUp/PageDown in the list.
No other global chords; the summon hotkey is configured in Preferences/onboarding.

### 2.10 Launcher motion — FINAL 2026-11-09 (launcher untouched by task-8)

**Open (summon) 150 ms.** Card only: opacity 0→1, translateY(-8px→0), scale(0.985→1);
`--ps-ease-out-expo = cubic-bezier(0.16, 1, 0.3, 1)`. The window shows and the input takes focus on the first frame;
typing, search dispatch and results are **not** delayed by the animation. The action that summoned it (hotkey) stays
instant; the card fades in around it.

**Close 110 ms.** Card: opacity 1→0, translateY(0→-4px), scale(1→0.99);
`--ps-ease-launcher-out = cubic-bezier(0.4, 0, 0.2, 1)`. The action itself runs immediately (Enter/click opens the
result before/while the card fades); the native window hides on `animationend` or a 200 ms fallback. Copy actions never
close. Re-summon during close cancels the close and restarts open (Rust safety hide must be cancellable).

**Instant:** query typing/caret/IME, search dispatch, result list replacement, selection (110 ms colour only), footer
status text, window resize, preview geometry, filter application (search dispatches immediately), scrolling. Reduced
motion: 0 ms, no transform, window hides next frame. No shadow animation, no height animation, no bounce.

### 2.11 Filter strip — FINAL 2026-11-10 (task-47, four-chip normal launcher)

A horizontal filter strip between the search row and the results. It is the reference's segmented-control language
applied to a scrolling filter row. The v10 Normal Launcher exposes **four chips only**; every other source is reached by
the query syntax or the empty-state quick destinations.

**Placement and geometry (unchanged).** Inside the results column, directly under the search row's hairline and above the
results: block **40 px** (36 compact) = 7 px top + 26 px chip + 7 px bottom + 1 px bottom hairline; inline padding **16 px**
(12 compact); chip height **26 px** (24 compact); gap **6 px** (5 compact); chip radius **6 px**; chip padding-inline
**10 px** (8 compact). The strip scrolls horizontally; it never wraps and never changes the results column width.
Tokens: `--ps-strip-height/--ps-strip-height-compact`, `--ps-chip-height/-compact`, `--ps-chip-gap`,
`--ps-chip-pad-x`, `--ps-chip-radius`, `--ps-strip-pad-x`.

**Chips and order (FINAL).** Exactly four chips, fixed order:
**ALL · FILES · FOLDERS · APPS.** Labels are uppercase, localised, 11/600, `--ps-ink-2` at rest; no counts, no icons, no
truncation. Do **not** render chips for CLIPBOARD, WEB, ACTIONS, DOCUMENTS/OCR or GIT: those scopes stay in ALL and are
reachable via prefixes/quick destinations. Never render a ghost/disabled chip.

**Shortcuts.** From the search input, **Ctrl+1 … Ctrl+4** select ALL/FILES/FOLDERS/APPS in that order, without moving
focus out of the input. **Ctrl+5 and above are unbound** — do not capture them. The radiogroup state and result-count live
region announce the change; chips show no shortcut hint.

**Scopes and prefixes.** Selecting a chip dispatches a new search with `scope` and resets the selection to row 0. Typed
prefixes do **not** select a chip (the strip stays on ALL; the query text is the only indicator); choosing any chip clears
the leading prefix. Everything outside the four chips remains reachable:

| Chip / scope | `scope` | Query prefixes |
|---|---|---|
| ALL | `all` | (none) |
| FILES | `files` | `file:` `files:` |
| FOLDERS | `folders` | `folder:` `folders:` |
| APPS | `apps` | `app:` `apps:` |
| *(no chip)* CLIPBOARD | `clipboard` | `clip:` `clipboard:` |
| *(no chip)* WEB | `browser` | `web:` `history:` `bookmarks:` `google:` |
| *(no chip)* DOCUMENTS / OCR | `content` | `content:` `ocr:` |
| *(no chip)* CODE / GIT | `git` | `git:` `commits:` `todos:` |
| *(no chip)* ACTIONS | `actions` | `calc:` `command:` |

Filter choice is **not** persisted across summons (default ALL). Quick destinations (§2.4) write the bookmark/history/
clipboard/git prefixes; other scopes are typed directly.

**Strict FILES vs FOLDERS (FINAL).** Exact object types; never mixed:
- **FILES** = file-backed rows only: `SourceKind::File`, `content_match` and `ocr_text`. A folder row never appears.
- **FOLDERS** = `SourceKind::Folder` only. A file/document row never appears.
- **ALL** keeps the fixed groups separate (FILES, FOLDERS, DOCUMENTS distinct; no mixing inside a group).
- Backend debt: `ps-core` still maps FILES to `File | Folder` (`crates/ps-core/src/engine.rs:1517-1519`); the
  launcher filters folder rows out of FILES (`guardScopeResults`) until the engine change lands.

**Row anatomy.**

| Scope | Row anatomy |
|---|---|
| FILES row | 36 px icon slot · 12 px gap · real shell/file-type icon 32 px **or** extension badge (2–4 chars, 10/650, caps, 35×37, radius 7, `--ps-badge-*`) · filename 13/680 ellipsis · path 11 `--ps-ink-2` below (4 px gap, ellipsis) · trailing recency 10–11 + size when real. Content-match rows keep the file row and open at the match. |
| FOLDERS row | 36 px icon slot · 12 px gap · folder glyph 20 px tinted `--ps-badge-folder-bg/ink` (never a file icon/extension badge) · folder name 13/680 ellipsis · full path 11 `--ps-ink-2` below · trailing "{n} items" only when `meta.itemCount` is real, otherwise nothing. |
| Mixed ALL | Row types keep their group headings; no visual mixing inside a group. |

**States.** Unchanged: rest transparent/ink-2; hover `--ps-surface-soft`/ink; selected `--ps-accent-soft` + 1 px
`--ps-accent-border` + **2 px inset bottom bar** `--ps-selected-bar` + ink 650 (bar = 6.45:1 light / 6.04:1 dark);
focus-visible 2 px `--ps-focus` ring offset 2; pressed surface-soft; no disabled state while a query is pending.

**Keyboard and focus.** One roving tab stop: Tab from the search input lands on the selected chip; Tab again leaves the
strip for the preview/footer. ←/→ change the filter immediately and keep focus; Home/End go to ALL/APPS; Space/Enter
re-applies; Esc clears to ALL and returns focus to input before the normal query-clear/close chain. Ctrl+1–4 select from
the input. Overflow scrolls the focused/selected chip into view (`scrollIntoView({inline:'nearest',block:'nearest'})`,
no smooth behaviour). Pointer click only.

**0 results.** "No matches in {label}." + "Try another category, or press Esc to clear the filter." Count reflects the
scoped set.

**Overflow/localisation.** Horizontal scroll only, hidden scrollbar, no fades; labels from `filter.all`,
`filter.files`, `filter.folders`, `filter.apps` only (no unused chip keys); never truncate; logical properties for RTL.
`aria-label="Filter results"`; chip accessible names localised; one result-count live region, no extra live region.

**Accessibility.** `role="radiogroup"`; each chip `role="radio"` + `aria-checked`; roving tabindex; never tabs or
aria-pressed. Ctrl+1–4 are shortcuts, not tab stops. Motion: background/border/box-shadow 110 ms; reduced motion 0 ms.

### 2.12 Clipboard categories — grouped sections (FINAL 2026-11-10, task-47)

When the **clipboard scope** is active — via the `clipboard:` / `clip:` prefix or the Clipboard History quick
destination — the returned clipboard rows are grouped into quiet category sections **inside the results viewport**. There
is no Clipboard chip and no second chip row: the main strip stays on ALL, and the prefix/quick destination is the scope.
No category filter controls and no duplicated filtering.

**Order and visibility.** Fixed order **LINKS · EMAILS · PATHS · CODE · TEXT · IMAGES**; only non-empty sections render,
each exactly once. Classification uses the rules below; TEXT is the fallback. IMAGES renders only when the engine returns
real image rows (alpha: images currently depend on the G2 image IPC; no fake section). Section order never changes with
query, score or pinned state. When the clipboard scope is not active (ALL), clipboard rows stay in the single CLIPBOARD
ResultGroup in engine order and are not sectioned.

**Section labels (subordinate to ResultGroup).**
- Uppercase **10 px/700**, letter-spacing **+0.10em**, colour `--ps-ink-3`; line-height **14 px**; block **28 px**
  (24 compact) = **8 px top + 14 px line + 6 px bottom**; inline padding **14 px**; no count, no icon, no hairline, no
  action, no collapse control. (ResultGroup stays 34/30 block, 10/750/+0.12em — clipboard sections are one step quieter.)
- Spacing: the first section label starts at the results viewport's normal top padding; every following section is
  preceded by **6 px** vertical gap. Rows keep the standard **58 px** (48 compact) pitch.
- Localisation keys: `clipboard.links`, `clipboard.emails`, `clipboard.paths`, `clipboard.code`,
  `clipboard.text`, `clipboard.images`; no truncation; logical `padding-inline`/`gap` for RTL.
- Dark: label `--ps-ink-3` exactly as light (do not dim below 4.5:1); same CSS px metrics at 125/150/200 %.
- Semantics: each section is `role="group"` + `aria-labelledby` to its visible label; the label element is
  `role="presentation"` (same pattern as ResultGroup); sections add **no tab stop and no interactive control**.

**Row anatomy (no per-row category tags).** The section label is the category; do not repeat it as a tag. Rows keep the
standard clipboard anatomy: 36 px icon slot · clipboard glyph 20 px or 32×32 image thumbnail radius 4 (cover; never
stretch) · title = first non-empty line, whitespace collapsed, max 120 chars, 13/680 ellipsis · sub 11
`--ps-ink-2` = "`{source app} · {relative time}`" · trailing relative time 10–11 `--ps-ink-3`. Long text stays
single-line; preview/copy actions unchanged. Pinned rows sort **first within their section**, then engine order (newest);
pinned is never a separate section.

**Category classification (grouping only; not a UI control, not a security boundary).** Deterministic, from the text
exposed to the launcher (first line / 120 chars):

| Section | Rule |
|---|---|
| LINKS | Trimmed single token matching `^https?://\S+$` |
| EMAILS | Trimmed single token matching `^[^\s@]+@[^\s@]+\.[^\s@]{2,}$` |
| PATHS | Trimmed single absolute Windows path: `^[A-Za-z]:\\` or `^\\\\` or `^%[A-Za-z_]+%\\`; matching quotes are stripped |
| CODE | Not LINKS/EMAILS/PATHS and either a fenced block or ≥2 lines with ≥2 code signals (`{ } ; => def function class import </`, indentation) |
| TEXT | Everything else (always catches plain text) |
| IMAGES | The clipboard item is an image (engine flag; alpha: no image rows exist, so this section is absent) |

**Height and overflow.** Section labels live inside the results viewport: **no extra 36 px**, no second strip, no change
to the filter strip or window minimum. Standard maths: card `H = clamp(296, 72 + 44 + 40 + 23 + Σ(34 + rowH·n), 544)`,
window `H + 56` (min 560×352), results max `min(365, H − 156)`. The section labels count inside the viewport's content
like normal group labels; no +36 px, no 332/388 minimum. The viewport scrolls; section labels scroll with rows (no sticky
headers); no collapse/expand.

**Keyboard (flattened).** ↑/↓ traverse the visible rows continuously across section boundaries — no stop on a section
label; Home/End first/last row; PageUp/PageDown unchanged; typing stays in the input. Enter/Ctrl+C unchanged. Esc chain is
back to **menu → preview → clear filter → clear query → hide** (no category step). Tab order is back to
input → filter strip (selected chip) → preview/footer; sections add no tab stops. Screen readers hear the section label
when the active option crosses into it; no extra live region.

**Empty and loading states.** No per-category empty states exist: empty sections are simply not rendered.

| Situation | Copy |
|---|---|
| Clipboard history off | "Clipboard history is off." / "Turn it on in ProtonSearch → Preferences to keep copied text on this PC." |
| History on, no items at all | "No clipboard history yet." / "Copied text and images will appear here from now on. Only copies made while ProtonSearch is running are captured." |
| Query returns no clipboard matches | "No matches in Clipboard." / "Try different words, or press Esc to clear the filter." |
| Loading | Standard 3 skeleton rows at 58 px pitch; no section labels until data lands; previous rows stay during subsequent queries. |
| IMAGES unavailable | No IMAGES section is rendered (no fake section, no empty category); the IPC gap is stated honestly in §10. |

Capture remains live-only: ProtonSearch does not import the Windows clipboard history and cannot show copies made before
it was running. No category chips, no category counts, no category shortcuts, no filtering controls of any kind.

---

## 3. Full Desktop App

### 3.0 App type scale and alignment grid (FINAL 2026-11-09)

User feedback: the app text is too small and alignment reads inconsistent. Every app/onboarding surface now uses one
larger scale and one 12-column grid. The launcher keeps its final scale and specs (shadow/motion/filter FINAL — §2.10,
§2.11); nothing in this section changes the launcher. Exact tokens: DESIGN_SYSTEM §4.2b (type) and §5.5 (grid).

**Type scale (app + onboarding; launcher unchanged)**

| Role | Size / line-height | Weight | Token |
|---|---|---|---|
| Page title h1 | `clamp(28px, 3.6vw, 44px)` / 1.08, `-0.04em` | 600 | `--ps-text-display` |
| Setup h1 | 34 / 1.16, `-0.04em` | 600 | `--ps-text-h1-setup` |
| Stat | 36 / 1.0, `-0.05em` | 650 | `--ps-text-stat` |
| Page lead | 15 / 24 | 400 | `--ps-text-lead` |
| Body | 14 / 21 | 400 | `--ps-text-body` |
| Row title / strong | 14 / 20 | 600 | `--ps-text-strong` |
| Small / nav / metric label | 13 / 20 | 400–600 (nav 600, active 700) | `--ps-text-small` |
| Meta / metric unit | 12 / 18 | 400–600 | `--ps-text-meta` |
| Micro caps / field label | 11 / 16 + caps (+0.06em) | 650–750 | `--ps-text-micro` |
| Section label | 12 / 16 + caps (+0.08em) | 750 | `--ps-text-section` |
| Kbd | 11 / 14 | 650 | `--ps-text-kbd` |
| Control / button | 13 / 20 | 700 | `--ps-text-control` |

**Grid** (base unit 4 px; vertical rhythm 4/8 px)

| Item | Value |
|---|---|
| Content width | 1120 px (excluding window padding) |
| Window margins | 40 px ≥1100 · 32 px 820–1099 · 24 px 700–819 · 20 px ≤700 |
| Columns / gutter | 12 columns; 24 px gutter ≥820, 16 px ≤819 |
| Sidebar | 232 / 200 / 64 rail |
| Home | left 7 / right 5 columns, gap 24, stacked ≤819 |
| Settings / Indexes | label column 280 px + 24 px gap + panel; stacked ≤1023 |
| Card / panel padding | 24 px (16 compact) |
| Section rhythm | 32 px between sections · 24 px between panels · 24 px card padding |
| Page header rhythm | eyebrow → 8 → h1 → 12 → lead (max 60 ch) → 32 → content; 64 px bottom |
| Rows | nav 44 · list 68 · metric 44 · control 36 · field 40 · buttons 28/36/40 |
| Icons | 16 inline/nav · 20 state/panel · 32 file/app |

```css
.app-shell { grid-template-columns: var(--ps-app-sidebar) minmax(0, 1fr); }
.workspace {
  max-width: calc(var(--ps-content-max) + 2 * var(--ps-app-margin));
  padding: var(--ps-app-margin);
}
.route-grid { display: grid; grid-template-columns: repeat(12, minmax(0, 1fr)); column-gap: var(--ps-app-gutter); }
.home-columns > :first-child { grid-column: span 7; }
.home-columns > :last-child { grid-column: span 5; }
.settings-section { grid-template-columns: var(--ps-app-label-col) minmax(0, 1fr); column-gap: 24px; }
.panel { padding: var(--ps-app-card-pad); }
```

Rules:
- **One left edge, one right edge.** Eyebrow, h1, lead, search entry, first panel and section labels start on column 1;
  the last panel ends on column 12. Panels span columns, never magic widths; nothing stops 1–2 px short of the grid.
- **Baseline rhythm.** Section headings share a baseline; a panel's first line is 24 px from its top; row text is
  vertically centred in 4 px-multiple rows. No 3/5/7/9/13/15 px gaps anywhere in the app.
- **Numbers.** Right-aligned, tabular figures; label left (`--ps-text-small`, ink-2), value right (`--ps-text-body`,
  600, ink); units in `--ps-text-meta` ink-2; zero prints "0", unavailable "—"; deltas carry an explicit sign and are
  never colour-only.
- **Type comes from tokens only.** No `font-size: 12px` literals in views or CSS.

**Violations to fix in the current build (initial audit):** `desktop.css:24` sidebar 220 and `:897/:920` 190/56 →
232/200/64; `:172` workspace 36/40 + max 1040 → margin token + 1120; `:901/:963/:983` 32/32, 31/24, 31/18 →
32/24/20 margins; `:252-253` home 1.25fr/.9fr gap 38 → 7/5 gap 24; `:813` settings label 200 → 280; `ui.css:36`
panel padding 20 → 24; nav/list/metric/control/field tokens 40/64/—/32/38 → 44/68/44/36/40; `base.css:113` button
12 px literal → `--ps-text-control`; page-header lead margin ~25 → 12, header→content 37 → 32; icon sizes 15/16/18/19
→ 16/20/32; raw error text rendered at `IndexesView.tsx:275`, `HomeView.tsx:122`, `ActivityView.tsx:155,204`,
`WorkflowsView.tsx:151`, `PreferencesView.tsx:132` → §3.7; mixed radii 8/12 → panel 12/control 6/row 8/field 7.

**Fit:** at 560 px window (64 rail + 24 margins) content is ~472 px; body 14 at 40–55 ch fits, label/value columns
stack, settings label column stacks ≤1023, Home stacks ≤819. At 125/150/200 % type scales physically (14 → 17.5/21/28 px);
allow vertical scrolling, never shrink text to fit and never drop below 11 px caps.


### 3.1 Shell

- Native window frame (deviation from the reference's card-on-stage; the app fills the window — master prompt §V.B).
- Layout: sidebar + workspace, on the 12-column grid (§3.0). Sidebar `--ps-sidebar`, 1 px `--ps-line` right border:
  - brand block: 29 px mark + "ProtonSearch" 13/750/`-0.035em` (reference:54);
  - label "WORKSPACE" (11/750/+0.12em, `--ps-ink-3`) (reference:54);
  - nav: Home, Search & indexes, Activity, Workflows, Preferences — 16 px icon in 20 px slot + 13/600 label,
    row 44 px, radius 6, hover `--ps-surface-soft`, active `--ps-nav-active-bg` + `--ps-nav-active-ink` + weight 700
    + a 2 px inset `--ps-selected-bar` start bar (accessibility correction confirmed 2026-11-09; reference:55-58);
    icons are real SVGs, not `⌂ ⌕ ◷ ↗ ⚙` placeholders;
  - userfoot pinned bottom: avatar 32 px (initial of optional name) + first name 12 px; without a name: neutral
    person glyph + "This PC" (never infer identity — master prompt §V.B); static, not a control.
- Workspace: window margins 40/32/24/20 px per band, content max-width 1120, 12-column grid with 24/16 px gutters,
  left-aligned (no centring). Page header: eyebrow 11 caps → 8 → h1 clamp(28–44) → 12 → lead 15/24 max 60 ch → 32 →
  content; 64 px bottom padding.
- Skip link "Skip to main content" as the first focusable element; route change moves focus to the route h1
  (`tabindex="-1"`) and updates `document.title` ("Home — ProtonSearch").
- Sidebar widths: 232 (≥1100), 200 (820–1099), 64 icon rail (≤819, labels visually hidden but present for AT and shown
  as tooltips). At ≤700 the rail stays 64 and content stacks.

### 3.2 Home

**Question it answers:** "Is my index healthy and what can I do right now?"

| Element | Spec / exact copy |
|---|---|
| Eyebrow | "Your workspace / Home" (rendered uppercase tracked, 11 px) |
| H1 | Time-of-day greeting with optional name: `Good morning[, {name}].` — buckets 05:00–11:59 morning, 12:00–16:59 afternoon, 17:00–04:59 evening, local time via `Intl.DateTimeFormat`, punctuation final "." ; no name → "Good morning." (reference:140) |
| Lead | "Everything on your computer, within reach." — 15/24, max 60 ch (reference:140) |
| Big search | Button, **52 px** tall, raised surface, 1 px border `#dce4db`, radius 8, **14 px** placeholder-ink text "Find a file, open an app, or run an action…", search glyph left, ↗ right (reference:64, 140). Opens/summons the launcher. Accessible name = visible text. |
| Columns | **7 / 5 of 12 columns**, gap 24 (reference:67); ≤819: single column, gap 24 |
| Recently accessed | h2 "Recently accessed" (12 caps 750) + up to 3 recent rows (icon 32, title **14/600**, sub **12** `--ps-ink-2`, trailing arrow) + secondary button "Manage sources" (opens Search & indexes) (reference:69-71, 140) |
| Your index | h2 "Your index" (12 caps) + stat **36/650** (real total, tabular) + sub **12** "items indexed on this PC" + state line ("Indexing" / "Index up to date"; alpha limitation: the IPC has no percentage, so no progress bar is rendered — §10) + metric rows **44 px** high (label 13/ink-2 left, value 14/600/tabular right): Documents, Applications, Images & OCR, Other files (reference:72-74, 140) |
| Notice slot | Above the columns, only when real: paused indexing / index error / incomplete setup. One line + action, no banners with illustrations. |

**States**

- Loading: skeleton blocks at the real sizes (h1 line 44 px, lead 15/24, search 52 px, 3 recent rows 68 px, stat 36 px,
  4 metric rows 44 px); layout identical when data lands.
- Empty (fresh install, onboarding skipped): greeting + big search + quiet empty block "Nothing indexed yet." /
  "Add a folder to start building the index. It stays on this PC." / "Add location" (+ secondary "Resume setup").
  Recently accessed: "Nothing opened recently." / "Files and apps you open from the launcher will appear here." — no
  fake rows. Copy/behavior: §3.7.
- Incomplete setup: quiet notice "Setup is {n} of 7 steps done." + "Resume setup".
- Error (status unavailable): the Your index block shows the quiet error state "Couldn't read the index status." /
  "The local search engine didn't respond. Your settings are unaffected." / "Try again" + collapsed Details (§3.7);
  the rest of Home still works. Never a full-page error.
- Indexing: truthful state words and counts only; no percentage or progress bar in alpha (§10).
- Offline: local Home unchanged; only network-dependent blocks (agent provider, network roots) show the offline
  variant (§3.7). No updater exists in alpha (§10), so no updater-offline state.
- Dark: art-directed tokens; the stat and greeting keep the same hierarchy.

### 3.3 Search & indexes

**Question:** "What is indexed, how deep, and is it healthy?" h1 "Search & indexes"; lead "Choose what ProtonSearch
can find, without blocking the fast filename index." (reference:142)

Sections (32 px apart; each h2 is 12 px caps/750; label column **280 px** + 24 gap + panel at ≥1024, stacked below;
grid §3.0):

1. **LOCATIONS** — rows: root name (14/600) + path (12 px) · depth control (SegmentedControl: "Names only",
   "+ Content", "+ OCR") · status dot + word · item count (tabular) · overflow menu (Pause/Resume, Remove…).
   Buttons: "Add location…" (alpha: a paste-path form; the native folder picker is an IPC/Tauri gap — §10). Footer note: "Filename search works everywhere. Content and OCR
   run in bounded background jobs and never block the launcher."
   Empty: quiet empty block (SCREENS §3.7): "No locations yet." / "Add a folder so ProtonSearch can find your files." /
   "Add location".
   Remove confirm: "Remove {folder} from the index? Results from this folder stop appearing. Your files are not
   changed." Cancel / Remove.
2. **INDEX STATUS** — state line ("Indexing" / "Paused" / "Error"), "Last scan completed {localized relative time}",
   "Files 89,813", "Documents 14,609", "OCR documents 0". Alpha limitations (§10): the IPC exposes no percentage and
   no index-size field, so neither a progress bar nor "Index size" is rendered.
   Actions: "Pause indexing" / "Resume indexing"; "Rebuild index…" (confirm: "Rebuild the index? Search stays
   available while filenames are re-scanned. This can take a few minutes and uses more CPU." Cancel / Rebuild).
3. **EXCLUSIONS** — list of excluded patterns with remove buttons; "Add pattern…" text field + button; note "These
   patterns are skipped for content and OCR." Empty: "No exclusions. Everything in your locations is indexed."
4. **SOURCES** — per-source rows (Files, Documents, OCR, Browser history & bookmarks, Clipboard, Git, Apps & actions):
   status dot + word, last updated, detail ("Re-indexing…"; alpha: no percentage — §10), "Try again" on error; row-level
   copy per §3.7 (e.g. "Couldn't read Chrome history." / "ProtonSearch will try again automatically."). Empty: "No sources
   reporting." / "The local engine will list them when available."
5. **CLOUD & REMOVABLE** (only when the engine reports such roots) — rows: root path, status ("Unavailable — network
   location disconnected"), quiet row state (§3.7, no Retry); note "Placeholders are indexed by name. Content is read
   only when available."

**States:** loading skeletons; 0 locations; indexing/paused/rebuilding; per-root error; overall engine error as the quiet
state in §3.7; offline network roots as a row state ("Network location unavailable." / "Local files are still
searchable."), never a global error; dark tokens; changes autosave with a "Saved" stamp (2 s) next to the affected
section title.

### 3.4 Activity

**Question:** "What did I do recently, and is it private?" h1 "Your activity"; lead "A local history of recent
searches, launches, and optional workflows." (reference:143)

- Privacy strip: "Stored on this PC. Pause recording at any time." + Toggle "Pause recording". Alpha limitation (§10):
  there is no clear-activity command, so the clear control is omitted and the panel says clearing is not available
  in this build.
- Filters (SegmentedControl): "All" · "Searches" · "Launches"; filters apply instantly and are reflected in the URL
  query (deep-linkable state).
- List: date group labels "Today", "Yesterday" (12 px meta), then localized dates; rows: 32 px icon · title 14/600 ·
  sub 12 px
  "Search · 09:41" / "Launched · 09:38" / "Clipboard · 09:32" · trailing quiet arrow. Row is one button (opens the
  item or repeats the search where applicable).
- Empty: "No activity yet." / "Searches and launches will appear here while recording is on."
- Paused: notice "Activity recording is paused." + "Resume" text button; existing entries remain readable.
- Clear-history confirmation is **not rendered in v2.0 alpha** (no IPC command; §10). Re-add this confirm with the
  command.
- States: loading skeleton rows; read error as the quiet state "Couldn't load activity." / "Your history stays on this
  PC." / "Try again" (§3.7); filters with no matches → "No {filter} here." / "Try another filter or show everything." /
  "Show all"; dark. Keep the frame.

### 3.5 Workflows

**Question:** "What can run, and what is allowed to run?" h1 "Workflows"; lead "Native commands respond instantly.
The optional Pi agent runtime starts only when you start a workflow, then stops."

**AGENTS panel (top of the route; id `workflows-agents`).** Title "Agents" (12 caps/750); hint "Agent runtimes detected
on this PC. Optional; nothing runs until you start a workflow."; header action "Refresh" (13/600; "Checking…" disabled
while the probe runs). The panel lists only what the shell's `detect_agents` actually returned — **no invented rows** —
and only Pi is runnable in this alpha.

- **Detected row** (68 px, padding 12 0, 1 px `--ps-line` rule): 24 px icon slot with a 20 px runtime glyph · name
  **14/600** · version badge **18 px** (11/650 — the app type floor is 11 px, "v1.5.0") when real · availability badge
  **18 px** (11/650) · path row **12/18
  `--ps-ink-2`**, middle-ellipsis with the full path in `title` · detail **12/18 `--ps-ink-3`**.
- **Availability badges:** "Available" = neutral (surface-soft + ink-2), "Not found" = muted (ink-3), "Running" only when
  a run is active (8 px positive dot + word). Never a green "Ready" or an "AI" badge. Only badge text/colour + words.
- **Pi is primary:** 2 px inset `--ps-selected-bar` start bar + "Default" quiet tag. When not selected it offers a
  **"Use"** button (36 px, secondary); selecting shows an "In use" quiet tag instead of the button (no dead button).
  Other detected runtimes show "Detected · not wired in this build" in the detail line and **no control**; unavailable
  entries (if the shell returns one) show "Not found" + "Not found on PATH." Never render a fake run button.
- **Loading:** quiet line "Looking for agent runtimes…" with `aria-busy="true"` (no spinner, no skeleton). **Empty:**
  "No agent runtimes found." / "Install the Pi CLI, then refresh. Nothing is downloaded automatically." / **Refresh**
  action. **Load error:** quiet state "Couldn't look for agents." / "Nothing runs until an agent runtime is available." /
  "Try again" (§3.7 tones).
- **RECENT RUNS** (inside the Agents panel): h3 "Recent runs" (12 caps/750). Up to **4** rows visible (max **10** stored in
  UI-local metadata): row 36 px, 8 px status dot + word **13/600** — "Completed" positive, "Failed" negative,
  "Cancelled" ink-2 — and meta **12 `--ps-ink-2`**: "`{runtime}@ · {relative time} · {duration}@" with tabular
  numbers. No runs: "No runs yet." Helper: "Run history stays on this PC and keeps status, time and duration only —
  never prompts, tool output or paths." Never show prompts, tool output, provider text or paths in this list.

**RUN A PI WORKFLOW panel (id `pi-agent-panel`; the Agents "Use" action scrolls here).** Title "Run a Pi workflow";
hint "Pi starts only for a prompt you start here, then stops. Read-only in this alpha."

- Runtime metrics (44 px rows, label 13 ink-2 / value 14/600 right, tabular where numeric): Runtime ("Pi 1.5.0" when
  detected, else "Pi CLI"), Provider ("Pi default" or configured), Model ("Pi default" or configured), Idle process
  ("None" / "Running this workflow"). No fake "online"/"model ready" states.
- Workspace row: field label "Workspace"; path **12/18**, ellipsis, full path in `title` (default
  `%APPDATA%\protonsearch\v2\agent-workspace` shown as managed default); "Open in Explorer" secondary disabled until
  an explicit workspace is set; "Set workspace…" text button opens the 40 px field + "Save workspace"/"Cancel". Invalid:
  "Enter a full folder path, for example `C:\Users\You\AgentWorkspace`." The helper says the workspace is the default
  working folder, **not a sandbox**.
- Enable switch "Enable agent workflows" (44 px row); off → quiet notice "Agent workflows are off." /
  "ProtonSearch works fully without them. Turn the switch on to run a Pi workflow."
- Prompt: textarea rows=3, maxlength 2000, disabled when off/not detected/running; label 11 caps; placeholder "Describe
  the workflow you want, for example: list the TODOs in this workspace and propose a plan."
- Safety lines (12/18 ink-2, always visible, three sentences): Pi runs with your Windows permissions; read tools can open
  any file you can read; the workspace is not a sandbox. There is no per-tool approval; file edits and shell access are
  disabled in this alpha. When a run starts, the prompt, context and tool output are sent to the configured provider.
- Actions: **Run workflow** (primary, 36 px; disabled with a visible reason when off, not detected, prompt empty, or a run
  is active), **Cancel workflow** while active, "Try again" after finished/cancelled/failed. Elapsed line right:
  "Running · 3.2 s" / "Last run · 4.2 s" (tabular; 0.1 s below a minute, `m ss` above).
- **Run summary** (below actions; hidden until the first run, shown for running/finished/failed/cancelled): three 44 px
  metric rows — **Status** (8 px dot + "Running"/"Completed"/"Failed"/"Cancelled"; completed positive, failed negative,
  cancelled ink-2), **Started** (localized `Today, 14:32` — `Intl.DateTimeFormat` with date when not today), **Duration**
  (tabular "4.2 s"). No runs yet → helper "No runs yet. Start a workflow above." Never invent a prior run. For a finished
  run the elapsed line and the Duration row must show the **same** value (prefer the run's reported `durationMs` when
  the terminal event supplies it; the screenshot capture caught a mismatch when wall-clock elapsed was used).
- Result block: final answer 14/21 max 60 ch width, selectable. Activity log: h3 "Workflow activity"; 32 px rows, 14 px
  icon, label 12/600, text 12/18; max 200 entries; `aria-live="polite"`; never raw provider text, tool stderr, paths or
  codes — product copy plus a sanitized `"Reference: {code}"` inside Details only.
- **Quiet errors** (§3.7): probe failure `warn` "Pi isn't detected." / "Install the Pi CLI, then refresh. Nothing is
  downloaded automatically." / "Refresh"; start failure `error` "Agent workflows are unavailable." / "Search still works.
  The optional runtime didn't start." / "Try again" + Details; run failure `error` "Pi couldn't finish this workflow." /
  "Search still works. Nothing outside the workspace was changed." / "Try again" + sanitized Details; cancellation is
  neutral "Workflow cancelled." / "Nothing outside the workspace was changed." — never an error tone. No process may idle
  after a terminal state; cancel/stop is best effort and the panel reports only what the shell confirms.

- **NATIVE COMMANDS** panel: rows "System actions", "Windows tools", "Index control" with one-line descriptions and
  "Confirmation required" badges; actions execute in the launcher, not here. No fake buttons.
- **PERMISSIONS** panel: "Files & folders — read only", "Clipboard", "Shell commands", "Network" with state
  "Ask every time" (default); note "Writes, deletes, shell commands, and network access always ask first." Link
  "Review approvals" only once an approvals log exists; otherwise omit the link rather than ship a dead control.
- **QUICKLINKS & SNIPPETS** — alpha limitation (§10): no bridge command to list/edit; read-only migration note only.
- **States:** loading, no agents, agents detected, off, running, completed, failed, cancelled, offline (provider
  unreachable post-alpha: "Can't reach the provider." / "Check your connection, then try again. Local search still
  works." / "Try again"), dark.

### 3.6 Preferences

h1 "Make it yours"; lead "Customize behavior, appearance and privacy independently of search performance."
(reference:145)

Sections (label column + panel; one save model: **autosave with a "Saved" stamp**, no global Save; destructive
actions confirm):

1. **LAUNCHER SHORTCUT** — "Current shortcut" + kbd "Alt + Space" + "Change…" (capture dialog: "Press the shortcut
   you want." helper "Use at least one of Ctrl, Alt, Shift, or Win."; Esc cancels; invalid/conflict inline state per §3.7
   ("That shortcut isn't available." / "Another app is already using it. Choose a different combination.");
   "Reset to Alt + Space"). Saves immediately.
2. **APPEARANCE** — "Theme" segmented System / Light / Dark; "Density" segmented Comfortable / Compact; "Reduced
   motion" line "Follows your Windows animation setting."; live preview panel showing a mini launcher row in the
   selected theme/density (the reference's settings pages show values; the preview shows the result).
3. **PRIVACY & HISTORY** — toggles "Record activity" ("Search and launch history on this PC"), "Clipboard history";
   text buttons "Clear activity…" and "Clear clipboard history…" (confirm with counts); note "No telemetry. Nothing
   leaves your PC unless you opt into an agent provider."
4. **INDEXING** — toggles "Background content indexing", "Pause indexing on battery"; link "Index health" → Search &
   indexes; link "Rebuild index…".
5. **AGENT & MODELS** — toggle "Enable agent workflows" (default off); provider/model shown read-only. Alpha
   limitation (§10): no model-cache command exists, so no cache size or "Manage…" control; note "No model is downloaded
   without asking."
6. **UPDATES & DIAGNOSTICS** — version/protocol/schema/platform rows plus a "Refresh diagnostics" action (read-only).
   Alpha limitation (§10): no updater, log-folder, or diagnostics-export commands, so no update check, logs or export
   controls are rendered.
7. **ABOUT** — version, "Local-first. No account." and a licenses link.

**States:** loading skeleton; settings read failure as the quiet state "Couldn't load settings." / "Defaults are shown.
Changes can't be saved until settings load." / "Try again" (§3.7); save failure "Couldn't save that change." / "Your
previous setting is still active." / "Try again"; invalid/conflicting hotkey inline with `role="alert"`; dark; disabled
toggles (e.g. OCR off when no model) stay readable and explain why in 12 px helper text. No updater-offline state exists
in alpha (§10).

---

### 3.7 State presentation — error / empty / offline (FINAL 2026-11-09)

Quiet, professional, local-first. A state shows **one human sentence first**; technical detail is collapsed and never
leaks raw exceptions, codes, stack traces or file paths onto the surface. Local search never reports "offline"; offline
is only for genuinely network-dependent content. Component anatomy and tokens: DESIGN_SYSTEM §8.3.

**Components (class guidance for ui-app)**
- `.state` base: surface background, 1 px `--ps-line` border, radius 12, padding 20/24, 20 px outline icon, title
  15/600 ink, body 14/21 `--ps-ink-2` max 60 ch, actions row 16 px above, optional details 12 px below.
- `.state--warn` (warning icon + `--ps-warning-pill-border`), `.state--error` (negative icon +
  `--ps-negative-pill-bg` border, **no red fill**), `.state--offline` (cloud-off icon, neutral border),
  `.state--empty` (neutral, optional icon, one action).
- `.row-state` for one **auto-retry / reconnectable** row failure (source read, network root): 16 px **warning** icon +
  13/20 ink-2 sentence + "Try again" 13/600 accent; no border or background. Blocking row failures use `.state--error`
  with the negative colour instead (severity rule: recovering = warn, blocked = error).
- `.state-details`: native `<details>` with summary "Details" (12/600 ink-2); content 12/18 ink-2 = sanitized
  message + reference id only. Omit when there is nothing safe to show.
- Existing primitives map: `Callout` → `.state` (tone attr), `EmptyState` → `.state--empty`; add
  `OfflineNotice` = `.state--offline`; add `DetailsDisclosure`.

**Severity rules.** At most one negative block per view; a degraded feature uses `warn`, never `error`. Empty is a
neutral state, not an error. Offline is neutral (no red). Informational alpha limitations are neutral, not warnings.
Never place a state block as a full-screen banner; it sits in the block that failed, aligned to the grid. No illustrations
or centred copy in the app.

**Copy rules.** Title: sentence case, ≤6 words, ends with a period, names the object ("Couldn't load activity.").
Body: ≤2 sentences, active voice, second person; say what happened and what happens/works now; then one specific action
("Try again", "Open Search & indexes", "Add location", "Show all", "Open settings"). No "Oops", "Error", "Something
went wrong", "Please try again later", codes, paths or exclamation marks. Numbers/dates via `Intl`. The action is
optional only when the state will resolve itself; otherwise every error/empty state has exactly one next step.

**Error → copy map (canonical)**

| Path / trigger | Title | Body (what works) | Action | Announce |
|---|---|---|---|---|
| Engine status load fails (Home, Indexes, Preferences) | "Couldn't read the index status." | "The local search engine didn't respond. Your settings are unaffected." | Try again | status |
| `EngineStatus.warnings` non-empty | "Index needs attention." | "Some items couldn't be indexed. Search still works with what is available." | Open Search & indexes | status |
| Indexer state `error` | "Indexing stopped." | "New files aren't being added. Existing search still works." | Open Search & indexes | alert |
| Indexer paused (Home / Indexes) | "Indexing is paused." | "New files aren't being added. Existing search still works." | Resume indexing (Indexes) · Open Search & indexes (Home) | status |
| Settings load fails | "Couldn't load settings." | "Defaults are shown. Changes can't be saved until settings load." | Try again | status |
| Settings save fails (any toggle/field) | "Couldn't save that change." | "Your previous setting is still active." | Try again | alert |
| Hotkey invalid format | "That shortcut isn't valid." | "Use at least one of Ctrl, Alt, Shift, or Win." | (edit field) | alert |
| Hotkey unavailable | "That shortcut isn't available." | "Another app is already using it. Choose a different combination." | (edit field) | alert |
| Activity load fails | "Couldn't load activity." | "Your history stays on this PC." | Try again | status |
| Activity row open fails | "Couldn't open “{name}”." | "It may have been moved or deleted." | Remove from history? no — none | alert |
| Agent runtime fails to start | "Agent workflows are unavailable." | "Search still works. The optional runtime didn't start." | Try again | alert |
| Agent not set up (preference enabled, no runtime in this build) | "Agent workflows aren't set up." | "Your preference is saved, but no runtime is installed in this build. Nothing was downloaded or started." | none | status |
| Agent provider unreachable / offline (post-alpha: no provider can be configured in this build) | "Can't reach the provider." | "Check your connection, then try again. Local search still works." | Try again | status |
| Source read fails (browser/clipboard/git) | "Couldn't read {source}." | "ProtonSearch will try again automatically." | Try again | status (row) |
| Network location unavailable | "Network location unavailable." | "Local files are still searchable. Reconnect the drive to index it again." | Try again (post-alpha; alpha shows the reconnect copy only) | status |
| Onboarding finish fails | "Couldn't finish setup." | "Your choices are saved on this PC." | Try again | alert |
| Launcher failure states | launcher final spec (§2.3) unchanged | — | — | — |

**Empty → copy map (canonical)**

| Surface | Title | Body | Action |
|---|---|---|---|
| Home — no index | "Nothing indexed yet." | "Add a folder to start building the index. It stays on this PC." | Add location |
| Home — no recents | "Nothing opened recently." | "Files and apps you open from the launcher will appear here." | Open launcher (secondary) |
| Search & indexes — no locations | "No locations yet." | "Add a folder so ProtonSearch can find your files." | Add location |
| Search & indexes — no exclusions | "No exclusions." | "Everything in your locations is indexed." | none |
| Search & indexes — no sources | "No sources reporting." | "The local engine will list them when available." | none |
| Search & indexes — no errors | "No errors." | "The index is running normally." | none |
| Activity — empty | "No activity yet." | "Searches and launches will appear here while recording is on." | none |
| Activity — filtered empty | "No {filter} here." | "Try another filter or show everything." | Show all |
| Workflows — agent off | "Agent workflows are off." | "ProtonSearch works fully without them." | none |
| Onboarding — no locations | "No locations selected." | "Go back to add at least one folder." | Back to locations |

**Offline rules.** Local search, indexing, history and settings never show offline. Only: network roots (row status
"Unavailable — network location disconnected" + body "Local files are still searchable."), the agent provider
("Can't reach the provider."), and (post-alpha) the updater. Do not add a global offline banner; the affected block or
row carries the state. Offline is `role="status"`, recovers silently when the network returns if the engine reports it;
otherwise offer "Try again". Never claim local search is affected.

**Details disclosure.** `<details class="state-details"><summary>Details</summary>…`; content is a sanitized engine
message (no raw `err.message` unless already humanised) plus a short reference id; never file paths, stack traces,
HTTP bodies or settings values. If nothing safe exists, render no disclosure. "Open logs folder" is post-alpha (no IPC
command in this build).

**aria.** Load-time failures: `role="status"` and announce once; action-triggered failures: `role="alert"` once;
offline transitions: `role="status"`; do not re-announce on re-render; "Try again" keeps focus on itself; after success the
block disappears and the updated region is not announced twice. Details uses native disclosure semantics.

**Do / don't.** Do: "Couldn't load activity." + "Try again." Don't: `Error: ENOENT …`, "Something went wrong.",
"Please try again later.", a red panel across the view, a code/path in the title, or a "Try again" that does nothing.
Current raw renders to replace: `IndexesView.tsx:275`, `HomeView.tsx:122`, `ActivityView.tsx:155,204`,
`WorkflowsView.tsx:151`, `PreferencesView.tsx:132`.


## 4. Onboarding (first run)

### 4.1 Rules

- Seven steps; **skippable** and **resumable**. State persists after every step (`OnboardingState`); closing the
  window never loses progress. Existing users (legacy settings/index detected) are marked complete and **never see
  onboarding** (master prompt §V.C).
- Rail is position, not percent: numbers 1–7 with labels; current step filled `#295a70` white; completed steps accent
  ink; future steps muted. No fake progress bar.
- Typography: eyebrow "Getting started / 0{n} of 07"; h1 **34/600/1.16**; lead **15/24**, max **480 px**; content body
  **14/21**; footer: left
  "Back" ghost + "Skip setup" text link, right primary. Rail hidden ≤700 (final 2026-11-09; code currently 760 —
  follow this doc); eyebrow then carries the position.
- Primary labels: step 1 "Get started", steps 2–6 "Continue", step 7 "Start using ProtonSearch". Back is hidden on
  step 1. "Skip setup" is available on every step.
- Focus moves to the step h1 on change; Enter activates the primary; Esc is a no-op (prevents accidental loss);
  closing the window is safe.
- No account, no cloud sign-in, no silent downloads, no name inference (master prompt §V.C).

### 4.2 Step 01 — Welcome & privacy

- h1 "Find anything, without the friction."
- lead "A fast, private Windows search experience built around your work. We only need a few preferences to get
  started." (reference:148)
- Content: three rows (label **14/600** + one **12 px** line):
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
  (secondary; alpha: paste-path form, the native picker is an IPC gap — §10; added roots appear in the list).
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
  list, the trade-off text for the highest selected mode (12 px, changing with selection):
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
- Content: checkbox row "Set up agent workflows later" · description "The Pi runtime stays off until you run one.";
  note "ProtonSearch never downloads Pi, a model or a provider key. Install the Pi CLI yourself later if you want agent
  workflows."; disclosure "What leaves this PC?" expanding to truthful copy: "Nothing leaves this PC unless you install Pi,
  configure a provider and run an agent workflow. Then the prompt, context and tool output are sent to that provider."
- Default unchecked.
- Back / Continue / Skip setup.

### 4.8 Step 07 — Ready

- h1 "Ready when you are."
- lead "Filename search is available now. Content and OCR will fill in the background; you can change everything
  later." (reference:148)
- Summary panel (metric rows): "Included locations" {n}; "Filename search" "Always available"; "Document content"
  "Enabled"/"Off"; "OCR search" "Enabled"/"Off"; "Agent workflows" "Set up later"/"Off"; "Shortcut" "Alt + Space".
- Note 12 px: "Indexing starts after you continue. You can keep using search while it runs."
- Primary "Start using ProtonSearch" → app Home (focus h1). Secondary "Finish setup later" → app Home with the
  incomplete-setup banner; indexing still starts in the background if locations were chosen.
- No fake percentage on this step; in alpha Home shows state words and counts only (no progress bar; §10).

### 4.9 Resumable / skippable / existing users

- Every step transition persists `OnboardingState` (step index + values). Closing/reopening resumes at the saved
  step with the eyebrow showing "Getting started / 03 of 07" and a quiet 12 px line "Setup saved. You can close this
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
| indexing | normal + footer state | "Indexing" (no percentage in alpha — §10) | truthful |
| paused | normal + footer | "Index paused" | — |
| dark | all above | — | on-dark mark |
| high contrast | system colours | — | §12.4 DESIGN_SYSTEM |
| 125/150/200 % DPI | clamps to work area | — | §6 |

### 5.2 App + onboarding

| Screen | Loading | Empty | Error | Offline | Other |
|---|---|---|---|---|---|
| Home | skeleton | no index / no recents | quiet error state (§3.7) | n/a | indexing, incomplete setup |
| Search & indexes | skeleton | no locations | quiet error state / row errors + Try again (§3.7) | network root row state | indexing, paused, rebuild |
| Activity | skeleton rows | no activity | quiet error state (§3.7) | n/a | recording paused, filtered empty |
| Workflows | agents checking; first run | no agents detected; no runs yet | quiet error state (§3.7) | provider unreachable (offline variant) | agents detected/selected, off, running, completed, failed, cancelled, read-only alpha |
| Preferences | skeleton | n/a | quiet error state (§3.7) | n/a (no updater in alpha) | hotkey conflict, disabled reasons |
| Onboarding | n/a | no locations (step 4) | hotkey conflict (inline, §3.7) | n/a | resumed, skipped, complete |

All app error/empty/offline presentation follows §3.7; the app action label is "Try again" (the launcher keeps its final
"Retry"). No raw exception text, codes, paths or stack traces are ever shown on the surface.

---

## 6. Responsive and DPI

### 6.1 Breakpoints

| Width band | Launcher | Desktop app | Onboarding |
|---|---|---|---|
| ≥1100 (1000–1280 measured) | 820 wide; side preview only if ≥1200 | sidebar 232; margin 40; 12-col/24; Home 7/5 | rail 240; content 40/42 |
| 820–1099 | 820 wide; preview bottom drawer | sidebar 200; margin 32; 12-col/24; Home 7/5 | rail 240; content 32/32 |
| 701–819 | `min(820, wa−32)`; drawer | sidebar 64 rail; margin 24; gutter 16; 1-col | rail 240; content 32/32 |
| 560–700 (**700 breakpoint**) | `min(820, wa−32)`, search 61 px, input 16 px, row meta hidden, logo 26, footer may wrap | rail 64; margin 24; single column; settings stack; recent meta hides | rail hidden; content 24/22; eyebrow "Step n of 7"; footer stacks if needed |
| 560 (test width) | min supported; hints hide | min supported | min supported |

Final window contract (2026-11-09): **main app window minimum 560×480** (onboarding uses that same window). The
onboarding rail hides at **≤700**. Launcher geometry: card 820 (preview 1180) + **28 px transparent shadow pad on every
side** (window = card + 56); side preview panel 360 px, shown when `workArea.width ≥ 1200`. The pad is part of the
window: a press in it dismisses the launcher.

At ≤700 exactly (reference:94-100): launcher search row 61 px, input 16 px, trailing meta hidden, results max 350,
logo 26; app sidebar becomes **64 px** with hidden labels (keep them sr-only), margins 24, gutter 16, columns 1,
onboarding rail hidden, setup content padding 24/22, and the setup scrolls inside the 560×480 minimum window. Type does
not shrink for narrow windows: wrap or stack instead.

### 6.2 DPI (125 / 150 / 200 %)

- CSS px stay constant; physical size multiplies. Launcher card/height clamp to the work area:
  `min(820, workAreaCss.width − 80)`, card height `H ≤ workAreaCss.height − 80`; window = card + 56 each axis;
  results max = `H − 156`. The 28 px pad and every shadow layer are CSS px, so they scale together and stay inside the
  window at 125/150/200 % (verify no straight cut-off line).
- At 200 % on a 1366×768 display the work area is ~640×350 CSS px: launcher = 608×302, search row 72, footer 44,
  results 186 px (~3 rows), preview becomes a drawer with 140 px content height. Nothing overlaps; hints hide.
- Icons are requested at 32/48/64/96 px source for 100/125/150/200 % (never upscale); hairlines stay 1 CSS px
  (1.25/1.5/2 device px, rendered crisply by the compositor); text remains vector.
- Test list: 560/820/1000/1280 × 125/150/200 % × light/dark × 1.0 text size; plus Windows text size 125 % if the
  WebView honours it — record actual result (do not assume).
- App type: body 14/21, meta 12/18, caps 11/16 at 100 %; at 125/150/200 % these render 17.5/21/28, 15/18/24 and
  13.75/16.5/22 px physically. At 200 % on a small screen allow vertical scrolling; never reduce type below the 11 px
  caps or hide content to fit. Verify Home/Indexes/Preferences/onboarding at 560×480 + 200 %.
- High contrast and reduced motion are independent switches; test with Black + Aquatic HC themes and reduced motion
  on.

---

## 7. Motion per surface (summary; tokens in DESIGN_SYSTEM §6)

| Surface | Motion | Reduced motion |
|---|---|---|
| Launcher open | card fade+8 px rise+scale .985→1, 150 ms ease-out-expo; input/typing instant | instant show, no transform |
| Launcher close | card fade+4 px rise+scale 1→.99, 110 ms ease-launcher-out; action runs immediately | instant hide |
| App window reveal | shell fade+4 px, 160 ms ease-out; route changes instant | 0 ms |
| Filter chip | colour 110 ms; search dispatches immediately | 0 ms |
| Result selection / hover | colour 110 ms | 0 ms |
| Footer/status text | none (text swap) | same |
| Preview | content fade 160 ms + 6 px | none |
| Context menu | fade 90 ms + scale 0.98→1 120 ms | none |
| Route change (app) | none | same |
| Onboarding steps | content fade 160 ms + 4 px | none |
| Progress | 240 ms interpolation; indeterminate 1200 ms loop | instant / static bar |
| Dialogs | scrim fade 120 ms + dialog scale 0.98→1 160 ms | none |

Nothing animates on typing, Enter, Esc, arrow-key selection or window resize beyond the colour cross-fade and the
150/110 ms card open/close. No layout animation and no shadow animation anywhere. Reduced motion removes the card
transform and all durations.

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
9. **Recovery:** mock index error → Home shows the quiet error state → Enter on "Try again" → status returns; launcher
   still works.
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
| Index status | "Index ready" · "Indexing" · "Index paused" · "Index unavailable" (no percentage in alpha — §10) |
| Footer link | "Open ProtonSearch" |
| Preview unavailable | "No preview for this item." · "Preview is unavailable right now." · Retry |
| Preview truncated | "Showing the first {n} characters. Press Enter to open the full item." |
| Quick destination — Bookmarks | "Browser Bookmarks" · "Search saved bookmarks" / "Off — enable in Preferences" |
| Quick destination — History | "Browser History" · "{n} items" / "No items indexed yet" / "Off — enable in Preferences" |
| Quick destination — Clipboard | "Clipboard History" · real count / off copy |
| Quick destination — Agents | "Agents" · "Agent runtime running"/"stopped"/"needs attention"/"No agent configured" |
| Quick destinations — secondary | "Git history" · "Agent History" · "Open Workflows history" |
| Quick destinations syntax | "Prefixes: bookmarks:, history:, clipboard:, git:, ocr:, content:, calc:" |
| Home no index | "Nothing indexed yet." · "Add a folder to start building the index. It stays on this PC." · "Add location" |
| Home incomplete | "Setup is {n} of 7 steps done." · "Resume setup" |
| Search & indexes no roots | "No locations yet." · "Add a folder so ProtonSearch can find your files." · "Add location" |
| Remove location confirm | "Remove {folder} from the index? Results from this folder stop appearing. Your files are not changed." |
| Rebuild confirm | "Rebuild the index? Search stays available while filenames are re-scanned. This can take a few minutes and uses more CPU." |
| Activity empty | "No activity yet." · "Searches and launches will appear here while recording is on." |
| Activity paused | "Activity recording is paused." · "Resume" |
| Clear activity confirm (post-alpha) | "Clear all activity? This removes {n} recorded items from this PC. It can't be undone." |
| Workflows agent off | "Agent workflows are off." · "ProtonSearch works fully without them." |
| Agents empty | "No agent runtimes found." · "Install the Pi CLI, then refresh. Nothing is downloaded automatically." · "Refresh" |
| Agents load error | "Couldn't look for agents." · "Nothing runs until an agent runtime is available." · "Try again" |
| Agent recent runs empty | "No runs yet." |
| Agent run failure | "Pi couldn't finish this workflow." · "Search still works. Nothing outside the workspace was changed." · "Try again" |
| Clipboard history off | "Clipboard history is off." · "Turn it on in ProtonSearch → Preferences to keep copied text on this PC." |
| Clipboard no history | "No clipboard history yet." · "Copied text and images will appear here from now on. Only copies made while ProtonSearch is running are captured." |
| Clipboard 0 matches | "No matches in Clipboard." · "Try different words, or press Esc to clear the filter." |
| Workflows runtime error | "Agent workflows are unavailable." · "Search still works. The optional runtime didn't start." · "Try again" |
| Preferences save | "Saved" (2 s stamp) |
| Hotkey conflict | "That shortcut isn't available." · "Another app is already using it. Choose a different combination." |
| Updates offline (post-alpha) | "Can't check for updates right now — you're offline." |
| Onboarding saved | "Setup saved. You can close this window and continue later." |
| Onboarding skip | "Finish setting up ProtonSearch" |

App error/empty/offline copy is canonical in §3.7 (every path mapped). The rows above cover launcher/onboarding/confirm
copy and key app strings. Rows marked (post-alpha) are specified but not rendered in v2.0 alpha; the alpha UI says
plainly that the capability is not available in this build (§10). App action label is "Try again"; launcher keeps
"Retry" (final).

## 10. Alpha scope: CUTs and IPC limitations (v2.0 alpha)

Truthful-UI rule (master prompt hard rule 5): when a capability is CUT or absent from the frozen IPC contract, the UI
either omits the control or states plainly that it is not available in this build. It never ships a dead control, a
fake percentage, sample data, or a teaser.

### 10.1 CUT for v2.0 alpha (deferred, not broken)

| CUT | Why | Post-alpha re-entry |
|---|---|---|
| Clipboard multi-select + Paste-N | Needs a selection model over the active-row model, a paste-aggregation path and IPC action support the alpha contract does not carry; a stub would be a dead affordance. | Shift+Enter toggle, Shift+↑↓ extend, Ctrl+A all, Enter paste N, `aria-multiselectable`, footer count, tests. |
| Exact-command ACTIONS pin | Makes group order non-deterministic while tiers land (groups can reorder mid-query) and adds engine coupling; the fixed eight-group order is simpler to scan and test. Calculator/command rows still appear — ACTIONS simply renders last. | Only with a real relevance finding; would need an explicit `isExact` signal in `SearchResult`. |

### 10.2 IPC capability gaps (alpha limitations)

| Gap | Affected surface | Alpha behaviour (truthful UI) |
|---|---|---|
| No progress percentage/phase in `EngineStatus` | Launcher footer, Home index block, Search & indexes status, onboarding finish | State words + real counts only; no progress bar and no "· 42 %" anywhere. |
| No clear-activity command | Activity | Clear control omitted; the panel states clearing is not available in this build. |
| No quicklinks/snippets list or CRUD command | Workflows (launcher quicklink results only if the engine supplies them) | Read-only migration note; no editor, no dead button. |
| No native folder-picker command | Search & indexes locations, onboarding locations | Validated paste-path form; helper states the picker is not available in this build. |
| No updater / logs / diagnostics-export commands | Preferences -> Updates & diagnostics | Read-only version/protocol rows + Refresh diagnostics; no update check, logs or export controls. |
| Clipboard images are stored but not exposed as search rows (`ClipboardEntry` has no image/OCR fields; search excludes `is_image`) | Launcher Clipboard sectioned results | The IMAGES section is omitted entirely while no image rows exist; never render a fake/empty IMAGES section or a category control. Text sections and capture copy are unaffected. |
| No Windows clipboard-history import/backfill (capture is live-only) | Launcher Clipboard empty states, Preferences clipboard toggle | Copy states capture starts now; never promise or imply pre-install history. Preference helper says previous Windows clipboard history is not imported. |
| `detect_agents` may return CLIs that are not wired to a run panel (Codex/OpenCode/Cline/Antigravity) | Workflows Agents list | Show them as detected with "Detected · not wired in this build" and no Run/Use control; only Pi is runnable. Never invent rows. |

### 10.3 UI-level alpha limitations

Hotkey capture/conflict UX is still a code item tracked by REVIEW_1 M2; the contract in §3.6/§4.6 stands and must not
be quietly downgraded. Clipboard capture starts when clipboard history is enabled (no OS backfill); agent run history is
UI-local metadata only (max 10 records, status/time/duration, never prompts, paths or tool output). Anything else added to
the alpha must be listed here before sign-off.

---

## 11. Acceptance screenshots (must exist before G4 sign-off)

Launcher: first summon, empty input, 0 matches, 1 match, many+selected, preview (1180 card / 360 side panel; and bottom
drawer below 1200 work width), loading, index error, indexing/paused, dark, 560/820/1000/1280, 125/150/200 % DPI, high
contrast, keyboard focus. **Shadow proof:** light/dark cards on a light and a dark wallpaper at 100 % and 150 % (no
straight cut-off edge on any side; corner radius reads). **Motion proof:** open and close captured mid-animation (or a
short clip) plus reduced-motion instant. **Filter strip:** all four chips ALL/FILES/FOLDERS/APPS in final order with Ctrl+1–4, selected state, keyboard focus, hover, both themes,
FILES/FOLDERS exact rows (no mixed types), scoped 0-match copy, overflow at 560/820 with the selected chip scrolled into
view, and a long localised label (German) proving no truncation. **Quick destinations:** empty query, all six real destinations (counts/off/agent states), 2×2 grid + secondary pills, syntax hint, keyboard traversal and prefix insertion, both themes, 560/820 widths. **Preview:** real payload states (text with highlight, image, OCR, truncated, metadata, unsupported, error+Retry, loading skeleton), side 360 and drawer, keyboard, both themes. **Clipboard sections:** only under CLIPBOARD, grouped labels in fixed order (LINKS/EMAILS/PATHS/CODE/TEXT; IMAGES absent in
alpha), no chip row/tags/tabs, section label metrics and 6 px gap, flattened ↑/↓ across section boundaries, empty
history/0-match states, standard card height (no +36 px), 560/820 and 125/150/200 % proof, both themes. **Agents:** detected list with
available/unavailable + real path/version, no fake rows, Pi Use→In use and panel focus, empty/loading/load-error, recent
runs (empty + completed/failed/cancelled), run summary status/started/duration while running and after finish, quiet
errors; both themes and 560 width. App: Home (loaded, loading, empty, error — no progress bar), Search & indexes
(locations incl. paste-path form, depth, health, rebuild confirm, alpha-limit copy), Activity (list, empty, paused —
no clear control), Workflows (off/on, alpha-limit copy, error), Preferences (theme/density, hotkey field/validation,
alpha-limit diagnostics). Onboarding: all 7 steps, resumed state, 700 boundary (rail shown at 701, hidden at 700),
dark. **Type/alignment proof:** route screenshots at 1280/1000/560 with the 12-column grid overlay; body 14/meta 12 at
100/125/150/200 %; no 10 px text in the app; settings label column 280 + panel; Home 7/5; numeric columns right
aligned. **State proof:** one capture per §3.7 path (engine down, index warning, index error, settings load/save failure,
activity load failure, agent unavailable, provider offline, network root, empty Home/indexes/activity/filtered) showing
human copy, collapsed Details and no raw exception text. Alpha-limitations sheet: one capture per §10 item proving no
dead control or fake percentage is rendered. Reference comparison sheet: reference HTML vs production for launcher, Home, onboarding, light/dark (master prompt
§XI.3).
