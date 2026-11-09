# Pi Agent Security Review — ProtonSearch v2 (G6)

Reviewer: L1-D (rel-recon) / Date: 2026-11-09 (late session)
Task: task-12 (claimed after task-11 "Pi CLI agent integration" completed)
Scope: the Pi agent runtime added in crates/ps-agent/src/pi/** , its lifecycle integration in
crates/ps-agent/src/{lib,error}.rs, the frozen contract in crates/ps-ipc
(AgentSettings/AgentEvent), and the Pi CLI installed on this host
(@earendil-works/pi-coding-agent 1.5.0). The legacy "Hermes" runtime is replaced by Pi; the
embedded Pydantic AI sidecar is kept as an optional fallback (not re-reviewed here).
Method: static review with file:line evidence + targeted greps + reading the installed Pi 1.5.0
docs + offline execution of the test suite (fake Pi JSONL stream, real pi --version probe).
No real model call was made (no credentials on this host: ~/.pi/agent/auth.json is empty and no
provider env vars were set at probe time).
Limitations: no pen-test, no fuzzing, no live provider/MCP/extension execution. Items that could
not be exercised are marked UNVERIFIED.

## 1. What runs, and the trust boundary

Pi is a real coding agent: it can read, edit and write files and run shell commands with the
user's OS permissions, and it does not ask for approval before every tool call (Pi docs
security.md:3). ProtonSearch spawns one "pi --mode json" process per workflow run, on demand, in
a workspace, with a read-only tool allowlist by default and an explicit
settings.agent.allow_writes opt-in for edit,write,bash,powershell tools.

Trust boundary (plain text):

    untrusted inputs                        trusted host (same user)                 external
    user prompt, workspace files,           Tauri shell / Workflows UI               Pi model provider
    AGENTS.md/CLAUDE.md, tool output,  -->  ps-agent::pi lifecycle                   (network)
    web content                             - allowlist, args, stdin prompt          Pi update/catalog/
                                            - timeouts, tree kill, redaction         telemetry endpoints
                                            - events -> UI (no raw stderr)           (Pi startup)
                                            v2 data dir: workspace + support files
                                            user env (provider keys + other secrets inherited)

Assets: user files and their contents; the local index; the ability to edit files and run shell
commands; provider credentials in Pi's auth store or environment; unrelated credentials in the
user environment; the user's expectation that running a workflow authorises a bounded, read-only
action unless they opted into writes.

## 2. Threat model

| # | Threat | Adversary / trigger | Primary controls (code-verified unless noted) |
|---|---|---|---|
| T1 | Prompt injection via file content, tool output, AGENTS.md/CLAUDE.md, context paths | Malicious/compromised file or repo in reach of read tools | Safety preamble declares content untrusted (args.rs:37-55); allowlist limits tools; no per-tool approval by design; residual risk (no sandbox) |
| T2 | Credential exfiltration via model output or tools | Injected content, user prompt, compromised model | Known env values + sk-/pk-/Bearer redacted from stderr/previews (client.rs:551-581, redact.rs:10-60); final Output NOT redacted (PI-SEC-04); env inherited wholesale (PI-SEC-05) |
| T3 | Arbitrary code execution when writes enabled (bash/powershell/edit/write) | Injection or user asks; user opted into writes | All-or-nothing opt-in (args.rs:22-35; ps-ipc:540-571 default allow_writes=false); no per-tool approval; no OS sandbox (pi/mod.rs:15-17; Pi security.md:3,19) |
| T4 | Supply chain / user Pi install: malicious Pi binary, extension, MCP server, skill | Compromised npm install, user extension/MCP, PATH hijack | Launcher prefers explicit/managed path (probe.rs:92-109) but falls back to PATH; user-level extensions/MCP/skills not disabled (PI-SEC-01/02/07) |
| T5 | Network egress: model provider, Pi startup catalog/version/telemetry, MCP tools | Normal use; telemetry attribution | Provider call is the feature; PI_OFFLINE/PI_TELEMETRY not set (PI-SEC-06); MCP tools may remain enabled (PI-SEC-01) |
| T6 | Run-away/orphan process | Model/provider hang, crash | Idle 120 s + max 15 min watchdogs (client.rs:463-502; pi/mod.rs:43-46); taskkill /PID /T then /F (process.rs:176-205); one run at a time (client.rs:203-213) |
| T7 | Protocol abuse/desync: malformed or oversized JSONL, forged event types | Compromised/misbehaving Pi | 4 MiB line cap -> protocol abort + tree kill (client.rs:34,409-426); malformed JSON -> protocol error and stream continues (events.rs:71-89); unknown event types ignored |
| T8 | Settings/IPC tampering: pi_path, allow_writes, model changed by compromised UI/CDP | Same-user malicious UI script | Persisted settings; allow_writes from config, not per request; pi_path unvalidated (PI-SEC-07); Tauri shell not wired yet (PI-SEC-10) |
| T9 | Session/log leakage: prompts, tool output, file contents persisted/exported | Pi session files, exports, crash logs | --no-session passed (args.rs:77-84); stderr in-memory ring, redacted (client.rs:36-38,444-461); Pi may write its own caches/config outside v2 (PI-SEC-12) |
| T10 | Resource exhaustion: huge prompt/context list, huge probe output, repeated probe spawns | Large prompt, malicious launcher, status polling | 4 MiB stdout cap; prompt held in memory; probe capture unbounded/uncached (PI-SEC-08/14) |

## 3. Code-verified controls (good; keep)

| Control | Evidence |
|---|---|
| Read-only tool allowlist default: read,grep,find,ls | crates/ps-agent/src/pi/args.rs:22-35,76-84; crates/ps-ipc/src/lib.rs:540-571 (allow_writes:false, runtime pi) |
| Writes/shell only when the user opted in: edit,write,bash,powershell appended | args.rs:24,29-35; config uses settings.allow_writes (pi/mod.rs:115); no request-level override |
| Prompt delivered on stdin, never on the command line; stdin closed -> Pi's documented initial-message path | args.rs:9-12; client.rs:262-266; E2E argv test tests/pi_e2e.rs:280-320 |
| No shell-string interpolation; .cmd launcher wrapped as cmd.exe /d /s /c with every arg quoted once; cmd metacharacters rejected | process.rs:44-55,65-108 |
| Settings-controlled values (provider/model/thinking/prompt path) validated: non-empty, length-bounded, no control chars; thinking from fixed allowlist; provider requires model | args.rs:67-117 |
| CREATE_NO_WINDOW on Pi, probe, taskkill, tasklist | process.rs:19-29,120,153; probe.rs:148,184; client spawn via pi_command |
| Project trust declined non-interactively: --no-approve; ephemeral: --no-session; --mode json | args.rs:77-84; Pi docs security.md:75-82, cli.md:98-99,237-243 |
| One run at a time (second run -> AgentError::Busy) | client.rs:203-213; E2E tests/pi_e2e.rs:161-176 |
| Idle 120 s / total 15 min kills with distinct error codes | client.rs:463-507; pi/mod.rs:43-46; events.rs:250-276; E2E tests/pi_e2e.rs:213-230 |
| Cancellation kills the whole tree: taskkill /PID /T then /F after 2 s; E2E proves grandchild dies within 3 s | client.rs:326-341; process.rs:176-205; tests/pi_e2e.rs:177-211 |
| JSONL: LF framing, 4 MiB hard line cap, oversized line -> protocol error + tree kill | client.rs:34,399-442; E2E tests/pi_e2e.rs:232-241 |
| Malformed JSON -> one protocol_parse_error, stream continues | events.rs:71-89; E2E tests/pi_e2e.rs:243-253 |
| stderr drained into bounded 200-line ring, redacted, never raw to UI | client.rs:36-38,83-107,444-461 |
| Tool-result previews capped at 400 chars and redacted | events.rs:26-27,151-175,378-381,409-417 |
| Redaction: exact known credential env values + sk-/pk- keys + Bearer tokens | client.rs:551-581; redact.rs:10-60; E2E tests/pi_e2e.rs:255-278 |
| Error text to UI human-readable and redacted; raw stderr never reaches events | events.rs:246-349; client.rs:504-525 |
| Workspace and support files live under the v2 data dir (agent-workspace, agent/) | pi/mod.rs:43-50,78-148; client.rs:215-222; created on demand |
| Probe is cheap: pi --version + node --version, 5 s timeout, pipes drained, tree killed on timeout; no model call, no auth read | probe.rs:20,161-227; real probe output below |
| No credentials read from Pi auth.json; no environment logging; known env values enter in-memory redaction list only | probe.rs:1-9; client.rs:551-581 |
| Offline test suite proves lifecycle without model/credential/network: fake Pi JSONL over .cmd wrapper in a path with a space, cancellation/tree kill, idle timeout, oversize/malformed framing, redaction, argv assertions | src/bin/ps_agent_fake_pi.rs; tests/pi_e2e.rs; 36 unit + 12 Pi integration + 4 legacy tests pass |

Pi behaviours relied on (verified from installed Pi 1.5.0 docs and --help):
- --tools <list> REPLACES the default selection (default tools are read,bash,edit,write); a
  plain-name allowlist is therefore a real restriction, not a union. (pi-coding-agent/docs/cli.md:119-128)
- MCP tools are kept unless an allowlist entry starts with mcp__; to remove them use
  --exclude-tools 'mcp__*' or --no-mcp. (cli.md:120,138,198) -> PI-SEC-01.
- Pi has no per-tool approval; --no-approve only declines project trust. (security.md:3,75-82)
- Project trust does not cover AGENTS.md/CLAUDE.md; context files load regardless unless context
  loading is disabled. (security.md:57)
- Extensions execute inside the Pi process with the same permissions. (security.md:3,90; cli.md:196-214)
- Sessions default to ~/.pi/agent/sessions unless --no-session; --session-dir overrides. (sessions.md:58-63; cli.md:98-99)
- --offline disables automatic network activity including model catalog refreshes (inference still
  works); PI_SKIP_VERSION_CHECK=1 disables the pi.dev version request; PI_TELEMETRY overrides
  install/update telemetry and provider attribution. (cli.md:241-242; environment-variables.md:81-87)
- pi auth print-api-key / print-bearer-token print secrets to stdout; ProtonSearch must never
  invoke them. (cli.md:313-326)

## 4. Findings (ranked)

### PI-SEC-01 — HIGH: MCP tools survive the "read-only" allowlist
Evidence: our args are exactly --mode json --no-session --no-approve --tools read,grep,find,ls
(crates/ps-agent/src/pi/args.rs:76-84). Installed Pi docs state: "--tools Replaces the default
selection ... MCP tools are kept unless an entry starts with mcp__" (pi-coding-agent/docs/cli.md:120)
and "To remove MCP tools, use --exclude-tools 'mcp__*' or --no-mcp" (cli.md:138,198). No --no-mcp
is passed and no allowlist entry starts with mcp__.
Impact: on a user machine with any enabled MCP server (user-level ~/.pi/agent/mcp.json; project
MCP is blocked by --no-approve), the model is still offered those MCP tools in read-only runs,
including network/file capabilities the server exposes. This contradicts the read-only contract
and widens injection blast radius; it also enables network egress beyond the model provider.
Remediation: add --no-mcp (defense-in-depth: also --exclude-tools "mcp__*") to the default arg
vector for both read-only and write runs, until MCP is an explicit opt-in with a server/tool
allowlist. Add an E2E argv assertion and update PI_INTEGRATION.md/UI copy.
Gate: must-fix before Pi is reachable from the Workflows UI / before any public build.

### PI-SEC-02 — MEDIUM: user-level extensions and skills load and execute in-process
Evidence: Pi docs: extensions execute inside the Pi process with the user's permissions
(pi-coding-agent/docs/security.md:3,90); extension/skill discovery is controlled by
--no-extensions / --no-skills (cli.md:196-214). Our args pass neither (args.rs:76-84).
--no-approve only declines project trust (PI_INTEGRATION.md:74-77; Pi security.md:47-57), so
user-installed extensions/skills still load.
Impact: arbitrary user-installed (or supply-chain-compromised) extension code runs when the user
starts an agent workflow; it can register tools outside the allowlist, read env/files and make
network calls, defeating the model-level restriction. The user may not realise that running a
workflow executes installed Pi extensions.
Remediation: pass --no-extensions --no-skills --no-prompt-templates by default; offer a separate
explicit "load my Pi extensions/skills" opt-in if desired (with warning). Document in
PI_INTEGRATION.md.
Gate: must-fix (or explicit consent-gated opt-in) before UI enablement.

### PI-SEC-03 — MEDIUM: AGENTS.md/CLAUDE.md context files load despite --no-approve
Evidence: Pi security.md:57: "Context files such as AGENTS.override.md, AGENTS.md, and CLAUDE.md
load regardless of project trust unless you disable context loading." Our args do not pass
--no-context-files (args.rs:76-84; flag exists at cli.md:214). Default workspace is the
app-managed, empty <data_dir>/agent-workspace (pi/mod.rs:111-115), so current risk is low;
PiAgentConfig::with_workspace allows pointing at a real repository, where this becomes an
injection channel.
Impact: a malicious repository can inject persistent instructions into the system context even
though project trust was declined.
Remediation: pass --no-context-files by default for the agent workspace; expose an explicit
opt-in for trusted workspaces; keep dedicated-workspace guidance.
Gate: should-fix before a workspace picker ships; document now.

### PI-SEC-04 — MEDIUM: final assistant output is not redacted
Evidence: tool previews/errors/stderr go through redacted_preview/redact_text
(events.rs:151-175,378-381,409-417; client.rs:444-461), but the final answer is stored raw
(events.rs:351-370) and emitted raw as AgentEvent::Output (events.rs:303-322). Redaction shapes
are limited to exact known env values, sk-/pk- and Bearer (client.rs:551-581; redact.rs:10-60).
Impact: a model coerced by injected content (or a user asking) can print secrets read from files
or the environment into the UI/agent history and to the provider without the redaction layer
noticing (AIza..., gsk_..., hf_..., ghp_..., AKIA..., xoxb-..., non-Bearer tokens are not
matched). Inconsistent with the preamble and the redaction promise.
Remediation: run redact_text over the final assistant text (and any future Output) with the same
known-secrets list and broaden shape patterns; add a test that a final answer containing a known
env secret and an AIza/gsk_ token is redacted.
Gate: must-fix before the Workflows UI displays final answers.

### PI-SEC-05 — MEDIUM: the whole user environment is inherited by Pi and its tools
Evidence: client.rs:238-247 spawns Pi with default environment inheritance and only adds
extra_env; nothing strips unrelated secrets. collect_redactions (client.rs:551-581) includes many
credential env names, confirming the client expects them in the child. Pi docs: shell tools
receive inherited env and PI_* markers (environment-variables.md:20-53).
Impact: when writes are enabled, bash/powershell can dump AWS_SECRET_ACCESS_KEY, GITHUB_TOKEN,
etc.; extensions can read them regardless. A model/provider compromise or prompt injection can
exfiltrate credentials unrelated to the agent's purpose. Even in read-only mode, file-reading
tools can read .env files (workspace/OS permissions permitting).
Remediation: construct a minimal child environment: system essentials (SystemRoot, SystemDrive,
TEMP/TMP, PATH, USERPROFILE, APPDATA, LOCALAPPDATA) + only provider variables Pi needs; strip
known unrelated secret names. Alternatively document as a hard limitation and require
container/dedicated-user isolation for high assurance.
Gate: should-fix (or explicit warning + container guidance) before public release.

### PI-SEC-06 — LOW-MEDIUM: Pi startup network/telemetry/version-check not disabled
Evidence: Pi docs: --offline disables automatic network activity including model catalog
refreshes (model inference still works), and PI_SKIP_VERSION_CHECK/PI_TELEMETRY exist
(pi-coding-agent/docs/cli.md:241-242; environment-variables.md:81-87). Our spawn sets none
(client.rs:238-247), and PI_INTEGRATION.md:213-216 says "--offline is not passed (provider
inference needs the network)" — inaccurate: --offline only disables automatic startup
operations, not inference.
Impact: Pi may contact pi.dev/catalog/telemetry endpoints on startup before/independent of the
user's request, contradicting privacy claims and leaking provider attribution.
Remediation: set PI_OFFLINE=1, PI_SKIP_VERSION_CHECK=1, PI_TELEMETRY=0 for probe and runs (or
pass --offline); keep inference working. Re-verify against Pi 1.5.0 and record it.
Gate: should-fix before public release; update docs.

### PI-SEC-07 — LOW: launcher resolution falls back to PATH; pi_path trusted blindly
Evidence: probe.rs:92-109: explicit settings path wins (returns None if missing), then
PROTONSEARCH_PI_PATH, then managed ~/.pi/agent/bin/pi.cmd, then the first pi.cmd/pi.exe on PATH.
The explicit path is used as-is (probe.rs:95-97); pi_path comes from AgentSettings
(ps-ipc:538-571), writable via normal settings IPC.
Impact: a same-user attacker (or compromised UI/IPC caller) can point pi_path at any executable,
or plant pi.cmd in a user-writable PATH directory when the managed install is absent; the next
agent run executes it. Same-user execution is already possible via other actions, but this
bypasses the intended "run the installed Pi" contract.
Remediation: default to managed installs only; if PATH fallback stays, show the resolved path in
the UI and require confirmation; treat pi_path changes as sensitive; verify the resolved file is
a Pi launcher (name/version probe) before use.
Gate: should-fix; document residual if accepted.

### PI-SEC-08 — LOW: probe is uncached and capture is unbounded
Evidence: PiAgentConfig::from_settings calls probe_pi every time (pi/mod.rs:84-123), and the
Tauri agent_status path probes on each call (apps/desktop/src-tauri/src/main.rs:492-527);
probe_pi spawns pi --version plus node --version (probe.rs:161-175,145-155). run_capture/read_all
read child output into a Vec<u8> without a byte cap (probe.rs:179-227).
Impact: repeated status polls spawn process pairs (latency/CPU), and a malicious/broken launcher
could emit unbounded output and exhaust memory before the 5 s timeout.
Remediation: cache PiInfo per resolved path/version (re-probe on demand/version change); cap
captured bytes (e.g. 64 KiB); consider a --version-only fast path.
Gate: should-fix before RC.

### PI-SEC-09 — LOW: PID-reuse race in tree termination
Evidence: taskkill /PID <pid> /T then /F in process.rs:176-205; the pid is stored at spawn
(client.rs:247-248) and used by cancel/watchdog even if the child exited microseconds earlier.
If Windows reuses the pid, the kill could target an unrelated process tree.
Impact: rare, local, non-escalating (same user), but could terminate unrelated user work.
Remediation: prefer a Windows Job Object with KILL_ON_JOB_CLOSE (also fixes orphan-node risk),
or verify process creation time/handle before taskkill; keep current path as fallback.
Gate: nice-to-have before RC; document until then.

### PI-SEC-10 — INFO: Tauri shell is not wired to Pi yet
Evidence: apps/desktop/src-tauri/src/main.rs:12,47,489-558 still imports/spawns the legacy
Pydantic AgentClient; AgentStatus reports "pydantic-ai (optional sidecar)" (main.rs:513);
probe_agent_status does not probe Pi. docs/agent/PI_INTEGRATION.md:4 says "wired into the Tauri
shell by L0", which is not yet true in the reviewed tree.
Impact: the Pi backend is not reachable from the app at review time, so this review covers the
library/lifecycle; no Pi-specific runtime risk is exposed to users yet. Wiring must be reviewed
when it lands.
Remediation (wiring contract): gate agent_start on settings.agent.enabled and runtime == Pi;
build config with PiAgentConfig::from_settings(&settings.agent, data_dir); do not accept
allow_writes from the request (persisted setting only); forward events with app.emit; map
agent_cancel to client.cancel() tree kill and agent_stop/quit_app to client.shutdown(); cache
agent_status probes. Keep Pydantic only for AgentRuntime::Pydantic.
Gate: required before enabling the Workflows UI; do not ship Pi reachable without PI-SEC-01..05.

### PI-SEC-11 — INFO (accepted by design): no per-tool approval and no OS sandbox
Pi has no per-tool approval prompt and no sandbox: the working folder does not prevent tools from
reaching other paths; enabled tools use OS permissions (pi-coding-agent/docs/security.md:3,19,33).
Writes are all-or-nothing opt-in; the Workflows approval UI only applies to the legacy Pydantic
fallback (PI_INTEGRATION.md:114-116). This must be disclosed in the UI in plain language and is
the main reason to recommend a container/VM/dedicated account for high-assurance use
(docs/containerization.md).
Gate: UI disclosure at G6; container guidance in release docs.

### PI-SEC-12 — INFO: session/log/data locations
--no-session is passed, so Pi persists no session for our runs (args.rs:80; cli.md:98-99). Our
support file is <data_dir>/agent/pi-system-prompt.txt (no secrets) and the workspace is
<data_dir>/agent-workspace (client.rs:215-222; pi/mod.rs:43-50,146-148). Pi may still write its
own config/state (~/.pi/agent/auth.json, models-store, trust.json, caches) outside the v2 data
dir; we do not read auth.json and never write Pi config (probe.rs:1-9). Session export/share is
not invoked.
Residual: user Pi caches/credentials live in the user's Pi install and are outside v2 data
management; document this.

### PI-SEC-13 — INFO: test-only fake Pi binary
crates/ps-agent/src/bin/ps_agent_fake_pi.rs emulates Pi JSONL for offline tests only. It must
not be shipped in the installer; G7 packaging must verify only ProtonSearch.exe plus intended
resources are bundled.

### PI-SEC-14 — INFO: unbounded prompt/context sizes
build_prompt appends every context_paths entry (control chars stripped, 1024 chars each) without
a count/total cap (client.rs:376-397); the prompt is written to stdin in memory. AgentRunRequest
comes from the trusted UI, so this is robustness rather than injection.
Remediation: cap context paths (e.g. 32) and total prompt bytes (e.g. 64 KiB), log a warning
when truncated.

### PI-SEC-15 — INFO: Pi version is not minimum-enforced
probe_pi parses the version but does not compare it to a minimum (probe.rs:108-175); only Node's
floor (22.19.0) is enforced. A downgraded/compat-broken Pi at the resolved path would be
launched.
Remediation: pin a minimum supported Pi version (1.5.0) and surface an "update Pi" message;
re-validate Pi docs claims when bumping.

## 5. Release gates (Pi runtime)

Must-fix before Pi is enabled in the UI / public build:
1. PI-SEC-01: --no-mcp (+ mcp__* exclusion) in the default arg vector; E2E argv test; docs.
2. PI-SEC-02: --no-extensions --no-skills --no-prompt-templates by default (or explicit,
   consent-gated opt-in).
3. PI-SEC-04: redact final Output; broaden secret shapes; test.
4. PI-SEC-05: minimal child environment (provider vars + system essentials) or explicit warning
   + container guidance.
5. PI-SEC-11: UI disclosure of read-only vs writes, no per-tool approvals, workspace scope,
   provider data leaving the machine.
6. PI-SEC-10: reviewed Tauri wiring that enforces persisted allow_writes and maps cancel/stop to
   tree kill.

Should-fix before RC:
7. PI-SEC-03: --no-context-files default for the dedicated workspace.
8. PI-SEC-06: PI_OFFLINE=1, PI_SKIP_VERSION_CHECK=1, PI_TELEMETRY=0 (verify inference OK).
9. PI-SEC-07: managed-path-only default / confirm pi_path; show resolved launcher.
10. PI-SEC-08: cache probe, cap captured bytes.
11. PI-SEC-09: Job Object kill-on-close (or creation-time verification).
12. PI-SEC-14/15: prompt caps; Pi minimum version check.

Accepted / documented residual risks:
- No per-tool approval and no OS sandbox (Pi design): the allowlist limits offered tools, not
  process reach; writes are all-or-nothing. Container/VM/dedicated user recommended for high
  assurance.
- Provider/model sees the prompt, context paths and tool outputs; that is the product feature and
  only happens when the user runs a workflow with a configured provider.
- Inherited env exposure until PI-SEC-05 is fixed.
- Pi install/path supply chain is the user's installed CLI (MIT npm package, unsigned); the
  managed path is preferred but not integrity-verified.
- No real model/provider run was performed in this review; provider auth, MCP, extension and
  telemetry behaviour beyond docs are UNVERIFIED on this host.

## 6. Verification evidence

- Installed Pi: @earendil-works/pi-coding-agent 1.5.0, license MIT, launcher
  C:\Users\Pranshul\.pi\agent\bin\pi.cmd; Node v26.7.0 (>= 22.19 floor).
- pi --version -> 1.5.0 (no model call, no credentials).
- cargo test -p ps-agent -> 36 unit + 12 Pi integration + 4 legacy = 52 passed, 0 failed
  (Pi integration 3.38 s; log .agents/notes/pi-agent-tests.log, exit 0).
- Real probe test output: real pi probe: version=1.5.0 path=C:\Users\Pranshul\.pi\agent\bin\pi.cmd node_ok=true.
- Offline E2E covers stdin prompt delivery, argv assertions (no prompt on cmd line; read-only
  allowlist), happy mapping, Busy gate, cancellation with grandchild tree kill (<3 s), idle
  timeout, oversized line abort, malformed-line recovery, stderr/preview redaction, --version
  probe; fake Pi uses no network and no credentials.
- No real credentials present at review: ~/.pi/agent/auth.json is empty; no provider env vars
  found in the audit shell. Provider path/MCP/extension behaviour UNVERIFIED.
- Exact commands:
  cargo test -p ps-agent
  cargo test -p ps-agent --test pi_e2e real_pi_version_probe -- --nocapture
  & 'C:\Users\Pranshul\.pi\agent\bin\pi.cmd' --version
