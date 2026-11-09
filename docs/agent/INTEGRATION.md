# Agent sidecar integration contract (for the Tauri shell)

- Date: 2026-11-09
- Owner: L2-A4 (agent sidecar); wiring owner: L0 (apps/desktop/src-tauri is
  lead-owned; this document is the contract, no shell files were edited here)
- Consumers: `apps/desktop/src-tauri/src/main.rs` (Rust) and the Workflows UI
- Runtime crate: `crates/ps-agent`; process: `agent-sidecar` (Python)

The sidecar is **optional and on-demand**: never create it in `setup()`, never
touch it on the search path. Search works with the Python runtime absent.

## 1. Crate dependency

Add to `apps/desktop/src-tauri/Cargo.toml` (L0):

```toml
ps-agent = { path = "../../../crates/ps-agent" }
```

## 2. Rust API surface (crates/ps-agent)

```rust
pub use ps_agent::{
    default_config, runtime_available, runtime_available_for, venv_python,
    AgentClient, AgentApprovalResponse, AgentEvent, AgentError, AgentRunRequest,
    EventCallback, SidecarConfig, DEFAULT_IDLE_TIMEOUT, MAX_LINE_BYTES, MAX_RESTARTS,
};

// Probe only: file existence of the venv python + protonsearch_agent/__main__.py.
// Never spawns, never imports pydantic-ai. Use for AgentStatus / settings UI.
pub fn default_config(data_dir: &Path) -> Option<SidecarConfig>;
pub fn runtime_available(python: &Path, script: &Path) -> bool;

impl AgentClient {
    // Spawns: CREATE_NO_WINDOW, piped stdin/stdout/stderr, stderr drain thread,
    // stdout JSONL reader thread, idle watchdog thread. Fails with
    // AgentError::NotAvailable when the runtime is missing (no system-python
    // fallback, ever).
    pub fn spawn<F>(config: SidecarConfig, on_event: F) -> Result<Self, AgentError>
    where F: Fn(&AgentEvent) + Send + Sync + 'static;

    // Convenience for tests/queues; callback equivalent.
    pub fn spawn_channel(config: SidecarConfig)
        -> Result<(AgentClient, std::sync::mpsc::Receiver<AgentEvent>), AgentError>;

    pub fn send_run(&self, request: &AgentRunRequest) -> Result<(), AgentError>;
    pub fn approve(&self, response: &AgentApprovalResponse) -> Result<(), AgentError>;
    pub fn cancel(&self, run_id: Option<&str>) -> Result<(), AgentError>;     // None = active run
    pub fn is_alive(&self) -> bool;
    pub fn pid(&self) -> Option<u32>;
    pub fn restart_count(&self) -> u32;                                        // 0 or 1
    pub fn stderr_tail(&self, max_lines: usize) -> Vec<String>;                // redacted
    pub fn shutdown(&self);                                                    // also on Drop
}

pub const MAX_RESTARTS: u32 = 1;
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
```

Semantics the shell can rely on:

* **Threading:** `send_run`/`approve`/`cancel` are short writes; `shutdown`
  waits up to 2 s for an EOF exit then kills. The event callback runs on the
  client's stdout reader thread — forward it to the WebView with
  `app.emit(...)` (Tauri is thread-safe), never block or call back into the
  client from inside the callback.
* **One run at a time.** A second `send_run` while a run is active is queued
  by the sidecar (one deep); a third gets an `error` event with code `busy`.
* **Idle timeout:** after `idle_timeout` (default 120 s) with no active run the
  watchdog closes stdin and kills the child. The client is then dead; the next
  `agent_start` must spawn a fresh client (`is_alive()` is false).
* **Crash policy:** an unexpected exit is reported as `error`
  (`sidecar_crashed`, plus `finished{ok:false}` if a run was active) and the
  client restarts **at most once** (`sidecar_restarted` on success,
  `sidecar_unavailable` on failure). After a crash the in-flight run is dead;
  the UI should offer Retry.
* **Shutdown:** call `agent_stop()` / `client.shutdown()` from `quit_app`
  and when a workflow window closes (the `Drop` impl is a backstop).
* **Capability probe:** `default_config(data_dir)` is the whole probe; map
  `None` to `AgentStatus { state: NotConfigured, .. }` without spawning.

## 3. Suggested Tauri wiring (L0 edits main.rs)

```rust
use ps_agent::{AgentClient, AgentApprovalResponse, AgentEvent, AgentRunRequest};
use std::sync::Mutex;

struct AppState {
    engine: Arc<Engine>,
    data_dir: PathBuf,
    agent: Mutex<Option<AgentClient>>, // ADD
}

#[tauri::command]
fn agent_status(state: tauri::State<'_, AppState>) -> Option<ps_ipc::AgentStatus> {
    let config = ps_agent::default_config(&state.data_dir)?;
    Some(ps_ipc::AgentStatus {
        state: ps_ipc::AgentState::Stopped, // Running is derived from the UI run state
        runtime: Some("pydantic-ai 2.66.0".into()),
        model: config.model.clone(),
        detail: Some("optional Python sidecar; starts only when a workflow runs".into()),
    })
}

#[tauri::command]
fn agent_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: AgentRunRequest,
) -> Result<(), String> {
    // Respect the opt-in setting before spawning anything.
    if !state.engine.settings().agent.enabled {
        return Err("agent workflows are disabled in settings".into());
    }
    let mut slot = state.agent.lock().map_err(|e| e.to_string())?;
    if slot.as_ref().map(|client| !client.is_alive()).unwrap_or(true) {
        let config = ps_agent::default_config(&state.data_dir)
            .ok_or("agent sidecar runtime is not available on this machine")?;
        let handle = app.clone();
        let client = AgentClient::spawn(config, move |event: &AgentEvent| {
            // AgentEvent fields are camelCase on the wire (ps-ipc, §5).
            let _ = handle.emit("agent:event", event);
        })
        .map_err(|e| e.to_string())?;
        *slot = Some(client);
    }
    slot.as_ref()
        .expect("client was just installed")
        .send_run(&request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn agent_cancel(state: tauri::State<'_, AppState>, run_id: String) -> Result<(), String> {
    match state.agent.lock().map_err(|e| e.to_string())?.as_ref() {
        Some(client) => client.cancel(Some(&run_id)).map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

#[tauri::command]
fn agent_approve(
    state: tauri::State<'_, AppState>,
    response: AgentApprovalResponse,
) -> Result<(), String> {
    match state.agent.lock().map_err(|e| e.to_string())?.as_ref() {
        Some(client) => client.approve(&response).map_err(|e| e.to_string()),
        None => Err("agent sidecar is not running".into()),
    }
}

#[tauri::command]
fn agent_stop(state: tauri::State<'_, AppState>) {
    if let Ok(mut slot) = state.agent.lock() {
        if let Some(client) = slot.take() {
            client.shutdown();
        }
    }
}
```

Register the four commands in `invoke_handler` and add `mutex` state; call
`agent_stop` from `quit_app` (and optionally from the workflow view close).
Do not call any of this from `setup()` or from the search command.

## 4. Environment variables (read once at spawn)

| Variable | Meaning | Default |
|---|---|---|
| `PROTONSEARCH_AGENT_SIDECAR_DIR` | override sidecar directory | packaged/dev paths |
| `PROTONSEARCH_AGENT_PYTHON` | override interpreter | `<dir>/.venv/Scripts/python.exe` |
| `PROTONSEARCH_AGENT_MODEL` | pydantic-ai model id, e.g. `openai:gpt-4o-mini` | none (not configured) |
| `PROTONSEARCH_AGENT_BASE_URL` | OpenAI-compatible base URL | provider default |
| `PROTONSEARCH_AGENT_API_KEY` | key override (otherwise OS Credential Manager) | none |
| `PROTONSEARCH_AGENT_TEST_MODEL` | `1` = deterministic scripted model, no key/network | off |
| `PROTONSEARCH_AGENT_IDLE_TIMEOUT_MS` | idle kill after N ms | 120000 |
| `PROTONSEARCH_AGENT_MAX_STEPS` | per-run tool-approval budget | 8 |
| `PROTONSEARCH_AGENT_RUN_TIMEOUT_S` | hard run timeout seconds | 300 |
| `PROTONSEARCH_AGENT_CONFIG` | JSON config path (model/baseUrl/testModel/...) | `%APPDATA%/protonsearch/v2/agent.json` |

The shell should pass model/base URL via `SidecarConfig.model` /
`SidecarConfig.extra_env` (e.g. `("PROTONSEARCH_AGENT_BASE_URL", url)`) rather
than mutating the app's process env. Keys must never be logged.

## 5. Wire format the Workflows UI receives

`AgentEvent` is emitted as `emit("agent:event", event)`; serde (frozen
ps-ipc) produces an externally tagged object. **Resolved 2026-11-09:** L0 added
`rename_all_fields = "camelCase"` to `AgentEvent`, so inner fields are
camelCase (`runId`, `approvalId`, `durationMs`, `finalAnswer`), matching the TS
mirror in `apps/desktop/src/bridge/types.ts`. Envelope keys stay as before:
`type`, `steps`, `message`/`fraction`, `tool`/`ok`/`preview`, `code`. The
sidecar emits exactly this shape and `crates/ps-agent/src/protocol.rs` parses
it (unit tests + Rust E2E cover the shape).

```json
{"type":"started","runId":"run-1","runtime":"pydantic-ai 2.66.0","model":"test:scripted"}
{"type":"plan","steps":["Interpret the request","Use approved host calls to search or preview what is needed","Report what the host approved and what remains"]}
{"type":"progress","message":"waiting for approval: search_files","fraction":null}
{"type":"approval_request","approvalId":"apr-...","title":"Search your files","detail":"Search the local index for 'budget' (limit 5). Read-only.","action":{"type":"none"},"destructive":false}
{"type":"tool_result","tool":"search_files","ok":true,"preview":"search_files(query='budget', limit=5) approved; delegated to the host"}
{"type":"output","text":"...","finalAnswer":true}
{"type":"error","code":"cancelled","message":"run cancelled by the host"}
{"type":"finished","ok":false,"durationMs":61}
```

UI rendering guidance:

| Event | Workflows UI treatment |
|---|---|
| `started` | open the run panel; show runtime + model; Start disabled until `finished` |
| `plan` | checklist skeleton (steps are advisory, not live status) |
| `progress` | inline status line; `fraction` null means indeterminate |
| `approval_request` | modal with title/detail/destructive; Approve/Deny calls `agent_approve`. The `action` is structured (`ActionSpec`); never render it as a shell command |
| `tool_result` | per-tool row, ok/denied badge, preview text (already user-safe) |
| `output` | assistant message; `finalAnswer:true` ends the stream |
| `error` | red banner; codes `cancelled`, `timeout`, `not_configured`, `busy`, `protocol_error`, `sidecar_crashed`, `sidecar_restarted`, `sidecar_unavailable`, `run_failed` |
| `finished` | close the run; `ok:false` keeps the error banner, offer Retry |

## 6. Known v1 limitation and proposed extension (DEFERRED by L0, post-alpha)

The frozen `AgentEvent` set has **no channel for host results to reach the
agent loop** (search hits, file preview text). In v1 an approved tool emits
`tool_result{ok:true, preview:"... approved; delegated to the host"}` and the
final answer cannot cite actual content. This is a contract limitation, not
hidden: the sidecar never claims it saw the data.

Proposed v2 (backwards-compatible for the UI; needs a ps-ipc review +
`PROTOCOL_VERSION` bump):

```text
host -> sidecar (new inbound control message, not an AgentEvent):
{"type":"capability_result","runId":"run-1","approvalId":"apr-...","ok":true,"data":{"hits":[...]}}
```

* Rust host executes the approved capability (search/read) in `agent_approve`
  or a dedicated command, then writes `capability_result` on stdin.
* Sidecar resolves the awaiting tool future with `data`, returns it to the
  model, and emits a `tool_result` preview derived from the data.
* ps-agent API grows an optional resolver callback; UI event schema unchanged.

## 7. Test evidence (run on this host)

```powershell
cargo test -p ps-agent
# unit: 10 passed (framing/parse/redaction/probe); integration: 4 passed
# (approval flow, cancel, idle-timeout kill, crash+one restart) in 3.11 s
agent-sidecar/.venv/Scripts/python.exe -m unittest discover -s agent-sidecar/tests
# 18 tests passed in 20.077 s (protocol shapes + real subprocess E2E)
```

The Rust E2E tests skip with a printed SKIP message when
`agent-sidecar/.venv` is absent; they never fake a pass.
