//! Safe native action execution. OWNER: sources-content.
//!
//! Every action arrives as a structured ps_ipc::ActionSpec. Destructive actions
//! are explicit NativeCommand variants and are confirmed by the UI/engine before
//! they reach this module; this module only executes.

use std::path::Path;

use anyhow::anyhow;
use ps_ipc::{ActionSpec, NativeCommand, OpenOutcome};

use crate::quicklinks;

/// Execute a user-invoked action using the default v2 database for workflows.
pub fn execute(action: &ActionSpec) -> OpenOutcome {
    execute_with(action, None)
}

/// Execute a user-invoked action; v2_db is used to resolve RunWorkflow ids.
pub fn execute_with(action: &ActionSpec, v2_db: Option<&Path>) -> OpenOutcome {
    let result: anyhow::Result<String> = match action {
        ActionSpec::OpenPath {
            path,
            line,
            column,
        } => {
            let target = Path::new(path);
            if !target.exists() {
                return OpenOutcome {
                    ok: false,
                    message: format!("path does not exist: {path}"),
                };
            }
            ps_windows::open_path_at(target, *line, *column)
                .map(|_| format!("Opened {path}"))
        }
        ActionSpec::RevealPath { path } => {
            let target = Path::new(path);
            ps_windows::reveal_path(target).map(|_| format!("Revealed {path}"))
        }
        ActionSpec::OpenUrl { url } => ps_windows::open_url(url).map(|_| format!("Opened {url}")),
        ActionSpec::LaunchApp { app_id, target } => {
            ps_windows::launch_app(app_id, target.as_deref())
                .map(|_| format!("Launched {app_id}"))
        }
        ActionSpec::CopyText { text } => {
            ps_windows::set_clipboard_text(text).map(|_| "Copied to clipboard".to_string())
        }
        ActionSpec::RunNative { command } => return run_native(*command),
        ActionSpec::RunWorkflow { id } => run_workflow(id, v2_db),
        ActionSpec::None => {
            return OpenOutcome {
                ok: false,
                message: "no action".to_string(),
            }
        }
    };
    match result {
        Ok(message) => OpenOutcome { ok: true, message },
        Err(error) => OpenOutcome {
            ok: false,
            message: format!("{error:#}"),
        },
    }
}

/// Run a reviewed native command. Returns a structured outcome; never panics.
pub fn run_native(command: NativeCommand) -> OpenOutcome {
    let result: anyhow::Result<&'static str> = match command {
        NativeCommand::LockWorkstation => {
            ps_windows::lock_workstation().map(|_| "Workstation locked")
        }
        NativeCommand::Sleep => ps_windows::sleep_pc().map(|_| "Suspending"),
        NativeCommand::Shutdown => ps_windows::shutdown_pc().map(|_| "Shutting down"),
        NativeCommand::Restart => ps_windows::restart_pc().map(|_| "Restarting"),
        NativeCommand::EmptyRecycleBin => {
            ps_windows::empty_recycle_bin().map(|_| "Recycle bin emptied")
        }
        NativeCommand::OpenSettings => ps_windows::open_settings().map(|_| "Opened Settings"),
        NativeCommand::OpenTaskManager => {
            ps_windows::open_task_manager().map(|_| "Opened Task Manager")
        }
        NativeCommand::OpenTerminal => ps_windows::open_terminal().map(|_| "Opened terminal"),
        NativeCommand::ToggleDarkMode => toggle_dark_mode().map(|_| "Toggled app theme"),
        // Indexing state is owned by the engine; these are acknowledgements.
        NativeCommand::PauseIndexing => Ok("Indexing pause is handled by the engine"),
        NativeCommand::ResumeIndexing => Ok("Indexing resume is handled by the engine"),
    };
    match result {
        Ok(message) => OpenOutcome {
            ok: true,
            message: message.to_string(),
        },
        Err(error) => OpenOutcome {
            ok: false,
            message: format!("{error:#}"),
        },
    }
}

/// Flip HKCU Personalize AppsUseLightTheme/SystemUsesLightTheme and notify the
/// shell. Only the current user's settings are touched.
fn toggle_dark_mode() -> anyhow::Result<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    let key = hkcu
        .open_subkey_with_flags(path, KEY_QUERY_VALUE | KEY_SET_VALUE)
        .map_err(|error| anyhow!("cannot open Personalize registry key: {error}"))?;
    let current: u32 = key.get_value("AppsUseLightTheme").unwrap_or(1);
    let next = if current == 1 { 0u32 } else { 1u32 };
    key.set_value("AppsUseLightTheme", &next)
        .map_err(|error| anyhow!("cannot write AppsUseLightTheme: {error}"))?;
    let _ = key.set_value("SystemUsesLightTheme", &next);
    let _ = ps_windows::broadcast_setting_change();
    Ok(())
}

fn run_workflow(id: &str, v2_db: Option<&Path>) -> anyhow::Result<String> {
    let db = match v2_db {
        Some(path) => path.to_path_buf(),
        None => crate::v2_db_path(),
    };
    let entries = quicklinks::load(&db);
    let entry = entries
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| anyhow!("workflow not found: {id}"))?;
    match entry.kind {
        crate::WorkflowKind::Quicklink => {
            ps_windows::open_url(&entry.payload)?;
            Ok(format!("Opened {}", entry.name))
        }
        crate::WorkflowKind::Snippet => {
            ps_windows::set_clipboard_text(&entry.payload)?;
            Ok(format!("Copied snippet {}", entry.name))
        }
    }
}