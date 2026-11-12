# ADR-0006: Versioned IPC between Rust engine, Tauri shell, WebView UI and agent sidecar

- Status: **Accepted** (Gate 1)
- Owners: L1-A, L0

## Context

Three boundaries need contracts: (1) Rust engine <-> Tauri shell (in-process, but semantically an API), (2) Tauri shell <-> WebView UI (Tauri commands/events), (3) Rust <-> Python agent sidecar (stdio). Without a single source of truth the UI, engine and benchmarks drift.

## Decision

- crates/ps-ipc is the single Rust source of truth for all DTOs, with PROTOCOL_VERSION = 1 and camelCase wire names. apps/desktop/src/bridge/types.ts mirrors it; both are Lead-owned and change through an explicit version bump.
- UI->engine is strictly **request/response per user action** with a monotonic requestId. The UI drops any response whose id is stale. No push-based result streaming in v2 alpha; status changes are polled slowly (desktop app only) or emitted as coarse Tauri events.
- All actions are **structured ActionSpec values**. No command strings, no shell interpolation, no arbitrary URL schemes; the shell executes them through reviewed native helpers.
- The agent sidecar speaks one JSON object per line, each tagged with type; stdin/stdout only, stderr for logs; run-scoped ids; explicit cancellation and approval messages.
- Errors are structured values (OpenOutcome, AgentEvent::Error), not panics across the boundary.

## Consequences

- Both sides can be implemented and tested independently (mock bridge in the browser; stub engine in tests).
- Adding a capability means changing the frozen contract deliberately, with a protocol version bump and mirrored TS types.
- Tauri capabilities/CSP restrict what the WebView can do; the UI cannot read arbitrary files or shell out on its own.
