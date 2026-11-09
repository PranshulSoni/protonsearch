//! Runtime-agnostic agent task model, persistence and change service (V9).
//!
//! The V9 Agents card and desktop Agents view need real, durable task state
//! rather than a transcript-only view. This module provides:
//!
//! * [model] - runtime-agnostic task types (persistable, camelCase JSON, ready
//!   to be lifted into ps-ipc); they map the frozen [crate::AgentEvent] stream;
//! * [store::AgentTaskStore] - durable JSON store at
//!   <data_dir>/agents/tasks.json with a redacted log ring, parent/child ids,
//!   permissions, restart history, bounded history and crash recovery;
//! * [service::AgentTaskService] - store + change listener + runtime launch
//!   adapters + activity/search projection used by the Tauri shell;
//! * [model::task_activity] - projection for the engine's activity history.
//!
//! Honest semantics: Pi exposes no sub-agents (tasks are flat, parentId null),
//! no pause/resume (controls are always disabled) and no numeric progress
//! (progress stays indeterminate). Anything else would be fake state.
//!
//! Privacy: only a redacted title/summary, logs, states and permissions are
//! persisted. Prompt text, model output and tool payloads are never stored.

mod model;
mod service;
mod store;

pub use model::*;
pub use service::{AgentTaskListener, AgentTaskService};
pub use store::{tasks_path, AgentTaskStore};
