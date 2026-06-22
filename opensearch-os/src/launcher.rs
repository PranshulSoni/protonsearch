use std::process::Command;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        UI::Shell::ShellExecuteW,
        UI::WindowsAndMessaging::SW_SHOWNORMAL,
        Foundation::HWND,
    },
};

pub fn launch(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() { return; }

    let cmd_lower = cmd.to_lowercase();
    if cmd.starts_with("http://") || cmd.starts_with("https://") || cmd_lower.ends_with(".lnk") {
        let cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            ShellExecuteW(
                HWND::default(),
                w!("open"),
                PCWSTR(cmd_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
        return;
    }

    let _ = if cmd.starts_with("ms-settings:") || cmd.starts_with("shell:") {
        Command::new("explorer.exe").arg(cmd).spawn()
    } else if let Some(rest) = cmd.strip_prefix("control.exe") {
        let mut c = Command::new("control.exe");
        for arg in rest.split_whitespace() {
            c.arg(arg);
        }
        c.spawn()
    } else if cmd.ends_with(".msc") {
        Command::new("mmc.exe").arg(cmd).spawn()
    } else {
        let mut parts = cmd.split_whitespace();
        if let Some(exe) = parts.next() {
            let mut c = Command::new(exe);
            for arg in parts {
                c.arg(arg);
            }
            c.spawn()
        } else {
            return;
        }
    };
}
