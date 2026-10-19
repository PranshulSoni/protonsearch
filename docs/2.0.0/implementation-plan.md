# Proton 2.0 implementation plan

> For implementation agents: use the repository rules and the pinned rust-skills
> selectively. Use writing-plans/executing-plans guidance for bite-sized execution.
> This document is the plan; it does not authorize implementing every stage in one pass.

**Goal:** deliver a customizable Windows dock that preserves Proton Search and grows
into a widget/agent workspace.
**Architecture:** Rust services and Windows adapters, thin native UI candidate,
independent recovery, isolated agent execution, staged widget trust model.
**Stack:** Rust/windows-rs/SQLite FTS5; Slint candidate vs Tauri 2 comparison.
**Status:** proposed, 20 October 2026. Production implementation has not started.

## Execution rules

- Inspect rules.md, .agents/AGENTS.md and .agents/rules.md.
- Current repository requires commit/push after edits, app stopped before editing/building,
  no graphify, and no background launcher launch. User manually launches the app for
  native verification or explicitly changes that instruction.
- Use meaningful regression tests for extraction and security-sensitive behavior.
- Never run migration experiments against the user's only production database.
- Keep each stage independently reviewable. A failed gate triggers fallback/replan.
- Proposed paths below do not yet exist; adapt naming only with equivalent boundaries.
- No new runtime dependency or version bump is implied by this planning branch.

## Dependencies and owners

Maintainer owns product/release decisions; Rust/search engineer owns extraction;
Windows engineer owns AppBar/task identity/recovery; UI engineer owns rendering and
accessibility; runtime/security engineer owns grants and agents; QA/release owns matrix.
One person may hold several roles. Sequence remains the same for a solo project.

Stages 1 and 2 can proceed after Stage 0 with agreed domain contracts. Dock alpha needs
both. Do not expand native hiding or public plugin execution while core gates fail.

## Stage 0 — establish a safe baseline

### Task 0.1: behavioral and test inventory

Read: protonsearch/src/main.rs, search.rs, indexer.rs, launcher.rs, settings.rs,
ai.rs, settings_ui.rs; Cargo.toml, installer.iss, update.json.
Create later: docs/baseline/search-behavior.md and anonymized tests/fixtures.

1. Inventory prefixes, query ranking, source scopes, fast/slow coalescing and result actions.
2. Capture safe fixtures for apps/files/content/browser/Git/clipboard/notes.
3. Run existing unit suites on Windows and record actual count/failures.
4. Capture warm/cold search and open latency on named hardware; existing doc claims
   are not measurements.
5. Document any existing failures before extraction.

Planned verification:
`cargo test --manifest-path protonsearch/Cargo.toml`
`cargo fmt --manifest-path protonsearch/Cargo.toml --check`
`cargo clippy --manifest-path protonsearch/Cargo.toml --all-targets`

Exit: baseline evidence, known failures and fixture expectations are reviewed.
Do not claim these commands were run by the planning pass.

### Task 0.2: preview identity and migration safety

Read: installer.iss:31–33; settings.rs:192–223; main.rs:732–740.
Plan/create later: src/app_paths.rs, src/storage/migrations.rs,
tests/migration_import.rs, preview-specific installer configuration.

1. Define distinct preview install/AppId, startup, process/window/single-instance and
   data identity; preview data candidate is %LOCALAPPDATA%/ProtonPreview/2.
2. Audit every current database/table and setting source.
3. Implement consistent backup via SQLite API plus external artifacts.
4. Add idempotent import into preview, not in-place upgrade.
5. Change uninstall to preserve user data by default; explicit deletion is separate.
6. Test interrupted migration, duplicate legacy folders, WAL source, corrupt source,
   low disk, retry and rollback.

Exit: uninstalling preview cannot delete v1 data; failed import leaves v1 unchanged.

### Task 0.3: CI and release safety baseline

Create later: .github/workflows/windows-ci.yml; dependency/license inventory.
Add format/lint/test tasks on Windows MSVC, locked dependency resolution and redacted
logs. Test jobs do not execute privileged desktop automation. Review existing
third-party asset/runtime licenses.

Exit: baseline reproducible in CI or documented platform-dependent exceptions.

## Stage 1 — prove desktop feasibility before renderer lock

### Task 1.1: AppBar and geometry spike

Create later: experiments/appbar-spike/ with isolated test data.
Reference: sources S01/S02/S03, windows.html.

1. Register HWND; negotiate work area; respect returned rectangle.
2. Handle position/activation/fullscreen notifications and unregister on exit.
3. Show centered content inside reserved edge band; test transparent area hit testing.
4. Test mixed DPI, negative coordinates, rotation, monitor disconnect and competing bars.
5. Test user-controlled auto-hide conflicts without stealing registration.

Exit: maximized windows respect reserved area and no stranded input/reservation remains.
Failure: narrow placement modes or use coexistence overlay with documented behavior.

### Task 1.2: independent recovery spike

Create later: recovery executable, OS-state journal, heartbeat protocol and crash fixtures.
Do this before any taskbar hiding is enabled.

1. Watchdog handshake and ownership-aware atomic journal.
2. Restore command works independently through Task Manager/Run.
3. Kill/hang dock, kill watchdog, restart Explorer, suspend/resume, interrupt mutation.
4. Enter safe mode after repeated crashes. Restore native UI before relaunch/update.
5. No Explorer kill, Winlogon mutation, injection or elevation by default.

Exit: native access returns within candidate 3-second loss-detection budget on supported
builds; startup repairs stale owned journals. Otherwise keep hiding disabled.

### Task 1.3: identical renderer comparison

Create later: experiments/slint-dock/ and experiments/tauri-dock/.
Use the same ten built-ins, sample state, two monitors and panel interactions.

1. Prove HWND lifecycle, message integration/subclass cleanup and nonactivating dock.
2. Prove activating panels, focus return, outside-click and keyboard-only control.
3. Test Narrator/UIA semantics, high contrast and 100–200% text/DPI.
4. Measure full process-tree private memory, working set, CPU and frame/input timings.
5. Verify applicable Slint distribution/SDK terms and Tauri custom-command policy.

Exit: ADR-001 chooses exactly one production renderer with trace artifacts.
Slint preferred if hard gates pass; Tauri fallback under same budgets; WPF evaluation
only if both fail. Exact dependency/toolchain versions pinned after this decision.

### Task 1.4: task identity and native tray continuity

Create later: experiments/task-model/ and native-tray fallback test harness.

1. Separate application identity, window instance and dock pin.
2. Enumerate snapshot, receive WinEvents, debounce reconciliation.
3. Validate HWND generation/liveness on each action; test packaged apps/PWAs/elevation.
4. Group multiple windows and handle activation denial without focus hacks.
5. Prove route to native tray, language/input, security, VPN/sync and removable devices.
6. Classify private taskbar adapters as optional build-specific compatibility behavior.

Exit: honest supported matrix; no complete tray hosting promise without evidence.
If native tray access is unreliable, taskbar hiding does not ship.

## Stage 2 — extract the existing search and platform services

### Task 2.1: domain/action seams

Create later: protonsearch/src/domain.rs, action.rs and focused tests.
Modify later: search.rs, launcher.rs, main.rs.

Extract IDs, result DTOs, query generations and typed Action enum. Retain adapter for
legacy launch_command behavior until fixtures prove parity. Reject unsafe new action
inputs at the boundary. Do not recode query/ranking algorithms in this step.

Exit: existing launcher consumes new types with unchanged fixture results/actions.

### Task 2.2: storage seam

Create later: src/storage.rs, storage/migrations.rs, app_paths.rs.
Modify later: database setup in search/indexer/main/settings_ui/browser/git modules.

Centralize connections, busy_timeout, WAL and short-transaction policy; migrate scattered
schema ownership to ordered versions. Separate personal/rebuildable state for preview.
Test lock contention, shutdown, backup, corruption and retry.

Exit: database ownership clear, backups consistent, migration tests pass.

### Task 2.3: search facade

Create later: src/search_service.rs and tests/search_service.rs.
Modify later: main.rs fast/slow workers and source adapters.

Define SearchRequest, fast/slow batches, cancellation, snapshot/resync.
Keep latest-query coalescing and reject stale generation results.
Retain extraction child timeout and bounded worker queues.

Exit: prefix/action parity and no material regression against Stage 0 latency baseline.

### Task 2.4: Windows provider seam

Create later: src/platform/{mod,apps,windows,audio,power,hotkeys,clipboard}.rs.
Move native functionality from launcher/main incrementally.
Keep HWND/COM lifetime and unsafe safety invariants inside platform boundary.

Exit: old launcher works through adapters; provider tests and manual OS cases pass.

Only now consider workspace crates listed in architecture.html. Avoid a giant
move-only rewrite that obscures behavior changes.

## Stage 3 — useful dock alpha in coexistence mode

Create later: chosen dock executable/UI directory, widget registry, layout model,
Start/Search panels and task widgets.

1. One monitor dock with event-driven built-ins and typed actions.
2. Start cached app drawer; Search existing service; Alt+Space still available.
3. Running-app grouping and window picker; audio, clock, power/network and tray access.
4. Panel toggle, Escape/focus return and outside dismissal.
5. Add per-display windows and deterministic overflow.
6. Verify keyboard/Narrator, DPI/fullscreen/restart/quit before visual embellishment.

Exit: daily-use coexistence alpha; no taskbar hiding or external packages required.

## Stage 4 — customization, weather and profiles

Create later: widget manifest/config/view schemas, host DSL renderer, profile storage,
edit UI, weather broker, import/export validator.

1. Compact/wide/rich layouts with provider supported-size constraints.
2. Save/Cancel/undo and keyboard move/resize; removal doesn't remove recovery access.
3. Work/Home/Presentation profile snapshots; dormant instances keep state.
4. Per-monitor mapping, disconnect fallback and reconnect without duplicates.
5. Weather manual city, terms/attribution, caching and stale/offline state.
6. Export strips secrets/grants; malformed/unknown package import safely rejected.

Exit: restart/corruption/layout boundary tests pass; no layout jitter or hidden data leak.

## Stage 5 — controlled concurrent agents

Create later: job store/scheduler, capability broker, Hermes provider adapter,
approval views and execution boundary tests.
Modify later: ai.rs and existing main/settings agent state.

1. Remove global always-approve inheritance, fixed auth fallback and CLI secret transfer.
2. Persist job transitions and run IDs; reconcile interrupted jobs without replaying writes.
3. Start two independent jobs; fair queue; one desktop-input lease.
4. Per-run grants, exact approval binding, expiry, revocation, deadline and output limits.
5. Stop job/all jobs; track process trees and provider cancellation limitations.
6. Label native external adapters trusted full-user authority unless OS isolation proven.
7. Support a local inference adapter as optional installation; label remote data flow.

Exit: prompt injection, cross-run approval, forged IPC, path/reparse, timeout,
restart/replay and cancellation tests pass. Dock input remains responsive on failure.

## Stage 6 — preview release and optional replacement mode

Create later: signed preview installer/update feed, rollback mechanism and matrix report.

1. Publish exact supported Windows builds/modes/hardware.
2. Sign binaries/installers and authenticate update metadata; stage atomic update.
3. Restore native taskbar before update/uninstall; verify health before replacement reenable.
4. Test fresh install, side-by-side import, downgrade, offline/low disk and corrupted config.
5. Enable optional hiding only where all Stage 1 gates pass; native Explorer remains.
6. Recheck license attribution and SDK/API terms.

Exit: data/recovery/accessibility/search/security gates pass.
If hide mode fails, ship explicitly as coexistence preview; do not claim taskbar parity.

## Stage 7 — curated widget ecosystem

Create later: SDK docs, schemas, examples, validator, package lock and catalog manifest.

1. Stabilize first-party manifest/instance/view contracts.
2. Declarative packages first: validated assets/actions, digests, provenance and rollback.
3. Updates adding capabilities require consent; support revocation and compatible pins.
4. WASM logic later: no ambient WASI, per-guest fuel/epoch/memory budgets,
   deadlines for host calls, bounded output and worker crash isolation.
5. Adversarial sample packages must not hang UI or access ungranted resources.
6. Public catalog/payments/cloud sync are separate business/backend projects.

Exit: external developer builds supported sample without raw toolkit/OS API exposure;
malformed/hostile packages and revoked updates behave correctly.

## Measurement protocol and targets

Reference laptop/desktop hardware, supported Windows builds, indexing state, corpus size,
sample counts and cold/warm definitions must accompany every performance claim.
Candidate targets: warm surface p95 50 ms, 60 Hz frame p95 16.7 ms,
fast search p95 50 ms after debounce, content batch p95 200 ms, profile switch 100 ms,
resident dock/platform/recovery private bytes 150 MiB total and 10-minute idle CPU
0.5% normalized to machine capacity. Agent/model and search memory separately and
combined. These are goals, not measurements; Stage 1 may revise with a written reason.

## Stage completion evidence

For every stage: changed paths, test commands/results, native manual matrix where needed,
perf traces if relevant, unresolved failures, rollback path and next gate.
Never infer a Windows integration success from a browser mockup or passing Rust tests.
