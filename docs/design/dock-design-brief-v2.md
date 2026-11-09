# ProtonDock design brief v2 (2026-11-09)

Status: direction for the visual revision of `dock-editable.html`. Subject to user approval;
once approved, this file plus the revised HTML become the design authority and get pinned.

## 1. Direction

**A quiet dock whose hierarchy comes from content.** The dock is a control surface, not a
decorated row. Every tile earns its place with one real datum; every popover holds the detail;
nothing glows, nothing is tinted by category, nothing pretends to be live.

Reference points: docktor for the interaction model (window previews, glanceable widgets,
restraint), the Windows 11 design language for materials and density, and this repo's
anti-slop rules for the bans. Do not imitate Liquid Glass; use neutral dark glass and real
content.

## 2. Hard bans (from CLAUDE.md / UI_SKILLS.md)

1. No category tints on widget surfaces; no decorative gradients; no gradient text.
2. No emoji as icons or copy.
3. No shadow on every surface; at most two elevation levels in the whole file.
4. No generic AI blue/purple palette or glow; one accent, used sparingly.
5. No uniform same-size card grid without hierarchy; size follows content.
6. No fabricated "live" values; demo values must be labelled as preview data.
7. No unmotivated motion; reduced motion disables all of it.
8. No mixed icon systems; one 24px stroke set.

## 3. Tokens (contract for CSS)

Colour roles (dark first; light and glass map to the same roles):

- `--bg-0: #0a0b0d` (desktop), `--bg-1: #101216` (panels)
- `--surface-1: rgba(20,22,27,.78)` (rail), `--surface-2: rgba(26,28,34,.92)` (popover)
- `--border-subtle: rgba(255,255,255,.08)`, `--border-strong: rgba(255,255,255,.16)`
- `--text-1: #f2f3f5`, `--text-2: #b4bac4`, `--text-3: #7f8692`
- `--accent: #8cc8ff` (focus/active only; never a fill for whole tiles)
- `--ok: #3fb950`, `--warn: #d29922`, `--bad: #f85149` (status only, never decoration)
- `--brand-win: #0078d4` (Windows flag only)

Spacing: 2, 4, 6, 8, 12, 16, 24, 32. Radii: 8 (chip), 12 (tile), 16 (card), 24 (dock),
999 (pill). Type: system stack ("Segoe UI Variable", "Segoe UI", system-ui); sizes 10, 11, 12,
13, 15, 17; weights 400/500/600; tabular numerals for all metrics.

Elevation: `--elev-1: 0 6px 18px rgba(0,0,0,.28)` (popover), `--elev-2: 0 16px 44px rgba(0,0,0,.42)`
(modal/toast). Tiles have **no** shadow.

Motion: `--dur-1: 120ms`, `--dur-2: 180ms`, `--dur-3: 240ms`, `--dur-4: 320ms`;
`--ease-out: cubic-bezier(.22,.61,.36,1)`, `--ease-in-out: cubic-bezier(.65,0,.35,1)`.
One authored entrance (dock cluster, 320ms); popovers 180ms; hover/press 120ms; profile switch
240ms. `@media (prefers-reduced-motion: reduce)` sets all durations to 1ms and disables
transforms/loops.

Layout: tiles 44–48px high, gap 8, rail padding 8, radius 24; shelf is a separate row of
pills; popovers max-width 360, anchored with a 6px caret. Breakpoints: 1024, 1280, 1440, 1920.

## 4. Component anatomy

- **Dock rail**: neutral glass, one border, no per-tile background difference except hover.
  Tiles: icon (18–20px) for compact; icon + one value for standard; icon + label + value for
  wide. Widths: 48, 96, 140, 168, 200 px (per widget allowed set; keep current defaults).
- **Shelf**: profile pills (active = white .09 fill, no blue), theme segmented control, style
  segmented control, edit toggle, Customize pill. Text 11–12px.
- **Tile states**: rest, hover (surface .06 white + lift 1px, 120ms), press (scale .985),
  active (accent ring 1px), disabled (opacity .5), off (icon/text 3), focus-visible (2px accent
  outline offset 2), drag (elev-1, scale 1.02).
- **Popover**: header (icon + title + status), body (content), footer (actions). One elevation,
  no tint, no glow. Max one highlighted number.
- **Widget content**: one primary datum per tile; popover detail; honest empty states
  ("No session", "Not configured", "Unavailable"); sample values only with a "Preview" badge.
- **Search launcher**: neutral card, one accent on selection, 12px radius, no glass stacking.

## 5. Icon contract

One set: 24×24 viewBox, stroke 1.6, round caps/joins, 18px default render, 20px for app-like
glyphs; filled only for the Windows flag and status dots. Required icons: windows-start (solid
brand flag), search, running-apps, pinned-apps, system-stats, calendar, peripherals,
status-battery, claude, codex, assistant (neutral robot outline, no orb), wifi, bluetooth,
notifications, system-tray, music-player, quick-notes, clock, plus UI icons (close, plus,
chevron, ellipsis, check, trash, folder, file, play, pause, next, prev, volume, sun, moon, gear,
arrow-right, external, grip). No emoji. Every icon-only button gets aria-label.

## 6. Content and behaviour rules

- Labels are sentence case, no exclamation marks, no marketing copy; one line per control.
- No fake "live/connected" claims. Simulated telemetry is allowed only in the design preview
  and must be marked with a `data-demo` state and a visible "Preview data" chip in the
  customizer, not on the dock.
- Inline `onclick` handlers are removed (behaviour lives in the script).
- Every interactive element is keyboard reachable with a visible focus ring; Escape closes
  popovers, customizer and search.
- `prefers-reduced-motion` is respected; no infinite animation except the explicitly reduced
  case.
- Browser surfaces themed: `::selection`, scrollbars, caret, focus.

## 7. Acceptance checklist (revision is done only if all pass)

- [ ] 0 emoji; 0 category tints; 0 gradient text; 0 decorative gradients.
- [ ] ≤ 2 elevation tokens used, tiles have none.
- [ ] ≤ 14 unique hex colours (excluding status and brand).
- [ ] At least 4 responsive breakpoints, no horizontal overflow at 1024/1280/1440/1920.
- [ ] `prefers-reduced-motion` block present and complete.
- [ ] Every interactive element has focus-visible styling and an accessible name.
- [ ] All icons from the single 24px set; no mixed stroke widths.
- [ ] All demo values flagged; no unlabelled "live" telemetry.
- [ ] Popovers/tiles use the token roles only; no literal colours outside tokens.
- [ ] Motion uses duration/easing tokens; no scattered ad-hoc transitions.
