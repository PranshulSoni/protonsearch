//! Explicitly requested Pi installation into app-owned storage.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use serde::Serialize;
use super::{probe_pi, PiInfo};
use super::probe::{run_capture, run_capture_cancellable};
use super::process::pi_command;
use crate::AgentError;

const PACKAGE: &str = "@earendil-works/pi-coding-agent";
static INSTALLING: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiInstallPlan {
    pub version: String,
    pub unpacked_bytes: u64,
    pub integrity: String,
    pub license: String,
    pub destination: String,
    pub source_url: &'static str,
    pub dependencies_size_known: bool,
}

pub fn managed_launcher(data_dir: &Path) -> PathBuf {
    data_dir.join("pi-runtime/current/node_modules/.bin/pi.cmd")
}

fn npm() -> Result<PathBuf, AgentError> {
    let mut command = Command::new("where.exe");
    command.arg("npm.cmd");
    let output = run_capture(command, Duration::from_secs(5))?;
    output.stdout.lines().map(str::trim).map(PathBuf::from)
        .find(|path| path.is_absolute() && path.is_file())
        .ok_or_else(|| AgentError::Pi("Install Node.js 22.19 or newer before installing Pi.".into()))
}

fn npm_command(args: &[String]) -> Result<Command, AgentError> {
    pi_command(&npm()?, args)
}

fn valid_version(version: &str) -> bool {
    !version.is_empty() && version.len() <= 64 && version.as_bytes()[0].is_ascii_digit()
        && version.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

pub fn plan(data_dir: &Path) -> Result<PiInstallPlan, AgentError> {
    if !super::node_version_ok() {
        return Err(AgentError::Pi("Pi requires Node.js 22.19 or newer.".into()));
    }
    let args = ["view", PACKAGE, "version", "dist.unpackedSize", "dist.integrity", "license", "--json", "--registry=https://registry.npmjs.org"]
        .iter().map(|value| value.to_string()).collect::<Vec<_>>();
    let output = run_capture(npm_command(&args)?, Duration::from_secs(30))?;
    if output.exit_code != Some(0) || output.stdout_truncated {
        return Err(AgentError::Pi("Could not retrieve Pi installation information from npm.".into()));
    }
    let value: serde_json::Value = serde_json::from_str(&output.stdout)
        .map_err(|_| AgentError::Pi("npm returned invalid installation information.".into()))?;
    let version = value["version"].as_str().unwrap_or_default().to_string();
    let integrity = value["dist.integrity"].as_str().unwrap_or_default().to_string();
    if !valid_version(&version) || !integrity.starts_with("sha512-") {
        return Err(AgentError::Pi("Pi package version or integrity information is invalid.".into()));
    }
    Ok(PiInstallPlan {
        version,
        unpacked_bytes: value["dist.unpackedSize"].as_u64().unwrap_or(0),
        integrity,
        license: value["license"].as_str().unwrap_or("See upstream license").to_string(),
        destination: data_dir.join("pi-runtime/current").to_string_lossy().into_owned(),
        source_url: "https://pi.dev/docs/latest/quickstart",
        dependencies_size_known: false,
    })
}

pub fn cancel_install() { CANCELLED.store(true, Ordering::Relaxed); }
pub fn is_installing() -> bool { INSTALLING.load(Ordering::Relaxed) }

struct InstallGuard;
impl Drop for InstallGuard {
    fn drop(&mut self) { INSTALLING.store(false, Ordering::Relaxed); }
}

pub fn install(data_dir: &Path, version: &str, confirmed: bool, ensure_idle: impl FnOnce() -> Result<(), AgentError>) -> Result<PiInfo, AgentError> {
    if !confirmed { return Err(AgentError::Pi("Confirm the Pi installation before continuing.".into())); }
    if !valid_version(version) { return Err(AgentError::Pi("Invalid Pi package version.".into())); }
    if INSTALLING.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_err() {
        return Err(AgentError::Busy);
    }
    let _guard = InstallGuard;
    ensure_idle()?;
    CANCELLED.store(false, Ordering::Relaxed);
    let root = data_dir.join("pi-runtime");
    let staging = root.join("staging");
    let current = root.join("current");
    let previous = root.join("previous");
    std::fs::create_dir_all(&root).map_err(AgentError::Io)?;
    if !current.exists() && previous.exists() { std::fs::rename(&previous, &current).map_err(AgentError::Io)?; }
    if staging.exists() { std::fs::remove_dir_all(&staging).map_err(AgentError::Io)?; }
    std::fs::create_dir(&staging).map_err(AgentError::Io)?;
    let args = vec!["install".into(), format!("{PACKAGE}@{version}"), "--ignore-scripts".into(),
        "--no-audit".into(), "--fund=false".into(), "--package-lock=true".into(),
        "--registry=https://registry.npmjs.org".into(), "--prefix".into(), staging.to_string_lossy().into_owned()];
    let result = run_capture_cancellable(npm_command(&args)?, Duration::from_secs(300), &CANCELLED);
    let output = match result {
        Ok(output) => output,
        Err(error) => { let _ = std::fs::remove_dir_all(&staging); return Err(error); }
    };
    if output.exit_code != Some(0) || CANCELLED.load(Ordering::Relaxed) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(AgentError::Pi(if CANCELLED.load(Ordering::Relaxed) { "Pi installation cancelled." } else { "Pi installation failed. The previous installation is retained." }.into()));
    }
    let launcher = staging.join("node_modules/.bin/pi.cmd");
    let info = probe_pi(Some(&launcher)).ok_or_else(|| AgentError::Pi("Installed Pi failed its version check.".into()))?;
    if info.version != version { return Err(AgentError::Pi("Installed Pi version did not match the requested version.".into())); }
    if previous.exists() { std::fs::remove_dir_all(&previous).map_err(AgentError::Io)?; }
    if current.exists() { std::fs::rename(&current, &previous).map_err(AgentError::Io)?; }
    if let Err(error) = std::fs::rename(&staging, &current) {
        if previous.exists() { let _ = std::fs::rename(&previous, &current); }
        return Err(AgentError::Io(error));
    }
    super::clear_probe_cache();
    match probe_pi(Some(&managed_launcher(data_dir))) {
        Some(info) => Ok(info),
        None => {
            let _ = std::fs::remove_dir_all(&current);
            if previous.exists() { let _ = std::fs::rename(&previous, &current); }
            Err(AgentError::Pi("Pi failed its activation check. The previous installation was restored.".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_version_cannot_inject_commands_or_select_another_package() {
        assert!(valid_version("1.5.0"));
        assert!(valid_version("1.5.1-beta.2"));
        for value in ["", "latest", "1.5.0 & calc", "1.5.0/../../other", "1.5.0\n"] { assert!(!valid_version(value)); }
    }
    #[test]
    fn installation_requires_confirmation_before_any_files_are_written() {
        assert!(install(Path::new("unused"), "1.5.0", false, || Ok(())).is_err());
    }
}
