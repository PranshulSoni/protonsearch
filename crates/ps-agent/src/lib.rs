//! Agent runtime clients (Gate 6). OWNER: agent-sidecar.
//!
//! Two interchangeable backends map their native event stream onto the frozen
//! ps-ipc [`AgentEvent`] contract:
//!
//! * [`pi`] — the default runtime: the installed Pi coding-agent CLI
//!   (`@earendil-works/pi-coding-agent`, spawned as `pi --mode json`). One
//!   process per run, read-only tools by default, opt-in writes, idle/run
//!   timeouts and tree-wide cancellation. See `docs/agent/PI_INTEGRATION.md`.
//! * The legacy embedded Python sidecar ([crate::AgentClient], Pydantic AI) is
//!   kept as an optional fallback; see `docs/agent/INTEGRATION.md`.
//!
//! Nothing here runs on the search path: clients are created lazily when a
//! workflow starts. The Pi probe never makes a model call.

mod client;
pub mod config;
mod error;
pub mod pi;
mod protocol;
pub mod task;

pub use client::{AgentClient, EventCallback, MAX_RESTARTS, SHUTDOWN_GRACE};
pub use config::{
    default_config, runtime_available, runtime_available_for, venv_python, SidecarConfig,
    DEFAULT_IDLE_TIMEOUT,
};
pub use error::AgentError;
pub use pi::{
    build_pi_run_args, clear_probe_cache, is_child_env_allowed, locate_pi, locate_pi_with_source,
    pi_safety_preamble, pi_tool_allowlist, pi_version_supported, probe_pi, probe_pi_cached, PiAgentClient,
    PiAgentConfig, PiInfo, PiRunArgsSpec, PiSource, DEFAULT_MAX_RUN_DURATION, DEFAULT_PI_IDLE_TIMEOUT,
    MAX_CAPTURE_BYTES, MIN_PI_VERSION,
};
pub use ps_ipc::{AgentApprovalResponse, AgentEvent, AgentRunRequest};
pub use task::{
    AgentPermissionState, AgentTask, AgentTaskActivity, AgentTaskControls, AgentTaskErrorInfo,
    AgentTaskLaunch, AgentTaskListener, AgentTaskLogEntry, AgentTaskLogLevel, AgentTaskNode,
    AgentTaskProgress, AgentTaskProgressKind, AgentTaskRestart, AgentTaskRuntime, AgentTaskService,
    AgentTaskState, AgentTaskStore, MAX_LOG_ENTRIES, MAX_PERSISTED_TASKS,
};

/// Agent protocol version spoken by the sidecar (frozen ps-ipc version).
pub const PROTOCOL_VERSION: u32 = ps_ipc::PROTOCOL_VERSION;

/// JSONL frame limit shared with the Python sidecar.
pub const MAX_LINE_BYTES: usize = protocol::MAX_LINE_BYTES;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        // Use the workspace target dir instead of the OS temp dir: the host
        // sandbox may deny writes under %TEMP%, while target/ is always usable.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join(format!("ps-agent-test-{tag}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn runtime_probe_requires_both_files_without_spawning() {
        let dir = scratch_dir("probe");
        let python = dir.join("python.exe");
        let script = dir.join("protonsearch_agent").join("__main__.py");
        assert!(!runtime_available(&python, &script));
        fs::write(&python, b"not really python").unwrap();
        assert!(!runtime_available(&python, &script));
        fs::create_dir_all(script.parent().unwrap()).unwrap();
        fs::write(&script, b"# placeholder").unwrap();
        assert!(runtime_available(&python, &script));
        let config = SidecarConfig {
            python,
            sidecar_dir: dir.clone(),
            model: None,
            test_model: false,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            extra_env: Vec::new(),
        };
        assert!(runtime_available_for(&config));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn venv_python_path_is_platform_conventional() {
        let dir = std::env::temp_dir().join("ps-agent-venv");
        let python = venv_python(&dir);
        if cfg!(windows) {
            assert!(python.ends_with(".venv\\Scripts\\python.exe"));
        } else {
            assert!(python.ends_with(".venv/bin/python"));
        }
    }

    #[test]
    fn missing_runtime_returns_not_available() {
        let dir = scratch_dir("missing");
        let config = SidecarConfig {
            python: dir.join("missing-python.exe"),
            sidecar_dir: dir.clone(),
            model: None,
            test_model: true,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            extra_env: Vec::new(),
        };
        match AgentClient::spawn(config, |_event| {}) {
            Err(AgentError::NotAvailable) => {}
            Err(other) => panic!("expected NotAvailable, got {other}"),
            Ok(_) => panic!("expected NotAvailable but the client spawned"),
        }
        let _ = fs::remove_dir_all(dir);
    }
}
