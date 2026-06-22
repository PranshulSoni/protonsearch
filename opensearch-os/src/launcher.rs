use std::process::Command;

pub fn launch(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() { return; }

    let _ = if cmd.starts_with("ms-settings:") {
        Command::new("explorer.exe").arg(cmd).spawn()
    } else if let Some(rest) = cmd.strip_prefix("control.exe") {
        let mut c = Command::new("control.exe");
        let rest = rest.trim();
        if !rest.is_empty() { c.arg(rest); }
        c.spawn()
    } else if cmd.ends_with(".msc") {
        Command::new("mmc.exe").arg(cmd).spawn()
    } else {
        // raw command — split on first space
        let (exe, arg) = cmd.split_once(' ').unwrap_or((cmd, ""));
        let mut c = Command::new(exe);
        if !arg.is_empty() { c.arg(arg); }
        c.spawn()
    };
}
