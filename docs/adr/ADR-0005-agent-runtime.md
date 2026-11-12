# ADR-0005: Optional agent layer — adopt an existing Python runtime, do not invent one

- Status: **Accepted** (Gate 6) — Pydantic AI 2.66.0 selected after a measured spike against Nanobot 0.4.1
  (evidence: docs/agent/AGENT_RUNTIME_EVALUATION.md)
- Owners: L2-A4 (agent sidecar), L1-A, L0

## Context

The brief requires an optional agentic workflow layer that must not inflate baseline memory/startup, must never start for search, must run as an isolated subprocess behind a versioned local protocol, and must use an existing Python agent framework rather than a custom planning/tool loop. Candidates named by the brief: Nanobot, Pydantic AI; broader options include LangGraph-based runtimes and ACP-compatible coding agents.

## Decision (direction)

- Evaluate **Pydantic AI** and **Nanobot** in a short licensed spike measuring Windows compatibility, install size, import/startup latency, structured streaming, MCP/tool support, provider configuration, and permission hooks.
- Ship the sidecar as an **on-demand subprocess** (python -m protonsearch_agent) started only when the user launches a workflow; killed after a configurable idle timeout; never present in the process tree during normal search.
- Transport: versioned **JSON Lines over stdio** with request/event schemas already frozen in crates/ps-ipc (AgentRunRequest/AgentEvent/AgentApprovalResponse). ACP/MCP are tool protocols, not our agent engine; MCP tools are exposed to the sidecar through an explicit allowlist where useful.
- Permissions: the agent can only call a narrow, typed Rust capability API (search, read approved files, list/open actions). Destructive/OS/network actions require per-action approval events surfaced in the UI. No credentials in the repo; user-provided keys go to OS secret storage.

## Consequences

- A Python runtime is an optional component, installed separately or bundled as an optional resource; the core app remains fully functional without it.
- The protocol is fixed now so the UI can be built before the sidecar exists; the sidecar can be swapped for a different runtime without changing the UI.
- No agent code may be on the search hot path. A crash or hang in the sidecar must be recoverable without restarting the app.
