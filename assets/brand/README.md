# ProtonSearch brand mark — reconstructed vector

**Owner:** L1-B Product Design (`design-spec`). **Status:** proposed brand asset for G3–G4; L0 accepts it into the
shipped tree. Version 1.0 (2026-11-09).

## 1. Provenance (read this first)

The master prompt references three files that are **not present in this workspace** (master prompt:11;
`docs/implementation/WORK_BREAKDOWN.md:104`). Verified on this machine:

    PS> Test-Path assets\protonsearch-mark.svg              -> False
    PS> Test-Path assets\protonsearch-logo-concept.png     -> False
    PS> Test-Path assets\original-logo-crop-reference.png  -> False
    PS> Get-ChildItem assets\brand                         -> (did not exist; created here)

The only authoritative visual source available is the **inline SVG in the supplied reference HTML**
(`protonsearch-v2-ui-reference.html:118`, a 64×64 `viewBox` with three paths: dark-navy body `#102448`, blue upper
facet `#1d5189`, off-white `#fafbf9` counterform). These files are therefore **reconstructions**, not originals:

- the reference path geometry is preserved (same silhouette, italic geometric P, the diagonal two-tone fold, the
  counterform shape), so the mark is recognisably the supplied mark;
- markup is cleaned and hardened: `role="img"`, `<title>`/`<desc>`, `shape-rendering="geometricPrecision"`, no
  metadata bloat, no external references, no gradients/glow/glass (master prompt:194, §V.D);
- the opaque off-white counterform is converted into a **true cut-out** (mask) so the counterform takes the surface
  colour (on approved light surfaces this equals the reference's `#fafbf9` within 1 RGB step) — this prevents a
  light "patch" when the mark is used on a dark or shaded surface;
- a hand-tuned **16 px optical variant** and a **dark-surface variant** were added, plus a monochrome currentColor
  variant for high contrast/print/tray;
- verification: rendered in headless Chrome at 16/29/48/128 px on light and dark surfaces and inspected at the pixel
  level for 16 px (artefacts in `.agents/notes/ref-captures/mark16-compare.png`, `brand-test.png`,
  `mark-study.png`; not shipped).

If the user later supplies the original `logo-concept.png` / crop, re-derive the vectors from it and treat this
reconstruction as a placeholder of equal intent — do not claim they are the original files.

## 2. Files

| File | viewBox | Intended use |
|---|---|---|
| `protonsearch-mark.svg` | `0 0 64 64` | Two-tone mark on light surfaces, ≥24 px. Body `#102448`, facet `#1d5189`, counterform cut out. |
| `protonsearch-mark-16.svg` | `0 0 16 16` | Two-tone mark **<24 px** (16–23 px), e.g. favicon/app-list 16 px. Same geometry thickened by a 0.8/64 stroke and with the counterform opened 16 % so it survives antialiasing. |
| `protonsearch-mark-on-dark.svg` | `0 0 64 64` | Two-tone mark on dark surfaces (dark `--ps-surface`/`--ps-sidebar`), ≥24 px. Body `#e3e9ee` (off-white), facet `#5b93c4` (lifted brand blue), counterform cut out. The navy body is invisible on dark (~1.0:1); this variant is required. |
| `protonsearch-mark-mono.svg` | `0 0 64 64` | Single-colour silhouette with cut-out counterform; fills with `currentColor`. Use in Windows high-contrast/forced-colours mode, one-colour print, and as the tray/notification base. |

Colours are **flattened** in the SVGs (no CSS variables inside the files) so they render identically in
`<img>`, Tauri window icons, and HTML. The monochrome file is the only one that inherits colour.

Measured contrast (WCAG 2.1 formula, script `.agents/notes/brand-contrast.py`):

| Pair | Ratio |
|---|---|
| body `#102448` on `--ps-surface #fbfcfa` | 14.92:1 |
| facet `#1d5189` on `#fbfcfa` | 7.88:1 |
| body on `--ps-ground #e8ebe8` | 12.78:1 |
| facet on `#e8ebe8` | 6.75:1 |
| on-dark body `#e3e9ee` on `#1c1e1a` | 13.73:1 |
| on-dark facet `#5b93c4` on `#1c1e1a` | 5.13:1 |
| mono `currentColor` light/dark | set by the surface text token (14.06:1 / 14.46:1 in the design system) |

## 3. Usage rules

### Do

- Use the **29 px** mark at the start of the launcher search row and in the app sidebar beside the wordmark
  (reference:25, 30, 54). Use 26 px at ≤700 px window width.
- Use the **16 px file** for any rendered width below 24 px; use the **on-dark file** whenever the surface is a dark
  theme token (check the resolved theme, not the OS "app mode" alone).
- Keep clearspace of **≥25 % of the mark height** on every side (e.g. 7 px around a 29 px mark; minimum 4 px at
  16 px). The mark's own SVG box has internal padding; don't crop it further.
- Put the mark on a flat approved surface: `--ps-surface`, `--ps-surface-raised`, `--ps-sidebar` (light) or their
  dark equivalents.
- Pair it with the wordmark "ProtonSearch" (12 px/750/`-0.035em`) when the product is named; the mark alone is fine
  where the name is already present or space is 16–24 px.
- Give the mark an accessible name: `role="img" aria-label="ProtonSearch"` (or `<title>` in the SVG). When it repeats
  the adjacent wordmark, treat the mark as decorative for AT (`aria-hidden="true"` on the image) to avoid double
  announcements.

### Don't

- Don't use gradients, glows, glass, drop shadows, inner shadows, outlines, strokes, bevels, or 3D effects.
- Don't rotate, skew, mirror, condense, expand, or letterspace the mark; don't re-draw the fold or move the
  counterform.
- Don't recolour the two-tone mark outside the four approved files; the brand navy `#102448` is not a UI accent and
  must not be pressed into service as one (UI accent: `--ps-accent #285975`).
- Don't place the mark on photography, illustration, or a coloured fill that is not an approved surface.
- Don't use the 64 px two-tone file below 24 px (the counterform fills in) — switch to the 16 px file.
- Don't use the on-dark file on light surfaces (it will look washed out) or the light file on dark surfaces.
- Don't ship a raster upscale of the mark; if a raster is needed, render it from the SVG at the target size
  (Windows icon sizes: 16/20/24/32/48/64/128/256).

## 4. Wordmark and naming

- Product name: **ProtonSearch** — one word, capital P and S, never "Proton Search", never "ProtonSwitch", never all
  caps in running copy. Preserve the earlier **OmniSearch** name only in migration copy ("Migrated from OmniSearch").
- Wordmark face: the UI face (Inter, weight 750, tracking `-0.035em`, 12 px in the sidebar, 12–14 px in onboarding).
  No separate logo typeface, no custom lettering for v2.
- Alt/aria labels: "ProtonSearch" (mark), "ProtonSearch home" (the app icon), "ProtonSearch quick launcher" (window
  title, not the mark).
- `translate="no"` on the brand name and wordmark in HTML.

## 5. Where each variant is used in the product

| Surface | File | Rendered size |
|---|---|---|
| Launcher search row (light) | `protonsearch-mark.svg` | 29 px (26 px ≤700) |
| Launcher search row (dark) | `protonsearch-mark-on-dark.svg` | 29 px (26 px ≤700) |
| App sidebar brand block (light/dark) | same rules as the launcher | 29 px |
| Onboarding rail brand block | same rules | 29 px |
| Favicon / small UI glyph / app-list 16 px | `protonsearch-mark-16.svg` | 16–23 px |
| Windows high contrast / print / tray base | `protonsearch-mark-mono.svg` | 16–256 px (rasterise per size) |
| App icon (`.ico`/`.png`, built by release) | derive from the two-tone files; do not invent a new mark | 16–256 px |

App-icon production notes for `release`: rasterise from the SVG at 16/20/24/32/40/48/64/128/256 px; supply light and
dark-appropriate variants only where the OS surface demands it; never bake a background colour into the icon unless
the icon is a tile (ProtonSearch's mark is not a tile — no fake rounded-square badge).
