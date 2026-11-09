# ProtonSearch agent sidecar (protocol v1)

Optional, on-demand Python agent process for ProtonSearch v2 (Gate 6). It is
**never started for search** and never imported by the Rust search path: the
desktop app only spawns it when the user runs an agent workflow, and kills it
after an idle timeout. The Rust lifecycle client lives in `crates/ps-agent`.

## What it is

* A real [Pydantic AI](https://pydantic.dev/docs/ai/) agent (v2.66.0): typed
  tools, a real model loop, deterministic test model.
* Speaks the frozen JSONL protocol from `crates/ps-ipc/src/lib.rs`:
  reads `AgentRunRequest` / cancel / `AgentApprovalResponse` lines, emits
  `AgentEvent` lines (`started`, `plan`, `progress`, `approval_request`,
  `tool_result`, `output`, `error`, `finished`).
* **Safe by construction**: it never touches the OS itself. Its only tools
  (`search_files`, `read_file_preview`, `open_path`, `run_native_command`)
  emit a structured `approval_request` (an `ActionSpec` from ps-ipc) and wait
  for the user's approval; the Rust host executes. No shell, no `subprocess`,
  no filesystem reads, no network in the sidecar itself.
* No credentials are bundled. A model is configured by the user; keys come
  from the environment or the Windows Credential Manager (read-only helper).

See `docs/agent/AGENT_RUNTIME_EVALUATION.md` (why Pydantic AI),
`docs/agent/INTEGRATION.md` (Rust/Tauri wiring and protocol details) and
`docs/agent/SECURITY.md` (threat model).

## Install (measured on this host)

Native CPython 3.14.7 is required; **do not use the MSYS2 python on `PATH`**
(it cannot create venvs). The exact interpreter used here:

```powershell
C:/Users/Pranshul/AppData/Local/Programs/Python/Python314/python.exe --version
# Python 3.14.7
```

```powershell
cd C:/Users/Pranshul/Documents/Projects/protonsearch
C:/Users/Pranshul/AppData/Local/Programs/Python/Python314/python.exe -m venv agent-sidecar/.venv
agent-sidecar/.venv/Scripts/python.exe -m pip install -r agent-sidecar/requirements.txt
```

Pinned dependency: `pydantic-ai==2.66.0`. Measured footprint on this host:
229.0 MB venv (213.7 MB site-packages), import cost 1.83–2.62 s (see the
evaluation doc). `requirements.lock.txt` is a machine-generated freeze of the
exact working set for reproducibility.

`.venv`, `.venv-nanobot` and friends are gitignored; they are machine-local.

## Run

The host normally starts it; by hand:

```powershell
cd agent-sidecar
$env:PROTONSEARCH_AGENT_TEST_MODEL = "1"   # deterministic, no key, no network
.venv/Scripts/python.exe -m protonsearch_agent
```

Then write JSON lines to stdin. Example session (approve both tool calls):

```json
{"runId":"demo-1","prompt":"find the quarterly budget file","contextPaths":["C:/Users/Public/budget.txt"]}
{"type":"approval_response","runId":"demo-1","approvalId":"apr-...","approved":true}
```

Stdout is protocol JSONL only; diagnostics go to stderr (the Rust client drains
and redacts stderr). `python -m protonsearch_agent --describe` prints a JSON
capability record and exits (no protocol session needed).

Exit paths: EOF on stdin, SIGINT/SIGTERM (POSIX; see caveat below), or the
host killing the process. Exit code 0 on a clean EOF.

## Model configuration (product mode)

```text
PROTONSEARCH_AGENT_MODEL        pydantic-ai model id, e.g. "openai:gpt-4o-mini"
PROTONSEARCH_AGENT_BASE_URL     OpenAI-compatible base URL (maps to OPENAI_BASE_URL)
PROTONSEARCH_AGENT_API_KEY      API key (alternative: OS Credential Manager target
                                "protonsearch/agent-api-key"; env wins)
PROTONSEARCH_AGENT_CONFIG       path to a JSON override file (default
                                %APPDATA%/protonsearch/v2/agent.json)
PROTONSEARCH_AGENT_MAX_STEPS    tool-approval budget per run (1..32, default 8)
PROTONSEARCH_AGENT_RUN_TIMEOUT_S  hard run timeout (default 300)
```

The JSON config file may carry `model`, `baseUrl`, `testModel`, `maxSteps`,
`runTimeoutS` but **never** an API key. Provider keys for other providers are
read by pydantic-ai from its own environment variables (`ANTHROPIC_API_KEY`,
...). No provider call was made during the spike (no credentials), so real
provider round-trips are **UNVERIFIED**; the model string/provider plumbing is
straight from the framework and the test-model path is fully verified.

## Deterministic test mode (supported CI path)

`PROTONSEARCH_AGENT_TEST_MODEL=1` selects a scripted pydantic-ai
`FunctionModel` that needs no key and makes no network call:

1. `search_files` approval → `read_file_preview` approval (when
   `contextPaths` is non-empty) → short final answer.
2. Same code path as product mode: same agent loop, typed tools, approval
   events, cancel handling and event stream.

It exists so Gate 6 E2E/CI can verify streaming, cancel and approvals
deterministically. It is **not** the product default and is labelled
`test:scripted` in the `started` event.

## Tests

```powershell
cd agent-sidecar
.venv/Scripts/python.exe -m unittest discover -s tests -v
```

* `tests/test_protocol.py` — wire-shape unit tests (stdlib unittest).
* `tests/test_scripted_sidecar.py` — spawns the real process in test-model
  mode: approval flow, denied approval, cancel-while-waiting, unknown message
  resilience, `--describe`, clean exit. Skips with a clear message when the
  venv is absent (never fakes a pass).

Measured on this host (command:
`agent-sidecar/.venv/Scripts/python.exe agent-sidecar/scripts/measure_sidecar.py`,
3 runs, test-model mode): startup spawn→first event **2045.3 ms median**
(1544.1–2152.1 ms); one full run request→finished **2105.9 ms median**; idle
window 2.0 s → **15.6 ms CPU** (max of runs) and **9.11 MB working set** after
a completed run. The process is not spawned at all unless a workflow runs, so
idle baseline cost for search is zero.

## Limitations / honest notes

* **Protocol v1 cannot return host *data* to the agent loop.** The frozen
  `AgentEvent` set has no capability-result message, so approved tools report
  "approved; delegated to the host" and the final answer cannot cite actual
  search hits or file previews. A v2 message (host → sidecar capability
  result, plus a ps-ipc bump) is proposed in `docs/agent/INTEGRATION.md`.
* Token-by-token model output streaming is not implemented in v1; events are
  streamed incrementally, and `output` is emitted once when the answer is
  final. Test mode exercises exactly this path.
* SIGTERM is only delivered as a catchable signal on POSIX. On Windows,
  `TerminateProcess` does not run Python handlers, so the Rust host's graceful
  path is "close stdin → EOF", with a kill fallback. This is documented and
  verified from the host side.
* Windows Credential Manager *read* path is implemented via `ctypes` but a
  successful read of a provisioned credential is UNVERIFIED on this host (no
  credential was created).
