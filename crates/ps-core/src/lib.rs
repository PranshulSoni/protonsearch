//! ProtonSearch v2 engine facade: query coordination, ranking, persistence.
//!
//! Ownership:
//! - engine.rs / db.rs / catalog.rs / settings.rs / paths.rs: L0 integration layer
//! - query.rs / rank.rs: engine-core
//!
//! The public Engine API below is consumed by the Tauri shell and benchmarks.

pub mod catalog;
pub mod db;
pub mod engine;
pub mod migration;
pub mod paths;
pub mod query;
pub mod rank;
pub mod settings;

pub use engine::{Engine, EngineConfig};
pub use ps_ipc as ipc;