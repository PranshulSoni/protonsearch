# Reference → Production Mapping

**Owner:** L1-B Product Design (`design-spec`). **Status:** binding for G3–G4.
**Reference:** `protonsearch-v2-ui-reference.html` (25 239 bytes, 159 lines) — every class and JS behaviour below is
cited by line. **Rule:** the reference is the literal visual foundation (master prompt:167); its demo chrome is not
shipped, its accessibility failures are corrected, and every deviation is listed in §4 with a reason.

Production paths are the planned component locations under the ownership map in `docs/implementation/WORK_BREAKDOWN.md`
§1 (`ui-launcher` → `apps/desktop/src/launcher/**`, `ui-app` → `apps/desktop/src/app/**` + `src/onboarding/**`).
L0's scaffold may rename files; ownership and component contracts do not change.

---

## 1. Page-level scaffolding (reference only — **not shipped**)

| Reference | Lines | Production decision | Why |
|---|---|---|---|
| `.stage`, `.outer` | 18–19 | not shipped; informs `--ps-ground #e8ebe8` | Specimen page that centres cards on a desktop-coloured stage. The real launcher is a frameless window; the app fills its window. |
| `.controlbar`, `.overline`, `.segments` + `chooseView()` | 20–24, 106–111, 132–133 | not shipped; replaced by real windows/routes (launcher window, app router, onboarding route) | This is a design-review switcher, not product navigation. Shipping it would add chrome the master prompt bans (no tabs in the launcher). |
| `.demo-label` | 26, 114 | not shipped | Honest disclaimer for a mock; production shows real data or nothing. |
| `body` typography, `button/input` reset | 16–17 | global reset + type tokens (`tokens.css`) | Keep the exact metrics: 13 px base, `-0.012em`, Inter-first stack. |
| `:focus-visible` outline | 17 | `--ps-focus #5482a3`, 2 px, offset 2 (DESIGN_SYSTEM §8.2) | Keep, contrast-verified (4.0:1 surface). |
| `:root` custom properties | 12–15 | semantic tokens (§11 DESIGN_SYSTEM), with accessible grey corrections | Reference token names are demo-local; production names are semantic. |
| `.logo` + inline SVG | 25, 118 | `BrandMark` component + `assets/brand/protonsearch-mark.svg` / `-16` / `-on-dark` / `-mono` | The referenced `assets/protonsearch-mark.svg`, `logo-concept.png`, `original-logo-crop-reference.png` do **not exist** in the workspace (`WORK_BREAKDOWN.md:104`, master prompt:11). The inline SVG is the source of truth; reconstructed vector + usage rules in `assets/brand/README.md`. |

---

## 2. Quick Launcher

| Reference | Lines | Production component (planned path) | Keep / adapt / replace |
|---|---|---|---|
| `.launcher` surface | 27–29 | `LauncherWindow` (`apps/desktop/src/launcher/LauncherWindow.tsx`) | **Keep** radius 14, 1 px `#cdd5ce` border; **adapt** the reference's single drop shadow into a layered, clipped-safe token that fits the 28 px transparent window pad (extent ≤24 px; DESIGN_SYSTEM §5.4), and use the content height formula with a 40 px filter strip (SCREENS §2.1). Frameless; no OS title bar; native window shadow off. |
| `.segments` (demo segmented row) | 22–24 | `FilterStrip` (`apps/desktop/src/launcher/FilterStrip.tsx`) | **Adapt, not copy**: keep the reference's muted language — pale selected fill, 5–6 px radius, 4–6 px gaps, 11–12 px/650 sentence case, no colour on chrome — but drop the outer track (nine filters + scroll would read as a second toolbar) and add a 2 px accent bar for the selected state (fill alone fails WCAG 1.4.11). New strip is a radiogroup, 40 px block, 26 px chips, horizontal overflow. Spec: SCREENS §2.11. |
| `.searchline` | 30 | `SearchRow` (`launcher/SearchRow.tsx`) | **Keep** 72 px height, 14/20 px padding, 14 px gap, 1 px bottom hairline. Explicit comment at line 29 is a requirement: no title bar, no window controls. |
| `.searchline input` | 31 | `SearchInput` inside `SearchRow` | **Keep** 21 px/430/`-0.04em`, transparent, no visible border; **adapt** placeholder ink to `#69746e` (reference `#b4bfba` = 1.84:1, fails); add combobox ARIA. |
| `.count` | 32 | `MatchCount` | **Keep** 11 px right-aligned; **adapt**: live-region status, pluralisation, "Searching…", accessible ink. |
| `.results` | 33 | `ResultsViewport` | **Keep** min 140 / max 365 / padding 10-11-13 / thin scrollbar; **adapt**: virtualise >40 rows; states (skeleton/empty/error) specified in SCREENS §2.3. |
| `.group-label` | 34 | `ResultGroup` + `GroupLabel` | **Keep** 10 px/750/+0.12em caps, 14/13/8 padding; **adapt** semantics: `role="group"` labelled by this element (it is not an option). |
| `.result` + `.selected` | 35–37 | `ResultRow` (`launcher/ResultRow.tsx`) | **Keep** 58 px, 8/12 padding, radius 8, gap 12, 110 ms colour transition, selected fill `#e7eef2` + border; **adapt**: add 2 px inset `#285975` start bar (selection state fails WCAG 1.4.11 at 1.14:1 without it) and identity-stable selection (KEYBOARD §4). |
| `.fileicon` + `.word/.folder/.code/.action` | 38–41 | `ResultIcon` + `FileTypeBadge` | **Keep** 35×37 slot, radius 7, 1 px border, 11 px/750 badge letter, family tints (folder ink corrected `#a57b3c`→`#7d5a20` for 4.5:1); **adapt**: real shell icons are first-class (master prompt §V.A), badge is fallback/loading; folder uses an SVG glyph, not `▰`. |
| `.resultbody`, `.filename`, `.filepath`, `.resultmeta` | 42–44 | `ResultRow` internals | **Keep** hierarchy (13/680 title, 11 px path, 10–11 px trailing meta); **adapt** contrast (path `#929e99`→`--ps-ink-2`, meta→`--ps-ink-3`), truncation + accessible description, meta hidden ≤700 px. |
| `.footer`, `.leftfoot`, `.rightfoot`, `.ready`, `kbd`, `.footlink` | 45–49 | `LauncherFooter`, `StatusDot`, `Kbd`, `OpenAppButton` | **Keep** 44 px bar, 9/16 padding, dot + status + link, three kbd hints; **adapt**: contextual hints (SCREENS §2.5), real `EngineStatus`, status becomes a polite live region, "Open ProtonSearch" becomes a button, hit targets ≥24 px (reference `.footlink` padding 3 px is too small). |
| `.empty` | 50 | `EmptyState` | **Keep** centred quiet text; **replace** sample copy with production copy (SCREENS §2.3/§9); no illustration. |
| Search ranking JS | 134–135 | engine (`ps-core`) via Tauri `search` | **Replace**: no client-side scoring/filtering; selection by result id; group order fixed by source kind (SCREENS §2.2); empty state from real result set. |
| Keyboard JS | 137 | `useLauncherKeyboard` hook | **Keep intent** ↑↓ wrap, Esc clears, Enter acts; **replace/expand** full map in KEYBOARD §2 (Enter opens via ActionSpec, Esc chain, Ctrl+Space, Ctrl+C, context menu, stale protection). Reference Enter merely fills the input with the row name — production must actually open. |
| State (`view, section, firstName, currentStep, search, selected, locations, options`) | 128–129 | React state: route, settings/onboarding DTOs, query, `{resultId, epoch, index}` selection, roots/modes | **Replace** demo state with `ps-ipc` DTOs and persisted settings; no global `innerHTML` rendering (`clean()` at 131 exists only to escape demo strings; React escapes by default). |

---

## 3. Full Desktop App and Onboarding

| Reference | Lines | Production component (planned path) | Keep / adapt / replace |
|---|---|---|---|
| `.desktop-app` card | 51–52 | `AppShell` + app window (`apps/desktop/src/app/AppShell.tsx`) | **Replace** card-on-stage with a real window that fills its frame (master prompt §V.B: "not just the launcher stretched larger"); native Windows title bar. **Keep** 190 px sidebar / 1 px divider / warm panel palette. |
| `.sidebar` | 53 | `AppSidebar` | **Keep** tinted `#f3f6f2`, right hairline, 21/13 padding; **adapt** widths 220/190/56 rail per breakpoint (SCREENS §3.1). |
| `.sidebar .brand` + `.logo` | 54 | `BrandBlock` | **Keep** 29 px mark + 12/750 wordmark. |
| `.sidebar-label` | 54 | "WORKSPACE" label | **Keep** 10 px caps; **adapt** ink to `#5f6d67` (reference `#9aa69e` = 2.32:1). |
| `.navbtn` + `[aria-current=page]` | 55–57 | `NavItem` | **Keep** 40 px row, radius 7, neutral active fill `#e0eae8` + ink `#234759` (no coloured pill, no left bar); **adapt** roving tabindex + arrow navigation. |
| `.navglyph` + `nav` glyphs `⌂ ⌕ ◷ ↗ ⚙` | 58, 138 | 16 px Lucide icon set | **Replace** unicode glyphs: font-dependent, inconsistent weight, invisible to AT. Same 19 px slot. |
| `.userfoot`, `.avatar` | 59–60 | `SidebarUser` | **Keep** bottom-pinned, 30 px tile; **adapt**: name optional; fallback "This PC" with a neutral glyph (never infer identity — master prompt §V.B); static, not a button. |
| `.workspace` | 61 | `RouteOutlet` | **Keep** 36/40 padding; **adapt** responsive paddings and content max-width 1040 (SCREENS §3.1). |
| `.eyebrow`, `h1`, `.lead` | 61–63 | `PageHeader` | **Keep** hierarchy and clamp(26–41 px)/630/`-0.065em`; **adapt** weights to tokens (600), lead ink to `#5c6862` (reference `#8c9a92` = 2.85:1). |
| `.bigsearch` | 64–66 | `HomeSearchButton` | **Keep** raised white field look, radius 8, search glyph + placeholder + ↗; **adapt** 48 px height, real button semantics, opens the launcher. |
| `.columns` | 67 | `HomeColumns` | **Keep** 1.25fr/.9fr, gap 38; stack ≤819. |
| `h2` | 68 | `SectionLabel` | **Keep** 11 px/750/+0.1em caps; contrast corrected. Not used for table headers. |
| `.recent`, `.recent-icon`, `.recent-title`, `.recent-sub` | 69–71 | `RecentRow`/`RecentList` | **Keep** anatomy; **adapt** rows are buttons, trailing arrow decorative, real recents from `get_activity`. |
| `.statsnum`, `.statsub`, `.thinprogress` | 72–73 | `StatBlock` + `ProgressBar` | **Keep** 34/680 stat, 4 px bar; **replace** sample "124,318" and "(sample)" copy with real `EngineStatus` counts; progress is truthful or indeterminate. |
| `.metric` | 74 | `MetricRow` | **Keep** label/value rows with hairline; **adapt** `<dl>` semantics, tabular numerals, real counts. |
| `.section` | 75 | route section spacing | **Keep** 24 px rhythm (32–40 for settings groups). |
| `.info-panel` | 76 | `Panel` | **Keep** 1 px border, radius 8, 19/21 padding, no shadow at rest. |
| `.muted` | 77 | helper text | **Keep** 12 px/1.65; **adapt** ink to `--ps-ink-2` (reference `#8b9990` = ~3:1). |
| `.smallbtn`, `.primarybtn` | 78–80 | `Button` variants | **Keep** 6 px radius, 10×14, 12/720, primary `#234a61`; **adapt** one filled primary per screen, hover tokens, press = colour only. |
| `.onboarding` | 81–82 | `OnboardingRoute` | **Keep** rail+content split, 240 px rail; **replace** card-on-stage with a real setup window 900×620 (native frame), 7 steps (master prompt §V.C lists intro/privacy, name, locations, depth, hotkey, agent, finish). |
| `.setup-side`, `.steps`, `.step-indicator`, `.step-num` | 83–86 | `SetupRail`, `StepIndicator` | **Keep** tinted rail `#e7eee9`, 25 px circles, current filled `#295a70`; **adapt** 7 items, `aria-current="step"`, rail hidden ≤700. |
| `.setup-main`, `.setup-main h1`, `.lead` | 87 | `SetupStep` | **Keep** 44/43 padding, 32 px h1, lead max-width 420; **adapt** responsive padding 32/22 ≤700. |
| `.fieldlabel`, `.field` | 88–89 | `Field`/`Label` | **Keep** caps 11/740 label, 13 px input, radius 7; **adapt** label association, `aria-describedby` helper, focus ring, contrast. |
| `.choices`, `.choice` | 90–91 | `ChoiceRow` + `SegmentedControl` | **Keep** checkbox rows with right-aligned description for locations; **replace** the global "capabilities" checkboxes with per-root depth segmented controls + trade-off copy (master prompt §V.C: names-only/content/OCR per root with resource tradeoffs). "Configure AI later" moves to the agent step. |
| `.setup-bottom` | 92 | `StepFooter` | **Keep** Back/primary placement; **adapt** "Skip setup" on every step, state persisted per transition, Enter = primary, Esc no-op. |
| `.warning` | 93 | `HelperText` | **Keep** 10 px note; **adapt** ≥11 px when sentence-case, contrast fixed. |
| Media query `max-width:700px` | 94–100 | breakpoint contract (SCREENS §6.1) | **Keep the 700 px content breakpoint** exactly: launcher 61 px search row / 16 px input / hidden trailing meta / 26 px logo; app 56 px rail, stacked columns; onboarding rail hidden. Production also defines 560/820/1000/1280 bands and the app's 560 px minimum. |
| `prefers-reduced-motion` | 101 | global reduced-motion CSS (DESIGN_SYSTEM §6.3) | **Keep**; production uses ~0 ms durations rather than `transition:none` so state changes still paint. |
| Onboarding JS steps/flow | 147–153 | `OnboardingRoute` + step components + `OnboardingState` | **Keep** copy where still true; **replace** the 4-step flow with the 7-step flow; persist/resume; "Skip for now" generalised; Enter/Continue; no fake completeness. |
| Step 0 name field | 149 | Step 02 | **Move** to its own step (optional, capped 40 chars, no inference). |
| Step 1 locations | 150 | Step 03 | **Keep** the four suggestions and their descriptions verbatim (reference copy is good). |
| Step 2 capabilities | 151 | Step 04 | **Replace** per-root modes + truthful resource trade-offs; "Models stay opt-in" moves to Step 06. |
| Step 3 summary | 152 | Step 07 | **Keep** metric-row summary; add shortcut; no fake progress. |
| Back/next handlers | 153 | `StepFooter` | **Adapt**: persist at every transition; Skip on every step; Esc no-op; focus h1 on change. |
| `render()` | 155–156 | router | **Replace** global re-render with route components; no `innerHTML`. |

---

## 4. Explicit deviations from the reference (with justification)

| # | Reference | Production | Justification |
|---|---|---|---|
| D1 | App shown as a 1000 px card on a stage | App fills a native window (1120×760 default) | Master prompt §V.B: the desktop app is a companion app, not a launcher-sized card. A real window is also the only accessible/resizable platform surface. |
| D2 | Demo switcher (Launcher / Desktop app / Onboarding) | Real windows/routes | Demo chrome; shipping tabs would violate the launcher's "no tabs" rule. |
| D3 | Secondary greys `#828e8b`, `#a0aaa6`, `#929e99`, `#95a19c`, `#9aa69e`, `#82948b`, `#8c9a92`, `#b4bfba` | `--ps-ink-2 #5c6862`, `--ps-ink-3 #68736d`, placeholder `#69746e` | Reference values fail WCAG AA 4.5:1 (2.3–3.3:1). Corrections documented in DESIGN_SYSTEM §3.3. |
| D4 | Selected row = fill + low-contrast border only | + 2 px inset `#285975` start bar (dark: `#8fb6cc`) | Fill vs surface is 1.14:1; WCAG 1.4.11 state contrast needs ≥3:1. Bar is 6.45:1 and survives forced colours. |
| D5 | Unicode nav glyphs `⌂ ⌕ ◷ ↗ ⚙` | 16 px Lucide outline icons | Font-dependent rendering, inconsistent metrics, no AT semantics. |
| D6 | Folder badge glyph `▰`, action glyph `↗` | SVG folder/command glyphs | Same reason; and the badge must stay legible in dark/HC. |
| D7 | File letter badges only | Real shell icons first-class; badges as fallback/loading | Master prompt §V.A requires real app/file icons; badges keep layout stable when shell icons are unavailable. |
| D8 | Clicking a result fills the input with its name | Enter opens via `ActionSpec`; click opens | Reference demo has no OS integration; production must act (master prompt §IV query path, §V.A). Selection is not a text edit. |
| D9 | Client-side sample scoring and filtering | Engine ranking via Tauri `search`; deterministic merge | Master prompt §IV: ranking is the engine's job; UI must not re-score or filter results. |
| D10 | 4-step onboarding | 7 steps (welcome+privacy, name, locations, depth, shortcut, agent, ready) | Master prompt §V.C enumerates these decisions; the reference's 4 steps predate them. The rail/visual pattern is preserved. |
| D11 | "Skip for now" only on step 1 | "Skip setup" on every step + resumable state | Master prompt §V.C: skippable and resumable first-run tasks. |
| D12 | Mock statuses ("Illustrative", "(sample)", "Index ready" hard-coded) | Real `EngineStatus`; absent until known | Master prompt hard rule 2 and §V.C truthful progress; no fabricated evidence. |
| D13 | Onboarding sample name "Alex" | Optional name; neutral fallback; never inferred | Master prompt §V.B (identity only with consent). |
| D14 | Static footer hints "↑↓ Navigate / Enter Open / Esc Clear" | Contextual hints (max 3) + truthful status | Master prompt §V.A: hints only in the footer and relevant; "Esc Close" when empty. |
| D15 | No preview | Ctrl+Space preview (side panel/drawer) + optional auto-preview | Master prompt §V.A: previews only when useful; expanded exploration. |
| D16 | No context menu / multi-select | Context menu (key/right-click); clipboard multi-select is **CUT for v2.0 alpha** (SCREENS §10) | Master prompt §V.A requires result context menus and multi-select "where appropriate"; the context menu ships, multi-select is deferred with recorded rationale. |
| D17 | Dark theme absent | Art-directed warm-graphite dark palette | Master prompt §V.D: dark mode separately art-directed; reference is light-only. |
| D18 | 10 px meta inside `.resultmeta`/footer | 10 px only for caps/kbd/footer; meta 11 px | Generic house floor 11 px; kept the reference's 10 px roles where they are tracked caps or keycaps, sentence-case meta raised to 11 px. |
| D19 | App sidebar 190 px fixed; no rail | 220 / 190 / 56 rail at breakpoints | Reference only shows one width; the app must work down to 560 px. |
| D20 | Missing brand files referenced by master prompt | Reconstructed `assets/brand/*.svg` documented as reconstructions | Files confirmed absent (`WORK_BREAKDOWN.md:104`); the inline SVG is the visual source. |
| D21 | Launcher shadow drawn as a single large drop shadow (`0 24px 56px`) on a page mock | Layered 4-stop clipped-safe shadow inside a 28 px transparent window pad (extent ≤24 px) + 1 px rim on dark | The old blur (extent ≈108 px) was hard-clipped by the native window edge — the reported "shadow doesn't look proper". The pad is a real window constraint (Rust `LAUNCHER_SHADOW_PAD = 28`). |
| D22 | No open/close motion in the reference | Launcher card open 150 ms / close 110 ms; app shell reveal 160 ms; input/selection/results instant | A palette summoned hundreds of times/day must not delay typing: only the card fades/moves; the window, focus and search dispatch are immediate. Reduced motion removes the transform and duration. |
| D23 | No filter/category strip in the reference | Horizontal filter strip above the results (All + 8 categories), adapted from the reference's segment language | Needed for scope control on a 50-result list; without it users rely on remembered prefixes. Language borrowed, not the demo chrome. |
| D24 | Launcher was a card on a page stage | Floating frameless window with 28 px transparent shadow pad; clicks in the pad dismiss | The card must have real elevation without clipping; the pad is part of the launcher surface, not a click-through frame. |

---

## 5. Traceability checklist for reviewers

---

## 6. Current scaffold mapping (2026-11-09)

The Lead's scaffold landed after this document's first draft. It consolidates each surface into one view component;
the reference-to-contract mapping above still governs the anatomy **inside** those components. Actual files:

| Reference surface | Scaffold file(s) | Notes for `ui-launcher` / `ui-app` |
|---|---|---|
| `.launcher`, `.searchline`, `.results`, `.group-label`, `.result`, `.footer`, `.empty` | `apps/desktop/src/launcher/LauncherView.tsx`, `launcher.css` | consolidated view; split into SearchRow/ResultRow/Footer components when it grows past review size, keeping the contract names from §2 |
| `.fileicon` variants | `apps/desktop/src/launcher/ResultIcon.tsx` | real shell icon + badge fallback (§2 mapping, DESIGN_SYSTEM §9.2) |
| `.logo`, `.brand` | `apps/desktop/src/components/Logo.tsx` | currently inlines the light three-path mark at 27 px with an opaque counterform; switch to `assets/brand/` variants: on-dark in dark theme, 16 px file <24 px, 29 px desktop (brand contract in `assets/brand/README.md`) |
| `.desktop-app`, `.sidebar`, `.navbtn`, `.navglyph`, `.userfoot`, `.avatar`, `.workspace`, Home blocks | `apps/desktop/src/app/DesktopApp.tsx`, `desktop.css`, `describe.ts`, `sections.ts`, `roots.ts`, `hotkey.ts` | all five routes are hosted here in the scaffold; verify each route against SCREENS §3 and split into route components when it grows |
| Onboarding rail + steps | `apps/desktop/src/onboarding/OnboardingView.tsx`, `logic.ts`, `onboarding.css` | currently 4-step layout in the reference; production must be 7 steps (SCREENS §4), resumable/skippable |
| kbd, highlight | `apps/desktop/src/components/Kbd.tsx`, `Highlight.tsx` | keep token-driven styles; no hard-coded hex |
| Tokens / reset / shell | `apps/desktop/src/styles/tokens.css` (L0), `base.css`, `shell.css` | tokens already match DESIGN_SYSTEM §11 with the reconciliations in §3.6 (preview 1180, image badge family, `--faint` not for text) |
| Data + mock bridge | `apps/desktop/src/bridge/api.ts`, `types.ts`, `mock.ts` | `api.ts` is the only module allowed to call Tauri `invoke`; UI never re-scores results (§2) |
| Window shell | `apps/desktop/src-tauri/**` (L0-owned) | launcher frameless/instant show, app native frame, DPI clamps (§6 SCREENS) |

Reconciliation items already logged to L0: brand-variant switch in `Logo`, 29 px desktop mark (currently 27), preview
width 1180/360 (now final), `--faint` retired from the contract, the 4→7 onboarding step expansion, and the v2.0 alpha CUTs/limitations in SCREENS §10.


- Every reference class in §2–§3 has exactly one production component, and no production component invents chrome the
  reference lacks (no title bar, tabs, fake badges, Ctrl+K).
- Every deviation in §4 appears in `DESIGN_SYSTEM.md` §2.3 or this table — no undocumented drift.
- Copy in SCREENS §9 supersedes every mock string in the reference; "sample", "illustrative", "Alex", "124,318" must
  not appear in production.
- Accessibility corrections (D3, D4, D7, D14–D16) are re-verified in the visual/a11y audits (`docs/qa/visual/**`
  owned by `ux-qa`).
