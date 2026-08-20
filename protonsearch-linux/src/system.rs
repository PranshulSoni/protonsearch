use anyhow::{Context, Result};
use serde::Serialize;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_OUTPUT: usize = 64 * 1024;
const MAX_INPUT: usize = 1024 * 1024;
const MAX_IO_PER_PASS: usize = 64 * 1024;
const IO_POLL_INTERVAL: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, Serialize)]
pub struct CommandResult {
    pub program: String,
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

pub fn command_available(program: &str) -> bool {
    if program.contains('/') {
        return is_executable(std::path::Path::new(program));
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(program))
        .any(|candidate| is_executable(&candidate))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Launch an interactive desktop program without inheriting the launcher's
/// terminal or blocking the GTK event loop.
pub fn spawn_detached(program: &Path, args: &[&str]) -> Result<()> {
    let program = program
        .to_str()
        .context("program path is not valid UTF-8")?;
    if !is_executable(program.as_ref()) {
        anyhow::bail!("executable unavailable: {program}");
    }
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn {program}"))?;
    Ok(())
}

pub fn spawn_detached_command(program: &str, args: &[&str]) -> Result<()> {
    if !command_available(program) {
        anyhow::bail!("executable unavailable: {program}");
    }
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn {program}"))?;
    Ok(())
}

pub fn run(program: &str, args: &[&str]) -> Result<CommandResult> {
    if !command_available(program) {
        anyhow::bail!("executable unavailable: {program}");
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command);
    let mut child = command
        .spawn()
        .with_context(|| format!("spawn {program}"))?;
    finish_child(program, &mut child, None)
}

pub fn run_with_input(program: &str, args: &[&str], input: &str) -> Result<CommandResult> {
    if input.len() > MAX_INPUT {
        anyhow::bail!("input for {program} is too large");
    }
    if !command_available(program) {
        anyhow::bail!("executable unavailable: {program}");
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command);
    let mut child = command
        .spawn()
        .with_context(|| format!("spawn {program}"))?;
    finish_child(program, &mut child, Some(input))
}

fn finish_child(program: &str, child: &mut Child, input: Option<&str>) -> Result<CommandResult> {
    let mut stdin = child.stdin.take();
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let setup_result = (|| -> std::io::Result<()> {
        if let Some(pipe) = stdin.as_ref() {
            set_nonblocking(pipe)?;
        }
        if let Some(pipe) = stdout_pipe.as_ref() {
            set_nonblocking(pipe)?;
        }
        if let Some(pipe) = stderr_pipe.as_ref() {
            set_nonblocking(pipe)?;
        }
        Ok(())
    })();
    if let Err(error) = setup_result {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error).with_context(|| format!("configure pipes for {program}"));
    }

    let input = input.map(str::as_bytes).unwrap_or_default();
    let mut input_offset = 0;
    if input.is_empty() {
        stdin = None;
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let started = Instant::now();
    let mut timed_out = false;

    let status = loop {
        let mut made_progress = false;

        if let Some(pipe) = stdin.as_mut() {
            match write_available(pipe, input, &mut input_offset) {
                Ok(written) => made_progress |= written > 0,
                Err(error) if error.kind() == ErrorKind::BrokenPipe => input_offset = input.len(),
                Err(error) => {
                    terminate_child(child);
                    return Err(error).with_context(|| format!("write stdin for {program}"));
                }
            }
            if input_offset == input.len() {
                stdin = None;
            }
        }
        if let Some(pipe) = stdout_pipe.as_mut() {
            let (read, closed) = match read_available(pipe, &mut stdout) {
                Ok(result) => result,
                Err(error) => {
                    terminate_child(child);
                    return Err(error).with_context(|| format!("read stdout for {program}"));
                }
            };
            made_progress |= read > 0;
            if closed {
                stdout_pipe = None;
            }
        }
        if let Some(pipe) = stderr_pipe.as_mut() {
            let (read, closed) = match read_available(pipe, &mut stderr) {
                Ok(result) => result,
                Err(error) => {
                    terminate_child(child);
                    return Err(error).with_context(|| format!("read stderr for {program}"));
                }
            };
            made_progress |= read > 0;
            if closed {
                stderr_pipe = None;
            }
        }

        let child_status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                terminate_child(child);
                return Err(error).with_context(|| format!("wait for {program}"));
            }
        };
        if let Some(status) = child_status {
            break status.code();
        }
        if started.elapsed() >= COMMAND_TIMEOUT {
            timed_out = true;
            kill_process_group(child.id());
            let _ = child.kill();
            break child.wait()?.code();
        }
        if !made_progress {
            thread::sleep(IO_POLL_INTERVAL);
        }
    };

    // A grandchild may keep an inherited writer open after the direct child
    // exits. Read only bytes already buffered instead of waiting for EOF.
    if let Some(pipe) = stdout_pipe.as_mut() {
        let _ = read_available(pipe, &mut stdout)?;
    }
    if let Some(pipe) = stderr_pipe.as_mut() {
        let _ = read_available(pipe, &mut stderr)?;
    }

    Ok(CommandResult {
        program: program.to_string(),
        status,
        stdout: String::from_utf8_lossy(&stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&stderr).trim().to_string(),
        timed_out,
    })
}

fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        command.process_group(0);
    }
}

fn kill_process_group(pid: u32) {
    #[cfg(unix)]
    if let Ok(pid) = i32::try_from(pid) {
        if pid > 0 {
            unsafe {
                let _ = libc::kill(-pid, libc::SIGKILL);
            }
        }
    }
}

fn terminate_child(child: &mut Child) {
    kill_process_group(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

fn set_nonblocking<T: AsRawFd>(pipe: &T) -> std::io::Result<()> {
    let descriptor = pipe.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags == -1 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn write_available<W: Write>(
    writer: &mut W,
    input: &[u8],
    offset: &mut usize,
) -> std::io::Result<usize> {
    let mut written = 0;
    while *offset < input.len() && written < MAX_IO_PER_PASS {
        let end = input.len().min(*offset + (MAX_IO_PER_PASS - written));
        match writer.write(&input[*offset..end]) {
            Ok(0) => break,
            Ok(count) => {
                *offset += count;
                written += count;
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == ErrorKind::WouldBlock => break,
            Err(error) => return Err(error),
        }
    }
    Ok(written)
}

fn read_available<R: Read>(reader: &mut R, output: &mut Vec<u8>) -> std::io::Result<(usize, bool)> {
    let mut consumed = 0;
    let mut buffer = [0_u8; 8192];
    while consumed < MAX_IO_PER_PASS {
        let remaining = MAX_IO_PER_PASS - consumed;
        let read_len = remaining.min(buffer.len());
        match reader.read(&mut buffer[..read_len]) {
            Ok(0) => return Ok((consumed, true)),
            Ok(count) => {
                consumed += count;
                let retained = (MAX_OUTPUT - output.len()).min(count);
                output.extend_from_slice(&buffer[..retained]);
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == ErrorKind::WouldBlock => break,
            Err(error) => return Err(error),
        }
    }
    Ok((consumed, false))
}

pub fn open_target(target: &str) -> Result<CommandResult> {
    let target = target.trim();
    if target.is_empty() {
        anyhow::bail!("empty open target");
    }
    if target.contains('\n') || target.contains('\r') {
        anyhow::bail!("open target contains a newline");
    }
    if target.starts_with("http://") || target.starts_with("https://") {
        if command_available("gio") {
            return run("gio", &["open", target]);
        }
        return run("xdg-open", &[target]);
    }
    let path = std::path::Path::new(target);
    if !path.is_absolute() || !path.exists() {
        anyhow::bail!("open target must be an existing absolute path or http(s) URL");
    }
    if command_available("gio") {
        run("gio", &["open", target])
    } else {
        run("xdg-open", &[target])
    }
}

pub fn os_release() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("PRETTY_NAME="))
                .map(|line| {
                    line.trim_start_matches("PRETTY_NAME=")
                        .trim_matches('"')
                        .to_string()
                })
        })
        .unwrap_or_else(|| "Linux".to_string())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BatteryStatus {
    pub present: bool,
    pub percentage: Option<String>,
    pub state: Option<String>,
}

pub fn battery_status() -> BatteryStatus {
    let root = Path::new("/sys/class/power_supply");
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.filter_map(|entry| entry.ok()) {
            let path = entry.path();
            let Ok(kind) = std::fs::read_to_string(path.join("type")) else {
                continue;
            };
            if kind.trim() != "Battery" {
                continue;
            }
            return BatteryStatus {
                present: true,
                percentage: read_small(path.join("capacity")),
                state: read_small(path.join("status")),
            };
        }
    }
    if command_available("upower") {
        if let Ok(devices) = run("upower", &["-e"]) {
            if let Some(device) = devices
                .stdout
                .lines()
                .find(|line| line.contains("/battery_"))
            {
                if let Ok(details) = run("upower", &["-i", device.trim()]) {
                    return BatteryStatus {
                        present: details.status == Some(0),
                        percentage: value_after(&details.stdout, "percentage:"),
                        state: value_after(&details.stdout, "state:"),
                    };
                }
            }
        }
    }
    BatteryStatus {
        present: false,
        percentage: None,
        state: None,
    }
}

pub fn battery_available() -> bool {
    battery_status().present || command_available("upower")
}

fn read_small(path: std::path::PathBuf) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().chars().take(64).collect())
}

fn value_after(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix(key)
            .map(|value| value.trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_lookup_does_not_use_a_shell() {
        assert!(command_available("true"));
        assert!(!command_available(&format!("{} -c true", "sh")));
    }

    #[test]
    fn command_lookup_rejects_non_executable_files() {
        let path = std::env::temp_dir().join(format!(
            "protonsearch-not-executable-{}",
            std::process::id()
        ));
        std::fs::write(&path, b"not executable").unwrap();
        assert!(!command_available(&path.to_string_lossy()));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bounded_input_commands_capture_output() {
        let result = run_with_input("cat", &[], "hello").unwrap();
        assert_eq!(result.status, Some(0));
        assert_eq!(result.stdout, "hello");
        assert!(!result.timed_out);
    }

    #[test]
    fn captures_large_stdout_and_stderr_without_blocking() {
        let script = "i=0; while [ \"$i\" -lt 5000 ]; do \
                      printf 0123456789abcdef0123456789abcdef; \
                      printf fedcba9876543210fedcba9876543210 >&2; \
                      i=$((i + 1)); done";

        let result = run("sh", &["-c", script]).unwrap();

        assert_eq!(result.status, Some(0));
        assert_eq!(result.stdout.len(), MAX_OUTPUT);
        assert_eq!(result.stderr.len(), MAX_OUTPUT);
        assert!(!result.timed_out);
    }

    #[test]
    fn inherited_pipe_writers_do_not_delay_completed_commands() {
        let started = Instant::now();

        let result = run("sh", &["-c", "sleep 4 & printf ready"]).unwrap();

        assert_eq!(result.status, Some(0));
        assert_eq!(result.stdout, "ready");
        assert!(started.elapsed() < COMMAND_TIMEOUT);
    }
}
