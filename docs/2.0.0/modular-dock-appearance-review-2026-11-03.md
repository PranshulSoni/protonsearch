# Modular dock appearance: critique and implementation checkpoint

Date: 2026-11-03. Mode: Operate. Method: independent Impeccable visual assessment plus independent implementation/code review. The surface is native Slint, not a browser page.

## Baseline critique

The supplied screenshot shows one continuous dark enclosure with distributed controls. The approved reference requires distinct source-widget housings with visible gaps, consistent geometry and three selectable appearances.

| Visible heuristic | Baseline score | Evidence |
| --- | --- | --- |
| System status | 2/4 | Time, volume and running dots; forecast unavailable and combined status absent |
| Real-world match | 3/4 | Familiar application and system pictograms |
| Consistency | 1/4 | Required independent widget composition absent |
| Recognition | 2/4 | Apps recognizable; several actions unlabeled |
| Minimalism | 1/4 | Broad shared enclosure and uneven unused space |
| Visible subset | 9/20 | Not a complete usability score |

User control, efficiency, error prevention/recovery and help are unscored from a screenshot. No post-implementation visual score is claimed before the new executable is inspected.

Two priorities were adopted:
1. Independent, content-sized widget surfaces; Applications stays one source group with individual app actions.
2. Stable geometry, readable typography/glyphs and restrained materials across Light, Dark and LiquidGlass.

Strengths retained: real installed-app artwork, running indicators, original controller/provider actions, keyboard and accessibility contracts. First-time users need recognizable boundaries and a labeled Search action. Low-vision users need real wallpaper/contrast checks; screenshots alone cannot certify accessibility.

Questions skipped: the user already selected the modular concept, requested implementation and explicitly authorized subagents.

## Implemented source changes

- Source-widget surface metadata groups app rows without losing their separate targets.
- Transparent 10-logical-pixel housing gaps, matching width calculations and scroll-aware popup anchors.
- Additive persisted appearance selection and themed system glyphs, controls, popovers, focus and hover states.
- Additive Status and Calendar kinds reuse real system snapshots and local date. Legacy kinds remain available.
- New profiles use the seven-widget arrangement; existing saved lists, enabled flags and pins are preserved.
- New Search widgets have room for a label. Compact legacy Clock and Search widths remain supported.
- Context menus anchor to their originating items.
- Missing application artwork receives a cached theme-appropriate fallback; real app artwork is unchanged.

## Verification checkpoint

- Model tests: 12 passed.
- Binary dock geometry/material tests: 7 passed; unrelated binary tests were not rerun.
- Locked debug build of ProtonSearch: passed.
- Cargo check: passed.
- Changed Rust files: scoped rustfmt check passed.
- Source diff whitespace check: passed.
- Independent code reviewer: approved after compact-clock and fallback-icon findings were fixed.
- Rust specialist review was attempted but its global gate stopped at pre-existing formatting in indexer.rs, main.rs and search.rs. It issued no Rust-specialist approval. The independent code reviewer inspected the changed Rust/UI paths.
- Existing warnings were left outside scope. Clippy was not run.

Browser detector, DOM overlay and browser visibility checks are inapplicable to native Slint. No server or overlay was started. No native visual acceptance is claimed; the app remains stopped as requested.

## Remaining acceptance and limits

The user needs to open the new build to inspect all three appearances, widget boundaries, popup placement, configuration, overflow and DPI. Existing profiles must add Status and Calendar through Customize if desired; they are not silently replaced.

LiquidGlass uses translucency, not true desktop blur/refraction. Weather still lacks a live forecast provider. System Tray remains a Windows settings entry rather than hosted background-app icons. Calendar shows local date, not connected-account events. These provider integrations are outside this appearance checkpoint.

The previous executable and layout are backed up under scratch/dock-appearance-before-2026-11-03. No source changes to the search engine, AppBar lifecycle or taskbar recovery were required.

