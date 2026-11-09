//! End-to-end tests for the durable agent task model (V9 Agents card backend).
//!
//! They drive the real Pi client against the offline fake Pi stream and feed the
//! resulting AgentEvents into AgentTaskService, then assert lifecycle,
//! persistence across "app restart", cancellation, redaction and privacy.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ps_agent::{
    AgentEvent, AgentRunRequest, AgentTask, AgentTaskService, AgentTaskState, PiAgentClient, PiAgentConfig,
    PiInfo, PiSource,
};
use serde_json::Value;

fn fake_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ps_agent_fake_pi"))
}

fn case_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join(format!("agent tasks e2e {tag}-{nanos}"));
    fs::create_dir_all(&root).expect("create test root");
    root
}

fn make_cmd_wrapper(root: &Path) -> PathBuf {
    let dir = root.join("bin with space");
    fs::create_dir_all(&dir).expect("create wrapper dir");
    let launcher = dir.join("pi.cmd");
    fs::write(
        &launcher,
        format!("@echo off\r\n\"{}\" %*\r\n", fake_bin().display()),
    )
    .expect("write wrapper");
    launcher
}

fn pi_config(root: &Path, mode: &str, idle_timeout: Duration) -> PiAgentConfig {
    let workspace = root.join("workspace with space");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(workspace.join("ps_fake_pi_mode"), mode).expect("write fake mode");
    PiAgentConfig {
        pi: PiInfo {
            path: make_cmd_wrapper(root),
            version: "1.5.0-fake".to_string(),
            node_ok: true,
            source: PiSource::Explicit,
        },
        workspace,
        data_dir: root.join("data"),
        allow_writes: false,
        model: Some("fake-model".to_string()),
        provider: Some("fake-provider".to_string()),
        thinking: None,
        idle_timeout,
        max_run_duration: Duration::from_secs(120),
        extra_env: Vec::new(),
    }
}

fn request(run_id: &str, prompt: &str) -> AgentRunRequest {
    AgentRunRequest {
        run_id: run_id.to_string(),
        prompt: prompt.to_string(),
        context_paths: Vec::new(),
        model: None,
        max_steps: Some(8),
    }
}

fn drive_run(
    service: &AgentTaskService,
    task: &AgentTask,
    client: &PiAgentClient,
    receiver: &Receiver<AgentEvent>,
    run: &AgentRunRequest,
) -> Vec<AgentEvent> {
    client.send_run(run).expect("send run");
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut events = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out; events: {events:?}");
        match receiver.recv_timeout(remaining) {
            Ok(event) => {
                let terminal = matches!(event, AgentEvent::Finished { .. });
                service.apply_event(&task.id, &event);
                events.push(event);
                if terminal {
                    return events;
                }
            }
            Err(error) => panic!("event stream ended: {error}; events: {events:?}"),
        }
    }
}

fn read_tasks_file(data_dir: &Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(ps_agent::task::tasks_path(data_dir)).expect("tasks.json exists"),
    )
    .expect("tasks.json is valid json")
}

#[test]
fn pi_run_lifecycle_persists_and_survives_restart() {
    let root = case_root("lifecycle");
    let data_dir = root.join("data");
    let secret = "sk-live-supersecret-abcdef";
    let observed: Arc<Mutex<Vec<AgentTaskState>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&observed);
    let service = AgentTaskService::open(&data_dir, vec![secret.to_string()])
        .expect("open service")
        .with_listener(Arc::new(move |task: &AgentTask| {
            sink.lock().expect("lock").push(task.state);
        }));
    let config = pi_config(&root, "happy", Duration::from_secs(30));
    let info = config.pi.clone();
    let run = request(
        "run-task-1",
        "find the quarterly budget file using sk-live-supersecret-abcdef",
    );
    let task = service
        .begin_pi(&run, &info, Some("fake-model".into()), false, None)
        .expect("begin task");
    assert_eq!(task.state, AgentTaskState::Queued);
    assert!(task.title.contains("find the quarterly budget"));
    assert!(!task.prompt_summary.contains(secret), "{}", task.prompt_summary);
    assert!(task.permissions.read_only);
    assert_eq!(task.permissions.granted, vec!["read", "grep", "find", "ls"]);

    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn pi");
    let events = drive_run(&service, &task, &client, &receiver, &run);
    assert!(matches!(
        events.last(),
        Some(AgentEvent::Finished { ok: true, .. })
    ));
    client.shutdown();

    let completed = service.task(&task.id).expect("stored task");
    assert_eq!(completed.state, AgentTaskState::Completed);
    assert!(completed.started_at.is_some());
    assert!(completed.ended_at.is_some());
    assert_eq!(completed.runtime, ps_agent::AgentTaskRuntime::Pi);
    assert_eq!(completed.runtime_version.as_deref(), Some("1.5.0-fake"));
    assert_eq!(
        completed.progress.kind,
        ps_agent::AgentTaskProgressKind::Indeterminate
    );
    assert!(completed.progress.fraction.is_none());
    assert!(completed
        .log
        .iter()
        .any(|entry| entry.message.starts_with("started pi")));
    assert!(completed.log.iter().any(|entry| entry.message == "tool read ok"));
    assert!(matches!(
        observed.lock().expect("lock").last(),
        Some(AgentTaskState::Completed)
    ));

    // Activity/search projection.
    let activity = service.activity(10);
    assert_eq!(activity.len(), 1);
    assert_eq!(activity[0].state, AgentTaskState::Completed);
    assert!(activity[0].detail.contains("pi · completed · read-only"));
    assert!(activity[0].search_text.contains("completed"));

    // Persisted file: no prompt secret, no assistant output, no tool payload.
    let raw = fs::read_to_string(ps_agent::task::tasks_path(&data_dir)).expect("tasks file");
    assert!(!raw.contains(secret), "prompt secret persisted");
    assert!(!raw.contains("Fake answer for"), "assistant output persisted");
    assert!(!raw.contains("tool read completed"), "tool payload persisted");
    assert!(raw.contains("[redacted]"), "redaction marker missing");

    // Simulated app restart: completed history survives.
    drop(service);
    let reopened = AgentTaskService::open(&data_dir, Vec::new()).expect("reopen");
    let restored = reopened.task("run-task-1").expect("persisted task");
    assert_eq!(restored.state, AgentTaskState::Completed);
    assert_eq!(restored.title, completed.title);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cancel_marks_task_cancelled_and_ignores_late_finish() {
    let root = case_root("cancel");
    let data_dir = root.join("data");
    let service = AgentTaskService::open(&data_dir, Vec::new()).expect("open service");
    let config = pi_config(&root, "idle", Duration::from_secs(60));
    let info = config.pi.clone();
    let run = request("run-task-cancel", "long running task");
    let task = service.begin_pi(&run, &info, None, false, None).expect("begin");
    let (client, receiver) = PiAgentClient::spawn_channel(config).expect("spawn pi");
    client.send_run(&run).expect("send run");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "never started");
        let event = receiver.recv_timeout(remaining).expect("event");
        let is_started = matches!(event, AgentEvent::Started { .. });
        service.apply_event(&task.id, &event);
        if is_started {
            break;
        }
    }
    assert_eq!(
        service.task(&task.id).expect("task").state,
        AgentTaskState::Running
    );
    let cancelled = service.cancel(&task.id, "user pressed cancel").expect("cancel");
    assert_eq!(cancelled.state, AgentTaskState::Cancelled);
    assert_eq!(cancelled.error.expect("error").code, "cancelled");
    assert!(!cancelled.controls.can_cancel);
    client.cancel().expect("cancel process");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "never finished");
        let event = receiver.recv_timeout(remaining).expect("event");
        let terminal = matches!(event, AgentEvent::Finished { .. });
        service.apply_event(&task.id, &event);
        if terminal {
            break;
        }
    }
    // Late runtime events must not flip the host-cancelled state.
    assert_eq!(
        service.task(&task.id).expect("task").state,
        AgentTaskState::Cancelled
    );
    client.shutdown();
    let _ = fs::remove_dir_all(root);
}

#[test]
fn app_restart_recovers_running_task_as_offline() {
    let root = case_root("restart");
    let data_dir = root.join("data");
    let service = AgentTaskService::open(&data_dir, Vec::new()).expect("open");
    let info = PiInfo {
        path: "pi.cmd".into(),
        version: "1.5.0".into(),
        node_ok: true,
        source: PiSource::Managed,
    };
    let run = request("run-task-stale", "stale task");
    let task = service.begin_pi(&run, &info, None, false, None).expect("begin");
    service.apply_event(
        &task.id,
        &AgentEvent::Started {
            run_id: task.id.clone(),
            runtime: "pi 1.5.0".into(),
            model: "fake-model".into(),
        },
    );
    assert_eq!(
        service.task(&task.id).expect("task").state,
        AgentTaskState::Running
    );
    drop(service);
    let reopened = AgentTaskService::open(&data_dir, Vec::new()).expect("reopen");
    let recovered = reopened.task("run-task-stale").expect("task");
    assert_eq!(recovered.state, AgentTaskState::Offline);
    assert_eq!(recovered.error.expect("error").code, "app_restarted");
    assert!(recovered.controls.can_retry);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn error_and_output_redaction_is_applied_to_persisted_state() {
    let root = case_root("redaction");
    let data_dir = root.join("data");
    let secret = "sk-live-supersecret-abcdef";
    let service = AgentTaskService::open(&data_dir, vec![secret.to_string()]).expect("open");
    let info = PiInfo {
        path: "pi.cmd".into(),
        version: "1.5.0".into(),
        node_ok: true,
        source: PiSource::Managed,
    };
    let run = request("run-task-redact", "task with secret sk-live-supersecret-abcdef");
    let task = service.begin_pi(&run, &info, None, false, None).expect("begin");
    service.apply_event(
        &task.id,
        &AgentEvent::Started {
            run_id: task.id.clone(),
            runtime: "pi 1.5.0".into(),
            model: "fake-model".into(),
        },
    );
    service.apply_event(
        &task.id,
        &AgentEvent::Error {
            code: "provider_error".into(),
            message: "provider rejected key sk-live-supersecret-abcdef".into(),
        },
    );
    service.apply_event(
        &task.id,
        &AgentEvent::Output {
            text: "the answer contains sk-live-supersecret-abcdef".into(),
            final_answer: true,
        },
    );
    service.apply_event(
        &task.id,
        &AgentEvent::Finished {
            ok: false,
            duration_ms: 3,
        },
    );
    let task = service.task(&task.id).expect("task");
    assert_eq!(task.state, AgentTaskState::Failed);
    assert!(!task.title.contains(secret));
    assert!(!task.prompt_summary.contains(secret));
    let error = task.error.expect("error");
    assert!(!error.message.contains(secret), "{}", error.message);
    assert!(error.message.contains("[redacted]"), "{}", error.message);
    for entry in &task.log {
        assert!(!entry.message.contains(secret), "{}", entry.message);
    }
    let raw = fs::read_to_string(ps_agent::task::tasks_path(&data_dir)).expect("tasks file");
    assert!(!raw.contains(secret), "secret persisted in tasks file");
    assert!(!raw.contains("the answer contains"), "output persisted");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn persisted_file_has_expected_schema_shape() {
    let root = case_root("schema");
    let data_dir = root.join("data");
    let service = AgentTaskService::open(&data_dir, Vec::new()).expect("open");
    let info = PiInfo {
        path: "pi.cmd".into(),
        version: "1.5.0".into(),
        node_ok: true,
        source: PiSource::Managed,
    };
    let task = service
        .begin_pi(&request("run-task-schema", "schema"), &info, None, false, None)
        .expect("begin");
    service.apply_event(
        &task.id,
        &AgentEvent::Finished {
            ok: true,
            duration_ms: 1,
        },
    );
    let file = read_tasks_file(&data_dir);
    assert_eq!(file["schemaVersion"].as_u64(), Some(1));
    let first = &file["tasks"][0];
    for key in [
        "id",
        "runtime",
        "runtimeVersion",
        "parentId",
        "title",
        "promptSummary",
        "state",
        "progress",
        "createdAt",
        "startedAt",
        "endedAt",
        "updatedAt",
        "log",
        "error",
        "permissions",
        "writeAccess",
        "controls",
        "restarts",
        "model",
    ] {
        assert!(first.get(key).is_some(), "missing field {key}: {first}");
    }
    assert_eq!(first["state"].as_str(), Some("completed"));
    assert_eq!(first["progress"]["kind"].as_str(), Some("indeterminate"));
    assert_eq!(first["controls"]["canPause"].as_bool(), Some(false));
    let _ = fs::remove_dir_all(root);
}
