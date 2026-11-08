use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use windows::Win32::{
    Foundation::*,
    Graphics::{Dwm::*, Gdi::*},
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub work: [i32; 4],
    pub monitor: [i32; 4],
    pub dpi: u32,
    pub fullscreen: bool,
}

#[derive(Default, Clone, Copy)]
pub struct State {
    pub suppressed: bool,
    pub restore_visible: bool,
}

// Maximized captioned applications, shell surfaces and our own windows are not fullscreen content.
fn fullscreen(
    bounds: [i32; 4],
    monitor: [i32; 4],
    excluded: bool,
    captioned_maximized: bool,
) -> bool {
    !excluded
        && !captioned_maximized
        && bounds[0] <= monitor[0]
        && bounds[1] <= monitor[1]
        && bounds[2] >= monitor[2]
        && bounds[3] >= monitor[3]
}

pub fn sample(hwnd: HWND) -> Result<Snapshot, String> {
    // SAFETY: all calls are read-only; handles and output buffer sizes are validated here.
    unsafe {
        if !IsWindow(hwnd).as_bool() {
            return Err("Dock window is no longer available".into());
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return Err("Cannot inspect dock monitor".into());
        }
        let rect = |r: RECT| [r.left, r.top, r.right, r.bottom];
        let mut snapshot = Snapshot {
            work: rect(info.rcWork),
            monitor: rect(info.rcMonitor),
            dpi: GetDpiForWindow(hwnd),
            fullscreen: false,
        };
        let foreground = GetForegroundWindow();
        if foreground.0.is_null()
            || IsIconic(foreground).as_bool()
            || MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST) != monitor
        {
            return Ok(snapshot);
        }
        let mut pid = 0;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        let mut class = [0u16; 80];
        let len = GetClassNameW(foreground, &mut class);
        let shell = matches!(
            String::from_utf16_lossy(&class[..len.max(0) as usize]).as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        );
        let mut frame = RECT::default();
        // Physical DWM bounds avoid counting invisible resize borders or virtualized coordinates.
        if DwmGetWindowAttribute(
            foreground,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_err()
        {
            return Ok(snapshot);
        }
        let caption = GetWindowLongW(foreground, GWL_STYLE) as u32 & WS_CAPTION.0 == WS_CAPTION.0;
        snapshot.fullscreen = fullscreen(
            rect(frame),
            snapshot.monitor,
            pid == std::process::id() || shell,
            caption && IsZoomed(foreground).as_bool(),
        );
        Ok(snapshot)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Hide,
    Restore,
    Reposition,
    None,
}

impl State {
    pub fn update(&mut self, fullscreen: bool, visible: bool, monitor_changed: bool) -> Action {
        if fullscreen && !self.suppressed {
            self.suppressed = true;
            self.restore_visible = visible;
            return if visible { Action::Hide } else { Action::None };
        }
        if !fullscreen && self.suppressed {
            self.suppressed = false;
            let restore = std::mem::take(&mut self.restore_visible);
            return if restore {
                Action::Restore
            } else {
                Action::None
            };
        }
        if !fullscreen && visible && monitor_changed {
            Action::Reposition
        } else {
            Action::None
        }
    }
}

pub fn start(app: tauri::AppHandle, hwnd: isize, running: Arc<AtomicBool>) -> std::io::Result<()> {
    // shortcut: 750ms native sampling avoids global hooks; use shell notifications if measured latency requires it.
    std::thread::Builder::new()
        .name("dock-lifecycle".into())
        .spawn(move || {
            let mut previous = None;
            let retry = Arc::new(AtomicBool::new(false));
            while running.load(Ordering::Acquire) {
                if retry.swap(false, Ordering::AcqRel) {
                    previous = None;
                }
                let snapshot = sample(HWND(hwnd as *mut _));
                if let Ok(snapshot) = snapshot {
                    if previous != Some(snapshot) {
                        let monitor_changed = previous.is_none_or(|old: Snapshot| {
                            old.work != snapshot.work
                                || old.monitor != snapshot.monitor
                                || old.dpi != snapshot.dpi
                        });
                        let handle = app.clone();
                        let active = running.clone();
                        let retry = retry.clone();
                        if app
                            .run_on_main_thread(move || {
                                if !active.load(Ordering::Acquire) {
                                    return;
                                }
                                if let Err(error) =
                                    super::refresh_dock_lifecycle(&handle, monitor_changed)
                                {
                                    protonsearch::applog::log(&format!("Dock lifecycle: {error}"));
                                    retry.store(true, Ordering::Release);
                                }
                            })
                            .is_err()
                        {
                            break;
                        }
                        previous = Some(snapshot);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(750));
            }
        })
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_foreign_monitor_covering_content_suppresses_dock() {
        let monitor = [0, 0, 3000, 2000];
        assert!(fullscreen(monitor, monitor, false, false));
        assert!(!fullscreen([0, 0, 3000, 1950], monitor, false, false));
        assert!(!fullscreen(monitor, monitor, true, false));
        assert!(!fullscreen(monitor, monitor, false, true));
        assert!(fullscreen(
            [-2000, 0, 0, 1500],
            [-2000, 0, 0, 1500],
            false,
            false
        ));
    }
    #[test]
    fn fullscreen_resumes_only_a_previously_visible_dock() {
        let mut state = State::default();
        assert_eq!(state.update(true, true, false), Action::Hide);
        assert_eq!(state.update(true, false, true), Action::None);
        assert_eq!(state.update(false, false, false), Action::Restore);
        assert_eq!(state.update(false, true, false), Action::None);
        assert_eq!(state.update(true, false, false), Action::None);
        assert_eq!(state.update(false, false, true), Action::None);
        assert_eq!(state.update(false, true, true), Action::Reposition);
    }
    #[test]
    fn resume_reuses_measured_size_in_changed_work_area() {
        let mut state = State::default();
        let measured = (950.0, 132.0);
        let original =
            crate::window_geometry::bounds([0, 0, 3000, 1950], 1.0, measured.0, measured.1, true)
                .unwrap();
        assert_eq!(state.update(true, true, false), Action::Hide);
        assert_eq!(state.update(false, false, true), Action::Restore);
        let restored =
            crate::window_geometry::bounds([0, 0, 2560, 1400], 1.0, measured.0, measured.1, true)
                .unwrap();
        assert_eq!(&original[2..], &restored[2..]);
        assert_eq!(restored, [805, 1258, 950, 132]);
    }
}
