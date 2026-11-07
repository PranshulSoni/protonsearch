//! Read local AI usage from Codex rollout logs and Claude Code transcripts.
use serde_json::{json, Value};
use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const LIVE_WINDOW_SECONDS: u64 = 300;
const TASK_LIMIT: usize = 160;
const SCAN_DEPTH: usize = 8;

#[derive(Default)]
struct Session {
    workspace: Option<String>,
    model: Option<String>,
    task: Option<String>,
    started_at: Option<i64>,
    total_tokens: Option<u64>,
    rate_limits: Option<Value>,
}

struct Candidate {
    path: PathBuf,
    modified: SystemTime,
}

fn now_millis() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|_| "Invalid system clock".into())
}

fn now_seconds() -> Result<i64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .map_err(|_| "Invalid system clock".into())
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .ok_or_else(|| "Home directory unavailable".into())
}

fn nested(root: &Path, parts: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in parts {
        path.push(part);
    }
    path
}

fn newest_file(
    root: &Path,
    predicate: impl Fn(&OsStr) -> bool + Copy,
    depth: usize,
) -> Option<Candidate> {
    if depth > SCAN_DEPTH {
        return None;
    }
    let entries = fs::read_dir(root).ok()?;
    let mut newest: Option<Candidate> = None;
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            if let Some(candidate) = newest_file(&path, predicate, depth + 1) {
                if newest
                    .as_ref()
                    .map_or(true, |current| candidate.modified > current.modified)
                {
                    newest = Some(candidate);
                }
            }
        } else if file_type.is_file() && path.file_name().is_some_and(predicate) {
            let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) else {
                continue;
            };
            if newest
                .as_ref()
                .map_or(true, |current| modified > current.modified)
            {
                newest = Some(Candidate { path, modified });
            }
        }
    }
    newest
}

fn is_rollout(name: &OsStr) -> bool {
    let name = name.to_string_lossy();
    name.starts_with("rollout-") && name.ends_with(".jsonl")
}

fn is_jsonl(name: &OsStr) -> bool {
    name.to_string_lossy().ends_with(".jsonl")
}

fn compact_text(value: &str, limit: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= limit {
        return compact;
    }
    let mut truncated = compact
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>();
    truncated.push('\u{2026}');
    truncated
}

const TASK_SOURCE_LIMIT: usize = 4_000;

fn task_candidate(text: &str) -> Option<&str> {
    let text = text.trim();
    if text.is_empty()
        || text.len() > TASK_SOURCE_LIMIT
        || text.starts_with("<environment_context>")
        || text.starts_with("<user_instructions>")
        || text.starts_with("<INSTRUCTIONS>")
    {
        None
    } else {
        Some(text)
    }
}

fn user_task(payload: &Value) -> Option<String> {
    if payload["type"] != "message" || payload["role"] != "user" {
        return None;
    }
    let mut selected = payload["content"].as_str().and_then(task_candidate);
    if let Some(items) = payload["content"].as_array() {
        for item in items {
            if let Some(text) = item["text"].as_str().and_then(task_candidate) {
                selected = Some(text);
            }
        }
    }
    selected.map(|text| compact_text(text, TASK_LIMIT))
}

fn parse_rollout(path: &Path) -> Result<Session, String> {
    let file = File::open(path).map_err(|_| "Cannot read Codex session".to_string())?;
    let mut session = Session::default();
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let payload = &value["payload"];
        match value["type"].as_str() {
            Some("session_meta") => {
                session.workspace = payload["cwd"].as_str().map(str::to_string);
                session.started_at = payload["timestamp"]
                    .as_str()
                    .and_then(|timestamp| chrono::DateTime::parse_from_rfc3339(timestamp).ok())
                    .map(|timestamp| timestamp.timestamp());
            }
            Some("turn_context") => {
                session.model = payload["model"].as_str().map(str::to_string);
            }
            Some("event_msg") => match payload["type"].as_str() {
                Some("token_count") => {
                    if let Some(tokens) =
                        payload["info"]["total_token_usage"]["total_tokens"].as_u64()
                    {
                        session.total_tokens = Some(tokens);
                    }
                    if payload["rate_limits"].is_object() {
                        session.rate_limits = Some(payload["rate_limits"].clone());
                    }
                }
                Some("token_usage_record") | Some("thread_token_usage") => {
                    if let Some(tokens) = payload["total_tokens"]
                        .as_u64()
                        .or_else(|| payload["usage"]["total_tokens"].as_u64())
                    {
                        session.total_tokens = Some(tokens);
                    }
                }
                Some("thread_settings_applied") => {
                    session.model = payload["thread_settings"]["model"]
                        .as_str()
                        .map(str::to_string)
                        .or(session.model);
                }
                _ => {}
            },
            Some("response_item") => {
                if let Some(task) = user_task(payload) {
                    session.task = Some(task);
                }
            }
            _ => {}
        }
    }
    Ok(session)
}

fn compact_span(seconds: u64) -> String {
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let hours = minutes / 60;
    if hours < 24 {
        let rest = minutes % 60;
        return if rest == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h {rest}m")
        };
    }
    let days = hours / 24;
    let rest = hours % 24;
    if rest == 0 {
        format!("{days}d")
    } else {
        format!("{days}d {rest}h")
    }
}

fn reset_in(resets_at: Option<i64>, now: i64) -> Option<String> {
    let resets_at = resets_at?;
    let seconds = resets_at - now;
    Some(if seconds <= 0 {
        "now".into()
    } else {
        compact_span(seconds as u64)
    })
}

fn elapsed(started_at: Option<i64>, now: i64) -> Option<String> {
    let started_at = started_at?;
    let seconds = now - started_at;
    if seconds < 0 {
        return None;
    }
    Some(compact_span(seconds as u64))
}

fn compact_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        let value = format!("{:.1}", tokens as f64 / 1_000_000.0);
        format!("{}M", value.trim_end_matches(".0"))
    } else if tokens >= 1_000 {
        let value = format!("{:.1}", tokens as f64 / 1_000.0);
        format!("{}K", value.trim_end_matches(".0"))
    } else {
        tokens.to_string()
    }
}

fn chatgpt_data(session: &Session, now: i64) -> Value {
    let Some(rate_limits) = session.rate_limits.as_ref() else {
        return json!({
            "status": "empty",
            "model": session.model,
            "tokensUsed": session.total_tokens,
        });
    };
    let primary = &rate_limits["primary"];
    let secondary = &rate_limits["secondary"];
    json!({
        "status": "live",
        "model": session.model,
        "sessionPercent": primary["used_percent"].as_f64(),
        "weeklyPercent": secondary["used_percent"].as_f64(),
        "resetIn": reset_in(primary["resets_at"].as_i64(), now),
        "weeklyResetIn": reset_in(secondary["resets_at"].as_i64(), now),
        "tokensUsed": session.total_tokens,
        "tokensLimit": Value::Null,
        "plan": rate_limits["plan_type"].as_str(),
        "creditsPercent": Value::Null,
        "creditsUsed": Value::Null,
        "creditsLimit": Value::Null,
    })
}

fn agent_data(session: &Session, modified: Option<SystemTime>, now: i64) -> Value {
    let live = modified.is_some_and(|modified| {
        SystemTime::now()
            .duration_since(modified)
            .is_ok_and(|elapsed| elapsed.as_secs() <= LIVE_WINDOW_SECONDS)
    });
    json!({
        "status": if live { "live" } else { "idle" },
        "workspace": session.workspace,
        "branch": Value::Null,
        "task": session.task,
        "duration": elapsed(session.started_at, now),
        "tokens": session.total_tokens.map(compact_tokens),
        "filesChanged": Value::Null,
        "toolCalls": Value::Null,
    })
}

fn claude_usage(path: &Path) -> Result<Option<(Option<String>, u64)>, String> {
    let file = File::open(path).map_err(|_| "Cannot read Claude session".to_string())?;
    let mut usage = None;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value["type"] != "assistant" {
            continue;
        }
        let message = &value["message"];
        let record = &message["usage"];
        let tokens = [
            "input_tokens",
            "output_tokens",
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
        ]
        .into_iter()
        .filter_map(|key| record[key].as_u64())
        .fold(None, |total: Option<u64>, value| {
            Some(total.unwrap_or(0).saturating_add(value))
        });
        if let Some(tokens) = tokens {
            usage = Some((message["model"].as_str().map(str::to_string), tokens));
        }
    }
    Ok(usage)
}

fn claude_data(projects: &Path) -> Value {
    let Some(file) = newest_file(projects, is_jsonl, 0) else {
        return json!({"status":"disconnected","message":"Claude Code usage is unavailable"});
    };
    match claude_usage(&file.path) {
        Ok(Some((model, tokens))) => json!({
            "status": "live",
            "model": model,
            "sessionPercent": Value::Null,
            "weeklyPercent": Value::Null,
            "resetIn": Value::Null,
            "weeklyResetIn": Value::Null,
            "tokensUsed": tokens,
            "tokensLimit": Value::Null,
            "plan": Value::Null,
        }),
        _ => json!({"status":"disconnected","message":"Claude Code usage limits are unavailable"}),
    }
}

fn snapshot_at(home: &Path) -> Result<Value, String> {
    let sessions_root = nested(home, &[".codex", "sessions"]);
    let session_file = newest_file(&sessions_root, is_rollout, 0);
    let (session, modified) = match session_file {
        Some(file) => (Some(parse_rollout(&file.path)?), Some(file.modified)),
        None => (None, None),
    };
    let now = now_seconds()?;
    let (chatgpt, agent) = match session.as_ref() {
        Some(session) => (
            chatgpt_data(session, now),
            agent_data(session, modified, now),
        ),
        None => (
            json!({"status":"disconnected"}),
            json!({"status":"disconnected"}),
        ),
    };
    let claude = claude_data(&nested(home, &[".claude", "projects"]));
    let live = chatgpt["status"] == "live" || claude["status"] == "live";
    let status = if live {
        "live"
    } else if session.is_some() {
        "empty"
    } else {
        "disconnected"
    };
    let mut envelope = json!({
        "status": status,
        "updatedAt": now_millis()?,
        "data": {"chatgpt": chatgpt, "claude": claude, "agent": agent},
    });
    if !live {
        envelope["message"] = json!(if session.is_some() {
            "No Codex usage window found"
        } else {
            "No local AI sessions found"
        });
    }
    Ok(envelope)
}

/// Read the newest local Codex rollout and any local Claude usage.
pub fn snapshot() -> Result<Value, String> {
    let home = home_dir()?;
    snapshot_at(&home)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_home() -> PathBuf {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("ai-usage-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_lines(path: &Path, lines: &[Value]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
    }

    fn now_seconds_test() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn ignores_injected_system_prompts_when_looking_for_a_task() {
        let payload = json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "x".repeat(5_000)}]
        });
        assert!(user_task(&payload).is_none());
    }

    #[test]
    fn reads_newest_codex_limits_and_agent_metadata_without_inventing_quota() {
        let home = temp_home();
        let now = now_seconds_test();
        let session_path = home
            .join(".codex")
            .join("sessions")
            .join("2026")
            .join("11")
            .join("07")
            .join("rollout-test.jsonl");
        let started = chrono::Utc::now() - chrono::Duration::minutes(63);
        write_lines(
            &session_path,
            &[
                json!({"type":"session_meta","payload":{"id":"test","timestamp":started.to_rfc3339(),"cwd":"C:\\Projects\\demo"}}),
                json!({"type":"turn_context","payload":{"model":"gpt-6-luna"}}),
                json!({"type":"response_item","payload":{"type":"message","role":"user","content":[
                    {"type":"input_text","text":"<environment_context>ignored</environment_context>"},
                    {"type":"input_text","text":"Fix live weather\nand AI usage"}
                ]}}),
                json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":34769}},"rate_limits":{
                    "limit_id":"codex","plan_type":"plus",
                    "primary":{"used_percent":83.0,"window_minutes":300,"resets_at":now + 4 * 3600 + 12 * 60},
                    "secondary":{"used_percent":13.0,"window_minutes":10080,"resets_at":now + 6 * 86400 + 3 * 3600}
                }}}),
            ],
        );

        let value = snapshot_at(&home).unwrap();
        assert_eq!(value["status"], "live");
        assert_eq!(value["data"]["chatgpt"]["status"], "live");
        assert_eq!(value["data"]["chatgpt"]["model"], "gpt-6-luna");
        assert_eq!(value["data"]["chatgpt"]["sessionPercent"], 83.0);
        assert_eq!(value["data"]["chatgpt"]["weeklyPercent"], 13.0);
        assert_eq!(value["data"]["chatgpt"]["plan"], "plus");
        assert_eq!(value["data"]["chatgpt"]["tokensUsed"], 34_769);
        assert_eq!(value["data"]["chatgpt"]["tokensLimit"], Value::Null);
        assert_eq!(value["data"]["chatgpt"]["resetIn"], "4h 12m");
        assert_eq!(value["data"]["chatgpt"]["weeklyResetIn"], "6d 3h");
        assert_eq!(value["data"]["agent"]["status"], "live");
        assert_eq!(value["data"]["agent"]["workspace"], "C:\\Projects\\demo");
        assert_eq!(
            value["data"]["agent"]["task"],
            "Fix live weather and AI usage"
        );
        assert_eq!(value["data"]["agent"]["tokens"], "34.8K");
        assert!(value["data"]["agent"]["duration"].is_string());
        assert_eq!(value["data"]["agent"]["branch"], Value::Null);
        assert_eq!(value["data"]["agent"]["filesChanged"], Value::Null);
        assert_eq!(value["data"]["agent"]["toolCalls"], Value::Null);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn claude_usage_is_reported_only_when_a_real_assistant_record_exists() {
        let home = temp_home();
        let claude_path = home
            .join(".claude")
            .join("projects")
            .join("test")
            .join("session.jsonl");
        write_lines(
            &claude_path,
            &[
                json!({"type":"assistant","message":{"model":"claude-test","usage":{
                    "input_tokens":1000,"output_tokens":200,"cache_creation_input_tokens":50,"cache_read_input_tokens":30
                }}}),
            ],
        );

        let value = snapshot_at(&home).unwrap();
        assert_eq!(value["status"], "live");
        assert_eq!(value["data"]["claude"]["status"], "live");
        assert_eq!(value["data"]["claude"]["model"], "claude-test");
        assert_eq!(value["data"]["claude"]["tokensUsed"], 1280);
        assert_eq!(value["data"]["claude"]["tokensLimit"], Value::Null);
        assert_eq!(value["data"]["claude"]["sessionPercent"], Value::Null);
        assert_eq!(value["data"]["claude"]["weeklyPercent"], Value::Null);
        assert_eq!(value["data"]["chatgpt"]["status"], "disconnected");
        assert_eq!(value["data"]["agent"]["status"], "disconnected");
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn missing_local_sessions_return_disconnected_instead_of_sample_data() {
        let home = temp_home();
        let value = snapshot_at(&home).unwrap();
        assert_eq!(value["status"], "disconnected");
        assert_eq!(value["data"]["chatgpt"]["status"], "disconnected");
        assert_eq!(value["data"]["claude"]["status"], "disconnected");
        assert_eq!(value["data"]["agent"]["status"], "disconnected");
        assert!(value["message"].is_string());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn agent_is_idle_when_the_session_file_is_older_than_five_minutes() {
        let now = now_seconds_test();
        let modified = SystemTime::now() - std::time::Duration::from_secs(LIVE_WINDOW_SECONDS + 1);
        let value = agent_data(&Session::default(), Some(modified), now);
        assert_eq!(value["status"], "idle");
        assert_eq!(value["branch"], Value::Null);
        assert_eq!(value["filesChanged"], Value::Null);
        assert_eq!(value["toolCalls"], Value::Null);
    }

    #[test]
    fn reset_and_duration_formatting_is_compact() {
        let now = 1_000_000;
        assert_eq!(
            reset_in(Some(now + 4 * 3600 + 12 * 60), now).as_deref(),
            Some("4h 12m")
        );
        assert_eq!(
            reset_in(Some(now + 6 * 86400 + 3 * 3600), now).as_deref(),
            Some("6d 3h")
        );
        assert_eq!(reset_in(Some(now + 59), now).as_deref(), Some("59s"));
        assert_eq!(reset_in(Some(now), now).as_deref(), Some("now"));
        assert_eq!(elapsed(Some(now - 65), now).as_deref(), Some("1m"));
        assert_eq!(elapsed(Some(now + 1), now), None);
    }
}
