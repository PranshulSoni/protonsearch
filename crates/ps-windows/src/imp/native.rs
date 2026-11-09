//! Reviewed native commands (lock/sleep/shutdown/restart/empty bin/settings).
//!
//! All of these are fixed, enumerated operations. No command string is built
//! from user input; child processes receive structured argv only.

use std::process::Command;

use anyhow::{anyhow, bail};
use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{BOOLEAN, HWND, LPARAM, WPARAM};
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::UI::Shell::{
    SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
};

use super::shell::shell_execute;

fn no_window(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    {
        command
    }
}

pub(crate) fn lock_workstation() -> anyhow::Result<()> {
    unsafe { LockWorkStation() }.map_err(|error| anyhow!("LockWorkStation failed: {error}"))
}

pub(crate) fn sleep_pc() -> anyhow::Result<()> {
    let result = unsafe { SetSuspendState(BOOLEAN(0), BOOLEAN(0), BOOLEAN(0)) };
    if result.0 == 0 {
        bail!("SetSuspendState was refused by the system");
    }
    Ok(())
}

pub(crate) fn shutdown_pc() -> anyhow::Result<()> {
    run_shutdown_tool("/s")
}

pub(crate) fn restart_pc() -> anyhow::Result<()> {
    run_shutdown_tool("/r")
}

/// Runs the OS shutdown.exe with a fixed argv (no shell, no interpolation).
fn run_shutdown_tool(mode: &str) -> anyhow::Result<()> {
    let status = no_window(Command::new("shutdown.exe").arg(mode).arg("/t").arg("0"))
        .status()
        .map_err(|error| anyhow!("could not start shutdown.exe: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        bail!("shutdown.exe exited with {status}")
    }
}

pub(crate) fn empty_recycle_bin() -> anyhow::Result<()> {
    unsafe {
        SHEmptyRecycleBinW(
            HWND::default(),
            PCWSTR::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
    }
    .map_err(|error| anyhow!("SHEmptyRecycleBin failed: {error}"))
}

pub(crate) fn open_settings() -> anyhow::Result<()> {
    shell_execute("open", "ms-settings:", None)
}

pub(crate) fn open_task_manager() -> anyhow::Result<()> {
    shell_execute("open", "taskmgr.exe", None)
}

pub(crate) fn open_terminal() -> anyhow::Result<()> {
    if shell_execute("open", "wt.exe", None).is_ok() {
        return Ok(());
    }
    shell_execute("open", "powershell.exe", None)
}

/// Notify the shell that a user setting (e.g. theme) changed.
pub(crate) fn broadcast_setting_change() -> anyhow::Result<()> {
    let _ = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            1000,
            None,
        )
    };
    Ok(())
}
