# ProtonSearch v2 — v10 Gate G1 Architecture & Contract Review

Owner: L1-E (rel-recon). Task: task-36. Date: 2026-11-10 (snapshot HEAD 3747c71; working tree includes
G2/G4/G5/G6/G7 scaffold commits 03633c7 and 3747c71 plus in-flight edits; 44 uncommitted changes at
review start). G2 is executing in parallel, so line numbers are a snapshot and must be re-checked
before merge. No code was edited for this review.

Inputs: docs/reference/v10-handoff master prompt section 14 (G1), V9 audit/settings concept,
docs/implementation/V10_GATE0_AUDIT.md, FEATURE_TEST_MATRIX.md, CROSS_FEATURE_PAIRWISE_MATRIX.md,
BUG_LEDGER.md, RELEASE_GATES.md, docs/design/ISLAND_V9_SPEC.md, ISLAND_V9_GAP_LIST.md,
ISLAND_SETTINGS_SPEC.md, docs/qa/preview-audit/PREVIEW_P0_REPORT.md,
docs/qa/clipboard-audit/CLIPBOARD_IMAGE_P0_REPORT.md, docs/adr/ADR-0001..0006,
docs/arch/OVERVIEW.md, and the current tree (crates/**, apps/desktop/**).

## 1. Verdict

CONDITIONAL PASS for G1. The architecture and contracts are concrete enough for G2 to continue in
parallel: the two-launcher settings model, island window/placement maths, monitor enumeration,
preview DTOs, clipboard image result fields, agent task store/API and the new Tauri command surface
all exist in code, and the existing 6 ADRs have been reviewed. However, seven contract/architecture
conditions (B1-B7 below) must be closed before the corresponding G2/G5/G6/G7 tasks can be accepted,
because they affect contract ownership, payload bounds, persistence migration and security. G1 does
not block code work; it blocks acceptance and gate closure.

## 2. Architecture map (current tree)

### 2.1 Process and windows

- One Rust host process (apps/desktop/src-tauri/src/main.rs). Windows created in create_windows:
  - main label "main" (desktop settings app), frameless launcher label "launcher", and the new island
    label "island" (main.rs:27-33, 359-478). All three load index.html; the frontend routes by window
    label/param (apps/desktop/src/main.tsx:9-20; App.tsx:266-269).
  - Launcher: 820 logical card width (1180 with preview) + 28 px transparent shadow padding,
    frameless, transparent, shadow off, always-on-top, skip-taskbar, non-resizable, hidden until
    summoned (main.rs:38-49, 401-432).
  - Island: V9 surface 520x66 + 96 px transparent shadow padding, hidden by default, focused(false),
    always-on-top, skip-taskbar, non-resizable (main.rs:29-33, 434-476). The surface size is
    reported/updated by the UI through set_island_geometry (main.rs:1148-1159); the native window is
    the surface plus padding so the V9 shadow is not clipped (main.rs:167-204).
  - Monitor enumeration is a new Win32 module: EnumDisplayMonitors + GetMonitorInfoW +
    GetDpiForMonitor, returning physical rects, work area and effective DPI (monitors.rs:1-74),
    exposed as MonitorInfo through the list_monitors command (main.rs:1133-1136; ps-ipc:850-865).
- Tray, single-instance, global shortcut and exit cleanup are in main.rs:1173-1292. The global
  shortcut now toggles the primary launcher mode: island when primary_mode=DynamicIsland and
  island.enabled, otherwise the normal launcher (main.rs:257-266).
- React views: launcher (apps/desktop/src/launcher/**), desktop app (apps/desktop/src/app/**: Home,
  Search & indexes, Activity, Workflows, Preferences, Island settings view), onboarding, and the new
  island surface (apps/desktop/src/island/**: IslandView, IslandCards, IslandPageDots, islandDims,
  islandData, islandLogic tests). The island is currently a placeholder/shell against the V9 spec
  (ISLAND_V9_GAP_LIST.md), not the full eight-card product.
- Data path: Engine facade in ps-core; a background thread starts scanners after windows exist
  (main.rs:1227-1230); search never blocks on extraction/agent/Python.

### 2.2 Crates and data ownership (target = one core, two launchers, optional agents)

| Crate | Owns | Notes |
|---|---|---|
| ps-ipc | DTOs, settings schema, AgentEvent, preview/island/clipboard/monitor types | single source of truth per ADR-0006; but see C1 (agent task model lives in ps-agent) |
| ps-core | Engine facade, coordinator/ranker, settings apply/persist, activity, migration, clipboard listener lifecycle | engine.rs:849+ update_settings currently applies patch without v10 validation |
| ps-index | compact filename arena, scan/persist/incremental | unchanged from v2 |
| ps-content | SQLite FTS5 content index, extractors, legacy import | preview extraction seam for G2 |
| ps-sources | apps/recent/clipboard/browser/git/quicklinks/calculator/actions | clipboard image capture/persistence for P0-2 |
| ps-windows | Win32 helpers: icons, shell, clipboard formats, foreground window, monitors/DPI | clipboard decode/format support for P0-2 |
| ps-ocr | Windows.Media.Ocr extraction pipeline | OCR FTS indexing still pending |
| ps-agent | Pi runtime (hardened), optional Pydantic fallback, NEW runtime-agnostic task model/store/service | task/{model,store,service}.rs + pi/** |
| ps-bench | dev benchmark CLI | not shipped |
| apps/desktop/src-tauri | Tauri shell, windows/hotkey/tray, command surface, agent task service wiring | L0-owned |
| apps/desktop/src | React launcher/island/desktop/onboarding UI | L1-B and L2-B owners |

Storage: data_dir = %APPDATA%\protonsearch\v2 (ps_core paths). Known files: index.db (SQLite WAL),
settings.json (atomic), filename-index.bin, cache/icons/**, agent task store
<data_dir>/agents/tasks.json (new, ps-agent/src/task/store.rs:1-9,24-25,49-52), clipboard image
cache under the clipboard source dir. Legacy %APPDATA%\protonsearch\file_index.db is read-only
migration input.

### 2.3 Query/latency path and preview seam

Hotkey -> Tauri handler -> show/focus pre-created window -> search(SearchRequest) -> Engine tiered
search -> SearchResponse -> UI drops stale requestId. New v10 seam: the launcher preview panel
(Ctrl+Space) consumes a PreviewPayload from apps/desktop/src/launcher/previewClient.ts. The current
provider is metadata-only and returns the literal fallback "Preview not available — press Enter to
open." for filename/file results (previewClient.ts:7-10, 71-106, 129-138; ResultPreview.tsx:50-100).
This matches the G0-A finding: no preview pipeline/IPC command existed, so G2 task-37 (backend) and
task-38 (UI) are in progress. ps-ipc now defines PreviewRequest/PreviewPayload (ps-ipc:867-910) but
no Tauri preview command or engine implementation exists yet (no preview in the invoke_handler,
main.rs:1238-1274; no preview in bridge/api.ts). Contract condition B2.

### 2.4 Island settings/placement contract

- Typed settings: LauncherMode {normal,dynamic_island}; IslandCardId (8 cards, snake_case);
  IslandCardConfig{id,enabled}; IslandAnchor (9 anchors); IslandPlacement{monitor,anchor,offsetX,
  offsetY,keepInWorkArea,autoFlipExpanded}; IslandSettings{enabled,placement,cards,defaultCard,
  showDots,hideOnFocusLoss,skipDisabled,returnToSearch}; LauncherSettings{primaryMode,island};
  ClipboardSettings{captureText,captureImages,retentionDays,maxItems,pause,excludeApps,clearOnExit};
  MonitorInfo; PreviewRequest/PreviewPayload (ps-ipc:646-910). Settings gains launcher and clipboard
  with serde defaults (ps-ipc:928-931).
- Native application: place_island computes anchor fraction x work area, applies offsets * monitor
  scale, optionally clamps into work area, resizes native window to surface+padding and positions it
  (main.rs:145-205). update_settings hot-applies island placement and hides both launchers on a
  primary-mode change (main.rs:535-579). set_island_geometry clamps surface size to 240-900 x 40-760
  (main.rs:1148-1159). Show/hide/toggle/island focus-loss behavior: main.rs:207-255, 464-475.
- Gaps: validation/guarantees are weaker than the contract (C5/C6); no monitor hotplug handler; no
  persisted monitor-id healing; keepInWorkArea=false disables the clamp entirely. B5.

### 2.5 Agent task contract (new in v10)

- Runtime-agnostic task model lives in crates/ps-agent/src/task/model.rs: AgentTask, AgentTaskState
  (queued/running/completed/failed/cancelled/offline), AgentTaskRuntime (pi/pydantic/unknown),
  AgentTaskProgress (determinate only with real fraction), AgentTaskControls (canCancel/canPause/
  canResume/canRetry; pause/resume hard-false and the service returns Unsupported - honest semantics:
  service.rs:162-173, model.rs:213-233), permissions (requested/granted/denied/pending, readOnly),
  log ring, error, restarts, model, writeAccess. Limits: 200 tasks, 100 log entries, 80-char title,
  200-char summary, 300-char log line, 400-char error; task schema version 1 (model.rs:17-30).
- Persistence: <data_dir>/agents/tasks.json, versioned JSON, atomic tmp+rename, corrupt file moved
  aside, non-terminal tasks recovered as Offline after app restart, prunes oldest terminal tasks
  (store.rs:1-9, 27-93, 471-525, 555-580). Privacy: prompt text/model output/tool payloads are never
  persisted; only redacted title/summary/logs/errors (store.rs:1-9, 155-216, 218-346; model.rs:1-11,
  383-400). Redactions are applied from the Vec passed at open.
- Tauri wiring: AppState.agent_tasks = Arc<AgentTaskService> opened at setup with
  AgentTaskService::open(&data_dir, Vec::new()) (main.rs:62, 1198-1205); command surface:
  agent_tasks, agent_task_tree, agent_task, agent_permissions, agent_stop_all, agent_pause,
  agent_resume (main.rs:1074-1131; registered 1262-1268), plus agent_status/start/cancel/approve/
  stop (main.rs:938-1072). agent_start records a task and maps the canonical task id to the runtime
  run id, applies Pi events into the store, emits agent:event/agent:task (main.rs:943-995). The RR-02
  alpha gate forces Pi allowWrites=false server-side until a per-run consent token exists
  (main.rs:959-962) - security-positive.
- Gaps: task model is not in ps-ipc (C1); redactions list is empty at open (C10); cancellation is
  coarse (cancels every non-terminal task in the window; main.rs:1019-1034) (B6).

## 3. Contract review: ps-ipc v10 additions + new Tauri commands

### 3.1 ps-ipc additions

| Contract | Location | Verdict | Findings |
|---|---|---|---|
| LauncherMode / IslandCardId / IslandAnchor / IslandCardConfig / IslandPlacement / IslandSettings / LauncherSettings | ps-ipc:646-807 | ACCEPT WITH CHANGES | Good typed model, camelCase wire, snake_case enums; missing server-side validation for card list (duplicates/missing/default-disabled), offsets, monitor ids; no schema version/migration (C4/C5) |
| ClipboardSettings | ps-ipc:809-848 | REVISE | captureText/captureImages default true while privacy.clipboard_history defaults false (ps-ipc:505-525): two gates for the same capture path, precedence undefined; retention/max items not enforced by a documented cleanup contract; excludeApps semantics/format undefined (C3) |
| MonitorInfo | ps-ipc:850-865 | ACCEPT WITH NOTES | Physical rect + work area + scale; id is the device name (monitors.rs:45) which can change across dock/undock; no stable UUID/ordinal contract; document/heal (C5/C8) |
| PreviewRequest / PreviewField / PreviewRange / PreviewPayload | ps-ipc:867-910 | ACCEPT DIRECTION, INCOMPLETE | No Tauri command/engine implementation yet; only resultId (good: no arbitrary paths over IPC) but the engine must authorize result selection; format is a free string not enum; no truncated/error code; imageDataUrl/ocrText unbounded; maxChars optional/unbounded; no request id/stale contract; no both-launcher parity rule (C2/B2) |
| ResultMeta thumbnailDataUrl/imageWidth/imageHeight/mime | ps-ipc:252-281 | REVISE | Good direction (data URL + dimensions + MIME) but no documented size bound, MIME allowlist, dimension sanity, or rule that only clipboard-image rows carry it; up to 50 rows x data URL can inflate IPC/WebView memory; tie to ADR-0009 (C3) |
| Settings.launcher / Settings.clipboard / SettingsPatch.launcher/clipboard | ps-ipc:912-1008 | ACCEPT WITH CHANGES | Additive serde-default fields load old settings.json; DATA_SCHEMA_VERSION is still 1 (ps-ipc:20) and PROTOCOL_VERSION still 1 (ps-ipc:17); no v10 migration test/version policy; agent task store has its own schema version 1; define versioning (C4) |
| AgentEvent | ps-ipc AgentEvent | ACCEPT | Frozen v2 event stream reused by Pi/Pydantic and the task store; no change needed beyond redaction guarantees |
| AgentTask / AgentTaskNode / permissions / controls / DetectedAgent | NOT in ps-ipc - crates/ps-agent/src/task/model.rs and Tauri DetectedAgent struct (main.rs:784-795) | REVISE / ADR REQUIRED | Violates ADR-0006 (ps-ipc is the single source of truth; TS mirrors it via an explicit version bump). The model file itself says it is designed to be lifted into ps-ipc (model.rs:1-11). Today bridge/types.ts manually mirrors ~80 lines of AgentTask types; drift is detected only by tests/manual review. Decision needed: move to ps-ipc (preferred) or formally declare ps-agent-owned shell DTOs and generate/check TS (C1/B1) |

### 3.2 New Tauri commands (registered main.rs:1238-1274)

| Command | Location | Review |
|---|---|---|
| list_monitors | main.rs:1133-1136 | Read-only enumeration; safe; single-monitor host limits test coverage |
| show_island / hide_island | main.rs:1138-1146 | Same-user WebView can show/hide the island; no arguments; acceptable for trusted UI; no generation token on hide path (C6) |
| set_island_geometry(width,height) | main.rs:1148-1159 | Clamps size (240-900 x 40-760) - good; position comes from persisted placement; no monitor-overflow check for the surface itself (C7); must validate against work area in G5 |
| detect_agents | main.rs:891-936 | Caches results; probes Pi via cached probe; runs "where <exe>" then "<path> --version" with 2.5 s timeout / 64 KiB cap / CREATE_NO_WINDOW; rejects cmd metacharacters & ^ | < > " % CR LF in the path and quotes a .cmd via cmd /d /s /c (main.rs:797-888). Residual: executing a user-PATH agent binary with --version on first panel open is a same-user code-execution surface; version text is truncated (80 chars) and not used for trust (C9) |
| agent_tasks / agent_task_tree / agent_task / agent_permissions | main.rs:1074-1104 | Bounded limits (<=200); read-only snapshots; no secrets by design (task model stores only redacted summaries) - but empty redactions at store open (C10); types not in ps-ipc (C1) |
| agent_stop_all / agent_pause / agent_resume | main.rs:1106-1131 | stop_all cancels task records and shuts the Pi client; pause/resume return the service Unsupported error (honest) - contract should document the error shape to UI |
| agent_start / agent_cancel / agent_approve / agent_stop / agent_status | main.rs:938-1072 | agent_start gates on settings.agent.enabled, forces Pi read-only for alpha (RR-02), records a task, maps events; agent_cancel cancels every non-terminal task then the Pi client (coarse; run_id ignored for Pi); approval is explicitly rejected for Pi (no per-tool approvals). Good security baseline |
| preview command | MISSING | ps-ipc defines preview DTOs but no Tauri command or engine implementation; task-37/38 in flight. This is the P0-1 contract gap and a G2 acceptance blocker (B2) |
| update_settings island hot-apply | main.rs:535-579 | Re-places the island on launcher setting changes and hides launchers on a mode change; no validation of island cards/offsets, no monitor hotplug handling; relies on Engine.apply only (B4/B5) |

### 3.3 Capabilities/security surface

New commands add no Tauri plugin permissions: capabilities remain core:default + global-shortcut:
default; no shell/http/fs plugin was added for v10 (verify at G2 merge). All commands are callable by
the same-user WebView, consistent with the local trusted-UI model; security-critical side effects are
bounded (read-only agent alpha gate, destructive native actions still require the V2-04 15 s
confirmation handshake in engine.execute_action). Detect-agents version probes and monitor
enumeration are the only new native side effects.

## 4. ADR acceptance list

The repo has ADR-0001..0006 only (docs/adr/). The brief expected ADR-0007 to exist; it does not -
recorded as a documentation gap, not a code gap. New ADRs below are numbered from 0007.

| ADR | Current status | G1 verdict | Required action |
|---|---|---|---|
| ADR-0001 frontend stack (Tauri 2 + React/TS) | Accepted (Gate 1) | ACCEPT (unchanged) | Keep; v10 island/settings are the same stack |
| ADR-0002 search core (in-memory filename arena + tiered coordinator) | Accepted, perf provisional | ACCEPT (unchanged) | Keep; 100k/1M targets met on a28042d; content tier is ADR-0003's problem |
| ADR-0003 FTS5 vs Tantivy | Provisional | KEEP PROVISIONAL | Content p95 43 ms vs 30 ms target at HEAD 8692997; re-confirm at G3/G5 with the snippet/rank-only split |
| ADR-0004 OCR engine | Provisional | KEEP PROVISIONAL | OCR extraction measured; OCR FTS indexing/search integration still pending; re-confirm at G3 |
| ADR-0005 agent runtime (Pydantic AI) | Accepted (Gate 6) | REVISE / SUPERSEDE DIRECTION | User decision: Pi CLI is the v10 default runtime (hardened; 71 tests) with Pydantic kept as optional fallback. Update status/decision/evidence and security reference to docs/security/PI_AGENT_SECURITY.md; record the alpha read-only gate RR-02 |
| ADR-0006 IPC contract | Accepted (Gate 1) | ACCEPT WITH CONDITIONS | Add v10 addendum: (a) protocol/data-schema version policy for additive vs breaking changes (both still 1 despite v10 additions), (b) move AgentTask/DetectedAgent into ps-ipc or declare shell-owned DTOs; keep TS checked against the Rust schema |
| ADR-0007 (new) Two-launcher settings model | Missing | ACCEPT WITH CHANGES | Define LauncherSettings as source of truth: primaryMode + island + cards; validation rules (unique card ids, default card enabled, offsets clamped, search enabled); settings schema version 2 + migration from v1; clipboard privacy/capture precedence; owner L0/L1-A; tests required |
| ADR-0008 (new) Island window/placement contract | Missing | ACCEPT WITH CHANGES | Invariants: always at least N px visible (clamp even when keepInWorkArea=false), 9 anchors + offsets as DIPs, work-area/taskbar/DPI handling, monitor-id fallback/healing + hotplug recovery, shadow-pad native window contract, focus-loss policy, always-on-top/skip-taskbar flags, show/hide generation token; multi-monitor evidence blocked on hardware; owner L0 |
| ADR-0009 (new) Clipboard image storage + IPC | Missing | ACCEPT WITH CHANGES | Storage contract (content hash, PNG, thumbnail, dimensions, retention, max items/bytes, secure wipe, clear-on-exit, exclude apps), format precedence/dedup across CF_DIB/CF_DIBV5/CF_BITMAP/PNG/HDROP/HTML, IPC bounds (thumbnailDataUrl <= agreed bytes, MIME allowlist, dimension sanity, only image rows), paste-back contract/fallback, single privacy gate; link CLIPBOARD_IMAGE_P0_REPORT root causes; owner L2-A/L1-A |
| ADR-0010 (new) Preview IPC | Missing | ACCEPT WITH CHANGES | Add preview command (task-37) with: resultId-only input resolved via engine (no arbitrary paths), maxChars clamped/default, format enum, text/snippet/highlight ranges validated against text length, image data URL bounds, OCR snippet policy, explicit unsupported/error/truncated states, stale request/cancel contract, both launcher presentations parity; link PREVIEW_P0_REPORT; owner L1-A/L0 |
| ADR-0011 (new) Agent task persistence + contract | Missing | ACCEPT WITH CHANGES | Decide ownership of AgentTask DTOs (prefer ps-ipc), persistence file path/schema/migration, redaction inputs (Tauri must pass known-secret redactions; empty today), retention/pruning, cancel-by-task semantics, honest pause/resume, no prompt/model-output persistence (verified), crash recovery/Offline, error/log caps; owner L1-D/L0; security review by L1-E |
| ADR-0012 (new) Native adapters | Missing | ACCEPT WITH CHANGES (implementation G7) | Capability-probe DTO in ps-ipc, allowlisted WinRT/Win32 APIs per function (media/GSMTC, audio endpoints, Wi-Fi, Bluetooth, brightness/volume, notifications), bounded calls/no shell strings, disabled/redirect fallbacks, permission guidance, real-hardware test matrix and honesty states; owner L1-A/L2-A |

## 5. Reviewer assignments for G2-G9 (no self-review)

Ratified structure: L0 + 5 L1 leads + 3 L2-scope specialists (sources-content under L1-A; ui-launcher
and ui-app under L1-B); no L3 workers. Owners implement; an independent reviewer re-runs evidence.
A reviewer may be an L1/L2 owner only for a different scope; L1-E can review all non-L1-E scopes;
L0 is final acceptance authority.

| Gate/workstream | Owner (implementer) | Independent reviewer(s) | Evidence required |
|---|---|---|---|
| G2 preview backend (task-37) | engine-core (L1-A) | rel-recon (contract/security/bounds) + ui-app (consumer contract) | IPC/command contract, fixture excerpts/hashes, both launchers screenshots, unit/integration/E2E |
| G2 preview UI (task-38) | ui-app (L2-B2) | engine-core (payload semantics) + rel-recon (safety/fallbacks) | screenshots, keyboard/a11y, zero-blank behavior, stale/cancel tests |
| G2 clipboard images (task-39) | sources-content (L2-A) | rel-recon (privacy/storage bounds) + engine-core (IPC/persistence) | real Windows bitmap copy tests (Paint/Snipping Tool/browser), restart/re-paste, format matrix, BUG_LEDGER before/after |
| G3 search/retrieval | engine-core (L1-A) + perf-bench (L1-C, measurement only) | rel-recon (regression/contract) + perf-bench (methodology only) | benchmark JSON before/after, relevance golden, content p95 plan/status |
| G4 Normal Launcher (task-32) | ui-launcher (L2-B1) | rel-recon (integration/E2E) + perf-bench (latency) | real-app E2E, keyboard/focus, preview seam, DPI screenshots |
| G5 V9 Island renderer (task-41) | ui-launcher (L2-B1) | design-spec (V9 fidelity; not implementation owner) + rel-recon (integration/security) + perf-bench (frame pacing) | reference comparison/screens, motion capture metrics, card data adapters, no-clipping evidence |
| G6 Settings/placement/card manager (task-40) | ui-app (L2-B2) | design-spec (spec conformance; not code owner) + rel-recon (persistence/migration/security) | drag/keyboard placement E2E, Apply/Cancel/Reset, restart migration, screenshots |
| G7A agent runtime/task model (task-34) | agent-sidecar (L1-D) | rel-recon (security/privacy/RR-02) + engine-core (IPC/persistence contract) | task lifecycle/crash/offline tests, redaction tests with planted secrets, no model output persisted, approvals/no-approval honesty |
| G7B native adapters (task-35) | sources-content (L2-A) | engine-core (Win32/IPC) + rel-recon (permission/security) + perf-bench (device action latency) | capability matrix on supported/unsupported hardware, honest disabled states, bounded calls |
| G8 cross-feature verification | rel-recon (L1-E, matrix owner) | L0 (final) + engine-core + perf-bench + design-spec sample re-runs; L1-E cannot self-accept | executed FEATURE_TEST_MATRIX + pairwise evidence, security audit, benchmark report, bug dispositions |
| G9 release | L0 (with L1-E evidence) | L0 final acceptance + perf-bench (benchmark reproducibility) + engine-core (install/migration spot check); L1-E prepares, does not self-approve | installer hashes/signing status, migration/rollback drill, screenshots/video of 8 cards + placement editor, docs/changelog, honest gaps |

Conflict-of-interest guards: (1) owner never signs their own work; (2) L1-B may review visual fidelity
but must not be the sole reviewer of L1-B-owned implementation - L1-E co-reviews; (3) shared-contract
changes (ps-ipc, bridge, tokens) require L0 review; (4) L1-E's G8/G9 evidence is accepted by L0 with
independent sample re-runs by at least two other leads; (5) reviewers record command/run IDs and
screenshots in the task handoff, not just an opinion.

## 6. G1 exit checklist

| Criterion | Status | Notes |
|---|---|---|
| Architecture map current + v10 target with gaps | PASS | section 2; island shell/cards placeholder gap documented in ISLAND_V9_GAP_LIST.md |
| ps-ipc v10 additions reviewed with file:line | PASS | section 3.1; findings C1-C10 recorded |
| New Tauri commands reviewed with file:line | PASS | section 3.2; preview command missing; detect_agents residual documented |
| ADR-0001..0006 reviewed | PASS | section 4 |
| New ADR list with accept/revise verdicts | PASS (conditional) | ADR-0007..0012 proposed; ADR-0005/0006 need revision/addendum |
| Reviewer assignments G2-G9 (no self-review) | PASS | section 5 |
| Ratified 9-member structure confirmed | PASS | L0 + 5 L1 + 3 L2-scope; no L3 workers |
| Contract/version/migration policy | CONDITIONAL | DATA_SCHEMA_VERSION/PROTOCOL_VERSION still 1; task store schema v1; migration tests pending (C4) |
| P0 contracts closed | CONDITIONAL | preview command and clipboard image bounds/ADR still missing; G0 reproductions complete, G2 in progress |
| Architecture risks documented | PASS | B1-B8 below |

### Blockers to G2 acceptance (G1 conditions)

- B1 AgentTask/DetectedAgent contract home (ps-ipc vs shell-owned) must be decided and recorded
  (ADR-0011); bridge/types.ts must be checked against it.
- B2 Preview IPC: command + engine implementation + bounded PreviewPayload + stale/cancel/error/
  unsupported contract (ADR-0010) before task-37/38 acceptance.
- B3 Clipboard image storage+IPC contract: thumbnail bounds, MIME allowlist, dimensions, single
  privacy gate, retention/wipe/dedupe/paste-back (ADR-0009) before task-39 acceptance.
- B4 Two-launcher settings validation + schema version/migration/test (ADR-0007); update_settings
  must reject invalid island card lists/offsets/defaults rather than persisting them.
- B5 Island placement invariants (always-visible clamp, monitor-id healing/hotplug, generation token
  for hide/re-summon) before G5 acceptance (ADR-0008); currently keepInWorkArea=false allows an
  unreachable island and the toggle hide has no generation guard.
- B6 Agent task store redactions: Tauri opens the store with an empty redaction list; either pass
  known-secret redactions (provider env values) or document/store-level default redaction, and test
  with planted secrets. Also define per-task cancel semantics (current Pi cancel cancels all active).
- B7 ADR-0005 revision to Pi default/Pydantic fallback, ADR-0006 v10 addendum/versioning, and the
  missing ADR-0007 numbering gap must be fixed so G1 ADR acceptance is auditable.
- B8 Environment blockers (not code): single monitor -> dual/mixed-DPI/disconnect placement tests
  blocked; no signing cert -> unsigned-only release claim; no non-Pi provider credentials -> live
  agent runs unverified; G0-C perf report pending separately (G1 removed the hard dependency, but G3/
  G4/G5 performance acceptance needs it).

## 7. Open decisions for L0

1. Where AgentTask/DetectedAgent types live (recommend ps-ipc).
2. Whether v10 additive DTOs bump PROTOCOL_VERSION and DATA_SCHEMA_VERSION (recommend protocol 2,
   data schema 2 + migration test) or a documented compatibility policy is adopted.
3. Island visibility invariant: remove keepInWorkArea=false or always clamp; minimum visible px.
4. Clipboard capture single gate: privacy.clipboard_history vs clipboard.captureText/Images precedence.
5. Preview ownership: engine resolves resultId -> path via index; never accept paths from UI.
6. Native adapters scope: which APIs are in v10 vs deferred/CUT, with capability honesty.
7. Whether the write-enabled agent path opens at all in v10 alpha (current server gate forces
   read-only; RR-02 remains NO-GO for write UI until a token handshake).

## 8. Method/evidence

Reviewed files: crates/ps-ipc/src/lib.rs; apps/desktop/src-tauri/src/{main.rs,monitors.rs};
apps/desktop/src/{main.tsx,App.tsx,bridge/api.ts,bridge/types.ts,launcher/previewClient.ts,
launcher/ResultPreview.tsx,island/**,app/views/IslandSettingsView.tsx}; crates/ps-agent/src/task/*,
crates/ps-agent/src/lib.rs; crates/ps-core/src/engine.rs settings path; docs/adr/ADR-0001..0006;
docs/design/ISLAND_*; docs/qa/preview-audit/PREVIEW_P0_REPORT.md; docs/qa/clipboard-audit/
CLIPBOARD_IMAGE_P0_REPORT.md; docs/arch/OVERVIEW.md; docs/implementation gate/matrix docs.
Commands: read-only greps/reads; git log/status/rev-parse; no builds or tests were run for this review
(G2 is compiling in parallel); verdict is architecture/contract based.

Status: G1 CONDITIONAL PASS (task-36). G2 may continue in parallel; the B1-B7 contract conditions
must be closed before the corresponding G2+ acceptance. Next actions: L0 decisions in section 7,
then L1-E re-reviews the contract changes at the G2 acceptance review.


## 9. Post-G1 decisions and fix status (2026-11-10, task-45)

L0 accepted the G1 review and issued decisions; the shell fixes were landed in the working tree while
this ADR/policy set was written. This section records the decisions and re-review status; the earlier
sections remain the original review snapshot.

| Decision (L0) | Status | Evidence / follow-up |
|---|---|---|
| 1. Move AgentTask/AgentTaskNode/DetectedAgent DTOs to ps-ipc; bridge/types.ts becomes a documented mirror | DECIDED, delegated to task-44 (agent-sidecar, pending at review) | ADR-0011; types remain in crates/ps-agent/src/task/model.rs and Tauri DetectedAgent until task-44 lands; re-review then |
| 2. No PROTOCOL_VERSION/DATA_SCHEMA_VERSION bump; additive serde-field policy documented + legacy-settings-parse test | DONE | docs/adr/IPC_ADDITIVE_VERSIONING_POLICY.md; constants still 1 (ps-ipc:17,20); legacy parse test PRESENT at crates/ps-ipc/src/lib.rs:1273-1297 (loads v1 JSON with defaults); additive config test also present at ps-ipc:1299+ |
| 3. Island: always clamp (work area when keepInWorkArea, monitor bounds otherwise); offsets +/-2000; size clamped to monitor; generation token on hide; hotplug healing documented follow-up | LANDED | main.rs:196-261 (always clamp + size limits), 605-635 (offset validation, 8 unique cards, enabled default), 1253-1261 (size clamp), 269-320 (ISLAND_GENERATION show/hide guard); ADR-0008; hotplug healing remains a G6 follow-up |
| 4. Clipboard: single master gate = privacy.clipboard_history AND !clipboard.pause; captureText/captureImages are format toggles; engine enforces thumbnail <=128 KiB, dims <=8192, MIME png/jpeg allowlist | CONTRACT LANDED / ENGINE BOUNDS PENDING task-39 | ps-ipc:809-853 (gate + docs + defaults), validation ranges main.rs:627-633; ADR-0009; engine enforcement of the three image bounds must be proven by task-39 tests before G2 acceptance |
| 5. Preview: resultId-only command approved; PreviewPayload gains status/truncated/error/imageMime; bounds/caching owned by engine task-37 | PARTIAL: engine landed, Tauri command/bridge pending task-37 | engine.preview + LRU cache + honest unknown-id payload at engine.rs:667-719; tests engine.rs:3337-3559 (text/PDF/DOCX/image/OCR/unknown/cache); ps-ipc:872-934; ADR-0010; shell command absent in the invoke_handler at review time |
| 6. Native adapters: capability-gated; milestone 1 audio/brightness/notifications, then media/Wi-Fi/Bluetooth (task-35) | DECIDED, implementation in progress | ADR-0012; task-35; capability matrix + real-hardware evidence required at G7B |
| 7. Write-enabled agent stays CLOSED in alpha (allowWrites=false) | LANDED | main.rs:1060-1075 forces config.allow_writes=false for Pi regardless of settings; RR-02 remains NO-GO for write UI until a server-side per-run consent token exists; ADR-0011 |

Verified shell/contract fixes (file evidence):
- settings validation before persist: main.rs:605-635 (8 unique cards, enabled default card, offsets
  +/-2000, clipboard retention 0-3650, max items 10-100000), used by update_settings (637-659).
- agent task store redactions from provider environment values: main.rs:1303-1322 (OPENAI/ANTHROPIC/
  GEMINI/GOOGLE/OPENROUTER/DEEPSEEK/AZURE/XAI/GROQ keys when >=8 chars) passed to AgentTaskService.
- island clamp + generation: main.rs:196-261, 269-320; launcher generation unchanged (78-79, 162-169).
- size clamp: main.rs:1253-1261 (240-900 x 40-760) and monitor-bounded surface (221-251).
- read-only agent alpha gate: main.rs:1060-1075.

Remaining re-review conditions:
- R1: task-44 lands (DTO move complete, bridge/types.ts documented mirror, tests/contract check).
- R2: task-37 lands the Tauri preview command + bridge method with bounded payload; task-38 UI wiring.
- R3: task-39 lands engine image bounds (thumbnail <=128 KiB, dims <=8192, MIME png/jpeg allowlist)
  and real Windows clipboard matrix; ADR-0009 evidence updated.
- R4: ADR-0005 revision (Pi default/Pydantic fallback) and ADR-0006 v10 addendum are L0-owned; the
  additive policy (docs/adr/IPC_ADDITIVE_VERSIONING_POLICY.md) amends ADR-0006 for the v10 decision.
- R5: island hotplug healing remains a G6 follow-up and must be recorded in the G5/G6 evidence.
- Environment blockers remain: single-monitor (dual/mixed-DPI/disconnect tests), no signing cert, no
  non-Pi provider credentials.

Updated verdict: G1 decisions are implemented/documented except the delegated DTO move (task-44) and
the in-flight preview command/image-bounds work. **G1 remains CONDITIONAL PASS until the R1-R3
re-review; G2 acceptance must include R1-R3 as gate conditions.** No code was edited for this task.


## 10. G2 P0 re-review (task-46, 2026-11-10): R1-R3 + UI spec review + reviewer sign-off

Snapshot: HEAD 93f84d1 (v10 checkpoint: preview backend+IPC command/bridge, agent DTO move
preparation, G1 contract fixes, island/settings WIP, clipboard image fix WIP) plus working tree.
No code was edited for this review. Independent commands were run where possible; app-process GUI
capture was not re-run for every surface (noted per item).

### 10.1 R1 - agent task DTO move to ps-ipc: PASS

| Check | Result |
|---|---|
| Canonical types in ps-ipc | PASS - crates/ps-ipc/src/agent_task.rs (18,446 B) defines AgentTask, AgentTaskNode, AgentTaskActivity, DetectedAgent, progress/permissions/controls/log/error/restart types; ps-ipc/src/lib.rs exposes the module |
| No duplicate local DTOs | PASS - grep over crates/ps-agent and apps/desktop/src-tauri finds no struct AgentTask/AgentTaskNode/DetectedAgent definitions outside ps-ipc; ps-agent/src/task/model.rs re-exports ps_ipc::agent_task; Tauri imports ps_ipc::agent_task::DetectedAgent (main.rs:15) and task commands return the ps-ipc-backed ps_agent aliases (main.rs:1178-1207) |
| Byte-identical fixtures | PASS - tests agent_task_fixture_is_byte_identical, full_agent_task_fixture_is_byte_identical and detected_agent_fixture_is_stable pin the exact JSON strings (agent_task.rs:331-412,483-500); older optional-field records still parse |
| Independent test run | PASS - cargo test -p ps-ipc: 17 passed, 0 failed (includes the 4 agent-task contract/fixture tests; .agents/notes/g2-rereview-tests.log) |
| TS mirror | PASS per L0 decision - bridge/types.ts remains the documented mirror ("TypeScript mirror of crates/ps-ipc/src/lib.rs (protocol v1). Keep this file in sync...") and is not machine-checked. Policy requires a schema check at G8; task-44 marked complete. |
| Residue | None blocking R1. Risk: manual TS mirror drift; tracked by docs/adr/IPC_ADDITIVE_VERSIONING_POLICY.md. |

### 10.2 R2 - preview command/bounds/cache + bridge: PASS (backend/contract); UI acceptance pending task-38

| Check | Result |
|---|---|
| Tauri command | PASS - get_preview registered in the invoke_handler (main.rs:1166-1172; registration main.rs:1392); argument is ps_ipc::PreviewRequest |
| resultId-only / no raw paths | PASS - PreviewRequest contains only resultId + optional maxChars (ps-ipc:872-878); the engine resolves the id through its result registry and returns an honest Unsupported payload for unknown ids (engine.rs:667-711); the UI never passes filesystem paths |
| Payload fields | PASS - PreviewPayload carries status (ok/unsupported/error), truncated, error, imageMime, unsupportedReason, text/snippet/highlights/imageDataUrl/ocrText/fields (ps-ipc:880-934); producer sanitizes/truncates text |
| Bounds/cache | PASS - maxChars clamped in engine.preview (engine.rs:672-711); PreviewCache is a bounded LRU-like cache keyed by resultId+maxChars (crates/ps-core/src/engine/preview.rs:63-110); image data URLs come from the bounded clipboard thumbnail path; cache test preview_cache_is_bounded_and_reused passes |
| Engine tests | PASS - cargo test -p ps-core: 48 passed, 0 failed, including 8 preview tests (text/structured formats, scanned PDF/binary/unsupported honesty, image thumbnail + indexed OCR text, unknown id, cache bounds/truncation, clipboard text/folder honesty) |
| Bridge | PASS - api.getPreview(resultId, maxChars?) typed in bridge/api.ts:64,86; ResultPreview renders imageDataUrl (validated against an image-data URL pattern), unsupported/error states and snippet/text; launcher tests prime a real PreviewPayload (launcherSurface.test.tsx preview pane suite) |
| UI final acceptance | PENDING - task-38 (G2-PREVIEW UI) is still in_progress; the task-32 preview screenshot shows the pre-fix fallback and must not be used as evidence against the new pipeline. Fresh native screenshots (light/dark, 100/125/150/200% DPI, regular+narrow) and both-launcher parity are G2/G8 acceptance items. |
| App-level preview E2E | NOT INDEPENDENTLY RE-RUN in this pass (engine/command/bridge verified); G0-A fixture corpus and the engine tests cover the payload path. |

### 10.3 R3 - clipboard image P0, real-Windows matrix, bounds/gate/paste-back: PASS (independent reproduction)

| Check | Result |
|---|---|
| Independent tests | PASS - cargo test -p ps-windows: 7 unit + 11 integration = 18 passed; cargo test -p ps-sources: 18 unit + 31 integration (1 pre-existing ignored action-smoke) = 49 passed; all in .agents/notes/g2-rereview-tests.log |
| Format coverage tests | PASS - decoder tests: BI_BITFIELDS DIB, DIBV5 BI_BITFIELDS, 24/32 bpp, palette, top-down, malformed/oversized rejection, paste-payload round-trip; ps-sources tests: image stored as PNG, dedupe, categories, thumbnail-within-IPC-contract, oversized dimensions rejected, capture options vs master gate, restart/paste-back, retention, delete cleanup |
| Independent real .NET SetImage | PASS - rebuilt the audit harness from the current tree (docs/qa/clipboard-audit/harness; build exit 0), started the production listener (db_audit --listen-external) and copied a real System.Windows.Forms.Clipboard.SetImage 320x200 bitmap from a separate STA PowerShell process: listener count 1 -> 2, new row category=Image, PNG 6,315 B, thumbnail 3,671 B (128x80), format=dibv5, imageWidth/Height=320x200. These bytes match the BOOT evidence exactly, independently corroborating docs/qa/clipboard-audit/BOOT_P0_REPAIR_EVIDENCE.md |
| Paste-back | PASS - db_audit --paste-latest wrote the 320x200 PNG back to the clipboard (format dibv5 row; 6,315 bytes) |
| Bounds / master gate | PASS (code + tests + BOOT matrix) - master gate privacy.clipboard_history AND !clipboard.pause; captureText/captureImages are format toggles applied inside the gate; thumbnails <=128 px and <=128 KiB, source dims <=8192, MIME image/png allowlist; UI receives bounded thumbnailDataUrl/dims/mime only |
| Caveats | The synthetic scoped matrix through the listener showed ordering/baseline artifacts for some scenarios (e.g. first scenario has a pre-existing seeded row); the direct capture_current path, unit/integration tests and the real .NET external capture all pass, so the synthetic artifacts are test-harness sequencing, not production defects. Full app-process clipboard GUI E2E (interactive Snipping Tool/Paint/browser copy) remains a G2 acceptance item per BOOT "Remaining / not yet verified" (OLE-only sources and image OCR text also future work). |

Independent evidence files: docs/qa/clipboard-audit/INDEPENDENT_REREVIEW_R3_2026-11-10.md;
logs .agents/notes/clipboard-rereview-matrix.log and clipboard-rereview-external.log;
DB/PNGs under docs/qa/clipboard-audit/evidence/runtime/rereview-external/.

### 10.4 UI spec review (island renderer + normal launcher v2)

| Surface | Verdict | Findings / spec-level regressions |
|---|---|---|
| Dynamic Island renderer | PARTIAL - not signable at G5 | Geometry/type/dot/motion tokens match ISLAND_V9_SPEC (islandCardDims table, expandedSearchHeight 185+53*min(n,5), 550/500/450/220 ms motion, reduced-motion instant; island tests). Screenshots are Chromium mock, not native Windows: expanded cards only captured at 1150/520/335 (860/430 missing), and native DPI/taskbar/fullscreen/High Contrast/frame pacing are unmeasured (VISUAL_AUDIT_ISLAND gaps 5-6). Card completeness still below spec: Pinned disabled (spec requires pinning), clipboard Images rows do not render the now-available thumbnailDataUrl, Agent card does not use the now-available agent_tasks/agent_task_tree APIs (audit gap 2/3 is stale after task-44/R3), Media/Controls/Notifications disabled pending task-35 (honest, but still spec-incomplete). No fake data - good. |
| Normal Launcher v2 | CONDITIONAL | Implementation matches Lead G4 addendum C1/C2 (4 chips ALL/FILES/FOLDERS/APPS, quick destinations Browser Bookmarks/Browser History/Clipboard History/Agents + Git history/Agent History, honest off states). However the binding design docs still specify 7 chips (SCREENS section 2.11: ALL/FILES/FOLDERS/APPS/CLIPBOARD/WEB/ACTIONS) and a recents-only empty state (SCREENS section 2.4); the task-32 addendum explicitly says "frozen until design-spec's Normal Launcher micro-spec lands". Therefore this is a documented pending design-spec sync, not an unrecorded code regression - G4 sign-off requires the micro-spec update (4 chips + quick destinations + prefix reachability) and a revision of SCREENS 2.6 preview alpha text after R2. Evidence is browser mock only; native Windows screenshots at 100/125/150/200% DPI, dark/light and 560 narrow are still required. Row anatomy/groups/footer maths match the screenshots inspected. |

Screens inspected: island compact-1150 (520x66, 8 dots), island search-1150 (550x397, filters+4 results+foot+dots), island clipboard-1150 (510x391, 7 tabs, Pinned disabled), launcher task32-1180-results-dark (4 chips, grouped rows, footer), launcher task32-1180-preview-dark (pre-fix fallback panel - superseded by R2), task40-launcher-dark (Preferences/launcher mode page).

### 10.5 Reviewer sign-off table (G1/G2/G4/G5 status as of 2026-11-10)

| Area | Reviewer | Verdict | Evidence | Conditions / blockers | Gate |
|---|---|---|---|---|---|
| R1 agent DTO move | L1-E (rel-recon) | PASS | canonical agent_task.rs, re-exports, no duplicates, byte-identical fixtures, ps-ipc 17/17 | bridge mirror stays documented/manual; add automated TS check by G8 | G1 |
| R2 preview backend/contract | L1-E | PASS (backend/contract) | get_preview registered, resultId-only, bounds/cache/status, ps-core 48/48, bridge getPreview + ResultPreview | task-38 UI final screenshots/E2E; both-launcher parity at G8 | G2 |
| R3 clipboard images | L1-E | PASS (module + real-Windows harness) | ps-windows 18/18, ps-sources 49 (1 ignored), independent .NET SetImage -> row/PNG/thumb, paste-back | full app GUI matrix (Snipping/Paint/browser) and OLE-only/OCR text follow-ups | G2 |
| G1 overall | L1-E | **CLOSED (recommend)** | R1 PASS, R2 backend PASS, R3 PASS, ADR/policy done, shell fixes verified | preview UI and app-level clipboard E2E belong to G2 acceptance | G1 |
| G2 P0-1 previews | L1-E | CONDITIONAL | backend/command/bridge PASS; engine tests PASS | task-38 in progress; fresh native screenshots + both launchers | G2 |
| G2 P0-2 clipboard | L1-E | CONDITIONAL PASS | independent module-level + real .NET reproduction; BOOT evidence | app-process GUI matrix; OLE fallback; image OCR text | G2 |
| G4 normal launcher v2 | L1-E | CONDITIONAL | 4 chips + quick destinations + row/footer anatomy match Lead addendum | design-spec micro-spec sync (7->4 chips, quick destinations, SCREENS 2.6 preview update); native DPI screenshots | G4 |
| G5 island renderer | L1-E | PARTIAL - NOT SIGNED | geometry/motion/dots match spec; mock screenshots; honest disabled cards | thumbnails, pinning, agent hierarchy wiring, task-35 adapters, native DPI/frame pacing, expanded 860/430 captures | G5 |

### 10.6 Remaining blockers after this re-review

1. G2-P0-1: task-38 preview UI final acceptance (fresh native screenshots, both launcher presentations, stale/cancel states) - open.
2. G2-P0-2: full app-process clipboard GUI E2E (interactive Snipping Tool/Paint/browser) and OLE-only sources/image OCR text - documented follow-ups; module-level and real .NET harness evidence pass.
3. G5: wire clipboard image thumbnails and the new agent_tasks APIs into the island cards; implement Pinning or formally cut it; task-35 native adapters for Media/Controls/Notifications; native Windows DPI/frame-pacing evidence; expanded-card captures at 860/430.
4. G4: design-spec micro-spec update to match the Lead-directed 4-chip + quick-destinations model, and SCREENS 2.6 preview text update after R2; native Windows DPI/dark/narrow screenshots.
5. G1 policy: bridge/types.ts mirror remains manual; add an automated Rust<->TS schema check at G8.
6. Environment/other gates unchanged: single monitor (dual/mixed-DPI blocked), no signing certificate, no non-Pi provider credentials, content p95 <30 ms (task-42) remains the top perf gap.
