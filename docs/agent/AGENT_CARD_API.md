# Agent task/card API (V9 Agents card + desktop Agents view)

- Date: 2026-11-10
- Owner: L1-D (agent-sidecar / Agent Runtime & Extensibility)
- Tasks: task-34 (backend/persistence/service) and task-44 (DTO move to
  ps-ipc). Persistence/service/helpers live in `crates/ps-agent/src/task/**`;
  **wire DTOs live in `crates/ps-ipc/src/agent_task.rs` (V10 single source of
  truth)**. Tauri shell and UI are Lead/UI-owned and were not edited.
- Status: backend + persistence + tests implemented; **accepted by L0
  (2026-11-10), DTOs moved to ps-ipc, and wired into the Tauri shell.** See the
  decision record below.

This document is the contract for the V9 Agents card (launcher) and the desktop
Agents view. It defines the runtime-agnostic task record, its persistence,
lifecycle, honest-progress rules, permissions, cancellation semantics and the
accepted Rust/Tauri API.

## 0. Decision record (L0, 2026-11-10)

1. **~~Types stay in `crates/ps-agent/src/task`~~ — superseded by decision 6**
   (task-44 moved the DTOs into ps-ipc; ps-agent re-exports them and the
   `PROTOCOL_VERSION` is still unbumped because the JSON shapes are unchanged).
2. **Commands/events accepted** with the names/signatures in §8:
   `agent_tasks`, `agent_task_tree`, `agent_task`, `agent_permissions`,
   `agent_cancel(run_id?)`, `agent_stop_all`; `agent_pause`/`agent_resume`
   return Unsupported. `agent:task` snapshots are emitted alongside the
   unchanged `agent:event` transcript.
3. **Shell wiring landed (L0):** `AgentTaskService` in `AppState` with
   `app.emit("agent:task")`; `agent_start` begins the Pi task before spawn and
   uses the canonical `task.id` as `run_id` so events map 1:1; cancel marks the
   stored active task then kills; quit calls `stop_all`; pause/resume are exposed
   but honestly unsupported.
4. **Activity/search write deferred**: the engine activity API is internal for
   now. The persisted tasks file remains the source of truth and survives
   restarts; §8's activity projection is documented as a follow-up
   (engine-core/L0), not implemented in this phase.
5. **UI contract**: the Agents card uses `controls.canPause`/
   `controls.canResume` and hides disabled controls.
6. **DTO single source of truth (task-44, 2026-11-10):** `AgentTask`,
   `AgentTaskNode`, `AgentPermissionState`, progress/log/restart/controls/
   activity types and `DetectedAgent` now live in
   `crates/ps-ipc/src/agent_task.rs`. ps-agent re-exports the same nominal types
   (`ps_agent::AgentTask` == `ps_ipc::agent_task::AgentTask`), so its public API
   and the shell's existing imports keep working. JSON shapes are byte-identical
   — pinned by frozen pre-move serialization fixtures in ps-ipc contract tests.
   `apps/desktop/src/bridge/types.ts` is the TypeScript mirror of ps-ipc (not of
   ps-agent) and must be kept in sync when DTOs change. No
   `PROTOCOL_VERSION`/`DATA_SCHEMA_VERSION` bump (wire and persisted shapes
   unchanged).

## 1. Goals and non-goals

Goals: real process/task state with persisted IDs and history; parent/child ids
where the runtime exposes them; statuses queued/running/completed/failed/
cancelled/offline; redacted log ring; permission state; launch/cancel semantics
that match what the runtime can actually do; crash isolation; on-demand start;
history that the existing Activity/search can index.

Non-goals: model output storage (only the redacted title/summary is kept);
fake progress percentages; fake sub-agent trees for runtimes without sub-agents;
pause/resume (neither current runtime supports it); a second transcript (the
live event stream via `agent:event` remains the transcript source).

## 2. Architecture

```text
Tauri agent_start
  └─ AgentTaskService::begin_pi(request, PiInfo, model, allow_writes, parent?)
       └─ AgentTaskStore.begin_task  → task {id, state: queued, permissions...}
  └─ PiAgentClient::spawn(...); send_run(request)
       └─ every AgentEvent (started/progress/tool_result/error/finished/...)
            ├─ app.emit("agent:event", event)          (live transcript, existing)
            └─ AgentTaskService::apply_event(task.id, event)
                 └─ state machine + redacted log + persistence
                 └─ listener(task) → app.emit("agent:task", task)
agent_cancel(id) → AgentTaskService::cancel(id) (marks Cancelled) + PiAgentClient::cancel()
agent_stop_all() → AgentTaskService::stop_all() (marks all) + kill each active client
```

The service is runtime-agnostic: it consumes the frozen ps-ipc `AgentEvent`
stream, so the same backend serves Pi (default) and the optional Pydantic sidecar
fallback. Runtime-specific knowledge is limited to initial permissions and the
runtime label.

## 3. Data model

The DTOs themselves live in `ps_ipc::agent_task` (single source of truth);
ps-agent re-exports them and owns persistence policy. Persisted JSON file:
`<data_dir>/agents/tasks.json` (v2 data dir;
`%APPDATA%/protonsearch/v2/agents/tasks.json` on Windows), schema:

```json
{
  "schemaVersion": 1,
  "tasks": [
    {
      "id": "run-2026-11-10-001",
      "runtime": "pi",
      "runtimeVersion": "1.5.0",
      "parentId": null,
      "title": "Find the quarterly budget file",
      "promptSummary": "find the quarterly budget file",
      "state": "running",
      "progress": { "kind": "indeterminate", "fraction": null },
      "createdAt": 1762700000000,
      "startedAt": 1762700000123,
      "endedAt": null,
      "updatedAt": 1762700000456,
      "log": [
        { "at": 1762700000000, "level": "info", "message": "queued (pi runtime, read-only)" },
        { "at": 1762700000123, "level": "info", "message": "started pi 1.5.0" },
        { "at": 1762700000400, "level": "info", "message": "tool: read" },
        { "at": 1762700000456, "level": "info", "message": "tool read ok" }
      ],
      "error": null,
      "permissions": {
        "requested": ["read", "grep", "find", "ls"],
        "granted": ["read", "grep", "find", "ls"],
        "denied": [],
        "pending": null,
        "readOnly": true
      },
      "writeAccess": false,
      "controls": { "canCancel": true, "canPause": false, "canResume": false, "canRetry": false },
      "restarts": [],
      "model": "openai/gpt-5"
    }
  ]
}
```

Field rules:

- `id`: stable task id; equals the run id. If a previous task used the same run
  id, the store appends a numeric suffix — **always use the id returned by
  `begin`**, not the request's run id, for later calls.
- `runtime`: `pi` | `pydantic` | `unknown`. Filled from the `started` event
  label ("pi 1.5.0" / "pydantic-ai 2.66.0") when the launch did not know it.
- `runtimeVersion`: parsed from the started label (or the probed Pi version).
- `parentId`: parent task id or null. **Pi exposes no sub-agents, so Pi tasks
  are always flat (null).** The field + tree API exist for future runtimes; the
  UI must not fabricate a tree.
- `title` / `promptSummary`: redacted, length-capped (80 / 200 chars) summaries
  of the prompt. The raw prompt is never persisted.
- `state`: `queued` | `running` | `completed` | `failed` | `cancelled` |
  `offline`.
- `progress`: `kind` is `determinate` only when the runtime emitted a real
  fraction; Pi never does, so Pi tasks are `indeterminate` with `fraction:null`.
  Terminal states reset to indeterminate.
- `createdAt/startedAt/endedAt/updatedAt`: Unix milliseconds.
- `log`: bounded ring (100 entries, 300 chars each). Only status lines are
  stored (started/finished/tool name + ok/permission/recovery/errors). Assistant
  text, tool previews/payloads and plan steps are **not** stored.
- `error`: `{ code, message }`, message redacted and capped at 400 chars.
- `permissions`: see §6.
- `writeAccess`: true when edit/write/bash/powershell were enabled for the run.
- `controls`: what the UI may offer in this state. Pause/resume are always
  false (unsupported by both runtimes).
- `restarts`: restart history observed from the runtime (currently only the
  Pydantic sidecar reports automatic restarts; Pi crashes fail the task).
- `model`: model id when known.

## 4. Lifecycle and event mapping

```text
begin ──► queued ──(started)──► running ──(finished ok)────────► completed
                │                   ├──(finished !ok, last error cancelled)──► cancelled
                │                   ├──(finished !ok / error)────────────────► failed
                │                   └──(host cancel / stop_all)──────────────► cancelled
                └──(spawn failure)──► failed
                └──(runtime unavailable)──► offline
open() recovery: queued/running persisted by a previous app process ──► offline
```

Event mapping (`AgentTaskService::apply_event`):

| Frozen AgentEvent | Task change |
|---|---|
| `started { runtime, model }` | state → running, startedAt set, runtime/version/model parsed, log "started <runtime>" |
| `progress { message, fraction }` | fraction ⇒ determinate progress; status lines (`tool: …`, `provider retry …`, `compacting context`, `waiting for approval: …`) appended to log. Assistant text deltas are ignored and never persisted |
| `approval_request { title, … }` | permission requested/pending set (fallback runtime); log warn |
| `tool_result { tool, ok, … }` | pending permission resolved granted/denied; log "tool <name> ok/failed" (no preview) |
| `output { final_answer: true, … }` | log "final answer ready"; text not stored |
| `error { code, message }` | error stored (redacted); `sidecar_restarted`/`sidecar_crashed` add a restart record; log error |
| `finished { ok, duration_ms }` | completed / failed / cancelled (cancelled only when the last error code is `cancelled`), endedAt set, log |
| `plan { … }` | ignored (model content; not persisted) |
| events after a terminal state | ignored (host cancellation is authoritative) |

## 5. Honest semantics

- **Progress**: determinate only with a real runtime fraction. Pi has none, so a
  Pi card shows an indeterminate indicator, never a fake percentage or ETA.
- **Parent/child**: Pi is single-level. `parentId` is null and the tree API
  returns flat roots. Do not render expandable children for Pi tasks.
- **Pause/resume**: unsupported by Pi and the Pydantic sidecar. The service
  returns `AgentError::Unsupported` (Tauri response:
  `{ supported: false, reason }`) and controls always disable both buttons.
- **Retry**: not a runtime control; a retry is a **new** task with a new id (the
  UI may prefill the previous prompt). `canRetry` is true for failed/cancelled/
  offline tasks.
- **Cancellation**: the host marks the task cancelled first (idempotent), then
  kills the runtime process tree. Later runtime events cannot flip the state
  back. If the task already finished, cancel is a no-op returning null.
- **Crash isolation**: a runtime crash ends the task as failed (Pi has no
  automatic restart); a Pydantic sidecar restart is recorded in `restarts` and
  the run continues/fails per its events. An app restart turns queued/running
  tasks offline with error code `app_restarted`.
- **Permissions**: Pi is run-level — the user pre-authorised a tool allowlist in
  settings, so `requested == granted` at launch (`read_only` reflects the
  mode). The Pydantic fallback asks per action; `approval_request` populates
  requested/pending and the following tool result resolves granted/denied.

## 6. Persistence, privacy and limits

- Path: `<data_dir>/agents/tasks.json`, `schemaVersion: 1`, camelCase fields.
- Atomic replace (write temp file, replace, rename); corrupt files are moved to
  `*.json.corrupt-<nanos>` instead of being silently overwritten.
- History cap: newest 200 tasks (oldest terminal tasks pruned first).
- Log ring: 100 entries × 300 chars per task.
- Never persisted: raw prompt, assistant/model output, tool previews/payloads,
  plan steps, context-path contents. Only redacted summary/title, status logs,
  error messages, permissions and timestamps.
- Redaction reuses the Pi runtime redaction (exact known credential values plus
  sk-/pk-/AIza/gsk_/hf_/ghp_/AKIA/xox*/Bearer/Basic/api_key= shapes). The shell
  passes its known-secret list into `AgentTaskService::open`.
- The store is plain JSON in the per-user v2 data dir (same protection as
  settings/index metadata; no extra encryption). No network access.
- On open, stale queued/running tasks are recovered as offline (crash
  isolation) so the card never shows phantom running work.

## 7. Rust API (crates/ps-agent/src/task)

The DTO types used below are defined in `ps_ipc::agent_task` and re-exported by
`ps_agent` (`ps_agent::AgentTask` etc. resolve to the ps-ipc types), so existing
shell code and imports keep working; new code may import either path.

```rust
pub type AgentTaskListener = Arc<dyn Fn(&AgentTask) + Send + Sync + 'static>;

pub struct AgentTaskService;
impl AgentTaskService {
    pub fn open(data_dir: &Path, redactions: Vec<String>) -> Result<Self, AgentError>;
    pub fn with_listener(self, listener: AgentTaskListener) -> Self;

    // launch adapters
    pub fn begin(&self, request: &AgentRunRequest, launch: &AgentTaskLaunch) -> Result<AgentTask, AgentError>;
    pub fn begin_pi(&self, request: &AgentRunRequest, info: &PiInfo, model: Option<String>,
                    allow_writes: bool, parent_id: Option<String>) -> Result<AgentTask, AgentError>;
    pub fn begin_pydantic(&self, request: &AgentRunRequest, model: Option<String>,
                          allow_writes: bool) -> Result<AgentTask, AgentError>;

    // lifecycle feedback from the runtime event stream
    pub fn apply_event(&self, task_id: &str, event: &AgentEvent) -> Option<AgentTask>;
    pub fn mark_launch_failed(&self, task_id: &str, code: &str, message: &str) -> Option<AgentTask>;
    pub fn mark_offline(&self, task_id: &str, message: &str) -> Option<AgentTask>;
    pub fn cancel(&self, task_id: &str, reason: &str) -> Option<AgentTask>; // mark first, then kill
    pub fn stop_all(&self, reason: &str) -> Vec<AgentTask>;                 // mark all, then kill each

    // queries for the card/view
    pub fn tasks(&self, limit: usize) -> Vec<AgentTask>;      // newest first, 0 = all (capped 200)
    pub fn tree(&self, limit: usize) -> Vec<AgentTaskNode>;
    pub fn task(&self, id: &str) -> Option<AgentTask>;
    pub fn permissions(&self, id: &str) -> Option<AgentPermissionState>;
    pub fn activity(&self, limit: usize) -> Vec<AgentTaskActivity>; // activity/search projection
    pub fn pause(&self, _id: &str) -> Result<(), AgentError>;       // Unsupported
    pub fn resume(&self, _id: &str) -> Result<(), AgentError>;      // Unsupported
    pub fn last_persist_error(&self) -> Option<String>;
}
```

Wire-up sketch for the shell (L0):

```rust
let service = Arc::new(AgentTaskService::open(&data_dir, redactions)?
    .with_listener({ let app = app.clone(); Arc::new(move |task: &AgentTask| {
        let _ = app.emit("agent:task", task);
    }) }));

// agent_start:
let task = service.begin_pi(&request, &pi_info, model, settings.allow_writes, None)?;
let service_cb = service.clone();
let task_id = task.id.clone();
let client = PiAgentClient::spawn(config, move |event: &AgentEvent| {
    let _ = app.emit("agent:event", event);          // existing live transcript
    let _ = service_cb.apply_event(&task_id, event); // task state/log/permissions
})?;
client.send_run(&request)?;

// agent_cancel: service.cancel(id, "user"); client.cancel();
// agent_stop_all / quit: for task in service.stop_all("app quitting") { /* kill matching client */ }
```

## 8. Accepted Tauri commands and events

DTO types for the TS mirror (all camelCase JSON; snake_case enum values) live in
`crates/ps-ipc/src/agent_task.rs` (exported as `ps_ipc::agent_task::*` and
re-exported by ps-agent):

`AgentTask`, `AgentTaskNode`, `AgentTaskRuntime`, `AgentTaskState`,
`AgentTaskProgressKind`, `AgentTaskProgress`, `AgentPermissionState`,
`AgentTaskLogLevel`, `AgentTaskLogEntry`, `AgentTaskErrorInfo`,
`AgentTaskRestart`, `AgentTaskControls`, `AgentTaskActivity`, `DetectedAgent`.
`AgentEvent` / `AgentRunRequest` are unchanged. The shell's `detect_agents`
command should use `ps_ipc::agent_task::DetectedAgent` (the local shell struct was
the duplicated copy; JSON shape is unchanged).

| Tauri command | Signature | Notes |
|---|---|---|
| `agent_tasks` | `(limit: Option<usize>) -> Vec<AgentTask>` | persisted, newest first; limit 0/None = 200 |
| `agent_task_tree` | `(limit: Option<usize>) -> Vec<AgentTaskNode>` | flat roots for Pi |
| `agent_task` | `(id: String) -> Option<AgentTask>` | card/detail refresh |
| `agent_permissions` | `(id: String) -> Option<AgentPermissionState>` | permissions panel |
| `agent_cancel` | `(id: String) -> Result<Option<AgentTask>, String>` | mark cancelled, then kill tree |
| `agent_stop_all` | `() -> Result<Vec<AgentTask>, String>` | app quit / tray stop |
| `agent_pause` | `(id: String) -> Result<(), String>` | always returns unsupported error; UI disables |
| `agent_resume` | `(id: String) -> Result<(), String>` | always returns unsupported error; UI disables |

Existing `agent_start` / `agent_stop` keep their current shape: `agent_start`
must create the task before spawning and pass the canonical `task.id` into the
event callback.

Events (accepted):

| Event | Payload | When |
|---|---|---|
| `agent:task` | one `AgentTask` snapshot | every visible change: queued, running, status log, permissions, progress fraction, terminal, cancel, recovery |
| `agent:event` | frozen `AgentEvent` | unchanged live transcript stream; the card should not duplicate it as stored history |

Activity/search integration (**deferred follow-up**, decision 4): when the
engine activity API is ready, append one activity entry per terminal task with
`kind: Workflow` (existing enum; an `Agent` variant would be additive),
`title: "Agent: <title>"`, `detail: "<runtime> · <state> · <read-only|write
access>"`, and use `AgentTaskActivity::search_text` as indexable text.
`AgentTaskService::activity()` already returns the projection sorted newest
first; the persisted task file remains the source of truth, so history survives
app restarts. The helper is ready but nothing writes to the engine yet.

## 9. V9 Agents card + desktop Agents view contract

Card states:

- **Empty / no runtime configured**: show the empty state and a "Set up agent"
  action; do not show a fake task. `probe_pi` result determines availability
  (runtime absent ⇒ offline state, not an error toast loop).
- **Offline runtime**: card/list shows runtime unavailable + reason; launch is
  disabled. Existing history still renders.
- **Queued**: title + runtime + "Queued"; cancel enabled.
- **Running**: indeterminate progress indicator only; current status line
  (tool/provider retry/compaction) from the log ring; cancel enabled; elapsed
  time from `startedAt` is allowed (it is real), but no percentage/ETA.
- **Completed / Failed / Cancelled**: state badge, duration from
  startedAt/endedAt, redacted error for failures, retry (= new task) for
  failed/cancelled/offline.
- **Offline tasks from app restart**: badge + "app restarted before this task
  finished" error; retry allowed.
- **Log view**: the redacted log ring only; never raw stderr (the existing event
  stream never exposed raw stderr either). Log ordering oldest → newest.
- **Permissions view**: read-only vs write access; for the fallback runtime the
  per-action requested/granted/denied list and any pending approval. Pi
  permissions are the pre-authorised tool allowlist.
- **No-fake-progress rule**: determinate bars only when
  `progress.kind === "determinate"`; otherwise spinner/indeterminate. No
  timers-as-percentages, no simulated stages.
- **Parent/child rule**: only render a tree when at least one task has a
  non-null parentId; Pi tasks render flat even in the desktop tree view.
- **Cancel UX**: optimistic "Cancelling…" after the command, then authoritative
  state from `agent:task`; a finished task ignores cancel.
- **Ordering/pagination**: newest first by createdAt; `agent_tasks(limit)` caps
  at 200; the desktop view can page/filter client-side (state, runtime, text).
- **Pause/resume buttons**: always disabled with a tooltip ("not supported by
  this runtime"); never silently pretend.

## 10. Tests (run on this host 2026-11-10)

`cargo test -p ps-agent` → **75 unit + 5 agent-task E2E + 16 Pi E2E + 4 legacy
sidecar E2E = 100 passed, 0 failed**.
`cargo test -p ps-ipc` → **17 passed**, including the new agent_task contract
tests (frozen pre-move AgentTask fixtures byte-identical, full fixture, missing
optional-field defaults, enum strings, node nesting, DetectedAgent fixture).
`cargo check --workspace` clean (only pre-existing warnings in other crates and
the shell, none from this move).

Agent-task coverage:

- lifecycle state machine: queued → running → completed; failed with redacted
  error; host cancel wins over late runtime events; terminal events ignored;
  controls/canRetry/canCancel per state; duplicate run ids get suffixes;
- persistence/restart: completed task survives store reopen; stale running tasks
  recover as offline (`app_restarted`); persisted file schema (camelCase,
  schemaVersion 1, all fields);
- cancellation semantics: cancel marks cancelled + returns snapshot; process
  kill; late `finished` cannot flip state; stop_all marks all active tasks;
- redaction/privacy: prompt secret, error secret, assistant output and tool
  payloads never appear in `tasks.json`; title/summary/log/error redacted;
  log ring bounded at 100;
- metadata: runtime label/version parsing, determinate progress only with real
  fraction (text deltas never persisted), permission mapping for Pi read-only/
  writes and Pydantic approval flow, restart history, parent/child tree,
  activity projection with lower-case search text;
- E2E through the real Pi client + offline fake Pi: begin → run → events →
  completed task + activity projection + persisted history after reopen.

## 11. Known limitations

1. Pi has no sub-agents over `--mode json`: parent/child is API-ready but Pi
   tasks are flat; do not invent children.
2. No pause/resume in either runtime; controls are permanently disabled.
3. Pi emits no numeric progress: determinate progress is only possible for a
   future runtime that reports fractions.
4. The log ring stores redacted status lines, not raw runtime logs; raw stderr
   remains drained/redacted by the runtime client and is never shown.
5. History is capped at 200 tasks by default (bounded file size); older tasks are
   pruned, not archived.
6. The store is not encrypted (per-user data dir, same as other v2 metadata).
7. Activity/search integration is deferred (decision 4): engine-core must add
   the write path when its activity API is no longer internal. ps-agent already
   exposes `AgentTaskService::activity()` and the projection/search text.
8. The Agents card UI + TS mirrors are pending (UI task): the shell is wired and
   serializes the ps-ipc DTOs directly; `apps/desktop/src/bridge/types.ts` is the
   hand-maintained mirror of `ps_ipc::agent_task` and must be updated whenever
   those DTOs change. The card must use the honest controls and the
   no-fake-progress rules in §9.
9. The shell still owns a local `DetectedAgent` struct at review time (the
   duplicate this move replaces); L0 updates `main.rs` to import
   `ps_ipc::agent_task::DetectedAgent`. Until then the two definitions are
   field-identical and serialize the same JSON.
9. The shell/UI wiring itself is Lead/UI-owned; this backend change did not edit
   Tauri or UI files.
