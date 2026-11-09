//! Agent task persistence helpers and ps-ipc DTO re-exports.
//!
//! V10: the wire DTOs (AgentTask, AgentTaskNode, AgentPermissionState, progress/
//! log/restart/controls/activity types and DetectedAgent) live in
//! ps-ipc::agent_task as the single source of truth. This module re-exports them
//! so the existing ps_agent public API keeps working
//! (ps_agent::AgentTask == ps_ipc::agent_task::AgentTask), and owns the
//! persistence policy plus redaction/projection helpers used by the task store
//! and service. JSON shapes are unchanged (ps-ipc contract fixtures pin them).

use crate::pi::redact_text;

pub use ps_ipc::agent_task::{
    AgentPermissionState, AgentTask, AgentTaskActivity, AgentTaskControls, AgentTaskErrorInfo,
    AgentTaskLogEntry, AgentTaskLogLevel, AgentTaskNode, AgentTaskProgress, AgentTaskProgressKind,
    AgentTaskRestart, AgentTaskRuntime, AgentTaskState, MAX_LOG_ENTRIES,
};

/// Persistence schema version for <data_dir>/agents/tasks.json.
pub const TASKS_SCHEMA_VERSION: u32 = 1;
/// Newest tasks kept in the store/history file; older terminal tasks are pruned.
pub const MAX_PERSISTED_TASKS: usize = 200;
/// Redacted title length (shown on the Agents card).
pub const MAX_TITLE_CHARS: usize = 80;
/// Redacted prompt-summary length.
pub const MAX_SUMMARY_CHARS: usize = 200;
/// One log line length.
pub const MAX_LOG_CHARS: usize = 300;
/// Stored error message length.
pub const MAX_ERROR_CHARS: usize = 400;

/// Launch parameters for a new task, independent of the runtime client. The DTO
/// itself (AgentTask) is in ps-ipc; this is host-side launch policy and stays in
/// ps-agent.
#[derive(Debug, Clone)]
pub struct AgentTaskLaunch {
    pub runtime: AgentTaskRuntime,
    pub runtime_version: Option<String>,
    pub model: Option<String>,
    pub allow_writes: bool,
    /// Parent id for runtimes that expose sub-agents (Pi does not).
    pub parent_id: Option<String>,
}

impl AgentTaskLaunch {
    pub fn for_pi(
        info: &crate::pi::PiInfo,
        model: Option<String>,
        allow_writes: bool,
        parent_id: Option<String>,
    ) -> Self {
        Self {
            runtime: AgentTaskRuntime::Pi,
            runtime_version: Some(info.version.clone()),
            model,
            allow_writes,
            parent_id,
        }
    }

    pub fn for_pydantic(
        model: Option<String>,
        allow_writes: bool,
        parent_id: Option<String>,
    ) -> Self {
        Self {
            runtime: AgentTaskRuntime::Pydantic,
            runtime_version: None,
            model,
            allow_writes,
            parent_id,
        }
    }
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit).collect();
    out.push('…');
    out
}

/// Redact then cap. Used for everything the store persists.
pub(crate) fn redact_truncate(text: &str, redactions: &[String], limit: usize) -> String {
    truncate_chars(&redact_text(text, redactions), limit)
}

/// Redacted (title, summary) for a prompt. Raw prompt text is never stored.
pub(crate) fn summarize_prompt(prompt: &str, redactions: &[String]) -> (String, String) {
    let cleaned = prompt.trim();
    let summary = redact_truncate(cleaned, redactions, MAX_SUMMARY_CHARS);
    let mut title = redact_truncate(cleaned, redactions, MAX_TITLE_CHARS);
    if title.trim().is_empty() {
        title = "Agent run".to_string();
    }
    (title, summary)
}

/// Activity/search projection for one task (engine integration follow-up).
pub fn task_activity(task: &AgentTask) -> AgentTaskActivity {
    let detail = format!(
        "{} · {} · {}",
        task.runtime.label(),
        task.state.as_str(),
        if task.write_access {
            "write access"
        } else {
            "read-only"
        }
    );
    let mut search_text = format!(
        "{} {} {} {}",
        task.title,
        task.prompt_summary,
        task.runtime.label(),
        task.state.as_str()
    );
    if let Some(model) = &task.model {
        search_text.push(' ');
        search_text.push_str(model);
    }
    AgentTaskActivity {
        task_id: task.id.clone(),
        timestamp: task.ended_at.unwrap_or(task.updated_at),
        title: task.title.clone(),
        detail,
        state: task.state,
        runtime: task.runtime,
        search_text: search_text.to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task() -> AgentTask {
        AgentTask {
            id: "run-1".into(),
            runtime: AgentTaskRuntime::Pi,
            runtime_version: Some("1.5.0".into()),
            parent_id: None,
            title: "Find the budget".into(),
            prompt_summary: "find the budget file".into(),
            state: AgentTaskState::Running,
            progress: AgentTaskProgress::indeterminate(),
            created_at: 100,
            started_at: Some(101),
            ended_at: None,
            updated_at: 102,
            log: Vec::new(),
            error: None,
            permissions: AgentPermissionState::default(),
            write_access: false,
            controls: AgentTaskControls::for_state(AgentTaskState::Running),
            restarts: Vec::new(),
            model: Some("fake-model".into()),
        }
    }

    #[test]
    fn dtos_are_the_ps_ipc_types() {
        // Same nominal type, not a ps-agent copy: this compiles only because the
        // re-export resolves to ps_ipc::agent_task::AgentTask.
        fn expect_ps_ipc(task: ps_ipc::agent_task::AgentTask) -> ps_ipc::agent_task::AgentTask {
            task
        }
        let task: AgentTask = sample_task();
        let back: ps_ipc::agent_task::AgentTask = expect_ps_ipc(task.clone());
        assert_eq!(back, task);
        let node: ps_ipc::agent_task::AgentTaskNode = task.node();
        assert_eq!(node.task.id, "run-1");
    }

    #[test]
    fn serde_is_camel_case_and_roundtrips() {
        let task = sample_task();
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"parentId\":null"), "{json}");
        assert!(json.contains("\"promptSummary\""), "{json}");
        assert!(json.contains("\"writeAccess\":false"), "{json}");
        assert!(json.contains("\"runtimeVersion\":\"1.5.0\""), "{json}");
        assert!(json.contains("\"state\":\"running\""), "{json}");
        let back: AgentTask = serde_json::from_str(&json).unwrap();
        assert_eq!(back, task);
    }

    #[test]
    fn runtime_label_parsing_extracts_version() {
        assert_eq!(
            AgentTaskRuntime::from_started_label("pi 1.5.0"),
            (AgentTaskRuntime::Pi, Some("1.5.0".into()))
        );
        assert_eq!(
            AgentTaskRuntime::from_started_label("pydantic-ai 2.66.0"),
            (AgentTaskRuntime::Pydantic, Some("2.66.0".into()))
        );
        assert_eq!(
            AgentTaskRuntime::from_started_label("something else"),
            (AgentTaskRuntime::Unknown, None)
        );
    }

    #[test]
    fn controls_never_offer_pause_or_resume() {
        for state in [
            AgentTaskState::Queued,
            AgentTaskState::Running,
            AgentTaskState::Completed,
            AgentTaskState::Failed,
            AgentTaskState::Cancelled,
            AgentTaskState::Offline,
        ] {
            let controls = AgentTaskControls::for_state(state);
            assert!(!controls.can_pause);
            assert!(!controls.can_resume);
            assert_eq!(
                controls.can_cancel,
                matches!(state, AgentTaskState::Queued | AgentTaskState::Running)
            );
        }
    }

    #[test]
    fn progress_determinate_is_clamped_and_indeterminate_has_no_fraction() {
        assert_eq!(
            AgentTaskProgress::indeterminate(),
            AgentTaskProgress {
                kind: AgentTaskProgressKind::Indeterminate,
                fraction: None
            }
        );
        assert_eq!(AgentTaskProgress::determinate(1.7).fraction, Some(1.0));
        assert_eq!(AgentTaskProgress::determinate(-0.5).fraction, Some(0.0));
    }

    #[test]
    fn prompt_summary_is_redacted_and_capped() {
        let redactions = vec!["sk-live-supersecret-abcdef".to_string()];
        let prompt = format!(
            "please use sk-live-supersecret-abcdef and AIzaSyD-EXAMPLEFAKEKEY0123456789abcdefg {}",
            "x".repeat(500)
        );
        let (title, summary) = summarize_prompt(&prompt, &redactions);
        assert!(!title.contains("sk-live-supersecret"), "{title}");
        assert!(!summary.contains("sk-live-supersecret"), "{summary}");
        assert!(!summary.contains("AIzaSyD-EXAMPLEFAKEKEY"), "{summary}");
        assert!(title.chars().count() <= MAX_TITLE_CHARS + 1);
        assert!(summary.chars().count() <= MAX_SUMMARY_CHARS + 1);
    }

    #[test]
    fn activity_projection_has_searchable_lower_case_text() {
        let mut task = sample_task();
        task.state = AgentTaskState::Completed;
        task.ended_at = Some(500);
        let activity = task_activity(&task);
        assert_eq!(activity.timestamp, 500);
        assert!(
            activity.detail.contains("pi · completed · read-only"),
            "{}",
            activity.detail
        );
        assert!(activity.search_text.contains("pi"));
        assert!(activity.search_text.contains("completed"));
        assert_eq!(activity.search_text, activity.search_text.to_lowercase());
    }
}
