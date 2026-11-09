# Design audit: dock-editable.html (2026-11-09)

Scope: `dock-editable.html` (6,598 lines: 2,945 CSS, 757 markup, 2,885 JS) audited against
`CLAUDE.md`, `AGENTS.md` and the vendored anti-slop rules (`.agents/skills/UI_SKILLS.md`,
impeccable `craft-floor.md`/`component-review.md`, taste-skill `cards.md`/`icons.md`/`system.md`).

Verdict: **FAIL** the repo's own anti-slop bar. The file is a feature-rich prototype, but its
design layer is decoration-led, not content-led, and it has no accessibility or responsive
foundation. A stylesheet/icon/content/motion revision is required, not a polish pass.

## 1. Measured violations (whole file)

| Signal | Count | Rule / why it fails |
|---|---|---|
| `@media` queries | **0** | No responsive behaviour at all |
| `prefers-reduced-motion` | **0** | CLAUDE.md: reduced motion is the only motion off-switch |
| `box-shadow` declarations | **28** | "no shadow on every surface" |
| gradients (linear/radial) | **21** | "no decorative gradients" |
| unique hex colours in CSS | **49** (155 literals) | palette sprawl; roles are not used |
| `rgba()` literals | **300** | no colour roles/tokens |
| category-tint selectors | per-tile `--tint` gradients | "no category tints on widget surfaces" |
| `!important` | **12** | specificity fights instead of a system |
| `::selection` styling | **0** | browser surface not themed (craft-floor) |
| `:focus-visible` rules | **3** for 46 buttons | keyboard focus incomplete |
| inline `style="…"` in markup | **24** | state cannot be themed or reduced-motioned |
| inline `onclick` in markup | **8** | behaviour in markup, not in the script |
| `<svg>` in markup | **37** | no icon system; mixed fill/stroke, mixed viewBoxes |
| `iconSvg:` strings in JS | **10** + 51 inline SVG strings | icon set is scattered and inconsistent |
| keyframes | **4** (`siri-spin`, `siri-pulse`, `eq-bounce`, `ringSpin`) | motion is scattered, not authored |
| emoji | **2** (💧, 💨 in the weather row) | "no emoji" is a ban |
| buttons without `type` | 0 | OK |
| `aria-` attributes | 36 | partial; many icon-only controls unlabeled |
| `tabindex` | 0 | arrow-key/roving focus absent |

Composition findings (the "AI slop" read):

1. **Every tile is a category-tinted glass gradient** — the dock is a uniform row of
   same-size rounded rectangles whose only differentiation is hue. That is the textbook
   uniform-card-grid failure: no hierarchy, colour used decoratively.
2. **Decoration carries the interest** — 22 `backdrop-filter` layers, 21 gradients, 28 shadows.
   The design has no content-driven hierarchy (no window previews, no artwork-led music tile,
   no event-led calendar tile).
3. **Telemetry fake-posed as live** — weather is hardcoded to "24° San Francisco"; system stats
   are fixed numbers; battery has a "simulate" toggle; Wi-Fi/Bluetooth/notifications are
   pre-baked. In a design spec this is acceptable only if labelled; the dock presents it as
   live ("88% Charging", "CPU 18%").
4. **Icon chaos** — 37 inline SVGs + 51 SVG strings + 2 emoji, mixed stroke widths, mixed
   optical sizes, Windows flag drawn as four equal squares, Airbnb-style "assistant" orb.
5. **Motion is scattered** — only 4 keyframes, 45 transitions with no duration/easing tokens,
   and no reduced-motion path. Hover/shadow motion runs on surfaces that should be static.
6. **Theme duplication** — the same values are re-declared per theme with literal colours
   instead of roles; `dark`, `light` and `liquid-glass` are three parallel palettes.
7. **Dead/duct-taped CSS** — 19 classes defined but never used, 26 classes used but never
   styled; the stylesheet is 2,945 lines with no section structure.

## 2. Rule citations used in the revision

- `CLAUDE.md`: "No generic AI look: no random gradients, purple/blue glow, emoji, oversized
  icon tiles, uniform card grids without hierarchy, or shadows on every surface."
- `UI_SKILLS.md`: widget surfaces black/white/translucent glass only; no category tints;
  no decorative gradients; no emoji; no shadow on every surface; motion is part of the design;
  cite the rule that changed a design.
- impeccable `craft-floor.md`: shadows carry offset+soft blur (no zero-offset halo); theme the
  browser surfaces (selection, caret, scrollbars, focus rings); one authored moment; real
  states (hover, disabled, loading, error, empty); no gradient text; no emoji as icons;
  no uniform same-size card scaffolds.
- taste-skill `icons.md`: one stroke system, consistent 24px grid, optical sizing, no brand
  marks redrawn as generic glyphs.

## 3. Brief audit (what the current direction is trying to be)

The file's implied direction is "Tahoe/Liquid-Glass dock with category-tinted telemetry
tiles". That direction is why it reads as AI slop: it imitates materials instead of using
content; it uses colour as decoration; it has no hierarchy; and it presents sample data as
live. The revision (brief v2) keeps the dock concept and the widget inventory but changes the
direction to: **neutral materials, content-led hierarchy, one accent, honest states, authored
motion, and a real icon system**.

## 4. What survives from the current file

- The information architecture: 19 widgets, 4 profiles, dock + shelf + popovers + customizer +
  search launcher + toast; defaults in `DEFAULT_PROFILES`.
- The behaviour model: `renderDock`, `openPopoverForWidget`, drag reorder, width cycling,
  profile storage, theme storage, edit mode, search launcher, calculator.
- The screenshot/QA evidence value of the previous passes; no claim is made that it looks good.

Everything visual is in scope for revision: CSS (full rewrite), icon system, widget content,
popover anatomy, motion tokens, responsive rules, reduced motion, focus and state styling.
