# VISUAL AUDIT — Desktop app + onboarding (app/** · onboarding/**)

**Owner:** L2-B2/B3 (ui-app). **Date:** 2026-11-09. **Scope:** Home, Search & indexes, Activity,
Workflows, Preferences, and the seven-step first-run onboarding. The launcher is out of scope
(ui-launcher / ux-qa).

**Evidence classes**

- **Real WebView2 captures** — `docs/qa/visual/app-captures/real-app-*.png`, 1180×800 logical at
  150 % DPI (1770×1200 device pixels), live engine data (131,199 indexed entries), Windows window
  chrome. Supplied by L0; five routes only (no onboarding, no narrow/DPI variants).
- **Browser-mock captures** — `docs/qa/visual/screenshots/app-*.png`,
  `onboarding-*.png`. Real React UI rendered in Chromium by Playwright against the browser mock
  bridge; every image is real UI, data is mock. 74 captures produced for this pass, 0 horizontal
  overflow at any captured size.

---

## 1. Summary

| Severity | Found | Fixed in this pass | Carried |
|---|---:|---:|---:|
| High (misleading evidence / blocked review) | 1 | 1 (evidence replaced; tool fix needed by owner) | 0 |
| Medium (visual/accessibility defect) | 2 | 2 | 0 |
| Low (polish) | 2 | 2 | 0 |
| Decision / observation (no defect) | 2 | — | 2 documented |

Build and tests after the pass: `npm run build` green; `npm test` 9 files / 138 tests green.

---

## 2. State matrix

### 2.1 Desktop app

| Route | States captured | Viewports | Themes | Files |
|---|---|---|---|---|
| Home | loaded, index-error (`?state=error`) | 1180×800, 920×700, 560×480 | light, dark | `app-home-{light,dark}.png`, `app-home-light-{920x700,560x480}.png`, `app-home-error-{light,dark}.png` |
| Search & indexes | locations + depth, error warning | 1180×800, 920×700, 560×480 | light, dark | `app-indexes-{light,dark}.png`, `app-indexes-light-{920x700,560x480}.png`, `app-indexes-error-{light,dark}.png` |
| Activity | list (real-style search rows) | 1180×800, 920×700, 560×480 | light, dark | `app-activity-*.png` |
| Workflows | native-command documentation, agent off | 1180×800, 920×700, 560×480 | light, dark | `app-workflows-*.png` |
| Preferences | shortcut capture, appearance; privacy/agent below fold | 1180×800, 920×700, 560×480 | light, dark | `app-preferences-*.png` |
| All routes | sidebar rail at 56 / 190 / 220 px bands | 560 / 920 / 1180 | light | same files above (rail 56 at 560, 190 at 920, 220 at 1180) |

DPI: Home and Preferences at deviceScaleFactor **1.25 / 1.5 / 2.0** on a 1180×800 viewport —
`app-home-dpi{125,15,2}.png`, `app-preferences-dpi{125,15,2}.png` (file name encodes the
scale). No layout reflow or overlap; CSS-px metrics stay constant as designed.

### 2.2 Onboarding (7 steps)

| Dimension | Coverage |
|---|---|
| Steps | 1 Welcome · 2 Your name · 3 Locations · 4 Search depth · 5 Shortcut · 6 Agent · 7 Ready |
| Viewports | 900×660, 560×480 (rail hidden, full-page capture), 1180×800 |
| Themes | light and dark for every step × viewport (42 images: `onboarding-{WxH}-{theme}-step-{n}-{slug}.png`) |
| Extra states | `onboarding-900x660-light-step-4-depth-empty.png` (no locations selected), `onboarding-900x660-light-step-4-depth-resumed.png` (reload after step 4 persists via local draft) |
| DPI | 125/150/200 % verified via app captures + CSS px review; no onboarding-specific DPI capture (see §6) |

Trend checks: rail hidden exactly at 560, visible at 900/1180; step circles, caps eyebrow,
32 px h1, 13 px lead, 240 px rail, 560 px card floor, footer Back/Skip/primary all consistent;
dark theme independently art-directed (no inversion).

---

## 3. Findings, severity and fixes

### VQA-01 (High, evidence fixed; tooling owner action) — checked-in shots runner captured the wrong onboarding step
`tools/visual-qa/shots.mjs` still encodes the earlier five-step flow and uses
`getByText("Continue", { exact: false })`. On the current 7-step Welcome step that text matches
the rail sentence “Setup saved. You can close this window and continue later.”, so the click was a
no-op and **all onboarding captures were identical step 1**. The 12 stale/misnamed files were
removed and a correct run was produced (see §2.2). The runner itself is outside my write scope:
**action for the visual-QA tooling owner** — 7 steps, click `.setup-bottom .btn-primary`, tick a
location before depth, capture 900×660/560×480/1180×800 × light/dark.

### VQA-02 (Medium, fixed) — onboarding card collapsed and top-aligned at the 900×660 capture size
`onboarding.css` used `@media (max-height: 700px)` to drop `min-height: 0`; at 900×660
the card shrank to content height (438–610 px depending on step) and stuck to the top edge instead
of the 560 px reference card centred in the stage. Breakpoint moved to **600 px**: 560×480 still
uses the compressed path; 620/660/800 use the full 560 px floor and centred stage.
Evidence: `onboarding-900x660-light-step-6-agent.png` (fixed) vs previous pass.

### VQA-03 (Medium, fixed) — the “What leaves this PC?” disclosure had no expand marker
`display: flex` on `<summary>` removes the native `::marker`, so the disclosure looked
like a static row. Added a token-coloured CSS chevron with a 90 ms rotation on open (respects
`prefers-reduced-motion` / `data-motion="reduced"`). Evidence: step 6 light/dark captures.

### VQA-04 (Low, fixed) — Workflows repeated a “Built in” pill on every command row
Every row carried a pill, which is not the reference’s density and was flagged in REVIEW_1 §4.
Default rows are now silent; only “Confirmation required” keeps the warning pill.
Evidence: `app-workflows-light.png`.

### VQA-05 (Low, fixed) — Workflows first group heading touched the panel hint
Added section rhythm (margin-top on all command groups, slightly tighter for the first).
Evidence: `app-workflows-light.png` (heading now separated from the intro copy).

### VQA-06 (Decision, documented) — number grouping follows the OS locale
The real capture shows **1,31,199** because Windows is en-IN and `formatNumber` uses
`Number.prototype.toLocaleString()` (Intl). **Decision: keep locale-aware.** DESIGN_SYSTEM §4.4
requires Intl for numbers and dates; the UI language is English but numeric separators match the
user’s Windows regional format and stay consistent with localized dates/times. Forcing en-US
grouping would violate the token/design contract and make numbers inconsistent with dates. If the
product later standardises numerals to the UI language, change `formatNumber` once in
`src/lib/format.ts` (Lead-owned) — this is recorded as a design decision, not a defect.

### VQA-07 (Observation, no action) — zero counts while engine sources are stubs
Real capture: Home “Documents / Applications / Images & OCR” show 0, Applications source is
“0 items · Starting”, while filename entries show 131,199. This is truthful engine state
(sources-content still stubbing); no fabricated values are added. It will populate when
`ps-sources` lands. The amber “Starting” dot is paired with the word “Starting”, so state is not
colour-only.

### Carried design-spec deltas (not defects; recorded earlier)
Settings label column is 200 px, not the reference 450 px composition (REVIEW_1 m8, design
decision pending); inline confirm bars instead of modal dialogs (deliberate — no focus traps;
keyboard-complete with Esc/focus restore); Activity URL filters and search replay remain out of
scope (no router / no bridge replay command); Home “Sources” section is intentional per master
prompt §V.B and lead direction.

---

## 4. Checks performed against SCREENS §3–§6 and KEYBOARD_AND_A11Y

| Contract item | Result |
|---|---|
| Sidebar 220/190/56 at ≥1100 / 820–1099 / ≤819 | Pass; 56 at 560 viewport, 190 at 920, 220 at 1180 (captures) |
| Workspace padding bands 36/40, 32/32, 31/24, 31/18 | Pass; no horizontal overflow at 560/920/1180 |
| Route h1, eyebrow, lead, caps section-label hierarchy | Pass on all five routes, light/dark |
| Skip link first focusable; nav roving tabindex; route h1 focus | Pass (code + smoke tests); h1 ring suppressed intentionally |
| Switch = 24 px+ target + visible On/Off | Pass (state word visible in all captures) |
| Segmented selected state perceivable without colour memory | Pass (raised fill + 1 px boundary + ink weight in captures) |
| Selected nav cue = fill + ink + 2 px accent bar | Pass (visible in all route captures) |
| Honest states: engine error, stubbed sources, unavailable actions | Pass; no fake progress percentages anywhere |
| Onboarding rail hidden ≤700, eyebrow carries position | Pass: 560 hidden, 900/1180 visible; eyebrow “… 0n of 07” |
| Onboarding step h1 programmatic focus, Esc no-op, skip always available | Pass (smoke tests + code review) |
| Target sizes ≥24 px (linkbtn/switch/seg/icon buttons) | Pass from prior fix pass (min-height 24/28/32); visual check shows no cramped controls |
| Reduced motion / forced colours | CSS tokens + overrides present; runtime OS-setting verification not performed here |
| Dark theme art direction (not inversion) | Pass on every captured route/step |

---

## 5. Files

Captured (new/regenerated):

- `docs/qa/visual/screenshots/app-{home,indexes,activity,workflows,preferences}-{light,dark}.png`
- `docs/qa/visual/screenshots/app-{route}-light-{560x480,920x700}.png`
- `docs/qa/visual/screenshots/app-home-error-{light,dark}.png`,
  `app-indexes-error-{light,dark}.png`
- `docs/qa/visual/screenshots/app-{home,preferences}-dpi{125,15,2}.png`
- `docs/qa/visual/screenshots/onboarding-{900x660,560x480,1180x800}-{light,dark}-step-{1..7}-{slug}.png`
- `docs/qa/visual/screenshots/onboarding-900x660-light-step-4-depth-{empty,resumed}.png`

Code fixed:

- `apps/desktop/src/onboarding/onboarding.css` — card-height breakpoint (VQA-02), disclosure
  marker (VQA-03).
- `apps/desktop/src/app/views/WorkflowsView.tsx` + `apps/desktop/src/app/desktop.css` —
  default pill removal (VQA-04), group rhythm (VQA-05).

Removed: stale `onboarding-step-1..5-*.png` (and two earlier quote-mangled duplicates)
produced by the outdated runner.

---

## 6. Not verified here (honest gaps)

1. **Real WebView2 onboarding** — supplied real captures cover the five app routes only; all
   onboarding evidence is Chromium + mock bridge. Real-window onboarding at 900×660/560×480 needs
   an L0/ux-qa capture run.
2. **Real narrow/DPI windows** — 560×480, 920×700 and 125/150/200 % DPI verified in the browser
   renderer, not in the Tauri window with live engine; WebView2 text scaling could differ.
3. **Empty and filtered-empty app states** — the browser mock seeds settings/activity; there is no
   `state=empty` fixture. Home “Nothing indexed yet”, Activity “No activity yet”, and filtered-empty
   were code-reviewed and covered by unit/smoke tests, not screenshot-verified. A mock fixture is
   needed.
4. **Degraded index state** — mock supports ready/indexing/error only; `EngineStatus` has no
   `degraded` enum (only `SearchStatus` does), so the degraded rendering path was not exercised.
5. **High-contrast Windows themes (Black/White/Aquatic/Desert)** — forced-colours CSS exists but
   real HC rendering was not captured.
6. **Reduced-motion OS setting** — token collapse is code-verified; no animated-state video/GIF.
7. **Screen-reader sessions (Narrator/NVDA)** — keyboard journeys are covered by automated
   component tests; no SR transcript exists yet.
8. **Engine-populated states** — Documents/Applications/Images counts, sources health beyond
   “Starting”, and agent runtime states depend on crates still being stubbed (see VQA-07).
9. **Launcher** — outside this audit’s scope (ui-launcher / ux-qa).

---

## 7. Regeneration

The audit harness is a temporary Playwright script (browser mock, preview server on port 4182) and
is not checked in. Recommended permanent command after the tooling fix in VQA-01:
`node tools/visual-qa/shots.mjs --out docs/qa/visual/screenshots` (all app routes + 7-step
onboarding, light/dark, all viewports). Real-window captures are produced separately by L0/ux-qa.

---

## 8. Task-2 follow-up — window-open motion and count truthfulness (2026-11-09)

**Implemented**

- **App open (task-2 §1):** `app/transition.ts` + `DesktopApp` apply
  `.app-shell.is-opening` with `opacity 0→1 + translateY(4px→0)` over
  `var(--ps-dur-panel)` (160 ms, inside the 140–180 ms brief). It plays on first mount
  (app start / onboarding → app) and when the document becomes visible again (tray
  “Open ProtonSearch” / launcher footer), never on route changes (the shell does not remount).
- **Onboarding → app:** real 160 ms root crossfade through the View Transition API in
  `OnboardingView`; the app shell skips its own mount animation when the crossfade is active
  (no double fade). Falls back to the shell fade when the API or motion setting disallows it.
- **Reduced motion:** `prefers-reduced-motion` and `data-motion="reduced"` collapse
  `--ps-dur-panel` to 0 ms (plus base.css animation overrides); no View Transition is started.
- **Count truthfulness (task-2 §2):** `app/describe.ts` helpers
  `hasIndexCount / indexCountText / sourceStatusText / rootStatusText`;
  Home, Search & indexes, and Preferences diagnostics all read
  `EngineStatus.filenameEntries`. “Starting…” is shown only before the first count; a numeric 0 is
  shown once the engine reports 0; a starting source/root no longer prints “0 items” while the
  engine reports entries.

**Evidence** (Playwright, browser mock, 1180×800; `docs/qa/visual/screenshots/`):

| Check | Result |
|---|---|
| First paint animation class + computed style | `is-opening` present; `animationName: app-window-in`; `animationDuration: 0.16s`; keyframes and rule present |
| Home indexed count (light/dark) | `124,318` from mock `EngineStatus.filenameEntries`; source rows `126 items · Ready` (no transient 0) |
| Reduced motion | `matchMedia(reduce) = true`, `data-motion="reduced"`, `--ps-dur-panel = 0s`, shell `animationDuration = 0s` |
| Onboarding → app crossfade | `startViewTransition` called once; app shell mounts without its own `is-opening` class |
| Reduced-motion crossfade fallback | 0 View Transition calls; app still reaches Home |
| Route changes | shell animation class not re-added (unit/smoke test) |

Screenshots: `task2-app-open-light.png`, `task2-app-open-dark.png`,
`task2-app-open-light-reduced-motion.png`, `task2-onboarding-to-app.png`.
Tests: `transition.test.ts` (flag consumption, reduced-motion detection, support probe),
`describe.test.ts` (count truthfulness), `smoke.test.tsx` (one-shot open animation, no
animation on route changes, filename count not 0).

**Not verified:** tray/launcher-footer window-show path in a real Tauri window (the browser mock
cannot emit native window visibility; `visibilitychange` is the replay hook, needs a real
WebView2 check); OS-level reduced-motion with the real setting; animation frame pacing on WebView2.
The app is unaffected by the launcher's padded window — no shared layout code.

---

## 9. Task-5 — onboarding dark-mode elevation (design review L6)

**Fix (onboarding.css only; exact REVIEW_1 §8.2 / L6 values)**

- Stage padding 26 → **24 px**.
- Card min-height 560 → `min(620px, calc(100vh - 48px))` so it fills the window instead of floating in a void.
- Card border → **1 px `var(--ps-line)`** (was line-strong).
- Layered clip-safe shadow, light: `0 2px 6px rgba(36,60,43,.08), 0 8px 16px rgba(36,60,43,.06)`;
  dark: `0 2px 6px rgba(0,0,0,.30), 0 8px 16px rgba(0,0,0,.28)`. The previous single
  `var(--ps-shadow-app) 0 18px 45px` was invisible against the dark ground and read as a flat rectangle.
- Short-window media override no longer zeroes the min-height.

**Before / after probes** (Playwright, browser mock, computed styles):

| Viewport / theme | Before | After |
|---|---|---|
| 900×660 dark | min-height 560, padding 26, card top 50, single 18/45 shadow | min-height **612** (fills 24+612+24), padding 24, border `rgb(46,50,44)`, layered dark shadow, card top 24 |
| 1180×800 dark | min-height 560, card top 120 | min-height **620**, top 90, layered dark shadow |
| 900×660 light | single 18/45 shadow | layered light-green shadow, min-height 612 |
| 1180×800 light | min-height 560 | min-height 620 |

Screenshots: `task5-before-{900x660,1180x800}-{light,dark}.png`,
`task5-after-{900x660,1180x800}-{light,dark}.png`.

**Not verified:** real WebView2 dark window at 150 % DPI (the layered shadow/DAU contrast should be confirmed on
the real desktop; the browser renderer shows the elevation correctly).

---

## 10. Task-6 — Preferences: default web-search engine + toggle

New **Search** settings section (after Launcher shortcut): switch “Offer web search results”
(`webSearchEnabled`), segmented engine control **Google / Bing / DuckDuckGo / YouTube** mapped to
`google / bing / duck_duck_go / you_tube` (`webSearchEngine`), and the required helper copy
“Opens the result in your browser; nothing is sent until you choose it.” Autosave via
`updateSettings({ webSearchEnabled } / { webSearchEngine })` with the standard 2 s “Saved” stamp.

Evidence (Playwright, 1180×800 light/dark and 900×660 light): section present with 4 radios
[Google, Bing, DuckDuckGo, YouTube]; switching “Offer web search results” off persists and disables the engine
radios; switching it back on re-enables them. Screenshots:
`task6-preferences-websearch-{1180x800,900x660}-{light,dark}.png`.
The launcher’s “Search the web for …” result row itself is launcher scope and not changed here.

---

## 11. Task-7 — professional typography, alignment grid, quiet error states (2026-11-09)

**Evidence class:** browser-mock captures (real React UI, mock data) + computed-style/canary probes. Build/test green
(10 files / 167 tests). Launcher untouched; `tokens.css` not edited.

### 11.1 Type scale (final task-8 values; TEMP block pending L0 tokens)
New `apps/desktop/src/app/app-scale.css` scoped to `html[data-window="main"]` (TEMP comment; covers app + onboarding, launcher untouched):
display clamp(28,3.6vw,44)/1.08 · setup h1 34/1.16 · stat 36/1.0 650 · lead 15/24 · body 14/21 · strong/row title 14/20 ·
small/nav 13/20 · meta 12/18 · micro/caps 11/16 · section 12/16 caps · kbd 11/14 · control 13/20; sidebar 232/200/64,
row nav 44 / list 68 / metric 44 / control 36 / field 40, compact density overrides included. Probes at 1180×800:
lead 15 px, nav 13 px, metric value 14 px.

### 11.2 Alignment grid
Workspace `max-width: calc(1120 + 2 × margin); padding margin (24–40 per band) + 64 bottom`; 12-col-equivalent
composition: Home 7/5 gap 24, stacked ≤819; Settings 280 px label + 24 gap, stacked ≤1023; card padding 24; section
gap 32 / panel gap 24; page-header rhythm eyebrow 8 / h1 12 / lead 32, lead max 60 ch; Home search 52 px; nav row 44;
list row 68; metric row 44; buttons/controls 36; inline icons 16, state icons 20, file tiles 32; nav icon slot 20.
Fixed inconsistent label-column/panel alignment, double spacing above first panels, notice/strip rhythm, and
notice-after-search spacing. No overflow at 560/720/820/1000/1180.

### 11.3 Quiet state components and copy (SCREENS §3.7)

> **Correction (2026-11-10, REVIEW_2 follow-up):** at the time of this pass the “offline/empty variants implemented”
> and “no raw engine strings” claims below were overstated. The missing variants, the direct `agent.detail`/`source.detail`
> renders and the severity-selector typo were fixed in task-15; see §12 for the corrected status and evidence.
New `StateNotice` (`.state/.state--warn/.state--error/.state--offline`, 20 px icon, title 15/600, body 14/21 max 60 ch,
native `Details` disclosure, one action) and `.row-state` for one failing row/source. Raw `err.message`/engine
warning strings are no longer rendered anywhere in app views (grep-verified); `errorMessage` helper removed.
Copy map implemented: index-status load failure, engine warnings, indexer stopped, settings save failure (retry keeps
the failed patch), hotkey invalid/unavailable, activity load/open, agent unavailable/off, source read failure,
network root, onboarding finish failure, and empty states (Home no index/no recents, Search & indexes no
locations/exclusions/sources, Activity empty/filtered with “Show all”, Workflows agent off, onboarding no locations).
Aria: load failures `role=status`, action-triggered `role=alert`, details native disclosure.

**Engine warning aggregation (lead request):** many engine warning strings (e.g. malformed PDFs) now render as one
`Index needs attention.` notice with the canonical body and a sanitized Details line (count only, no paths/raw strings);
engine can keep capping/aggregating upstream — UI never prints individual warning strings.

### 11.4 Canary and coverage
- **Canary 560×480 @200 % DPI, all five routes:** no horizontal scroll, minimum rendered text = 11 px, zero elements
  below 11 px.
- Route captures: `task7-{home,indexes,activity,workflows,preferences}-{light,dark}-1180x800.png`,
  `task7-{route}-light-{720x560,560x480}.png`, `task7-canary-{route}-560x480-dpi2.png`.
- Error captures: `task7-error-{home,indexes}-{light,dark}.png`.
- Onboarding: `task7-onboarding-{light,dark}-900x660.png` (34 px h1, 15 px lead, body 14, no overflow; L6 card still fills).

### 11.5 Known limits / not verified
- `App.tsx` fatal bootstrap/settings-load screen is L0-owned and still prints a raw message (“ProtonSearch could
  not start” + detail). The settings-load error copy in §3.7 needs an App.tsx change by L0; out of this task’s scope.
- No IPC command for agent “Try again”, per-source retry, or network-root retry: those rows state automatic retry
  instead of a dead control.
- Real WebView2 125/150/200 % DPI, Windows high-contrast and screen-reader sessions are not captured here (browser
  renderer canary only).
- Task-13 (Pi agent panel) could not be claimed: blocked by task-11 and the shared task board rejects claiming an
  unready task. Ready when the Pi backend lands.

---

## 12. REVIEW_2 follow-up / task-15 — quiet-states must-fix (2026-11-10)

Source: design-spec REVIEW_2 (F1–F9). Scope: app/** + onboarding/**; `tokens.css` and `App.tsx` untouched (L0 items).
Build/tests after the fixes: **11 files / 174 tests green**.

| Item | Status | Evidence |
|---|---|---|
| **F1 MAJOR** raw engine/source detail on the surface | **Fixed** | `describeAgent` no longer forwards `agent.detail` (product copy for running/stopped/error/not-configured); `describeSource` no longer injects `source.detail` into descriptions; IndexesView no longer renders `source.detail`; poisoned-detail regression tests in `describe.test.ts` (path “C:\\Users\\…/ENOENT/provider stack” asserted absent from all product copy, incl. per-state agent coverage). |
| **F2 MAJOR** missing `.state--offline`/`.state--empty` | **Fixed** | `StateTone = neutral/warn/error/offline/empty`; cloud-off icon added; `.state--offline`/`.state--empty` styles (12 px radius, 20/24 padding, 20 px icon, title 15/600, body 14/21); `EmptyState` migrated to the shared `.state .state--empty` anatomy (old `.empty-block` CSS removed). Verified: empty block radius **12 px**, padding **20/24**, icon **20 px**; gallery below. |
| **F3 MEDIUM** severity icons always grey | **Fixed + asserted** | CSS selectors now target `.state--warn .state-icon` / `.state--error .state-icon` (dead `.state-warn/.state-error/.row-error` selectors removed). Computed colours in-browser: error **rgb(169,42,47)** (negative), warn **rgb(138,90,18)** (warning), not `--ps-ink-2` grey. `ui.test.tsx` asserts the selectors and the emitted classes; `EmptyState`/tone markup covered too. |
| F4 copy gaps / pluralisation | Fixed | `summarizeWarnings` uses `Intl.PluralRules` (“1 item / 2 items”); paused (“Indexing is paused.”) and agent-not-set-up (“Agent workflows aren’t set up.”) surfaces carry product copy. Docs rows are design-spec-owned. |
| F5 Home margin-top 40→32 | Fixed | `.home-columns` margin-top `var(--ps-app-section-gap)` (32 px). |
| F6 onboarding step circle/pitch + stale comment | Fixed | 28 px circle, 56 px step pitch per DESIGN row 27; header comment now “hidden ≤700”. |
| F7 dead `errorMessage` helper | Fixed | Removed from `hooks.ts`; `grep` shows no usages. |
| F8 App.tsx raw Technical details | L0-owned | Not edited; Lead is sanitizing. Still the only raw-detail surface. |
| F9 `.state--neutral` with no CSS | Fixed | Explicit neutral rule added; all five variants covered by `ui.test.tsx`. |

**State screenshots:** `task15-state-error-home-light.png` (error/alert), `task15-state-warn-indexes-light.png`
(aggregated engine warnings; amber icon), `task15-state-empty-indexes-light.png` (migrated EmptyState), and a
component gallery of warn/error/offline/empty in `task15-states-gallery-{light,dark}.png` (same `ui.css` build,
explicitly labelled not a product route; offline/empty have no product path in the current mock).

**Row-state tone (REVIEW_2 nit):** `.row-state` uses the warning colour deliberately: these rows are auto-retry
states (“ProtonSearch will try again automatically” / reconnectable network root), not blocking failures. Blocking
failures use `.state--error` with the negative colour. This is documented in `ui.css` next to the selector.

**Audit correction:** §11.3 above was overstated at the time and is now corrected by this table; it is safe to treat
§11.3 + §12 as the current quiet-state record. Not verified here: real WebView2 screenshots of the states, Windows
high-contrast, screen-reader announcements (browser-rendered/computed evidence only).