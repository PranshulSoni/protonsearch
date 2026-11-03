//! Restore Explorer windows recorded in a legacy dock recovery lease.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use windows::Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*};

#[derive(Clone, Serialize, Deserialize)]
struct TaskbarWindow {
    hwnd: isize,
    pid: u32,
}
#[derive(Serialize, Deserialize)]
struct Lease {
    owner: u32,
    windows: Vec<TaskbarWindow>,
}

fn lease_path(layout: &Path) -> PathBuf {
    layout.with_file_name("dock-taskbar-lease.json")
}
fn taskbar_class(hwnd: HWND) -> bool {
    let mut class = [0u16; 80];
    // SAFETY: the fixed buffer is writable and the API bounds its output.
    let len = unsafe { GetClassNameW(hwnd, &mut class) };
    matches!(
        String::from_utf16_lossy(&class[..len.max(0) as usize]).as_str(),
        "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
    )
}
fn explorer_window(item: &TaskbarWindow) -> bool {
    let hwnd = HWND(item.hwnd as *mut _);
    let mut pid = 0;
    // SAFETY: handle is validated afresh, including its owning process before use.
    unsafe {
        if !IsWindow(hwnd).as_bool() || !taskbar_class(hwnd) {
            return false;
        }
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid == item.pid
            && crate::get_process_path(pid).is_some_and(|path| {
                Path::new(&path)
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("explorer.exe"))
            })
    }
}
fn owner_alive(pid: u32) -> bool {
    // SAFETY: the helper only queries the process identified by this PID.
    if !unsafe { crate::get_process_path(pid) }.is_some_and(|path| {
        std::env::current_exe().is_ok_and(|exe| exe.to_string_lossy().eq_ignore_ascii_case(&path))
    }) {
        return false;
    }
    // SAFETY: process handle is closed on all success paths.
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
            return false;
        };
        let alive = WaitForSingleObject(process, 0) == WAIT_TIMEOUT;
        let _ = CloseHandle(process);
        alive
    }
}

/// Recover after the owned renderer exits, or recover a stale lease at startup.
/// A live renderer's lease is never reclaimed by an unrelated controller.
pub fn restore_after_exit(layout: &Path, exited_pid: Option<u32>) -> Result<()> {
    let path = lease_path(layout);
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let lease = read_lease(file)?;
    if exited_pid.map_or_else(|| owner_alive(lease.owner), |pid| pid != lease.owner) {
        return Ok(());
    }
    for window in lease.windows {
        if explorer_window(&window) {
            // SAFETY: only the Explorer windows recorded before our hide operation are restored.
            unsafe {
                let _ = ShowWindow(HWND(window.hwnd as *mut _), SW_SHOWNOACTIVATE);
            }
        }
    }
    std::fs::remove_file(path)?;
    Ok(())
}

fn read_lease(file: std::fs::File) -> Result<Lease> {
    let mut bytes = Vec::new();
    file.take(65_537).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 65_536, "taskbar recovery lease too large");
    let lease: Lease = serde_json::from_slice(&bytes).context("invalid taskbar recovery lease")?;
    ensure!(
        lease.owner != 0 && lease.windows.len() <= 64,
        "invalid taskbar recovery owner or window count"
    );
    Ok(lease)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_invalid_lease_cannot_restore_or_delete_windows() {
        let dir =
            std::env::temp_dir().join(format!("protonsearch-recovery-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let layout = dir.join("dock-layout.json");
        let lease = lease_path(&layout);
        let _ = std::fs::remove_file(&lease);
        assert!(restore_after_exit(&layout, None).is_ok());
        std::fs::write(&lease, b"{\"owner\":0,\"windows\":[]}").unwrap();
        assert!(restore_after_exit(&layout, None).is_err());
        assert!(lease.exists());
        std::fs::remove_file(&lease).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
}
