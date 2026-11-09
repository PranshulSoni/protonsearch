//! Durable, runtime-agnostic agent task store.
//!
//! Location: <data_dir>/agents/tasks.json (versioned JSON, atomic replace).
//! The store is the source of truth for the V9 Agents card / desktop view: it
//! persists IDs, states, redacted summaries, redacted log rings, permissions,
//! error and restart history, but never prompts, model output or tool payloads.
//!
//! On open, tasks left Queued/Running by a previous app process are recovered
//! as Offline (crash isolation) so the UI never shows a phantom "running" task.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use ps_ipc::AgentEvent;

use crate::error::AgentError;

use super::model::*;

const AGENTS_DIR: &str = "agents";
const TASKS_FILE: &str = "tasks.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TasksFile {
    schema_version: u32,
    tasks: Vec<AgentTask>,
}

#[derive(Default)]
struct StoreInner {
    /// Newest first.
    tasks: Vec<AgentTask>,
}

/// Persistent agent task store. Cheap to clone behind an Arc; all methods take
/// &self and are internally synchronized.
pub struct AgentTaskStore {
    path: PathBuf,
    redactions: Vec<String>,
    inner: Mutex<StoreInner>,
    persist_error: Mutex<Option<String>>,
}

/// Where the task history file is stored for a v2 data dir.
pub fn tasks_path(data_dir: &Path) -> PathBuf {
    data_dir.join(AGENTS_DIR).join(TASKS_FILE)
}

impl AgentTaskStore {
    /// Open (or create) the store in the given data dir. Corrupt files are
    /// moved aside rather than silently overwriting user history.
    pub fn open(data_dir: &Path, redactions: Vec<String>) -> Result<Self, AgentError> {
        let path = tasks_path(data_dir);
        let mut tasks = load_tasks(&path);
        let mut recovered = false;
        let now = now_ms();
        for task in tasks.iter_mut() {
            if !task.state.is_terminal() {
                recovered = true;
                task.state = AgentTaskState::Offline;
                task.ended_at = Some(now);
                task.updated_at = now;
                task.error = Some(AgentTaskErrorInfo {
                    code: "app_restarted".into(),
                    message: "the app restarted before this task finished".into(),
                });
                task.progress = AgentTaskProgress::indeterminate();
                task.push_log(
                    AgentTaskLogLevel::Warn,
                    "recovered as offline: app restarted before the task finished".into(),
                    now,
                );
            }
            task.refresh_controls();
        }
        sort_newest_first(&mut tasks);
        prune(&mut tasks);
        let store = Self {
            path,
            redactions,
            inner: Mutex::new(StoreInner { tasks }),
            persist_error: Mutex::new(None),
        };
        if recovered {
            store.persist_current();
        }
        Ok(store)
    }

    pub fn tasks_path(&self) -> &Path {
        &self.path
    }

    pub fn redactions(&self) -> &[String] {
        &self.redactions
    }

    /// Bounded, newest-first task list. limit 0 means MAX_PERSISTED_TASKS.
    pub fn tasks(&self, limit: usize) -> Vec<AgentTask> {
        let inner = self.lock_inner();
        let take = if limit == 0 {
            MAX_PERSISTED_TASKS
        } else {
            limit.min(MAX_PERSISTED_TASKS)
        };
        inner.tasks.iter().take(take).cloned().collect()
    }

    pub fn task(&self, id: &str) -> Option<AgentTask> {
        let inner = self.lock_inner();
        inner.tasks.iter().find(|task| task.id == id).cloned()
    }

    pub fn permissions(&self, id: &str) -> Option<AgentPermissionState> {
        self.task(id).map(|task| task.permissions)
    }

    /// Parent/child tree; roots are tasks whose parent is absent from the
    /// bounded window. Pi tasks are always single-level (parentId null).
    pub fn tree(&self, limit: usize) -> Vec<AgentTaskNode> {
        let tasks = self.tasks(limit);
        let by_id: HashMap<String, AgentTask> = tasks
            .iter()
            .cloned()
            .map(|task| (task.id.clone(), task))
            .collect();
        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        let mut roots: Vec<String> = Vec::new();
        for task in &tasks {
            match &task.parent_id {
                Some(parent) if by_id.contains_key(parent) && parent != &task.id => {
                    children.entry(parent.clone()).or_default().push(task.id.clone());
                }
                _ => roots.push(task.id.clone()),
            }
        }
        for child_ids in children.values_mut() {
            child_ids.sort_by(|left, right| {
                let left_created = by_id.get(left).map(|task| task.created_at).unwrap_or(0);
                let right_created = by_id.get(right).map(|task| task.created_at).unwrap_or(0);
                right_created.cmp(&left_created)
            });
        }
        roots
            .into_iter()
            .filter_map(|id| build_node(&id, &by_id, &children, 0))
            .collect()
    }

    /// Create a Queued task. The id is the runtime run id; a numeric suffix is
    /// added if an old task already used it (the returned task.id is canonical).
    pub fn begin_task(&self, id: &str, prompt: &str, launch: &AgentTaskLaunch) -> AgentTask {
        let mut inner = self.lock_inner();
        let id = unique_id(&inner.tasks, id);
        let now = now_ms();
        let (title, prompt_summary) = summarize_prompt(prompt, &self.redactions);
        let (requested, granted, read_only) = match launch.runtime {
            AgentTaskRuntime::Pi => {
                let tools: Vec<String> = crate::pi::pi_tool_allowlist(launch.allow_writes)
                    .split(',')
                    .map(str::to_string)
                    .collect();
                (tools.clone(), tools, !launch.allow_writes)
            }
            _ => (Vec::new(), Vec::new(), !launch.allow_writes),
        };
        let mut task = AgentTask {
            id: id.clone(),
            runtime: launch.runtime,
            runtime_version: launch.runtime_version.clone(),
            parent_id: launch.parent_id.clone(),
            title,
            prompt_summary,
            state: AgentTaskState::Queued,
            progress: AgentTaskProgress::indeterminate(),
            created_at: now,
            started_at: None,
            ended_at: None,
            updated_at: now,
            log: Vec::new(),
            error: None,
            permissions: AgentPermissionState {
                requested,
                granted,
                denied: Vec::new(),
                pending: None,
                read_only,
            },
            write_access: launch.allow_writes,
            controls: AgentTaskControls::for_state(AgentTaskState::Queued),
            restarts: Vec::new(),
            model: launch.model.clone(),
        };
        task.push_log(
            AgentTaskLogLevel::Info,
            format!(
                "queued ({} runtime{})",
                launch.runtime.label(),
                if launch.allow_writes {
                    ", write access"
                } else {
                    ", read-only"
                }
            ),
            now,
        );
        inner.tasks.insert(0, task.clone());
        prune(&mut inner.tasks);
        let _ = self.persist_locked(&inner);
        task
    }

    /// Apply one frozen ps-ipc event to the task. Returns the updated snapshot
    /// when the visible task changed. Assistant text deltas are deliberately not
    /// persisted (no output storage) and return None.
    pub fn apply_event(&self, id: &str, event: &AgentEvent) -> Option<AgentTask> {
        let mut inner = self.lock_inner();
        let index = inner.tasks.iter().position(|task| task.id == id)?;
        let mut task = inner.tasks[index].clone();
        if task.state.is_terminal() {
            return None;
        }
        let now = now_ms();
        let mut changed = false;
        match event {
            AgentEvent::Started { runtime, model, .. } => {
                let (parsed_runtime, version) = AgentTaskRuntime::from_started_label(runtime);
                if task.runtime == AgentTaskRuntime::Unknown {
                    task.runtime = parsed_runtime;
                }
                if version.is_some() {
                    task.runtime_version = version;
                }
                if task.model.is_none() && !model.is_empty() && model != "default" {
                    task.model = Some(model.clone());
                }
                task.state = AgentTaskState::Running;
                if task.started_at.is_none() {
                    task.started_at = Some(now);
                }
                task.push_log(
                    AgentTaskLogLevel::Info,
                    redact_truncate(&format!("started {}", runtime), &self.redactions, MAX_LOG_CHARS),
                    now,
                );
                changed = true;
            }
            AgentEvent::Progress { message, fraction } => {
                if let Some(fraction) = fraction {
                    task.progress = AgentTaskProgress::determinate(*fraction);
                    changed = true;
                }
                if is_status_message(message) {
                    task.push_log(
                        AgentTaskLogLevel::Info,
                        redact_truncate(message, &self.redactions, MAX_LOG_CHARS),
                        now,
                    );
                    changed = true;
                }
                // Assistant text deltas carry model output: never persisted.
            }
            AgentEvent::ApprovalRequest { title, .. } => {
                let label = redact_truncate(title, &self.redactions, MAX_LOG_CHARS);
                if !task.permissions.requested.contains(&label) {
                    task.permissions.requested.push(label.clone());
                }
                task.permissions.pending = Some(label.clone());
                task.push_log(
                    AgentTaskLogLevel::Warn,
                    format!("permission requested: {label}"),
                    now,
                );
                changed = true;
            }
            AgentEvent::ToolResult { tool, ok, .. } => {
                let label = redact_truncate(tool, &self.redactions, MAX_LOG_CHARS);
                if let Some(pending) = task.permissions.pending.take() {
                    if *ok {
                        task.permissions.granted.push(pending);
                    } else {
                        task.permissions.denied.push(pending);
                    }
                }
                task.push_log(
                    AgentTaskLogLevel::Info,
                    format!("tool {label} {}", if *ok { "ok" } else { "failed" }),
                    now,
                );
                changed = true;
            }
            AgentEvent::Output { final_answer, .. } => {
                if *final_answer {
                    task.push_log(AgentTaskLogLevel::Info, "final answer ready".into(), now);
                    changed = true;
                }
            }
            AgentEvent::Error { code, message } => {
                let message = redact_truncate(message, &self.redactions, MAX_ERROR_CHARS);
                task.error = Some(AgentTaskErrorInfo {
                    code: code.clone(),
                    message: message.clone(),
                });
                if matches!(code.as_str(), "sidecar_restarted" | "sidecar_crashed") {
                    task.restarts.push(AgentTaskRestart {
                        at: now,
                        reason: redact_truncate(&message, &self.redactions, MAX_LOG_CHARS),
                    });
                }
                task.push_log(AgentTaskLogLevel::Error, format!("{code}: {message}"), now);
                changed = true;
            }
            AgentEvent::Finished { ok, duration_ms } => {
                task.state = if *ok {
                    AgentTaskState::Completed
                } else if task
                    .error
                    .as_ref()
                    .map(|error| error.code == "cancelled")
                    .unwrap_or(false)
                {
                    AgentTaskState::Cancelled
                } else {
                    AgentTaskState::Failed
                };
                task.ended_at = Some(now);
                task.progress = AgentTaskProgress::indeterminate();
                task.push_log(
                    AgentTaskLogLevel::Info,
                    format!(
                        "finished {} in {} ms",
                        if *ok { "ok" } else { "not ok" },
                        duration_ms
                    ),
                    now,
                );
                changed = true;
            }
            // Plan steps and progress deltas are model content; never persisted.
            AgentEvent::Plan { .. } => {}
        }
        if !changed {
            return None;
        }
        task.updated_at = now;
        task.refresh_controls();
        prune_log(&mut task);
        inner.tasks[index] = task.clone();
        let _ = self.persist_locked(&inner);
        Some(task)
    }

    /// Mark a task as failed before/without a runtime event (e.g. spawn error).
    pub fn mark_launch_failed(&self, id: &str, code: &str, message: &str) -> Option<AgentTask> {
        self.transition_terminal(
            id,
            AgentTaskState::Failed,
            code,
            message,
            AgentTaskLogLevel::Error,
            format!("launch failed: {code}"),
        )
    }

    /// Mark a task offline (runtime unavailable / disconnected).
    pub fn mark_offline(&self, id: &str, message: &str) -> Option<AgentTask> {
        self.transition_terminal(
            id,
            AgentTaskState::Offline,
            "runtime_offline",
            message,
            AgentTaskLogLevel::Warn,
            "runtime offline".to_string(),
        )
    }

    /// Mark a task cancelled by the host. Returns the task snapshot so the
    /// caller can kill the process; None when the task already finished.
    pub fn cancel(&self, id: &str, reason: &str) -> Option<AgentTask> {
        self.transition_terminal(
            id,
            AgentTaskState::Cancelled,
            "cancelled",
            reason,
            AgentTaskLogLevel::Warn,
            "cancelled by the host".to_string(),
        )
    }

    /// Mark every active task cancelled. Returns the snapshots to kill.
    pub fn stop_all(&self, reason: &str) -> Vec<AgentTask> {
        let mut inner = self.lock_inner();
        let now = now_ms();
        let mut cancelled = Vec::new();
        for task in inner.tasks.iter_mut() {
            if task.state.is_terminal() {
                continue;
            }
            task.state = AgentTaskState::Cancelled;
            task.ended_at = Some(now);
            task.updated_at = now;
            task.progress = AgentTaskProgress::indeterminate();
            task.error = Some(AgentTaskErrorInfo {
                code: "cancelled".into(),
                message: redact_truncate(reason, &self.redactions, MAX_ERROR_CHARS),
            });
            task.push_log(AgentTaskLogLevel::Warn, "cancelled by stop_all".to_string(), now);
            task.refresh_controls();
            cancelled.push(task.clone());
        }
        if !cancelled.is_empty() {
            let _ = self.persist_locked(&inner);
        }
        cancelled
    }

    /// Retry persistence explicitly; mutation methods persist best-effort and
    /// record any failure here.
    pub fn flush(&self) -> Result<(), AgentError> {
        let inner = self.lock_inner();
        self.persist_locked(&inner)
    }

    pub fn last_persist_error(&self) -> Option<String> {
        self.persist_error
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    fn transition_terminal(
        &self,
        id: &str,
        state: AgentTaskState,
        code: &str,
        message: &str,
        level: AgentTaskLogLevel,
        log_message: String,
    ) -> Option<AgentTask> {
        let mut inner = self.lock_inner();
        let index = inner.tasks.iter().position(|task| task.id == id)?;
        if inner.tasks[index].state.is_terminal() {
            return None;
        }
        let now = now_ms();
        let task = &mut inner.tasks[index];
        task.state = state;
        task.ended_at = Some(now);
        task.updated_at = now;
        task.progress = AgentTaskProgress::indeterminate();
        task.error = Some(AgentTaskErrorInfo {
            code: code.to_string(),
            message: redact_truncate(message, &self.redactions, MAX_ERROR_CHARS),
        });
        task.push_log(level, log_message, now);
        task.refresh_controls();
        let snapshot = task.clone();
        let _ = self.persist_locked(&inner);
        Some(snapshot)
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, StoreInner> {
        self.inner.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    fn persist_current(&self) {
        let inner = self.lock_inner();
        let _ = self.persist_locked(&inner);
    }

    fn persist_locked(&self, inner: &StoreInner) -> Result<(), AgentError> {
        let payload = TasksFile {
            schema_version: TASKS_SCHEMA_VERSION,
            tasks: inner.tasks.clone(),
        };
        let json = serde_json::to_string_pretty(&payload)?;
        let result = write_atomic(&self.path, json.as_bytes());
        let mut error_slot = self
            .persist_error
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        match &result {
            Ok(()) => *error_slot = None,
            Err(error) => *error_slot = Some(error.to_string()),
        }
        result
    }
}

fn load_tasks(path: &Path) -> Vec<AgentTask> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    match serde_json::from_str::<TasksFile>(&text) {
        Ok(file) => file.tasks,
        Err(_) => {
            // Keep a copy of the unreadable file for diagnostics instead of
            // silently overwriting history on the next persist.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0);
            let corrupt = path.with_extension(format!("json.corrupt-{nanos}"));
            let _ = fs::rename(path, corrupt);
            Vec::new()
        }
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AgentError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(AgentError::Io)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).map_err(AgentError::Io)?;
    // std::fs::rename does not reliably replace an existing file on Windows.
    if path.exists() {
        fs::remove_file(path).map_err(AgentError::Io)?;
    }
    fs::rename(&tmp, path).map_err(AgentError::Io)?;
    Ok(())
}

fn unique_id(tasks: &[AgentTask], base: &str) -> String {
    let base = if base.trim().is_empty() { "task" } else { base };
    if !tasks.iter().any(|task| task.id == base) {
        return base.to_string();
    }
    for suffix in 2..10_000u32 {
        let candidate = format!("{base}-{suffix}");
        if !tasks.iter().any(|task| task.id == candidate) {
            return candidate;
        }
    }
    format!("{base}-{}", now_ms())
}

fn is_status_message(message: &str) -> bool {
    message.starts_with("tool: ")
        || message.starts_with("provider retry ")
        || message == "compacting context"
        || message == "context compacted"
        || message.starts_with("waiting for approval: ")
}

fn sort_newest_first(tasks: &mut [AgentTask]) {
    tasks.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
}

fn prune(tasks: &mut Vec<AgentTask>) {
    while tasks.len() > MAX_PERSISTED_TASKS {
        // Prefer dropping the oldest terminal task; fall back to the oldest.
        match tasks.iter().rposition(|task| task.state.is_terminal()) {
            Some(index) => {
                tasks.remove(index);
            }
            None => {
                tasks.pop();
            }
        }
    }
}

fn prune_log(task: &mut AgentTask) {
    while task.log.len() > MAX_LOG_ENTRIES {
        task.log.remove(0);
    }
}

fn build_node(
    id: &str,
    by_id: &HashMap<String, AgentTask>,
    children: &HashMap<String, Vec<String>>,
    depth: usize,
) -> Option<AgentTaskNode> {
    let task = by_id.get(id)?.clone();
    let mut node = task.node();
    if depth < 16 {
        if let Some(child_ids) = children.get(id) {
            let mut seen: HashSet<&str> = HashSet::new();
            for child in child_ids {
                if !seen.insert(child.as_str()) {
                    continue;
                }
                if let Some(child_node) = build_node(child, by_id, children, depth + 1) {
                    node.children.push(child_node);
                }
            }
        }
    }
    Some(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join(format!("ps-agent-task-test-{tag}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn launch_pi(allow_writes: bool) -> AgentTaskLaunch {
        AgentTaskLaunch {
            runtime: AgentTaskRuntime::Pi,
            runtime_version: Some("1.5.0".into()),
            model: Some("fake-model".into()),
            allow_writes,
            parent_id: None,
        }
    }

    fn started(run_id: &str) -> AgentEvent {
        AgentEvent::Started {
            run_id: run_id.into(),
            runtime: "pi 1.5.0".into(),
            model: "fake-model".into(),
        }
    }

    fn finished(ok: bool) -> AgentEvent {
        AgentEvent::Finished { ok, duration_ms: 42 }
    }

    #[test]
    fn lifecycle_queued_running_completed_persists_across_reopen() {
        let dir = scratch_dir("lifecycle");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        let task = store.begin_task("run-1", "find the budget", &launch_pi(false));
        assert_eq!(task.state, AgentTaskState::Queued);
        assert!(task.controls.can_cancel);
        let task = store.apply_event("run-1", &started("run-1")).expect("running");
        assert_eq!(task.state, AgentTaskState::Running);
        assert!(task.started_at.is_some());
        let task = store.apply_event("run-1", &finished(true)).expect("completed");
        assert_eq!(task.state, AgentTaskState::Completed);
        assert!(task.ended_at.is_some());
        assert!(!task.controls.can_cancel);
        assert!(!task.controls.can_retry, "completed tasks are not retryable");
        store.flush().expect("flush");
        assert!(store.last_persist_error().is_none());

        let reopened = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        let task = reopened.task("run-1").expect("persisted task");
        assert_eq!(task.state, AgentTaskState::Completed);
        assert_eq!(task.runtime, AgentTaskRuntime::Pi);
        assert_eq!(task.runtime_version.as_deref(), Some("1.5.0"));
        assert!(task
            .log
            .iter()
            .any(|entry| entry.message.starts_with("started pi")));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn failure_keeps_redacted_error_and_terminal_state() {
        let dir = scratch_dir("failure");
        let store = AgentTaskStore::open(&dir, vec!["sk-live-supersecret-abcdef".into()]).unwrap();
        store.begin_task("run-2", "do a thing", &launch_pi(false));
        store.apply_event("run-2", &started("run-2"));
        store.apply_event(
            "run-2",
            &AgentEvent::Error {
                code: "auth_error".into(),
                message: "401 invalid api key sk-live-supersecret-abcdef".into(),
            },
        );
        let task = store.apply_event("run-2", &finished(false)).expect("failed");
        assert_eq!(task.state, AgentTaskState::Failed);
        let error = task.error.expect("error");
        assert_eq!(error.code, "auth_error");
        assert!(
            !error.message.contains("sk-live-supersecret"),
            "{}",
            error.message
        );
        assert!(error.message.contains("[redacted]"), "{}", error.message);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cancelled_by_host_wins_over_late_runtime_events() {
        let dir = scratch_dir("cancel");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        store.begin_task("run-3", "long task", &launch_pi(false));
        store.apply_event("run-3", &started("run-3"));
        let cancelled = store.cancel("run-3", "user pressed cancel").expect("cancelled");
        assert_eq!(cancelled.state, AgentTaskState::Cancelled);
        assert_eq!(cancelled.error.expect("error").code, "cancelled");
        assert!(store.apply_event("run-3", &finished(false)).is_none());
        assert_eq!(
            store.task("run-3").expect("task").state,
            AgentTaskState::Cancelled
        );
        assert!(store.cancel("run-3", "again").is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reopen_recovers_running_tasks_as_offline() {
        let dir = scratch_dir("recover");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        store.begin_task("run-4", "still running", &launch_pi(false));
        store.apply_event("run-4", &started("run-4"));
        store.flush().expect("flush");
        drop(store);
        let reopened = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        let task = reopened.task("run-4").expect("task");
        assert_eq!(task.state, AgentTaskState::Offline);
        assert_eq!(task.error.expect("error").code, "app_restarted");
        assert!(task.controls.can_retry);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn progress_is_determinate_only_when_a_real_fraction_exists() {
        let dir = scratch_dir("progress");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        store.begin_task("run-5", "preview a file", &launch_pi(false));
        store.apply_event("run-5", &started("run-5"));
        let task = store.apply_event(
            "run-5",
            &AgentEvent::Progress {
                message: "assistant text delta".into(),
                fraction: None,
            },
        );
        assert!(task.is_none(), "text deltas must not update persisted state");
        let task = store
            .apply_event(
                "run-5",
                &AgentEvent::Progress {
                    message: "reading files".into(),
                    fraction: Some(0.25),
                },
            )
            .expect("determinate progress");
        assert_eq!(task.progress.kind, AgentTaskProgressKind::Determinate);
        assert_eq!(task.progress.fraction, Some(0.25));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn permissions_map_read_only_and_writes_runs() {
        let dir = scratch_dir("permissions");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        let read_only = store.begin_task("run-6", "read only", &launch_pi(false));
        assert!(read_only.permissions.read_only);
        assert_eq!(read_only.permissions.granted, vec!["read", "grep", "find", "ls"]);
        assert!(read_only.permissions.requested.iter().any(|tool| tool == "grep"));
        let writes = store.begin_task("run-7", "write", &launch_pi(true));
        assert!(!writes.permissions.read_only);
        assert!(writes.permissions.granted.iter().any(|tool| tool == "write"));
        assert!(writes.permissions.granted.iter().any(|tool| tool == "bash"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fallback_permission_flow_records_requested_granted_denied() {
        let dir = scratch_dir("approvals");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        store.begin_task(
            "run-8",
            "fallback",
            &AgentTaskLaunch::for_pydantic(None, false, None),
        );
        store.apply_event("run-8", &started("run-8"));
        store.apply_event(
            "run-8",
            &AgentEvent::ApprovalRequest {
                approval_id: "apr-1".into(),
                title: "Search your files".into(),
                detail: "read-only".into(),
                action: ps_ipc::ActionSpec::None,
                destructive: false,
            },
        );
        let task = store.task("run-8").expect("task");
        assert_eq!(task.permissions.pending.as_deref(), Some("Search your files"));
        assert!(task
            .permissions
            .requested
            .contains(&"Search your files".to_string()));
        store.apply_event(
            "run-8",
            &AgentEvent::ToolResult {
                tool: "search_files".into(),
                ok: true,
                preview: "approved".into(),
            },
        );
        let task = store.task("run-8").expect("task");
        assert!(task
            .permissions
            .granted
            .contains(&"Search your files".to_string()));
        assert!(task.permissions.pending.is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn prompt_output_and_tool_payloads_are_not_persisted() {
        let dir = scratch_dir("privacy");
        let secret = "sk-live-supersecret-abcdef";
        let store = AgentTaskStore::open(&dir, vec![secret.into()]).unwrap();
        let prompt = format!("find budget using {secret}");
        store.begin_task("run-9", &prompt, &launch_pi(false));
        store.apply_event(
            "run-9",
            &AgentEvent::Output {
                text: "assistant output must not be stored".into(),
                final_answer: true,
            },
        );
        store.flush().expect("flush");
        let raw = fs::read_to_string(tasks_path(&dir)).expect("tasks file");
        assert!(!raw.contains(secret), "secret persisted");
        assert!(
            !raw.contains("assistant output must not be stored"),
            "output persisted"
        );
        assert!(!raw.contains("tool read completed"), "tool preview persisted");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parent_child_tree_roundtrips_but_pi_tasks_stay_flat() {
        let dir = scratch_dir("tree");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        let parent = store.begin_task("parent", "parent task", &launch_pi(false));
        let mut child_launch = launch_pi(false);
        child_launch.parent_id = Some(parent.id.clone());
        let child = store.begin_task("child", "child task", &child_launch);
        assert_eq!(child.parent_id.as_deref(), Some("parent"));
        assert!(store.task("parent").expect("parent").parent_id.is_none());
        let tree = store.tree(0);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].task.id, "parent");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].task.id, "child");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn duplicate_run_ids_get_a_suffix() {
        let dir = scratch_dir("ids");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        let first = store.begin_task("same", "one", &launch_pi(false));
        let second = store.begin_task("same", "two", &launch_pi(false));
        assert_eq!(first.id, "same");
        assert_eq!(second.id, "same-2");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restart_history_is_recorded_from_fallback_events() {
        let dir = scratch_dir("restarts");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        store.begin_task(
            "run-10",
            "fallback",
            &AgentTaskLaunch::for_pydantic(None, false, None),
        );
        store.apply_event("run-10", &started("run-10"));
        store.apply_event(
            "run-10",
            &AgentEvent::Error {
                code: "sidecar_restarted".into(),
                message: "agent sidecar exited unexpectedly; restarted once".into(),
            },
        );
        let task = store.task("run-10").expect("task");
        assert_eq!(task.restarts.len(), 1);
        assert!(task.restarts[0].reason.contains("restarted"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn log_ring_is_bounded() {
        let dir = scratch_dir("logring");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        store.begin_task("run-11", "tool heavy", &launch_pi(false));
        store.apply_event("run-11", &started("run-11"));
        for index in 0..(MAX_LOG_ENTRIES + 25) {
            store.apply_event(
                "run-11",
                &AgentEvent::Progress {
                    message: format!("tool: read {index}"),
                    fraction: None,
                },
            );
        }
        let task = store.task("run-11").expect("task");
        assert!(task.log.len() <= MAX_LOG_ENTRIES, "{}", task.log.len());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stop_all_marks_active_tasks_and_leaves_terminal_ones() {
        let dir = scratch_dir("stopall");
        let store = AgentTaskStore::open(&dir, Vec::new()).unwrap();
        store.begin_task("active", "active", &launch_pi(false));
        store.apply_event("active", &started("active"));
        let done = store.begin_task("done", "done", &launch_pi(false));
        store.apply_event(&done.id, &started(&done.id));
        store.apply_event(&done.id, &finished(true));
        let cancelled = store.stop_all("app quitting");
        assert_eq!(cancelled.len(), 1);
        assert_eq!(cancelled[0].id, "active");
        assert_eq!(
            store.task("active").expect("active").state,
            AgentTaskState::Cancelled
        );
        assert_eq!(store.task("done").expect("done").state, AgentTaskState::Completed);
        let _ = fs::remove_dir_all(dir);
    }
}
