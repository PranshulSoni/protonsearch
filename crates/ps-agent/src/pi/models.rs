//! Models the installed Pi CLI can run (`pi --list-models`).
//!
//! Pi lists only providers that have credentials, so the listing is read with
//! the same environment a run gets. It also uses the run hardening flags: user
//! extensions, skills, MCP servers and context files never load, even here.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use serde::Serialize;

use super::env::apply_minimal_child_environment;
use super::probe::run_child_capture;
use super::process::pi_command;
use super::PiAgentConfig;

/// A listing reads local state only; this bounds a wedged Pi.
const LIST_TIMEOUT: Duration = Duration::from_secs(15);

/// One model Pi can run. `id` is what follows the provider in `provider/id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PiModel {
    pub provider: String,
    pub id: String,
}

/// Models offered by the configured Pi. Empty when Pi is missing, fails or lists
/// nothing. Spawns Pi and blocks, so call it off the UI thread.
pub fn list_pi_models(config: &PiAgentConfig) -> Vec<PiModel> {
    let args: Vec<String> = [
        "--no-approve",
        "--no-mcp",
        "--no-context-files",
        "--no-extensions",
        "--no-skills",
        "--no-prompt-templates",
        "--list-models",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let Ok(mut command) = pi_command(&config.pi.path, &args) else {
        return Vec::new();
    };
    apply_minimal_child_environment(&mut command, &config.extra_env);
    match run_child_capture(command, LIST_TIMEOUT, &AtomicBool::new(false)) {
        Ok(capture) if !capture.timed_out && capture.exit_code == Some(0) => parse_model_table(&capture.stdout),
        _ => Vec::new(),
    }
}

/// Reads the table Pi prints: a `provider model context ...` header, then one row
/// per model. Output without that header yields no models.
fn parse_model_table(output: &str) -> Vec<PiModel> {
    let mut lines = output.lines();
    let has_header = lines.by_ref().any(|line| {
        let mut columns = line.split_whitespace();
        columns.next() == Some("provider") && columns.next() == Some("model")
    });
    if !has_header {
        return Vec::new();
    }
    lines
        .filter_map(|line| {
            let mut columns = line.split_whitespace();
            Some(PiModel {
                provider: columns.next()?.to_string(),
                id: columns.next()?.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rows_after_the_header_and_nothing_without_one() {
        let table = "provider     model        context  max-out  thinking  images\n\
                     opencode     beam-free    1.0M     131.1K   yes       no\r\n\
                     opencode-go  qwen3.8-max  1M       131.1K   yes       yes\n";
        assert_eq!(
            parse_model_table(table),
            vec![
                PiModel { provider: "opencode".into(), id: "beam-free".into() },
                PiModel { provider: "opencode-go".into(), id: "qwen3.8-max".into() },
            ]
        );
        assert!(parse_model_table("No models available.\n").is_empty());
    }
}
