//! Opt-in Explorer taskbar visibility, with a durable recovery lease.
//! We only restore windows which this dock actually hid; Explorer is never terminated.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, path::{Path, PathBuf}, rc::Rc};
use windows::Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*};

#[derive(Clone, Serialize, Deserialize)]
struct TaskbarWindow { hwnd: isize, pid: u32 }
#[derive(Serialize, Deserialize)]
struct Lease { owner: u32, windows: Vec<TaskbarWindow> }

fn lease_path(layout: &Path) -> PathBuf { layout.with_file_name("dock-taskbar-lease.json") }
fn taskbar_class(hwnd: HWND) -> bool {
    let mut class = [0u16; 80];
    // SAFETY: the fixed buffer is writable and the API bounds its output.
    let len = unsafe { GetClassNameW(hwnd, &mut class) };
    matches!(String::from_utf16_lossy(&class[..len.max(0) as usize]).as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
}
fn explorer_window(item: &TaskbarWindow) -> bool {
    let hwnd = HWND(item.hwnd as *mut _);
    let mut pid = 0;
    // SAFETY: handle is validated afresh, including its owning process before use.
    unsafe {
        if !IsWindow(hwnd).as_bool() || !taskbar_class(hwnd) { return false; }
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid == item.pid && crate::get_process_path(pid).is_some_and(|path| {
            Path::new(&path).file_name().is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("explorer.exe"))
        })
    }
}
fn owner_alive(pid: u32) -> bool {
    // SAFETY: process handle is closed on all success paths.
    unsafe {
        let Ok(process) = OpenProcess(SYNCHRONIZE, false, pid) else { return false; };
        let alive = WaitForSingleObject(process, 0) == WAIT_TIMEOUT;
        let _ = CloseHandle(process);
        alive
    }
}

/// Recover after the owned renderer exits, or recover a stale lease at startup.
/// A live renderer's lease is never reclaimed by an unrelated controller.
pub fn restore_after_exit(layout: &Path, exited_pid: Option<u32>) -> Result<()> {
    let path = lease_path(layout);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let lease: Lease = serde_json::from_slice(&bytes).context("invalid taskbar recovery lease")?;
    if exited_pid.map_or_else(|| owner_alive(lease.owner), |pid| pid != lease.owner) { return Ok(()); }
    for window in lease.windows {
        if explorer_window(&window) {
            // SAFETY: only the Explorer windows recorded before our hide operation are restored.
            unsafe { let _ = ShowWindow(HWND(window.hwnd as *mut _), SW_SHOWNOACTIVATE); }
        }
    }
    std::fs::remove_file(path)?;
    Ok(())
}

pub struct TaskbarVisibility {
    layout: PathBuf,
    hidden: bool,
    _thread: PhantomData<Rc<()>>,
}
impl TaskbarVisibility {
    pub fn new(layout: PathBuf) -> Self { Self { layout, hidden: false, _thread: PhantomData } }
    /// Call after the dock has a configured, visible native window, then once per heartbeat.
    /// Re-enumeration handles Explorer restart and newly connected monitors.
    pub fn set_hidden(&mut self, hidden: bool) -> Result<()> {
        if !hidden {
            if self.hidden { restore_after_exit(&self.layout, Some(std::process::id()))?; self.hidden = false; }
            return Ok(());
        }
        let path = lease_path(&self.layout);
        let mut lease = if self.hidden {
            serde_json::from_slice::<Lease>(&std::fs::read(&path)?)?
        } else { Lease { owner: std::process::id(), windows: Vec::new() } };
        unsafe extern "system" fn collect(hwnd: HWND, param: LPARAM) -> BOOL {
            // SAFETY: EnumWindows is synchronous; param points at our live Vec.
            unsafe {
                if taskbar_class(hwnd) && IsWindowVisible(hwnd).as_bool() {
                    let mut pid = 0;
                    GetWindowThreadProcessId(hwnd, Some(&mut pid));
                    let item = TaskbarWindow { hwnd: hwnd.0 as isize, pid };
                    if explorer_window(&item) { (*(param.0 as *mut Vec<TaskbarWindow>)).push(item); }
                }
            }
            TRUE
        }
        let mut visible = Vec::<TaskbarWindow>::new();
        // SAFETY: callback borrows the Vec only for the duration of enumeration.
        unsafe { EnumWindows(Some(collect), LPARAM(&mut visible as *mut _ as isize))?; }
        if visible.is_empty() { return Ok(()); }
        for item in &visible {
            if !lease.windows.iter().any(|old| old.hwnd == item.hwnd && old.pid == item.pid) { lease.windows.push(item.clone()); }
        }
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        // Save before hiding so controller crash recovery always has the original window set.
        let temporary = path.with_extension("json.tmp");
        let file = std::fs::File::create(&temporary)?;
        serde_json::to_writer(&file, &lease)?;
        file.sync_all()?;
        std::fs::rename(temporary, &path)?;
        self.hidden = true;
        for item in visible {
            if explorer_window(&item) { unsafe { let _ = ShowWindow(HWND(item.hwnd as *mut _), SW_HIDE); } }
        }
        Ok(())
    }
}
impl Drop for TaskbarVisibility {
    fn drop(&mut self) {
        if self.hidden {
            if let Err(error) = restore_after_exit(&self.layout, Some(std::process::id())) {
                crate::applog::log(&format!("cannot restore Windows taskbar: {error}"));
            }
        }
    }
}
