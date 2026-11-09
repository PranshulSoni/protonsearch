# REVIEW_2 — Task-7 app readability / alignment / quiet-state implementation review

**Reviewer:** L1-B Product Design & Experience (`design-spec`). **Date:** 2026-11-09, snapshot ~23:52.
**Subject:** ui-app’s task-7 implementation against the final task-8 contract (DESIGN_SYSTEM §4.2b, §5.5, §8.3;
SCREENS §3.0, §3.7). **Scope:** review only — no code edits.
**Evidence:** current source; independently re-ran `npm test` — **10 files / 167 tests passed** (23:51);
reviewed `docs/qa/visual/VISUAL_AUDIT_APP.md` §11 and the task-7 captures (home light/error-home/error-indexes at
1180×800, canary home 560×480 @200 %, onboarding light 900×660).

**Verdict: ACCEPT WITH FOLLOW-UP.** The core deliverable is real and visually much stronger: the app-scale token block
is exact, the grid follows the contract, the quiet-state copy map is largely implemented, and the canary holds. Two
major follow-ups (raw engine/source detail still reachable on the surface; the offline/empty component variants and
audit claim are incomplete) plus one CSS severity bug must be fixed before quiet states are called done. No blockers.

## 1. What was verified as correct

| Contract item | Evidence |
|---|---|
| App type scale block exact, main-window scoped, launcher untouched; media + compact included | `apps/desktop/src/app/app-scale.css:5-65` matches DESIGN_SYSTEM §4.2b verbatim; imported by `desktop.css:1` and `onboarding.css:1`; `tokens.css` has no `data-window` block (still TEMP). |
| Grid: content 1120, margins, 12-col gutter, sidebar 232/200/64, Home 7/5, settings 280+24, panel padding 24, nav/list/metric/control/field 44/68/44/36/40, home search 52 | `desktop.css:26,190-194,288-292,855-860`; `ui.css:34-40,78-96,147-157`; verified visually in the 1180 captures. |
| Page-header rhythm eyebrow 8 / h1 display / lead 15-24 / content | `desktop.css:196-248`. |
| Quiet state anatomy: surface + 1 px line + 20 px icon + title 15/600 + body 14/21 max 60 ch + one action + native Details | `ui.tsx:378-425`, `ui.css:536-606`. |
| Copy map largely implemented: index load/warnings/stopped, save failure, hotkey invalid/unavailable, activity load/open, agent off/unavailable, source/network rows, onboarding finish, empty states | `HomeView.tsx:80-98,139-145,163-169,178-187`; `IndexesView.tsx:566-586,766-774,792-815`; `ActivityView.tsx:169-235`; `PreferencesView.tsx:138-145,503-509`; `WorkflowsView.tsx:156-164,217-252`; `HotkeyInput.tsx:178-200`; `OnboardingView.tsx:645-653`. |
| Warning aggregation sanitized (count only, no paths) | `describe.ts:348-361`; tested in `describe.test.ts`. |
| Errors no longer use raw `errorMessage` in views; helper is unused | `hooks.ts:24` is the only remaining reference. |
| Canary 560×480 @200 %: no horizontal overflow, minimum text 11 px | audit §11.4 + `task7-canary-home-560x480-dpi2.png` (1120×960 physical = 560×480 CSS @2×); code has no app text <11 px. |
| Onboarding: h1 34 / lead 15 / max 480, card min-height min(620, 100vh−48), stage 24 | `onboarding.css:10-21,137-169`; `task7-onboarding-light-900x660.png`. |

## 2. Follow-ups (findings)

### F1 — MAJOR: raw engine/source detail still reaches the surface
The audit's “raw engine strings no longer rendered anywhere” is not true for detail fields:
- `WorkflowsView.tsx:209` renders `{agent.detail}` as `config-desc` for any non-off agent state; `describe.ts:420`
  passes through `agent?.detail` (e.g. “Sidecar exited with code 1”, provider paths) when the engine supplies it.
- `IndexesView.tsx:772-774` renders `{source.detail}` directly as field-help; `describe.ts:154` also feeds
  `source.detail` into `describeSource(...).description`, which is rendered at `IndexesView.tsx:765`.
Both violate SCREENS §3.7 (no raw exception/engine strings, codes or paths on the surface) and contradict the audit
claim. **Fix:** stop rendering these raw values. Keep the product copy from §3.7; if diagnostics are needed, sanitize
(strip absolute paths, stack prefixes and codes) and pass to `StateNotice details` only. Add a unit test with a
poisoned detail containing `C:\\Users\\...` that asserts it never appears in the DOM. Re-audit the claim.

### F2 — MAJOR: `.state--offline` / `.state--empty` variants are not implemented (and audit overclaims them)
DESIGN_SYSTEM §8.3 and SCREENS §3.7 define `.state--offline` (cloud-off icon, neutral border) and `.state--empty`
(same block anatomy); the audit §11.3 claims both exist. In code:
- `StateTone` is only `neutral | warn | error` (`ui.tsx:371`) and the class is always `state--<tone>`
  (`ui.tsx:399`); `ui.css` has no `.state--offline` or `.state--empty` rule.
- Empty states still use the old `.empty-block` (`ui.tsx:427-443`; `ui.css:364-392`): radius 8 and padding 16 versus
  the state contract’s 12/24, no optional 20 px icon, and visually a different component from error notices.
- Offline paths (network root, provider) use `row-state`/`warn` copy; there is no cloud-off state or “You’re offline.”
  block as specified.
**Fix:** add `offline` to `StateTone` (cloud-off icon, neutral border), add `.state--offline` and `.state--empty`
CSS, and either migrate `EmptyState` onto `.state--empty` or restyle `.empty-block` to the same metrics
(radius 12, padding 24, 20 px optional icon, title 15/600, body 14/21 max 60 ch). If the alpha intentionally defers
these variants, amend DESIGN_SYSTEM §8.3 + SCREENS §3.7 and correct audit §11.3 instead of claiming they exist.

### F3 — MEDIUM: severity icon colour never applies (selector typo)
`ui.css:612` and `ui.css:620` target `.state-warn .state-icon` / `.state-error .state-icon`, while the DOM emits
`state--warn` / `state--error` (`ui.tsx:399`). Result: warning and error icons always paint `--ps-ink-2` — visible
in `task7-error-home-light.png` (grey triangle in a rose-bordered error notice). The border colour is the only
severity cue. **Fix:** `.state--warn .state-icon { color: var(--ps-warning) }` and
`.state--error .state-icon { color: var(--ps-negative) }`; add a visual regression for the two tones.

### F4 — MEDIUM: copy-map gaps vs SCREENS §3.7
- Provider-unreachable row is not implemented (“Can’t reach the provider.”); no provider can be configured in this build,
  so mark it post-alpha in the docs (done in this review pass) or implement when the Pi panel lands.
- Network-root row has no “Try again” action (no IPC command); SCREENS now notes “post-alpha; alpha shows the reconnect
  copy only”.
- Home paused state uses “Indexing is paused.” / “New files aren’t being added. Existing search still works.”; not in the
  §3.7 canonical table — add it to the map (or align with “Indexing stopped.” only for `error`).
- Workflows “Agent workflows aren’t set up.” is informational copy not in the map; add it as an alpha-limitation row.
- `summarizeWarnings` details uses “item(s)” (`describe.ts:357-359`); use `Intl.PluralRules` (“1 item” / “2 items”).
- The onboarding legacy panel prints the absolute `legacy.appDataPath` (`OnboardingView.tsx:303`) on the surface; not an
  error path, but for professional polish show a “Local data folder” label with the path in a tooltip/Details instead.

### F5 — LOW: Home grid rhythm is 40, not 32
`desktop.css:292` sets `.home-columns { margin-top: var(--space-8) }` (40 px) while `.page-lead` already has a
32 px bottom margin (`:242-248`); adjacent margins collapse to 40, so the page-header → content rhythm is 40 not the
contract’s 32. **Fix:** `margin-top: 0` (rely on the lead margin) or 32; verify the notice variant too.

### F6 — LOW: onboarding step rail metrics drifted from DESIGN_SYSTEM §8 row 27
`onboarding.css:83` still uses a 25 px step circle (the updated component row says 28 px / 56 px pitch). Visually fine,
but it is an undocumented contract drift. **Fix:** set 28 px circle / 56 px pitch, or amend the DESIGN row back to
25/52 with a note. Also the file header comment still says “hidden <=760” while the media query is 700 (`:4` vs
`:543`) — fix the comment.

### F7 — LOW: dead helper contradicts the audit
`hooks.ts:24` `errorMessage()` is now unused, while audit §11.3 says the helper was removed. Remove the export (and
its import sites/tests if any) or correct the audit.

### F8 — MEDIUM (L0-owned): bootstrap failure still exposes the raw message in Details
`App.tsx:81-101` now presents a good human FatalScreen, but `<code>{message}</code>` inside the Technical details
disclosure can still contain absolute paths/raw exception text. SCREENS §3.7 says Details must be sanitized (message +
reference id only). **Fix (L0):** sanitize the message (strip paths, stack frames, codes), or replace it with a generated
reference id + a short generic reason; keep the human title/body/action. The settings-load copy path in §3.7 remains
unused until App.tsx routes bootstrap/settings failures through it.

### F9 — NIT: generated `state--neutral` class has no rule
`ui.tsx:399` always appends `state--<tone>`; `neutral` yields `state--neutral` which has no CSS (base `.state`
covers it). Either render plain `state` for neutral or add the class to the docs; harmless today.

## 3. Documentation / evidence corrections

- `VISUAL_AUDIT_APP.md` §11.3 claims `.state--offline`, `.state--empty` and “raw engine strings no longer rendered
  anywhere”; correct these to match F1/F2 (ux-qa-owned file).
- SCREENS §3.7: provider-offline and network-root retry marked post-alpha in this review pass (two rows).
- No changes needed to DESIGN_SYSTEM §4.2b/§5.5: app-scale.css matches them exactly.
- TEMP block in `app-scale.css` must move into `tokens.css` (L0) before release; then delete the TEMP file/imports.
  Until then the duplicated values are a drift risk.

## 4. Acceptance recommendation

Task-7 is **accepted** as a strong implementation of the readability/grid/quiet-state contract. Merge-blocking for the
quiet-state claim: **F1, F2, F3** (small, contained fixes) and the audit correction. F4–F6 are follow-up polish;
F8 is an L0 copy/sanitisation task; F5/F6/F9 can trail. After F1–F3, re-capture one error and one empty state per
theme at 1180×800 and re-run the 560×480 @200 % canary.

## 5. Re-check (2026-11-10) — ui-app task-15 fixes

**Independently verified:** `npm test` → **11 files / 174 tests passed** (00:02); `npx tsc --noEmit` clean.
Reviewed the new captures: `task15-state-error-home-light.png`, `task15-state-warn-indexes-light.png`,
`task15-state-empty-indexes-light.png`, `task15-states-gallery-{light,dark}.png`; audit §11.3 correction +
§12.

**Closed**
| Finding | Verification |
|---|---|
| F1 raw engine/source detail | `describeAgent` no longer forwards `agent.detail` (product copy for running/stopped/error/not-configured, `describe.ts:418-476`); `describeSource` fallback is the generic “Reported by the local engine.” and never injects `source.detail` (`:150-156`); Indexes no longer renders `source.detail`. Poisoned-detail tests at `describe.test.ts:409-421` assert the secret/path is absent from all agent states, source descriptions and warning details. |
| F2 offline/empty variants | `StateTone = neutral | warn | error | offline | empty` (`ui.tsx:378`); cloud-off icon (`ui.tsx:414`); `.state--offline`/`.state--empty` rules with base 12 px radius / 20-24 padding / 20 px icon (`ui.css:606-622`); `EmptyState` renders `.state.state--empty` with the same anatomy (`ui.tsx:445-466`); `.empty-block` is gone from CSS and DOM. Gallery + empty-route capture confirm the visual. |
| F3 severity icon colour | Selectors now `.state--warn .state-icon` / `.state--error .state-icon` (`ui.css:590-600`); gallery shows amber warning and red error icons; `ui.test.tsx:57-69` asserts the canonical selectors and the absence of the old ones. |
| F5/F6/F9 + plural | `.home-columns` margin 32 (`desktop.css:287-292`); onboarding step circle 28 / comment ≤700 (`onboarding.css:3,83`); `.state--neutral` rule added (`ui.css:602-604`); `summarizeWarnings` uses `Intl.PluralRules` (“1 item / 2 items”, `describe.ts:356`). |
| F4 docs rows | SCREENS §3.7 now includes the paused and agent-not-set-up rows (this pass); provider/network rows already marked post-alpha. |

**Still open (non-blocking)**
- **F7 was not actually fixed:** `hooks.ts:24` still exports the unused `errorMessage()` while audit §12 says
  “Removed from hooks.ts; grep shows no usages”. Remove the function or correct the audit line. (Dead code only.)
- **F8 (L0):** `App.tsx:81-101` still shows the raw bootstrap `message` inside Technical details; sanitize it or show a
  reference id. The settings-load copy path in §3.7 remains unused for bootstrap failures.
- **Offline variant has no product call site yet** (network roots stay a `row-state`, provider-unreachable is post-alpha);
  keep `.state--offline` as the reserved post-alpha variant or wire it when per-root/provider states land. Audit §12
  already notes this honestly.
- **Nit:** `.row-state svg` uses `--ps-warning` (`ui.css:638-641`); row-level failures are error/negative in §3.7.
  Either make the auto-retry row neutral/warn deliberately and document it, or switch to `--ps-negative`.

**Re-check verdict:** F1–F3 are closed; the quiet-state family is now consistent (error/warn/offline/empty, same anatomy,
correct severity colour, no raw detail paths). Task-7 is **approved**; only the F7 cleanup/claim correction and the L0
F8 sanitisation remain, plus the TEMP app-scale block moving into `tokens.css` before release.
