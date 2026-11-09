# Pi agent runtime integration (v2 default)

- Date: 2026-11-09
- Owner: L2-A4 (agent sidecar); wired into the Tauri shell by L0
- Decision (user): **for agentic purposes drop Hermes and adopt the installed Pi
  coding-agent CLI as the v2 agent runtime.** The embedded Pydantic AI sidecar
  (`agent-sidecar/`, `crates/ps-agent` legacy client) is kept as an optional
  fallback and is not deleted.
- Contract: frozen ps-ipc `AgentRunRequest` / `AgentEvent` (wire format
  unchanged). `AgentSettings` gained `runtime` (default `pi`), `pi_path`,
  `allow_writes` by L0.

## 1. What is installed (verified on this host)

| Fact | Value | How measured |
|---|---|---|
| Package | `@earendil-works/pi-coding-agent` 1.5.0 | package.json under the managed install |
| Launcher | `C:/Users/Pranshul/.pi/agent/bin/pi.cmd` | `Get-Item` |
| Version | **1.5.0** | `cmd /c "C:\Users\Pranshul\.pi\agent\bin\pi.cmd" --version` → `1.5.0` |
| Node.js | **v26.7.0** (Pi requires >= 22.19) | `node --version` |
| Probe result | `version=1.5.0 path=C:\Users\Pranshul\.pi\agent\bin\pi.cmd node_ok=true` | `cargo test -p ps-agent --test pi_e2e real_pi_version_probe_skips_when_absent -- --nocapture` |

The probe runs only `pi --version` (and `node --version` for `node_ok`) with a
5 s timeout and CREATE_NO_WINDOW. It makes no model call, uses no credentials,
does no network I/O and never reads `auth.json`.

## 2. Architecture

```text
Tauri shell (L0)
   agent_start(req) ──► ps_agent::PiAgentClient::send_run
                          ├─ probe (cached PiInfo: path + version + node_ok)
                          ├─ write safety preamble file (<data_dir>/agent/pi-system-prompt.txt)
                          ├─ spawn ONE pi process: cmd.exe /d /s /c "pi.cmd" <args>
                          │     cwd = workspace, stdin/stdout/stderr piped,
                          │     CREATE_NO_WINDOW, environment inherited
                          ├─ write prompt to stdin, close stdin (piped-stdin initial message)
                          ├─ stdout reader thread: JSONL -> ps_ipc::AgentEvent -> app.emit("agent:event")
                          ├─ stderr drain thread: redacted 200-line ring buffer (never forwarded raw)
                          └─ watchdog thread: idle 120 s / max run 15 min -> taskkill tree
   agent_cancel() ──► taskkill /PID <pid> /T (graceful) then /F after 2 s
   agent_stop()   ──► shutdown: kill active tree, refuse new runs
```

Pi is **one-shot per run**: `pi --mode json ...` streams the session and exits.
There is no persistent sidecar process, no idle Python/Node process between
runs, and nothing runs on the search path.

Module layout: `crates/ps-agent/src/pi/{mod,probe,args,events,client,process,redact}.rs`;
offline test helper `crates/ps-agent/src/bin/ps_agent_fake_pi.rs` (never used by the app).

## 3. Exact commands

Canonical invocation built by `build_pi_run_args`:

```text
pi --mode json --no-session --no-approve --tools read,grep,find,ls
   [--provider <name> --model <pattern>] [--thinking <level>]
   [--system-prompt <C:\Users\<user>\AppData\Roaming\protonsearch\v2\agent\pi-system-prompt.txt>]
```

- Writes enabled (`settings.agent.allow_writes == true`):
  `--tools read,grep,find,ls,edit,write,bash,powershell`.
- The user prompt is **not a command-line argument**. It is written to Pi's
  stdin and then stdin is closed. Pi's documented "piped stdin initial
  message" path (implemented in `dist/cli/initial-message.js`) combines it with
  the first CLI message. Reasons: Windows `cmd.exe` batch-launcher quoting, the
  8191-character command-line limit, and keeping untrusted prompt text out of
  the batch parser entirely.
- `--system-prompt <file>` REPLACES Pi's default coding prompt (per Pi docs).
  The generated preamble tells the model that content is untrusted data, that
  it cannot ask questions, that secrets must never be printed, and that this
  run is read-only unless writes were explicitly enabled.
- `--no-approve` declines project trust, so project `.pi/settings.json`,
  project MCP servers, project extensions/skills/prompts and project trust
  prompts are not loaded. User-level Pi config/extensions still load (they are
  the user's own installation; see security section).
- The Windows launcher is a batch file, so the client wraps it as
  `cmd.exe /d /s /c ""<pi.cmd>" "<arg>" ..."`. Arguments are quoted exactly
  once and rejected if they contain `" % ! ^ & | < >`, CR/LF or control
  characters. The application prompt never goes through this path.
- Environment is inherited so the user's Pi provider auth
  (`~/.pi/agent` login or provider env vars) works. The client never logs the
  environment; known credential values are added to the redaction list.
- Working directory: `PiAgentConfig.workspace`, default
  `<data_dir>/agent-workspace` (created automatically).
- Context paths from `AgentRunRequest.contextPaths` are appended to the prompt
  as an explicitly labelled "UNTRUSTED DATA" list; the model can read them with
  the read-only tools. `maxSteps` has no Pi equivalent and is currently ignored
  (documented limitation).

## 4. Event mapping (Pi JSONL -> ps-ipc AgentEvent)

Pi emits strict JSONL on stdout (LF records, 4 MiB cap enforced by the client;
CRLF tolerated). Diagnostics go to stderr. See Pi docs `docs/json.md`.

| Pi event | Emitted AgentEvent | Notes |
|---|---|---|
| `session` (header) | `Started { runId, runtime: "pi <version>", model }` | once per run; model = request/config model or `default` |
| `message_update` with `assistantMessageEvent.type == "text_delta"` | `Progress { message: delta, fraction: null }` | delta text; final answer is authoritative |
| `message_end` (role assistant) | buffered | text collected; final text emitted at exit |
| `tool_execution_start` | `Progress { message: "tool: <name>" }` | no arguments shown (secret safety) |
| `tool_execution_end` | `ToolResult { tool, ok: !isError, preview }` | preview = result text, max 400 chars, redacted |
| `agent_settled` + clean exit 0 | `Output { text, finalAnswer: true }` then `Finished { ok: true, durationMs }` | final assistant text (trimmed) |
| `agent_settled` `aborted: true` | `Error { code: "aborted" }` + `Finished { ok: false }` | |
| auth/provider/assistant error, or nonzero exit | `Error { code, message }` + `Finished { ok: false }` | codes: `auth_error`, `provider_error`, `output_truncated`, `pi_error`, `pi_exit_error` |
| `auto_retry_start` / `auto_retry_end` | `Progress "provider retry n/m: ..."`; final failure becomes `Error` | |
| `compaction_start/end` | `Progress "compacting context"/"context compacted"` | |
| malformed JSON line | `Error { code: "protocol_parse_error" }` | stream continues |
| line > 4 MiB | `Error { code: "protocol_line_too_long" }` + `Finished { ok: false }` | tree killed; framing cannot resync |
| idle 120 s / run > 15 min / cancel | `Error { code: "idle_timeout" / "run_timeout" / "cancelled" }` + `Finished { ok: false }` | process tree killed |
| unknown Pi event | ignored | forward compatibility |

No `approval_request` is ever emitted by the Pi backend: Pi has no per-tool
approval prompt. The Workflows approval UI is only used by the Pydantic
fallback.

UI note: `Progress` carries both streamed assistant text deltas and status
lines. Status lines produced by this backend are exactly `tool: <name>`,
`provider retry ...`, `compacting context` and `context compacted`; anything
else is assistant text and should be appended to the live transcript, then
replaced by the final `Output` event.

## 5. Rust API (crates/ps-agent/src/pi)

```rust
pub struct PiInfo { pub path: PathBuf, pub version: String, pub node_ok: bool }

pub fn locate_pi(explicit: Option<&Path>) -> Option<PathBuf>;
pub fn probe_pi(explicit: Option<&Path>) -> Option<PiInfo>; // spawns pi --version (5 s)
pub fn node_version_ok() -> bool;
pub fn pi_tool_allowlist(allow_writes: bool) -> &'static str;
pub fn pi_safety_preamble(allow_writes: bool) -> String;
pub fn build_pi_run_args(spec: &PiRunArgsSpec) -> Result<Vec<String>, AgentError>;

pub struct PiAgentConfig {
    pub pi: PiInfo, pub workspace: PathBuf, pub data_dir: PathBuf,
    pub allow_writes: bool, pub model: Option<String>, pub provider: Option<String>,
    pub thinking: Option<String>,
    pub idle_timeout: Duration,        // default 120 s
    pub max_run_duration: Duration,    // default 15 min
    pub extra_env: Vec<(String, String)>,
}
impl PiAgentConfig {
    pub fn from_settings(settings: &ps_ipc::AgentSettings, data_dir: &Path) -> Result<Self, AgentError>;
    pub fn with_workspace(self, path: impl Into<PathBuf>) -> Self;
    // with_model / with_thinking / with_allow_writes
}

pub struct PiAgentClient;
impl PiAgentClient {
    pub fn spawn<F: Fn(&AgentEvent) + Send + Sync + 'static>(config, on_event) -> Result<Self, AgentError>;
    pub fn spawn_channel(config) -> Result<(Self, Receiver<AgentEvent>), AgentError>;
    pub fn send_run(&self, request: &AgentRunRequest) -> Result<(), AgentError>; // AgentError::Busy if active
    pub fn cancel(&self) -> Result<(), AgentError>;
    pub fn shutdown(&self);                  // also on Drop
    pub fn is_running(&self) -> bool;
    pub fn pid(&self) -> Option<u32>;
    pub fn run_id(&self) -> Option<String>;
    pub fn stderr_tail(&self, max_lines: usize) -> Vec<String>; // redacted
    pub fn info(&self) -> &PiInfo;
}
```

Suggested Tauri wiring (L0): resolve runtime from `settings.agent.runtime`;
for `AgentRuntime::Pi` use `PiAgentConfig::from_settings` +
`PiAgentClient::spawn` lazily in `agent_start` (never `setup()`/search), forward
the callback with `app.emit("agent:event", event)`, map `agent_cancel` to
`client.cancel()`, `agent_stop`/`quit_app` to `client.shutdown()`, and
`agent_status` to `probe_pi` (no spawn of a run). Keep the Pydantic path for
`AgentRuntime::Pydantic` as documented in `docs/agent/INTEGRATION.md`.

Idle timeout and max run duration can be overridden with
`PROTONSEARCH_AGENT_IDLE_TIMEOUT_MS` and `PROTONSEARCH_AGENT_MAX_RUN_MS`.
Thinking level (not present in ps-ipc settings yet) comes from
`PROTONSEARCH_AGENT_THINKING` or `PiAgentConfig::with_thinking`.
Pi location order: `settings.pi_path` -> `PROTONSEARCH_PI_PATH` ->
`~/.pi/agent/bin/pi.cmd` -> `pi` on PATH.

## 6. Security model (honest)

- **Read-only by default.** `allow_writes = false` (ps-ipc default) restricts
  the model to `read,grep,find,ls`. `edit,write,bash,powershell` appear only
  when the user explicitly opts in via `settings.agent.allow_writes`.
- **No OS sandbox.** Pi's own docs are explicit: it runs with the permissions
  of the account that started it, the working directory controls resource
  discovery but does not prevent tools from reaching other paths, and there is
  no per-tool approval. The allowlist limits *which tools are offered*, not
  what those tools can touch. For real isolation, run ProtonSearch/Pi in a
  container, VM or dedicated user account (Pi docs `containerization.md`).
- **No per-action approvals.** Unlike the Pydantic fallback, the Pi backend
  never emits `approval_request`. The user opts into writes *for the whole run*
  before it starts. A prompt-injected model can therefore use any enabled
  read-only tool on any readable file; with writes enabled it can also modify
  files and run shells, subject only to OS permissions.
- **Project trust is declined** (`--no-approve`): project `.pi` settings,
  extensions, MCP servers and skills are not loaded. Context files
  (`AGENTS.md`, `CLAUDE.md`) still load regardless of project trust — treat
  workspace content as untrusted and keep the workspace dedicated (the default
  `<data_dir>/agent-workspace` is empty and app-managed).
- **Prompt injection.** Retrieved file content and tool output are data, never
  instructions; the system-prompt preamble says so. That reduces but does not
  eliminate injection risk; with writes enabled a successful injection can
  edit files/run commands. Container isolation is the recommended mitigation.
- **Secrets.** The client never logs the environment. Known credential env
  values and `sk-/pk-` key shapes and `Bearer` tokens are redacted from stderr
  and from tool-result previews before they reach the UI; raw stderr is never
  emitted. Pi provider auth lives in the user's Pi install / provider env and is
  never read by ProtonSearch.
- **No shell-string interpolation.** The prompt is delivered on stdin; launcher
  arguments are validated and quoted once for the batch wrapper; `taskkill`
  receives only a numeric pid.
- **Network.** Pi may contact the configured model provider and refresh model
  catalogs; ProtonSearch adds no telemetry and no other network access. Run
  `--offline` is not passed (provider inference needs the network); this is the
  user's configured agent runtime, started only on demand.

## 7. Cancellation and limits

| Limit | Default | Behavior |
|---|---|---|
| Idle timeout | 120 s without stdout events | mark `idle_timeout`, kill tree, `Error` + `Finished{ok:false}` |
| Max run duration | 15 min | mark `run_timeout`, kill tree, `Error` + `Finished{ok:false}` |
| Cancellation | user/tray stop | `taskkill /PID <pid> /T` (graceful), then `/T /F` after 2 s; whole tree (cmd -> node -> bash children) |
| Crash | process exits without `agent_settled` | `Error { code: "pi_exit_error" }` + `Finished{ok:false}` |
| Line cap | 4 MiB per JSONL line | `protocol_line_too_long`, tree killed |
| Concurrency | one run per client | second `send_run` returns `AgentError::Busy` |
| Startup | no persistent process | each run spawns Pi fresh (Node startup ~1-2 s, not measured as a budget here) |

## 8. Tests (offline; run on this host 2026-11-09)

`cargo test -p ps-agent` → **36 unit + 12 Pi integration + 4 legacy sidecar
integration = 52 passed, 0 failed** (Pi integration 2.77 s).

The Pi E2E tests use `ps_agent_fake_pi` (scripted JSONL emitter) through a
generated `.cmd` launcher in a directory whose name contains a space, so the
batch wrapper, stdin prompt delivery, event mapping, redaction, busy gate,
cancellation and the process-tree kill are all exercised without a model:

- happy path: Started(runtime `pi 1.5.0-fake`, model), text-delta Progress,
  `tool: read` Progress, ToolResult ok, final Output + Finished ok;
- direct executable launch (no .cmd wrapper);
- second concurrent run rejected with `Busy`, then cancel → `cancelled` +
  `Finished{ok:false}`;
- `taskkill /T` kills a spawned grandchild within 3 s;
- idle timeout (300 ms in the test) → `idle_timeout`;
- oversized (>4 MiB) line → `protocol_line_too_long`;
- malformed line → `protocol_parse_error` and the stream still finishes;
- stderr + tool preview secret redaction;
- argv assertions: `--mode json --no-session --no-approve --tools read,grep,find,ls`,
  `--model`, `--provider`, `--system-prompt`, and the writes allowlist;
- fake probe returns `1.5.0-fake`; the real probe asserted
  `version=1.5.0 path=C:\Users\Pranshul\.pi\agent\bin\pi.cmd node_ok=true`.

## 9. Known limitations

1. ps-ipc `AgentSettings` currently has no `workspace` or `thinking` field
   (task text mentioned workspace). This build defaults the workspace to
   `<data_dir>/agent-workspace` and accepts `PiAgentConfig::with_workspace`;
   thinking comes from env. Recommend adding both fields to ps-ipc if the UI
   should configure them (L0-owned).
2. No per-tool approval and no OS sandbox (by Pi design): writes are an
   all-or-nothing opt-in; use a container for isolation.
3. `AgentRunRequest.maxSteps` is ignored (Pi has no equivalent); run bounded by
   timeouts instead.
4. No token-level thinking output is forwarded; `message_update` text deltas
   only. Final `Output` is authoritative.
5. `@file` arguments are not used; context paths are passed as labelled text
   so untrusted paths never become CLI arguments.
6. Startup latency of a real Pi run was not benchmarked (requires a model
   credential); tests cover process lifecycle offline.
