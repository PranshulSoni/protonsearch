use std::path::Path;
use serde::Serialize;
use super::{probe_pi_cached, pi_version_supported, PROVIDER_ENV_VARS};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiInstallStatus {
    pub state: &'static str,
    pub path: Option<String>,
    pub version: Option<String>,
    pub node_ok: bool,
    pub authentication_configured: bool,
    pub installation_method: Option<&'static str>,
    pub git_bash_available: bool,
    pub detail: String,
}

/// Checks presence only. Credential values never leave this function.
fn authentication_configured(provider: Option<&str>) -> bool {
    if PROVIDER_ENV_VARS.iter().filter(|name| name.ends_with("_KEY") || name.ends_with("_TOKEN"))
        .any(|name| provider.is_none_or(|provider| name.starts_with(&provider.to_uppercase().replace('-', "_")))
            && std::env::var_os(name).is_some_and(|value| !value.is_empty())) {
        return true;
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    let Some(home) = home else { return false; };
    let path = std::path::PathBuf::from(home).join(".pi/agent/auth.json");
    let Ok(metadata) = std::fs::metadata(&path) else { return false; };
    if metadata.len() > 1024 * 1024 { return false; }
    let Ok(bytes) = std::fs::read(path) else { return false; };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return false; };
    let Some(entries) = value.as_object() else { return false; };
    entries.iter().any(|(name, entry)| {
        provider.is_none_or(|selected| selected == name)
            && entry.as_object().is_some_and(|fields| {
                ["key", "access", "refresh", "apiKey", "accessToken"].iter()
                    .any(|field| fields.get(*field).and_then(|value| value.as_str()).is_some_and(|value| !value.is_empty()))
            })
    })
}

pub fn status(data_dir: &Path, explicit: Option<&Path>, provider: Option<&str>, running: bool) -> PiInstallStatus {
    let managed = super::install::managed_launcher(data_dir);
    let selected = explicit.or_else(|| managed.is_file().then_some(managed.as_path()));
    let info = probe_pi_cached(selected);
    let auth = authentication_configured(provider);
    let git_bash = ["ProgramFiles", "ProgramFiles(x86)"]
        .iter().filter_map(std::env::var_os)
        .any(|root| std::path::PathBuf::from(root).join("Git/bin/bash.exe").is_file());
    let Some(info) = info else {
        return PiInstallStatus { state: "not_installed", path: None, version: None,
            node_ok: super::node_version_ok(), authentication_configured: auth,
            installation_method: None, git_bash_available: git_bash,
            detail: "Install Pi or select an existing Pi executable.".into() };
    };
    let (state, detail) = if running { ("running", "Pi is running a workflow.") }
        else if !info.node_ok { ("error", "Install Node.js 22.19 or newer.") }
        else if !pi_version_supported(&info.version) { ("version_mismatch", "Update Pi to a supported version.") }
        else if !auth { ("needs_authentication", "Connect a provider in Pi with /login, or configure your local model. Provider charges may apply.") }
        else { ("ready", "Pi credentials are configured. The provider connection is verified when a run starts.") };
    PiInstallStatus { state, path: Some(info.path.to_string_lossy().into_owned()), version: Some(info.version),
        node_ok: info.node_ok, authentication_configured: auth,
        installation_method: Some(if info.path.starts_with(data_dir.join("pi-runtime")) { "app_managed" } else {
            match info.source { super::PiSource::Explicit => "explicit", super::PiSource::Managed => "user_managed", _ => "external" }
        }), git_bash_available: git_bash, detail: detail.into() }
}
