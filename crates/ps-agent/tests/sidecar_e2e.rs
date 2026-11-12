//! Integration tests against the real Python sidecar in deterministic
//! test-model mode (PROTONSEARCH_AGENT_TEST_MODEL=1): no network, no API key.
//!
//! These tests skip with a clear message when the machine-local venv has not
//! been created; they never fake a pass.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use ps_agent::{
    runtime_available_for, AgentApprovalResponse, AgentClient, AgentEvent, AgentRunRequest, SidecarConfig,
};
use ps_ipc::ActionSpec;

const EVENT_TIMEOUT: Duration = Duration::from_secs(45);

fn sidecar_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("agent-sidecar")
}

fn test_config(idle_timeout: Duration) -> Option<SidecarConfig> {
    let sidecar_dir = sidecar_dir();
    let python = if cfg!(windows) {
        sidecar_dir.join(".venv").join("Scripts").join("python.exe")
    } else {
        sidecar_dir.join(".venv").join("bin").join("python")
    };
    let config = SidecarConfig {
        python,
        sidecar_dir,
        model: None,
        test_model: true,
        idle_timeout,
        extra_env: Vec::new(),
    };
    if runtime_available_for(&config) {
        Some(config)
    } else {
        None
    }
}

fn spawn() -> Option<(AgentClient, Receiver<AgentEvent>)> {
    let config = test_config(Duration::from_secs(60))?;
    match AgentClient::spawn_channel(config) {
        Ok(pair) => Some(pair),
        Err(err) => panic!("sidecar failed to spawn: {err}"),
    }
}

fn wait_for<F>(receiver: &Receiver<AgentEvent>, predicate: F, timeout: Duration) -> AgentEvent
where
    F: Fn(&AgentEvent) -> bool,
{
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            panic!("timed out waiting for the expected sidecar event");
        }
        match receiver.recv_timeout(remaining) {
            Ok(event) if predicate(&event) => return event,
            Ok(_) => continue,
            Err(err) => panic!("sidecar event stream ended unexpectedly: {err}"),
        }
    }
}

fn run_request(run_id: &str) -> AgentRunRequest {
    AgentRunRequest {
        run_id: run_id.to_string(),
        prompt: "find the quarterly budget file".to_string(),
        context_paths: vec!["C:/Users/Public/protonsearch-e2e-budget.txt".to_string()],
        model: None,
        max_steps: Some(8),
    }
}

#[test]
fn approval_flow_streams_events_and_finishes() {
    let Some((client, events)) = spawn() else {
        eprintln!(
            "SKIP approval_flow_streams_events_and_finishes: sidecar venv missing at {}",
            sidecar_dir().join(".venv").display()
        );
        return;
    };
    assert!(client.is_alive());
    client.send_run(&run_request("e2e-run")).expect("send run");

    match wait_for(
        &events,
        |event| matches!(event, AgentEvent::Started { .. }),
        EVENT_TIMEOUT,
    ) {
        AgentEvent::Started {
            run_id,
            runtime,
            model,
        } => {
            assert_eq!(run_id, "e2e-run");
            assert!(runtime.contains("pydantic-ai"), "{runtime}");
            assert_eq!(model, "test:scripted");
        }
        other => panic!("unexpected event {other:?}"),
    }
    wait_for(
        &events,
        |event| matches!(event, AgentEvent::Plan { .. }),
        EVENT_TIMEOUT,
    );

    let first_approval = wait_for(
        &events,
        |event| matches!(event, AgentEvent::ApprovalRequest { .. }),
        EVENT_TIMEOUT,
    );
    let first_id = match &first_approval {
        AgentEvent::ApprovalRequest {
            approval_id,
            title,
            action,
            destructive,
            ..
        } => {
            assert_eq!(title, "Search your files");
            assert_eq!(*action, ActionSpec::None);
            assert!(!destructive);
            approval_id.clone()
        }
        other => panic!("unexpected event {other:?}"),
    };
    client
        .approve(&AgentApprovalResponse {
            run_id: "e2e-run".into(),
            approval_id: first_id,
            approved: true,
        })
        .expect("approve search");
    let first_result = wait_for(
        &events,
        |event| matches!(event, AgentEvent::ToolResult { .. }),
        EVENT_TIMEOUT,
    );
    match first_result {
        AgentEvent::ToolResult { tool, ok, preview } => {
            assert_eq!(tool, "search_files");
            assert!(ok);
            assert!(preview.contains("delegated to the host"), "{preview}");
        }
        other => panic!("unexpected event {other:?}"),
    }

    let second_approval = wait_for(
        &events,
        |event| matches!(event, AgentEvent::ApprovalRequest { .. }),
        EVENT_TIMEOUT,
    );
    let second_id = match &second_approval {
        AgentEvent::ApprovalRequest {
            approval_id, title, ..
        } => {
            assert_eq!(title, "Preview a file");
            approval_id.clone()
        }
        other => panic!("unexpected event {other:?}"),
    };
    client
        .approve(&AgentApprovalResponse {
            run_id: "e2e-run".into(),
            approval_id: second_id,
            approved: true,
        })
        .expect("approve preview");
    wait_for(
        &events,
        |event| matches!(event, AgentEvent::ToolResult { ok: true, .. }),
        EVENT_TIMEOUT,
    );

    let output = wait_for(
        &events,
        |event| matches!(event, AgentEvent::Output { .. }),
        EVENT_TIMEOUT,
    );
    match output {
        AgentEvent::Output { text, final_answer } => {
            assert!(final_answer);
            assert!(text.contains("TEST MODEL"), "{text}");
            assert!(text.contains("e2e-run"), "{text}");
        }
        other => panic!("unexpected event {other:?}"),
    }
    let finished = wait_for(
        &events,
        |event| matches!(event, AgentEvent::Finished { .. }),
        EVENT_TIMEOUT,
    );
    match finished {
        AgentEvent::Finished { ok, duration_ms } => {
            assert!(ok);
            assert!(duration_ms < 60_000);
        }
        other => panic!("unexpected event {other:?}"),
    }

    let stderr = client.stderr_tail(200).join("\n");
    assert!(
        !stderr.contains("Traceback (most recent call last)"),
        "sidecar stderr: {stderr}"
    );
    client.shutdown();
    assert!(!client.is_alive(), "sidecar must exit after shutdown");
}

#[test]
fn cancel_while_waiting_for_approval() {
    let Some((client, events)) = spawn() else {
        eprintln!("SKIP cancel_while_waiting_for_approval: sidecar venv missing");
        return;
    };
    client.send_run(&run_request("e2e-cancel")).expect("send run");
    wait_for(
        &events,
        |event| matches!(event, AgentEvent::ApprovalRequest { .. }),
        EVENT_TIMEOUT,
    );
    client.cancel(Some("e2e-cancel")).expect("send cancel");
    let cancelled = wait_for(
        &events,
        |event| matches!(event, AgentEvent::Error { code, .. } if code == "cancelled"),
        EVENT_TIMEOUT,
    );
    match cancelled {
        AgentEvent::Error { message, .. } => assert!(message.contains("cancelled"), "{message}"),
        other => panic!("unexpected event {other:?}"),
    }
    let finished = wait_for(
        &events,
        |event| matches!(event, AgentEvent::Finished { .. }),
        EVENT_TIMEOUT,
    );
    assert!(matches!(finished, AgentEvent::Finished { ok: false, .. }));
    client.shutdown();
}

#[test]
fn idle_timeout_kills_the_sidecar() {
    let Some(config) = test_config(Duration::from_millis(400)) else {
        eprintln!("SKIP idle_timeout_kills_the_sidecar: sidecar venv missing");
        return;
    };
    let (client, events) = AgentClient::spawn_channel(config).expect("spawn");
    client.send_run(&run_request("e2e-idle")).expect("send run");
    loop {
        match wait_for(
            &events,
            |event| {
                matches!(
                    event,
                    AgentEvent::ApprovalRequest { .. }
                        | AgentEvent::Finished { .. }
                        | AgentEvent::Output { .. }
                )
            },
            EVENT_TIMEOUT,
        ) {
            AgentEvent::ApprovalRequest { approval_id, .. } => {
                client
                    .approve(&AgentApprovalResponse {
                        run_id: "e2e-idle".into(),
                        approval_id,
                        approved: true,
                    })
                    .expect("approve");
            }
            AgentEvent::Finished { ok, .. } => {
                assert!(ok);
                break;
            }
            AgentEvent::Output { .. } => continue,
            _ => unreachable!(),
        }
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    while client.is_alive() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(!client.is_alive(), "idle watchdog must kill the sidecar");
    client.shutdown();
}

#[cfg(windows)]
#[test]
fn crash_is_detected_and_restarted_once() {
    let Some((client, events)) = spawn() else {
        eprintln!("SKIP crash_is_detected_and_restarted_once: sidecar venv missing");
        return;
    };
    let pid = client.pid().expect("sidecar pid");
    let status = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .status()
        .expect("run taskkill");
    assert!(status.success(), "taskkill failed: {status}");

    wait_for(
        &events,
        |event| matches!(event, AgentEvent::Error { code, .. } if code == "sidecar_crashed"),
        Duration::from_secs(30),
    );
    wait_for(
        &events,
        |event| matches!(event, AgentEvent::Error { code, .. } if code == "sidecar_restarted"),
        Duration::from_secs(30),
    );
    assert_eq!(client.restart_count(), 1);
    assert!(client.is_alive(), "restarted sidecar should be running");
    client.shutdown();
}
