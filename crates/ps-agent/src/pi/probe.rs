//! Locate and probe the installed Pi coding-agent CLI.
//!
//! The probe is cheap and harmless: it runs `pi --version` (no model call, no
//! credential use) and `node --version` with a 5 s timeout and CREATE_NO_WINDOW.
//! Resolution order: an explicit absolute path from settings, then
//! PROTONSEARCH_PI_PATH, then the managed install
//! (~/.pi/agent/bin/pi.cmd), then a launcher on PATH. Every candidate must be an
//! absolute existing file with the expected `pi` launcher name (PI-SEC-07).
//!
//! Probes run with the minimal probe environment (PI-SEC-05) and with
//! PI_OFFLINE=1 / PI_SKIP_VERSION_CHECK=1 / PI_TELEMETRY=0 (PI-SEC-06).
//! Capture output is capped at 64 KiB (PI-SEC-08) and results are cached per
//! resolved launcher.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use super::env::apply_probe_environment;
use super::process::{configure_no_window, is_pid_alive, pi_command, terminate_process_tree};

/// How long a `--version` probe may take before we give up.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Minimum Node.js version required by the installed Pi launcher.
pub const MIN_NODE_VERSION: (u32, u32, u32) = (22, 19, 0);

/// Minimum Pi version supported (PI-SEC-15). Bump only after re-validating the
/// flags and JSONL contract against the installed docs.
pub const MIN_PI_VERSION: (u32, u32, u32) = (1, 5, 0);

/// Maximum bytes captured from a probe child (PI-SEC-08).
pub const MAX_CAPTURE_BYTES: usize = 64 * 1024;

/// Where the resolved Pi launcher came from (useful for the UI to display).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiSource {
    /// Explicit absolute path from ps-ipc AgentSettings.pi_path.
    Explicit,
    /// PROTONSEARCH_PI_PATH environment variable.
    Env,
    /// Managed install at ~/.pi/agent/bin/pi.cmd.
    Managed,
    /// A pi launcher discovered on PATH (least trusted).
    Path,
}

/// Result of probing the Pi runtime. Never contains credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiInfo {
    /// Launcher used for runs (pi.cmd on the managed Windows install).
    pub path: PathBuf,
    /// Version reported by `pi --version`, e.g. "1.5.0".
    pub version: String,
    /// True when Node.js was found and is at least 22.19.0 (Pi requirement).
    pub node_ok: bool,
    /// How the launcher path was resolved.
    pub source: PiSource,
}

#[derive(Debug)]
pub(crate) struct Capture {
    pub stdout: String,
    #[allow(dead_code)]
    pub stdout_truncated: bool,
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

/// A launcher must be an absolute existing file called `pi` (with the
/// platform-appropriate executable extension) — never an arbitrary executable.
fn is_plausible_launcher(path: &Path) -> bool {
    if !path.is_absolute() || !path.is_file() {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    if !stem.eq_ignore_ascii_case("pi") {
        return false;
    }
    if cfg!(windows) {
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| {
                extension.eq_ignore_ascii_case("cmd")
                    || extension.eq_ignore_ascii_case("exe")
                    || extension.eq_ignore_ascii_case("bat")
            })
            .unwrap_or(false)
    } else {
        true
    }
}

fn managed_launcher_paths() -> Vec<PathBuf> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    launcher_names()
        .iter()
        .map(|name| home.join(".pi").join("agent").join("bin").join(name))
        .collect()
}

fn path_candidates() -> Vec<PathBuf> {
    let Some(path_var) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for dir in std::env::split_paths(&path_var) {
        if !dir.is_absolute() {
            continue;
        }
        for name in launcher_names() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }
    found
}

/// Resolve the Pi launcher and its provenance without spawning anything.
pub fn locate_pi_with_source(explicit: Option<&Path>) -> Option<(PathBuf, PiSource)> {
    if let Some(path) = explicit {
        // A configured path is trusted only when it is a plausible Pi launcher;
        // a missing/relative/arbitrary path is rejected outright (PI-SEC-07).
        return is_plausible_launcher(path).then(|| (path.to_path_buf(), PiSource::Explicit));
    }
    if let Some(env_path) = std::env::var_os("PROTONSEARCH_PI_PATH") {
        let path = PathBuf::from(env_path);
        if is_plausible_launcher(&path) {
            return Some((path, PiSource::Env));
        }
    }
    if let Some(candidate) = managed_launcher_paths()
        .into_iter()
        .find(|candidate| is_plausible_launcher(candidate))
    {
        return Some((candidate, PiSource::Managed));
    }
    path_candidates()
        .into_iter()
        .find(|candidate| is_plausible_launcher(candidate))
        .map(|path| (path, PiSource::Path))
}

/// Resolve the Pi launcher without spawning anything.
///
/// `explicit` (ps-ipc AgentSettings.pi_path) wins; a configured but invalid
/// path returns None instead of silently using a different install.
pub fn locate_pi(explicit: Option<&Path>) -> Option<PathBuf> {
    locate_pi_with_source(explicit).map(|(path, _source)| path)
}

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

fn parse_semver(text: &str) -> Option<(u32, u32, u32)> {
    let text = text.trim().trim_start_matches(['v', 'V']);
    let mut parts = text.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().unwrap_or(0);
    let patch = parts
        .next()
        .unwrap_or("0")
        .split(|character: char| !character.is_ascii_digit())
        .next()
        .unwrap_or("0")
        .parse::<u32>()
        .unwrap_or(0);
    Some((major, minor, patch))
}

fn version_at_least(version: &str, minimum: (u32, u32, u32)) -> bool {
    parse_semver(version)
        .map(|parsed| parsed >= minimum)
        .unwrap_or(false)
}

fn node_version_at_least_minimum(output: &str) -> bool {
    version_at_least(output, MIN_NODE_VERSION)
}

/// True when the reported Pi version is supported (PI-SEC-15).
pub fn pi_version_supported(version: &str) -> bool {
    version_at_least(version, MIN_PI_VERSION)
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

/// Probe the Pi CLI: resolve the launcher and read its version (uncached).
///
/// Returns None when Pi is absent, the launcher cannot run, or the probe times
/// out. This performs no network access, no model call and no credential read.
pub fn probe_pi(explicit: Option<&Path>) -> Option<PiInfo> {
    let (path, source) = locate_pi_with_source(explicit)?;
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
        source,
    })
}

fn probe_cache() -> &'static Mutex<HashMap<String, PiInfo>> {
    static CACHE: OnceLock<Mutex<HashMap<String, PiInfo>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Clear the cached probe results (tests / explicit re-probe after an update).
pub fn clear_probe_cache() {
    if let Ok(mut cache) = probe_cache().lock() {
        cache.clear();
    }
}

/// Cached probe (PI-SEC-08): status polling does not re-spawn Pi every time.
/// The cache key is the explicit path or "<auto>"; clear it after a Pi update.
pub fn probe_pi_cached(explicit: Option<&Path>) -> Option<PiInfo> {
    let key = explicit
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<auto>".to_string());
    if let Ok(cache) = probe_cache().lock() {
        if let Some(info) = cache.get(&key) {
            return Some(info.clone());
        }
    }
    let info = probe_pi(explicit)?;
    if let Ok(mut cache) = probe_cache().lock() {
        cache.insert(key, info.clone());
    }
    Some(info)
}

/// Run a command with piped stdio and a timeout, draining both pipes on
/// separate threads. Output capture is capped (PI-SEC-08).
pub(crate) fn run_capture(command: Command, timeout: Duration) -> Result<Capture, crate::AgentError> {
    run_capture_cancellable(command, timeout, &std::sync::atomic::AtomicBool::new(false))
}

pub(crate) fn run_capture_cancellable(mut command: Command, timeout: Duration, cancelled: &std::sync::atomic::AtomicBool) -> Result<Capture, crate::AgentError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_no_window(&mut command);
    apply_probe_environment(&mut command);
    let mut child = command.spawn().map_err(crate::AgentError::Spawn)?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_thread = thread::spawn(move || read_limited(stdout, MAX_CAPTURE_BYTES));
    let stderr_thread = thread::spawn(move || read_limited(stderr, MAX_CAPTURE_BYTES));

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if Instant::now() >= deadline || cancelled.load(std::sync::atomic::Ordering::Relaxed) {
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
    let (stdout, stdout_truncated) = stdout_thread.join().unwrap_or_default();
    let (stderr, _) = stderr_thread.join().unwrap_or_default();
    Ok(Capture {
        stdout,
        stdout_truncated,
        stderr,
        exit_code: status.and_then(|status| status.code()),
        timed_out,
    })
}

fn read_limited(pipe: Option<impl Read>, limit: usize) -> (String, bool) {
    let Some(pipe) = pipe else {
        return (String::new(), false);
    };
    let mut buffer = Vec::new();
    let mut pipe = pipe;
    let mut limited = (&mut pipe).take(limit as u64 + 1);
    let _ = limited.read_to_end(&mut buffer);
    let truncated = buffer.len() > limit;
    buffer.truncate(limit);
    let _ = std::io::copy(&mut pipe, &mut std::io::sink());
    (String::from_utf8_lossy(&buffer).into_owned(), truncated)
}

/// True when the pid is alive (used by tests and diagnostics).
pub fn pid_alive(pid: u32) -> bool {
    is_pid_alive(pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_plain_and_prefixed_versions() {
        assert_eq!(parse_version("1.5.0\n").as_deref(), Some("1.5.0"));
        assert_eq!(parse_version("v1.5.0\nextra").as_deref(), Some("1.5.0"));
        assert_eq!(parse_version("no digits here"), None);
    }

    #[cfg(windows)]
    #[test]
    fn cancelled_capture_terminates_the_child() {
        let cancelled = std::sync::atomic::AtomicBool::new(true);
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", "Start-Sleep -Seconds 20"]);
        let started = Instant::now();
        let result = run_capture_cancellable(command, Duration::from_secs(30), &cancelled).unwrap();
        assert!(result.timed_out);
        assert!(started.elapsed() < Duration::from_secs(5));
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
    fn pi_version_floor_is_1_5_0() {
        assert!(pi_version_supported("1.5.0"));
        assert!(pi_version_supported("v1.6.2"));
        assert!(!pi_version_supported("1.4.9"));
        assert!(!pi_version_supported("garbage"));
    }

    #[test]
    fn missing_explicit_path_does_not_fall_back() {
        let missing = PathBuf::from(r"C:\definitely\not\here\pi.cmd");
        assert_eq!(locate_pi(Some(&missing)), None);
    }

    #[test]
    fn relative_or_non_pi_paths_are_rejected() {
        assert_eq!(locate_pi(Some(Path::new("pi.cmd"))), None);
        assert_eq!(locate_pi(Some(Path::new("notepad.exe"))), None);
        assert!(!is_plausible_launcher(Path::new("pi.cmd")));
    }

    #[test]
    fn capture_output_is_capped() {
        let payload = vec![b'x'; MAX_CAPTURE_BYTES + 1024];
        let mut input = Cursor::new(payload);
        let (text, truncated) = read_limited(Some(&mut input), MAX_CAPTURE_BYTES);
        assert!(truncated);
        assert_eq!(text.len(), MAX_CAPTURE_BYTES);
        assert_eq!(input.position(), (MAX_CAPTURE_BYTES + 1024) as u64);
    }
}
