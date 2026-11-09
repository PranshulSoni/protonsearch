# VISUAL_AUDIT_LAUNCHER — ProtonSearch v2 Quick Launcher

**Owner:** ui-launcher (L2-B1). **Date:** 2026-11-09, capture window 17:38–17:47 +02:00.
**Scope:** `apps/desktop/src/launcher/**` rendered in (a) the real Tauri/WebView2 app with live engine data and
(b) the real React UI in Chromium against the browser-mock bridge. **Contracts:** `docs/design/SCREENS.md` §2,
`docs/design/KEYBOARD_AND_A11Y.md`, `docs/design/DESIGN_SYSTEM.md`, `docs/security/V2_SECURITY_REVIEW.md` V2-04.

This audit is launcher-only. The desktop app/onboarding audit lives in `docs/qa/visual/VISUAL_AUDIT_APP.md`.

---

## 1. Environment and artifacts

| Item | Value |
|---|---|
| Host | Windows 11, WebView2 (real app), AMD Ryzen AI 9, 31 GB |
| Real app build | `cargo build -p protonsearch-v2 --bin ProtonSearch` (debug), binary 17:43:57; frontend `npm run build` |
| Real data dir | `.devdata` — migrated index, **131,199 filename entries**, engine reported "Indexing…" during captures |
| Real DPI | host display at **150 %** (physical PNG = CSS × 1.5: 1230×384 = 820×256 CSS) |
| Real capture driver | CDP attach (Playwright `connectOverCDP`), Tauri `show_launcher`, strict waits on `#ps-results[aria-busy]` + input value + expected count + window resize, then `page.screenshot` |
| Browser mock | `vite preview` of the production build + Playwright Chromium; mock bridge clearly identified below |
| Real-state report | `docs/qa/visual/app-captures/capture-run-report.json` (states, window sizes, supplementary latency) |
| Build/test evidence | `npm run build` exit 0 (tsc --noEmit + vite); `npm test` **9 files / 143 tests passed** (launcher: 52 tests) |
| Harness note | `e2e-drive.mjs` was run per instructions; its `latency-real-app.json` undercounts later queries (see §7) |

---

## 2. Real-app state matrix (live engine, 150 % DPI)

| # | State | File (`docs/qa/visual/app-captures/`) | Window (CSS) | Observed |
|---|---|---|---:|---|
| 1 | Empty input, no recents | `real-launcher-empty.png` | 820×256 | placeholder rows, exact "Start typing…" copy, **no scrollbar**, count blank, footer "Indexing… · 1,31,199 items" |
| 2 | Empty input, recents/query results (readme) | `real-launcher-recent-exact.png` | 820×480 | 50 matches, grouped rows, count "50 matches" |
| 3 | Many results + selected row | `real-launcher-architecture.png` | 820×480 | 50 matches; FILES group; selected fill + 2 px inset bar; MD/JS badges; meta "18 Oct" |
| 4 | Typo query | `real-launcher-typo.png` | 820×480 | 19 matches, "protonserch" |
| 5 | 0 matches | `real-launcher-zero.png` | 820×256 | exact copy "No matches for “…”." + prefix hint; count "No matches"; footer hints = Esc Clear only |
| 6 | 1 match | `real-launcher-one-match.png` | 820×256 | "1 match"; JSO badge for .json; selected row; no scrollbar |
| 7 | Long list / scroll | `real-launcher-long-list.png` | 820×480 | 50 matches; APPLICATIONS group first (AN/AI/AT/AS monogram badges), FILES below; thin scrollbar |
| 8 | Preview side panel | `real-launcher-preview.png` | **1180×480** | side panel 360 px; title/kind/path/size/"Modified 18 Oct"; honest "Preview not available — press Enter to open."; action row Open / Reveal in Explorer / Copy path; hints "Ctrl+Space Close preview · Esc Close" |
| 9 | Context menu | `real-launcher-context-menu.png` | 820×480 | anchored at the **selected row edge**; Open (focused ring), Reveal in Explorer, Copy path (Ctrl+C) |
| 10 | Destructive native command confirmation (V2-04) | `real-launcher-confirm.png` | 820×278 | ACTIONS group "Empty Recycle Bin / Windows action - requires confirmation"; inline bar: "Confirmation required for EmptyRecycleBin. Ask again to execute." + **Cancel (default focus, visible ring)** + "Run action"; selection preserved |

The real destructive flow was exercised safely: Enter produced the server-side challenge, **nothing executed**;
Esc cancelled, window returned to 820×256, focus returned to the input, selection and query preserved.
(`engine.rs` `execute_action` returns `ok:false` and records a 15 s pending request before any native call.)

---

## 3. Browser-mock state matrix (real React UI, mock bridge)

| State | File (`docs/qa/visual/screenshots/`) | Viewport / scale |
|---|---|---|
| Results light / selected / empty / zero / loading (3 flat skeletons) | `launcher-results-light`, `launcher-selected`, `launcher-empty`, `launcher-zero`, `launcher-loading` | 860×640, 1× |
| Dark theme | `launcher-results-dark` | 860×640, 1×, dark |
| Long list / 1 result | `launcher-long-list`, `launcher-one-result` | 860×640, 1× |
| Context menu (keyboard) | `launcher-context-menu` | 860×640, 1× |
| Preview side light/dark (work area 1400 ≥ 1200) | `launcher-preview-side`, `launcher-preview-side-dark` | 1280×800, screen 1400×900 |
| Preview bottom drawer (work area 1000 < 1200) | `launcher-preview-drawer` | 860×640, screen 1000×700 |
| Narrow surface 560 px (61 px search row, 26 px mark, hidden meta/hints) | `launcher-560-empty`, `launcher-560-results`, `launcher-560-dark` | 560×640, 1× |
| DPI 125 / 150 / 200 % | `launcher-dpi1-25-results`, `launcher-dpi1-5-results`, `launcher-dpi2-results` | 820×600 CSS, scale 1.25/1.5/2 |

Browser captures show the real UI; the bridge is the mock (sample rows, no OS actions). They are used for
theme/DPI/narrow/preview-drawer coverage that the single real window could not produce.

---

## 4. Findings fixed in this pass

| ID | Sev | Finding | Fix | Evidence |
|---|---|---|---|---|
| L1 | Blocker | Window height omitted the 44 px footer (REVIEW_1 B2) → small result sets scrolled/clipped by 44 px | `computeLauncherMetrics`: `raw = searchline + 23 + content + footer + border`; viewport = `base − searchline − footer − confirm − border`; tests updated (382→384, 344→346, floor 256/138, clamp 480/362) | real empty 256 px / 50-match 480 px fixed heights; layout tests |
| L2 | Major | 2 px launcher border not counted → empty/1-row states showed a scrollbar | `WINDOW_BORDER = 2` added to geometry and results cap; empty state min-height 112 | `real-launcher-empty.png` (no scrollbar), `real-launcher-one-match.png` |
| L3 | Major | Loading skeleton inherited ui-app's global `.skeleton-row` background → wide grey band behind bars | Launcher-scoped classes `.launcher-skeleton-*`; explicit transparent row | `launcher-loading.png` (flat bars only) |
| L4 | Major | App/file fallbacks used generic `<>`/arrow glyphs, hiding file type/app identity | File-ish results show deterministic extension badges (MD/JS/DOC/JSO/SVG); apps show initials; glyphs only for semantic kinds | `real-launcher-architecture.png`, `real-launcher-long-list.png`, preview SVG |
| L5 | Major | Context menu was anchored to the launcher's lower-right corner, not the row edge (SCREENS §2.8) | Anchored to selected row using the same fixed offsets/scrollTop as keyboard nav (top/bottom flip near mid-height) | `real-launcher-context-menu.png`, `launcher-context-menu.png` |
| L6 | Blocker for V2-04 | Destructive native commands had no launcher confirmation UI; also `App.openResult` hides the launcher even on the server challenge, so the bar could not appear | Inline confirm bar above footer: role=alertdialog, engine message, **Cancel default focus**, "Run action", Esc cancels, focus/selection preserved; launcher uses `api.openResult` + `api.confirmNativeAction` directly and owns hide-after-execute | real `real-launcher-confirm.png`; 4 new unit tests (challenge, Esc cancel, bridge confirm, non-destructive unchanged) |
| L7 | Minor | Empty query + no recents showed "No matches" count | Count is blank in the quiet placeholder state; typed 0-match still shows "No matches" | `launcher-empty.png`, tests |
| L8 | Minor | Real `SearchResponse` may omit `indexedFiles` → footer item count disappeared | Fallback to `EngineStatus.filenameEntries`; verified "1,31,199 items" in real captures | `real-launcher-empty.png`, test |
| L9 | Minor | Selected-row meta/path used 4.20:1 tertiary ink (REVIEW_1 M5.1) | Selected row meta/path promoted to `--ps-ink-2` | `launcher.css`; not machine-measured this pass |
| L10 | Minor | Selection transition used linear easing; context-menu suppression was root-wide (blocked input/preview right-click) | `--ps-ease-standard`; context-menu prevention scoped to result rows only | code + menu screenshots |
| L11 | Minor | Search row stayed 72 px ≤700 px surface (REVIEW_1 M3/n5) | Media query to 61 px/16 px input; geometry uses `SEARCHLINE_HEIGHT_NARROW` for surface ≤700 | `launcher-560-*.png` |

---

## 5. DPI and theme coverage

| Mode | Real app | Browser mock | Verdict |
|---|---|---|---|
| Light 100 % | not capturable in this host session | 860×640 1× | PASS (mock) |
| Light 125 % | **not verified** | 1.25× (1025×750 physical) | PASS (mock), real pending |
| Light 150 % | **real capture of 10 states** (host DPI) | 1.5× | PASS |
| Light 200 % | **not verified** | 2× (1640×1200 physical) | PASS (mock), real pending |
| Dark | **not verified** (host theme light; launcher window does not hot-reload the setting) | results, 560 narrow, preview side | PASS (mock), real pending |
| High contrast / forced colours | **not verified** | not captured | CSS block present; needs Windows HC run |
| Narrow 560 px | **not producible** (shell window is 820 CSS; work-area clamp not exercised) | 560×640 empty/results/dark | PASS (mock), real pending |

---

## 6. Accessibility spot checks (code + visuals)

- Roles: dialog `ProtonSearch`; input combobox + `aria-activedescendant`; listbox "Results"; groups labelled by
  visible caps labels; options `aria-label` = title, `aria-describedby` = path + meta; error `role=alert`;
  footer state and throttled count polite live regions; preview `aside` + tabbable scroll region; menu
  `role=menu`; confirm bar `role=alertdialog` with Cancel focused.
- Keyboard (unit-tested): ↑/↓ wrap, PageUp/Dn ±6, Ctrl+Home/End, Home/End caret, Esc chain
  confirm → menu → preview → clear → hide, Tab/Shift+Tab input ↔ preview ↔ footer, Ctrl+Space preview,
  Ctrl+C copy path (native copy wins when text is selected), Menu key/Shift+F10/right-click menu, Enter
  epoch/id validation, rapid-arrow no queue (20 presses test), stale-response rejection.
- Confirm bar: default focus Cancel (real capture), Esc cancels, "Run action" requires explicit activation,
  focus returns to input, selection preserved. Non-destructive actions unchanged.
- Focus treatment: launcher input has no ring by design decision (caret only); every other control keeps the
  2 px `--ps-focus` ring (visible in context-menu and confirm captures).
- **Not verified:** Narrator/NVDA transcripts, Windows High Contrast themes, 100–200 % real DPI, real 560 px
  clamp, real dark. These remain manual/gate items.

---

## 7. Latency note (truthful correction)

Official `e2e-drive.mjs` report: query 1 "arch" = 32.3 ms, later queries 4.9/7.3/3.8 ms. The later numbers are
**not valid**: the harness selector counts rows still painted from the previous query, so it observes stale rows
instead of the committed response. A supplementary CDP measurement (input event → `#ps-results[aria-busy]` false
→ double rAF) on the same real app/150 % DPI gave warm end-to-end **commit-to-render 35.8–40.9 ms** for
"arch"/"readme"/"protonserch"/"a" (50/50/19/50 rows), recorded in
`app-captures/capture-run-report.json`. Distribution/p50/p95 must come from the perf harness, not this audit.

---

## 8. Known limitations / not-fixed observations (honest list)

1. **Real dark theme, real 100/125/200 % DPI, real 560 px window and Windows High Contrast are not captured.**
   Browser-mock covers them; gate evidence still needs real-window runs.
2. Preview bodies remain metadata/snippet only: the frozen IPC has no preview payload/renderer, so file text,
   image thumbnails and PDF/Office rendering are honestly "Preview not available — press Enter to open."
3. Multi-select (clipboard) is CUT pending an IPC extension; Esc chain has no multi-select branch (decided).
4. Exact-command ACTIONS pinning is not implemented — `SearchResult` has no exact-match marker; not invented.
5. Confirmation bar message text is the engine's raw Rust debug command name ("EmptyRecycleBin"). The UI must not
   rewrite engine security copy; recommend mapping to human labels in `ps-core` (`format!("{command:?}")`→label).
6. While the confirm bar is open the footer still shows Navigate/Open/Clear; SCREENS does not define a confirm
   hint set. Candidate follow-up: "Esc Cancel · Tab Run/Cancel" (design decision needed).
7. Footer item count is localized by OS locale (captures show en-IN grouping "1,31,199"); Intl-correct, but the
   design reference shows en-US grouping. Expected behaviour, called out for reviewers.
8. No app icons for installed apps in this build path: real results use monogram badges (contract exposes
   `IconRef::Monogram`/`AppIcon` only; shell icon extraction is a separate L0/ENGINE item).
9. `App.openResult` hides the launcher unconditionally after any action; the launcher now calls
   `api.openResult` directly to keep the confirmation bar and copy-path feedback visible. Other consumers may
   still need that shell behaviour fixed (needs Lead approval, outside launcher scope).
10. Leftover `vite preview` servers from earlier harness runs caused one inconsistent capture batch at 17:20
    (stale server on port 5173 + racing queries). Servers were cleaned; all real captures in this report are from
    the 17:40+ consistent run. Harness scripts should kill child processes, not only the shell wrapper.
11. `tools/visual-qa/shots.mjs` on disk during this pass still captured 5 onboarding steps at one viewport
    (mtime 17:28); Lead's announced 7 steps × 3 viewports update was not present at capture time. Launcher
    captures are unaffected; onboarding coverage should be re-run after that harness change lands.

---

## 9. Reproduce

```powershell
# UI build + tests
cd apps/desktop; npm run build; npm test

# real-app binary with confirmation support
cargo build -p protonsearch-v2 --bin ProtonSearch

# real captures + report (CDP; kills the app when done)
node tools/visual-qa/e2e-drive.mjs --serve --exe ..\..\target\debug\ProtonSearch.exe `
  --out ..\..\docs\qa\visual\app-captures --data-dir ..\..\.devdata

# browser-mock states (results/selected/empty/zero/loading/dark + narrow/preview/menu/DPI variants)
node tools/visual-qa/shots.mjs
```

**Verdict:** launcher states, geometry, keyboard/focus behaviour and the new destructive-action confirmation are
verified against real engine data at 150 % DPI, with dark/DPI/narrow coverage from the browser mock. Remaining
gaps are listed in §8 and are environment/harness limitations, not launcher code defects at this snapshot.

---

# Addendum — task-1 (filters / motion / shadow) and task-10 (app icons)

**Date:** 2026-11-09 late session. **Scope:** `apps/desktop/src/launcher/**` only.

## A1. Category / filter strip (task-1.1)

- 40 px strip between the 72 px search row and the results: chips **ALL · FILES · FOLDERS · DOCUMENTS · APPS ·
  CLIPBOARD · WEB · GIT · ACTIONS** (10 px caps, hairline bottom rule, selected = `--ps-accent-soft` tint +
  2 px inset bottom accent bar; no gradients/pills).
- Wired to `SearchRequest.scope`: all / files / folders / content / apps / clipboard / browser / git / actions.
  A filter change sends a new request id **and bumps the query epoch**, so the committed response resets selection
  to row 0. Scope is remembered per session (`sessionStorage: protonsearch.launcher.scope`).
- Keyboard: **Ctrl+1..9** from the search input (focus never leaves the input); chips are a single roving-tab stop
  (`role=radiogroup`, `role=radio`, `aria-checked`, `aria-keyshortcuts`); Arrow/Home/End inside the strip moves
  the selection and searches; clicking a chip refocuses the input. Wheel/drag scrolls the strip; chips are
  `flex: none` so they never wrap. Chips are not hovered into the Tab path while typing and never steal focus.
- Evidence (browser mock, real React UI, 860×640 and 560×640, light+dark):
  `screenshots/launcher-860-filter-{all,files,folders,content,apps,clipboard,browser,git,actions}-light.png`,
  `launcher-860-filter-content-dark.png`, `launcher-860-results-dark.png`, `launcher-560-all-light.png`,
  `launcher-560-filter-files-light.png`, `launcher-560-filter-files-dark.png`. All captures report
  `scrollWidth === clientWidth` (no horizontal overflow) at 860, 560 and 1280.
- Real engine evidence: `app-captures/real-padded-results.png` (ALL, 50 matches),
  `real-padded-filter-files.png` (FILES selected, 50 file-only results), `real-padded-zero.png`.
  Footer item count fallback verified in the same captures: "Indexing… · 1,31,199 items" while the engine
  SearchStatus counter is 0/absent. The footer word now prefers the fresher `SearchResponse.status` (ready/indexing)
  over the boot-time EngineStatus snapshot, so it flips to "Index ready" once the engine reports ready instead of
  staying on "Indexing…" (real captures show "Index ready · 1,31,199 items").

## A2. Open / close motion (task-1.2, task-10 handshake)

- Enter: 150 ms `opacity 0→1` + `scale(0.985→1)` on the card (ease-out). Exit: 110 ms `opacity 1→0` +
  `scale(1→0.99)` (ease-in-out) **before** hiding. Selection/typing/results stay instant (colour transition only).
- Shell handshake consumed: `launcher:dismiss` → `surface.dismiss()` → 110 ms animation → `api.hideLauncher()`
  (0 ms when `data-motion=reduced` or `prefers-reduced-motion`). `launcher:summoned` restarts the enter animation;
  a pending dismiss timer is cancelled on re-summon; the shell's 400 ms safety hide is generation-guarded; the
  wrapper also ignores a stale `launcher:hidden` that arrives after a newer summon.
- Real measurements (CDP, 150 % DPI): summon mid-frame opacity **0.919**; dismiss mid-frame opacity **0.824**,
  transform `scale(0.998)`; phases visible → closing → closed. Re-summon during the dismissal window ends with
  `data-phase="visible"`, window visible, input focused (stale-hide guard). Screenshots:
  `app-captures/real-launcher-summon-mid.png`, `real-launcher-dismiss-mid.png`.
- Unit tests: exit hides under reduced motion; reduced-motion immediate hide; re-summon cancels the pending close.

## A3. Shadow-safe chrome (task-1.3)

- Native window (`main.rs`) = card + 28 px logical padding on every side, including height. CSS adds nothing to the
  window size; `#root` uses the shell padding and the card keeps its own content height (820 px / 1180 px preview,
  centred), so the card is 820×content at x=28 in an 876-wide window.
- Refined layered shadow (Lead-supplied design values, local launcher variables pending token promotion):
  light `0 16px 40px rgba(36,60,43,.14), 0 2px 6px rgba(21,44,24,.06), 0 0 0 1px rgba(20,43,49,.05)`;
  dark `0 16px 40px rgba(0,0,0,.55), 0 2px 8px rgba(0,0,0,.45), 0 0 0 1px rgba(255,255,255,.04)`.
  Radius 14 px with `overflow:hidden`; no filter/clip. Real captures (transparent PNG):
  `real-padded-empty.png` (window 876×352 CSS, card 820×~292 at 28/28), `real-padded-results.png`
  (876×536, card 820×479), `real-padded-zero.png`; computed box-shadow matches the values above exactly.
- Observed sub-pixel note: on the empty/zero states the card's intrinsic content height is ~3.7 px less than the
  shell's content box (font metrics in the empty state), leaving a slightly larger transparent tail below the card;
  no clipping, no fringe. Recommend the shell/card height estimate converge (or a 1 px tolerance) if pixel-perfect
  centring is required.

## A4. Real app icons via `IconRef.app_icon_data` (task-10)

- `ResultIcon` now accepts `{ type: "app_icon_data", data }`: only `data:image/(png|jpeg|webp|gif|svg+xml);base64,`
  URLs longer than 64 chars are passed to `<img>` inside the fixed 36 px tile (28 px image, 32 px large),
  `object-fit: contain`, `decoding="async"`, alt="" (decorative, row label names the app). Invalid/unsafe data,
  decode errors and missing data fall back to the deterministic monogram badge — no broken-image glyph, no layout
  shift (fixed tile). Legacy `app_icon` path stays supported via `convertFileSrc` in Tauri.
- Unit tests: data URL renders with `data-icon-source="data"`; `data:text/html` is rejected → monogram "VS";
  firing `error` removes the img → monogram "NO". (3 tests, launcher suite now 63.)
- **Real icons verified after sources-content task-9** (engine-ready app index: 383/383 apps, icon cache under
  `PROTONSEARCH_DATA_DIR`): live captures show real `data:image/png;base64` icons in the launcher tiles:
  `app-captures/real-app-icon-chrome.png` (two Google Chrome app rows, real Chrome icon),
  `real-app-icon-vscode.png` (Visual Studio Code), `real-app-icon-notepad.png` (Notepad),
  `real-app-icons.png` and `real-app-icons-apps.png` (APPS filter, 5 app rows all `img:data`).
  DOM check per capture: `iconImages > 0`, all `data-icon-source="data"`, monogram fallback still used for
  non-app rows. Report: `app-captures/real-task10-report.json`.
  Captured against exe 23:40 (task-9 engine) and dist 23:46 (footer-status fix) at 150 % DPI; the footer now reads
  "Index ready · 1,31,199 items" in these captures instead of the stale boot "Indexing…".
  Engine-side observations to pass to sources-content (not launcher defects — the UI renders the IPC data it gets):
  some app rows carry mismatched cached images (e.g. "AMD Software" shows a blank document icon, "About iTunes"
  shows Word/screenshot icons) and the app list currently contains duplicate entries (Antigravity ×2,
  Google Chrome ×2, About iTunes ×2). Icon fallback/error behaviour remains covered by unit tests.

## A5. Build / test evidence for this addendum

- `npm run build` → exit 0 (tsc --noEmit + vite, ~0.3 s)
- `npm test` → **10 files / 165 tests passed**; launcher suite **63 tests** (task-1 filter/motion + task-10 icon and
  dismiss tests included); tsc clean.
- Browser captures: 26 launcher task-1 files (plus the full 62-shot harness run including the 7×3×2 onboarding
  matrix); all at 860×640 / 560×640 / 1280×800, light+dark, DPI 1.25/1.5/2, reduced-motion path unit-tested.
- Harness leftovers were cleaned (no ProtonSearch/vite-preview processes left after runs).

---

# Addendum — task-21 (final filter strip) and task-25 (clipboard grouped sections)

**Date:** 2026-11-10. **Scope:** `apps/desktop/src/launcher/**`. **Supersedes:** the earlier clipboard-category
chip-row draft (user rejected "filters on filters"); SCREENS §2.11/§2.12 are what shipped.

## B1. Filter strip (task-21)

- Final seven chips only: **ALL · FILES · FOLDERS · APPS · CLIPBOARD · WEB · ACTIONS** (DOCUMENTS/GIT removed, no
  ghost chips). Ctrl+1..7 from the input keeps focus; Ctrl+8/9 are unbound. Roving-tab radiogroup, ←/→/Home/End when
  the strip has focus, horizontal wheel scroll, no drag. The choice is **not persisted**: every launch starts on ALL.
- Removed scopes stay reachable through query prefixes (file:/files:, folder:/folders:, app:/apps:,
  clip:/clipboard:, web:/history:/bookmarks:/google:, content:/ocr:, git:/commits:/todos:, calc:/command:). A typed
  prefix does **not** move the strip; choosing a chip strips a leading prefix from the query.
- Strict FILES/FOLDERS: FILES = file-backed rows only, FOLDERS = folder rows only. Engine task-19 is still in
  progress, so the launcher applies a **client-side guard** (folder rows filtered out under FILES, non-folders under
  FOLDERS) as documented alpha debt; row anatomy verified for both (extension badge vs folder glyph, 20 px glyph).
- Evidence (browser mock, real React UI, seeded fixtures at capture time): `screenshots/launcher-task21-chips-light.png`,
  `launcher-task21-chips-dark.png`, `launcher-task21-files-light.png`, `launcher-task21-files-dark.png`,
  `launcher-task21-folders-light.png`, `launcher-task21-folders-dark.png`; no horizontal overflow at 860/560.

## B2. Clipboard grouped sections (task-25 — final design)

- **No second chip row, no category controls, no per-row tags.** Under the CLIPBOARD chip the result viewport shows
  clean sections in fixed order **LINKS · EMAILS · PATHS · CODE · TEXT · IMAGES**, only non-empty, each once.
  Section label: 10 px/700 caps +0.10em `--ps-ink-3`, 14 px line, 28 px block (24 compact = 8/6 + 14), inline
  padding 14; following sections get a 6 px gap; rows stay 58/48. This is one step quieter than the standard
  ResultGroup. Sections use `role="group"` + `aria-labelledby` to the visible label (`role="presentation"`);
  no tab stops, no collapse, no sticky.
- Classification (grouping only) uses the engine meta label when present, otherwise deterministic text rules from the
  exposed first line/120 chars (single URL, single email, absolute Windows path, fenced/multi-line code, else TEXT).
  IMAGES renders only when the engine emits real image rows (alpha: normally absent). Pinned rows sort inside their
  category (engine order preserved).
- Keyboard/selection: flattened visual traversal over sections and rows (↑/↓ continue across section boundaries,
  labels are not stops, Home/End/Ctrl+Home/End first/last, PgUp/Dn unchanged, Enter opens, Esc filter→query→hide).
  Section labels are announced through group semantics; no extra live region. Tab order is back to
  input → selected filter chip → preview/footer.
- Height maths reverted to the standard card: no `--ps-cat-*` strip, no 332 min, no H−192; `H = clamp(296,
  72+44+40+23+Σ(34+rowH·n), 544)`, window H+56, results `min(365, …)`. No layout shift when switching scope.
- Empty/loading copy: history off → "Clipboard history is off." / "Turn it on in ProtonSearch → Preferences to keep
  copied text on this PC."; no history → "No clipboard history yet." / "Copied text and images will appear here from
  now on. Only copies made while ProtonSearch is running are captured."; 0 matches → "No matches in Clipboard." /
  "Try different words, or press Esc to clear the filter."; loading = standard 3 skeleton rows (no section labels).
- Evidence light+dark: `screenshots/launcher-task25-clipboard-mixed-light.png` / `-dark.png` (LINKS/EMAILS/PATHS/CODE
  with recency, no tags), `launcher-task25-clipboard-empty-light.png` / `-dark.png`,
  `launcher-task25-clipboard-query-light.png` / `-dark.png` (query "latency"), `launcher-task25-clipboard-560-light.png`.
  Screenshots use the browser mock with seeded clipboard-fixture rows injected at capture time (the repo mock has no
  clipboard seed yet); no repo files outside `launcher/**` were modified for this.

## B3. Tests / build evidence

- `npm run build` exit 0; `npm test` → **14 files / 207 tests passed** at this snapshot; launcher suite **76 tests**
  covering seven chips, Ctrl+1..7, prefix clearing, non-persistence, strict FILES/FOLDERS rendering + guard,
  grouped-section order/non-empty/classification, flattened traversal across sections, empty/history-off/query empty
  copy, clipboard history-off copy, V2-04 confirm bar, icons, dismiss handshake.
- Remaining risk: real-engine screenshot of strict FILES/FOLDERS and grouped clipboard awaits task-19/24 builds;
  the UI guard and sectioning are engine-agnostic. `--ps-strip-*` tokens are still local fallbacks until L0 promotes
  them to tokens.css.
