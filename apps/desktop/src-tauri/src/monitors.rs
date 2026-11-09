//! Monitor enumeration with real work areas and DPI (L0 shell).
//!
//! Tauri's Monitor exposes full bounds only; the island placement editor and
//! placement maths need the work area (taskbar excluded) and per-monitor DPI.
//! This module reports physical pixel rectangles, matching WM display rects.

use ps_ipc::MonitorInfo;

#[cfg(windows)]
pub fn enumerate() -> Vec<MonitorInfo> {
    use std::ptr::null_mut;
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(data.0 as *mut Vec<MonitorInfo>);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        let ok = GetMonitorInfoW(
            monitor,
            &mut info.monitorInfo as *mut _ as *mut _,
        );
        if ok.as_bool() {
            let device = String::from_utf16_lossy(
                &info.szDevice[..info
                    .szDevice
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(info.szDevice.len())],
            );
            let rect = info.monitorInfo.rcMonitor;
            let work = info.monitorInfo.rcWork;
            let mut dpi_x = 96u32;
            let mut dpi_y = 96u32;
            let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
            monitors.push(MonitorInfo {
                id: device.clone(),
                name: device,
                // MONITORINFOF_PRIMARY = 0x1 (not re-exported by this windows crate version).
                primary: (info.monitorInfo.dwFlags & 1) != 0,
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left).max(0) as u32,
                height: (rect.bottom - rect.top).max(0) as u32,
                work_x: work.left,
                work_y: work.top,
                work_width: (work.right - work.left).max(0) as u32,
                work_height: (work.bottom - work.top).max(0) as u32,
                scale_factor: (dpi_x as f64 / 96.0).max(0.5),
            });
        }
        TRUE
    }

    let mut monitors: Vec<MonitorInfo> = Vec::new();
    let data = &mut monitors as *mut _ as isize;
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(callback), LPARAM(data));
    }
    if let Some(primary) = monitors.iter().position(|m| m.primary) {
        let first = monitors.remove(primary);
        monitors.insert(0, first);
    }
    let _ = null_mut::<u8>();
    monitors
}

#[cfg(not(windows))]
pub fn enumerate() -> Vec<MonitorInfo> {
    Vec::new()
}