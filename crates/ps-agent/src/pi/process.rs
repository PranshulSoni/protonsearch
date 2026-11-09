//! Windows process helpers for the Pi backend.
//!
//! Pi is distributed as a `.cmd` launcher on Windows (`~/.pi/agent/bin/pi.cmd`),
//! and `CreateProcess` cannot execute batch files directly. We therefore wrap
//! the launcher in `cmd.exe /d /s /c`, build the inner command line ourselves so
//! every argument is quoted exactly once, and reject arguments containing
//! cmd.exe metacharacters (`" % ! ^ & | < >` and newlines). The user prompt is
//! never placed on the command line (it goes over stdin), so untrusted text
//! cannot reach the batch parser.
//!
//! Cancellation uses `taskkill /PID <pid> /T`, which TerminateProcess would not
//! do: Pi's real work happens in child Node processes and, when writes are
//! enabled, in bash/PowerShell grandchildren. The tree kill is required.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Windows CREATE_NO_WINDOW: the Pi CLI and its helpers must never flash a console.
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
pub(crate) fn configure_no_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn configure_no_window(_command: &mut Command) {}

#[cfg(windows)]
fn is_batch(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_batch(_path: &Path) -> bool {
    false
}

fn reject_cmd_metacharacters(arg: &str) -> Result<(), String> {
    if arg.chars().any(|character| {
        matches!(
            character,
            '"' | '%' | '!' | '^' | '&' | '|' | '<' | '>' | '\r' | '\n'
        ) || character.is_control()
    }) {
        return Err(
            "argument contains characters that are unsafe for the Windows batch launcher".to_string(),
        );
    }
    Ok(())
}

/// Build the command used to invoke the Pi CLI.
///
/// * `.cmd`/`.bat` launchers: `cmd.exe /d /s /c ""<launcher>" "<arg>" ..."`
///   (`/s` strips the outer quote pair; every argument is quoted once).
/// * executables: spawned directly (this also covers non-Windows installs).
///
/// All arguments are validated before use, so a settings-controlled model or
/// provider string cannot inject cmd.exe syntax.
pub(crate) fn pi_command(pi_path: &Path, args: &[String]) -> Result<Command, crate::AgentError> {
    if is_batch(pi_path) && cfg!(windows) {
        for arg in args {
            reject_cmd_metacharacters(arg)
                .map_err(|message| crate::AgentError::Pi(format!("{message}: {arg:?}")))?;
        }
        reject_cmd_metacharacters(&pi_path.to_string_lossy())
            .map_err(|message| crate::AgentError::Pi(format!("pi launcher path rejected: {message}")))?;

        let mut inner = String::new();
        inner.push('"');
        inner.push_str(&pi_path.to_string_lossy());
        inner.push('"');
        for arg in args {
            inner.push(' ');
            inner.push('"');
            inner.push_str(arg);
            inner.push('"');
        }
        // Outer quotes are consumed by cmd /s, leaving the quoted launcher path
        // and quoted arguments for normal cmd.exe parsing.
        let wrapped = format!("\"{inner}\"");

        let mut command = Command::new("cmd.exe");
        command.arg("/d").arg("/s").arg("/c");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.raw_arg(wrapped);
        }
        #[cfg(not(windows))]
        {
            command.arg(wrapped);
        }
        configure_no_window(&mut command);
        Ok(command)
    } else {
        let mut command = Command::new(pi_path);
        command.args(args);
        configure_no_window(&mut command);
        Ok(command)
    }
}

/// True when the process id is still alive (Windows: tasklist; Unix: kill -0).
pub(crate) fn is_pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        match output {
            Ok(output) => String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\"")),
            Err(_) => false,
        }
    }
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

#[cfg(windows)]
fn taskkill(pid: u32, force: bool) -> bool {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new("taskkill");
    command.args(["/PID", &pid.to_string(), "/T"]);
    if force {
        command.arg("/F");
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn signal_process(pid: u32, force: bool) -> bool {
    Command::new("kill")
        .arg(if force { "-KILL" } else { "-TERM" })
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Graceful tree termination, then force after the grace period.
///
/// On Windows the graceful step is `taskkill /PID /T` (no /F), followed by
/// `taskkill /PID /T /F` if the process tree is still alive after the grace.
pub(crate) fn terminate_process_tree(pid: u32, grace: Duration) {
    #[cfg(windows)]
    {
        let _ = taskkill(pid, false);
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            if !is_pid_alive(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = taskkill(pid, true);
        let force_deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < force_deadline && is_pid_alive(pid) {
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = signal_process(pid, false);
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            if !is_pid_alive(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = signal_process(pid, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_launcher_is_wrapped_with_cmd_and_quoted_args() {
        let command = pi_command(
            Path::new(r"C:\Program Files\Pi Agent\pi.cmd"),
            &["--mode".to_string(), "json".to_string()],
        )
        .expect("wrapper command");
        assert_eq!(command.get_program().to_string_lossy(), "cmd.exe");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "/d");
        assert_eq!(args[1], "/s");
        assert_eq!(args[2], "/c");
        #[cfg(windows)]
        {
            let line = &args[3];
            assert!(line.starts_with("\"\""), "{line}");
            assert!(line.ends_with("\"\""), "{line}");
            assert!(line.contains("\"C:\\Program Files\\Pi Agent\\pi.cmd\""), "{line}");
            assert!(line.contains("\"--mode\" \"json\""), "{line}");
        }
    }

    #[test]
    fn cmd_metacharacters_are_rejected() {
        for bad in ["a&b", "a|b", "a>b", "a<b", "a^b", "a%b", "a!b", "a\"b", "a\nb"] {
            assert!(
                pi_command(Path::new(r"C:\pi\pi.cmd"), &[bad.to_string()]).is_err(),
                "expected rejection for {bad:?}"
            );
        }
    }

    #[test]
    fn direct_executable_is_not_wrapped() {
        let command =
            pi_command(Path::new(r"C:\tools\pi.exe"), &["--version".to_string()]).expect("direct command");
        assert!(command.get_program().to_string_lossy().ends_with("pi.exe"));
    }
}
