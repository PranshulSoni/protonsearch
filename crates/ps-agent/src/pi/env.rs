//! Minimal child environment for Pi runs and probes (PI-SEC-05/PI-SEC-06).
//!
//! The previous implementation inherited the entire user environment, exposing
//! unrelated secrets (AWS keys, GitHub tokens, CI tokens, ...) to Pi, its
//! extensions and, when writes are enabled, to bash/PowerShell. We now clear the
//! environment and forward only:
//!
//! 1. OS essentials Node/Pi need to start (PATH, SystemRoot, TEMP, user dirs,
//!    COMSPEC/PATHEXT, ... see ESSENTIAL_ENV_VARS);
//! 2. provider/auth variables documented by Pi 1.5.0 (providers.md "Use an API
//!    key from the environment", environment-variables.md), plus the
//!    AWS_CONTAINER_CREDENTIALS_* family and the documented proxy variables;
//! 3. Pi process configuration that is location, not secret
//!    (PI_CODING_AGENT_DIR, PI_PACKAGE_DIR);
//! 4. the PI_* controls ProtonSearch sets: PI_OFFLINE=1,
//!    PI_SKIP_VERSION_CHECK=1, PI_TELEMETRY=0 (PI-SEC-06);
//! 5. explicit `PiAgentConfig.extra_env` entries (trusted app injection, e.g. a
//!    provider base URL). These are never logged and are added to redaction.
//!
//! Probes use the same allowlist without provider credentials, so a status
//! probe cannot leak auth material.
//!
//! Limitation: a user who points Pi at a custom config directory via a
//! non-default `PI_CODING_AGENT_DIR` must set it through `extra_env`; arbitrary
//! `PI_*` variables are intentionally not inherited.

use std::ffi::OsString;
use std::process::Command;

#[cfg(windows)]
const ESSENTIAL_ENV_VARS: &[&str] = &[
    "SystemRoot",
    "windir",
    "SystemDrive",
    "COMSPEC",
    "PATHEXT",
    "PATH",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "USERNAME",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMFILES",
    "ProgramFiles(x86)",
    "PROGRAMDATA",
    "ALLUSERSPROFILE",
    "PUBLIC",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "OS",
    // cmd.exe injects a default PROMPT when it is missing from the environment.
    "PROMPT",
];

#[cfg(not(windows))]
const ESSENTIAL_ENV_VARS: &[&str] = &[
    "PATH", "HOME", "USER", "LOGNAME", "SHELL", "TERM", "TMPDIR", "LANG", "LC_ALL", "TZ",
];

/// Provider/auth and provider-configuration variables documented by Pi 1.5.0.
/// Values are forwarded only when present in the parent environment.
pub const PROVIDER_ENV_VARS: &[&str] = &[
    "AI_GATEWAY_API_KEY",
    "ANT_LING_API_KEY",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_OAUTH_TOKEN",
    "ANTHROPIC_FEDERATION_RULE_ID",
    "ANTHROPIC_IDENTITY_TOKEN_FILE",
    "ANTHROPIC_ORGANIZATION_ID",
    "ANTHROPIC_SERVICE_ACCOUNT_ID",
    "ANTHROPIC_WORKSPACE_ID",
    "AWS_ACCESS_KEY_ID",
    "AWS_BEARER_TOKEN_BEDROCK",
    "AWS_DEFAULT_REGION",
    "AWS_PROFILE",
    "AWS_REGION",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AWS_WEB_IDENTITY_TOKEN_FILE",
    "AZURE_OPENAI_API_KEY",
    "AZURE_OPENAI_API_VERSION",
    "AZURE_OPENAI_BASE_URL",
    "AZURE_OPENAI_DEPLOYMENT_NAME_MAP",
    "AZURE_OPENAI_RESOURCE_NAME",
    "BASETEN_API_KEY",
    "CEREBRAS_API_KEY",
    "CLOUDFLARE_ACCOUNT_ID",
    "CLOUDFLARE_API_KEY",
    "CLOUDFLARE_GATEWAY_ID",
    "COPILOT_GITHUB_TOKEN",
    "DEEPSEEK_API_KEY",
    "FIREWORKS_API_KEY",
    "GCLOUD_PROJECT",
    "GEMINI_API_KEY",
    "GOOGLE_APPLICATION_CREDENTIALS",
    "GOOGLE_CLOUD_API_KEY",
    "GOOGLE_CLOUD_LOCATION",
    "GOOGLE_CLOUD_PROJECT",
    "GROQ_API_KEY",
    "HF_TOKEN",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "KIMI_API_KEY",
    "META_API_KEY",
    "MINIMAX_API_KEY",
    "MINIMAX_CN_API_KEY",
    "MISTRAL_API_KEY",
    "MOONSHOT_API_KEY",
    "NO_PROXY",
    "NVIDIA_API_KEY",
    "OPENAI_API_KEY",
    "OPENCODE_API_KEY",
    "OPENROUTER_API_KEY",
    "QWEN_TOKEN_PLAN_API_KEY",
    "QWEN_TOKEN_PLAN_CN_API_KEY",
    "RADIUS_API_KEY",
    "TOGETHER_API_KEY",
    "TYPESAFE_API_KEY",
    "XAI_API_KEY",
    "XIAOMI_API_KEY",
    "XIAOMI_TOKEN_PLAN_AMS_API_KEY",
    "XIAOMI_TOKEN_PLAN_CN_API_KEY",
    "XIAOMI_TOKEN_PLAN_SGP_API_KEY",
    "ZAI_API_KEY",
    "ZAI_CODING_CN_API_KEY",
];

/// Non-secret Pi process configuration we may forward from the parent.
const PI_CONFIG_ENV_VARS: &[&str] = &["PI_CODING_AGENT_DIR", "PI_PACKAGE_DIR"];

/// Controls ProtonSearch always sets for Pi (PI-SEC-06).
const PI_CONTROL_ENV_VARS: &[(&str, &str)] = &[
    ("PI_OFFLINE", "1"),
    ("PI_SKIP_VERSION_CHECK", "1"),
    ("PI_TELEMETRY", "0"),
];

const AWS_CONTAINER_PREFIX: &str = "AWS_CONTAINER_CREDENTIALS_";

fn names_equal(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

fn lookup<'a>(parent: &'a [(OsString, OsString)], name: &str) -> Option<&'a OsString> {
    parent
        .iter()
        .find(|(key, _)| key.to_str().map(|key| names_equal(key, name)).unwrap_or(false))
        .map(|(_, value)| value)
}

/// Pure env construction (unit-testable): essentials + provider vars + Pi
/// config + PI_* controls + explicit extra_env, and nothing else.
pub(crate) fn minimal_child_env(
    parent: &[(OsString, OsString)],
    extra_env: &[(String, String)],
) -> Vec<(OsString, OsString)> {
    let mut env: Vec<(OsString, OsString)> = Vec::new();
    for name in ESSENTIAL_ENV_VARS {
        if let Some(value) = lookup(parent, name) {
            env.push((OsString::from(name), value.clone()));
        }
    }
    for name in PROVIDER_ENV_VARS {
        if let Some(value) = lookup(parent, name) {
            env.push((OsString::from(name), value.clone()));
        }
    }
    for (key, value) in parent {
        if let Some(key) = key.to_str() {
            if key.to_ascii_uppercase().starts_with(AWS_CONTAINER_PREFIX) {
                env.push((OsString::from(key), value.clone()));
            }
        }
    }
    for name in PI_CONFIG_ENV_VARS {
        if let Some(value) = lookup(parent, name) {
            env.push((OsString::from(name), value.clone()));
        }
    }
    for (name, value) in PI_CONTROL_ENV_VARS {
        env.push((OsString::from(name), OsString::from(value)));
    }
    for (key, value) in extra_env {
        env.push((OsString::from(key), OsString::from(value)));
    }
    env
}

/// Probe environment: essentials + Pi location config + PI_* controls. No
/// provider credentials: a version probe never needs them.
pub(crate) fn minimal_probe_env(parent: &[(OsString, OsString)]) -> Vec<(OsString, OsString)> {
    minimal_child_env(parent, &[])
        .into_iter()
        .filter(|(key, _)| {
            let name = key.to_string_lossy();
            !PROVIDER_ENV_VARS
                .iter()
                .any(|provider| names_equal(provider, name.as_ref()))
                && !name.to_ascii_uppercase().starts_with(AWS_CONTAINER_PREFIX)
        })
        .collect()
}

/// True when a child environment variable name is on the explicit allowlist
/// (essentials, provider vars, Pi location config, PI_* controls ProtonSearch
/// sets) or is part of the AWS container-credential family. `extra_env` keys are
/// explicit app injections and are checked separately.
pub fn is_child_env_allowed(name: &str) -> bool {
    // Windows injects hidden per-drive current-directory entries (names such as
    // "=C:") into every child environment; Command::env_clear cannot remove
    // them. They contain only working-directory paths, never credentials.
    if name.starts_with('=') {
        return true;
    }
    ESSENTIAL_ENV_VARS
        .iter()
        .any(|allowed| names_equal(allowed, name))
        || PROVIDER_ENV_VARS.iter().any(|allowed| names_equal(allowed, name))
        || PI_CONFIG_ENV_VARS
            .iter()
            .any(|allowed| names_equal(allowed, name))
        || PI_CONTROL_ENV_VARS.iter().any(|(key, _)| names_equal(key, name))
        || name.to_ascii_uppercase().starts_with(AWS_CONTAINER_PREFIX)
}

pub(crate) fn parent_environment() -> Vec<(OsString, OsString)> {
    std::env::vars_os().collect()
}

/// Clear the inherited environment and install the allowlisted one.
pub(crate) fn apply_minimal_child_environment(command: &mut Command, extra_env: &[(String, String)]) {
    let env = minimal_child_env(&parent_environment(), extra_env);
    command.env_clear();
    for (key, value) in env {
        command.env(key, value);
    }
}

/// Same, without provider credentials (used by the --version/node probes).
pub(crate) fn apply_probe_environment(command: &mut Command) {
    let env = minimal_probe_env(&parent_environment());
    command.env_clear();
    for (key, value) in env {
        command.env(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(name: &str, value: &str) -> (OsString, OsString) {
        (OsString::from(name), OsString::from(value))
    }

    fn synthetic_parent() -> Vec<(OsString, OsString)> {
        vec![
            os("PATH", "C:\\Windows\\System32"),
            os("SystemRoot", "C:\\Windows"),
            os("TEMP", "C:\\Temp"),
            os("USERPROFILE", "C:\\Users\\tester"),
            os("OPENAI_API_KEY", "sk-test-openai"),
            os("AWS_ACCESS_KEY_ID", "AKIATEST"),
            os("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI", "/v2/credentials/abc"),
            os("FAKE_SECRET", "planted-super-secret-value"),
            os("GITHUB_TOKEN", "ghp_shouldnotleak"),
            os("PROTONSEARCH_AGENT_MODEL", "leftover"),
        ]
    }

    #[test]
    fn planted_secret_is_not_inherited() {
        let env = minimal_child_env(&synthetic_parent(), &[]);
        let names: Vec<String> = env
            .iter()
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect();
        assert!(!names.iter().any(|name| name == "FAKE_SECRET"), "{names:?}");
        assert!(!names.iter().any(|name| name == "GITHUB_TOKEN"), "{names:?}");
        assert!(
            !names.iter().any(|name| name == "PROTONSEARCH_AGENT_MODEL"),
            "{names:?}"
        );
        assert!(names.iter().any(|name| name == "OPENAI_API_KEY"), "{names:?}");
        assert!(names.iter().any(|name| name == "AWS_ACCESS_KEY_ID"), "{names:?}");
        assert!(
            names
                .iter()
                .any(|name| name == "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI"),
            "{names:?}"
        );
        assert!(names.iter().any(|name| name == "PI_OFFLINE"), "{names:?}");
    }

    #[test]
    fn pi_control_variables_are_set() {
        let env = minimal_child_env(&synthetic_parent(), &[]);
        let get = |name: &str| {
            env.iter()
                .find(|(key, _)| key.to_string_lossy() == name)
                .map(|(_, value)| value.to_string_lossy().into_owned())
        };
        assert_eq!(get("PI_OFFLINE").as_deref(), Some("1"));
        assert_eq!(get("PI_SKIP_VERSION_CHECK").as_deref(), Some("1"));
        assert_eq!(get("PI_TELEMETRY").as_deref(), Some("0"));
    }

    #[test]
    fn probe_env_has_no_credentials() {
        let env = minimal_probe_env(&synthetic_parent());
        let names: Vec<String> = env
            .iter()
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect();
        assert!(!names.iter().any(|name| name == "OPENAI_API_KEY"), "{names:?}");
        assert!(!names.iter().any(|name| name == "AWS_ACCESS_KEY_ID"), "{names:?}");
        assert!(
            !names
                .iter()
                .any(|name| name.starts_with("AWS_CONTAINER_CREDENTIALS_")),
            "{names:?}"
        );
        assert!(names.iter().any(|name| name == "PATH"), "{names:?}");
        assert!(names.iter().any(|name| name == "PI_OFFLINE"), "{names:?}");
    }

    #[test]
    fn extra_env_is_explicitly_forwarded_and_can_override() {
        let extra = vec![
            (
                "MY_PROVIDER_BASE_URL".to_string(),
                "http://localhost:11434".to_string(),
            ),
            ("OPENAI_API_KEY".to_string(), "sk-explicit".to_string()),
        ];
        let env = minimal_child_env(&synthetic_parent(), &extra);
        let get = |name: &str| {
            env.iter()
                .find(|(key, _)| key.to_string_lossy() == name)
                .map(|(_, value)| value.to_string_lossy().into_owned())
        };
        assert_eq!(
            get("MY_PROVIDER_BASE_URL").as_deref(),
            Some("http://localhost:11434")
        );
        // Later entries win for Command::env; the vector intentionally contains both.
        assert!(
            env.iter()
                .filter(|(key, _)| key.to_string_lossy() == "OPENAI_API_KEY")
                .count()
                >= 2
        );
    }

    #[test]
    fn every_generated_name_is_allowlisted() {
        let env = minimal_child_env(&synthetic_parent(), &[]);
        for (key, _) in env {
            let name = key.to_string_lossy();
            assert!(is_child_env_allowed(&name), "{name} is not allowlisted");
        }
    }
}
