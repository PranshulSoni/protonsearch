# ProtonSearch v2 — final implementation handoff

## Start here

1. Supply `PROTONSEARCH_V2_FINAL_MASTER_IMPLEMENTATION_PROMPT.md` to your coding/orchestrator agent as the binding prompt.
2. Attach `protonsearch_dynamic_island_cards_v9_polished.html` as the **authoritative Dynamic Island UI reference**. Do not replace V9 with any earlier version or redesign it. This is a UI-only simulation.
3. Attach `protonsearch_island_settings_placement_concept.html` as the interactive placement / card-order settings reference. This is a UI-only simulation, not live Windows integration.
4. Attach `original_p_logo_reference.png` if exact original mark matching is useful.
5. Ask the agent to clone `https://github.com/PranshulSoni/protonsearch`, establish the real hierarchy, audit and reproduce both critical bugs (no file/text preview; clipboard images missing), then implement through all gates.

## Reference hierarchy

- **P0 product/feature/master implementation contract:** `PROTONSEARCH_V2_FINAL_MASTER_IMPLEMENTATION_PROMPT.md`
- **Dynamic Island geometry/motion/controls:** `protonsearch_dynamic_island_cards_v9_polished.html`
- **Original V9 audit / verified prototype behavior:** `PROTONSEARCH_ISLAND_V9_UI_AUDIT.md`
- **Full desktop app's placement editor/card manager visualization:** `protonsearch_island_settings_placement_concept.html`
- **Brand:** `original_p_logo_reference.png` and SVG mark in V9.

## Screenshots

- `settings_placement_preview.png`: monitor placement UI, anchor presets, offsets, taskbar constraint.
- `settings_cards_1240.png`: ordering/enabling the eight island cards and live dot preview.

## Critical distinctions

The HTML prototypes simulate all OS actions. A successful HTML interaction test is not evidence of native Windows functionality. The requested end product must connect the UI to Rust and supported Windows APIs, use an optional Python agent sidecar, run real Windows tests, and provide acceptance evidence for every feature and relevant pairwise interaction. Do not claim animation timings can be guaranteed identically across all hardware; use the V9 visual reference and measured real-world frame pacing.
