//! Sidecar location, capability probe and runtime knobs.
//!
//! The probe never spawns a process: it only checks that the venv interpreter
//! and the Python package entry point exist. Nothing here runs on the search
//! path; `default_config` is expected to be called lazily when a workflow is
//! started (or when the UI asks for the agent status).

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default: kill an idle sidecar after two minutes. The client kills only when
/// no run is active; a live run is never interrupted by the idle watchdog.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

/// Location and launch configuration for the Python sidecar.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// Interpreter to run, normally `<sidecar_dir>/.venv/Scripts/python.exe`.
    pub python: PathBuf,
    /// Directory containing the `protonsearch_agent` package (working dir).
    pub sidecar_dir: PathBuf,
    /// Optional model override; `None` means the sidecar uses its env/config.
    pub model: Option<String>,
    /// Use the deterministic scripted model (no key, no network).
    pub test_model: bool,
    /// Kill the process after this much inactivity with no active run.
    pub idle_timeout: Duration,
    /// Extra environment variables (e.g. provider base URL); values are never logged.
    pub extra_env: Vec<(String, String)>,
}

impl SidecarConfig {
    /// `<sidecar_dir>/protonsearch_agent/__main__.py` — the capability probe target.
    pub fn script(&self) -> PathBuf {
        self.sidecar_dir.join("protonsearch_agent").join("__main__.py")
    }

    pub fn with_idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = idle_timeout;
        self
    }

    pub fn with_test_model(mut self) -> Self {
        self.test_model = true;
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Conventional venv interpreter path for a sidecar directory.
pub fn venv_python(sidecar_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        sidecar_dir.join(".venv").join("Scripts").join("python.exe")
    } else {
        sidecar_dir.join(".venv").join("bin").join("python")
    }
}

/// True when both the interpreter and the Python entry point exist. Does not
/// spawn anything and does not import the framework, so it is safe to call
/// from a status query.
pub fn runtime_available(python: &Path, script: &Path) -> bool {
    python.is_file() && script.is_file()
}

/// Convenience probe for a full config.
pub fn runtime_available_for(config: &SidecarConfig) -> bool {
    runtime_available(&config.python, &config.script())
}

fn env_string(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.trim().parse::<u64>().ok()
}

fn env_bool(name: &str) -> bool {
    matches!(
        std::env::var(name).map(|value| value.trim().to_ascii_lowercase()),
        Ok(value) if value == "1" || value == "true" || value == "yes" || value == "on"
    )
}

/// Locate the sidecar and build its launch config.
///
/// Search order: `PROTONSEARCH_AGENT_SIDECAR_DIR`, a packaged
/// `agent-sidecar` (or `resources/agent-sidecar`) next to the executable, and
/// finally the development checkout relative to this crate. Returns `None`
/// when no usable runtime is present (Python venv missing). `_data_dir` is
/// reserved for future per-user copies of the sidecar; it is not used to walk
/// the filesystem today.
pub fn default_config(_data_dir: &Path) -> Option<SidecarConfig> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = env_string("PROTONSEARCH_AGENT_SIDECAR_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("agent-sidecar"));
            candidates.push(parent.join("resources").join("agent-sidecar"));
        }
    }
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("agent-sidecar"),
    );

    for sidecar_dir in candidates {
        let python = env_string("PROTONSEARCH_AGENT_PYTHON")
            .map(PathBuf::from)
            .unwrap_or_else(|| venv_python(&sidecar_dir));
        let config = SidecarConfig {
            python,
            sidecar_dir,
            model: env_string("PROTONSEARCH_AGENT_MODEL"),
            test_model: env_bool("PROTONSEARCH_AGENT_TEST_MODEL"),
            idle_timeout: env_u64("PROTONSEARCH_AGENT_IDLE_TIMEOUT_MS")
                .map(Duration::from_millis)
                .unwrap_or(DEFAULT_IDLE_TIMEOUT),
            extra_env: Vec::new(),
        };
        if runtime_available_for(&config) {
            return Some(config);
        }
    }
    None
}
