# Agent runtime evaluation — Pydantic AI vs Nanobot

- Date: 2026-11-09
- Owner: L2-A4 (agent sidecar); decision wired by L1-A / L0
- Gate: G6 (optional Python agent sidecar)
- Status: **Decision recorded: adopt Pydantic AI 2.66.0.** ADR-0005 itself is
  L0-owned and was not edited; its "Proposed" status should move to "Accepted"
  with the framework noted.

## Candidates

| Candidate | Version evaluated | Source |
|---|---|---|
| Pydantic AI | 2.66.0 (MIT) | installed in `agent-sidecar/.venv` |
| Nanobot | nanobot-ai 0.4.1 (HKUDS/nanobot, MIT) | installed in `agent-sidecar/.venv-nanobot` (gitignored) |

Both were installed and measured on this host: Windows, CPython **3.14.7**
(native install, `C:/Users/Pranshul/AppData/Local/Programs/Python/Python314/python.exe`).
The MSYS2 python on `PATH` cannot create venvs and was not used.

## Measured facts

All commands below were run from the repo root on 2026-11-09. Numbers vary
between runs on this noisy host; ranges and medians are stated rather than a
single cherry-picked value.

### Disk footprint

Command: `(Get-ChildItem <venv> -Recurse -File | Measure-Object Length -Sum).Sum / 1MB`

| Metric | Pydantic AI 2.66.0 | Nanobot 0.4.1 |
|---|---|---|
| venv size | **229.0 MB** | **400.6 MB** |
| site-packages size | 213.7 MB | 397.0 MB |
| top-level site-packages entries | 194 | 233 |
| Python files in the package | (framework) | 464 |
| pip install time | not measured by L2-A4 (lead installed it) — **UNVERIFIED** | **210 517 ms** for `pip install nanobot-ai==0.4.1` after venv creation |

### Import / startup latency

`scripts/measure_import.py <module>` wraps `time.perf_counter()` around the
import in a **fresh process**, 5 runs per package (so it includes interpreter
startup once outside the measured section); interpreter-only baseline
`python -c pass`: median **133.4 ms** (range 123.1–229.6 ms, 5 runs).

| Import | Median | Range |
|---|---|---|
| `import pydantic_ai` | **2916.2 ms** | 1588.6–7033.5 ms |
| spawn+import wall time `python -c "import pydantic_ai"` | **2413.5 ms** | 1792.3–2758.3 ms |
| `import nanobot` (empty package `__init__`) | **157.7 ms** | 151.5–208.9 ms |
| `import nanobot.nanobot` (actual runtime: Nanobot/AgentLoop/LLMRuntime/StreamEvent) | **2396.1 ms** | 1352.9–4032.5 ms |

Note: `import nanobot` being fast is misleading; the assistant runtime is
`nanobot.nanobot`, and the extra latency is the honest number to compare.
The lead's earlier spike measured pydantic-ai in-process import at 1832.9 ms;
that is consistent with the lower end of the range above.

### Real sidecar startup (the number that matters for G6)

Command: `agent-sidecar/.venv/Scripts/python.exe agent-sidecar/scripts/measure_sidecar.py`
(Pydantic AI sidecar in deterministic test-model mode, 3 runs,
spawn → first stdout event, including interpreter + framework import):

- startup median **2045.3 ms**, range **1544.1–2152.1 ms**
- one run request → `finished`: **2105.9 ms** median
- idle window 2.0 s after the run: **15.6 ms child CPU** (max of runs),
  **9.11 MB working set** — i.e. a blocked stdin read, no polling
- 3/3 runs completed with the full event sequence
  started → plan → progress → approval_request → tool_result → output → finished
- the process is **not spawned at all** unless a workflow runs, so baseline
  search cost is zero; see `crates/ps-agent` (`runtime_available` never spawns).

### Windows compatibility

| Check | Pydantic AI | Nanobot |
|---|---|---|
| Installs from PyPI on Windows x64 / CPython 3.14 | yes | yes (210 s, 400.6 MB) |
| Import works, no compiler needed | yes | yes (`nanobot.nanobot`) |
| CLI usable on a fresh profile | n/a (library) | **no**: `python -m nanobot --help` raised `FileNotFoundError: C:\Users\Pranshul\.nanobot\updates` after 221–312 ms (it expects its own `~/.nanobot` install/config directory). A one-shot embedding would inherit that setup model. |
| Extra OS-specific deps | pure-python wheels | pywin32, cryptography, sigstore, openpyxl/python-pptx, tiktoken, ... |

### Feature matrix (what was actually verified)

| Criterion | Pydantic AI 2.66.0 | Nanobot 0.4.1 |
|---|---|---|
| Typed tools / function calling | **verified**: probe ran a real tool-call loop (agent called `search_files(query, limit)`, framework validated args, dispatched, looped) | present (`nanobot.agent.tools`, `ToolRegistry` classes inspected) — semantics **UNVERIFIED** |
| MCP support | **verified importable**: `pydantic_ai.mcp` imports, `mcp 2.3.0` installed | present: `mcp` dist installed, `MCPProvider` exported by `nanobot.nanobot` — contract **UNVERIFIED** |
| Provider configuration | model id string + standard provider env (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, base URL); no provider call made here → real round-trip **UNVERIFIED** | own `Config` (pydantic-settings); providers for openai/anthropic/azure/bedrock/github-copilot/xai/openai-compatible verified by module listing — configuration contract **UNVERIFIED** |
| Deterministic test model, no key/network | **verified**: `FunctionModel` scripted agent runs the same loop in CI; used by both Python and Rust E2E tests (all green) | **no test model found**: grep of the installed package for TestModel/FakeModel/MockModel found no agent test double; `unconfigured_provider.py` exists. A no-key agent run is **not supported/UNVERIFIED** |
| Streaming | `Agent.run_stream` / `StreamedRunResult` present; our v1 sidecar streams **events** and emits one final output (token deltas deferred) | `RunStream`/`StreamEvent` classes present — contract **UNVERIFIED** |
| Permission/approval hooks | public `ApprovalRequired`, `ApprovalRequiredToolset`, `ToolApproved`, `ToolDenied` (inspected); we implemented approvals through the frozen ps-ipc events instead | `agent/execution_policy.py`, `goal_permission.py` modules exist — semantics **UNVERIFIED** |
| Embeddability as a one-shot subprocess | **high**: library-first, `Agent(...).run()` in an event loop; no config directory, no daemon | **low**: an assistant application (channels/, gateway/, webui/, desktop TUI, cron, session/memory, sigstore release trust) — 464 py files; embedding imports the full runtime and its user config |
| License | MIT (`pip show pydantic-ai`, License-Expression) | MIT (`pip show nanobot-ai` + classifier) |
| Python requirement | >=3.11 | >=3.11 |

## Decision

**Use Pydantic AI 2.66.0 for the ProtonSearch agent sidecar.** Nanobot is not
adopted. Reasons, in order of weight:

1. **CI/E2E without keys is a hard requirement for Gate 6** ("deterministic test
   mode"). Pydantic AI's `FunctionModel` makes the real agent loop (typed
   tools, approvals, event stream) testable with no network; Nanobot exposes no
   such test double, so its integration could only be exercised with a real
   provider credential (which the brief forbids bundling).
2. **Embedding model.** Pydantic AI is a library; Nanobot is an always-on
   personal-assistant application (gateway/channels/webui/cron/sessions) with
   its own `~/.nanobot` setup. Importing its runtime is 2.4 s median and it
   drags 397 MB of application features into a process that should do one
   request and die.
3. **Footprint.** 229.0 MB vs 400.6 MB venv; install 210 s for Nanobot.
4. **Windows fit.** Pydantic AI imports cleanly everywhere; Nanobot's CLI
   crashed on a fresh profile because its install directory did not exist.
5. **API fit.** Our required flow (typed allowlist tools that emit an approval
   and await a host decision inside a tool) maps directly onto pydantic-ai
   async tools with a dependency object; Nanobot's tool/permission contract
   would have to be reverse-engineered and justified as unverified spike work.

Residual pydantic-ai concerns: 2.9 s median import (mitigated: process starts
on demand, budget measured at ~2.0 s spawn→first event, idle process is not
spawned during search); large dependency tree (mitigated: optional component,
gitignored machine-local venv, pinned requirements + `requirements.lock.txt`);
token-level streaming not yet wired (v1 emits final output once — documented
limitation, not hidden).

## What was measured vs unverified (summary)

- **Measured**: venv sizes, install duration (Nanobot), package import latencies
  (both), interpreter baseline, sidecar spawn→first event, run duration, idle
  CPU/RSS, Windows install/import, CLI crash on fresh profile, tool-call loop
  (pydantic-ai), MCP module availability (both), licenses, Python requirements.
- **Unverified**: real provider API round-trips (no credentials, no network use
  during the spike — by design); Nanobot tool/MCP/permission contracts (only
  module/class presence inspected); Pydantic AI token-streaming against a real
  provider; Windows Credential Manager read of a provisioned key; behavior on
  non-Windows hosts.

## Reproduce

```powershell
# Pydantic AI venv (already present)
C:/Users/Pranshul/AppData/Local/Programs/Python/Python314/python.exe -m venv agent-sidecar/.venv
agent-sidecar/.venv/Scripts/python.exe -m pip install -r agent-sidecar/requirements.txt
agent-sidecar/.venv/Scripts/python.exe agent-sidecar/scripts/measure_import.py pydantic_ai

# Nanobot venv (gitignored; ~210 s)
C:/Users/Pranshul/AppData/Local/Programs/Python/Python314/python.exe -m venv agent-sidecar/.venv-nanobot
agent-sidecar/.venv-nanobot/Scripts/python.exe -m pip install nanobot-ai==0.4.1
agent-sidecar/.venv-nanobot/Scripts/python.exe agent-sidecar/scripts/measure_import.py nanobot.nanobot

# Sidecar startup/run/idle (test model, no key, no network)
agent-sidecar/.venv/Scripts/python.exe agent-sidecar/scripts/measure_sidecar.py
```
