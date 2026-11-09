//! Offline end-to-end tests for the Pi backend.
//!
//! They use the `ps_agent_fake_pi` helper binary (never a model, credential or
//! network) through a generated `.cmd` launcher placed in a directory with a
//! space, so the Windows cmd.exe wrapper and argument quoting are exercised too.
//! The real `pi --version` probe runs only when Pi is installed; it skips with a
//! clear message otherwise.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ps_agent::pi::pid_alive;
use ps_agent::{probe_pi, AgentEvent, AgentRunRequest, PiAgentClient, PiAgentConfig, PiInfo};

fn fake_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ps_agent_fake_pi"))
}

fn case_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join(format!("pi e2e {tag}-{nanos}"));
    fs::create_dir_all(&root).expect("create test root");
    root
}

/// Generate a Windows batch launcher in a directory whose name contains a
/// space, mirroring the managed `~/.pi/agent/bin/pi.cmd` shape.
fn make_cmd_wrapper(root: &Path) -> PathBuf {
    let dir = root.join("bin with space");
    fs::create_dir_all(&dir).expect("create wrapper dir");
    let launcher = dir.join("pi.cmd");
    let body = format!("@echo off\r\n\"{}\" %*\r\n", fake_bin().display());
    fs::write(&launcher, body).expect("write wrapper");
    launcher
}

fn test_config(
    root: &Path,
    mode: &str,
    allow_writes: bool,
    idle_timeout: Duration,
    max_run_duration: Duration,
    use_wrapper: bool,
) -> PiAgentConfig {
    let workspace = root.join("workspace with space");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(workspace.join("ps_fake_pi_mode"), mode).expect("write fake mode");
    let pi_path = if use_wrapper {
        make_cmd_wrapper(root)
    } else {
        fake_bin()
    };
    PiAgentConfig {
        pi: PiInfo {
            path: pi_path,
            version: "1.5.0-fake".to_string(),
            node_ok: true,
        },
        workspace,
        data_dir: root.join("data"),
        allow_writes,
        model: Some("fake-model".to_string()),
        provider: Some("fake-provider".to_string()),
        thinking: None,
        idle_timeout,
        max_run_duration,
        extra_env: Vec::new(),
    }
}

fn run_request(run_id: &str) -> AgentRunRequest {
    AgentRunRequest {
        run_id: run_id.to_string(),
        prompt: "find the quarterly budget file".to_string(),
        context_paths: vec!["C:/Users/Public/budget.txt".to_string()],
        model: None,
        max_steps: Some(8),
    }
}

fn collect_until_finished(receiver: &Receiver<AgentEvent>) -> Vec<AgentEvent> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut events = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            panic!("timed out waiting for Finished; events so far: {events:?}");
        }
        match receiver.recv_timeout(remaining) {
            Ok(event) => {
                let finished = matches!(event, AgentEvent::Finished { .. });
                events.push(event);
                if finished {
                    return events;
                }
            }
            Err(error) => panic!("pi event stream ended unexpectedly: {error}"),
        }
    }
}

fn has_error(events: &[AgentEvent], code: &str) -> bool {
    events
        .iter()
        .any(|event| matches!(event, AgentEvent::Error { code: actual, .. } if actual == code))
}

#[test]
fn happy_path_maps_pi_stream_to_agent_events() {
    let root = case_root("happy");
    let config = test_config(
        &root,
        "happy",
        false,
        Duration::from_secs(30),
        Duration::from_secs(120),
        true,
    );
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    assert!(!client.is_running());
    client.send_run(&run_request("pi-run-1")).expect("send run");
    let events = collect_until_finished(&receiver);

    let started = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::Started {
                run_id,
                runtime,
                model,
            } => Some((run_id.clone(), runtime.clone(), model.clone())),
            _ => None,
        })
        .expect("Started event");
    assert_eq!(started.0, "pi-run-1");
    assert_eq!(started.1, "pi 1.5.0-fake");
    assert_eq!(started.2, "fake-model");

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AgentEvent::Progress { message, .. } if message == "Fake ")),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AgentEvent::Progress { message, .. } if message == "tool: read")),
        "{events:?}"
    );
    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::ToolResult { tool, ok: true, preview } if tool == "read" && preview.contains("tool read completed")
    )), "{events:?}");
    let output = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::Output { text, final_answer } => Some((text.clone(), *final_answer)),
            _ => None,
        })
        .expect("Output event");
    assert!(output.1);
    assert!(
        output.0.contains("find the quarterly budget file"),
        "{}",
        output.0
    );
    assert!(
        matches!(events.last(), Some(AgentEvent::Finished { ok: true, .. })),
        "{events:?}"
    );
    assert!(!client.is_running());
}

#[test]
fn direct_executable_launch_is_supported() {
    let root = case_root("direct");
    let config = test_config(
        &root,
        "happy",
        false,
        Duration::from_secs(30),
        Duration::from_secs(120),
        false,
    );
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-direct")).expect("send run");
    let events = collect_until_finished(&receiver);
    assert!(
        matches!(events.last(), Some(AgentEvent::Finished { ok: true, .. })),
        "{events:?}"
    );
}

#[test]
fn busy_client_rejects_a_second_run_and_cancel_finishes_cleanly() {
    let root = case_root("busy");
    let config = test_config(
        &root,
        "idle",
        false,
        Duration::from_secs(30),
        Duration::from_secs(120),
        true,
    );
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-busy")).expect("first run");
    let second = client.send_run(&run_request("pi-busy-2"));
    assert!(matches!(second, Err(ps_agent::AgentError::Busy)), "{second:?}");
    assert!(client.is_running());
    client.cancel().expect("cancel");
    let events = collect_until_finished(&receiver);
    assert!(has_error(&events, "cancelled"), "{events:?}");
    assert!(
        matches!(events.last(), Some(AgentEvent::Finished { ok: false, .. })),
        "{events:?}"
    );
    assert!(!client.is_running());
}

#[cfg(windows)]
#[test]
fn cancel_kills_the_whole_process_tree_within_three_seconds() {
    let root = case_root("tree");
    let config = test_config(
        &root,
        "tree",
        false,
        Duration::from_secs(60),
        Duration::from_secs(300),
        true,
    );
    let workspace = config.workspace.clone();
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-tree")).expect("send run");

    let pid_file = workspace.join("ps_fake_pi_child.pid");
    let deadline = Instant::now() + Duration::from_secs(15);
    while !pid_file.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let child_pid: u32 = fs::read_to_string(&pid_file)
        .expect("fake grandchild wrote its pid")
        .trim()
        .parse()
        .expect("child pid");
    assert!(pid_alive(child_pid), "grandchild should be running before cancel");

    let cancel_started = Instant::now();
    client.cancel().expect("cancel");
    let tree_deadline = cancel_started + Duration::from_secs(3);
    while pid_alive(child_pid) && Instant::now() < tree_deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        !pid_alive(child_pid),
        "taskkill /T must kill node/bash grandchildren within 3 s"
    );
    let events = collect_until_finished(&receiver);
    assert!(has_error(&events, "cancelled"), "{events:?}");
    assert!(
        matches!(events.last(), Some(AgentEvent::Finished { ok: false, .. })),
        "{events:?}"
    );
}

#[test]
fn idle_timeout_kills_a_silent_run() {
    let root = case_root("idle");
    let config = test_config(
        &root,
        "idle",
        false,
        Duration::from_millis(300),
        Duration::from_secs(300),
        true,
    );
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-idle")).expect("send run");
    let events = collect_until_finished(&receiver);
    assert!(has_error(&events, "idle_timeout"), "{events:?}");
    assert!(
        matches!(events.last(), Some(AgentEvent::Finished { ok: false, .. })),
        "{events:?}"
    );
    assert!(!client.is_running());
}

#[test]
fn oversized_jsonl_line_is_reported_and_aborts_the_run() {
    let root = case_root("oversize");
    let config = test_config(
        &root,
        "oversize",
        false,
        Duration::from_secs(30),
        Duration::from_secs(300),
        true,
    );
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-oversize")).expect("send run");
    let events = collect_until_finished(&receiver);
    assert!(has_error(&events, "protocol_line_too_long"), "{events:?}");
    assert!(
        matches!(events.last(), Some(AgentEvent::Finished { ok: false, .. })),
        "{events:?}"
    );
}

#[test]
fn malformed_jsonl_line_is_reported_and_the_stream_continues() {
    let root = case_root("malformed");
    let config = test_config(
        &root,
        "malformed",
        false,
        Duration::from_secs(30),
        Duration::from_secs(300),
        true,
    );
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-malformed")).expect("send run");
    let events = collect_until_finished(&receiver);
    assert!(has_error(&events, "protocol_parse_error"), "{events:?}");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AgentEvent::Output { .. })),
        "{events:?}"
    );
    assert!(
        matches!(events.last(), Some(AgentEvent::Finished { ok: true, .. })),
        "{events:?}"
    );
}

#[test]
fn stderr_and_tool_previews_are_redacted() {
    let root = case_root("redact");
    let config = test_config(
        &root,
        "stderr_secret",
        false,
        Duration::from_secs(30),
        Duration::from_secs(300),
        true,
    );
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-redact")).expect("send run");
    let events = collect_until_finished(&receiver);
    let secret = "sk-live-supersecret-abcdef";
    let preview = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::ToolResult { preview, .. } => Some(preview.clone()),
            _ => None,
        })
        .expect("tool result");
    assert!(preview.contains("[redacted]"), "{preview}");
    assert!(!preview.contains(secret), "{preview}");
    let stderr = client.stderr_tail(50).join("\n");
    assert!(stderr.contains("[redacted]"), "{stderr}");
    assert!(!stderr.contains(secret), "{stderr}");
    for event in &events {
        if let AgentEvent::Error { message, .. } = event {
            assert!(!message.contains(secret), "{message}");
        }
    }
    assert!(
        matches!(events.last(), Some(AgentEvent::Finished { ok: true, .. })),
        "{events:?}"
    );
}

#[test]
fn command_line_uses_mode_json_no_session_allowlist_and_flags() {
    let root = case_root("args");
    let config = test_config(
        &root,
        "happy",
        false,
        Duration::from_secs(30),
        Duration::from_secs(120),
        true,
    );
    let workspace = config.workspace.clone();
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-args")).expect("send run");
    let _ = collect_until_finished(&receiver);
    let args: Vec<String> = serde_json::from_str(
        &fs::read_to_string(workspace.join("ps_fake_pi_args.json")).expect("fake argv recorded"),
    )
    .expect("argv json");
    let joined = args.join(" ");
    assert!(joined.contains("--mode json"), "{joined}");
    assert!(joined.contains("--no-session"), "{joined}");
    assert!(joined.contains("--no-approve"), "{joined}");
    assert!(joined.contains("--tools read,grep,find,ls"), "{joined}");
    assert!(joined.contains("--model fake-model"), "{joined}");
    assert!(joined.contains("--provider fake-provider"), "{joined}");
    assert!(joined.contains("--system-prompt"), "{joined}");
    assert!(!joined.contains("--tools read,grep,find,ls,edit"), "{joined}");
}

#[test]
fn writes_opt_in_adds_write_and_shell_tools() {
    let root = case_root("writes");
    let config = test_config(
        &root,
        "happy",
        true,
        Duration::from_secs(30),
        Duration::from_secs(120),
        true,
    );
    let workspace = config.workspace.clone();
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn client");
    client.send_run(&run_request("pi-writes")).expect("send run");
    let _ = collect_until_finished(&receiver);
    let args: Vec<String> = serde_json::from_str(
        &fs::read_to_string(workspace.join("ps_fake_pi_args.json")).expect("fake argv recorded"),
    )
    .expect("argv json");
    let joined = args.join(" ");
    assert!(
        joined.contains("--tools read,grep,find,ls,edit,write,bash,powershell"),
        "{joined}"
    );
}

#[test]
fn probe_reads_the_launcher_version_without_a_model_call() {
    let root = case_root("probe");
    let wrapper = make_cmd_wrapper(&root);
    let info = probe_pi(Some(&wrapper)).expect("fake pi probe");
    assert_eq!(info.version, "1.5.0-fake");
    assert_eq!(info.path, wrapper);
}

#[test]
fn real_pi_version_probe_skips_when_absent() {
    match probe_pi(None) {
        Some(info) => {
            assert!(!info.version.is_empty());
            eprintln!(
                "real pi probe: version={} path={} node_ok={}",
                info.version,
                info.path.display(),
                info.node_ok
            );
        }
        None => eprintln!("SKIP real_pi_version_probe: pi is not installed on this host"),
    }
}
