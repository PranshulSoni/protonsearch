//! Optional Python agent sidecar client (Gate 6). OWNER: agent-sidecar.
//!
//! The sidecar is a Pydantic AI agent process (agent-sidecar/ in the repo)
//! that is launched on demand, speaks versioned JSON Lines over stdio, and is
//! never started for search or launch. No credentials are bundled; the user
//! configures a model and the model key lives in the environment or the OS
//! secret store.
//!
//! This crate owns: process lifecycle (CREATE_NO_WINDOW, piped stdio, stderr
//! drain), JSONL framing with a hard line bound, event streaming, cancel,
//! approval responses, configurable idle-timeout kill, crash detection with at
//! most one automatic restart, and a non-spawning capability probe.
//!
//! Integration contract for the Tauri shell: docs/agent/INTEGRATION.md.

pub mod config;
mod client;
mod error;
mod protocol;

pub use client::{AgentClient, EventCallback, MAX_RESTARTS, SHUTDOWN_GRACE};
pub use config::{
    default_config, runtime_available, runtime_available_for, venv_python, SidecarConfig,
    DEFAULT_IDLE_TIMEOUT,
};
pub use error::AgentError;
pub use ps_ipc::{AgentApprovalResponse, AgentEvent, AgentRunRequest};

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
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
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
