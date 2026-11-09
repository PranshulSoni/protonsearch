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