//! High-level agent task service: store + change notifications + runtime launch
//! adapters used by the Tauri shell.
//!
//! The shell owns the actual runtime clients (PiAgentClient / AgentClient). This
//! service owns durable task state and is runtime-agnostic: it is told which
//! runtime a run uses, then fed the frozen AgentEvent stream. Every visible
//! change invokes the listener so the shell can emit a single
//! "agent:task" event with the updated snapshot.

use std::sync::{Arc, Mutex};

use ps_ipc::{AgentEvent, AgentRunRequest};

use crate::error::AgentError;
use crate::pi::PiInfo;

use super::model::*;
use super::store::AgentTaskStore;

/// Change listener invoked with the updated task snapshot after every visible
/// change. Called while no internal lock is held; panics are caught so a bad
/// listener cannot kill the runtime reader thread.
pub type AgentTaskListener = Arc<dyn Fn(&AgentTask) + Send + Sync + 'static>;

/// Durable, runtime-agnostic agent task service for the V9 Agents card and the
/// desktop Agents view.
pub struct AgentTaskService {
    store: AgentTaskStore,
    listener: Mutex<Option<AgentTaskListener>>,
}

impl AgentTaskService {
    /// Open the task history under <data_dir>/agents/tasks.json.
    /// Redactions are applied to everything persisted (prompt summary, logs,
    /// errors).
    pub fn open(data_dir: &std::path::Path, redactions: Vec<String>) -> Result<Self, AgentError> {
        Ok(Self {
            store: AgentTaskStore::open(data_dir, redactions)?,
            listener: Mutex::new(None),
        })
    }

    pub fn with_listener(self, listener: AgentTaskListener) -> Self {
        if let Ok(mut slot) = self.listener.lock() {
            *slot = Some(listener);
        }
        self
    }

    pub fn store(&self) -> &AgentTaskStore {
        &self.store
    }

    /// Create a Queued task for a run. The returned task id is canonical (a
    /// suffix is appended if an older task used the same run id); feed events
    /// with that id, not necessarily the request run id.
    pub fn begin(
        &self,
        request: &AgentRunRequest,
        launch: &AgentTaskLaunch,
    ) -> Result<AgentTask, AgentError> {
        if let Some(parent) = &launch.parent_id {
            if self.store.task(parent).is_none() {
                return Err(AgentError::TaskNotFound(parent.clone()));
            }
        }
        let task = self.store.begin_task(&request.run_id, &request.prompt, launch);
        self.notify(&task);
        Ok(task)
    }

    /// Convenience for the default Pi runtime.
    pub fn begin_pi(
        &self,
        request: &AgentRunRequest,
        info: &PiInfo,
        model: Option<String>,
        allow_writes: bool,
        parent_id: Option<String>,
    ) -> Result<AgentTask, AgentError> {
        self.begin(
            request,
            &AgentTaskLaunch::for_pi(info, model, allow_writes, parent_id),
        )
    }

    /// Convenience for the optional Pydantic fallback runtime.
    pub fn begin_pydantic(
        &self,
        request: &AgentRunRequest,
        model: Option<String>,
        allow_writes: bool,
    ) -> Result<AgentTask, AgentError> {
        self.begin(request, &AgentTaskLaunch::for_pydantic(model, allow_writes, None))
    }

    /// Feed one runtime event into the task. Returns the updated snapshot when
    /// the card-visible state changed.
    pub fn apply_event(&self, task_id: &str, event: &AgentEvent) -> Option<AgentTask> {
        let task = self.store.apply_event(task_id, event);
        if let Some(task) = &task {
            self.notify(task);
        }
        task
    }

    /// Mark a queued task failed (e.g. the runtime failed to spawn).
    pub fn mark_launch_failed(&self, task_id: &str, code: &str, message: &str) -> Option<AgentTask> {
        let task = self.store.mark_launch_failed(task_id, code, message);
        if let Some(task) = &task {
            self.notify(task);
        }
        task
    }

    /// Mark a task offline (runtime not installed/unavailable).
    pub fn mark_offline(&self, task_id: &str, message: &str) -> Option<AgentTask> {
        let task = self.store.mark_offline(task_id, message);
        if let Some(task) = &task {
            self.notify(task);
        }
        task
    }

    /// Mark the task cancelled before/after asking the runtime to kill the
    /// process. Returns the snapshot to display (and whose run id identifies the
    /// process to kill); None if the task already finished.
    /// Remove a task and its follow-ups from the history.
    pub fn delete(&self, task_id: &str) -> usize {
        self.store.delete(task_id)
    }

    /// Rename a task's title. None for an unknown id or a blank title.
    pub fn rename(&self, task_id: &str, title: &str) -> Option<AgentTask> {
        let task = self.store.rename(task_id, title);
        if let Some(task) = &task {
            self.notify(task);
        }
        task
    }

    pub fn cancel(&self, task_id: &str, reason: &str) -> Option<AgentTask> {
        let task = self.store.cancel(task_id, reason);
        if let Some(task) = &task {
            self.notify(task);
        }
        task
    }

    /// Mark every active task cancelled (app quit / tray stop). Returns the
    /// snapshots; the caller kills the matching processes.
    pub fn stop_all(&self, reason: &str) -> Vec<AgentTask> {
        let tasks = self.store.stop_all(reason);
        for task in &tasks {
            self.notify(task);
        }
        tasks
    }

    pub fn tasks(&self, limit: usize) -> Vec<AgentTask> {
        self.store.tasks(limit)
    }

    pub fn tree(&self, limit: usize) -> Vec<AgentTaskNode> {
        self.store.tree(limit)
    }

    pub fn task(&self, id: &str) -> Option<AgentTask> {
        self.store.task(id)
    }

    pub fn permissions(&self, id: &str) -> Option<AgentPermissionState> {
        self.store.permissions(id)
    }

    /// Honest unsupported controls: neither current runtime can pause/resume a
    /// run. The UI must show these controls disabled.
    pub fn pause(&self, _id: &str) -> Result<(), AgentError> {
        Err(AgentError::Unsupported(
            "pause is not supported by the pi or pydantic runtime".into(),
        ))
    }

    pub fn resume(&self, _id: &str) -> Result<(), AgentError> {
        Err(AgentError::Unsupported(
            "resume is not supported by the pi or pydantic runtime".into(),
        ))
    }

    /// Activity/search projection for the engine's activity history.
    pub fn activity(&self, limit: usize) -> Vec<AgentTaskActivity> {
        self.store.tasks(limit).iter().map(task_activity).collect()
    }

    /// Best-effort persistence state (None when the last write succeeded).
    pub fn last_persist_error(&self) -> Option<String> {
        self.store.last_persist_error()
    }

    fn notify(&self, task: &AgentTask) {
        let listener = self
            .listener
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone();
        if let Some(listener) = listener {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| listener(task)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn scratch_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join(format!("ps-agent-service-test-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn request(run_id: &str, prompt: &str) -> AgentRunRequest {
        AgentRunRequest {
            run_id: run_id.into(),
            prompt: prompt.into(),
            context_paths: Vec::new(),
            model: None,
            max_steps: None,
        }
    }

    fn info() -> PiInfo {
        PiInfo {
            path: "pi.cmd".into(),
            version: "1.5.0".into(),
            node_ok: true,
            source: crate::pi::PiSource::Managed,
        }
    }

    #[test]
    fn listener_fires_for_visible_changes_only() {
        let dir = scratch_dir("listener");
        let counter = Arc::new(AtomicUsize::new(0));
        let sink = Arc::clone(&counter);
        let service = AgentTaskService::open(&dir, Vec::new())
            .unwrap()
            .with_listener(Arc::new(move |_task: &AgentTask| {
                sink.fetch_add(1, Ordering::SeqCst);
            }));
        let task = service
            .begin_pi(&request("run-1", "find budget"), &info(), None, false, None)
            .expect("begin");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        service.apply_event(
            &task.id,
            &AgentEvent::Started {
                run_id: task.id.clone(),
                runtime: "pi 1.5.0".into(),
                model: "fake".into(),
            },
        );
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        // Assistant text delta: not a card-visible change.
        service.apply_event(
            &task.id,
            &AgentEvent::Progress {
                message: "streamed token".into(),
                fraction: None,
            },
        );
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        service.apply_event(
            &task.id,
            &AgentEvent::Finished {
                ok: true,
                duration_ms: 5,
            },
        );
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn parent_must_exist_for_child_tasks() {
        let dir = scratch_dir("parent");
        let service = AgentTaskService::open(&dir, Vec::new()).unwrap();
        let mut launch = AgentTaskLaunch::for_pi(&info(), None, false, None);
        launch.parent_id = Some("missing".into());
        let error = service
            .begin(&request("child", "child task"), &launch)
            .unwrap_err();
        assert!(matches!(error, AgentError::TaskNotFound(id) if id == "missing"));
        let parent = service
            .begin_pi(&request("parent", "parent"), &info(), None, false, None)
            .expect("parent");
        let child = service
            .begin_pi(
                &request("child-2", "child"),
                &info(),
                None,
                false,
                Some(parent.id.clone()),
            )
            .expect("child");
        assert_eq!(child.parent_id.as_deref(), Some("parent"));
        let tree = service.tree(0);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].children.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pause_and_resume_are_honestly_unsupported() {
        let dir = scratch_dir("pause");
        let service = AgentTaskService::open(&dir, Vec::new()).unwrap();
        let id = service
            .begin_pi(&request("run-2", "task"), &info(), None, false, None)
            .expect("begin")
            .id;
        assert!(matches!(service.pause(&id), Err(AgentError::Unsupported(_))));
        assert!(matches!(service.resume(&id), Err(AgentError::Unsupported(_))));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn stop_all_and_activity_projection_use_persisted_state() {
        let dir = scratch_dir("activity");
        let service = AgentTaskService::open(&dir, Vec::new()).unwrap();
        let running = service
            .begin_pi(&request("run-3", "find budget"), &info(), None, false, None)
            .expect("begin");
        service.apply_event(
            &running.id,
            &AgentEvent::Started {
                run_id: running.id.clone(),
                runtime: "pi 1.5.0".into(),
                model: "fake".into(),
            },
        );
        let activity = service.activity(10);
        assert_eq!(activity.len(), 1);
        assert_eq!(activity[0].state, AgentTaskState::Running);
        assert!(
            activity[0].detail.contains("pi · running"),
            "{}",
            activity[0].detail
        );
        let cancelled = service.stop_all("test stop");
        assert_eq!(cancelled.len(), 1);
        assert_eq!(
            service.task(&running.id).expect("task").state,
            AgentTaskState::Cancelled
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
