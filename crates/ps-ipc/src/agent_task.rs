//! Agent task DTOs (V10 single source of truth).
//!
//! These types were previously defined in crates/ps-agent/src/task/model.rs. They
//! now live in ps-ipc so the Rust engine, the Tauri shell and the TypeScript
//! bridge all share one contract:
//!
//! * all structs serialize camelCase, all enums snake_case (wire format);
//! * JSON shapes are byte-identical to the pre-move definitions; the contract
//!   tests below pin two full AgentTask fixtures;
//! * ps-agent re-exports these types (its crate-root API is unchanged), and
//!   apps/desktop/src/bridge/types.ts is the mirrored TypeScript contract.
//!
//! Persistence, redaction, lifecycle mapping, runtime launches and the
//! AgentTaskService remain in crates/ps-agent/src/task (policy, not wire).

use serde::{Deserialize, Serialize};

/// Per-task log ring size on the wire and in persisted task records.
pub const MAX_LOG_ENTRIES: usize = 100;

/// Which agent runtime produced the task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskRuntime {
    /// Pi coding-agent CLI (default).
    Pi,
    /// Embedded Pydantic AI sidecar (optional fallback).
    Pydantic,
    /// Unknown/legacy producer (including the CLI agents older builds ran);
    /// keep the record rather than dropping it.
    #[default]
    #[serde(other)]
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
    /// Pi's final answer, kept in memory for the island chat only: the task
    /// history file never stores assistant output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
}

impl AgentTask {
    /// Tree node for the desktop Agents view (children are populated by
    /// ps-agent's task store, which owns persistence policy).
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

#[cfg(test)]
mod tests {
    use super::*;

    // Frozen pre-move serialization fixtures (captured from
    // crates/ps-agent/src/task/model.rs before the DTO move). These pin the
    // byte-identical wire format.
    const TASK_FIXTURE: &str = r#"{"id":"run-1","runtime":"pi","runtimeVersion":"1.5.0","parentId":null,"title":"Find the budget","promptSummary":"find the budget file","state":"running","progress":{"kind":"indeterminate","fraction":null},"createdAt":100,"startedAt":101,"endedAt":null,"updatedAt":102,"log":[],"error":null,"permissions":{"requested":[],"granted":[],"denied":[],"pending":null,"readOnly":false},"writeAccess":false,"controls":{"canCancel":true,"canPause":false,"canResume":false,"canRetry":false},"restarts":[],"model":"fake-model"}"#;
    const TASK_FIXTURE_FULL: &str = r#"{"id":"run-2","runtime":"pi","runtimeVersion":"1.5.0","parentId":null,"title":"Find the budget","promptSummary":"find the budget file","state":"failed","progress":{"kind":"determinate","fraction":0.5},"createdAt":100,"startedAt":101,"endedAt":200,"updatedAt":200,"log":[{"at":100,"level":"info","message":"started pi 1.5.0"},{"at":150,"level":"warn","message":"permission requested: Search your files"},{"at":200,"level":"error","message":"provider_error: boom"}],"error":{"code":"provider_error","message":"boom"},"permissions":{"requested":["Search your files"],"granted":[],"denied":["Search your files"],"pending":null,"readOnly":false},"writeAccess":true,"controls":{"canCancel":false,"canPause":false,"canResume":false,"canRetry":true},"restarts":[{"at":175,"reason":"sidecar restarted"}],"model":"fake-model"}"#;

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
            answer: None,
        }
    }

    #[test]
    fn agent_task_fixture_is_byte_identical() {
        let task = sample_task();
        assert_eq!(serde_json::to_string(&task).unwrap(), TASK_FIXTURE);
        let parsed: AgentTask = serde_json::from_str(TASK_FIXTURE).unwrap();
        assert_eq!(parsed, task);
    }

    #[test]
    fn full_agent_task_fixture_is_byte_identical() {
        let mut full = sample_task();
        full.id = "run-2".into();
        full.state = AgentTaskState::Failed;
        full.progress = AgentTaskProgress::determinate(0.5);
        full.ended_at = Some(200);
        full.updated_at = 200;
        full.write_access = true;
        full.permissions = AgentPermissionState {
            requested: vec!["Search your files".into()],
            granted: vec![],
            denied: vec!["Search your files".into()],
            pending: None,
            read_only: false,
        };
        full.log = vec![
            AgentTaskLogEntry {
                at: 100,
                level: AgentTaskLogLevel::Info,
                message: "started pi 1.5.0".into(),
            },
            AgentTaskLogEntry {
                at: 150,
                level: AgentTaskLogLevel::Warn,
                message: "permission requested: Search your files".into(),
            },
            AgentTaskLogEntry {
                at: 200,
                level: AgentTaskLogLevel::Error,
                message: "provider_error: boom".into(),
            },
        ];
        full.error = Some(AgentTaskErrorInfo {
            code: "provider_error".into(),
            message: "boom".into(),
        });
        full.restarts = vec![AgentTaskRestart {
            at: 175,
            reason: "sidecar restarted".into(),
        }];
        full.controls = AgentTaskControls::for_state(AgentTaskState::Failed);
        assert_eq!(serde_json::to_string(&full).unwrap(), TASK_FIXTURE_FULL);
        let parsed: AgentTask = serde_json::from_str(TASK_FIXTURE_FULL).unwrap();
        assert_eq!(parsed, full);
    }

    #[test]
    fn older_records_with_optional_fields_missing_still_parse() {
        let minimal =
            r#"{"id":"old","title":"t","promptSummary":"p","state":"queued","createdAt":1,"updatedAt":1}"#;
        let task: AgentTask = serde_json::from_str(minimal).unwrap();
        assert_eq!(task.id, "old");
        assert_eq!(task.runtime, AgentTaskRuntime::Unknown);
        let legacy: AgentTask = serde_json::from_str(&minimal.replace("{", r#"{"runtime":"codex","#)).unwrap();
        assert_eq!(legacy.runtime, AgentTaskRuntime::Unknown, "removed CLI runtimes still load");
        assert_eq!(task.progress.kind, AgentTaskProgressKind::Indeterminate);
        assert!(task.log.is_empty());
        assert!(task.restarts.is_empty());
        assert!(task.parent_id.is_none());
        assert!(!task.controls.can_pause);
    }

    #[test]
    fn enums_and_progress_strings_are_stable() {
        assert_eq!(
            serde_json::to_string(&AgentTaskState::Cancelled).unwrap(),
            "\"cancelled\""
        );
        assert_eq!(
            serde_json::to_string(&AgentTaskRuntime::Pydantic).unwrap(),
            "\"pydantic\""
        );
        assert_eq!(
            serde_json::to_string(&AgentTaskLogLevel::Warn).unwrap(),
            "\"warn\""
        );
        assert_eq!(
            serde_json::to_string(&AgentTaskProgressKind::Determinate).unwrap(),
            "\"determinate\""
        );
        assert_eq!(AgentTaskProgress::determinate(1.5).fraction, Some(1.0));
        assert!(AgentTaskProgress::indeterminate().fraction.is_none());
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
    fn task_node_serializes_nested_tasks() {
        let node = AgentTaskNode {
            task: sample_task(),
            children: vec![AgentTaskNode {
                task: sample_task(),
                children: Vec::new(),
            }],
        };
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"children\":[{\"task\":{"), "{json}");
        assert!(json.contains("\"canResume\":false"), "{json}");
    }
}
