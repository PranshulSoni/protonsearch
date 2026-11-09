# REVIEW_1 — Design & Accessibility review of the v2 UI implementation

**Reviewer:** L1-B Product Design & Experience (`design-spec`). **Date:** 2026-11-09, snapshot ~17:00.
**Scope:** `apps/desktop/src/styles/tokens.css`, `styles/base.css`, `App.tsx`, `main.tsx`, `components/*`, `lib/*`,
`launcher/**`, `app/**`, `onboarding/**`, plus the adjacent L0 shell files needed to judge behaviour
(`apps/desktop/index.html`, `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json`).
**Contract:** `docs/design/DESIGN_SYSTEM.md`, `SCREENS.md`, `KEYBOARD_AND_A11Y.md`, `REFERENCE_MAPPING.md`,
`assets/brand/README.md`. No code was edited. No GUI was run (constraint); every runtime-dependent claim is marked
**UNVERIFIED** and needs a real-window check by L0/ux-qa.

**Snapshot caveat:** ui-launcher/ui-app were writing while this review ran (`LauncherSurface.tsx` 16:55,
`launcherSurface.test.tsx` 16:59, `OnboardingView.tsx` 16:50, `main.rs` 16:56, `tokens.css` 16:54, `Logo.tsx` 16:54).
Line numbers refer to that snapshot; re-check before acting. One finding from the first pass was already fixed during
review (brand mark now uses tokens + a true cut-out, see §1.3) — the review reflects current files, not my first draft.

---

## 0. Summary

| Severity | Count | Gate impact |
|---|---:|---|
| Blocker | 2 | Gate 3 cannot pass acceptance (keyboard-only + launcher first-use geometry). |
| Major | 10 | Gate 4 needs decisions; some are IPC-contract gaps, not UI defects. |
| Minor | 16 | Fix or amend docs before sign-off; none hide broken functionality. |
| Nit | 6 | Polish, can trail Gate 4. |

The implementation is **substantially ahead of a skeleton** and the design direction is being followed: tokens only,
no gradients/glow/glass, honest unavailable states, real Lucide-style icons, reference-derived launcher hierarchy,
throttled live counting, stale-response protection, and a cut-out brand mark. The two blockers are specific and
cheap to fix; the majors are mostly contract decisions (IPC capability vs spec text). The Lead's post-review decisions
are recorded in §0.1 and have been applied to the design docs.

### 0.1 Post-review decisions (Lead, 2026-11-09) — applied to the design docs

| # | Decision | Effect on this review |
|---|---|---|
| a | Selected-row **and** nav-active 2 px accent bar confirmed (code direction) | Closes the nav-active part of M5.4; DESIGN_SYSTEM §3.5/§8 updated. Remaining M5 contrast items are unchanged code work. |
| b | Preview geometry final: **window 1180 / side panel 360 / threshold 1200 work width** | Closes §1.2; SCREENS/DESIGN_SYSTEM updated. Launcher CSS/JS (1220/380/1240) is now code debt. |
| c | Clipboard multi-select + Paste-N **CUT** for v2.0 alpha; exact-command ACTIONS pin **CUT** | M1 closed as deferred (not a defect); the ACTIONS-pin half of M4 closed. SCREENS §2.2/§2.7/§10 + KEYBOARD §2/§5/§11 updated; tests must assert absence. |
| d | IPC gaps (progress %, clear-activity, quicklinks CRUD, folder picker, updater, preview payload) accepted as **alpha limitations** under the truthful-UI rule | M9 closed as documented limitation; SCREENS §10 added; UI copy must stay honest. |
| e | Main window minimum **560×480** | Resolves the window half of M3; SCREENS §1/§6 updated. main.rs min size (720×560) is now code debt. |
| f | Onboarding rail breakpoint **700** | Resolves the onboarding half of M3; SCREENS §4.1/§6.1 updated. onboarding.css (760) is now code debt. |

**Still open after the decisions (code work, not docs):** B1 input focus, B2 launcher height, M2 hotkey capture/registration,
M5 contrast items not covered by (a), M6 target sizes, M7 launcher boot, M8 multi-monitor, M10 Intl times, M3 code
alignment, and the minors/nits in §2.3–§2.4. Nothing else in this review is withdrawn.

---

## 1. Requested confirmations

### 1.1 `--faint` — **NOT changed** in the current file
`apps/desktop/src/styles/tokens.css:160` still defines `--faint: #8a948e` (light) and `:318` `--faint: #7c857b`
(dark). Measured: **3.04:1** on `--ps-surface` and **4.40:1** on dark surface — both below 4.5:1 for text.
It is **unused** in `apps/desktop/src` (grep: only the two declarations), so there is no live defect today.
**Fix (exact):** either delete the alias, or set it to `var(--ps-ink-3)` (as you intended) — but note `--ps-ink-3`
on `--ps-surface-soft` is 4.48:1 and on `--ps-accent-soft` 4.20:1, so the "use ink-3 everywhere" alias would still
fail on those two surfaces. Keeping it unused is acceptable if it is documented as decorative-only. **Final decision (2026-11-09): keep it
unused/out of the contract (or delete it); do not alias it to `--ps-ink-3`, which itself fails on soft (4.48:1) and
selected (4.20:1) surfaces.**

### 1.2 Preview width — token landed, **component values still disagree**
`--launcher-width-preview: 1180px` is in `tokens.css:218` (correct), and `src-tauri/src/main.rs:26` uses 1180. But:
- `launcher/launcher.css:21-22`: `.launcher--preview-side { width: min(1220px, 96vw); }` (should be the token);
- `launcher/launcher.css:368-370`: `.preview { width: 380px; }` (docs: 360 px);
- `launcher/launcherLayout.ts:25`: `PREVIEW_SIDE_MIN_WORK_WIDTH = 1240` (docs: 1200).
Consequence: on a 1200–1239 CSS-px work area the UI chooses the bottom drawer while the shell reserves the 1180 window,
and the side layout is 20 px wider than the design contract. **Fix:** use `var(--launcher-width-preview)`, 360 px, and
1200 in the module (or amend SCREENS §2.1/§2.6 back — but the token is already landed). **Final decision (2026-11-09): 1180 window / 360 panel / ≥1200 threshold; the code
must follow (REVIEW_1 M3 code task).**

### 1.3 Brand mark — **fixed during the review**
`components/Logo.tsx:19-37` now uses a real cut-out mask (no white patch) and token-driven fills
(`--ps-brand-body`/`--ps-brand-facet`, defined at `tokens.css:130-131` light and `:299-300` dark). This resolves my
earlier concern; remaining nits: the mark is 26 px in the launcher regardless of width
(`LauncherSurface.tsx:767`; docs 29 px desktop / 26 px ≤700) and the 16 px optical asset is not used (no current
context renders below 24 px, so acceptable).

---

## 2. Findings

### 2.1 Blockers

**B1 — No visible keyboard focus on any text input outside the launcher.**
`styles/base.css:47-50` declares `:focus-visible { outline: 2px solid var(--focus) }`, but `base.css:52-54` declares
`input:focus { outline: none; }` — `input:focus` has higher specificity (0,1,1 vs 0,1,0), so it wins for every
`<input>`. Only the launcher input restores a ring (`launcher.css:65-69`, class+pseudo beats it). Every app/onboarding
input therefore has **no visible focus indicator**: Preferences hotkey and results-per-query fields
(`PreferencesView.tsx:191-205, 240-258`), Indexes add/exclusion fields (`IndexesView.tsx:372-387, 649-660`),
onboarding name/root/shortcut fields (`OnboardingView.tsx:270-284, 325-340, 425-439`), `.field` has no focus rule
(`onboarding.css:257-270`). Violates KEYBOARD_AND_A11Y §8 and WCAG 2.4.7; blocks the keyboard-only gate.
**Fix:** delete `input:focus { outline: none }`; add
`input:focus:not(:focus-visible) { outline: none; }` if the mouse ring must be suppressed, plus a component-level
`.input:focus-visible, .field:focus-visible { outline: 2px solid var(--ps-focus); outline-offset: 2px; }`.
Verify by tabbing Preferences → Indexes → Onboarding without a mouse.

**B2 — Launcher window is 44 px shorter than its own content (footer height omitted).**
`launcher/launcherLayout.ts:90-93` computes `raw = SEARCHLINE_HEIGHT(72) + RESULTS_PADDING(23) + content` and clamps
to 480; `FOOTER_HEIGHT(44)` is only used when deriving `listViewport` (`:101-104`). The window therefore always
receives 44 px less than the DOM needs for search + results + footer, so the results list is 44 px shorter than the
content it is meant to show. Example in the tests: two groups / three rows compute `baseHeight = 338`
(`launcherLayout.test.ts:73-75`) while the surface needs `72 + 23 + 242 + 44 = 381`. Effect: a scrollbar and a
clipped last row appear for small result sets that should fit (SCREENS §2.1 growth rule; reference grows to fit),
and the "compact 256 px" floor is really 140 px of list where the design assumes 140 px + footer. **Fix (exact):**
`const raw = SEARCHLINE_HEIGHT + RESULTS_PADDING + content + FOOTER_HEIGHT;` and
`listViewport = max(0, min(MAX_RESULTS_VIEWPORT, baseHeight - SEARCHLINE_HEIGHT - FOOTER_HEIGHT));`
then update `launcherLayout.test.ts` expectations (3-row case becomes 382, max case clamp to 480 still yields a
364 px viewport; the 256 floor still yields 140). This is a 2-line code change plus tests.

### 2.2 Majors

**M1 — Clipboard multi-select is not implemented.**
`LauncherSurface.tsx:622-629`: Enter ignores Shift (the Shift branch falls through to nothing) and there is no
selection state, `aria-multiselectable`, check marker, or "N selected / Enter Paste N" footer. SCREENS §2.7 and
KEYBOARD §5 specify it for clipboard results (legacy parity #8, master prompt §V.A "multi-select where appropriate").
**Fix:** add `multiSelect: Set<resultId>`; Shift+Enter toggles the active row and enters the mode; Shift+↑/↓ extends,
Ctrl+A selects all; Esc clears the mode before the query; footer shows the count and Paste hint; listbox gets
`aria-multiselectable="true"` only in that mode; add tests alongside `launcherSurface.test.tsx`. **Decision (2026-11-09): CUT for v2.0 alpha** — recorded in SCREENS
§10 with rationale; do not build a stub or teaser, and add a test asserting the feature is absent.

**M2 — Hotkey configuration is typed text; no capture, no OS-conflict feedback, no live registration.**
Preferences (`PreferencesView.tsx:187-236`) and onboarding (`OnboardingView.tsx:419-473`) ask the user to type
"Alt+Space" into an input; `hotkey.ts:23-27` tells the user the new shortcut "is saved immediately and applied to the
global hotkey when ProtonSearch restarts", and `main.rs:390-391` only registers the shortcut at startup. SCREENS
§3.6/§4.6 and KEYBOARD §6 require a press-the-chord capture control, inline conflict error ("That shortcut is already
used by another app.") and immediate application. Today a conflicting chord is accepted silently and only fails at the
next launch (no feedback at all — `register(...)` result is discarded, `main.rs:391`). **Fix:** capture-mode control
(keydown capture, Esc cancels, modifier validation), a Tauri command that re-registers and returns a typed error,
inline error in Preferences/onboarding, and a truthful note when a restart is genuinely required. If re-registration
is deferred, the docs must say "takes effect after restart" and the note must be styled as a limitation, not help text.

**M3 — Responsive contract drift (measured bands differ from SCREENS §6).**
- App window minimum is **720×560** (`main.rs:181`), so the spec's 560 px app width cannot be produced or captured;
- onboarding rail hides at **≤760 px** (`onboarding.css:515`) not ≤700;
- launcher preview threshold/widths 1240/1220/380 vs 1200/1180/360 (see §1.2);
- at `max-width:700px` the launcher search row keeps 72 px height and only shrinks the font
  (`launcher.css:716-724`), while SCREENS §6.1 specifies 61 px + 16 px input + hidden meta at that breakpoint
  (meta hiding is implemented at `:726-728`, the 61 px height is not).
**Fix:** lower `min_inner_size` to **560×480** and align the onboarding rail to **700** — the final decision recorded
in §0.1 (the code currently uses 720×560 and 760). Every Gate 4 screenshot claim depends on this, so it must be settled first. **Decision (2026-11-09): main app window minimum 560×480; onboarding rail hides ≤700;
preview 1180/360/≥1200. Docs now state the final contract; code must follow (M3 remains a code task).**

**M4 — Result group taxonomy/labels diverge from the screen spec.**
`lib/results.ts:3-26` defines a 22-kind order; `GROUP_LABELS` (`:28-51`) invents labels not in SCREENS §2.2:
"COMMANDS", "CALCULATOR", "IN DOCUMENTS", "IN SCREENSHOTS", "WORKFLOWS", "BROWSER", "GIT", "WINDOWS", "SETTINGS",
"AGENT", "SYSTEM". SCREENS §2.2 fixes 8 groups (APPLICATIONS · FILES · FOLDERS · DOCUMENTS · CLIPBOARD · WEB · CODE ·
ACTIONS, exact command pinned first). Rendering is otherwise correct (visual order and keyboard order now agree —
`LauncherSurface.tsx:244-260`). **Fix (design decision):** either map the 22 engine kinds onto the 8 display groups
(recommended: keeps the reference's 10 px caps hierarchy and avoids "CALCULATOR" as a group header), or amend
SCREENS §2.2 to the implemented taxonomy. Do not leave both. **Decision (2026-11-09): the exact-command ACTIONS pin is CUT for v2.0 alpha; keep the fixed
eight-group order with ACTIONS last and map engine kinds onto it.**

**M5 — Contrast/state-cue failures (WCAG 1.4.11 / 4.5:1).**
1. **Selected row meta:** `launcher.css:193-200` keeps `--ps-ink-3` on the selected fill → **4.20:1** (needs 4.5).
   Fix: `.result[aria-selected="true"] .result-meta { color: var(--ps-ink-2); }` (**4.96:1**).
2. **Segmented/filter selected state:** `ui.css:77-110` and `:113-137`: selected fill (`--ps-surface-raised`) vs
   track (`--ps-surface-soft`) = **1.10:1**, track border vs surface = **1.24:1**. The checked state is not
   perceivable without colour memory. Fix: add `box-shadow: inset 0 0 0 1px var(--ps-accent)` (accent vs soft ≈
   6.9:1) or a 1 px `--ps-ink-3` border; keep the fill.
3. **Switch:** `ui.css:139-148` unchecked track `--ps-surface-soft` vs surface = **1.07:1**, border
   `--ps-line-strong` = **1.46:1** — the control boundary fails 3:1. Fix: `border-color: var(--ps-ink-3)` for the
   off state (4.5:1) and keep the 7.35:1 on-state accent fill.
4. **Sidebar active item:** `desktop.css:91-95` active bg vs sidebar = **1.13:1** (only ink changes, 1.71:1 between
   the two inks). Reference-style tint alone is not a compliant state cue. Fix: 2 px inset accent start-bar (same
   correction already accepted for the selected result row) and amend DESIGN_SYSTEM §8 "no left bar" for the nav.
5. `launcher.css:740-750` already hides the footer count at ≤620, good; no action.

**M6 — Pointer target sizes below WCAG 2.2 AA (24×24).**
`ui.css:239-247`: `.linkbtn` (Retry, Dismiss, Resume, Manage sources, Refresh, etc.) has 2 px vertical padding →
~16 px tall. `ui.css:139-143`: the switch is 36×20 (20 px high). Fix: `.linkbtn { min-height: 24px; padding: 4px 6px;
}` and make the switch 36×24 or wrap it in a 44 px row that toggles (the preferred rows already have the label).
Then audit every `.linkbtn` usage in Home/Indexes/Activity/Workflows/Preferences.

**M7 — Launcher boot: background flash and route-resolution risk (UNVERIFIED).**
`index.html:2` hard-codes `<html lang="en" data-window="main">`; `base.css:17-19` paints `body` with
`var(--stage)` for that attribute; `main.tsx:11-31` resolves the real view only after the module executes, using
`window.__TAURI_INTERNALS__.metadata.currentWindow.label` (the shape the Tauri `mocks.js:209-212` provides; its
presence in the installed Tauri 2.12 runtime is **unverified**). If the first paint happens before the module or if
that metadata path is unavailable, the transparent launcher window paints the grey stage and can even mount the
desktop app. Gate 3 explicitly requires "no white flash". **Fix:** an inline, pre-module script in `index.html` that
reads `location.search`/the window label synchronously and sets `data-window` plus the correct body background
before any stylesheet/React work; then verify on the real launcher window (L0 gate).

**M8 — Launcher is placed on the launcher window's monitor, not the cursor's monitor.**
`main.rs:41-67` uses `win.current_monitor()` (the hidden launcher's last monitor; at creation that is the primary) and
never moves to the display containing the cursor. SCREENS §1 and the master prompt's summon behaviour assume the
active display. **Fix:** use `app.cursor_position()` + `monitor_from_point` (or the foreground window's monitor),
reposition before `show()`, then clamp exactly as `place_launcher` does. Multi-monitor is P1, so a documented
limitation is an acceptable fallback for Gate 3 only if explicitly recorded.

**M9 — Screen spec promises data/controls the frozen IPC cannot deliver.**
Current honest UI ("not available in this build") is *good product behaviour*, but these are spec/contract conflicts
that must be resolved before Gate 4 sign-off:
- progress: `EngineStatus` (`bridge/types.ts:193-205`) has state string only — no percentage, no phase. SCREENS §3.2
  Home progress bar, §2.5 "Indexing · 42 %" and §4.8 truthful indexing progress cannot be implemented truthfully;
- file icons: `IconRef` (`:51-55`) has `app_icon | file_type | monogram | kind` only — DESIGN_SYSTEM §9.2
  "real shell icons first-class for files" cannot be met for files (badges are the fallback today — correct given the
  contract);
- history clear: no command — SCREENS §3.4 "Clear history…" shown as unavailable (`ActivityView.tsx:165-168`);
- folder picker, disk size, updater/logs/export/model-cache: no commands — SCREENS §3.3/§3.6 partially unimplemented
  (Indexes honestly says "native folder picker is not available", `IndexesView.tsx:428`).
**Fix:** choose per item — extend `ps-ipc`/`main.rs` (progress + clear-history + file-icon are the high-value three)
or amend SCREENS to mark them P1/"not in this build" (and keep the honest copy). The docs are the plan of record; they
must not promise what the contract cannot supply. **Decision (2026-11-09): recorded as alpha limitations with the
truthful-UI rule in SCREENS §10 (progress %, clear-activity, quicklinks CRUD, folder picker, updater, preview payload).**

**M10 — Launcher/preview relative times bypass `Intl`.**
`lib/format.ts:14-32` (`timeAgo`) returns hard-coded English "Just now", "min ago", "Yesterday", "days ago" and is
used by launcher rows and preview (`LauncherSurface.tsx:151-160`, `ResultPreview.tsx:113-116`) while the app already
has a locale-aware implementation (`app/describe.ts:279-304`, `Intl.RelativeTimeFormat`). DESIGN_SYSTEM §4.4 and
KEYBOARD §7 require `Intl` for dates. **Fix:** move the launcher/preview to `describe.relativeTime` (or move that
helper to `lib/` and re-export) and delete the duplicate English formatter.

### 2.3 Minors

| ID | Finding | Fix |
|---|---|---|
| m1 | Launcher footer adds "{n} items" after the status (`LauncherSurface.tsx:955-962`, `launcher.css:594`) — not in SCREENS §2.5 and competes with the match count. | Remove, or move into the status `title`/tooltip. |
| m2 | Launcher brand mark is 26 px at all widths (`LauncherSurface.tsx:767`); SCREENS says 29 px desktop, 26 ≤700. | Pass 29 and shrink by CSS/media in `launcher.css`. |
| m3 | Onboarding h1 focus ring is deliberately removed (`onboarding.css:144-148`) while app route h1 shows one (`desktop.css:180-183`). | Pick one rule and document it; heading focus is programmatic, so a ring is optional — the inconsistency is the issue. |
| m4 | Resume state is `localStorage` only (`OnboardingView.tsx:78-85, 116-126`; `logic.ts:231-236`), not `OnboardingState`; no "Setup saved — you can close this window and continue later." line (SCREENS §4.9). | Keep localStorage (contract has no per-step state) but add the resume notice and state the mechanism in SCREENS §4.9. |
| m5 | Inline confirm bars instead of the specified modal dialogs (`IndexesView.tsx:336-366, 581-609`; `ui.css:400-419`) — focus restore and Esc are implemented, but KEYBOARD §1's "Confirm dialog" contract is not what ships. | Amend KEYBOARD §1/SCREENS to the inline confirm pattern (it is keyboard-complete), or make them dialogs. Do not leave the spec silent. |
| m6 | Home shows zeroed metric rows *and* the "Nothing indexed yet." empty block (`HomeView.tsx:151-192`) — duplicated zero state. | Render the empty block instead of the metric list when `total === 0`. |
| m7 | Home adds a "Sources" section not in SCREENS §3.2 (`HomeView.tsx:194-233`). | Keep it (master prompt §V.B supports source management) and add it to SCREENS; or remove for spec purity. |
| m8 | Settings label column is 200 px (`desktop.css:788-793`) vs the reference-style 450 px label + panel in SCREENS §3.6/§3.5. | Either widen at ≥1100 or amend SCREENS; current 200 px reads as a normal settings list, not the reference's wide-label composition. |
| m9 | Globally suppressed context menu in the launcher root (`LauncherSurface.tsx:763`) also blocks right-click paste/copy in the search input and preview text. | Scope the prevention to result rows only. |
| m10 | Duplicate brand announcement: `Logo` always has `aria-label="ProtonSearch"` (`Logo.tsx:15`) beside visible "ProtonSearch" text (`DesktopApp.tsx:84-86`, `OnboardingView.tsx:592-593`). | When adjacent text names the product, render the mark `aria-hidden="true"`. |
| m11 | Theme flash: `main.tsx:29` applies the OS theme before settings resolve; a user-chosen opposite theme flashes. | Keep a boot class/transparent surface until `applyChrome` runs (BootScreen already covers the app; the launcher returns `null` — `App.tsx:236-238`). |
| m12 | Tray label hard-codes "Search (Alt+Space)" even after a custom hotkey (`main.rs:115`) — violates truthful UI copy. | Build the label from `settings().hotkey` or use "Search…". |
| m13 | `App.tsx:229-233` uses `.launcher-shell`/`.main-shell` wrappers that have no CSS (`styles/shell.css`), so the fatal screen has no window-appropriate surface in the launcher. | Add the two rules (transparent for launcher, ground for main) or drop the classes. |
| m14 | `index.html` has no `<meta name="theme-color">` (DESIGN_SYSTEM §3.5). | Add it and update it with the resolved theme. |
| m15 | Activity filters are not URL/deep-link state (`ActivityView.tsx:64`, `useState`), contrary to SCREENS §3.4. | Sync to `?filter=` or amend the spec. |
| m16 | Activity rows are actionable only for `kind === "open"` with a path (`ActivityView.tsx:113-145`); searches cannot be replayed. | Accept and note, or add "Run search again" for `search` rows. |

### 2.4 Nits

| ID | Finding | Fix |
|---|---|---|
| n1 | Selection transition uses `--ps-ease-linear` (`launcher.css:146-149`); DESIGN_SYSTEM §6 specifies standard easing for state colour. | Token swap to `--ps-ease-standard`. |
| n2 | Group/section labels use `--ps-ink-2` (`base.css:186`, `desktop.css:252, 804`, `ui.css:59`) where DESIGN_SYSTEM says `--ps-ink-3` (contrast is fine either way). | Choose one tier and align docs/code. |
| n3 | Icon stroke is 1.6 (`ui.tsx:173`); DESIGN_SYSTEM §9.1 says 1.5. | Set 1.5. |
| n4 | Segmented controls are 24 px high (`ui.css:87, 123`) — meets the 24 px AA floor but is below the design's 32 px control intent. | Raise to 28–32 px for pointer comfort. |
| n5 | Launcher search row stays 72 px at ≤700 (`launcher.css:716-724`); SCREENS §6.1 says 61 px. | Covered by M3 decision. |
| n6 | `tauri.conf.json:5` identifier `com.protonssearch.v2` has a double "s"; affects installer/updater identity. | Correct before first signed build/updater metadata. |

### 2.5 What the implementation does well (keep it)

- Honest states everywhere: "not available in this build", truthful engine labels, no fabricated percentages, no fake
  badges (`IndexesView.tsx:484, 428, 755`; `WorkflowsView.tsx:227-241`; `describe.ts:26-79`).
- No gradients/glow/glass/shadows at rest; token-only colours in feature CSS (grep of `apps/desktop/src` found
  literals only in `tokens.css`, `base.css` primary button and the brand SVG).
- Launcher keyboard model and stale-protection are real and unit-tested (`launcherLayout.ts:186-212`,
  `LauncherSurface.tsx:332-344, 492-499`; `launcherLayout.test.ts:123-160`; `launcherSurface.test.tsx:377-423`).
- Live count is throttled (`LauncherSurface.tsx:105-135`), visual count is `aria-hidden` with an sr-only status —
  this is exactly the pattern the a11y doc asks for.
- Visual/list order and keyboard order agree (`LauncherSurface.tsx:244-260`), fixing an easy regression class.
- Skip link, roving-tabindex sidebar, role=switch, radiogroup/aria-checked, filter group/aria-pressed, route h1 focus,
  `document.title`, forced-colours blocks, reduced-motion rules: all present (`DesktopApp.tsx:65-139`, `ui.tsx`).
- The brand mark is now a true cut-out and theme-tokenised (`Logo.tsx:19-37`).

---

## 3. Keyboard / accessibility conformance checklist

Based on reading the current code (not runtime testing). "Evidence" is the code that must be exercised.

| # | Item (KEYBOARD_AND_A11Y §11) | Verdict | Evidence / gap |
|---|---|---|---|
| 1 | Keyboard-only journeys complete | **FAIL** | Multi-select missing (M1, `LauncherSurface.tsx:622-629`); app/onboarding text inputs have no visible focus (B1). Everything else (summon→type→arrows→Enter, Esc chain, preview, menu, nav) is implemented. |
| 2 | Every focus stop visible at 100–200 % & HC | **FAIL** | `base.css:52-54` overrides `:focus-visible` for all inputs except `.search-input` (`launcher.css:65-69`); onboarding `.field` (`onboarding.css:257`) and app `.input` (`base.css:144`) have no replacement. |
| 3 | Focus traps: launcher / dialog / menu | **PASS (launcher/menu) · N/A (dialogs)** | Launcher Tab wrap input↔footer (`LauncherSurface.tsx:700-706`, Shift+Tab at `:631-637`) and menu Tab-closes/Esc-returns (`ResultActions.tsx:64-100`) are implemented and tested (`launcherSurface.test.tsx:336-375, 622-637`). No modal dialogs exist (inline confirm bars, m5) so the dialog trap is untested/unspecified. |
| 4 | Stale-selection protection | **PASS** | epoch+requestId+committed guard (`LauncherSurface.tsx:332-344, 492-499`, `commitSelection` `launcherLayout.ts:199-212`); tests `launcherSurface.test.tsx:377-423`. |
| 5 | Rapid typing / no queued lag | **PASS (code) · NOT VERIFIED (E2E)** | No debounce, selection state synchronous, 20-arrow test exists (`launcherSurface.test.tsx:671-684`); frame/latency measurement belongs to Gate 3 runtime evidence. |
| 6 | Multi-select keyboard-only | **NOT IMPLEMENTED** | No state or tests; Shift+Enter is consumed but does nothing (`LauncherSurface.tsx:622-629`). |
| 7 | Reduced motion | **PASS (code)** | `base.css:193-209` + `tokens.css` `[data-motion="reduced"]` zero durations + media query; onboarding step animation is covered. |
| 8 | High contrast / forced colours | **PARTIAL** | Launcher (`launcher.css:757-793`), onboarding (`onboarding.css:582-589`), shared bits (`ui.css:499-507`) and `dot` border exist; app nav active/switch/segmented states have no HC-specific mark, and inputs have no visible focus in any mode. |
| 9 | Target sizes ≥24 px | **FAIL** | `.linkbtn` ≈16 px (`ui.css:239-247`); switch 36×20 (`ui.css:139-143`); segmented 24 px is acceptable; rows/buttons/icon buttons pass. |
| 10 | SR sessions recorded (Narrator/NVDA scripts) | **NOT DONE** | No evidence in `docs/qa` yet; scripts in KEYBOARD §10 are ready to be executed. Cannot pass by code reading. |
| 11 | Contrast audit | **FAIL** | Three concrete fails: selected-row meta 4.20:1 (M5.1), segmented/filter selected 1.10:1 + border 1.24:1 (M5.2), switch boundary 1.07–1.46:1 (M5.3), nav active 1.13:1 (M5.4). Token `--faint` 3.04:1 is unused but should be retired (§1.1). |

---

## 4. Reference-fidelity comparison per screen

Method: compare the DOM/CSS structure and metrics against `protonsearch-v2-ui-reference.html` (tokens 12-15,
launcher 27-50, app 51-80, onboarding 81-100, responsive 94-101, mark 118) and the accepted screens. Current build was
not screenshotted in this pass (no GUI run); these are structural comparisons the visual QA must confirm.

**Quick Launcher.** Structurally faithful: frameless surface with radius 14, 1 px `#cdd5ce`-family border, light
shadow token, 72 px search row with the P at the start and a 21 px input, grouped 58 px rows (36 px icon tile,
13 px/680 title, 11 px path, 10 px trailing meta), a 44 px footer with status dot + "Open ProtonSearch" + three
right-aligned kbd hints, and no title bar/Ctrl+K/tabs. Divergences: brand mark 26 px instead of 29; preview side
width 380/1220 instead of the 360/1180 token; footer adds a "{n} items" segment; group/section label ink is one tier
darker than the reference; the added 2 px selection bar and input focus ring are intentional accessibility
corrections (documented in DESIGN_SYSTEM §2.3/§3.3). The search/kbd/footer copy matches; the reference's sample
"3 matches" behaviour is preserved via a real engine count.

**Home.** Faithful hierarchy: uppercase eyebrow, clamp(26–41 px)/630/`-0.065em` greeting as h1, one 13 px lead, a
48 px raised "Find a file, open an app, or run an action…" entry with search/↗ glyphs, the same 1.25/0.9 two-column
composition at 38 px gap, 3-row "Recently accessed" block with a secondary "Manage sources", the 34 px/680
`-0.075em` index stat with subline and metric rows. Deviations: no 4 px progress hairline (contract has no percentage,
M9); adds a status dot+word beside "Your index" and a third "Sources" section (not in the reference or SCREENS §3.2);
zero state doubles up (metric zeros + empty block). The big search no longer carries a hotkey chip (good — the
reference did not have one).

**Search & indexes.** The reference's page mock is intentionally replaced by a real management surface; the visual
system (eyebrow/h1/lead/11 px caps section labels/1 px hairline panels/right-aligned tabular values/quiet segmented
depth control) reads as the same product. Fidelity concerns are composition, not style: six+ stacked panels and 200 px
label columns make it denser and more "settings list" than the reference's single info-panel card; the honest
"native folder picker is not available" line is the right product call but makes the Locations panel look like a dev
tool. No gradients/shadows/dashboard clichés.

**Activity.** No reference screen exists; production composes from the reference's row grammar (icon tile + 12 px
title + 11 px sub + quiet time) inside a privacy strip + filter bar + day-grouped list. It matches the design system
and does not drift into a dashboard. Missing from SCREENS: clear-history control (M9, honestly disclosed), URL-synced
filters (m15), and replay for search rows (m16).

**Workflows.** Same judgement: reference mock replaced by three to four panels (Native commands, Agent runtime,
Permissions, Quicklinks). The command rows introduce a right-hand pill on every row ("Built in" / "Confirmation
required") — the reference has no pill language at this density and the two-tone pill repetition reads noisier than
the rest of the app; recommend keeping the warning pill only where confirmation is required and letting the default
state be silent. The agent/off/permission copy is exactly the honest, non-advertising framing the master prompt
demands.

**Preferences.** The reference settings grammar (caps label column + panel; 12 px rows; right-aligned control) is
preserved, with a narrower 200 px label column and a real autosave + "Saved" stamp. The hotkey row is the one place
the reference-derived idea breaks: a typed accelerator string instead of a capture control (M2). Diagnostics is a
long real-value list rather than the reference's three mock rows — acceptable because it is real, but it is the one
area that reads like a developer panel; grouping/collapsing the low-frequency rows would restore the reference's
restraint.

**Onboarding.** The closest match to the reference: 240 px tinted rail with the 29 px mark and wordmark, numbered
25 px step circles (current filled), 240→content split, 32 px/600 step h1, one 13 px lead at 420 px, choice rows with
right-hand descriptions, and the reference's "No account required. Your files remain on your PC." rail footnote. Seven
steps and per-root depth segmented controls are deliberate scope changes (master prompt §V.C; SCREENS §4). Deviations
to resolve: rail hides at 760 in code (final contract 700); h1 focus ring suppressed unlike the app route heading; the shortcut step uses a
typed field (M2); there is no visible "setup saved/resumable" line; the legacy-data notice is a good truthful addition
not present in the reference.

---

## 5. Acceptance screenshots required for Gate 3 / Gate 4

Use the mock entry (`?view=launcher|main`) for UI-only states and the real app + CDP harness
(`PROTONSEARCH_CDP_PORT`, `main.rs:165-173`) for window/DPI/latency evidence. Store under `docs/qa/visual/` per
WORK_BREAKDOWN (ux-qa). Every image: full window, no crop that hides chrome, filename pattern
`{surface}-{state}-{width}-{dpi}-{theme}`. States marked **(alpha)** must show the truthful limitation (omitted
control or plain "not available in this build" copy), never a dead control.

| # | Surface | State | Width(s) | DPI | Theme | Notes |
|---|---|---|---|---|---|---|
| 1 | Launcher | first summon, focused input, empty query | 820 | 100 | light | no flash, no title bar |
| 2 | Launcher | empty input with RECENT group (if recents) | 820 | 100 | light | |
| 3 | Launcher | 0 matches (exact copy) | 820 | 100 | light | |
| 4 | Launcher | 1 match | 820 | 100 | light | |
| 5 | Launcher | many results + selected row | 820 | 100 | light | ACTIONS last; **no multi-select affordance** (CUT) |
| 6 | Launcher | first-query loading (3 skeletons) | 820 | 100 | light | slow mock |
| 7 | Launcher | preview side panel | 1180 window, 360 panel, work area ≥1200 | 100 | light | Ctrl+Space |
| 8 | Launcher | preview bottom drawer | 820 window, work area <1200 | 100 | light | |
| 9 | Launcher | context menu open | 820 | 100 | light | keyboard-opened |
| 10 | Launcher | index error + Retry | 820 | 100 | light | footer "Index unavailable" |
| 11 | Launcher | indexing / paused footer | 820 | 100 | light | two captures; **no percentage** (alpha) |
| 12 | Launcher | dark: summon, selected, 0-match, preview | 820 / 1180 | 100 | dark | on-dark brand mark |
| 13 | Launcher | 560 / 820 / 1000 / 1280 window widths | listed | 100 | light | clamp behaviour |
| 14 | Launcher | 125 / 150 / 200 % DPI at 820 | 820 CSS | 125/150/200 | light | work-area clamp at 200 |
| 15 | Launcher | high-contrast (Black + Aquatic), selected + focus | 820 | 100 | HC | |
| 16 | App | Home loaded (recent + index + sources) | 820 / 1000 / 1280 | 100 | light | |
| 17 | App | Home loading, empty index, index error | 1000 | 100 | light | 3 captures; **no progress bar** (alpha) |
| 18 | App | Home incomplete-setup notice (skip fixture) | 1000 | 100 | light | |
| 19 | App | Search & indexes: locations (paste-path form), depth | 820 / 1280 | 100 | light | **no folder picker** (alpha) |
| 20 | App | Search & indexes: status/indexing/paused + rebuild confirm | 1280 | 100 | light | **no percentage, no index size** (alpha) |
| 21 | App | Sources with one error + cloud/unavailable root | 1280 | 100 | light | truthful error copy |
| 22 | App | Activity: list, empty, paused, filtered-empty, error | 1280 | 100 | light | 5 captures; **no clear control** (alpha) |
| 23 | App | Workflows: off/on, agent error, quicklinks note | 1280 | 100 | light | **no setup/start/stop, no quicklinks editor** (alpha) |
| 24 | App | Preferences: shortcut, appearance, privacy, indexing, agent, diagnostics | 1280 | 100 | light | diagnostics read-only; **no update/logs/export controls** (alpha) |
| 25 | App | Preferences: hotkey field validation; theme/density applied | 1280 | 100 | light/dark | |
| 26 | App | minimum window **560×480**, 820, 1000, 1280; sidebar rail ≤819 | listed | 100 | light | proves the final window contract |
| 27 | App | dark theme on every route | 1280 | 100 | dark | |
| 28 | App | 125/150/200 % DPI on Home + Preferences | 1000 CSS | 125/150/200 | light | |
| 29 | App | high-contrast Black/White on Home/Preferences/sidebar | 1280 | 100 | HC | focus + selected + nav bar |
| 30 | Onboarding | all 7 steps | 900×620 | 100 | light | one image each |
| 31 | Onboarding | step 03 resumed after close/reopen | 900×620 | 100 | light | proves resumability |
| 32 | Onboarding | dark theme step 1 + step 4 | 900×620 | 100 | dark | |
| 33 | Onboarding | rail at **701 (visible)** and **700 (hidden)** | 701 / 700 | 100 | light | final breakpoint |
| 34 | Comparisons + alpha sheet | reference HTML vs production (launcher, Home, onboarding; light + dark); plus one capture per SCREENS §10 item proving no dead control/fake percentage | 1280 | 100 | both | master prompt §XI.3 |
| 35 | Motion | reduced-motion on: launcher selection, preview, onboarding step | — | 100 | light | video/GIF or before/after pair |

User-visual pass (2026-11-09) adds these captures beyond the 35: launcher shadow on a light and a dark wallpaper at
100 % and 150 % (four images, no cut-off edge); launcher open and close frames mid-animation (two images or a short
clip) plus the reduced-motion instant; filter strip at rest/selected/focus at 560 and 820, light + dark, plus the
scoped 0-match copy and a long localised label proving no truncation.

Mandatory master-prompt §V.D states are covered by rows 1–15/16–33 (first summon, empty, 0, 1, many, selected, preview,
loading, index-error, dark, each onboarding step, Home, settings, narrow width, 125/150/200 %, high contrast, keyboard
focus). Clipboard multi-select is intentionally **not** in the manifest — row 5 exists to prove its absence.

---

## 6. Document amendments applied (2026-11-09)

| Doc | Change applied |
|---|---|
| `SCREENS.md` | §2.1 height formula now includes the 44 px footer (matches B2 fix); §2.2 ACTIONS pin CUT; §2.5 multi-select hints removed; §2.6 preview 1180/360/≥1200 + metadata-only payload; §2.7 rewritten as CUT; §2.9 chord removed; §1/§6 window minimum **560×480**; §3.1 nav-active 2 px bar; §3.2–§3.6 progress %, clear-history, quicklinks, picker, updater, preview payload recorded as alpha limitations; §4.1 rail **≤700**; §4.4 paste-path; §4.8 no fake progress; §6.1 bands fixed (rail visible 701–819); new **§10 Alpha scope**; copy deck post-alpha rows; acceptance renumbered §11 and updated. |
| `DESIGN_SYSTEM.md` | §3.5 nav-active 2 px bar; §3.6 final preview/nav/window/alpha decisions + `--faint` decision; §8 Sidebar row, LauncherFooter hints/status, ProgressBar (not rendered in alpha), StatBlock, ResultRow multi-select CUT, PreviewPanel alpha payload; token block nav-bar comment. |
| `KEYBOARD_AND_A11Y.md` | §2 multi-select rows replaced by one "unbound — CUT" row; §2.1 Esc chain updated; §5 rewritten as CUT with re-entry; §7 listbox single-select + progress post-alpha; §10.2 Home SR line; §11 item 6 N/A; §12 limitation 3 updated. |
| `REVIEW_1.md` | This decisions log (§0.1), decision notes on §1.1/§1.2/M1/M3/M4/M9, corrected 35-shot manifest (§5), this amendments table, verdict update. |

Remaining **code** work is unchanged: B1, B2, M2, M5 (except nav bar), M6, M7, M8, M10, M3 code alignment, plus the
minors/nits. The CUTs must be enforced by tests (absence, not half-build).

---

## 7. Verdict

Gate 3 (integrated launcher) **cannot be accepted** until B1, B2, M1, M2 and M7 are resolved or explicitly waived with
evidence; M5/M6 must be fixed before the a11y audit can pass. Gate 4 should not start visual sign-off until M3
(window/breakpoint contract) and M9 (IPC-vs-spec capability decision) are settled, because both change what the
screenshots are supposed to show. The remaining findings are review-fix level and can trail the gate with a written
waiver. The visual direction, honesty of states, and keyboard model are on the right track — this review is mostly
about closing contract gaps, not redesigning.

**Decision status (2026-11-09):** M1 and the ACTIONS-pin half of M4 are closed as CUT for v2.0 alpha; §1.2, M3 and M9
are closed in the docs (final preview geometry, 560×480 / ≤700 window contract, alpha limitations) and remain code
tasks where noted. The blockers B1/B2 and majors M2/M5/M6/M7/M8/M10 are unchanged code work. With those fixed, the
Gate 3 launcher is acceptable against this review.

---

## 8. Live-capture review — user screenshots (2026-11-09)

**Evidence:** `docs/qa/visual/app-captures-release/live-now.png` and `live-1-1195x837.png` (both 1195×837 desktop
captures), plus zoomed crops and pixel sampling (scratch: `.agents/notes/crops/`). The user-reported issues are
addressed by the design changes above; the remaining observations are listed here so L0/ux-qa can verify them on a real
screen.

### 8.1 User-reported issues → resolution

| Report | Root cause | Resolution |
|---|---|---|
| "That shadow behind it doesn't look proper" | The launcher token was a single `0 24px 56px` layer: extent ≈108 px inside a 28 px transparent window pad, so the native window clipped it into a hard square edge; the card read as flat, especially on dark. | New layered 4-stop token with extent ≤24 px inside the 28 px pad, plus a 1 px rim on dark (DESIGN_SYSTEM §5.4). CSS + layout contract specified; ui-launcher replaces the token/usage. |
| Open/close motion needs art direction | There was no launcher motion at all: CSS said show/hide instant, Rust emitted `launcher:dismiss` but no frontend listener existed, and the only close was a 400 ms safety hide. | Open 150 ms `cubic-bezier(0.16,1,0.3,1)` (fade + 8 px rise + scale .985→1); close 110 ms `cubic-bezier(0.4,0,0.2,1)` (fade + 4 px rise + scale 1→.99); input/selection/results stay instant. Exact CSS + shell/JS contract in DESIGN_SYSTEM §6.3/§6.4, SCREENS §2.10. |
| Horizontal category/filter strip must be specified | No filter UI existed in the launcher. | Full spec added: SCREENS §2.11 (40 px block, 26 px chips, 9 filters, All first, selected = pale fill + 2 px accent bar, radiogroup keyboard model, overflow scrolling, localisation rules, scope mapping) + DESIGN_SYSTEM §8 FilterStrip + REFERENCE_MAPPING `.segments` → FilterStrip. Geometry formula updated to include the 40 px strip. |

### 8.2 Further defects found in the captures

| ID | Severity | Finding | Evidence | Exact fix |
|---|---|---|---|---|
| L1 | **High** | **Stale legacy app icon.** The native title bar shows the old silver/red "S" swirl, not the ProtonSearch P. The same asset is checked in for the app icon. | `titlebar-now-3x.png`; `apps/desktop/src-tauri/icons/32x32.png` is the legacy swirl (matches the crop). | Generate `icon.ico` + 16/20/24/32/48/64/128/256 PNGs and tray/notification sizes from the brand assets (`assets/brand/protonsearch-mark*.svg`). Use the primary two-tone on light and the on-dark variant when the app theme is dark (regenerate via `window_icon()` on theme change); never ship the legacy "S". Verify 16–32 px on light and dark title bars + taskbar. |
| L2 | **High** | **Launcher shadow clips** (user report). Token extent ≈108 px vs 28 px pad → hard-cut shadow; no visible ambient falloff. | DESIGN_SYSTEM §5.4 math; legacy token still in `tokens.css:104` (light) / `:293` (dark). | Replace with the layered token in §5.4; keep `LAUNCHER_SHADOW_PAD = 28`; root padding 28 + card `height:100%`; no native shadow; verify on a light and a dark wallpaper at 100/150 %. |
| L3 | **High** | **Launcher open/close is unimplemented.** `launcher.css:18` still says show/hide instant; no frontend listener for `launcher:dismiss` (grep of `apps/desktop/src`); only Rust's 400 ms safety hide closes. | Code evidence above. | Implement DESIGN_SYSTEM §6.4 (`data-state` opening/closing + 150/110 ms), listen for `launcher:dismiss`, hide on `animationend` + 200 ms fallback; re-summon cancels close. Rust safety hide must be generation-cancellable (L0). |
| L4 | **High** | **Filter strip missing** in the shipped launcher (no chips/category UI). | `LauncherSurface.tsx` has no filter/scope UI; `lib/results.ts` only groups results. | Build SCREENS §2.11: component + `scope` on `SearchRequest`, geometry +40 px, keyboard/ARIA, localised labels. |
| L5 | **Medium** | **App window can exceed/overflow the captured work area.** Both live captures show the window's right and bottom edges (caption buttons, onboarding step 7, h1 tail) outside the 1195×837 frame. The window is fixed 1180×800 with `.center()` only — no work-area clamp. If the capture is a desktop grab (not a crop), ~15–20 % of the window is unreachable. | `main.rs:221-224`; both screenshots; pixel scan shows the window frame at x≈190 with content continuing past 1195. | Clamp initial and restored size to the monitor work area: `w = min(1180, wa.w − 48)`, `h = min(800, wa.h − 48)`, centre within it, `min_inner_size 560×480`; re-verify after a DPI change. If the capture is a crop, record that and re-capture full-window evidence. |
| L6 | **Medium** | **Dark window/card elevation is missing.** Onboarding card `#1c1e1a` on ground `#141513` = 1.09:1 with no shadow/border; ~120 px empty bands above/below the card make it read as a hole in a black window. | `live-now.png` (card at y≈305, large void), `live-1`; pixel sampling (20,21,19) ground vs card. | Onboarding stage: `padding: 24px`; card gets `border: 1px solid var(--ps-line)` and a clipped-safe card shadow `0 2px 6px rgba(0,0,0,.30), 0 8px 16px rgba(0,0,0,.28)` (extent 24 ≤ stage padding 26), and `min-height: min(620px, calc(100vh − 48px))` so it fills the window instead of floating in a void. Light theme uses `0 2px 6px rgba(36,60,43,.08), 0 8px 16px rgba(36,60,43,.06)`. No gradient. |
| L7 | **Low** | Title-bar caption text is OS grey and the icon nearly disappears on the dark title bar; it reads disabled. | `titlebar-live1-3x.png`. | Native frame keeps OS styling; the icon fix (L1) is the actionable part. If a branded caption is wanted, it requires custom decorations — out of scope for this pass. |
| L8 | **Low** | Onboarding h1/text clipping at the right edge in the captures is a consequence of L5 (or of the capture crop); re-verify after the clamp. | `right-clip-now-3x.png`. | No separate fix; verify with a full-window capture at 1195×837 and 150 % DPI. |

**Not defects:** the browser window overlapping ProtonSearch in `live-1` is another app; the white strip above the
frame in `live-now` is the app behind; the in-app brand mark and the launcher card itself look correct at this scale.

**Status:** L2/L3/L4 are specified in DESIGN_SYSTEM §5.4/§6 and SCREENS §2.10/§2.11 (ui-launcher can implement
directly); L1 and L5/L6 need L0/ux-qa action (assets + Rust window clamp) and are recorded against this review.
