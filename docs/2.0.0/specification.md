# Proton 2.0 product and system specification

Status: proposed for review, planning only.

Visual revision: [glass-dock-design.md](glass-dock-design.md) supersedes the original
green palette, uniform tile treatment and default dock composition. It proposes neutral
glass, an app/content dock plus a quiet status capsule, and different presentations
within the shared widget system. Architecture and safety contracts remain unchanged.
Date: 20 October 2026, Asia/Kolkata.
Baseline: 98d52ce, original branch 1.1.1; planning branch 2.0.0.

## Intent

Transform Proton Search into a customizable Windows workspace dock. Keep Proton Search
as the search experience and reuse its indexing, query, content extraction and Windows
launching investment. The dock contains widgets with meaningful compact, wide and rich
presentations, expanded panels and profile-specific compositions.

Working product name is Proton; no rename of repository, install identity or data is
authorized as an implementation consequence of this document.

## Product requirements

R01. Start, Search, Agents, apps, files, audio, battery, network, native tray access and
clock are configurable widget surfaces.
R02. A widget's size changes information density, not simply icon scale.
R03. Click toggles a panel; Escape closes it and returns focus. Panels stay in monitor
work areas and do not block fullscreen or secure desktop workflows.
R04. Users can add, remove, duplicate supported instances, reorder and resize widgets
through pointer and keyboard flows, with Save/Cancel and undo.
R05. Work, Home and Presentation presets have atomic manual switching, persistent
per-monitor composition and sensitive-preview controls.
R06. Switching profile does not start/stop applications or jobs, confer grants or
duplicate runtime state.
R07. Existing query prefixes, ranking semantics, hotkeys and fast/slow search phases
remain available. Search must be accessible without a functioning dock.
R08. Ordinary search never infers permission to execute an agent. A deliberate agent
action creates a job.
R09. Multiple jobs have independent grants, state, cancellation and budgets. Only one
computer-use job may hold desktop input at once.
R10. Everything visible uses the widget presentation contract; recovery, authorization,
focus and supervision are host-owned.
R11. Native taskbar coexistence is the initial mode. Hiding is optional and gated on
recovery, tray access, exact supported build testing and explicit user activation.
R12. No full Explorer replacement, Explorer injection or Winlogon mutation in 2.0.
R13. Preview uses separate data/install identity; migration never risks original data.
R14. High contrast, Narrator, keyboard navigation, text scaling, reduced motion and
localization are release requirements.
R15. Hundreds of widgets are an ecosystem goal. Launch inventory is a focused set of
first-party widgets; public executable extensions come later.
R16. Offline/error/stale/permission/hardware-absent states are designed for each provider.

## Architecture

Target units: domain, storage, search, platform-windows, widgets, dock, agent-runner,
recovery and later widget-worker. Start extracting modules in the existing crate;
move to a workspace when interfaces stabilize.

Dock owns composition/input and thin provider snapshots. Windows handles and COM/WinRT
interfaces stay in platform adapters with explicit thread/apartment ownership.
Search owns query routing, index lifecycle and bounded extraction children.
Agent runner owns durable jobs and provider adapters. Recovery is independent of
plugins, main databases and UI responsiveness.

Authenticated local IPC uses a versioned length-prefixed envelope, bounded payloads,
deadlines, request IDs, generation/revision tracking, cancellation and stable errors.
Caller identity comes from authenticated transport context, never JSON origin fields.
All privileged actions are re-authorized at execution time.

Keep personal state, rebuildable indexes and credential storage distinct. Ordered
schema migrations and consistent SQLite backups precede import. Settings/notes/pins/
chats/clipboard artifacts are not disposable indexes.

## Widget model

Definition != instance != provider != rendered view.
Supported sizes initially compact/wide/rich (1/2/3 units), optional per definition.
View DSL is renderer-neutral and host validated. No raw Slint/HTML/script/native API
exposure in early community packages. Built-ins use reviewed Rust providers.
Shared subscriptions prevent one poller per instance.

Third-party executable logic is postponed until Wasmtime worker, host-call limits,
capability mediation, process quotas, package provenance and revocation are proven.
Signatures establish origin/integrity, not safety.

Profiles store layouts, privacy, appearance, start sections and non-authorizing defaults.
Exports omit secrets/grants. Disconnected display mappings persist for reconnection.

## Agent model

Local orchestration is distinct from local model inference and local data retention.
Hermes is one adapter behind the scheduler. Existing global approval flags, fixed
gateway key fallback and credential command-line transfer are migration debt.
Untrusted content never grants authority. Approval is exact, expiring and bound to
run/tool/resource; revocation applies at execution.

Arbitrary native tools running as the user must be labeled trusted full-user execution
unless a tested OS containment mechanism restricts them. Process separation and Job
Objects alone do not establish a security sandbox.

## Provisional stack

Rust stable/MSVC + windows-rs + SQLite FTS5; Slint first renderer candidate.
Compare Tauri 2 + React/TypeScript/Vite on an identical prototype before ADR lock.
WPF + Rust services is a fallback if both fail hard Windows/accessibility gates.
Avoid committing to dual permanent renderers. Keep versions unchanged during planning.

Slint licensing and public SDK API exposure require clarification. Review exact terms
before renderer lock; renderer-neutral schema is not automatic legal clearance.
Weather provider terms/attribution/commercial use are separate release decisions.

## Success and acceptance

Stage-specific gates and candidate budgets are in roadmap.html and implementation-plan.md.
No target is a measured result. Establish reference hardware, corpus, Windows builds,
sample counts and cold/warm definitions before reporting performance.

Hard gates: search parity, usable native fallback, recovery, data preservation,
keyboard/accessibility, scoped agent actions and signed update rollback.
Failed replacement compatibility means coexistence-only release, honestly described.

## Source of detail

The HTML chapters are the detailed product/system design. Primary sources and their
scope are in sources.html. Repository evidence is in migration.html. Implementation
starts with baseline and feasibility, not a full application rewrite.
