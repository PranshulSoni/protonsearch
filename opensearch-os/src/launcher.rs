use std::process::Command;

pub fn launch(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() { return; }

    let _ = if cmd.starts_with("ms-settings:") {
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
