# ProtonSearch v2 — Design System

**Owner:** L1-B Product Design & Experience (`design-spec`). **Status:** binding for G3–G4 UI work, v1.0.
**Sources of truth:** `protonsearch-v2-ui-reference.html` (literal visual foundation, cited by line),
`PROTONSEARCH_V2_MASTER_PROMPT_ALL_IN_ONE.md` §V/§V.D (binding constraints), `docs/implementation/WORK_BREAKDOWN.md`.
**Companions:** `docs/design/SCREENS.md` (screen specs), `docs/design/KEYBOARD_AND_A11Y.md` (input + accessibility),
`docs/design/REFERENCE_MAPPING.md` (reference element → component), `assets/brand/README.md` (mark).

**Token ownership.** `apps/desktop/src/styles/tokens.css` is owned by L0 (Team Lead). This document defines the
contract; L0 lands it. UI owners read tokens only — no hard-coded hex values in components. If a token name must
change during scaffolding, keep the semantic name in the CSS custom property and note the rename here.

---

## 1. Design position — "the quiet instrument"

ProtonSearch is not a dashboard and not a marketing page. It is a **utility that is opened hundreds of times a
day** (master prompt §V.A "the primary hero experience"), with a companion app that is worked in rather than
browsed. The visual language is deliberately small, dense, and exact: warm-neutral ground, graphite ink, one
subdued blue accent, hairline separation, and the two-tone navy P as the only saturated brand element
(master prompt:194, reference `:root` at `protonsearch-v2-ui-reference.html:12-15`).

Five rules that decide every ambiguous case:

1. **Speed is the aesthetic.** No motion may add latency to a keyboard action; no chrome may compete with the
   query or the selected row. Selection feedback is colour, not travel (master prompt §V.D).
2. **The utility is quiet, the data is loud.** Colour goes on state and data (selected row, status, selected
   field), never on chrome. Chrome is ink on warm neutral.
3. **Density is respect.** A launcher row carries name, path, and time on one 58 px line; the app shows real
   counts, not hero cards. No oversized whitespace, no decorative tiles (master prompt hard rule 5 at line 26).
4. **One material.** Flat opaque surfaces, 1 px hairlines, a small shadow only where a surface genuinely floats
   above another window (the launcher, menus, dialogs). No gradients, glass, blur, or glow.
5. **Truthful surfaces.** Statuses, counts, and progress are real or absent — never decorative (master prompt
   hard rule 5; §C "show truthful progress"). "Index ready", "3 matches", "8,440 of 20,113" are engine facts.

### 1.1 Reference fidelity vs generic house rules

The reference is the literal visual foundation (master prompt:167). We implement its *system* — tokens, hierarchy,
component anatomy, density — not its demo chrome. Where an installed skill's generic house style conflicts with the
reference, the reference wins and the deviation is recorded below (§2.3). Where the reference conflicts with
accessibility law, accessibility wins and the correction is recorded (e.g. secondary greys, §4.3).

The reference demo page (`.stage`, `.controlbar`, `.segments`, `.demo-label`, reference:18-26, 106-114) is a
specimen sheet, **not** product UI. The product launcher is the frameless `.launcher` surface; the product app
fills its window instead of sitting as a card on a stage (see REFERENCE_MAPPING.md).

---

## 2. Rules that prevent slop

### 2.1 Banned, absolutely (master prompt:26, §V.D; verified in review)

| Banned | Why / replace with |
|---|---|
| Gradients on grounds, buttons, cards, text, tiles | Flat fills; tone steps only. |
| Glass / backdrop blur / acrylic / glow / neon borders | Opaque surfaces one tone step apart. |
| Purple / violet / indigo accents; cream + terracotta palette | Reference blue-grey accent `#285975`; warm grey-green neutrals. |
| Bounce, spring overshoot, elastic easing, hover lifts | Linear/ease-out fades, ≤ 180 ms, transform ≤ 4 px. |
| `transition: all`; animating layout props (width/height/margin/padding) | Explicit `background-color`, `border-color`, `opacity`, `transform` only. |
| A title bar, min/max/close buttons on the launcher; "Ctrl+K" hint or a kbd pill inside the search field | Frameless surface, logo at the start of the search row, hints in the footer only (master prompt §V.A). |
| Fake badges, fake "AI" dot, fake "ready" green, fake percentages, sample counts presented as real | Real `EngineStatus`; text labels; hidden until known. |
| Raw exception text, error codes, stack traces, absolute paths, or `err.message` on the surface | One human sentence + optional action; sanitized technical detail only inside a collapsed "Details" disclosure (DESIGN_SYSTEM §8.3). |
| Giant tiles, three-up feature cards, KPI cards with icons in tinted circles, sparklines on KPIs | One divided strip or plain type (app-screens house rules). |
| Zebra tables, header bands, uppercase tracked table headers | Row rules + sentence-case 12 px headers. |
| Emoji anywhere in product chrome, greetings, labels, empty states | Words. |
| Rainbow charts, a hue per bar, more than 4 hues on a screen | One hue per data set, neutrals otherwise. |
| Icon fonts, unicode glyphs as icons (reference uses `⌂ ⌕ ◷ ↗ ⚙` as placeholders) | A real 16 px outline icon set (§10.1). |
| Document preview always-on in the compact launcher | Preview only on demand (Ctrl+Space) or when the user enables it (§9.6). |
| Marketing voice ("Welcome back! ✨", "Supercharge", "AI-powered") | Plain product nouns and verbs. |

### 2.2 Kept from the reference on purpose

Hairline separators; 1 px component borders; a **quiet selected row** (tint + border + a 2 px state bar, §9.4);
letter file-type badges (fallback, §10.2); small tracked-caps **labels** (not table headers); the two-tone P;
a single 4 px progress hairline (post-alpha; alpha shows state words only); graphite text on warm off-white; the left rail in onboarding; a native-feeling
sidebar.

### 2.3 Conscious deviations from the installed skills' generic house style

| Skill rule (generic) | Our decision | Why it is right here |
|---|---|---|
| "No uppercase tracked labels" (design-taste-frontend) | Uppercase tracked **eyebrow**, **group label**, **section label**, **field label** are part of the reference and are kept; never uppercase table headers or buttons. | The reference defines this hierarchy (reference:21, 34, 61, 68, 88). Dense scanning needs authoritative micro-labels; they are limited to 10–11 px/750 and at most one tracked-caps family per screen block. |
| "Nothing under 11 px" (app-screens) | 10 px reserved for uppercase tracked labels, kbd hints, and footer hints; sentence-case meta minimum is 11 px; body minimum 13 px. | Reference:32,44-46,48; scales to 12.5/15/20 px at 125/150/200 % DPI. Risk logged in §13. |
| "No 1 px borders / card borders on pages" | 1 px borders allowed (app surfaces) — a border or a tone step, never both; no shadow on resting cards. | App-screen house rules explicitly relax this; the reference's surfaces are hairline-defined (reference:27,51,76). |
| "No shadows" | Shadow only on genuinely floating surfaces: launcher window, context menu, dialog, onboarding window. | These float above other windows; the reference's launcher/app shadows are the correct elevation cue (reference:28,52). |
| "Primary button is near-black, never brand colour" | Primary = `#234a61` (deep blue-grey) at ≤1 filled primary per screen. | Reference:80 defines the primary fill; it reads near-black in context and stays within the muted accent family. |
| "Never animate keyboard-initiated actions" (emil-design-eng) | Launcher show/hide, result selection, and Enter are instant. Colour transitions on selection are allowed (110 ms); preview/menu (occasional, user-toggled) may fade 90–160 ms. | Raycast-class launchers earn their speed by not animating the 100×/day path; a colour cross-fade does not delay paint or input. |
| "Density moves type size, padding and row pitch together" | Density changes row pitch, padding and control heights only; type stays (10/11/12/13). | Type is already at the dense end; shrinking it would break the 10 px floor and the reference's hierarchy. |
| "File-type icons should be real; badges are placeholders" | Real shell icons are first-class; the letter badge is the deterministic fallback and the loading state. | Master prompt §V.A requires realistic file icons and first-class app icons; the badge keeps the slot stable when a shell icon is missing. |

---

## 3. Colour

### 3.1 Reference palette as supplied (evidence)

`protonsearch-v2-ui-reference.html:12-17`:

| Reference token | Value | Used in reference for |
|---|---|---|
| `--desktop` | `#e8ebe8` | stage/desktop ground |
| `--panel` | `#fbfcfa` | launcher/app surface |
| `--ink` | `#202b31` | primary text |
| `--muted` | `#828e8b` | secondary labels (fails 4.5:1 — corrected in §3.3) |
| `--quiet` | `#a0aaa6` | tertiary (fails 4.5:1 — corrected) |
| `--line` | `#e0e5df` | hairline |
| `--blue` | `#285975` | links, active nav ink |
| `--blue-deep` | `#1e4158` | deep accent |
| `--selected` | `#e7eef2` | selected result fill |
| `--selected-border` | `#d6e2e9` | selected result border |
| `--soft` | `#f2f5f1` | soft control ground |
| `--sidebar` | `#f3f6f2` | app sidebar |
| focus outline | `#5482a3` | `:focus-visible` 2 px |
| `--sidebar`/`--panel` brand mark | `#102448` / `#1d5189` / `#fafbf9` | mark only (reference:118) |

### 3.2 Semantic tokens — light theme (default)

| Token | Value | Role | Text contrast (on the named surface) |
|---|---|---|---|
| `--ps-ground` | `#e8ebe8` | window ground behind panels; app stage | — |
| `--ps-surface` | `#fbfcfa` | primary window/panel surface | ink 14.06:1 |
| `--ps-surface-raised` | `#ffffff` | panels, fields, big search, content cards | ink 14.47:1 |
| `--ps-surface-soft` | `#f2f5f1` | inset control ground, segmented track | ink 13.17:1 |
| `--ps-sidebar` | `#f3f6f2` | app sidebar (never dark/saturated) | ink 13.28:1 |
| `--ps-line` | `#e0e5df` | 1 px hairline | decorative |
| `--ps-line-soft` | `#e8ece7` | row rules inside lists | decorative |
| `--ps-line-strong` | `#cdd5ce` | window/launcher edge | decorative |
| `--ps-control-border` | `#d5dfd6` | inputs, selects, secondary buttons | decorative (control boundary 1.37:1) |
| `--ps-ink` | `#202b31` | primary text, headings, values | 14.06:1 surface |
| `--ps-ink-2` | `#5c6862` | secondary text: paths, descriptions, nav rest | 5.65:1 surface · 4.84:1 ground · 5.34:1 sidebar · 4.96:1 selected |
| `--ps-ink-3` | `#68736d` | tertiary meta on **surface/raised only** (never on ground, soft, or selected rows) | 4.79:1 surface · 4.93:1 raised |
| `--ps-accent` | `#285975` | links, active nav ink, selected-state bar, focus text | 7.35:1 surface · 6.45:1 selected |
| `--ps-accent-strong` | `#1e4158` | hover/pressed accent text | 10.45:1 surface |
| `--ps-accent-soft` | `#e7eef2` | selected result fill | ink 12.34:1 |
| `--ps-accent-border` | `#d6e2e9` | selected result border (reference value) | decorative |
| `--ps-selected-bar` | `#285975` | 2 px inset start-edge marker on the selected row (state cue) | 6.45:1 vs selected fill |
| `--ps-focus` | `#5482a3` | 2 px focus ring | 4.0:1 surface · 4.12:1 raised |
| `--ps-nav-active-bg` | `#e0eae8` | active sidebar item fill | active ink 8.08:1 |
| `--ps-nav-active-ink` | `#234759` | active sidebar item text | 8.08:1 |
| `--ps-avatar-bg` | `#dae7e8` | avatar tile | ink `#245168` 6.76:1 |
| `--ps-avatar-ink` | `#245168` | avatar initial | 6.76:1 on avatar |
| `--ps-primary-bg` | `#234a61` | primary button fill | white 9.44:1 |
| `--ps-primary-ink` | `#ffffff` | primary button label | 9.44:1 |
| `--ps-primary-hover` | `#335e75` | primary hover | white 7.0:1 |
| `--ps-positive` | `#247b50` | success text | 5.07:1 surface |
| `--ps-warning` | `#8a5a12` | warning text | 5.75:1 surface |
| `--ps-negative` | `#a92a2f` | error/destructive text | 6.67:1 surface |
| `--ps-positive-pill-bg/ink` | `#dcfae5` / `#04250e` | positive pill | 14.77:1 |
| `--ps-negative-pill-bg/ink` | `#ffedea` / `#831c1e` | negative/destructive pill | 8.63:1 |
| `--ps-warning-pill-bg/ink/border` | `#fffbeb` / `#92400e` / `#fde68a` | notice pill | 6.84:1 |
| `--ps-kbd-bg/border/ink` | `#f0f3ef` / `#dbe2da` / `#5d6f65` | keycap hint | 4.78:1 |
| `--ps-progress-track` | `#e5ebe5` | progress bar track | decorative |
| `--ps-progress-fill` | `#407188` | progress bar fill | 4.41:1 vs track |
| `--ps-scrollbar-thumb` | `#d9dfda` | launcher scrollbar | decorative |
| `--ps-scrim` | `rgba(32,43,49,0.32)` | dialog scrim | — |

File-type badge tints (light; text/background contrast in §3.5): word `#e5eef5`/`#285e8b`; folder `#eee9dc`/`#7d5a20`;
code `#e9eee8`/`#386d58`; action `#e7e9eb`/`#436276`; neutral `#e8ece7`/`#526a66`;
image/media `#eae8ef`/`#574f68` (6.35:1). `--ps-badge-image-*` was added by L0 when landing the tokens; adopted here.

### 3.3 Accessibility corrections to the reference (recorded deviation)

The reference's secondary greys were measured and fail WCAG AA for text: `--muted #828e8b` = 3.30:1,
`--quiet #a0aaa6` = 2.32:1, filepath `#929e99` = 2.69:1, meta `#95a19c` = 2.60:1, sidebar label `#9aa69e` = 2.32:1,
eyebrow `#82948b` = 3.11:1, lead `#8c9a92` = 2.85:1, placeholder `#b4bfba` = 1.84:1, kbd `#63756b` = 4.38:1,
folder badge `#a57b3c` = 3.15:1. Production substitutes `--ps-ink-2 #5c6862`, `--ps-ink-3 #68736d` (surface only),
`--ps-focus`, and corrected badge inks (folder `#7d5a20` = 5.16:1, neutral `#526a66` = 4.87:1). **Net visual effect:
graphite text reads as the reference at normal viewing distance; light-grey meta is deliberately darker.** Every
pair was recomputed with the WCAG 2.1 relative-luminance formula (script: `.agents/notes/contrast_final.py`, output
`.agents/notes/contrast-final.txt`).

### 3.4 Semantic tokens — dark theme (separately art-directed)

Direction: **warm graphite**, not an inversion and not navy/slate. The ground keeps the light theme's faint
green-grey warmth; panels step *up* in lightness; hairlines are structural; the accent lifts to a desaturated
sky-blue for contrast; the brand mark switches to its on-dark variant (§11). Final values:

| Token | Value | Role | Text contrast |
|---|---|---|---|
| `--ps-ground` | `#141513` | window ground | ink 15.77:1 |
| `--ps-surface` | `#1c1e1a` | primary surface | ink 14.46:1 |
| `--ps-surface-raised` | `#242722` | raised panels/fields | ink 13.02:1 |
| `--ps-surface-soft` | `#202320` | inset control ground | ink 13.67:1 |
| `--ps-sidebar` | `#191b18` | sidebar one step off surface | ink 14.92:1 |
| `--ps-line` | `#2e322c` | hairline | decorative |
| `--ps-line-soft` | `#262923` | row rule | decorative |
| `--ps-line-strong` | `#3d423a` | window edge | decorative |
| `--ps-control-border` | `#666e63` | input/control boundary | 3.18:1 surface (≥3:1 requirement) |
| `--ps-ink` | `#ecefe8` | primary text | 14.46:1 surface |
| `--ps-ink-2` | `#b4bab0` | secondary text | 8.47:1 surface · 6.57:1 selected |
| `--ps-ink-3` | `#959b91` | tertiary meta | 5.9:1 surface · 4.58:1 selected |
| `--ps-accent` | `#8fb6cc` | links, active nav ink, selected bar | 7.78:1 surface · 6.04:1 selected |
| `--ps-accent-strong` | `#a8c9db` | hover accent | 9.63:1 surface |
| `--ps-accent-soft` | `#24333b` | selected result fill | ink 11.22:1 |
| `--ps-accent-border` | `#4f7183` | selected result border | 3.22:1 surface (≥3:1 state cue) |
| `--ps-selected-bar` | `#8fb6cc` | 2 px selection marker | 6.04:1 selected fill |
| `--ps-focus` | `#7fb0c9` | focus ring | 7.16:1 surface |
| `--ps-nav-active-bg` | `#2b3029` | active nav fill (brightness, not hue) | ink `#f2f5ef` 12.25:1 |
| `--ps-nav-active-ink` | `#f2f5ef` | active nav text | 12.25:1 |
| `--ps-avatar-bg` | `#223038` | avatar tile | ink `#9ec5de` 7.43:1 |
| `--ps-avatar-ink` | `#9ec5de` | avatar initial | 7.43:1 |
| `--ps-primary-bg` | `#2f637e` | primary button fill | `#f7f8f4` 6.14:1 |
| `--ps-primary-ink` | `#f7f8f4` | primary label | 6.14:1 |
| `--ps-primary-hover` | `#3a708c` | primary hover | 5.08:1 |
| `--ps-positive` | `#6fbf93` | success text | 7.64:1 surface |
| `--ps-warning` | `#d9b171` | warning text | 8.38:1 surface |
| `--ps-negative` | `#e08a8a` | error text | 6.54:1 surface |
| `--ps-positive-pill-bg/ink` | `#172a1d` / `#7fc99c` | positive pill | 7.75:1 |
| `--ps-negative-pill-bg/ink` | `#2c1a1c` / `#e59a9a` | negative pill | 7.41:1 |
| `--ps-warning-pill-bg/ink` | `#2a2317` / `#d9b171` | notice pill | 7.75:1 |
| `--ps-kbd-bg/border/ink` | `#262a24` / `#3a4038` / `#b0b7ad` | keycap hint | 7.10:1 |
| `--ps-progress-track` | `#2b3029` | progress track | decorative |
| `--ps-progress-fill` | `#8fb6cc` | progress fill | 6.25:1 vs track |
| `--ps-scrollbar-thumb` | `#3f463d` | scrollbar | decorative |
| `--ps-scrim` | `rgba(0,0,0,0.5)` | dialog scrim | — |

Dark badges: word `#1c2a33`/`#9ec5de`; folder `#2c2719`/`#d8b878`; code `#1c2a24`/`#a6cdb5`;
action `#212a30`/`#b6c9d6`; neutral `#23291f`/`#b9c4b3`; image/media `#2a2430`/`#c4b6d8` (all ≥7.8:1).

### 3.5 Colour rules

- **One accent family per screen.** The accent appears on: links, active nav text, the selected-row bar, the
  focus ring, and at most one filled control per view. Never on headers, tabs, or every button.
- **Selected row = fill + border + 2 px start bar.** Background tint alone is not a sufficient state cue
  (WCAG 1.4.11); the bar also survives forced-colours mode. On the dark theme the border itself is 3.22:1.
- **Active sidebar item = neutral fill + 2 px start bar** (same `--ps-selected-bar`; confirmed 2026-11-09). The
  reference's tint-only active row measures 1.13:1 against the sidebar — not a perceivable state cue (WCAG 1.4.11) —
  so the bar, plus the ink/weight change, is the accessibility correction.
- **Status is always word + colour.** Green/amber/red never carry meaning alone; the dot or pill is paired with
  text ("Index ready", "Paused", "Couldn't reach the network location").
- **Zero is normal ink.** A real zero prints as "0" in text colour; a missing/not-applicable value is a grey
  em-dash at meta size (never a large "--").
- **Colour share budget:** app chrome ≤3 % coloured pixels; charts/data carry the hue. No hue on the sidebar.
- **Never hard-code hex in components.** Use tokens; `currentColor` for icons; the only literal colours allowed are
  inside the brand SVG assets.
- **Theme switching:** `<html data-theme="light|dark">` + `color-scheme`; system mode listens to
  `matchMedia('(prefers-color-scheme: dark)')` and applies the resolved theme; window background and
  `<meta name="theme-color">` update with it.
- **Forced colours / high contrast** (§12.4): system colours only, no shadows, brand mark becomes monochrome.

### 3.6 Landed-token reconciliation (2026-11-09)

`apps/desktop/src/styles/tokens.css` (L0) already implements this palette with the `--ps-*` names. Deltas and
decisions recorded here so docs and code cannot drift:

- **Preview geometry (final, 2026-11-09):** `--launcher-width-preview: 1180px`, side panel **360 px**, threshold
  `workArea.width ≥ 1200`; the launcher default remains 820 px. The launcher CSS/JS still says 1220/380/1240 — that
  is code debt (REVIEW_1 §1.2); the docs are final.
- **Selected-row and nav-active bar (final, 2026-11-09):** both use a 2 px inset `--ps-selected-bar` start bar. The
  nav bar is confirmed (reference tint alone = 1.13:1; WCAG 1.4.11).
- **CUT for v2.0 alpha:** clipboard multi-select/Paste-N and the exact-command ACTIONS pin. Rationale and re-entry
  are in SCREENS §10; do not build stubs or teasers.
- **Window contract (final):** main app window minimum **560×480**; onboarding rail hides at **≤700**.
- **Alpha IPC gaps** (progress %, clear-activity, quicklinks CRUD, folder picker, updater, preview payload) with the
  truthful-UI rule are listed in SCREENS §10.
- **User visual pass (2026-11-09):** launcher shadow retuned to layered tokens that fit the 28 px pad (§5.4); launcher
  open/close motion 150/110 ms (§6.3/§6.4); horizontal filter strip added (SCREENS §2.11); stale app/title-bar icon is
  a defect (REVIEW_1 §8).
- **Filter/clipboard/agents pass (2026-11-10):** filter strip now **four chips ALL/FILES/FOLDERS/APPS** with Ctrl+1–4
  (v10, task-47); every other scope via prefixes/quick destinations; strict FILES/FOLDERS rows; clipboard category
  sections when clipboard scope is active (no chip row/tags); Workflows Agents section; preview is now the real G2 IPC
  payload (text/image/OCR/highlight/fields), quick destinations replace the empty prompt.
- **App readability pass (2026-11-09):** app/onboarding type scale raised one step (§4.2b, main-window tokens only);
  12-column alignment grid and rhythm added (§5.5); quiet error/empty/offline state family added (§8.3, copy map
  SCREENS §3.7); launcher tokens and specs unchanged/final.
- **Image/media badge family:** added by L0, adopted: light `#eae8ef`/`#574f68` (6.35:1), dark
  `#2a2430`/`#c4b6d8` (7.92:1). It is the sixth file-type family (images, screenshots, scanned media) in §9.2.
- **Spacing:** the global scale landed as `--space-1`…`--space-11` = 4/8/12/16/20/24/32/40/48/56/64 px. Use those
  for layout; component anatomy keeps the reference's local values (14/20 search padding, 11/13 results padding,
  8/12 row padding) as component CSS, not as new global tokens.
- **`--faint` legacy alias:** light `#8a948e` = 3.04:1 and dark `#7c857b` = 4.4:1 — **fails AA for text**. Do not use
  it for any word-bearing UI; use `--ps-ink-3` (surface-only) or `--ps-ink-2`. **Final decision (2026-11-09): keep it
  unused/out of the contract (or delete); do not alias it to `--ps-ink-3`, which itself fails on soft (4.48:1) and
  selected (4.20:1) surfaces.**
- **Legacy type aliases `--text-md: 14px` / `--text-lg: 16px`** are outside this type scale. They exist for scaffold
  views; migrate any view that needs them to `--ps-text-small/body` rather than adding new 14/16 px roles.
- **`--leading-tight: 1.16`** is a legacy alias. The route display h1 must render at **1.11** (reference:62); fix the
  alias or the consuming rule, not the reference value.
- **Brand in scaffold:** `apps/desktop/src/components/Logo.tsx` currently inlines the light three-path mark (opaque
  `#fafbf9` counterform, 27 px in the launcher). Replace with the `assets/brand/` variants: on-dark file when the
  resolved theme is dark, 16 px file below 24 px, 29 px on desktop. Contract: `assets/brand/README.md`.

---

## 4. Typography

### 4.1 Family and loading

```css
--ps-font-ui: "Inter", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
--ps-font-mono: "Cascadia Mono", "Cascadia Code", Consolas, "Courier New", monospace;
```

- **Ship Inter Variable** (SIL OFL 1.1), latin + latin-ext subset, weights 400–800, normal style, woff2,
  `font-display: swap`, preloaded for the launcher window (critical path). Reference:15 declares Inter first;
  bundling makes tracking and metrics deterministic across machines. If L0 decides not to bundle, the fallback is
  `Segoe UI Variable Text` and the weight map in §4.3 collapses to 400/600/700 — record that deviation.
- **Mono only for code/IDs.** Paths, filenames, counts and dates stay in the UI face with
  `font-variant-numeric: tabular-nums`. Code preview uses mono.
- No icon fonts, no third face. Italic only for citations/quotes in agent output (not chrome).

### 4.2 Type scale (roles)

| Token | Size / line-height | Weight | Tracking | Used for |
|---|---|---|---|---|
| `--ps-text-display` | `clamp(26px, 4vw, 41px)` / 1.11 | 600 | `-0.065em` | app route h1, greeting (reference:62) |
| `--ps-text-h1-setup` | 32 px / 1.15 | 600 | `-0.05em` | onboarding step h1 (reference:87) |
| `--ps-text-stat` | 34 px / 1.0 | 680 | `-0.075em` | the one big index number (reference:72) |
| `--ps-text-input` | 21 px / 1.2 | 430 | `-0.04em` | launcher query (reference:31) |
| `--ps-text-body` | 13 px / 20 px | 400 | `-0.012em` | default body, leads, descriptions (reference:16) |
| `--ps-text-strong` | 13 px / 18 px | 680 | `-0.012em` | result title, recent title |
| `--ps-text-small` | 12 px / 18 px | 400–650 | `-0.01em` | nav, metrics, buttons, secondary body |
| `--ps-text-meta` | 11 px / 16 px | 400–740 | `-0.005em` | paths, subtitles, counts, field labels |
| `--ps-text-micro` | 10 px / 14 px | 650–750 | `+0.12em` (caps) | group labels, eyebrows, kbd, footer hints |
| `--ps-text-section-label` | 11 px / 14 px | 750 | `+0.10em` (caps) | section h2 ("RECENTLY ACCESSED") |
| `--ps-text-kbd` | 10 px / 12 px | 650 | `+0.02em` | keycaps |

Rules: 4 sizes do the visible work (micro/meta/body/display); input and stat are the two exceptions. Five distinct
sizes per screen maximum; do not invent intermediate sizes. Sentence case everywhere except the tracked caps
labels and the brand name.

### 4.2b App + onboarding type scale — main window only (FINAL 2026-11-09)

User feedback: the app text is too small for comfortable reading. The app/onboarding surfaces take **one deliberate step
up at every role**; the launcher keeps its current scale (launcher specs are final and its density is intentional).
Rationale: 14 px body with 21 px leading is the Windows desktop baseline; 12 px meta and 11 px caps keep three clear
levels without a fourth tiny tier; display grows to 28–44 px so the h1 stays dominant after the body grows. Nothing
shrinks below 11 px in the app.

| Role | Token | Size / line-height | Weight | Tracking |
|---|---|---|---|---|
| Page title (h1) | `--ps-text-display` | `clamp(28px, 3.6vw, 44px)` / 1.08 | 600 | `-0.04em` |
| Setup h1 | `--ps-text-h1-setup` | 34 px / 1.16 | 600 | `-0.04em` |
| Stat | `--ps-text-stat` | 36 px / 1.0 | 650 | `-0.05em` |
| Page lead | `--ps-text-lead` | 15 px / 24 px | 400 | `-0.006em` |
| Body | `--ps-text-body` | 14 px / 21 px | 400 | `-0.006em` |
| Row title / strong | `--ps-text-strong` | 14 px / 20 px | 600 | `-0.006em` |
| Small / nav | `--ps-text-small` | 13 px / 20 px | 400–600 (nav 600, active 700) | `-0.004em` |
| Meta | `--ps-text-meta` | 12 px / 18 px | 400–600 | 0 |
| Micro / caps labels | `--ps-text-micro` | 11 px / 16 px | 650–750 + uppercase | `+0.06em` |
| Section label | `--ps-text-section` | 12 px / 16 px | 750 + uppercase | `+0.08em` |
| Keycap | `--ps-text-kbd` | 11 px / 14 px | 650 | `+0.02em` |
| Control / button | `--ps-text-control` | 13 px / 20 px | 700 | `-0.004em` |
| Field label | `--ps-text-micro` | 11 px caps / 16 | 700 | `+0.06em` |
| Launcher query | `--ps-text-input` | 21 px (unchanged; launcher only) | 430 | `-0.04em` |

Exact token override (L0 lands in `tokens.css`; ui-app may temporarily keep it in an app CSS file scoped to the window,
marked TEMP — do not scope to `.app-shell` only, because onboarding is in the same main window):

```css
html[data-window="main"] {
  --ps-text-display: clamp(28px, 3.6vw, 44px);
  --ps-text-h1-setup: 34px;
  --ps-text-stat: 36px;
  --ps-text-lead: 15px;
  --ps-text-body: 14px;
  --ps-text-strong: 14px;
  --ps-text-small: 13px;
  --ps-text-meta: 12px;
  --ps-text-micro: 11px;
  --ps-text-section: 12px;
  --ps-text-kbd: 11px;
  --ps-text-control: 13px;
  --ps-leading-display: 1.08;
  --ps-leading-setup: 1.16;
  --ps-leading-lead: 24px;
  --ps-leading-body: 21px;
  --ps-leading-small: 20px;
  --ps-leading-meta: 18px;
  --ps-leading-micro: 16px;
  --ps-content-max: 1120px;
  --ps-app-margin: 40px;
  --ps-app-gutter: 24px;
  --ps-app-card-pad: 24px;
  --ps-app-section-gap: 32px;
  --ps-app-label-col: 280px;
  --ps-app-sidebar: 232px;
  --ps-row-nav: 44px;
  --ps-row-list: 68px;
  --ps-control-height: 36px;
  --ps-field-height: 40px;
  --ps-app-metric-row: 44px;
}
@media (max-width: 1099px) { html[data-window="main"] { --ps-app-margin: 32px; --ps-app-sidebar: 200px; } }
@media (max-width: 819px)  { html[data-window="main"] { --ps-app-margin: 24px; --ps-app-gutter: 16px; --ps-app-sidebar: 64px; } }
@media (max-width: 700px)  { html[data-window="main"] { --ps-app-margin: 20px; } }
html[data-window="main"][data-density="compact"] {
  --ps-row-nav: 38px; --ps-row-list: 60px; --ps-control-height: 32px; --ps-field-height: 36px;
}
```

Density impact: cards and rows grow with the type — nav 44 px, list 68, metric 44, control 36, field 40, card padding 24.
At 560 px window width the app content is ~472 px wide (64 rail + 24 margins): body 14 / meta 12 still fit at 40–55 ch;
label columns stack.
DPI: sizes are CSS px; at 125/150/200 % they render 1.25/1.5/2× physical. Verify the 44 px display at 200 % and the 11 px
caps at 100 %; never add a size below 11 px to fit narrow layouts — wrap or stack instead.

### 4.3 Weight map (reference → token)

| Reference weights | Token | Fallback (Segoe UI Variable) |
|---|---|---|
| 400–430 | `--ps-fw-regular: 400` / `--ps-fw-input: 430` | 400 |
| 600–660 | `--ps-fw-semibold: 600` | 600 |
| 670–700 | `--ps-fw-strong: 680` | 700 |
| 710–790 | `--ps-fw-label: 750` | 700 |

### 4.4 Rules

- **Numbers:** `font-variant-numeric: tabular-nums` on counts, stats, metrics, durations, percentages and any
  updating value; right-align numbers in tables and metric rows (`.num`).
- **Truncation:** single-line ellipsis for names/paths; every truncation container needs `min-width: 0` and a
  `title`/accessible full value. Paths truncate from the **middle** (drive + ellipsis + leaf) when a full path is
  shown; the result row path truncates at the **end** (reference behaviour: start of hierarchy is more useful).
- **Measure:** paragraphs 45–75 ch; the route lead is capped at `max-width: 60ch`; no full-width one-line leads.
- **Case & punctuation:** sentence case for headings/buttons; numerals for counts ("8 results", not "eight");
  typographic `…` not `...`; curly quotes in prose copy; no exclamation marks in product copy; `&` only where
  the reference uses it ("Search & indexes").
- **Locale:** all dates/times/numbers via `Intl.DateTimeFormat` / `Intl.NumberFormat`; never hard-code
  "Yesterday" — use relative formatting or the localized string.
- **Minimum sizes:** app/onboarding body ≥14 px, sentence-case meta ≥12 px, caps/kbd ≥11 px; launcher keeps its
  final 13/11/10 scale. Nothing below 10 px anywhere at 100 % DPI.
- **DPI:** font sizes are CSS px; at 125/150/200 % the WebView renders them at 1.25/1.5/2× physical size. Verify
  the 10 px caps at 100 % (the riskiest) and the 41 px display at 200 % (the largest).

---

## 5. Spacing, radii, borders, shadows

### 5.1 Spacing scale (landed as `--space-1` … `--space-11`)

`4, 8, 12, 16, 20, 24, 32, 40, 48, 56, 64` px. Use these for layout, section and panel rhythm; do not
add global steps. Component anatomy keeps the reference's local values (search row 14/20, results 10/11/13, row
8/12, footer 9/16) as component CSS — they are not new tokens. No 3/5/7/9/13/15 px gaps in layout.

Reference-derived metrics to preserve:

| Context | Value |
|---|---|
| Launcher search row | 72 px tall, 14 px 20 px padding, 14 px gap (reference:30) |
| Launcher results | padding 10 px 11 px 13 px; max-height 365 px (reference:33) |
| Result row | 8 px 12 px padding, min-height 58 px, 12 px gap, radius 8 (reference:35-36) |
| Launcher footer | min-height 44 px, 9 px 16 px padding (reference:45-46) |
| Launcher native window | card + **28 px transparent pad** on every side; window = card + 56 (876 / 1236 wide); min window 560×352 |
| Launcher filter strip | 40 px block (36 compact): 7 px 16 px padding, 26 px chips, 6 px gap, 1 px bottom hairline |
| App sidebar | reference 190 px; production **232 / 200 / 64 rail** (§5.5); padding 24 px 12 px |
| App workspace | reference 36/40; production margins **40 / 32 / 24 / 20** at the four bands, content max 1120, 12-col grid gutter 24/16 (§5.5) |
| Onboarding rail | 240 px wide, 32 px 25 px padding (reference:82-83) |
| Onboarding content | 44 px 43 px (reference:87) |
| Two-column layout | reference 1.25/0.9; production **7/5 of 12 columns**, gap 24 px (§5.5) |
| Section rhythm | launcher 24 px; **app: 32 px between sections, 24 px between panels, 24 px card padding** (§5.5) |

### 5.2 Radii

`--ps-radius-kbd: 4px` · `--ps-radius-control: 6px` · `--ps-radius-field: 7px` · `--ps-radius-row: 8px` ·
`--ps-radius-segment: 9px` · `--ps-radius-window: 12px` · `--ps-radius-launcher: 14px` · `--ps-radius-full: 999px`.

Policy: never more than three panel radii on one screen (8 / 12 / 14); nested radii step down (14 → 8 → 7 → 4);
square is not used on interactive controls; the file-type badge is 7 px.

### 5.3 Borders and hairlines

`--ps-border: 1px`; `--ps-focus-width: 2px`; `--ps-focus-offset: 2px`; `--ps-selected-bar-width: 2px`.
A surface uses **either** a 1 px border **or** a tone step, never both; shadows only for floating surfaces.
Hairlines must be exactly 1 CSS px (never 0.5 px) so they survive DPI rounding.

### 5.4 Shadows (floating surfaces only) — launcher shadow FINAL 2026-11-09

| Token | Value | Used on |
|---|---|---|
| `--ps-shadow-launcher` (light) | `0 1px 2px rgba(21,44,24,0.10), 0 3px 6px rgba(36,60,43,0.08), 0 6px 12px rgba(36,60,43,0.10), 0 0 0 1px rgba(20,43,49,0.05)` | launcher card (light) |
| `--ps-shadow-launcher` (dark) | `0 1px 3px rgba(0,0,0,0.50), 0 3px 8px rgba(0,0,0,0.40), 0 6px 12px rgba(0,0,0,0.45), 0 0 0 1px rgba(236,239,232,0.08)` | launcher card (dark) |
| `--ps-shadow-app` | `0 18px 45px rgba(33,59,40,0.08)` | app/onboarding window (native shadow preferred) |
| `--ps-shadow-popover` | `0 8px 24px rgba(32,43,49,0.14), 0 1px 3px rgba(32,43,49,0.06)` | context menu, tooltip |
| `--ps-shadow-dialog` | `0 24px 64px rgba(32,43,49,0.20)` | confirm dialog |
| dark popover/dialog | `0 8px 24px rgba(0,0,0,0.45), 0 1px 3px rgba(0,0,0,0.35)` / `0 24px 64px rgba(0,0,0,0.55)` | |
| `--ps-launcher-shadow-pad` | `28px` | transparent room around the launcher card |

Resting cards: **no shadow**. No inset shadows, no glow, no glass, no backdrop blur.

#### Launcher shadow clipping contract

The launcher window is transparent and padded by `--ps-launcher-shadow-pad: 28px` on every side (Rust
`LAUNCHER_SHADOW_PAD = 28.0`). Any shadow extent larger than 28 px is hard-clipped by the native window edge — the
"shadow doesn't look proper" defect. The old value `0 24px 56px` has extent `24 + 1.5×56 ≈ 108 px` and was cut at
28 px, so its corner/tail vanished into a straight edge.

- **Extent budget:** for every layer, `|offsetY| + 1.5 × blur ≤ 24 px` (4 px headroom for the 1 px border and AA).
  No spread radius on the launcher.
- **Extents:** contact 4–5.5 px, mid 12–15 px, ambient **24 px**, rim 1 px. Top extent ≤ 12 px, sides ≤ 12 px.
- **Card:** `border: 1 px solid var(--ps-line-strong); border-radius: 14px; overflow: hidden`. The shadow is drawn
  from the card, so the radius reads correctly; never move the shadow to the native window.
- **Dark theme:** black ambient is nearly invisible on a dark desktop; the 1 px `rgba(236,239,232,0.08)` rim plus
  the `#3d423a` border is what makes the edge legible without a glow.
- **Layout:** `html[data-window="launcher"] #root { padding: 28px; display:flex; align-items:stretch;
  justify-content:center; background:transparent }`; `.launcher { width: min(820px, calc(100vw - 56px));
  height: 100%; max-height: none; margin: 0 }` (preview: `min(1180px, calc(100vw - 56px))`). The card must not touch
  the window edge.
- **Hit testing:** the 28 px pad belongs to the launcher window; a primary-button press there dismisses the launcher
  (same as outside-click) rather than reaching the app behind. No whole-window click-through.
- **DPI:** pad and shadow are CSS px and scale together; verify 125/150/200 % has no straight cut-off line.

### 5.5 Application alignment grid (app + onboarding, FINAL 2026-11-09)

One grid for every app route; components snap to it. Base unit **4 px**, vertical rhythm **4/8 px** (every padding, gap
and height is a multiple of 4).

| Token / metric | Value |
|---|---|
| `--ps-content-max` | 1120 px content (excluding window padding) |
| `--ps-app-margin` | 40 px ≥1100 · 32 px 820–1099 · 24 px 700–819 · 20 px ≤700 |
| `--ps-app-gutter` | 24 px ≥820 · 16 px ≤819 |
| Grid | **12 columns**, `grid-template-columns: repeat(12, minmax(0, 1fr)); column-gap: var(--ps-app-gutter)` |
| Sidebar | 232 / 200 / 64 rail |
| Home columns | left **7**, right **5** (gap 24), stacked ≤819 |
| Settings / Indexes | label column **280 px** + 24 gap + panel (label col 3-ish columns); stacked ≤1023 |
| Card / panel padding | **24 px** (was 20); compact 16 |
| Section gap | **32 px**; panel gap 24; page-header → content 32 |
| Row heights | nav 44 · list 68 · metric 44 · control 36 · field 40 · buttons 28/36/40 |
| Icon sizes | 16 inline/nav · 20 state/panel · 32 file/app |
| Page header rhythm | eyebrow → 8 → h1 → 12 → lead (max 60 ch) → 32 → content; route bottom padding 64 |

Rules:
- **One left edge and one right edge.** Eyebrow, h1, lead, search entry, first panel and section labels all start on
  column 1; the last card/panel ends on column 12. No element stops 1–2 px short of the grid; no centred cards.
- **Panels span columns, not magic widths.** Home: 7/5. Settings: label 280 + panel to the right edge. Indexes: full
  12. Lists/tables fill their panel; only icon and trailing columns are fixed.
- **Numbers:** right-aligned, `font-variant-numeric: tabular-nums`; label left `--ps-text-small`/ink-2, value right
  `--ps-text-body`/600/ink; units in `--ps-text-meta`/ink-2; zero prints "0", unavailable prints "—" (never a
  large dash); deltas carry an explicit sign and are never colour-only.
- **Vertical rhythm:** section headings sit on the same baseline grid; a panel's first line is 24 px from the top
  edge; row text is vertically centred in a 4 px-multiple row; no 3/5/7/9/13/15 px gaps.
- **All type comes from tokens** (`--ps-text-*` / `--ps-leading-*`). Hard-coded `font-size: 12px` in a view is a
  violation (current offenders: `styles/base.css:113`, `styles/shell.css:12,17`, `onboarding/onboarding.css:582`).

**Violations to fix in the current code** (audit checklist for ui-app):
1. `desktop.css:24` sidebar 220 and `:897` 190 / `:920` 56 → 232 / 200 / 64 with the window token.
2. `desktop.css:172` workspace `36px 40px 64px` → margin token + 24 grid gutter; `:901` 32/32; `:963` 31/24;
   `:983` 31/18 → 40/32/24/20 margins; no 31 px.
3. `desktop.css:252-253` Home `1.25fr/.9fr gap 38` → 7/5 of 12 with 24 gap (and 16 gap ≤819).
4. `desktop.css:813` settings `200px` label → 280 + 24 gap; stack ≤1023.
5. `ui.css:36` panel `padding: var(--space-5)` (20) → `--ps-app-card-pad` (24); `radius 12` stays.
6. Row heights: nav/list/metric/control/field tokens currently 40/64/—/32/38 → 44/68/44/36/40.
7. `base.css:113` button `font-size: 12px` literal and `:98-99` padding 9/14 → `--ps-text-control` (13) and
   padding 8/16 (md 36 high).
8. Page-header rhythm is currently loose (lead margin ~25 px, header → content 37 px) → 8/12/32 exact.
9. Icon sizes mix 15/16/18/19 → 16 inline/nav, 20 state, 32 file/app.
10. Raw error text rendered directly at `IndexesView.tsx:275`, `HomeView.tsx:122`, `ActivityView.tsx:155,204`,
    `WorkflowsView.tsx:151`, `PreferencesView.tsx:132` → §8.3 + SCREENS §3.7 (human sentence, Details collapsed).
11. Mixed radii 8/12 on panels and controls → panel 12, control 6, row 8, field 7; no new radii.
12. Centred/left-aligned state blocks: app empty/error blocks must be **left-aligned to the grid** (no centred
    full-width copy, no illustration).

---

## 6. Motion

### 6.1 Principles

1. **Input beats animation.** Selection updates paint in the same frame as the key press; no animation may gate
   state, focus, or the Enter action.
2. **Motion only where it explains a change.** The launcher card opens/closes (150/110 ms, §6.3); the app shell reveals
   at 160 ms. Input, list selection, results, Enter and window resizes stay instant. Preview, menus, onboarding steps
   and progress are the only other animated surfaces.
3. **Transform + opacity only** (never width/height/margin/padding/top/left). If a transition can cause layout,
   it is not used.
4. **No bounce, no spring overshoot, no hover lift.** Easing is ease-out or standard; nothing leaves the frame.
5. **Every animation has a reduced-motion branch** (§6.3).

### 6.2 Tokens

| Token | Value | Use |
|---|---|---|
| `--ps-dur-0` | 0 ms | keyboard list selection, focus ring, window resize, theme switch |
| `--ps-dur-fast` | 90 ms | button press, context-menu fade, inline status |
| `--ps-dur-base` | 110 ms | hover/selection colour, nav hover |
| `--ps-dur-panel` | 160 ms | preview panel content, onboarding step content, popover scale |
| `--ps-dur-progress` | 240 ms | determinate progress interpolation |
| `--ps-dur-loop` | 1200 ms | indeterminate progress sweep |
| `--ps-dur-launcher-in` | 150 ms | launcher card open |
| `--ps-dur-launcher-out` | 110 ms | launcher card close |
| `--ps-ease-standard` | `cubic-bezier(0.2, 0, 0, 1)` | default; UI state changes |
| `--ps-ease-out` | `cubic-bezier(0.23, 1, 0.32, 1)` | entering elements |
| `--ps-ease-in-out` | `cubic-bezier(0.77, 0, 0.175, 1)` | on-screen movement (preview content) |
| `--ps-ease-linear` | `linear` | progress only |
| `--ps-ease-out-expo` | `cubic-bezier(0.16, 1, 0.3, 1)` | launcher open, app shell entry |
| `--ps-ease-launcher-out` | `cubic-bezier(0.4, 0, 0.2, 1)` | launcher close |

### 6.3 Recipes and reduced motion

| Surface | Motion | Reduced motion |
|---|---|---|
| Launcher open (summon) | card: opacity 0→1 + translateY(-8px→0) + scale(0.985→1), 150 ms ease-out-expo; focus + typing immediate | instant show, no transform |
| Launcher close (Esc/outside/open) | card: opacity 1→0 + translateY(0→-4px) + scale(1→0.99), 110 ms ease-launcher-out; the action runs immediately | instant hide, 0 ms |
| App window reveal | shell only: opacity 0→1 + translateY(4px→0), 160 ms ease-out; window painted first, route changes instant | 0 ms |
| Result selection (↑/↓) | `background-color`, `border-color`, `box-shadow inset` 110 ms standard; no transform; retargets on rapid presses (CSS transition, no queue) | 0 ms |
| Hover on row/button | `background-color` 110 ms | 0 ms |
| Focus ring | none (appears instantly) | same |
| Preview toggle (Ctrl+Space) | panel content: opacity 0→1 160 ms ease-out + translateX(±6 px) 160 ms; native window resize is instantaneous | opacity only, 0 ms travel |
| Context menu | opacity 0→1 90 ms + scale 0.98→1 120 ms, transform-origin at the row edge | 0 ms |
| Onboarding step change | content: opacity 0→1 160 ms + translateY(4 px→0) 160 ms ease-out; rail state instant | 0 ms |
| Determinate progress | fill `transform: scaleX()` 240 ms linear; value text updates immediately | no interpolation; instant |
| Indeterminate progress | 1200 ms linear translateX loop, static 30 % bar, no gradient/shimmer | static 30 % bar, no loop; `aria-valuetext` still announces phase |
| Icons/logo | no animation ever | same |

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important; scroll-behavior: auto !important; }
}
```

Progress animations above are post-alpha only: v2.0 alpha renders no progress bar (no percentage in `EngineStatus` —
SCREENS §10).

### 6.4 Exact launcher open/close CSS (copy into launcher.css) — FINAL 2026-11-09

```css
.launcher { opacity: 1; transform: none; }
.launcher[data-state="opening"] {
  animation: ps-launcher-in var(--ps-dur-launcher-in) var(--ps-ease-out-expo) both;
  will-change: opacity, transform;
}
.launcher[data-state="closing"] {
  animation: ps-launcher-out var(--ps-dur-launcher-out) var(--ps-ease-launcher-out) both;
  will-change: opacity, transform;
  pointer-events: none;
}
@keyframes ps-launcher-in {
  from { opacity: 0; transform: translateY(-8px) scale(0.985); }
  to   { opacity: 1; transform: none; }
}
@keyframes ps-launcher-out {
  from { opacity: 1; transform: none; }
  to   { opacity: 0; transform: translateY(-4px) scale(0.99); }
}
:root[data-motion="reduced"] .launcher[data-state] {
  animation: none !important; opacity: 1; transform: none;
}
@media (prefers-reduced-motion: reduce) {
  .launcher[data-state] { animation: none !important; opacity: 1; transform: none; }
}
```

Shell/JS contract (ui-launcher, with L0 for the Rust safety timer):
1. Summon: show window, focus input **now**, set `data-state="opening"`, on `animationend` set
   `data-state="open"`. Typing and search dispatch are never delayed.
2. Dismiss: run the action immediately; set `data-state="closing"`; hide on `animationend` or a 200 ms fallback
   timer (reduced motion: hide next frame, timer 0). Copy actions never close.
3. Re-summon while closing: cancel the hide timer, set `data-state="opening"` again, do **not** hide. Rust's
   `dismiss_launcher` safety hide (currently 400 ms) must be cancellable with a generation token — L0 task.
4. The shadow never animates; window height/width changes are instant (never transitioned).
5. App shell reveal stays as today (`.app-shell.is-opening`, 160 ms, 4 px), fires only when the window becomes visible;
   route changes never animate; reduced motion shows instantly.

### 6.5 What must stay instant (regression list)

Typing/caret/IME, search dispatch, result-list replacement and count text, selection change (110 ms colour only), Enter
and the resulting action, focus changes, window resize/geometry, preview side/drawer resize, filter application (the
search dispatches immediately; only the chip colour transitions), footer status text, scrolling (no smooth scroll), and
everything under reduced motion.

### 6.6 Frame budget and React rules

60 Hz = 16.7 ms; 120 Hz = 8.3 ms (master prompt §V.D). Selection change must not re-render the whole result list:
key rows by stable result id, memoize row components, toggle a single `data-selected` attribute, keep
`aria-activedescendant` on the input. Virtualize result lists over 40 rows. Never read layout in the key handler.
CSS transitions for predetermined motion; no JS animation loops for UI state (emil-design-eng: CSS beats JS under
load; use WAAPI only for interruptible progress if needed).

---

## 7. Density

| Token | Comfortable (default) | Compact |
|---|---|---|
| Launcher result row | 58 px min-height, 8/12 px padding | 48 px, 6/10 px |
| Launcher group label | 34 px block (14 px top / 8 px bottom + 14 px line) | 30 px (10/6) |
| App nav row | **44 px** | **38 px** |
| App list/table row | **68 px** | **60 px** |
| App metric row | **44 px** | 40 px |
| Button height | **36 px** (sm 28 / lg 40) | **32 px** (sm 28) |
| Field height | **40 px** | **36 px** |
| Panel padding | **24 px** | 16 px |
| Stack gap | 16 px | 12 px |

Density is a user preference (Preferences → Appearance → Density) applied with `[data-density]` on the app root;
the launcher reads the same setting. Type sizes do not change. Touch/pointer targets never shrink below 24×24 px
(WCAG 2.2 AA), and rows remain ≥44 px tall only when the OS reports a coarse pointer
(`@media (pointer: coarse)` bumps rows to 44 px minimum) — on a Windows desktop, fine-pointer compact sizes apply.

---

## 8. Component inventory

State names: rest · hover · active(press) · focus-visible · selected · disabled · loading · error. Every component
below must implement all states that apply, including empty/loading/error at its container level. ARIA is in
`KEYBOARD_AND_A11Y.md`; this table is the visual/behavioural contract.

### 8.1 Component map

| # | Component | Anatomy | Sizes | Key states / notes |
|---|---|---|---|---|
| 1 | **LauncherWindow** | frameless floating surface, radius 14, 1 px `--ps-line-strong` border, layered `--ps-shadow-launcher`, 28 px transparent shadow pad; column: SearchRow / FilterStrip / ResultsViewport / Footer | card 820 (preview 1180) wide; card 296–544 px tall (window = card + 56) | no title bar, no window buttons, no drag strip over controls; open 150 ms / close 110 ms (§6); resize instant |
| 2 | **SearchRow** | 29 px brand mark (light/dark variant) · 14 px gap · input · match count (11 px `--ps-ink-3`) | 72 px tall (61 px narrow), 20/14 px padding | input 21 px/430; placeholder `#69746e`; no Ctrl+K pill, no clear button, no fake badge |
| 3 | **ResultsViewport** | scroll container, 140–365 px, thin scrollbar | padding 10/11/13 | loading skeletons on first query, 0-match empty state, inline error; virtualize >40 rows |
| 4 | **ResultGroup** | tracked caps 10 px/750/+0.12em, `--ps-ink-3`, 14/13/8 px padding | block 34 px | label is a real element (`role="group"` + `aria-label`); no count, no rule |
| 5 | **ResultRow** | 36 px icon slot · 12 px gap · (title 13/680 · path 11 `--ps-ink-2`) · meta 10–11 px right | 58 px; 48 compact | rest transparent; hover `#f0f3f0`; selected `--ps-accent-soft` + 1 px `--ps-accent-border` + inset 2 px `--ps-selected-bar`; focus never replaces selection; disabled dimmed with reason tooltip |
| 6 | **ResultIcon** | 35×37 slot radius 7; real shell icon 32 px **or** letter badge or command glyph | 32 px icon within 36 slot; 1×/2× source | loading neutral tile (no shimmer), missing icon → badge, never stretch/upscale; decorative for AT |
| 7 | **PreviewPanel** | side panel 360 px when `workArea.width ≥ 1200`, otherwise bottom drawer: head (icon/title/format/path), real `PreviewPayload` body (text/code with highlight, image max 300, OCR, facts), actions row; 160 ms/6 px fade | side: 360 × content; drawer: 240 px; image max-height 300; text max-height 300 | toggled by Ctrl+Space; `aria-label="Preview"`; loading flat skeleton / unsupported / error + Retry; debounce 120 ms + late-token guard; Esc closes; focusable scroll region; real IPC only, no fake body |
| 8 | **LauncherFooter** | left: 8 px status dot + status text + · + "Open ProtonSearch" text button; right: 3 kbd hints max | 44 px; 9/16 padding | hints contextual (Navigate/Open/Clear→Close/Preview/Copy path); Paste-N CUT for alpha; text 10 px; hidden by priority <900 px; status is live region |
| 9 | **Kbd** | 10 px/650, 2 px 5 px padding, radius 4, bg `--ps-kbd-bg`, 1 px border | auto | non-interactive; `aria-hidden` when the hint text already names the key? No — keep text visible, hide the decorative keycap from AT via `aria-label` on the hint group |
| 10 | **Button** | primary (fill `--ps-primary-bg`, radius 6, padding 8/16, **13/700**) · secondary (white, 1 px `--ps-control-border`, ink `#365a69`) · ghost/text (ink accent) · danger-text (negative, confirm required) | sm 28 · md **36** · lg **40** | hover/active/disabled/loading (label + 12 px dot? no spinner: label + aria-busy); one filled primary per screen; icon-only needs aria-label |
| 11 | **TextField / SearchField** | label above (**11 px caps 700**, 8–10 px gap), field **40 px**, radius 7, 1 px `--ps-control-border`, **14 px** ink, placeholder `#69746e` | **40 px** (36 compact); big search **52 px** | focus: 2 px `--ps-focus` ring offset 2 (not a border swap); invalid: negative border + inline error; disabled legible with reason; helper via aria-describedby |
| 12 | **CheckboxRow / ChoiceRow** | 16 px checkbox + label **14/600** + description **12 px** right or below; whole row is the hit target | **44–48 px** | checked/unchecked/indeterminate? no; focus ring on row; disabled row keeps 4.5:1 text |
| 13 | **SegmentedControl** | track `--ps-surface-soft`, 4 px padding, radius 9, buttons radius 6, **13/600** | **36 px** | selected = white/raised fill + ink; keyboard roving tabindex; used for theme, density, activity filters, per-root depth mode |
| 14 | **ToggleSwitch** | 28×16 track, 12 px thumb, label + status word | **44 px row** | on = `--ps-accent` (not green); focus ring; disabled reason |
| 15 | **ProgressBar** | 4 px track radius 3, fill `--ps-progress-fill`; label + "42 %" + detail line | 4 px bar; 3-line block | determinate (value 0–100 real), indeterminate (static 30 % + phase text), paused, error; `role="progressbar"`; never fake a number; **not rendered in v2.0 alpha** (IPC has no percentage — SCREENS §10) |
| 16 | **Panel / Card** | 1 px `--ps-line` or tone step, radius **12**, padding **24** (reference:76); no shadow at rest | auto | used for grouped content; never a 3-up feature grid |
| 17 | **SectionHeader** | eyebrow **11/750/+0.14em** · h1 display · lead **15/24** `--ps-ink-2` (max 60ch) · h2 section label **12/750/+0.08em** | — | one h1 per route; eyebrow may be omitted before an h1 |
| 18 | **MetricRow** | label **13 px** `#5c6862` + value **14/600** ink right, 10 px vertical padding, 1 px row rule | **44 px** | numbers tabular; last row no rule (reference:74) |
| 19 | **StatBlock** | stat **36/650** + subline **12 px** `--ps-ink-2` + state line + metrics | — | one per screen; the number is real; no progress in alpha (SCREENS §10) |
| 20 | **RecentRow** | 32×32 icon · title **14/600** · sub **12** `--ps-ink-3` · trailing arrow | **68 px** | hover row tint; whole row is one button |
| 21 | **EmptyState** | app: left-aligned block, optional 20 px outline icon, title 15/600, one 14/21 `--ps-ink-2` sentence (max 60 ch), one action; launcher keeps its centred 13 px empty copy | app block min-height 120 px; launcher 140 px | empty ≠ error; keeps the frame; no illustration, no emoji; copy in SCREENS §3.7 |
| 22 | **ErrorState / InlineNotice** | quiet `.state` block (§8.3): surface bg, 1 px line border, 20 px semantic icon, title 15/600, one 14/21 sentence, one action, optional collapsed Details | padding 20/24; radius 12 | errors live in the block that failed, never a full-screen banner; no red fill; `role="alert"` only for newly triggered failures, `role="status"` for load failures |
| 23 | **StatusDot** | 8 px dot + text label 11 px | — | colour + word; `aria-hidden` on dot; ready `#4f7d68`, indexing `#8a5a12`, paused `#68736d`, error `#a92a2f` (light) / equivalents dark |
| 24 | **Badge / Pill** | 16–20 px tall, radius full, pale tint + dark ink, 11/650 | — | status only, inside rows/beside values; never above a heading; no count badges unless a real queue count |
| 25 | **Sidebar** | brand block, "WORKSPACE" label, 5 nav items, userfoot at bottom | **232 / 200 / 64 rail** | active = neutral tint + ink + 2 px `--ps-selected-bar` start bar; no coloured pill; rail keeps sr-only labels |
| 26 | **Avatar** | **32 px** tile radius 8, initial 700 or person glyph when no name | — | optional name only; neutral fallback "This PC" |
| 27 | **StepRail / StepIndicator** | **28 px** numbered circle + label **13/600**; current filled `#295a70` white; completed accent ink; future muted | 7 steps, **56 px** pitch | rail hidden ≤700 px; current step announced; non-interactive |
| 28 | **ContextMenu** | floating panel radius 8, 1 px border, popover shadow, 12 px items, 6 px vertical padding, shortcut column right | min 180 px | keyboard: focus moves into menu, Esc returns to row; destructive item last, in negative colour, confirm |
| 29 | **Tooltip** | dark-ink surface 11 px, radius 4, 4×8 padding, 400 ms delay after first, none on subsequent | — | only for icon-only controls or truncated text; never for hints already in the footer |
| 30 | **Skeleton** | flat grey bar at real size, radius 4, no shimmer/no gradient | — | layout identical to loaded state; no CLS |
| 31 | **InlineStatus** | footer/inline confirmation text ("Path copied"), 10–11 px, auto-clears 2 s | — | `role="status"`; not a toast |
| 32 | **FilterStrip** | four chips only: `role="radiogroup"`, one roving tab stop; selected = `--ps-accent-soft` fill + 1 px `--ps-accent-border` + 2 px inset bottom `--ps-selected-bar`; 11/600 uppercase | block 40 px (36 compact); chip 26 px (24 compact); gap 6; inline pad 16 | **ALL · FILES · FOLDERS · APPS** with Ctrl+1–4 from the input; all other scopes via prefixes/quick destinations; FILES/FOLDERS exact object types; horizontal scroll (no wrap/fade); spec SCREENS §2.11 |
| 39 | **QuickDestinations** | empty-state 2×2 destination grid + secondary pills: Browser Bookmarks, Browser History, Clipboard History, Agents; secondary Git history, Agent History; no fake destinations | primary tile min-height 54, padding 9 12, radius 9, gap 8/12; secondary pill padding 5 9 radius 6; heading 10/750 +.12em ink-3 | listbox options, real EngineStatus text/counts only; ↑/↓+Enter from input; prefixes written for search destinations; main app for Agents/history; quick height 248/224; spec SCREENS §2.4 |
| 33 | **OfflineNotice** | `.state.state--offline` variant: cloud-off icon, title "You're offline.", one sentence, optional Try again | same block metrics as ErrorState | only for genuinely network-dependent content; local search/indexing never shows it; `role="status"` |
| 34 | **DetailsDisclosure** | native `<details>`: summary "Details" 12/600 `--ps-ink-2`; content 12/18 `--ps-ink-2` — sanitized message + reference id only | — | collapsed by default; no stack traces, no paths, no raw `err.message`; omit entirely if there is nothing safe to show |
| 35 | **ClipboardCategorySection** | quiet section inside the results viewport for CLIPBOARD only: caps label + its rows, `role="group"` + `aria-labelledby`; label 10/700 `+0.10em` `--ps-ink-3`, no count/icon/control; no chip row, no row tags, no filtering controls | label block **28 px** (24 compact = 8 top + 14 line + 6 bottom), inline pad 14; **6 px** between sections; rows 58/48 | fixed order **LINKS · EMAILS · PATHS · CODE · TEXT · IMAGES**, only non-empty; flattened keyboard traversal (labels are not stops); standard card height (no +36 px); spec SCREENS §2.12 |
| 36 | **AgentRuntimeRow** | 68 px row: 24 px icon slot (20 px glyph) · name 14/600 · version badge 18 (11/650, app floor) · availability badge 18 · path 12/18 middle-ellipsis + `title` · detail 12/18 ink-3; Pi primary = 2 px start bar + "Default" tag + "Use"/"In use" | 68 px row; 1 px rule | only shell-detected runtimes, never invented; only Pi runnable; other detected = "Detected · not wired in this build" with no control; badges pair word+colour, no fake green |
| 37 | **AgentRunSummary** | three 44 px metric rows: Status (8 px dot + word), Started (localized `Today, 14:32`), Duration (tabular `0.1 s`/`m ss`) | 44 px rows; label 13 ink-2 · value 14/600 right | shown while running and after finished/failed/cancelled; no runs = "No runs yet."; status words Completed/Failed/Cancelled/Running, never colour-only |

### 8.2 Complex-component rules

**ResultRow.** The row is one interactive target (a `button`-like option). Selected state is applied in the same
frame as the arrow key; only the two affected rows change attributes. Hover on a non-selected row uses
`#f0f3f0` (light) / `#242722` (dark); hover never overrides selection. Multi-select is **CUT for v2.0 alpha** (SCREENS §10): one active row only — no check markers, no footer count;
do not build a stub. Disabled rows (source
unavailable) stay readable at 4.5:1 and explain why on hover/focus.

**SearchRow.** The brand mark is 29 px and optically centred (reference:25,30); at ≤700 px it drops to 26 px. The
count text is the **only** right-aligned element; it reads "3 matches" / "1 match" / "No matches" (never "0
results" and never a pill). The input has no visible chrome: the search row is the field. Placeholder:
"Search files, apps, and actions…" (reference:136). During a slow query the count shows "Searching…" with
`aria-live="polite"` throttled to 300 ms; previous rows stay visible (no flicker, no skeleton after the first
query).

**PreviewPanel.** Opens only on demand or when the user has enabled "Preview on selection" in Preferences (default off). Side panel (360 px) when `workArea.width ≥ 1200` — window widens to the 1180 px preview token; otherwise a 240 px drawer. Renders the real `PreviewPayload`: title/format/path head; text/code with real highlight ranges (mono 11/1.55, max 300 px), image (object-contain max 300, bordered wrap), OCR section ("Text found in image", dashed top border), engine fields, truncated note ("Showing the first {n} characters…"), unsupported reason or "No preview for this item."; error shows "Preview is unavailable right now." + sanitized detail + Retry. Loading = flat 3-line skeleton at real size (no layout shift; pulse gradient is a current code deviation to remove). Debounce 120 ms, cache by result id+max chars, late-token guard. No fake thumbnail/body; preview actions only from the engine ActionSpec.

**LauncherFooter.** Status (left) is a live region, colour + words, from `EngineStatus`: "Index ready",
"Indexing", "Index paused", "Index unavailable — Retry". Alpha: the IPC has no percentage — SCREENS §10. "Open ProtonSearch" is a text button
(10 px/750, `#356d88`) that opens the app with the current query. Hints (right) are max three, ordered by
priority: Navigate → Open → Clear/Close → Preview → Copy path. Multi-select/Paste-N hints are CUT for v2.0 alpha.
Hints are hidden by priority, never wrapped to a second line at ≥900 px; below 900 px the footer may use two rows
(window grows by 20 px).

**Launcher window & shadow.** The card floats inside a transparent 28 px window pad; every shadow layer must fit
`|offsetY| + 1.5×blur ≤ 24 px` (§5.4). The card fills the pad (do not centre a shorter card vertically); a press in the
pad dismisses the launcher. Native window shadow stays off.

**FilterStrip.** A scrolling chip row between the search row and the results list, inside the results column so the preview panel keeps full body height. **Final order: ALL · FILES · FOLDERS · APPS only**, selected with Ctrl+1–4 from the input (Ctrl+5+ unbound). Clipboard, web, documents/OCR, git and actions stay in ALL and are reached with prefixes/quick destinations; no ghost chips. FILES/FOLDERS are exact: FILES never renders folders, FOLDERS never renders files. Radio-group semantics, not tabs: ←/→ changes immediately and keeps focus; Tab lands on the selected chip then leaves for preview/footer; Home/End first/last. Selected state uses the 2 px accent bar (fill alone fails WCAG 1.4.11). Overflow scrolls horizontally; selected chip scrolls into view; no gradient/fade edges. Labels uppercase/localised; no counts. Full spec: SCREENS §2.11.

**QuickDestinations.** Empty query + ALL + no recents/results shows real destinations only: primary 2×2 grid (Browser Bookmarks, Browser History, Clipboard History, Agents) and secondary pills (Git history, Agent History). Heading 10/750/+0.12em ink-3; tiles min-height 54, padding 9 12, border 1 line, radius 9, raised; selected accent-soft/accent-border; labels 13/680, subtitles 10 ink-2 (off = ink-3). Search destinations write bookmarks:/history:/clipboard:/git: into the input; Agents/history open the main app. Counts/agent state from EngineStatus only; no fake destinations. Height 248/224; one-column ≤700. Listbox options, ↑/↓+Enter from input. Full spec SCREENS §2.4.

**ClipboardCategorySection.** Inside the results viewport, clipboard results are grouped into quiet sections when the
clipboard scope is active (typed `clipboard:`/`clip:` or Clipboard History quick destination), not a chip:
LINKS · EMAILS · PATHS · CODE · TEXT · IMAGES (fixed order, only non-empty; IMAGES only with real image rows). There is **no second chip row, no filtering control and no per-row tag** — the user already chose
Clipboard once; grouping is presentation, not another filter. Section label: 10/700 caps +0.10em, ink-3, 28 px block
(8/14/6), 14 px inline pad, 6 px between sections; each section is a `role="group"` labelled by its visible label; labels
are not tab stops and the listbox traversal stays flat across sections. Card height/window math is the normal launcher
formula (no extra 36 px). See SCREENS §2.12 for classification, empty/loading states and keyboard.

**Agents section.** Workflows' top panel lists only runtimes the shell actually detected: 68 px rows, real path/version,
availability word + badge, no invented rows. Only Pi is runnable in alpha; other detected CLIs show "Detected · not wired
in this build" with no control. "Use" selects Pi and scrolls to the run panel; recent runs are local metadata only
(max 10; status/time/duration, never prompts/paths/output). Run summary is three 44 px rows (Status/Started/Duration).
Quiet error tones follow §8.3; never raw provider text.

**Buttons.** One filled primary per screen. Secondary = white/raised with 1 px border and ink `#365a69`; hover
`#f0f4ef`. Tertiary = text only, accent ink. Destructive = text/outlined negative with a confirmation that names
the object ("Remove Documents from the index?"). Buttons never use gradients, never scale on hover, press is a
colour change only.

**Progress bars.** Truthful values only. Determinate shows "42 %" text next to the bar and a detail line with real
counts ("8,440 of 20,113 documents"). Indeterminate shows a static 30 % bar and phase text ("Preparing files…",
"Reading documents…"); no shimmer. Paused: bar stays at the last value, label "Paused"; error: bar freezes,
notice below with Retry. If the engine cannot report a percentage, do not invent one. **v2.0 alpha renders no progress bar at all** (the IPC
has no percentage — SCREENS §10); this spec is post-alpha.

### 8.3 Quiet state presentation — error / empty / offline (FINAL 2026-11-09)

One small family of state blocks. The user sees a **human sentence first**; technical detail is collapsed and optional.
No raw exception text, no error codes, no stack traces, no absolute paths, no red-washed panels.

| State | Class | Tone | Anatomy |
|---|---|---|---|
| Blocked / failed | `.state .state--error` | negative icon + 1 px `--ps-negative-pill-bg` border (no fill) | 20 px outline icon · title 15/600 ink · one 14/21 `--ps-ink-2` sentence (max 60 ch) · one action · optional Details |
| Needs attention | `.state .state--warn` | warning icon + 1 px `--ps-warning-pill-border` border | same anatomy; used for degraded/partial, not failure |
| Informational | `.state` | `--ps-ink-2` icon + `--ps-line` border | same anatomy; used for alpha limitations, not errors |
| Offline | `.state .state--offline` | neutral/cloud-off icon | title "You're offline.", one sentence, optional Try again; only network-dependent blocks |
| Empty | `.state .state--empty` | neutral | optional 20 px icon · title 15/600 · one 14/21 sentence · one action; left-aligned in its panel |
| Row-level failure (auto-retry / reconnectable only) | `.row-state` | inline | 16 px **warning** icon · 13/20 `--ps-ink-2` sentence · "Try again" 13/600 accent; no border or background. Blocking row failures use `.state--error` with the negative colour (recovering = warn, blocked = error). |

Metrics: block padding **20 px 24 px**; icon 20 px; gap 12; title 15/600/ink; text 14/21 `--ps-ink-2` max 60 ch;
action row margin-top 16, buttons 36 px (secondary) or text; details margin-top 12; radius 12; background
`--ps-surface`; one border. Never stack two negative blocks in one view; degrade to warn. Empty states keep their
frame, are left-aligned to the grid, and never use centred copy or illustration in the app.

Copy rules (canonical strings and the per-path map are in SCREENS §3.7): title ≤ 6 words in sentence case, ends with a
period; body ≤ 2 sentences, active voice, second person, says **what happened** and **what to do next**; never "Oops",
"Error", "Something went wrong", codes, paths, stack traces or exclamation marks; the action is specific ("Try again",
"Open Search & indexes", "Add location", "Show all"). Details disclosure: native `<details>`, summary "Details"
12/600 `--ps-ink-2`, content 12/18 `--ps-ink-2`, sanitized message + reference id only. Screen readers: load-time
failures use `role="status"` (polite, announced once); failures triggered by an action use `role="alert"` (assertive,
once); offline transitions use `role="status"`; never re-announce the same message on re-render. "Try again" keeps focus
on itself; on success the block is removed and the updated content is not announced twice.

---

## 9. Iconography

### 9.1 UI icons

- One set only: **Lucide** (ISC) bundled as tree-shaken React SVG components; 24 px viewBox, 1.5 px stroke,
  round caps/joins, `currentColor`, `aria-hidden="true"` unless the control is icon-only (then `aria-label`).
- Sizes: 16 px (nav, buttons, inline), 20 px (empty states, block headers), 14 px (inside fields, beside meta).
  Never scale an SVG below 14 px; use the 16 px variant.
- Required glyphs (minimum set): home, search, activity/clock, workflows/zap, settings, folder, file, app-grid,
  clipboard, globe, code/terminal, calculator, arrow-up-right, chevron-right, x, pin, copy, external-link, refresh,
  pause, play, alert-triangle, check, info, plus, trash, shield, monitor, keyboard, sun, moon, monitor-theme,
  download, more-horizontal, menu.
- No unicode glyph icons (the reference's `⌂ ⌕ ◷ ↗ ⚙` at reference:138 are placeholders; they render differently
  per font and are invisible to AT). Deviation recorded in REFERENCE_MAPPING.md.
- Nav icons 16 px in a 19 px slot (reference:58); grey (`--ps-ink-2`), active `--ps-nav-active-ink`; never in a
  tinted circle.

### 9.2 File-type icons and letter badges

Two-tier strategy required by master prompt §V.A ("first-class app icons, realistic file icons"):

1. **Real shell icon** (primary): extracted by `ps-windows`/tauri icon cache at 32 px logical, sourced at
   32/48/64/96 px for 100–300 % DPI. App icons and file icons use the real asset; never recolour, never mask.
2. **Letter badge** (fallback/loading): the reference's 35×37 tile, radius 7, 1 px border, 11 px/750 uppercase
   extension (max 3 chars: `PDF`, `MD`, `TXT`), semantic family tint (§3.2 badges). Folder uses a folder glyph,
   command results use a 16 px command glyph in the neutral tile.

Mapping: documents (doc/docx/rtf/odt/pdf/txt-ish) → word tint; source/markdown/config/text → code tint; folders →
folder tint; apps and shell actions → action tint; images/screenshots/scanned media → image tint
(`--ps-badge-image-bg`/`--ps-badge-image-ink`); everything else → neutral tint. Badges are deterministic from
the extension so the slot never jumps between queries. Slot is 36×36 in the row; icon 32×32 centred; no upscale
below 32 px source (fall back to badge instead of a blurry icon).

### 9.3 Sizes summary

| Context | Icon | Slot |
|---|---|---|
| Launcher result | 32 px (badge 35×37) | 36 px |
| App recent list | 32 px | 32 px |
| Sidebar nav | 16 px | 19–20 px |
| Buttons/inline | 14–16 px | auto |
| Empty/error states | 20 px | 24 px |
| Status dot | 8 px (dot) | 8 px |

### 9.4 Accessibility

Icons that duplicate adjacent text are `aria-hidden="true"`; icon-only buttons get `aria-label` and a tooltip;
file-type badges are decorative because the row's accessible name includes the type in the meta line; the brand
mark has `role="img"` + accessible name "ProtonSearch" and is not announced twice next to the wordmark.

---

## 10. Brand usage (summary; full spec in `assets/brand/README.md`)

- Assets: `assets/brand/protonsearch-mark.svg` (64, light surfaces ≥24 px),
  `protonsearch-mark-16.svg` (≤23 px), `protonsearch-mark-on-dark.svg` (dark surfaces ≥24 px),
  `protonsearch-mark-mono.svg` (currentColor; forced colours/tray/print). The referenced
  `assets/protonsearch-mark.svg` / `logo-concept.png` / `original-logo-crop-reference.png` **do not exist in the
  workspace** (`docs/implementation/WORK_BREAKDOWN.md:104`); the inline SVG at
  `protonsearch-v2-ui-reference.html:118` is the visual source and these files are reconstructions.
- Two-tone navy P: body `#102448`, facet `#1d5189`, counterform cut out (takes the surface; equals the reference's
  `#fafbf9` on light panels). No gradients, glow, outline, shadow, rotation, recolour, or stretching.
- Minimum size 16 px (use the 16 variant below 24 px). Clearspace ≥ 25 % of the mark height on all sides.
- Approved surfaces: light surfaces `--ps-surface`/`--ps-surface-raised`/`--ps-sidebar`; dark surfaces
  `--ps-surface`/`--ps-sidebar` with the on-dark variant; monochrome when only one colour is available.
- Wordmark: "ProtonSearch" in the UI face, 12 px/750, tracking `-0.035em` beside the 29 px mark (reference:25,
  54). Never "Proton Search", never "ProtonSwitch".
- The brand navy is **not** a UI accent: interactive colour stays `--ps-accent #285975` so the mark remains the one
  saturated element.

---

## 11. Token file (paste-ready contract)

L0 lands this in `apps/desktop/src/styles/tokens.css`; values are normative, names may be namespaced but must keep
the semantics.

```css
:root {
  color-scheme: light;
  --ps-font-ui: "Inter", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  --ps-font-mono: "Cascadia Mono", "Cascadia Code", Consolas, monospace;
  --ps-fw-regular: 400; --ps-fw-input: 430; --ps-fw-semibold: 600; --ps-fw-strong: 680; --ps-fw-label: 750;

  --ps-ground: #e8ebe8; --ps-surface: #fbfcfa; --ps-surface-raised: #ffffff;
  --ps-surface-soft: #f2f5f1; --ps-sidebar: #f3f6f2;
  --ps-line: #e0e5df; --ps-line-soft: #e8ece7; --ps-line-strong: #cdd5ce; --ps-control-border: #d5dfd6;
  --ps-ink: #202b31; --ps-ink-2: #5c6862; --ps-ink-3: #68736d;
  --ps-accent: #285975; --ps-accent-strong: #1e4158; --ps-accent-soft: #e7eef2;
  --ps-accent-border: #d6e2e9; --ps-selected-bar: #285975; --ps-focus: #5482a3;
  --ps-nav-active-bg: #e0eae8; --ps-nav-active-ink: #234759;
  /* active nav item also carries a 2px inset var(--ps-selected-bar) bar */
  --ps-avatar-bg: #dae7e8; --ps-avatar-ink: #245168;
  --ps-primary-bg: #234a61; --ps-primary-ink: #ffffff; --ps-primary-hover: #335e75;
  --ps-positive: #247b50; --ps-warning: #8a5a12; --ps-negative: #a92a2f;
  --ps-positive-pill-bg: #dcfae5; --ps-positive-pill-ink: #04250e;
  --ps-negative-pill-bg: #ffedea; --ps-negative-pill-ink: #831c1e;
  --ps-warning-pill-bg: #fffbeb; --ps-warning-pill-ink: #92400e; --ps-warning-pill-border: #fde68a;
  --ps-kbd-bg: #f0f3ef; --ps-kbd-border: #dbe2da; --ps-kbd-ink: #5d6f65;
  --ps-progress-track: #e5ebe5; --ps-progress-fill: #407188; --ps-scrollbar-thumb: #d9dfda;
  --ps-scrim: rgba(32, 43, 49, 0.32);

  --ps-badge-word-bg: #e5eef5; --ps-badge-word-ink: #285e8b;
  --ps-badge-folder-bg: #eee9dc; --ps-badge-folder-ink: #7d5a20;
  --ps-badge-code-bg: #e9eee8; --ps-badge-code-ink: #386d58;
  --ps-badge-action-bg: #e7e9eb; --ps-badge-action-ink: #436276;
  --ps-badge-image-bg: #eae8ef; --ps-badge-image-ink: #574f68;
  --ps-badge-neutral-bg: #e8ece7; --ps-badge-neutral-ink: #526a66;

  --ps-radius-kbd: 4px; --ps-radius-control: 6px; --ps-radius-field: 7px; --ps-radius-row: 8px;
  --ps-radius-segment: 9px; --ps-radius-window: 12px; --ps-radius-launcher: 14px; --ps-radius-full: 999px;
  --ps-border: 1px; --ps-focus-width: 2px; --ps-focus-offset: 2px; --ps-selected-bar-width: 2px;

  --ps-launcher-shadow-pad: 28px;
  --ps-shadow-launcher: 0 1px 2px rgba(21, 44, 24, 0.10), 0 3px 6px rgba(36, 60, 43, 0.08),
    0 6px 12px rgba(36, 60, 43, 0.10), 0 0 0 1px rgba(20, 43, 49, 0.05);
  --ps-shadow-app: 0 18px 45px rgba(33, 59, 40, 0.08);
  --ps-shadow-popover: 0 8px 24px rgba(32, 43, 49, 0.14), 0 1px 3px rgba(32, 43, 49, 0.06);
  --ps-shadow-dialog: 0 24px 64px rgba(32, 43, 49, 0.20);

  --ps-dur-0: 0ms; --ps-dur-fast: 90ms; --ps-dur-base: 110ms; --ps-dur-panel: 160ms;
  --ps-dur-progress: 240ms; --ps-dur-loop: 1200ms;
  --ps-dur-launcher-in: 150ms; --ps-dur-launcher-out: 110ms;
  --ps-ease-standard: cubic-bezier(0.2, 0, 0, 1);
  --ps-ease-out: cubic-bezier(0.23, 1, 0.32, 1);
  --ps-ease-in-out: cubic-bezier(0.77, 0, 0.175, 1);
  --ps-ease-linear: linear;
  --ps-ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
  --ps-ease-launcher-out: cubic-bezier(0.4, 0, 0.2, 1);

  --ps-row-launcher: 58px; --ps-row-list: 64px; --ps-row-nav: 40px;
  --ps-control-height: 32px; --ps-field-height: 38px;
  --ps-content-max: 1040px;

  /* Launcher filter strip */
  --ps-strip-height: 40px; --ps-chip-height: 26px; --ps-chip-gap: 6px;
  --ps-chip-pad-x: 10px; --ps-chip-radius: 6px; --ps-strip-pad-x: 16px;
}

:root[data-theme="dark"] {
  color-scheme: dark;
  --ps-ground: #141513; --ps-surface: #1c1e1a; --ps-surface-raised: #242722;
  --ps-surface-soft: #202320; --ps-sidebar: #191b18;
  --ps-line: #2e322c; --ps-line-soft: #262923; --ps-line-strong: #3d423a; --ps-control-border: #666e63;
  --ps-ink: #ecefe8; --ps-ink-2: #b4bab0; --ps-ink-3: #959b91;
  --ps-accent: #8fb6cc; --ps-accent-strong: #a8c9db; --ps-accent-soft: #24333b;
  --ps-accent-border: #4f7183; --ps-selected-bar: #8fb6cc; --ps-focus: #7fb0c9;
  --ps-nav-active-bg: #2b3029; --ps-nav-active-ink: #f2f5ef;
  /* active nav item also carries a 2px inset var(--ps-selected-bar) bar */
  --ps-avatar-bg: #223038; --ps-avatar-ink: #9ec5de;
  --ps-primary-bg: #2f637e; --ps-primary-ink: #f7f8f4; --ps-primary-hover: #3a708c;
  --ps-positive: #6fbf93; --ps-warning: #d9b171; --ps-negative: #e08a8a;
  --ps-positive-pill-bg: #172a1d; --ps-positive-pill-ink: #7fc99c;
  --ps-negative-pill-bg: #2c1a1c; --ps-negative-pill-ink: #e59a9a;
  --ps-warning-pill-bg: #2a2317; --ps-warning-pill-ink: #d9b171; --ps-warning-pill-border: #4a3c20;
  --ps-kbd-bg: #262a24; --ps-kbd-border: #3a4038; --ps-kbd-ink: #b0b7ad;
  --ps-progress-track: #2b3029; --ps-progress-fill: #8fb6cc; --ps-scrollbar-thumb: #3f463d;
  --ps-scrim: rgba(0, 0, 0, 0.5);
  --ps-badge-word-bg: #1c2a33; --ps-badge-word-ink: #9ec5de;
  --ps-badge-folder-bg: #2c2719; --ps-badge-folder-ink: #d8b878;
  --ps-badge-code-bg: #1c2a24; --ps-badge-code-ink: #a6cdb5;
  --ps-badge-action-bg: #212a30; --ps-badge-action-ink: #b6c9d6;
  --ps-badge-image-bg: #2a2430; --ps-badge-image-ink: #c4b6d8;
  --ps-badge-neutral-bg: #23291f; --ps-badge-neutral-ink: #b9c4b3;
  --ps-shadow-launcher: 0 1px 3px rgba(0, 0, 0, 0.50), 0 3px 8px rgba(0, 0, 0, 0.40),
    0 6px 12px rgba(0, 0, 0, 0.45), 0 0 0 1px rgba(236, 239, 232, 0.08);
  --ps-shadow-app: 0 18px 45px rgba(0, 0, 0, 0.45);
  --ps-shadow-popover: 0 8px 24px rgba(0, 0, 0, 0.45), 0 1px 3px rgba(0, 0, 0, 0.35);
  --ps-shadow-dialog: 0 24px 64px rgba(0, 0, 0, 0.55);
}

:root[data-density="compact"] {
  --ps-row-launcher: 48px; --ps-row-list: 52px; --ps-row-nav: 34px;
  --ps-control-height: 28px; --ps-field-height: 32px;
  --ps-strip-height: 36px; --ps-chip-height: 24px; --ps-chip-gap: 5px;
  --ps-chip-pad-x: 8px; --ps-strip-pad-x: 12px;
}

html[data-window="main"] {
  --ps-text-display: clamp(28px, 3.6vw, 44px);
  --ps-text-h1-setup: 34px;
  --ps-text-stat: 36px;
  --ps-text-lead: 15px;
  --ps-text-body: 14px;
  --ps-text-strong: 14px;
  --ps-text-small: 13px;
  --ps-text-meta: 12px;
  --ps-text-micro: 11px;
  --ps-text-section: 12px;
  --ps-text-kbd: 11px;
  --ps-text-control: 13px;
  --ps-leading-display: 1.08;
  --ps-leading-setup: 1.16;
  --ps-leading-lead: 24px;
  --ps-leading-body: 21px;
  --ps-leading-small: 20px;
  --ps-leading-meta: 18px;
  --ps-leading-micro: 16px;
  --ps-content-max: 1120px;
  --ps-app-margin: 40px;
  --ps-app-gutter: 24px;
  --ps-app-card-pad: 24px;
  --ps-app-section-gap: 32px;
  --ps-app-label-col: 280px;
  --ps-app-sidebar: 232px;
  --ps-row-nav: 44px;
  --ps-row-list: 68px;
  --ps-control-height: 36px;
  --ps-field-height: 40px;
  --ps-app-metric-row: 44px;
}
@media (max-width: 1099px) { html[data-window="main"] { --ps-app-margin: 32px; --ps-app-sidebar: 200px; } }
@media (max-width: 819px)  { html[data-window="main"] { --ps-app-margin: 24px; --ps-app-gutter: 16px; --ps-app-sidebar: 64px; } }
@media (max-width: 700px)  { html[data-window="main"] { --ps-app-margin: 20px; } }
html[data-window="main"][data-density="compact"] {
  --ps-row-nav: 38px; --ps-row-list: 60px; --ps-control-height: 32px; --ps-field-height: 36px;
}
```

---

## 12. Cross-cutting implementation rules

### 12.1 Structure

- Token file: `apps/desktop/src/styles/tokens.css` (L0). Theme + density attributes on `<html>`.
- Styles: CSS Modules or plain CSS per component under the owning feature folder; **no utility-class soup**, no
  inline hex. Shared primitives live in `apps/desktop/src/components/` (Button, Field, Choice, ProgressBar,
  StatusDot, Kbd, EmptyState, InlineNotice, SectionHeader).
- Icons: `apps/desktop/src/components/icons/` generated from Lucide; no runtime icon font.
- Test hooks: `data-ps-theme`, `data-ps-density`, and stable `data-testid` values: `launcher-input`,
  `launcher-count`, `result-group`, `result-row`, `result-icon`, `preview-panel`, `launcher-footer`,
  `footer-hint`, `status-dot`, `nav-item`, `index-progress`, `onboarding-step`, `empty-state`, `error-state`.
  `ux-qa` uses these for screenshot appurtenance; visual QA is read-only on UI.

### 12.2 Rendering rules that protect the feel

- No white flash: launcher window background is set before first paint (Tauri window background colour +
  `background: var(--ps-surface)` on `html`), content mounted in the same frame as show.
- No layout shift when results arrive: rows have fixed heights; the window resizes only by the defined steps
  (SCREENS §2.3).
- No scroll hijacking; no custom smooth-scroll; keyboard selection uses `scrollIntoView({block:'nearest'})`.
- Never disable text selection in the app; in the launcher only the results list is non-selectable (input is).
- Scrollbars: thin, `scrollbar-width: thin`, `scrollbar-color: var(--ps-scrollbar-thumb) transparent` in the
  launcher; app panes follow the OS and may hide overlays.

### 12.3 Localization and copy

- All strings externalised; no concatenated sentences; plural forms via `Intl.PluralRules`.
- Dates/times/numbers via `Intl`; greeting via `Intl.DateTimeFormat` hour (see SCREENS §3.1).
- Brand name and code tokens get `translate="no"`.
- Active voice, second person, specific verbs ("Open folder", "Copy path", "Pause indexing"), no exclamation
  marks, no emoji, no marketing adjectives.

### 12.4 High contrast / forced colours

```css
@media (forced-colors: active) {
  * { forced-color-adjust: auto; }
  .ps-panel, .ps-row, .ps-field, .ps-button { border-color: CanvasText; box-shadow: none; background: Canvas; }
  .ps-row[aria-selected="true"] { background: Highlight; color: HighlightText; forced-color-adjust: none; }
  .ps-focusable:focus-visible { outline: 2px solid Highlight; }
  .ps-brand-mark { color: CanvasText; }
}
```
Rules: system colours only; the selected row becomes the system Highlight (the 2 px bar is not needed there); status
dots keep their text; shadows disappear; the brand mark uses the monochrome variant via `currentColor`; never rely
on hue alone. Test with Windows High Contrast themes (Black, White, Aquatic, Desert).

### 12.5 Quality gates for UI work (definition of done for components)

1. Tokens only; grep for `#[0-9a-fA-F]{3,8}` outside tokens/brand assets returns nothing in UI code.
2. All states implemented (rest/hover/active/focus-visible/selected/disabled/loading/error) and reachable by
   keyboard.
3. Contrast ratios match §3 tables; re-run `.agents/notes/contrast_final.py` after any palette change.
4. Screenshots at 560/820/1000/1280 × light/dark, plus 125/150/200 % DPI, combined with
   `docs/design/KEYBOARD_AND_A11Y.md` §9 checklist.
5. Reduced motion and forced colours verified.
6. No banned pattern from §2.1 appears in the diff.

---

## 13. Known risks carried by this system

1. **10 px tracked caps at 100 % DPI.** Legible in Inter 750, but the smallest text in the product; if QA finds it
   fails, bump group labels/eyebrows to 11 px first (keeps the hierarchy), not the whole scale.
2. **Inter bundling.** If Inter is not shipped, tracking/weights fall back to Segoe UI Variable and the reference's
   tight display tracking will look too tight; L0 decision needed before UI lock.
3. **Selected-row state cue.** The reference's fill-only selection fails WCAG 1.4.11; the added 2 px bar is the
   correction. Reviewers must not remove it without replacing it with another ≥3:1 cue.
4. **Dark palette is new art direction.** It has no reference pixels; ratios are verified, but taste must be checked
   on real screenshots at 125–200 % DPI before sign-off.
5. **Density without type change** may feel too similar between modes at first; it is a spacing-only control by
   design. If users cannot tell, differentiate row pitch further (58/46) rather than shrinking type.
