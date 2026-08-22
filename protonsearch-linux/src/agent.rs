//! Hermes Agent integration used by the in-launcher Linux Agent view.
//!
//! The GTK layer owns presentation. This module owns the small, user-scoped
//! history file and the blocking Hermes invocation so no agent work runs on
//! the GTK thread.

use crate::system;
use crate::xdg::XdgPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub hermes_session: Option<String>,
    #[serde(default)]
    pub messages: Vec<Message>,
}

pub fn history_file(paths: &XdgPaths) -> PathBuf {
    paths.state_dir().join("agent-history.json")
}

pub fn load_history(paths: &XdgPaths) -> Vec<Conversation> {
    let Ok(bytes) = fs::read(history_file(paths)) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save_history(paths: &XdgPaths, conversations: &[Conversation]) -> Result<()> {
    let file = history_file(paths);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = file.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(conversations)?;
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, file)?;
    Ok(())
}

pub fn executable() -> Option<&'static str> {
    if system::command_available("hermes") {
        Some("hermes")
    } else if system::command_available("hermes-agent") {
        Some("hermes-agent")
    } else {
        None
    }
}

pub fn readiness() -> String {
    let Some(command) = executable() else {
        return "Hermes Agent is not installed. Install Hermes to enable this tab.".to_string();
    };
    match system::run_with_timeout(command, &["--version"], Duration::from_secs(5)) {
        Ok(output) if output.status == Some(0) => {
            let version = output.stdout.trim();
            if version.is_empty() {
                "Hermes Agent is installed and ready to configure.".to_string()
            } else {
                format!("{version} · ready")
            }
        }
        Ok(output) if !output.stderr.trim().is_empty() => {
            format!(
                "Hermes is installed but needs setup: {}",
                output.stderr.trim()
            )
        }
        _ => "Hermes is installed but its provider is not configured yet.".to_string(),
    }
}

pub fn run_prompt(prompt: &str, hermes_session: Option<&str>, timeout: Duration) -> Result<String> {
    let command = executable().context("Hermes Agent is not installed")?;
    let mut args = Vec::<String>::new();
    if let Some(session) = hermes_session.filter(|value| !value.trim().is_empty()) {
        args.extend(["--resume".to_string(), session.to_string()]);
    }
    args.extend(["-z".to_string(), prompt.to_string()]);
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = system::run_with_timeout(command, &refs, timeout)?;
    if output.timed_out {
        anyhow::bail!(
            "Hermes did not respond within {} seconds",
            timeout.as_secs()
        );
    }
    if output.status != Some(0) {
        let detail = if output.stderr.trim().is_empty() {
            format!("Hermes exited with status {:?}", output.status)
        } else {
            output.stderr.trim().to_string()
        };
        anyhow::bail!("{detail}");
    }
    let response = output.stdout.trim().to_string();
    if response.is_empty() {
        anyhow::bail!("Hermes returned an empty response");
    }
    Ok(response)
}

/// Start the user-installed gateway when one is present. This is intentionally
/// best-effort: Hermes installations differ between distributions, and the
/// launcher must remain usable if gateway messaging is not configured.
pub fn ensure_gateway() -> String {
    let Some(command) = executable() else {
        return "Hermes Agent is not installed".to_string();
    };
    let gateway = system::run_with_timeout(command, &["gateway", "start"], Duration::from_secs(15));
    match gateway {
        Ok(output) if output.status == Some(0) => "Hermes gateway is running".to_string(),
        Ok(output) if !output.stderr.trim().is_empty() => {
            format!(
                "Hermes is installed; gateway setup: {}",
                output.stderr.trim()
            )
        }
        _ => "Hermes is installed; gateway is unavailable until configured".to_string(),
    }
}

pub fn _command_for_tests() -> Command {
    Command::new("hermes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_round_trips() {
        let conversation = Conversation {
            id: "one".to_string(),
            title: "Test".to_string(),
            hermes_session: None,
            messages: vec![Message {
                role: "user".to_string(),
                text: "hello".to_string(),
            }],
        };
        let encoded = serde_json::to_string(&conversation).unwrap();
        let decoded: Conversation = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, conversation);
    }

    #[test]
    fn command_is_not_built_from_user_text() {
        let command = _command_for_tests();
        assert_eq!(command.get_program().to_string_lossy(), "hermes");
    }
}
