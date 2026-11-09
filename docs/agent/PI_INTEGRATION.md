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

Module layout:
`crates/ps-agent/src/pi/{mod,probe,args,env,events,client,process,redact}.rs`;
offline test helper `crates/ps-agent/src/bin/ps_agent_fake_pi.rs` (never used by
the app, must not be shipped by the installer).

**Security hardening status (rel-recon PI-SEC review, 2026-11-10):** PI-SEC-01
(MCP off), PI-SEC-02 (user extensions/skills/prompt templates off), PI-SEC-03
(context files off), PI-SEC-04 (Output redaction + broadened shapes), PI-SEC-05
(minimal child env), PI-SEC-06 (offline/telemetry/version-check off), PI-SEC-07
(absolute launcher validation + source), PI-SEC-08 (cached/capped probe) and
PI-SEC-09 (pid-reuse narrowing) are implemented in this build. PI-SEC-10 (Tauri
wiring) remains L0's task; the UI must still disclose PI-SEC-11 (no sandbox, no
per-tool approval, writes opt-in) and the provider-data flow.

## 3. Exact commands

Canonical invocation built by `build_pi_run_args`:

```text
pi --mode json --no-session --no-approve
   --no-mcp --exclude-tools mcp__*
   --no-context-files --no-extensions --no-skills --no-prompt-templates
   --tools read,grep,find,ls
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
  prompts are not loaded.
- Security hardening defaults (rel-recon PI-SEC-01/02/03), present for **both**
  read-only and write runs:
  - `--no-mcp` **and** `--exclude-tools mcp__*` — MCP tools are not declared
    even when the user has user-level MCP servers configured (a plain `--tools`
    allowlist alone keeps them);
  - `--no-context-files` — `AGENTS.md`/`CLAUDE.md` are not loaded into the
    system context (they load regardless of project trust otherwise);
  - `--no-extensions --no-skills --no-prompt-templates` — user-level Pi
    resources execute code in-process, so none are loaded by default. There is
    **no opt-in** in this build; enabling any of them would be trusted-code
    execution requiring explicit consent and a UI warning.
- The Windows launcher is a batch file, so the client wraps it as
  `cmd.exe /d /s /c ""<pi.cmd>" "<arg>" ..."`. Arguments are quoted exactly
  once and rejected if they contain `" % ! ^ & | < >`, CR/LF or control
  characters. The application prompt never goes through this path.
- **Minimal child environment** (PI-SEC-05): the inherited user environment is
  cleared and only an explicit allowlist is forwarded:
  - OS essentials Node/Pi need: `PATH`, `SystemRoot`, `windir`, `SystemDrive`,
    `COMSPEC`, `PATHEXT`, `TEMP`/`TMP`, `USERPROFILE`, `USERNAME`,
    `HOMEDRIVE`/`HOMEPATH`, `APPDATA`, `LOCALAPPDATA`, `PROGRAMFILES`,
    `ProgramFiles(x86)`, `PROGRAMDATA`, `ALLUSERSPROFILE`, `PUBLIC`, plus the
    documented proxy variables;
  - provider/auth variables documented by Pi 1.5.0 (`OPENAI_API_KEY`,
    `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `XAI_API_KEY`, `DEEPSEEK_API_KEY`,
    `GROQ_API_KEY`, `OPENROUTER_API_KEY`, `HF_TOKEN`, Azure/AWS/Bedrock/
    Copilot/Cloudflare/... — full list in `PROVIDER_ENV_VARS`), forwarded only
    when present;
  - non-secret Pi location config `PI_CODING_AGENT_DIR`/`PI_PACKAGE_DIR`;
  - the `PI_*` controls ProtonSearch sets (below);
  - explicit `PiAgentConfig.extra_env` entries (trusted app injection, e.g. a
    provider base URL). Values are never logged and are added to redaction.
  Arbitrary secrets such as `AWS_SECRET_ACCESS_KEY`, `GITHUB_TOKEN`, CI tokens
  and `PROTONSEARCH_*` variables are **not** inherited. Probes use the same
  allowlist **without** provider credentials. Limitation: arbitrary `PI_*`
  variables from the parent are not inherited; use `extra_env` for those.
- Startup network controls (PI-SEC-06; docs correction): the child gets
  `PI_OFFLINE=1`, `PI_SKIP_VERSION_CHECK=1` and `PI_TELEMETRY=0`, including for
  `pi --version`/`node --version` probes. Per Pi 1.5.0 docs, `PI_OFFLINE`
  disables **automatic** network activity (model catalog refresh, version
  check, telemetry/attribution) while **model inference still works**. The
  previous version of this document incorrectly claimed `--offline` would block
  inference. Live inference with these variables set is **UNVERIFIED** on this
  host (no provider credentials); this is based on the installed Pi docs.
- Working directory: `PiAgentConfig.workspace`, default
  `<data_dir>/agent-workspace` (created automatically).
- Context paths from `AgentRunRequest.contextPaths` are appended to the prompt
  as an explicitly labelled "UNTRUSTED DATA" list; the model can read them with
  the read-only tools. PI-SEC-14 caps: max 32 context paths, 64 KiB total prompt
  (truncated with a marker). `maxSteps` has no Pi equivalent and is currently
  ignored (documented limitation).

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
pub enum PiSource { Explicit, Env, Managed, Path }
pub struct PiInfo { pub path: PathBuf, pub version: String, pub node_ok: bool, pub source: PiSource }

// Absolute + existing + launcher-named "pi" required; missing explicit path -> None.
pub fn locate_pi(explicit: Option<&Path>) -> Option<PathBuf>;
pub fn locate_pi_with_source(explicit: Option<&Path>) -> Option<(PathBuf, PiSource)>;
pub fn probe_pi(explicit: Option<&Path>) -> Option<PiInfo>;        // uncached, spawns pi --version (5 s)
pub fn probe_pi_cached(explicit: Option<&Path>) -> Option<PiInfo>; // cache for agent_status polls
pub fn clear_probe_cache();
pub fn pi_version_supported(version: &str) -> bool;              // minimum 1.5.0
pub fn node_version_ok() -> bool;
pub const MIN_PI_VERSION: (u32, u32, u32);
pub const MIN_NODE_VERSION: (u32, u32, u32);
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
    // Cached probe (PI-SEC-08); rejects Pi older than MIN_PI_VERSION (PI-SEC-15).
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
`~/.pi/agent/bin/pi.cmd` -> `pi` on PATH. Every candidate must be an
**absolute, existing file named `pi`** (with a .cmd/.exe/.bat extension on
Windows); `PiInfo.source` tells the UI which rule matched, and the resolved
path should be displayed (PI-SEC-07). The PATH fallback remains the least
trusted source and can be shadowed by a user-writable directory — prefer
`settings.pi_path`/the managed install for high assurance.

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
  extensions, MCP servers and skills are not loaded.
- **MCP is off** (PI-SEC-01, HIGH): the default args include `--no-mcp` **and**
  `--exclude-tools mcp__*`, so user-level MCP servers cannot add tools (including
  network/file capabilities) to a "read-only" run. There is no MCP opt-in yet.
- **User-level Pi code is off** (PI-SEC-02): `--no-extensions --no-skills
  --no-prompt-templates` mean user-installed extensions/skills/prompt templates
  never execute inside a ProtonSearch run. Extensions execute JS with the user's
  permissions, so enabling them later must be an explicit, warned, consent-gated
  opt-in (none exists in this build).
- **Context files are off** (PI-SEC-03): `--no-context-files` stops
  `AGENTS.override.md`/`AGENTS.md`/`CLAUDE.md` from being loaded into the system
  context even though they bypass project trust. Keep the workspace dedicated
  (the default `<data_dir>/agent-workspace` is empty and app-managed); treat
  workspace content as untrusted data.
- **Prompt injection.** Retrieved file content and tool output are data, never
  instructions; the system-prompt preamble says so. That reduces but does not
  eliminate injection risk; with writes enabled a successful injection can
  edit files/run commands. Container isolation is the recommended mitigation.
- **Secrets.** The child gets a minimal environment, not the user's whole
  environment (PI-SEC-05): unrelated credentials such as `AWS_SECRET_ACCESS_KEY`,
  `GITHUB_TOKEN`, CI tokens and `PROTONSEARCH_*` values are not inherited.
  Redaction (PI-SEC-04) covers stderr, tool-result previews, errors **and the
  final `Output` text**; it replaces exact known credential values and broad
  shapes (`sk-`/`pk-`, Google `AIza`, Groq `gsk_`, HuggingFace `hf_`, GitHub
  `ghp_/gho_/ghu_/ghs_/ghr_`, AWS `AKIA`, Slack `xox[bpas]-`, Bearer/Basic
  tokens and `api_key=…`/`token: …` assignment forms). Raw stderr is never
  emitted. Pi provider auth lives in the user's Pi install / provider env; it is
  forwarded only as the documented provider variables and is never read or
  printed by ProtonSearch.
- **No shell-string interpolation.** The prompt is delivered on stdin; launcher
  arguments are validated and quoted once for the batch wrapper; `taskkill`
  receives only a numeric pid.
- **Launcher validation** (PI-SEC-07): explicit/env/PATH candidates must be
  absolute, existing files named `pi`; `PiInfo.source` exposes the resolution
  source for UI display. A malicious same-user executable renamed to `pi.exe`
  on PATH / a tampered `settings.pi_path` is still the user's own execution
  surface — prefer the managed install for high assurance.
- **Probe hardening** (PI-SEC-08): probe results are cached per launcher
  (`probe_pi_cached`), probe output capture is capped at 64 KiB, and probes run
  with the credential-free minimal environment. `clear_probe_cache()` after a Pi
  update.
- **PID-reuse guard** (PI-SEC-09): tree termination marks the run and re-checks
  `Child::try_wait` before `taskkill`, so an already-exited child's pid is not
  signalled. A Windows Job Object with `KILL_ON_JOB_CLOSE` is the complete fix
  and is a documented follow-up (it would also cover an app crash orphan).
- **Network** (PI-SEC-06): the child runs with `PI_OFFLINE=1`,
  `PI_SKIP_VERSION_CHECK=1` and `PI_TELEMETRY=0`, disabling Pi's automatic
  catalog/version/telemetry network activity while the configured model provider
  inference still works (per Pi docs; live inference UNVERIFIED here — no
  credentials). ProtonSearch adds no telemetry of its own; the provider call is
  the feature and only happens when the user runs a workflow.

## 7. Cancellation and limits

| Limit | Default | Behavior |
|---|---|---|
| Idle timeout | 120 s without stdout events | mark `idle_timeout`, kill tree, `Error` + `Finished{ok:false}` |
| Max run duration | 15 min | mark `run_timeout`, kill tree, `Error` + `Finished{ok:false}` |
| Cancellation | user/tray stop | `taskkill /PID <pid> /T` (graceful), then `/T /F` after 2 s; whole tree (cmd -> node -> bash children) |
| Crash | process exits without `agent_settled` | `Error { code: "pi_exit_error" }` + `Finished{ok:false}` |
| Line cap | 4 MiB per JSONL line | `protocol_line_too_long`, tree killed |
| Concurrency | one run per client | second `send_run` returns `AgentError::Busy` |
| Prompt caps | 32 context paths / 64 KiB total (PI-SEC-14) | truncated with a marker before stdin write |
| Probe | 5 s timeout, 64 KiB capture cap, cached (PI-SEC-08) | `probe_pi`/`probe_pi_cached`; no model call, no credentials |
| Startup | no persistent process | each run spawns Pi fresh (Node startup ~1-2 s, not measured as a budget here) |

## 8. Tests (offline; run on this host 2026-11-09)

`cargo test -p ps-agent` (after the PI-SEC hardening) → **51 unit + 16 Pi
integration + 4 legacy sidecar integration = 71 passed, 0 failed** (Pi
integration 2.89 s). The legacy Pydantic sidecar tests still pass unchanged.

The Pi E2E tests use `ps_agent_fake_pi` (scripted JSONL emitter) through a
generated `.cmd` launcher in a directory whose name contains a space, so the
batch wrapper, minimal child environment, stdin prompt delivery, event mapping,
redaction, busy gate, cancellation and the process-tree kill are all exercised
without a model. Security-specific coverage:

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
- **argv hardening**: every spawn carries `--no-mcp`, `--exclude-tools mcp__*`,
  `--no-context-files`, `--no-extensions`, `--no-skills`,
  `--no-prompt-templates` (read-only and writes runs), plus
  `--tools read,grep,find,ls`, `--model`, `--provider`, `--system-prompt`
  and the writes allowlist;
- **minimal env (PI-SEC-05)**: the fake Pi records its environment; the test
  asserts every inherited variable is on the allowlist, `FAKE_SECRET` is
  absent, and `PI_OFFLINE=1`/`PI_SKIP_VERSION_CHECK=1`/`PI_TELEMETRY=0` are
  set;
- **final Output redaction (PI-SEC-04)**: a planted provider secret delivered
  via `extra_env` plus an AIza key and `api_key=…` value in the final answer
  are all replaced with `[redacted]`, and the planted secret never appears on
  the command line;
- **probe (PI-SEC-07/08)**: relative/non-`pi` paths are rejected, the resolved
  source is reported, and cached probes are stable;
- fake probe returns `1.5.0-fake`; the real probe asserted
  `version=1.5.0 path=C:\Users\Pranshul\.pi\agent\bin\pi.cmd node_ok=true`
  (`source=Managed`).

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
7. **Residual security risks after PI-SEC-01..09** (rel-recon follow-ups):
   - no Windows Job Object: an app crash can still leave Pi running until it
     exits or is killed externally (PI-SEC-09 residual; Job Object with
     `KILL_ON_JOB_CLOSE` is the planned fix; pid-reuse is narrowed but not
     eliminated between the liveness check and `taskkill`);
   - the PATH fallback for Pi remains the least trusted resolution source
     (absolute + named `pi` validated, resolved source shown; prefer managed
     install or explicit `settings.pi_path`);
   - user-level Pi *config* (`~/.pi/agent`) is not loaded by extensions/skills
     anymore, but Pi itself may still read/write its own caches/models store
     outside the v2 data dir (PI-SEC-12 residual);
   - live model inference with `PI_OFFLINE=1` was not verified on this host
     (no provider credentials), only documented behaviour;
   - no OS sandbox and no per-tool approval (PI-SEC-11): container/VM/dedicated
     user is recommended for high assurance; the UI must disclose this.
8. The `ps_agent_fake_pi` binary is a test/support target only and must not be
   shipped by the G7 installer (PI-SEC-13).
9. `AgentSettings` still has no `workspace`/`thinking` field; workspace defaults
   to `<data_dir>/agent-workspace` and thinking to
   `PROTONSEARCH_AGENT_THINKING` (see item 1). A custom `PI_CODING_AGENT_DIR` /
   `PI_CODING_AGENT_SESSION_DIR` cannot be inherited through the parent env;
   pass it via `PiAgentConfig.extra_env` if needed (documented minimal-env
   limitation).
