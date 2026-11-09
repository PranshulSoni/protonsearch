//! JSONL framing for the frozen ps-ipc agent protocol (protocol version 1).
//!
//! Framing: one JSON object per UTF-8 line, newline terminated, hard-capped at
//! MAX_LINE_BYTES in both directions. The host sends a bare AgentRunRequest
//! (camelCase fields, frozen struct), control envelopes for cancel and approval
//! responses, and receives AgentEvent values whose inner fields are camelCase
//! (frozen Rust serde shape: rename_all_fields = "camelCase").

use serde::Serialize;
use serde_json::Value;

use ps_ipc::{AgentApprovalResponse, AgentEvent, AgentRunRequest};

use crate::error::AgentError;

/// Hard bound for one JSONL frame, matching the Python sidecar's limit.
pub const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

/// Serialize a message and append the newline, enforcing the frame bound.
pub fn encode_line<T: Serialize>(message: &T, limit: usize) -> Result<Vec<u8>, AgentError> {
    let json = serde_json::to_string(message)?;
    let actual = json.len() + 1;
    if actual > limit {
        return Err(AgentError::LineTooLong { actual, limit });
    }
    let mut frame = Vec::with_capacity(actual);
    frame.extend_from_slice(json.as_bytes());
    frame.push(b'\n');
    Ok(frame)
}

/// A new run request, sent as the bare frozen AgentRunRequest struct.
pub fn encode_run_request(request: &AgentRunRequest) -> Result<Vec<u8>, AgentError> {
    encode_line(request, MAX_LINE_BYTES)
}

/// Host -> sidecar cancel control message.
#[derive(Serialize)]
struct CancelEnvelope<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(rename = "runId")]
    run_id: &'a str,
}

pub fn encode_cancel(run_id: &str) -> Result<Vec<u8>, AgentError> {
    encode_line(
        &CancelEnvelope {
            kind: "cancel",
            run_id,
        },
        MAX_LINE_BYTES,
    )
}

/// Approval decision: the frozen AgentApprovalResponse plus a type tag so the
/// sidecar can dispatch without guessing.
pub fn encode_approval_response(response: &AgentApprovalResponse) -> Result<Vec<u8>, AgentError> {
    let mut value = serde_json::to_value(response)?;
    if let Value::Object(ref mut map) = value {
        map.insert("type".to_string(), Value::String("approval_response".to_string()));
    } else {
        return Err(AgentError::Encode(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "approval response is not an object",
        ))));
    }
    encode_line(&value, MAX_LINE_BYTES)
}

/// Parse one AgentEvent line (trailing newline/CR allowed).
pub fn parse_event(line: &str) -> Result<AgentEvent, serde_json::Error> {
    serde_json::from_str(line.trim_end_matches(['\r', '\n']))
}

/// Redact known secret values and truncate a diagnostic line for logs/UI.
pub fn redact_and_truncate(line: &str, redactions: &[String], max_chars: usize) -> String {
    let mut out = line.trim_end().to_string();
    for secret in redactions {
        if secret.len() >= 6 {
            out = out.replace(secret.as_str(), "[redacted]");
        }
    }
    if out.chars().count() > max_chars {
        out = out.chars().take(max_chars).collect::<String>() + "...";
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_ipc::ActionSpec;

    fn sample_request() -> AgentRunRequest {
        AgentRunRequest {
            run_id: "run-1".into(),
            prompt: "find budget".into(),
            context_paths: vec!["C:/Users/x/budget.xlsx".into()],
            model: None,
            max_steps: Some(4),
        }
    }

    #[test]
    fn run_request_is_camel_case_and_newline_terminated() {
        let frame = encode_run_request(&sample_request()).unwrap();
        let text = std::str::from_utf8(&frame).unwrap();
        assert!(text.ends_with('\n'));
        assert!(text.contains("\"runId\":\"run-1\""), "{text}");
        assert!(text.contains("\"contextPaths\""), "{text}");
        assert!(text.contains("\"maxSteps\":4"), "{text}");
        let back: AgentRunRequest = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(back, sample_request());
    }

    #[test]
    fn cancel_envelope_shape() {
        let frame = encode_cancel("run-7").unwrap();
        assert_eq!(
            std::str::from_utf8(&frame).unwrap(),
            "{\"type\":\"cancel\",\"runId\":\"run-7\"}\n"
        );
    }

    #[test]
    fn approval_response_has_type_tag_and_camel_case() {
        let response = AgentApprovalResponse {
            run_id: "r".into(),
            approval_id: "apr-1".into(),
            approved: true,
        };
        let frame = encode_approval_response(&response).unwrap();
        let text = std::str::from_utf8(&frame).unwrap();
        assert!(text.contains("\"type\":\"approval_response\""), "{text}");
        assert!(text.contains("\"runId\":\"r\""), "{text}");
        assert!(text.contains("\"approvalId\":\"apr-1\""), "{text}");
        assert!(text.contains("\"approved\":true"), "{text}");
    }

    #[test]
    fn parses_every_frozen_event_variant() {
        let cases = [
            r#"{"type":"started","runId":"r","runtime":"pydantic-ai 2.66.0","model":"m"}"#,
            r#"{"type":"plan","steps":["a","b"]}"#,
            r#"{"type":"progress","message":"working","fraction":0.5}"#,
            r#"{"type":"approval_request","approvalId":"a1","title":"t","detail":"d","action":{"type":"none"},"destructive":false}"#,
            r#"{"type":"tool_result","tool":"search_files","ok":true,"preview":"p"}"#,
            r#"{"type":"output","text":"done","finalAnswer":true}"#,
            r#"{"type":"error","code":"cancelled","message":"m"}"#,
            r#"{"type":"finished","ok":false,"durationMs":12}"#,
        ];
        for raw in cases {
            parse_event(raw).unwrap_or_else(|err| panic!("failed to parse {raw}: {err}"));
        }
        match parse_event(cases[3]).unwrap() {
            AgentEvent::ApprovalRequest {
                approval_id,
                action,
                destructive,
                ..
            } => {
                assert_eq!(approval_id, "a1");
                assert_eq!(action, ActionSpec::None);
                assert!(!destructive);
            }
            other => panic!("unexpected variant {other:?}"),
        }
        match parse_event(cases[7]).unwrap() {
            AgentEvent::Finished { ok, duration_ms } => {
                assert!(!ok);
                assert_eq!(duration_ms, 12);
            }
            other => panic!("unexpected variant {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_event_type() {
        assert!(parse_event(r#"{"type":"teleport"}"#).is_err());
    }

    #[test]
    fn frame_limit_is_enforced() {
        let request = sample_request();
        let err = encode_line(&request, 8).unwrap_err();
        assert!(matches!(err, AgentError::LineTooLong { limit: 8, .. }), "{err}");
    }

    #[test]
    fn redaction_replaces_secret_values() {
        let line = "Authorization: Bearer sk-supersecretvalue";
        let out = redact_and_truncate(line, &["sk-supersecretvalue".to_string()], 200);
        assert_eq!(out, "Authorization: Bearer [redacted]");
        assert!(redact_and_truncate(line, &[], 10).chars().count() <= 13);
    }
}
