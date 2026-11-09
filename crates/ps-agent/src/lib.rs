//! Optional Python agent sidecar client (Gate 6). OWNER: agent-sidecar.
//!
//! The sidecar is launched on demand, speaks versioned JSON-lines over stdio,
//! and is never started for search/launch. No credentials are bundled.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SidecarConfig {
    pub python: PathBuf,
    pub script: PathBuf,
    pub model: Option<String>,
}

/// Locate the bundled sidecar entry point relative to the app data dir.
pub fn default_config(_data_dir: &Path) -> Option<SidecarConfig> {
    None
}

/// True when the sidecar runtime is likely usable (does not start it).
pub fn runtime_available(_python: &Path, _script: &Path) -> bool {
    false
}
