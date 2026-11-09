//! Pi coding-agent CLI backend (default v2 agent runtime).
//!
//! Architecture (details in docs/agent/PI_INTEGRATION.md):
//!
//! * [`probe_pi`] locates the installed Pi launcher and runs `pi --version`
//!   with a 5 s timeout (no model call, no credentials, no network).
//! * [`PiAgentClient`] spawns ONE `pi --mode json` process per run in the
//!   configured workspace, writes the prompt to stdin (never to the command
//!   line), maps Pi JSONL session events to ps-ipc [`ps_ipc::AgentEvent`]s,
//!   enforces idle/run timeouts, detects crashes and cancels the whole process
//!   tree with taskkill.
//! * Tools are read-only by default (`read,grep,find,ls`); `edit,write,bash,
//!   powershell` are added only when the user explicitly enabled writes.
//!
//! Honest security note: Pi has no per-tool approval prompt and no OS sandbox.
//! The allowlist and workspace limit what the model is offered, not what the
//! process can reach on the filesystem. See docs/agent/PI_INTEGRATION.md.

mod args;
mod client;
mod events;
mod probe;
mod process;
mod redact;

use std::path::Path;
use std::time::Duration;

use ps_ipc::AgentSettings;

use crate::error::AgentError;

pub use args::{
    build_pi_run_args, pi_safety_preamble, pi_tool_allowlist, PiRunArgsSpec, READ_ONLY_TOOLS,
    THINKING_LEVELS, WRITE_TOOLS,
};
pub use client::PiAgentClient;
pub use events::PiEventMapper;
pub use probe::{locate_pi, node_version_ok, pid_alive, probe_pi, PiInfo, MIN_NODE_VERSION, PROBE_TIMEOUT};

/// Kill a Pi run when no stdout event arrives for this long.
pub const DEFAULT_PI_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
/// Hard cap for one Pi run (models can loop; we never let a run last forever).
pub const DEFAULT_MAX_RUN_DURATION: Duration = Duration::from_secs(15 * 60);
/// Default working directory when settings do not provide one.
pub const DEFAULT_WORKSPACE_DIR_NAME: &str = "agent-workspace";
/// Default directory (inside data_dir) holding generated support files.
const AGENT_SUPPORT_DIR_NAME: &str = "agent";

/// Everything needed to launch Pi runs.
#[derive(Debug, Clone)]
pub struct PiAgentConfig {
    /// Probed launcher + version (see [`probe_pi`]).
    pub pi: PiInfo,
    /// Working directory for runs (created on demand).
    pub workspace: std::path::PathBuf,
    /// Application data directory; support files go to `<data_dir>/agent`.
    pub data_dir: std::path::PathBuf,
    /// Opt-in: allow edit/write/bash/powershell tools.
    pub allow_writes: bool,
    /// Optional model id passed as `--model`.
    pub model: Option<String>,
    /// Optional provider name passed as `--provider` (requires model).
    pub provider: Option<String>,
    /// Optional thinking level passed as `--thinking`.
    pub thinking: Option<String>,
    /// Kill a run with no events for this long.
    pub idle_timeout: Duration,
    /// Kill a run after this total duration.
    pub max_run_duration: Duration,
    /// Extra environment variables for the child (e.g. provider base URL).
    /// Values are never logged and are added to the redaction list.
    pub extra_env: Vec<(String, String)>,
}

impl PiAgentConfig {
    /// Build a config from the frozen ps-ipc settings, probing Pi (no model call).
    ///
    /// ps-ipc currently has no workspace field, so the workspace defaults to
    /// `<data_dir>/agent-workspace`; callers can override it with
    /// [`PiAgentConfig::with_workspace`].
    pub fn from_settings(settings: &AgentSettings, data_dir: &Path) -> Result<Self, AgentError> {
        let explicit = settings.pi_path.as_deref().map(Path::new);
        let pi = probe_pi(explicit).ok_or(AgentError::NotAvailable)?;
        let model = settings
            .model
            .clone()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let provider = settings
            .provider
            .clone()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let thinking = settings
            .thinking
            .clone()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                std::env::var("PROTONSEARCH_AGENT_THINKING")
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            });
        let idle_timeout = std::env::var("PROTONSEARCH_AGENT_IDLE_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(DEFAULT_PI_IDLE_TIMEOUT);
        let max_run_duration = std::env::var("PROTONSEARCH_AGENT_MAX_RUN_MS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(DEFAULT_MAX_RUN_DURATION);
        Ok(Self {
            pi,
            workspace: settings
                .workspace
                .as_ref()
                .map(|value| std::path::PathBuf::from(value.trim()))
                .filter(|value| !value.as_os_str().is_empty())
                .unwrap_or_else(|| data_dir.join(DEFAULT_WORKSPACE_DIR_NAME)),
            data_dir: data_dir.to_path_buf(),
            allow_writes: settings.allow_writes,
            model,
            provider,
            thinking,
            idle_timeout,
            max_run_duration,
            extra_env: Vec::new(),
        })
    }

    pub fn with_workspace(mut self, workspace: impl Into<std::path::PathBuf>) -> Self {
        self.workspace = workspace.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.thinking = Some(thinking.into());
        self
    }

    pub fn with_allow_writes(mut self, allow_writes: bool) -> Self {
        self.allow_writes = allow_writes;
        self
    }

    /// Directory prepended to the workspace-relative support files.
    pub(crate) fn support_dir(&self) -> std::path::PathBuf {
        self.data_dir.join(AGENT_SUPPORT_DIR_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_dir_lives_under_data_dir() {
        let config = PiAgentConfig {
            pi: PiInfo {
                path: "pi.cmd".into(),
                version: "1.5.0".into(),
                node_ok: true,
            },
            workspace: "ws".into(),
            data_dir: "data".into(),
            allow_writes: false,
            model: None,
            provider: None,
            thinking: None,
            idle_timeout: DEFAULT_PI_IDLE_TIMEOUT,
            max_run_duration: DEFAULT_MAX_RUN_DURATION,
            extra_env: Vec::new(),
        };
        assert!(config.support_dir().ends_with("agent"));
    }
}
