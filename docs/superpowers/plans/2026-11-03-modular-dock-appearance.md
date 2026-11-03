# Modular Dock Appearance Implementation Plan

> **For agentic workers:** Use subagents with separate UI and Rust ownership; integrate and review each checkpoint.
>
> **Goal:** Apply the approved seven-widget concept to the existing native dock without replacing the search engine or destroying saved layouts.
>
> **Architecture:** Keep one transparent Slint dock window and the existing controller/AppBar. Paint a separate housing per source widget; group application rows by their source widget ID. Persist one appearance choice and reuse existing providers and callbacks.
>
> **Tech Stack:** Existing Rust, Slint 1.17, Win32, serde. No new dependencies.
>
> **Spec:** The user's approved separate-widget reference, output/imagegen/dock-concepts-2026-11-03/02-floating-islands.png, and the final themes.png in the chat's polished-modular-dock visualization folder. This plan supersedes the continuous-shell visual direction, not the existing functional contracts.

## Global constraints

- Seven reference widget types: Applications, Weather, Status, Calendar, Time, Proton Search, System Tray.
- Keep optional existing widgets, pins, enabled state, profiles and order. Existing saved lists are not silently replaced.
- Use existing native app artwork. No generated app icons, invented forecast, event data or fake tray icons.
- All three appearances share geometry. Light is pearl; Dark is charcoal; LiquidGlass is restrained translucent material.
- Current renderer does not establish true per-widget desktop blur/refraction; report that limitation.
- No source edits or builds with ProtonSearch running. No automatic launcher restart after the user's explicit stop.
- Commit/push coherent checkpoints as required by .agents/AGENTS.md. Refresh the knowledge graph without claiming visual acceptance from compilation.

## Review focus

- Width calculation and popup anchors must agree with independent housing gaps, including horizontal scrolling and monitor/DPI changes.
- Adjacent applications in one Applications widget share a housing but retain individual focus, activation and context actions.
- Old configuration JSON lacking appearance and new widget kinds must deserialize without changing pins or lists.
- Light mode system glyphs, secondary text, keyboard focus and popovers must remain visible.
- Missing weather and third-party tray hosting remain explicit limitations, never simulated completed integrations.

## Step 1 — Critique and map

- [x] Inspect actual screenshot and approved three-theme reference.
- [x] Query graphify-out/graph.json before targeted source inspection.
- [x] Obtain an independent Impeccable native visual critique.
- [x] Map UI, row-building, geometry, persistence, panels, tests and exports.
- [x] Record scope and checklist here.

The critique found two priorities: independent widget boundaries; consistent compact geometry and restrained material. Visible-only heuristic subset: 9/20, not a functional usability score. Browser/DOM detectors do not apply to native Slint.

## Step 2 — Separate surfaces and preserve geometry

**Files:** protonsearch/ui/dock.slint; protonsearch/src/dock/mod.rs.

- [x] Add transient DockItem group_id, surface_width and gap_before fields.
- [x] Group expanded app rows by their source Applications instance, preserving individual buttons.
- [x] Replace shared painted shell with independent housings; keep transparent gaps.
- [x] Share the contract: window edges 6 logical px, housing y 7 px, gap 10 px, settings housing 44 px.
- [x] Update content width and popup offsets using the same group geometry.
- [x] Test empty, single, app-group and boundary widths plus scroll-aware popup anchors.

## Step 3 — Apply appearances and complete first-party presentation

**Files:** protonsearch/src/dock/model.rs; protonsearch/src/dock/mod.rs; protonsearch/ui/dock.slint; protonsearch/ui/app.slint.

- [x] Add serde-defaulted DockAppearance (Light/Dark/LiquidGlass), retaining old JSON compatibility.
- [x] Export and bind DockStyle and SystemStatus to dock and panel instances.
- [x] Theme text, system glyphs, focus/hover treatments, controls and popovers consistently.
- [x] Add appearance controls to existing Customize panel and persist changes.
- [x] Add independent Status and Calendar widget kinds using existing live system providers and local date.
- [x] Use the seven-widget arrangement for new layouts only; preserve existing saved profiles.
- [x] Reuse volume/mute and system settings actions in the Status panel.
- [x] Test appearance roundtrip, old-config loading, defaults and additive widget support.

## Step 4 — Build, independent review and native acceptance

- [x] Run cargo test --locked --manifest-path protonsearch/Cargo.toml --lib. Result: 12 passed.
- [x] Run targeted dock geometry tests in the ProtonSearch binary. Result: 7 passed.
- [x] Build ProtonSearch with its existing locked dependencies. Cargo check and scoped formatting also passed.
- [x] Obtain independent code review; resolve actionable findings. Both findings fixed and re-review approved.
- [ ] Complete Rust-specialist approval: attempted, but its global gate stopped at pre-existing unrelated formatting. No blanket Rust approval claimed.
- [x] Update graph and this checklist; commit and push only task-owned tracked files.
- [ ] Inspect native Light, Dark and Glass layouts and anchored Status/Calendar panels after the user opens the new build.
- [ ] Verify customization, overflow and target DPI on the real desktop.

Native visual acceptance stays open until inspected. The app remains stopped after builds unless the user asks to launch it again. No calendar-account integration, live-weather provider or third-party tray hosting is added by this appearance change.

