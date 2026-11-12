//! Non-Pi agent CLIs: Claude Code, Codex, OpenCode and Gemini CLI.
//!
//! One process per run, started without a console window in the workspace
//! folder. The prompt never goes on a command line that a Windows `.cmd` shim
//! would parse: it is written to stdin, or to a temp file for OpenCode (which
//! has no stdin mode). The answer is the process stdout; Claude Code's is the
//! `result` field of its JSON output.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ps_ipc::agent_task::AgentTaskRuntime;
use serde_json::Value;

use crate::pi::{pi_command, run_child_capture, run_child_capture_with_input, PiModel};

/// Version probe: a local read, so a short bound.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
/// Model listing reads the CLI's provider config.
const LIST_TIMEOUT: Duration = Duration::from_secs(20);
/// Longest run before the process tree is stopped.
pub const RUN_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// The Test button: one word, a short bound.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(90);
/// The Test button prompt.
pub const CHECK_PROMPT: &str = "Reply with the single word OK.";
/// OpenCode's message; the request itself is in the attached file.
const OPENCODE_MESSAGE: &str = "Answer the request in the attached file.";

/// One agent CLI this app can drive. The id is also the launcher name on PATH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliAgent {
    Claude,
    Codex,
    OpenCode,
    Gemini,
}

impl CliAgent {
    pub const ALL: [CliAgent; 4] = [CliAgent::Claude, CliAgent::Codex, CliAgent::OpenCode, CliAgent::Gemini];

    pub fn id(self) -> &'static str {
        match self {
            CliAgent::Claude => "claude",
            CliAgent::Codex => "codex",
            CliAgent::OpenCode => "opencode",
            CliAgent::Gemini => "gemini",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|agent| agent.id() == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            CliAgent::Claude => "Claude Code",
            CliAgent::Codex => "Codex",
            CliAgent::OpenCode => "OpenCode",
            CliAgent::Gemini => "Gemini CLI",
        }
    }

    pub fn task_runtime(self) -> AgentTaskRuntime {
        match self {
            CliAgent::Claude => AgentTaskRuntime::Claude,
            CliAgent::Codex => AgentTaskRuntime::Codex,
            CliAgent::OpenCode => AgentTaskRuntime::OpenCode,
            CliAgent::Gemini => AgentTaskRuntime::Gemini,
        }
    }

    fn reads_stdin(self) -> bool {
        self != CliAgent::OpenCode
    }
}

/// An installed CLI: the launcher found on PATH and the version it reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliInfo {
    pub path: PathBuf,
    pub version: String,
}

/// Finds the CLI and reads `--version`. No model call and no network.
pub fn probe(agent: CliAgent) -> Option<CliInfo> {
    let path = locate(agent.id())?;
    let command = pi_command(&path, &["--version".to_string()]).ok()?;
    let capture = run_child_capture(command, PROBE_TIMEOUT, &AtomicBool::new(false)).ok()?;
    if capture.timed_out || capture.exit_code != Some(0) {
        return None;
    }
    let version = first_version(&capture.stdout)?;
    Some(CliInfo { path, version })
}

/// Models the CLI can list. Only OpenCode has a listing command; the others
/// take a typed model name, so they list nothing.
pub fn list_models(agent: CliAgent) -> Vec<PiModel> {
    if agent != CliAgent::OpenCode {
        return Vec::new();
    }
    let Some(path) = locate(agent.id()) else {
        return Vec::new();
    };
    let Ok(command) = pi_command(&path, &["models".to_string()]) else {
        return Vec::new();
    };
    match run_child_capture(command, LIST_TIMEOUT, &AtomicBool::new(false)) {
        Ok(capture) if !capture.timed_out && capture.exit_code == Some(0) => capture
            .stdout
            .lines()
            .filter_map(|line| {
                let (provider, id) = line.trim().split_once('/')?;
                Some(PiModel { provider: provider.to_string(), id: id.to_string() })
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Arguments for one run. The prompt is not among them (see [`run`]);
/// `prompt_file` is required for OpenCode.
pub fn build_run_args(agent: CliAgent, model: Option<&str>, prompt_file: Option<&Path>) -> Result<Vec<String>, String> {
    let model = checked_model(model)?;
    let mut args: Vec<String> = match agent {
        CliAgent::Claude => ["-p", "--output-format", "json"].map(String::from).into(),
        CliAgent::Codex => ["exec", "--skip-git-repo-check", "--ephemeral", "--sandbox", "read-only"]
            .map(String::from)
            .into(),
        CliAgent::OpenCode => {
            let file = prompt_file.ok_or("OpenCode needs its prompt file.")?;
            vec!["run".into(), "--file".into(), file.to_string_lossy().into_owned()]
        }
        CliAgent::Gemini => ["-p", "Answer the request on stdin."].map(String::from).into(),
    };
    if let Some(model) = model {
        args.push("--model".into());
        args.push(model.into());
    }
    match agent {
        CliAgent::Codex => args.push("-".into()),
        CliAgent::OpenCode => args.push(OPENCODE_MESSAGE.into()),
        CliAgent::Claude | CliAgent::Gemini => {}
    }
    Ok(args)
}

/// A run that produced no answer: a short code for the task log and a message for the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    pub code: &'static str,
    pub message: String,
}

fn failed(message: impl Into<String>) -> CliError {
    CliError { code: "run_failed", message: message.into() }
}

/// The prompt as a temp file for OpenCode's `--file`, removed when dropped.
struct PromptFile(PathBuf);

impl PromptFile {
    fn write(prompt: &str) -> std::io::Result<Self> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_nanos()).unwrap_or(0);
        let path = std::env::temp_dir().join(format!("protonsearch-prompt-{}-{nanos}.txt", std::process::id()));
        fs::write(&path, prompt)?;
        Ok(Self(path))
    }
}

impl Drop for PromptFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Runs one prompt and returns the final answer. Blocks until the process exits,
/// `timeout` passes or `cancel` is set; the last two stop the whole process tree.
pub fn run(
    agent: CliAgent,
    launcher: &Path,
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<String, CliError> {
    fs::create_dir_all(workspace).map_err(|error| failed(format!("The workspace folder is unavailable: {error}")))?;
    let prompt_file = match agent {
        CliAgent::OpenCode => Some(
            PromptFile::write(prompt).map_err(|error| failed(format!("The prompt could not be written: {error}")))?,
        ),
        _ => None,
    };
    let args = build_run_args(agent, model, prompt_file.as_ref().map(|file| file.0.as_path())).map_err(failed)?;
    let mut command = pi_command(launcher, &args).map_err(|error| failed(error.to_string()))?;
    command.current_dir(workspace);
    let stdin = agent.reads_stdin().then_some(prompt);
    let capture = run_child_capture_with_input(command, timeout, cancel, stdin)
        .map_err(|error| failed(error.to_string()))?;
    if capture.timed_out {
        return Err(if cancel.load(Ordering::Relaxed) {
            CliError { code: "cancelled", message: "The run was cancelled.".into() }
        } else {
            CliError { code: "timeout", message: format!("{} did not finish in time.", agent.name()) }
        });
    }
    if capture.exit_code != Some(0) {
        let detail = capture.stderr.lines().map(str::trim).rfind(|line| !line.is_empty());
        return Err(failed(detail.map(str::to_string).unwrap_or_else(|| format!("{} exited with an error.", agent.name()))));
    }
    answer(agent, &capture.stdout)
}

/// The Test button: one word through the configured model, with a short bound.
pub fn check(agent: CliAgent, launcher: &Path, workspace: &Path, model: Option<&str>) -> Result<(), CliError> {
    run(agent, launcher, workspace, CHECK_PROMPT, model, CHECK_TIMEOUT, &AtomicBool::new(false)).map(|_| ())
}

/// Claude Code wraps its answer in JSON; the others print plain text.
fn answer(agent: CliAgent, stdout: &str) -> Result<String, CliError> {
    let text = stdout.trim();
    let answer = match serde_json::from_str::<Value>(text) {
        Ok(value) if agent == CliAgent::Claude => {
            if value.get("is_error").and_then(Value::as_bool).unwrap_or(false) {
                let reason = value.get("result").and_then(Value::as_str).unwrap_or_default().trim();
                return Err(failed(if reason.is_empty() { "Claude Code reported an error." } else { reason }));
            }
            value.get("result").and_then(Value::as_str).unwrap_or_default().trim().to_string()
        }
        _ => text.to_string(),
    };
    if answer.is_empty() {
        return Err(failed("The agent returned no answer."));
    }
    Ok(answer)
}

/// The first PATH entry holding the launcher. Windows tries `.exe`, then the
/// `.cmd`/`.bat` shims npm writes; the bare name there is a POSIX script.
fn locate(name: &str) -> Option<PathBuf> {
    let suffixes: &[&str] = if cfg!(windows) { &["exe", "cmd", "bat"] } else { &[""] };
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var).filter(|dir| dir.is_absolute()) {
        for suffix in suffixes {
            let candidate = if suffix.is_empty() { dir.join(name) } else { dir.join(format!("{name}.{suffix}")) };
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// The first token shaped like a version: "2.1.318 (Claude Code)",
/// "codex-cli 0.167.0", "opencode v2.2.7".
fn first_version(output: &str) -> Option<String> {
    output.split_whitespace().find_map(|token| {
        let bare = token.trim_start_matches(['v', 'V']);
        let version: String = bare
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '+'))
            .collect();
        (version.contains('.') && version.starts_with(|character: char| character.is_ascii_digit())).then_some(version)
    })
}

/// Model names reach a command line (and a `.cmd` shim), so only plain id characters pass.
fn checked_model(model: Option<&str>) -> Result<Option<&str>, String> {
    let Some(model) = model.map(str::trim).filter(|model| !model.is_empty()) else {
        return Ok(None);
    };
    if model.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '/' | ':' | '@' | '#')) {
        Ok(Some(model))
    } else {
        Err("Model names may use letters, digits and . - _ / : @ #.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_args_carry_the_model_flag_only_when_set() {
        let file = Path::new("prompt.txt");
        assert_eq!(build_run_args(CliAgent::Claude, None, None).unwrap(), ["-p", "--output-format", "json"]);
        assert_eq!(
            build_run_args(CliAgent::Claude, Some(" sonnet "), None).unwrap(),
            ["-p", "--output-format", "json", "--model", "sonnet"]
        );
        assert_eq!(
            build_run_args(CliAgent::Codex, Some("gpt-5.4"), None).unwrap(),
            ["exec", "--skip-git-repo-check", "--ephemeral", "--sandbox", "read-only", "--model", "gpt-5.4", "-"]
        );
        assert_eq!(
            build_run_args(CliAgent::OpenCode, Some("bai/claude-haiku-4.5"), Some(file)).unwrap(),
            ["run", "--file", "prompt.txt", "--model", "bai/claude-haiku-4.5", OPENCODE_MESSAGE]
        );
        assert_eq!(build_run_args(CliAgent::Gemini, None, None).unwrap(), ["-p", "Answer the request on stdin."]);
        assert!(build_run_args(CliAgent::Gemini, Some("pro & calc"), None).is_err());
        assert!(build_run_args(CliAgent::OpenCode, None, None).is_err());
    }
}
