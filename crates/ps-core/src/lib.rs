//! ProtonSearch v2 engine facade: query coordination, ranking, persistence.
//! OWNER: engine-core (L1-A). The public `Engine` API below is consumed by the
//! Tauri shell and the benchmark harness; keep it stable (L0 approval to change).

pub mod engine;
pub mod paths;
pub mod query;
pub mod rank;
pub mod settings;

pub use engine::{Engine, EngineConfig};
pub use ps_ipc as ipc;
