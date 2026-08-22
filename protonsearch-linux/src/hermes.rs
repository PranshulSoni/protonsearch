//! Small Linux-side Hermes Agent integration primitives.
//!
//! This module intentionally does not own any GTK objects.  Prompt execution
//! happens on a worker thread and emits line-oriented events through a caller
//! supplied callback.  A GTK caller can forward those events to its main
//! context with a channel without blocking the launcher.

use std::ffi::OsString;
use std::io::{self, BufRead, BufReader};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const DEFAULT_GATEWAY_HOST: &str = "127.0.0.1";
const DEFAULT_GATEWAY_PORT: u16 = 8642;
const DEFAULT_GATEWAY_BASE: &str = "http://127.0.0.1:8642";
const GATEWAY_SERVICE: &str = "hermes-gateway.service";
const PROBE_TIMEOUT: Duration = Duration::from_millis(350);
const CLI_TIMEOUT: Duration = Duration::from_secs(90);

/// The executable used by the installed Hermes distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HermesCommand {
    Hermes,
    HermesAgent,
}

impl HermesCommand {
    pub const fn program(self) -> &'static str {
        match self {
            Self::Hermes => "hermes",
            Self::HermesAgent => "hermes-agent",
        }
    }
}

/// Selects the modern command first, while retaining compatibility with older
/// Hermes installations that only expose `hermes-agent`.
pub fn select_command(available: impl Fn(&str) -> bool) -> Option<HermesCommand> {
    if available(HermesCommand::Hermes.program()) {
        Some(HermesCommand::Hermes)
    } else if available(HermesCommand::HermesAgent.program()) {
        Some(HermesCommand::HermesAgent)
    } else {
        None
    }
}

/// The state of the local Hermes gateway endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayState {
    Running,
    Unavailable,
}

/// Secret-free Hermes diagnostics suitable for displaying in settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HermesReadiness {
    pub command: Option<HermesCommand>,
    pub executable: Option<PathBuf>,
    pub version: Option<String>,
    pub gateway: GatewayState,
    /// True only when both the CLI and the local gateway are available.
    pub ready: bool,
}

impl HermesReadiness {
    pub fn unavailable() -> Self {
        Self {
            command: None,
            executable: None,
            version: None,
            gateway: GatewayState::Unavailable,
            ready: false,
        }
    }
}

/// Probe the normal local Hermes gateway endpoint and the installed CLI.
///
/// Only executable names, a bounded version line, and TCP reachability are
/// returned.  No environment variables, command arguments, tokens, or config
/// file contents are read or exposed.
pub fn readiness() -> HermesReadiness {
    let Some(command) = select_command(command_available) else {
        return HermesReadiness::unavailable();
    };
    let executable = command_path(command.program());
    let version = executable
        .as_deref()
        .and_then(|path| read_version(path).ok())
        .and_then(|output| parse_version(&output));
    let gateway = if gateway_healthy() {
        GatewayState::Running
    } else {
        GatewayState::Unavailable
    };
    HermesReadiness {
        command: Some(command),
        executable,
        version,
        gateway,
        ready: gateway == GatewayState::Running,
    }
}

/// Start the existing per-user gateway if it is not already reachable.
///
/// The systemd unit is preferred because it keeps the gateway independent of
/// the launcher process.  If systemd is unavailable or cannot start the unit,
/// the installed Hermes CLI is launched in the background with its supported
/// `gateway start` command.  The returned value only describes what was
/// attempted; callers should probe [`readiness`] again after a short delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayStart {
    AlreadyRunning,
    StartedBySystemd,
    StartedByHermes,
    NotAvailable,
}

pub fn ensure_gateway_started() -> io::Result<GatewayStart> {
    if gateway_reachable(DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT) {
        return Ok(GatewayStart::AlreadyRunning);
    }

    if command_available("systemctl") {
        let result = Command::new("systemctl")
            .args(["--user", "start", GATEWAY_SERVICE])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if matches!(result, Ok(status) if status.success()) {
            return Ok(GatewayStart::StartedBySystemd);
        }
    }

    let Some(command) = select_command(command_available) else {
        return Ok(GatewayStart::NotAvailable);
    };
    Command::new(command.program())
        .args(["gateway", "start"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(GatewayStart::StartedByHermes)
}

/// Events emitted by [`run_prompt_async`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HermesEvent {
    /// A complete line from Hermes stdout.
    Output(String),
    /// A gateway token/delta that should be appended without inserting a line break.
    Delta(String),
    /// A complete diagnostic line from Hermes stderr.
    Diagnostic(String),
    /// A tool is waiting for user approval in the gateway.
    Approval { tool: String, summary: String },
    /// Emitted once after both streams close and the process has been waited.
    Finished(HermesExit),
}

/// Process completion details that do not contain command output or secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HermesExit {
    pub status: Option<i32>,
    pub success: bool,
}

/// Run one Hermes prompt without blocking the caller.
///
/// The callback is invoked from the returned worker thread, never from the
/// caller's thread.  It is therefore safe for GTK to have the callback send
/// [`HermesEvent`] values to a `glib::MainContext` channel.  The returned join
/// handle can be retained by the owning view until completion.
pub fn run_prompt_async<F>(
    prompt: impl Into<String>,
    session: Option<&str>,
    callback: F,
) -> io::Result<JoinHandle<()>>
where
    F: FnMut(HermesEvent) + Send + 'static,
{
    let command = select_command(command_available)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Hermes Agent is not installed"))?;
    let prompt = prompt.into();
    if prompt.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Hermes prompt must not be empty",
        ));
    }

    let mut args = Vec::with_capacity(4);
    if let Some(session) = session.filter(|value| !value.trim().is_empty()) {
        args.push(OsString::from("--resume"));
        args.push(OsString::from(session));
    }
    args.push(OsString::from("-z"));
    args.push(OsString::from(prompt));

    let mut child = Command::new(command.program())
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("Hermes stdout pipe was unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("Hermes stderr pipe was unavailable"))?;

    let (events_tx, events_rx) = mpsc::channel::<HermesEvent>();
    let stdout_tx = events_tx.clone();
    let stdout_reader = thread::spawn(move || read_lines(stdout, stdout_tx, false));
    let stderr_reader = thread::spawn(move || read_lines(stderr, events_tx, true));
    let finished = Arc::new(AtomicBool::new(false));
    let finished_for_watchdog = finished.clone();
    let pid = child.id();
    thread::spawn(move || {
        thread::sleep(CLI_TIMEOUT);
        if !finished_for_watchdog.load(Ordering::Acquire) {
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGTERM);
            }
        }
    });

    Ok(thread::spawn(move || {
        let mut callback = callback;
        for event in events_rx {
            callback(event);
        }
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
        finished.store(true, Ordering::Release);
        let status = child.wait().ok().and_then(|status| status.code());
        callback(HermesEvent::Finished(HermesExit {
            status,
            success: status == Some(0),
        }));
    }))
}

/// Prefer the authenticated local Runs API when it is available, retaining
/// the CLI path for installations that have Hermes configured without its API
/// server. Both paths emit the same worker-thread events to the GTK layer.
pub fn run_prompt_best_effort_async<F>(
    prompt: impl Into<String>,
    session: Option<&str>,
    callback: F,
) -> io::Result<JoinHandle<()>>
where
    F: FnMut(HermesEvent) + Send + 'static,
{
    if gateway_reachable(DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT) {
        return run_gateway_prompt_async(prompt.into(), session, callback);
    }
    run_prompt_async(prompt, session, callback)
}

fn run_gateway_prompt_async<F>(
    prompt: String,
    session: Option<&str>,
    callback: F,
) -> io::Result<JoinHandle<()>>
where
    F: FnMut(HermesEvent) + Send + 'static,
{
    let session = session.map(str::to_string);
    Ok(thread::spawn(move || {
        let mut callback = callback;
        let result = run_gateway_prompt(&prompt, session.as_deref(), &mut callback);
        if let Err(error) = result {
            callback(HermesEvent::Diagnostic(error.to_string()));
            callback(HermesEvent::Finished(HermesExit {
                status: None,
                success: false,
            }));
        } else {
            callback(HermesEvent::Finished(HermesExit {
                status: Some(0),
                success: true,
            }));
        }
    }))
}

fn run_gateway_prompt(
    prompt: &str,
    session: Option<&str>,
    callback: &mut impl FnMut(HermesEvent),
) -> io::Result<()> {
    let key = std::env::var("HERMES_API_KEY")
        .or_else(|_| std::env::var("API_SERVER_KEY"))
        .unwrap_or_else(|_| "hermes".to_string());
    let mut body = serde_json::json!({
        "input": prompt,
        "stream": true,
    });
    if let Some(session) = session.filter(|value| !value.trim().is_empty()) {
        body["session_id"] = serde_json::Value::String(session.to_string());
    }
    let response = ureq::post(&format!("{DEFAULT_GATEWAY_BASE}/v1/runs"))
        .header("Authorization", &format!("Bearer {key}"))
        .header("Content-Type", "application/json")
        .config()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .send_json(body)
        .map_err(|error| io::Error::other(format!("Hermes gateway start failed: {error}")))?;
    let response_text = response
        .into_body()
        .read_to_string()
        .map_err(|error| io::Error::other(format!("invalid Hermes gateway response: {error}")))?;
    let payload: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|error| io::Error::other(format!("invalid Hermes run response: {error}")))?;
    let run_id = payload
        .get("run_id")
        .or_else(|| payload.get("id"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::other("Hermes gateway did not return a run id"))?;
    let stream = ureq::get(&format!("{DEFAULT_GATEWAY_BASE}/v1/runs/{run_id}/events"))
        .header("Authorization", &format!("Bearer {key}"))
        .header("Accept", "text/event-stream")
        .config()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .call()
        .map_err(|error| io::Error::other(format!("Hermes event stream failed: {error}")))?;
    let mut event_name = String::new();
    for line in BufReader::new(stream.into_body().into_reader()).lines() {
        let line = line?;
        if let Some(name) = line.strip_prefix("event:").map(str::trim) {
            event_name.clear();
            event_name.push_str(name);
            continue;
        }
        let Some(data) = line.strip_prefix("data:").map(str::trim) else {
            continue;
        };
        if data == "[DONE]" {
            break;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(data) else {
            continue;
        };
        let kind = if event_name.is_empty() {
            event
                .get("type")
                .or_else(|| event.get("event"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
        } else {
            event_name.as_str()
        };
        if [
            "approval_required",
            "approval.requested",
            "approval.request",
            "permission_request",
        ]
        .contains(&kind)
        {
            callback(HermesEvent::Approval {
                tool: event
                    .get("tool")
                    .or_else(|| event.get("tool_name"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Hermes tool")
                    .to_string(),
                summary: event
                    .get("summary")
                    .or_else(|| event.get("message"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Hermes is waiting for approval")
                    .to_string(),
            });
            continue;
        }
        if matches!(kind, "failed" | "error" | "run.failed") {
            let message = event
                .get("message")
                .or_else(|| event.get("error"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Hermes run failed");
            return Err(io::Error::other(message.to_string()));
        }
        let fragment = event
            .get("delta")
            .or_else(|| event.get("text"))
            .or_else(|| event.get("output"))
            .and_then(serde_json::Value::as_str);
        if let Some(fragment) = fragment.filter(|value| !value.is_empty()) {
            callback(HermesEvent::Delta(fragment.to_string()));
        }
    }
    Ok(())
}

fn read_lines<R: io::Read + Send + 'static>(
    reader: R,
    sender: mpsc::Sender<HermesEvent>,
    diagnostic: bool,
) {
    for line in BufReader::new(reader).lines() {
        let Ok(line) = line else { break };
        let event = if diagnostic {
            HermesEvent::Diagnostic(line)
        } else {
            HermesEvent::Output(line)
        };
        if sender.send(event).is_err() {
            break;
        }
    }
}

fn command_available(program: &str) -> bool {
    command_path(program).is_some()
}

fn command_path(program: &str) -> Option<PathBuf> {
    if program.contains('/') {
        let path = PathBuf::from(program);
        return is_executable(&path).then_some(path);
    }
    std::env::var_os("PATH")?
        .to_string_lossy()
        .split(':')
        .map(Path::new)
        .map(|directory| directory.join(program))
        .find(|path| is_executable(path))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn read_version(program: &Path) -> io::Result<String> {
    let output = Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        Ok(stdout.into_owned())
    } else {
        Ok(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

/// Keep only a bounded, human-readable version line; never surface arbitrary
/// command output in the readiness UI.
pub fn parse_version(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("hermes") || lower.contains("version")
        })
        .map(|line| line.chars().take(160).collect())
}

fn gateway_reachable(host: &str, port: u16) -> bool {
    let address = format!("{host}:{port}");
    let Ok(addresses) = address.to_socket_addrs() else {
        return false;
    };
    addresses
        .filter_map(|address| match address {
            SocketAddr::V4(_) | SocketAddr::V6(_) => Some(address),
        })
        .any(|address| TcpStream::connect_timeout(&address, PROBE_TIMEOUT).is_ok())
}

fn gateway_healthy() -> bool {
    if !gateway_reachable(DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT) {
        return false;
    }
    let key = std::env::var("HERMES_API_KEY")
        .or_else(|_| std::env::var("API_SERVER_KEY"))
        .unwrap_or_else(|_| "hermes".to_string());
    ureq::get(&format!("{DEFAULT_GATEWAY_BASE}/health"))
        .header("Authorization", &format!("Bearer {key}"))
        .config()
        .timeout_global(Some(PROBE_TIMEOUT))
        .build()
        .call()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_modern_hermes_command() {
        assert_eq!(
            select_command(|name| name == "hermes" || name == "hermes-agent"),
            Some(HermesCommand::Hermes)
        );
    }

    #[test]
    fn falls_back_to_legacy_hermes_agent_command() {
        assert_eq!(
            select_command(|name| name == "hermes-agent"),
            Some(HermesCommand::HermesAgent)
        );
    }

    #[test]
    fn reports_no_command_when_neither_is_available() {
        assert_eq!(select_command(|_| false), None);
    }

    #[test]
    fn parses_only_a_bounded_version_line() {
        let output = "Hermes Agent v0.19.1\nprovider_token=should-not-be-shown\n";
        assert_eq!(
            parse_version(output),
            Some("Hermes Agent v0.19.1".to_string())
        );
    }

    #[test]
    fn ignores_unrelated_command_output() {
        assert_eq!(parse_version("provider unavailable\nno details\n"), None);
    }

    #[test]
    fn unavailable_readiness_is_not_ready() {
        let readiness = HermesReadiness::unavailable();
        assert!(!readiness.ready);
        assert_eq!(readiness.gateway, GatewayState::Unavailable);
        assert_eq!(readiness.command, None);
    }
}
