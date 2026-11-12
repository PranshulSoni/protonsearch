//! Map Pi's JSONL session-event stream onto the frozen ps-ipc AgentEvent set.
//!
//! Pi emits (see its docs/json.md):
//!   session, agent_start, turn_start, message_start, message_update,
//!   message_end, tool_execution_start/update/end, agent_end, agent_settled,
//!   queue_update, compaction_*, auto_retry_*, ...
//!
//! Mapping:
//!   session header                  -> Started { runtime: "pi <version>", model }
//!   message_update (text_delta)     -> Progress { message: delta }
//!   message_end (role assistant)    -> buffered; final text emitted at finish
//!   tool_execution_start            -> Progress { message: "tool: <name>" }
//!   tool_execution_end              -> ToolResult { tool, ok: !isError, preview }
//!   agent_settled / process exit    -> Output { final_answer: true } + Finished
//!   provider/auth errors / nonzero  -> Error { code, message } + Finished { ok: false }
//!
//! The mapper is pure (no I/O) so the mapping is unit-tested offline with
//! scripted JSONL.

use serde_json::Value;

use ps_ipc::AgentEvent;

use super::redact::redact_text;

/// Tool preview upper bound in characters (UTF-8 safe truncation).
pub const PREVIEW_CHARS: usize = 400;
const ERROR_CHARS: usize = 500;

/// Why the client stopped a Pi process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiKillReason {
    /// The host asked to cancel the run.
    Cancelled,
    /// No stdout event for the configured idle timeout.
    IdleTimeout,
    /// Maximum run duration exceeded.
    RunTimeout,
    /// The JSONL framing was broken (oversized line); process killed.
    ProtocolAbort,
}

/// Converts Pi JSONL lines into AgentEvent values for one run.
pub struct PiEventMapper {
    run_id: String,
    runtime: String,
    model: String,
    redactions: Vec<String>,
    started: bool,
    settled: Option<bool>,
    assistant_text: String,
    error_message: Option<String>,
    aborted: bool,
}

impl PiEventMapper {
    pub fn new(run_id: String, runtime: String, model: String, redactions: Vec<String>) -> Self {
        Self {
            run_id,
            runtime,
            model,
            redactions,
            started: false,
            settled: None,
            assistant_text: String::new(),
            error_message: None,
            aborted: false,
        }
    }

    /// Map one stdout line. Malformed JSON produces one protocol error event;
    /// unknown event types are ignored for forward compatibility.
    pub fn on_line(&mut self, line: &str) -> Vec<AgentEvent> {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            return Vec::new();
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                return vec![AgentEvent::Error {
                    code: "protocol_parse_error".into(),
                    message: format!(
                        "pi emitted malformed JSON ({} bytes): {} ({error})",
                        line.len(),
                        self.redacted_preview(line, 120)
                    ),
                }];
            }
        };
        let Some(object) = value.as_object() else {
            return vec![AgentEvent::Error {
                code: "protocol_parse_error".into(),
                message: "pi emitted a JSON value that is not an object".into(),
            }];
        };
        let event_type = object.get("type").and_then(Value::as_str).unwrap_or_default();
        match event_type {
            "session" => {
                if self.started {
                    return Vec::new();
                }
                self.started = true;
                vec![AgentEvent::Started {
                    run_id: self.run_id.clone(),
                    runtime: self.runtime.clone(),
                    model: self.model.clone(),
                }]
            }
            "message_update" => {
                let Some(update) = object.get("assistantMessageEvent") else {
                    return Vec::new();
                };
                match update.get("type").and_then(Value::as_str) {
                    Some("text_delta") => {
                        let delta = update.get("delta").and_then(Value::as_str).unwrap_or_default();
                        if delta.is_empty() {
                            Vec::new()
                        } else {
                            vec![AgentEvent::Progress {
                                message: delta.to_string(),
                                fraction: None,
                            }]
                        }
                    }
                    Some("error") => {
                        self.record_error(from_value(update.get("error")));
                        Vec::new()
                    }
                    _ => Vec::new(),
                }
            }
            "message_end" => {
                if let Some(message) = object.get("message") {
                    if message.get("role").and_then(Value::as_str) == Some("assistant") {
                        self.observe_assistant_message(message);
                    }
                }
                Vec::new()
            }
            "tool_execution_start" => {
                let name = object
                    .get("toolName")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                vec![AgentEvent::Progress {
                    message: format!("tool: {name}"),
                    fraction: None,
                }]
            }
            "tool_execution_end" => {
                let name = object
                    .get("toolName")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let is_error = object.get("isError").and_then(Value::as_bool).unwrap_or(false);
                let text = object
                    .get("result")
                    .map(extract_tool_result_text)
                    .unwrap_or_default();
                let preview = if text.trim().is_empty() {
                    if is_error {
                        "tool failed without output".to_string()
                    } else {
                        "tool completed without output".to_string()
                    }
                } else {
                    self.redacted_preview(&text, PREVIEW_CHARS)
                };
                vec![AgentEvent::ToolResult {
                    tool: name,
                    ok: !is_error,
                    preview,
                }]
            }
            "agent_settled" => {
                let aborted = object.get("aborted").and_then(Value::as_bool).unwrap_or(false);
                self.settled = Some(aborted);
                if aborted {
                    self.aborted = true;
                }
                Vec::new()
            }
            "auto_retry_start" => {
                let attempt = object.get("attempt").and_then(Value::as_u64).unwrap_or(0);
                let max_attempts = object.get("maxAttempts").and_then(Value::as_u64).unwrap_or(0);
                let reason = object
                    .get("errorMessage")
                    .and_then(Value::as_str)
                    .map(|text| self.redacted_preview(text, 160))
                    .unwrap_or_else(|| "provider error".to_string());
                vec![AgentEvent::Progress {
                    message: format!("provider retry {attempt}/{max_attempts}: {reason}"),
                    fraction: None,
                }]
            }
            "auto_retry_end" => {
                let success = object.get("success").and_then(Value::as_bool).unwrap_or(true);
                if !success {
                    self.record_error(
                        object
                            .get("finalError")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    );
                }
                Vec::new()
            }
            "compaction_start" => vec![AgentEvent::Progress {
                message: "compacting context".into(),
                fraction: None,
            }],
            "compaction_end" => {
                if let Some(error) = object.get("errorMessage").and_then(Value::as_str) {
                    self.record_error(Some(error.to_string()));
                }
                vec![AgentEvent::Progress {
                    message: "context compacted".into(),
                    fraction: None,
                }]
            }
            "extension_error" => {
                let message = object
                    .get("error")
                    .and_then(Value::as_str)
                    .map(|text| self.redacted_preview(text, ERROR_CHARS))
                    .unwrap_or_else(|| "pi extension reported an error".to_string());
                vec![AgentEvent::Error {
                    code: "extension_error".into(),
                    message,
                }]
            }
            // agent_start/agent_end/turn_*/message_start/tool_execution_update/
            // queue_update/thinking_level_changed/entry_appended/... carry no
            // frozen-event equivalent; ignore them explicitly.
            _ => Vec::new(),
        }
    }

    /// Final events for the run. Called exactly once when the process exits or
    /// the client kills it.
    pub fn finish(
        &mut self,
        kill: Option<PiKillReason>,
        duration_ms: u64,
        exit_code: Option<i32>,
        stderr_tail: &str,
    ) -> Vec<AgentEvent> {
        if let Some(reason) = kill {
            if reason == PiKillReason::ProtocolAbort {
                return vec![AgentEvent::Finished {
                    ok: false,
                    duration_ms,
                }];
            }
            let (code, message) = match reason {
                PiKillReason::Cancelled => ("cancelled", "run cancelled by the host".to_string()),
                PiKillReason::IdleTimeout => (
                    "idle_timeout",
                    "pi produced no events within the idle timeout; process tree killed".to_string(),
                ),
                PiKillReason::RunTimeout => (
                    "run_timeout",
                    "pi exceeded the maximum run duration; process tree killed".to_string(),
                ),
                PiKillReason::ProtocolAbort => unreachable!("handled above"),
            };
            return vec![
                AgentEvent::Error {
                    code: code.into(),
                    message,
                },
                AgentEvent::Finished {
                    ok: false,
                    duration_ms,
                },
            ];
        }

        let stderr_line = stderr_tail
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| self.redacted_preview(line, ERROR_CHARS));

        if let Some(raw_error) = self.error_message.take() {
            let message = self.redacted_preview(&raw_error, ERROR_CHARS);
            let code = classify_error(&message);
            return vec![
                AgentEvent::Error {
                    code: code.into(),
                    message,
                },
                AgentEvent::Finished {
                    ok: false,
                    duration_ms,
                },
            ];
        }
        if self.aborted {
            return vec![
                AgentEvent::Error {
                    code: "aborted".into(),
                    message: "pi reported that the run was aborted".into(),
                },
                AgentEvent::Finished {
                    ok: false,
                    duration_ms,
                },
            ];
        }
        match self.settled {
            Some(false) if exit_code == Some(0) => {
                // PI-SEC-04: the final answer is displayed in the UI and stored
                // in agent history, so it gets the same redaction treatment as
                // stderr, previews and errors.
                let text = redact_text(self.assistant_text.trim_end(), &self.redactions);
                vec![
                    AgentEvent::Output {
                        text,
                        final_answer: true,
                    },
                    AgentEvent::Finished {
                        ok: true,
                        duration_ms,
                    },
                ]
            }
            Some(false) => {
                let message = match (&stderr_line, exit_code) {
                    (Some(line), Some(code)) => format!("pi exited with code {code}: {line}"),
                    (Some(line), None) => format!("pi terminated: {line}"),
                    (None, Some(code)) => format!("pi exited with code {code} after agent_settled"),
                    (None, None) => "pi terminated after agent_settled".to_string(),
                };
                let code = classify_error(&message);
                vec![
                    AgentEvent::Error {
                        code: code.into(),
                        message,
                    },
                    AgentEvent::Finished {
                        ok: false,
                        duration_ms,
                    },
                ]
            }
            Some(true) => vec![
                AgentEvent::Error {
                    code: "aborted".into(),
                    message: "pi aborted the run (agent_settled aborted=true)".into(),
                },
                AgentEvent::Finished {
                    ok: false,
                    duration_ms,
                },
            ],
            None => {
                let message = match (&stderr_line, exit_code) {
                    (Some(line), _) => format!("pi exited without agent_settled: {line}"),
                    (None, Some(code)) => format!("pi exited with code {code} without agent_settled"),
                    (None, None) => "pi exited without agent_settled".to_string(),
                };
                let code = if classify_error(&message) == "pi_error" {
                    "pi_exit_error"
                } else {
                    classify_error(&message)
                };
                vec![
                    AgentEvent::Error {
                        code: code.into(),
                        message,
                    },
                    AgentEvent::Finished {
                        ok: false,
                        duration_ms,
                    },
                ]
            }
        }
    }

    fn observe_assistant_message(&mut self, message: &Value) {
        let text = extract_text(message.get("content").unwrap_or(&Value::Null));
        if !text.is_empty() {
            self.assistant_text = text;
        }
        match message.get("stopReason").and_then(Value::as_str) {
            Some("error") => {
                self.record_error(
                    message
                        .get("errorMessage")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                );
            }
            Some("aborted") => {
                self.aborted = true;
            }
            Some("length") => {
                self.record_error(Some(
                    "assistant response was cut off at the output token limit".to_string(),
                ));
            }
            _ => {}
        }
    }

    fn record_error(&mut self, message: Option<String>) {
        if self.error_message.is_none() {
            self.error_message = message.or_else(|| Some("pi reported an error".to_string()));
        }
    }

    fn redacted_preview(&self, text: &str, limit: usize) -> String {
        let redacted = redact_text(text, &self.redactions);
        truncate_chars(&redacted, limit)
    }
}

/// Tool results are either a plain string, a content-block array, or an
/// object carrying a content-block array (Pi's ToolResult shape).
fn extract_tool_result_text(value: &Value) -> String {
    match value {
        Value::Object(object) => object.get("content").map(extract_text).unwrap_or_default(),
        other => extract_text(other),
    }
}

fn extract_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn from_value(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => Some(text.clone()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(limit).collect();
    truncated.push('…');
    truncated
}

fn classify_error(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|needle| lower.contains(needle));
    if has(&[
        "auth",
        "credential",
        "api key",
        "api-key",
        "unauthorized",
        "forbidden",
        "401",
        "403",
        "login",
        "not logged",
        "token expired",
    ]) {
        "auth_error"
    } else if has(&[
        "rate limit",
        "rate-limit",
        "overloaded",
        "too many requests",
        "429",
        "quota",
        "provider",
        "model not found",
        "context length",
        "context window",
        "capacity",
    ]) {
        "provider_error"
    } else if has(&["truncat", "output token limit", "token limit"]) {
        "output_truncated"
    } else {
        "pi_error"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapper() -> PiEventMapper {
        PiEventMapper::new(
            "run-1".into(),
            "pi 1.5.0".into(),
            "fake-model".into(),
            vec!["sk-known-secret-1234567890".into()],
        )
    }

    #[test]
    fn happy_stream_maps_to_started_progress_tool_and_final_output() {
        let mut mapper = mapper();
        let script = [
            r#"{"type":"session","version":3,"id":"s","cwd":"C:/w"}"#,
            r#"{"type":"agent_start"}"#,
            r#"{"type":"message_start","message":{"role":"assistant","content":[],"stopReason":"pending"}}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"Hello "}}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"world"}}"#,
            r#"{"type":"message_end","message":{"role":"assistant","content":[{"type":"text","text":"Hello world"}],"stopReason":"stop"}}"#,
            r#"{"type":"tool_execution_start","toolCallId":"c1","toolName":"read","args":{"path":"a.txt"}}"#,
            r#"{"type":"tool_execution_end","toolCallId":"c1","toolName":"read","result":{"content":[{"type":"text","text":"file body"}]},"isError":false}"#,
            r#"{"type":"agent_settled","aborted":false}"#,
        ];
        let mut events = Vec::new();
        for line in script {
            events.extend(mapper.on_line(line));
        }
        assert!(matches!(
            &events[0],
            AgentEvent::Started { run_id, runtime, model }
                if run_id == "run-1" && runtime == "pi 1.5.0" && model == "fake-model"
        ));
        assert!(events
            .iter()
            .any(|event| matches!(event, AgentEvent::Progress { message, .. } if message == "Hello ")));
        assert!(events
            .iter()
            .any(|event| matches!(event, AgentEvent::Progress { message, .. } if message == "tool: read")));
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolResult { tool, ok: true, .. } if tool == "read"
        )));
        assert!(!events
            .iter()
            .any(|event| matches!(event, AgentEvent::Output { .. })));

        let finish = mapper.finish(None, 123, Some(0), "");
        assert!(matches!(
            &finish[0],
            AgentEvent::Output { text, final_answer: true } if text == "Hello world"
        ));
        assert!(matches!(
            finish[1],
            AgentEvent::Finished {
                ok: true,
                duration_ms: 123
            }
        ));
    }

    #[test]
    fn tool_error_and_preview_are_mapped_and_redacted() {
        let mut mapper = mapper();
        mapper.on_line(r#"{"type":"session"}"#);
        let events = mapper.on_line(
            r#"{"type":"tool_execution_end","toolCallId":"c1","toolName":"grep","result":{"content":[{"type":"text","text":"found sk-known-secret-1234567890 in file"}]},"isError":true}"#,
        );
        match &events[0] {
            AgentEvent::ToolResult { tool, ok, preview } => {
                assert_eq!(tool, "grep");
                assert!(!ok);
                assert!(preview.contains("[redacted]"), "{preview}");
                assert!(!preview.contains("sk-known-secret"), "{preview}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn provider_error_in_assistant_message_maps_to_error_and_finished_false() {
        let mut mapper = mapper();
        mapper.on_line(r#"{"type":"session"}"#);
        mapper.on_line(r#"{"type":"message_end","message":{"role":"assistant","content":[],"stopReason":"error","errorMessage":"401 invalid api key"}}"#);
        let finish = mapper.finish(None, 10, Some(1), "provider rejected the key");
        assert!(matches!(&finish[0], AgentEvent::Error { code, .. } if code == "auth_error"));
        assert!(matches!(
            finish[1],
            AgentEvent::Finished {
                ok: false,
                duration_ms: 10
            }
        ));
    }

    #[test]
    fn nonzero_exit_without_settled_is_reported() {
        let mut mapper = mapper();
        mapper.on_line(r#"{"type":"session"}"#);
        let finish = mapper.finish(None, 5, Some(2), "boom: credential missing");
        assert!(matches!(&finish[0], AgentEvent::Error { code, .. } if code == "auth_error"));
        assert!(matches!(finish[1], AgentEvent::Finished { ok: false, .. }));
    }

    #[test]
    fn cancel_maps_to_error_and_unfinished() {
        let mut mapper = mapper();
        mapper.on_line(r#"{"type":"session"}"#);
        let finish = mapper.finish(Some(PiKillReason::Cancelled), 5, None, "");
        assert!(matches!(&finish[0], AgentEvent::Error { code, .. } if code == "cancelled"));
        assert!(matches!(finish[1], AgentEvent::Finished { ok: false, .. }));
    }

    #[test]
    fn idle_timeout_and_run_timeout_have_distinct_codes() {
        for (reason, expected) in [
            (PiKillReason::IdleTimeout, "idle_timeout"),
            (PiKillReason::RunTimeout, "run_timeout"),
        ] {
            let mut mapper = mapper();
            let finish = mapper.finish(Some(reason), 5, None, "");
            assert!(matches!(&finish[0], AgentEvent::Error { code, .. } if code == expected));
        }
    }

    #[test]
    fn protocol_parse_error_does_not_stop_the_mapper() {
        let mut mapper = mapper();
        let events = mapper.on_line("{not json");
        assert!(matches!(&events[0], AgentEvent::Error { code, .. } if code == "protocol_parse_error"));
        let events = mapper.on_line(r#"{"type":"session"}"#);
        assert!(matches!(&events[0], AgentEvent::Started { .. }));
    }

    #[test]
    fn abandoned_stream_without_settled_is_an_error() {
        let mut mapper = mapper();
        mapper.on_line(r#"{"type":"session"}"#);
        let finish = mapper.finish(None, 5, Some(0), "");
        assert!(matches!(&finish[0], AgentEvent::Error { code, .. } if code == "pi_exit_error"));
    }

    #[test]
    fn final_output_is_redacted_with_exact_and_shaped_secrets() {
        let mut mapper = mapper();
        mapper.on_line(r#"{"type":"session"}"#);
        let text = "known sk-known-secret-1234567890 and AIzaSyD-EXAMPLEFAKEKEY0123456789abcdefg plus api_key=abcdefgh12345678";
        let message = serde_json::json!({
            "type": "message_end",
            "message": {"role": "assistant", "content": [{"type": "text", "text": text}], "stopReason": "stop"}
        });
        let _ = mapper.on_line(&message.to_string());
        let _ = mapper.on_line(r#"{"type":"agent_settled","aborted":false}"#);
        let finish = mapper.finish(None, 10, Some(0), "");
        match &finish[0] {
            AgentEvent::Output { text, final_answer } => {
                assert!(final_answer);
                assert!(text.contains("[redacted]"), "{text}");
                assert!(!text.contains("sk-known-secret-1234567890"), "{text}");
                assert!(
                    !text.contains("AIzaSyD-EXAMPLEFAKEKEY0123456789abcdefg"),
                    "{text}"
                );
                assert!(!text.contains("abcdefgh12345678"), "{text}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn oversized_preview_is_truncated_to_the_limit() {
        let mut mapper = mapper();
        mapper.on_line(r#"{"type":"session"}"#);
        let long = "x".repeat(1000);
        let line = format!(
            r#"{{"type":"tool_execution_end","toolName":"read","result":{{"content":[{{"type":"text","text":"{long}"}}]}},"isError":false}}"#
        );
        let events = mapper.on_line(&line);
        match &events[0] {
            AgentEvent::ToolResult { preview, .. } => {
                assert!(
                    preview.chars().count() <= PREVIEW_CHARS + 1,
                    "{}",
                    preview.chars().count()
                );
                assert!(preview.ends_with('…'));
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
