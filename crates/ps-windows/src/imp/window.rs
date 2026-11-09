//! Foreground-window queries and process introspection.

use std::path::PathBuf;

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

use crate::WindowInfo;

/// Current foreground window, skipping our own process.
pub(crate) fn foreground_window() -> Option<WindowInfo> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 || pid == unsafe { GetCurrentProcessId() } {
        return None;
    }
    Some(WindowInfo {
        title: window_title(hwnd),
        process_id: pid,
        exe_path: process_exe(pid),
    })
}

/// Executable name (stem, e.g. "Code") of the foreground window's process.
pub(crate) fn foreground_process_name() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    let exe = process_exe(pid)?;
    exe.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .or_else(|| exe.file_name().map(|s| s.to_string_lossy().to_string()))
}

fn window_title(hwnd: HWND) -> String {
    let mut buf = vec![0u16; 512];
    loop {
        let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
        if len == 0 {
            return String::new();
        }
        let len = len as usize;
        if len < buf.len() {
            return String::from_utf16_lossy(&buf[..len]);
        }
        // Window title longer than the buffer: grow once, bounded.
        if buf.len() >= 32768 {
            return String::from_utf16_lossy(&buf[..len.min(buf.len())]);
        }
        buf.resize(buf.len() * 2, 0);
    }
}

fn process_exe(pid: u32) -> Option<PathBuf> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buf = [0u16; MAX_PATH as usize + 260];
    let mut size = buf.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
    };
    let _ = unsafe { CloseHandle(handle) };
    result.ok()?;
    let path = String::from_utf16_lossy(&buf[..size as usize]);
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}
