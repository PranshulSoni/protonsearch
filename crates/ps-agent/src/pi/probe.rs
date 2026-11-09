//! Locate and probe the installed Pi coding-agent CLI.
//!
//! The probe is cheap and harmless: it runs `pi --version` (no model call, no
//! credential use) and `node --version` with a 5 s timeout and CREATE_NO_WINDOW.
//! Resolution order: an explicit path from settings, then PROTONSEARCH_PI_PATH,
//! then the default managed install (~/.pi/agent/bin/pi.cmd), then PATH.
//!
//! The probe never spawns a model run, never reads auth.json and never logs the
//! environment.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::process::{configure_no_window, is_pid_alive, pi_command, terminate_process_tree};

/// How long a `--version` probe may take before we give up.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Minimum Node.js version required by the installed Pi launcher.
pub const MIN_NODE_VERSION: (u32, u32, u32) = (22, 19, 0);

/// Result of probing the Pi runtime. Never contains credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiInfo {
    /// Launcher used for runs (pi.cmd on the managed Windows install).
    pub path: PathBuf,
    /// Version reported by `pi --version`, e.g. "1.5.0".
    pub version: String,
    /// True when Node.js was found and is at least 22.19.0 (Pi requirement).
    pub node_ok: bool,
}

#[derive(Debug)]
pub(crate) struct Capture {
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

fn home_dir() -> Option<PathBuf> {
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        if !profile.is_empty() {
            return Some(PathBuf::from(profile));
        }
    }
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
}

fn launcher_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["pi.cmd", "pi.exe", "pi.bat"]
    } else {
        &["pi"]
    }
}

fn managed_launcher_paths() -> Vec<PathBuf> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for name in launcher_names() {
        paths.push(home.join(".pi").join("agent").join("bin").join(name));
    }
    paths
}

fn path_candidates() -> Vec<PathBuf> {
    let Some(path_var) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for dir in std::env::split_paths(&path_var) {
        for name in launcher_names() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }
    found
}

/// Resolve the Pi launcher without spawning anything.
///
/// `explicit` (ps-ipc AgentSettings.pi_path) wins; a configured but missing
/// path returns None instead of silently using a different install.
pub fn locate_pi(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return if path.is_file() {
            Some(path.to_path_buf())
        } else {
            None
        };
    }
    if let Some(env_path) = std::env::var_os("PROTONSEARCH_PI_PATH") {
        let path = PathBuf::from(env_path);
        if path.is_file() {
            return Some(path);
        }
    }
    managed_launcher_paths()
        .into_iter()
        .find(|candidate| candidate.is_file())
        .or_else(|| path_candidates().into_iter().next())
}

/// Parse "v1.5.0"/"1.5.0" style output into a display version.
fn parse_version(output: &str) -> Option<String> {
    let line = output.lines().map(str::trim).find(|line| !line.is_empty())?;
    let line = line.trim_start_matches(['v', 'V']);
    let version: String = line
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '+')
        })
        .take(64)
        .collect();
    if version.is_empty() || !version.chars().any(|character| character.is_ascii_digit()) {
        return None;
    }
    Some(version)
}

fn parse_node_version(output: &str) -> Option<(u32, u32, u32)> {
    let text = output.trim().trim_start_matches(['v', 'V']);
    let mut parts = text.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().unwrap_or(0);
    let patch_text = parts.next().unwrap_or("0");
    let patch = patch_text
        .split(|character: char| !character.is_ascii_digit())
        .next()
        .unwrap_or("0")
        .parse::<u32>()
        .unwrap_or(0);
    Some((major, minor, patch))
}

fn node_version_at_least_minimum(output: &str) -> bool {
    parse_node_version(output)
        .map(|(major, minor, patch)| (major, minor, patch) >= MIN_NODE_VERSION)
        .unwrap_or(false)
}

/// True when node is installed and new enough for Pi.
pub fn node_version_ok() -> bool {
    let mut command = Command::new("node");
    command.arg("--version");
    configure_no_window(&mut command);
    match run_capture(command, PROBE_TIMEOUT) {
        Ok(capture) if !capture.timed_out && capture.exit_code == Some(0) => {
            node_version_at_least_minimum(&capture.stdout)
        }
        _ => false,
    }
}

/// Probe the Pi CLI: resolve the launcher and read its version.
///
/// Returns None when Pi is absent, the launcher cannot run, or the probe times
/// out. This performs no network access and no model call.
pub fn probe_pi(explicit: Option<&Path>) -> Option<PiInfo> {
    let path = locate_pi(explicit)?;
    let args = vec!["--version".to_string()];
    let command = pi_command(&path, &args).ok()?;
    let capture = run_capture(command, PROBE_TIMEOUT).ok()?;
    if capture.timed_out || capture.exit_code != Some(0) {
        return None;
    }
    let version = parse_version(&capture.stdout)?;
    Some(PiInfo {
        path,
        version,
        node_ok: node_version_ok(),
    })
}

/// Run a command with piped stdio and a timeout, draining both pipes on
/// separate threads so the child can never block on a full pipe.
pub(crate) fn run_capture(mut command: Command, timeout: Duration) -> Result<Capture, crate::AgentError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_no_window(&mut command);
    let mut child = command.spawn().map_err(crate::AgentError::Spawn)?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_thread = thread::spawn(move || read_all(stdout));
    let stderr_thread = thread::spawn(move || read_all(stderr));

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    timed_out = true;
                    let pid = child.id();
                    let _ = child.kill();
                    terminate_process_tree(pid, Duration::ZERO);
                    let _ = child.wait();
                    break None;
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(crate::AgentError::Io(error)),
        }
    };
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();
    Ok(Capture {
        stdout,
        stderr,
        exit_code: status.and_then(|status| status.code()),
        timed_out,
    })
}

fn read_all(pipe: Option<impl Read>) -> String {
    let Some(mut pipe) = pipe else {
        return String::new();
    };
    let mut buffer = Vec::new();
    let _ = pipe.read_to_end(&mut buffer);
    String::from_utf8_lossy(&buffer).into_owned()
}

/// True when the pid is alive (used by tests and diagnostics).
pub fn pid_alive(pid: u32) -> bool {
    is_pid_alive(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_prefixed_versions() {
        assert_eq!(parse_version("1.5.0\n").as_deref(), Some("1.5.0"));
        assert_eq!(parse_version("v1.5.0\nextra").as_deref(), Some("1.5.0"));
        assert_eq!(parse_version("no digits here"), None);
    }

    #[test]
    fn node_version_floor_is_22_19_0() {
        assert!(node_version_at_least_minimum("v22.19.0"));
        assert!(node_version_at_least_minimum("v26.7.0"));
        assert!(!node_version_at_least_minimum("v22.18.9"));
        assert!(!node_version_at_least_minimum("v20.11.1"));
        assert!(!node_version_at_least_minimum("not a version"));
    }

    #[test]
    fn missing_explicit_path_does_not_fall_back() {
        let missing = PathBuf::from(r"C:\definitely\not\here\pi.cmd");
        assert_eq!(locate_pi(Some(&missing)), None);
    }
}
