//! Runtime-agnostic agent task model for the V9 Agents card / desktop view.
//!
//! These types are deliberately independent of Pi/Pydantic: any runtime whose
//! lifecycle can be expressed as the frozen ps-ipc AgentEvent stream maps into
//! the same task record. The model is designed so it can be lifted into ps-ipc
//! unchanged (all structs serialize camelCase, enums snake_case) — the proposed
//! IPC additions are listed in docs/agent/AGENT_CARD_API.md.
//!
//! Privacy by default: only a redacted title/summary of the prompt is stored;
//! the prompt text, model output, tool-result payloads and raw logs are never
//! persisted.

use serde::{Deserialize, Serialize};

use crate::pi::redact_text;

/// Persistence schema version for the tasks file.
pub const TASKS_SCHEMA_VERSION: u32 = 1;
/// Newest tasks kept in the store/history file; older terminal tasks are pruned.
pub const MAX_PERSISTED_TASKS: usize = 200;
/// Per-task redacted log ring size.
pub const MAX_LOG_ENTRIES: usize = 100;
/// Redacted title length (shown on the Agents card).
pub const MAX_TITLE_CHARS: usize = 80;
/// Redacted prompt-summary length.
pub const MAX_SUMMARY_CHARS: usize = 200;
/// One log line length.
pub const MAX_LOG_CHARS: usize = 300;
/// Stored error message length.
pub const MAX_ERROR_CHARS: usize = 400;

/// Which agent runtime produced the task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskRuntime {
    /// Pi coding-agent CLI (default).
    Pi,
    /// Embedded Pydantic AI sidecar (optional fallback).
    Pydantic,
    /// Unknown/legacy producer; keep the record rather than dropping it.
    #[default]
    Unknown,
}

impl AgentTaskRuntime {
    pub fn label(self) -> &'static str {
        match self {
            AgentTaskRuntime::Pi => "pi",
            AgentTaskRuntime::Pydantic => "pydantic",
            AgentTaskRuntime::Unknown => "unknown",
        }
    }

    /// Map the runtime label from AgentEvent::Started ("pi 1.5.0",
    /// "pydantic-ai 2.66.0") to (runtime, version).
    pub fn from_started_label(label: &str) -> (Self, Option<String>) {
        if let Some(rest) = label.strip_prefix("pi ") {
            let version = rest.trim();
            return (
                AgentTaskRuntime::Pi,
                (!version.is_empty()).then(|| version.to_string()),
            );
        }
        if label.to_ascii_lowercase().contains("pydantic") {
            let version = label
                .split_whitespace()
                .last()
                .filter(|token| token.chars().any(|ch| ch.is_ascii_digit()))
                .map(str::to_string);
            return (AgentTaskRuntime::Pydantic, version);
        }
        (AgentTaskRuntime::Unknown, None)
    }
}

/// Lifecycle state of one agent task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskState {
    /// Accepted by the host, process not started yet.
    #[default]
    Queued,
    /// Runtime reported "started" (or the host launched the process).
    Running,
    Completed,
    Failed,
    Cancelled,
    /// Runtime unavailable, or the app restarted before the task finished.
    Offline,
}

impl AgentTaskState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            AgentTaskState::Completed
                | AgentTaskState::Failed
                | AgentTaskState::Cancelled
                | AgentTaskState::Offline
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AgentTaskState::Queued => "queued",
            AgentTaskState::Running => "running",
            AgentTaskState::Completed => "completed",
            AgentTaskState::Failed => "failed",
            AgentTaskState::Cancelled => "cancelled",
            AgentTaskState::Offline => "offline",
        }
    }
}

/// How progress is represented. Determinate is only allowed when the runtime
/// reports a real fraction; Pi never does, so Pi tasks stay indeterminate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskProgressKind {
    #[default]
    Indeterminate,
    Determinate,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskProgress {
    pub kind: AgentTaskProgressKind,
    /// Real fraction in 0.0..=1.0; None for indeterminate progress.
    #[serde(default)]
    pub fraction: Option<f32>,
}

impl Default for AgentTaskProgress {
    fn default() -> Self {
        Self {
            kind: AgentTaskProgressKind::Indeterminate,
            fraction: None,
        }
    }
}

impl AgentTaskProgress {
    pub fn indeterminate() -> Self {
        Self::default()
    }

    pub fn determinate(fraction: f32) -> Self {
        Self {
            kind: AgentTaskProgressKind::Determinate,
            fraction: Some(fraction.clamp(0.0, 1.0)),
        }
    }
}

/// Permission state for a task.
///
/// Pi is run-level: the user pre-authorises a tool allowlist in settings, so
/// requested == granted at launch and pending stays None. The Pydantic fallback
/// asks per action; its approval events populate requested/pending and the
/// following tool result resolves granted/denied.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPermissionState {
    #[serde(default)]
    pub requested: Vec<String>,
    #[serde(default)]
    pub granted: Vec<String>,
    #[serde(default)]
    pub denied: Vec<String>,
    /// Approval currently waiting for the user (fallback runtime only).
    #[serde(default)]
    pub pending: Option<String>,
    /// True when the run has no write/shell tools (default).
    #[serde(default)]
    pub read_only: bool,
}

/// Restart recorded from the runtime (fallback sidecar auto-restart etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskRestart {
    /// Unix milliseconds.
    pub at: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskLogLevel {
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskLogEntry {
    pub at: i64,
    pub level: AgentTaskLogLevel,
    /// Redacted, length-capped status text. Never raw output.
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskErrorInfo {
    pub code: String,
    pub message: String,
}

/// Which controls the UI may offer for this task in its current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskControls {
    pub can_cancel: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_retry: bool,
}

impl AgentTaskControls {
    pub fn for_state(state: AgentTaskState) -> Self {
        Self {
            can_cancel: matches!(state, AgentTaskState::Queued | AgentTaskState::Running),
            // Pi and the Pydantic sidecar have no pause/resume; never offer it.
            can_pause: false,
            can_resume: false,
            can_retry: matches!(
                state,
                AgentTaskState::Failed | AgentTaskState::Cancelled | AgentTaskState::Offline
            ),
        }
    }
}

/// One persisted agent task (runtime-agnostic).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTask {
    /// Stable id; equals the run id (a suffix is added on collision).
    pub id: String,
    #[serde(default)]
    pub runtime: AgentTaskRuntime,
    #[serde(default)]
    pub runtime_version: Option<String>,
    /// Parent task id where the runtime exposes child agents. Pi does not (flat
    /// single level); the field exists for future runtimes.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Redacted, short prompt title for the Agents card.
    pub title: String,
    /// Redacted prompt summary (no raw prompt is stored).
    pub prompt_summary: String,
    pub state: AgentTaskState,
    #[serde(default)]
    pub progress: AgentTaskProgress,
    /// Unix milliseconds.
    pub created_at: i64,
    #[serde(default)]
    pub started_at: Option<i64>,
    #[serde(default)]
    pub ended_at: Option<i64>,
    pub updated_at: i64,
    #[serde(default)]
    pub log: Vec<AgentTaskLogEntry>,
    #[serde(default)]
    pub error: Option<AgentTaskErrorInfo>,
    #[serde(default)]
    pub permissions: AgentPermissionState,
    /// True when edit/write/bash/powershell were enabled for this run.
    #[serde(default)]
    pub write_access: bool,
    #[serde(default)]
    pub controls: AgentTaskControls,
    #[serde(default)]
    pub restarts: Vec<AgentTaskRestart>,
    #[serde(default)]
    pub model: Option<String>,
}

impl AgentTask {
    pub(crate) fn push_log(&mut self, level: AgentTaskLogLevel, message: String, at: i64) {
        self.log.push(AgentTaskLogEntry { at, level, message });
        while self.log.len() > MAX_LOG_ENTRIES {
            self.log.remove(0);
        }
    }

    pub(crate) fn refresh_controls(&mut self) {
        self.controls = AgentTaskControls::for_state(self.state);
    }

    /// Tree node for the desktop Agents view (children populated by the store).
    pub fn node(&self) -> AgentTaskNode {
        AgentTaskNode {
            task: self.clone(),
            children: Vec::new(),
        }
    }
}

/// Nested representation of the parent/child tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskNode {
    pub task: AgentTask,
    #[serde(default)]
    pub children: Vec<AgentTaskNode>,
}

/// Projection used to add agent history to Activity/search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskActivity {
    pub task_id: String,
    /// Unix milliseconds of the last state change.
    pub timestamp: i64,
    pub title: String,
    /// e.g. "pi · completed · read-only".
    pub detail: String,
    pub state: AgentTaskState,
    pub runtime: AgentTaskRuntime,
    /// Lower-case text the engine can feed into its activity/content index.
    pub search_text: String,
}

/// Launch parameters for a new task, independent of the runtime client.
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

    pub fn for_pydantic(model: Option<String>, allow_writes: bool, parent_id: Option<String>) -> Self {
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

/// Activity/search projection for one task.
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
