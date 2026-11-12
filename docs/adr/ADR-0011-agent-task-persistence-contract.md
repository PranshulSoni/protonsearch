# ADR-0011: Agent task persistence and contract

- Status: **Accepted with conditions** (G1 post-review L0 decision, 2026-11-10)
- Owners: L0 (decision), L1-D (runtime/task service), L1-A (IPC types), L1-E (security/privacy)
- References: ADR-0006 (IPC), docs/security/PI_AGENT_SECURITY.md (PI-SEC re-review), task-44

## Context

v10 adds a runtime-agnostic agent task model and history so the island Agents card and Workflows UI
can show real Pi/Pydantic runs with honest state, permissions and controls. G1 found the model lives
in crates/ps-agent/src/task and Tauri returns it directly; bridge/types.ts hand-mirrors it. The task
store persists summaries/logs under the v2 data dir, and the shell opened it with an empty
redaction list. L0 decided to move the DTOs to ps-ipc (task-44) and keep DetectedAgent with them.

## Decision

1. Types: AgentTask, AgentTaskState, AgentTaskRuntime, progress, permissions, controls, log entry,
   error and AgentTaskNode, plus DetectedAgent, become ps_ipc types (task-44). Until that lands,
   ps-agent-owned types are a documented temporary mirror; bridge/types.ts is a documented mirror of
   ps-ipc and must be checked when the move completes. No new shell-local DTOs may be added.
2. Persistence: <data_dir>/agents/tasks.json, versioned JSON, atomic temp+rename, corrupt files moved
   aside, bounded task/log retention (200 tasks, 100 log entries, bounded strings), non-terminal
   tasks recovered as Offline after restart, oldest terminal tasks pruned (ps-agent/src/task/store.rs).
3. Privacy: prompt text, model output and tool payloads are never persisted; only redacted title/
   summary/logs/error text. Redaction inputs must include known provider credential values from the
   environment (shell collects OPENAI/ANTHROPIC/GEMINI/GOOGLE/OPENROUTER/DEEPSEEK/AZURE/XAI/GROQ keys
   when >=8 chars; main.rs:1303-1322). Tests must include planted-secret cases.
4. Lifecycle: one runtime run maps 1:1 to a task id; events update the task; terminal states are
   completed/failed/cancelled/offline. Controls are honest: pause/resume are unsupported for Pi and
   Pydantic and the service returns an Unsupported error instead of pretending (service.rs:162-173).
   Cancel semantics must be per-task (current Pi cancel cancels all non-terminal tasks; fix tracked).
5. Security: the alpha gate keeps write-enabled agent runs closed: the shell forces allowWrites=false
   for Pi regardless of persisted settings until a server-side per-run consent token exists
   (main.rs:1060-1075; PI_AGENT_SECURITY RR-02). No per-tool approvals exist for Pi; the UI must say so.
6. Retention/crash behavior and schema changes are governed by the additive versioning policy; a
   breaking task-store change needs its own schema bump and migration test.
7. Host process model stays: one on-demand Pi process per run, no idle daemon, tree-kill cancellation,
   bounded timeouts; the task store is metadata only, never the execution mechanism.

## Consequences

- The Agents UI can be built against a stable, tested contract; runtime implementations stay swappable.
- Persisted task history is privacy-sensitive but bounded and redacted; no prompts/model output.
- Moving the types to ps-ipc is a breaking-enough refactor internally but additive on the wire; TS
  mirrors are checked rather than silently drifting.
- Write-enabled agents remain disabled in alpha; read-only agent workflows can proceed.

## Alternatives considered

- Keep DTOs in ps-agent (rejected by L0: violates single source of truth and TS drift).
- Persist full transcripts/model output (rejected: privacy and size risk).
- Expose pause/resume by freezing child processes (rejected: unreliable on Windows, false semantics).

## Verification

- ps-agent store/service tests (retention, crash/offline recovery, redaction, controls unsupported).
- Shell wiring: main.rs:1050-1200 (task commands), 1303-1322 (redactions); task store paths/schema.
- task-44: types moved/re-exported from ps-ipc, bridge/types.ts updated and checked.
- G7A: planted-secret redaction tests, lifecycle/crash tests, UI honesty review; L1-E security sign-off.
