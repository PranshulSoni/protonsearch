//! Process lifecycle for the Python agent sidecar.
//!
//! Threading model:
//! * caller thread: spawn/send/cancel/approve/shutdown (all non-blocking except
//!   shutdown, which waits up to the grace period for a clean EOF exit);
//! * stdout reader thread: JSONL framing, event parsing, crash detection;
//! * stderr drain thread: bounded tail, secret redaction; never forwarded to
//!   the app's stdout;
//! * watchdog thread: kills the sidecar after the configured idle timeout when
//!   no run is active.
//!
//! Event callbacks run on the stdout reader thread. They must be fast and
//! thread-safe (a Tauri AppHandle.emit call is); never block on the client's
//! mutexes from inside the callback.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ps_ipc::{AgentApprovalResponse, AgentEvent, AgentRunRequest};

use crate::config::SidecarConfig;
use crate::error::AgentError;
use crate::protocol::{self, MAX_LINE_BYTES};

/// Event sink used by the client. Called from a background thread.
pub type EventCallback = Arc<dyn Fn(&AgentEvent) + Send + Sync + 'static>;

/// At most one automatic restart after an unexpected sidecar exit.
pub const MAX_RESTARTS: u32 = 1;
/// How long a crash-restart or an explicit shutdown may take before a kill.
pub const SHUTDOWN_GRACE: Duration = Duration::from_millis(2000);
const STDERR_TAIL_LINES: usize = 200;
const WATCHDOG_TICK: Duration = Duration::from_millis(250);
const DEFAULT_WATCHDOG_TICK: Duration = WATCHDOG_TICK;

struct ActiveRun {
    run_id: String,
    started: Instant,
}

#[derive(Default)]
struct State {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    active_run: Option<ActiveRun>,
    stderr_tail: VecDeque<String>,
}

struct Shared {
    config: SidecarConfig,
    on_event: EventCallback,
    redactions: Vec<String>,
    state: Mutex<State>,
    activity: Mutex<Instant>,
    shutdown: AtomicBool,
    restart_count: AtomicU32,
    child_alive: AtomicBool,
}

impl Shared {
    fn emit(&self, event: AgentEvent) {
        (self.on_event)(&event);
    }

    fn touch(&self) {
        if let Ok(mut activity) = self.activity.lock() {
            *activity = Instant::now();
        }
    }

    fn stderr_tail_text(&self) -> String {
        let state = self.state.lock().unwrap_or_else(|poison| poison.into_inner());
        state
            .stderr_tail
            .iter()
            .rev()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

/// A handle to one sidecar process (plus its restart on crash).
///
/// Create one client per agent workflow session; after an idle-timeout kill or
/// an explicit shutdown the client is dead and should be dropped. Look for a
/// new client via crate::config::default_config when the user starts another
/// workflow.
pub struct AgentClient {
    shared: Arc<Shared>,
}

impl AgentClient {
    /// Spawn the sidecar. Fails with AgentError::NotAvailable when the venv
    /// interpreter or package entry point is missing; never falls back to a
    /// system Python.
    pub fn spawn<F>(config: SidecarConfig, on_event: F) -> Result<Self, AgentError>
    where
        F: Fn(&AgentEvent) + Send + Sync + 'static,
    {
        Self::spawn_with_callback(config, Arc::new(on_event))
    }

    pub fn spawn_with_callback(config: SidecarConfig, on_event: EventCallback) -> Result<Self, AgentError> {
        if !crate::config::runtime_available_for(&config) {
            return Err(AgentError::NotAvailable);
        }
        let shared = Arc::new(Shared {
            redactions: collect_redactions(&config),
            config,
            on_event,
            state: Mutex::new(State::default()),
            activity: Mutex::new(Instant::now()),
            shutdown: AtomicBool::new(false),
            restart_count: AtomicU32::new(0),
            child_alive: AtomicBool::new(false),
        });
        let child = spawn_child(&shared)?;
        attach_child(&shared, child)?;
        spawn_watchdog(&shared);
        Ok(Self { shared })
    }

    /// Spawn and receive events on a std channel (convenience for tests and for
    /// callers that prefer a queue over a callback).
    pub fn spawn_channel(config: SidecarConfig) -> Result<(Self, std::sync::mpsc::Receiver<AgentEvent>), AgentError> {
        let (tx, rx) = std::sync::mpsc::channel();
        let sender = Mutex::new(tx);
        let callback: EventCallback = Arc::new(move |event: &AgentEvent| {
            if let Ok(sender) = sender.lock() {
                let _ = sender.send(event.clone());
            }
        });
        let client = Self::spawn_with_callback(config, callback)?;
        Ok((client, rx))
    }

    /// Send a run request. The event stream (started/plan/...) arrives through
    /// the callback; one run is active at a time.
    pub fn send_run(&self, request: &AgentRunRequest) -> Result<(), AgentError> {
        let frame = protocol::encode_run_request(request)?;
        {
            let mut state = self.lock_state();
            let stdin = state.stdin.as_mut().ok_or(AgentError::NotRunning)?;
            stdin.write_all(&frame).map_err(AgentError::Io)?;
            stdin.flush().map_err(AgentError::Io)?;
            state.active_run = Some(ActiveRun {
                run_id: request.run_id.clone(),
                started: Instant::now(),
            });
        }
        self.shared.touch();
        Ok(())
    }

    /// Approve or deny the current pending approval request.
    pub fn approve(&self, response: &AgentApprovalResponse) -> Result<(), AgentError> {
        let frame = protocol::encode_approval_response(response)?;
        self.write_control(&frame)?;
        Ok(())
    }

    /// Ask the sidecar to cancel a run. Passing None cancels the active run.
    pub fn cancel(&self, run_id: Option<&str>) -> Result<(), AgentError> {
        let run_id = match run_id {
            Some(id) => id.to_string(),
            None => {
                let state = self.lock_state();
                match &state.active_run {
                    Some(active) => active.run_id.clone(),
                    None => return Ok(()),
                }
            }
        };
        let frame = protocol::encode_cancel(&run_id)?;
        self.write_control(&frame)?;
        Ok(())
    }

    fn write_control(&self, frame: &[u8]) -> Result<(), AgentError> {
        {
            let mut state = self.lock_state();
            let stdin = state.stdin.as_mut().ok_or(AgentError::NotRunning)?;
            stdin.write_all(frame).map_err(AgentError::Io)?;
            stdin.flush().map_err(AgentError::Io)?;
        }
        self.shared.touch();
        Ok(())
    }

    /// True while the sidecar process is alive.
    pub fn is_alive(&self) -> bool {
        if !self.shared.child_alive.load(Ordering::SeqCst) {
            return false;
        }
        let mut state = self.lock_state();
        match state.child.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    pub fn pid(&self) -> Option<u32> {
        let state = self.lock_state();
        state.child.as_ref().map(Child::id)
    }

    /// Number of automatic crash restarts performed so far (0 or 1).
    pub fn restart_count(&self) -> u32 {
        self.shared.restart_count.load(Ordering::SeqCst)
    }

    /// Bounded, redacted tail of the sidecar stderr.
    pub fn stderr_tail(&self, max_lines: usize) -> Vec<String> {
        let state = self.lock_state();
        state
            .stderr_tail
            .iter()
            .rev()
            .take(max_lines)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Graceful shutdown: close stdin (EOF), wait briefly, then kill.
    /// Idempotent; also called from Drop.
    pub fn shutdown(&self) {
        if self.shared.shutdown.swap(true, Ordering::SeqCst) {
            return;
        }
        stop_child(&self.shared, SHUTDOWN_GRACE);
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, State> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

impl Drop for AgentClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn collect_redactions(config: &SidecarConfig) -> Vec<String> {
    let mut values: Vec<String> = Vec::new();
    for (_, value) in &config.extra_env {
        if value.len() >= 6 {
            values.push(value.clone());
        }
    }
    for name in [
        "PROTONSEARCH_AGENT_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GOOGLE_API_KEY",
        "GEMINI_API_KEY",
        "AZURE_OPENAI_API_KEY",
    ] {
        if let Ok(value) = std::env::var(name) {
            if value.len() >= 6 {
                values.push(value);
            }
        }
    }
    values
}

fn spawn_child(shared: &Arc<Shared>) -> Result<Child, AgentError> {
    let config = &shared.config;
    let mut command = Command::new(&config.python);
    command
        .arg("-u")
        .arg("-m")
        .arg("protonsearch_agent")
        .current_dir(&config.sidecar_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PROTONSEARCH_AGENT_TEST_MODEL", if config.test_model { "1" } else { "0" })
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONIOENCODING", "utf-8");
    if let Some(model) = &config.model {
        command.env("PROTONSEARCH_AGENT_MODEL", model);
    }
    for (key, value) in &config.extra_env {
        command.env(key, value);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn().map_err(AgentError::Spawn)
}

fn attach_child(shared: &Arc<Shared>, mut child: Child) -> Result<(), AgentError> {
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AgentError::spawn_io("sidecar stdin was not piped"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AgentError::spawn_io("sidecar stdout was not piped"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AgentError::spawn_io("sidecar stderr was not piped"))?;
    {
        let mut state = shared.state.lock().unwrap_or_else(|poison| poison.into_inner());
        state.child = Some(child);
        state.stdin = Some(stdin);
    }
    shared.child_alive.store(true, Ordering::SeqCst);
    shared.touch();

    let reader_shared = Arc::clone(shared);
    thread::Builder::new()
        .name("ps-agent-stdout".into())
        .spawn(move || read_events(reader_shared, stdout))
        .map_err(AgentError::Spawn)?;
    let stderr_shared = Arc::clone(shared);
    thread::Builder::new()
        .name("ps-agent-stderr".into())
        .spawn(move || drain_stderr(stderr_shared, stderr))
        .map_err(AgentError::Spawn)?;
    Ok(())
}

fn read_events(shared: Arc<Shared>, stdout: ChildStdout) {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(read) => {
                if read > MAX_LINE_BYTES {
                    shared.emit(AgentEvent::Error {
                        code: "protocol_line_too_long".into(),
                        message: format!("sidecar sent a {read}-byte line; the limit is {MAX_LINE_BYTES}"),
                    });
                    stop_child(&shared, Duration::ZERO);
                    break;
                }
                if line.trim().is_empty() {
                    continue;
                }
                shared.touch();
                match protocol::parse_event(&line) {
                    Ok(event) => {
                        if matches!(event, AgentEvent::Finished { .. }) {
                            let mut state = shared
                                .state
                                .lock()
                                .unwrap_or_else(|poison| poison.into_inner());
                            state.active_run = None;
                        }
                        shared.emit(event);
                    }
                    Err(err) => {
                        let preview = protocol::redact_and_truncate(&line, &shared.redactions, 200);
                        shared.emit(AgentEvent::Error {
                            code: "protocol_parse_error".into(),
                            message: format!("{err}: {preview}"),
                        });
                    }
                }
            }
            Err(err) => {
                shared.emit(AgentEvent::Error {
                    code: "sidecar_read_failed".into(),
                    message: err.to_string(),
                });
                break;
            }
        }
    }
    handle_exit(shared);
}

fn drain_stderr(shared: Arc<Shared>, stderr: ChildStderr) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let redacted = protocol::redact_and_truncate(&line, &shared.redactions, 2000);
                if redacted.trim().is_empty() {
                    continue;
                }
                let mut state = shared.state.lock().unwrap_or_else(|poison| poison.into_inner());
                state.stderr_tail.push_back(redacted);
                while state.stderr_tail.len() > STDERR_TAIL_LINES {
                    state.stderr_tail.pop_front();
                }
            }
            Err(_) => break,
        }
    }
}

fn handle_exit(shared: Arc<Shared>) {
    if shared.shutdown.load(Ordering::SeqCst) {
        return;
    }
    shared.child_alive.store(false, Ordering::SeqCst);
    let (active, status_text) = {
        let mut state = shared.state.lock().unwrap_or_else(|poison| poison.into_inner());
        state.stdin = None;
        let status = state
            .child
            .as_mut()
            .and_then(|child| child.try_wait().ok().flatten())
            .map(|status| format!(" (exit {status})"))
            .unwrap_or_default();
        (state.active_run.take(), status)
    };
    let stderr = shared.stderr_tail_text();
    let stderr = if stderr.is_empty() {
        String::new()
    } else {
        format!("; stderr: {stderr}")
    };
    shared.emit(AgentEvent::Error {
        code: "sidecar_crashed".into(),
        message: format!("agent sidecar exited unexpectedly{status_text}{stderr}"),
    });
    if let Some(active) = active {
        let duration_ms = active.started.elapsed().as_millis() as u64;
        shared.emit(AgentEvent::Finished {
            ok: false,
            duration_ms,
        });
        let _ = active.run_id;
    }

    let previous_restarts = shared.restart_count.fetch_add(1, Ordering::SeqCst);
    if previous_restarts < MAX_RESTARTS {
        match spawn_child(&shared).and_then(|child| attach_child(&shared, child)) {
            Ok(()) => {
                let message = if stderr.is_empty() {
                    "agent sidecar exited unexpectedly; restarted once".to_string()
                } else {
                    format!("agent sidecar exited unexpectedly; restarted once{stderr}")
                };
                shared.emit(AgentEvent::Error {
                    code: "sidecar_restarted".into(),
                    message,
                });
            }
            Err(err) => {
                shared.emit(AgentEvent::Error {
                    code: "sidecar_unavailable".into(),
                    message: format!("restart failed: {err}"),
                });
            }
        }
    } else {
        shared.emit(AgentEvent::Error {
            code: "sidecar_unavailable".into(),
            message: "agent sidecar exited and the restart limit was reached".to_string(),
        });
    }
}

fn spawn_watchdog(shared: &Arc<Shared>) {
    let watchdog_shared = Arc::clone(shared);
    let _ = thread::Builder::new()
        .name("ps-agent-watchdog".into())
        .spawn(move || loop {
            thread::sleep(DEFAULT_WATCHDOG_TICK);
            if watchdog_shared.shutdown.load(Ordering::SeqCst) {
                return;
            }
            let inactive = {
                let state = watchdog_shared
                    .state
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                state.active_run.is_none()
            };
            if !inactive || !watchdog_shared.child_alive.load(Ordering::SeqCst) {
                continue;
            }
            let idle = watchdog_shared
                .activity
                .lock()
                .map(|activity| activity.elapsed())
                .unwrap_or_default();
            if idle >= watchdog_shared.config.idle_timeout {
                // Idle kill is terminal for this client: set shutdown so the
                // reader thread reports clean EOF instead of restarting.
                watchdog_shared.shutdown.store(true, Ordering::SeqCst);
                stop_child(&watchdog_shared, SHUTDOWN_GRACE);
                return;
            }
        });
}

fn stop_child(shared: &Arc<Shared>, grace: Duration) {
    let child = {
        let mut state = shared.state.lock().unwrap_or_else(|poison| poison.into_inner());
        state.stdin = None; // EOF: the sidecar exits cleanly on stdin close
        state.child.take()
    };
    if let Some(mut child) = child {
        let deadline = Instant::now() + grace;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
    }
    shared.child_alive.store(false, Ordering::SeqCst);
}

