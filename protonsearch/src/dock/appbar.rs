//! Separate hidden AppBar window reserves work area without changing Explorer's taskbar.
use super::platform::{self, MonitorBounds};
use anyhow::{bail, Result};
use std::{cell::Cell, marker::PhantomData, rc::Rc};
use windows::core::w;
use windows::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Shell::*, WindowsAndMessaging::*},
};
const CALLBACK: u32 = WM_APP + 171;

struct State {
    hwnd: Cell<HWND>,
    monitor: Cell<MonitorBounds>,
    height: Cell<i32>,
    rect: Cell<RECT>,
    busy: Cell<bool>,
    registered: Cell<bool>,
    taskbar_created: u32,
}
impl State {
    fn data(&self) -> APPBARDATA {
        APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: self.hwnd.get(),
            uCallbackMessage: CALLBACK,
            uEdge: ABE_BOTTOM,
            ..Default::default()
        }
    }
    fn register(&self) -> Result<()> {
        let mut data = self.data();
        // SAFETY: APPBARDATA describes our live hidden window and remains valid for this call.
        if unsafe { SHAppBarMessage(ABM_NEW, &mut data) } == 0 {
            bail!("Windows rejected AppBar registration");
        }
        self.registered.set(true);
        Ok(())
    }
    fn position(&self) -> Result<()> {
        if self.busy.replace(true) {
            return Ok(());
        }
        let result = self.position_inner();
        self.busy.set(false);
        result
    }
    fn position_inner(&self) -> Result<()> {
        let monitor = self.monitor.get();
        let height = self
            .height
            .get()
            .clamp(1, (monitor.bottom - monitor.top).max(1));
        let mut data = self.data();
        data.rc = RECT {
            left: monitor.left,
            top: monitor.top,
            right: monitor.right,
            bottom: monitor.bottom,
        };
        // SAFETY: the shell negotiates the work-area rectangle for this registered AppBar.
        unsafe {
            if SHAppBarMessage(ABM_QUERYPOS, &mut data) == 0 {
                bail!("AppBar position negotiation failed");
            }
            data.rc.top = data.rc.bottom.saturating_sub(height).max(data.rc.top);
            // ABM_SETPOS broadcasts ABN_POSCHANGED. Do not re-broadcast an unchanged
            // reservation, otherwise other appbars can create a position feedback loop.
            let previous = self.rect.get();
            if same_rect(previous, data.rc) {
                return Ok(());
            }
            if SHAppBarMessage(ABM_SETPOS, &mut data) == 0 {
                bail!("AppBar work-area reservation failed");
            }
            self.rect.set(data.rc);
            SetWindowPos(
                self.hwnd.get(),
                HWND_TOPMOST,
                data.rc.left,
                data.rc.top,
                (data.rc.right - data.rc.left).max(1),
                (data.rc.bottom - data.rc.top).max(1),
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )?;
        }
        Ok(())
    }
    fn refresh_monitor(&self) {
        let old = self.monitor.get();
        let monitors = platform::monitors();
        // Prefer the same origin; fall back to the nearest surviving display after removal.
        if let Some(next) = monitors.into_iter().min_by_key(|m| {
            let dx = i64::from(m.left) - i64::from(old.left);
            let dy = i64::from(m.top) - i64::from(old.top);
            dx * dx + dy * dy
        }) {
            self.monitor.set(next);
        }
    }
}

/// Must be created, updated, and dropped on the thread which pumps its native messages.
/// The Slint visual window remains separate from this reservation window.
pub struct AppBar {
    state: Box<State>,
    _thread: PhantomData<Rc<()>>,
}
impl AppBar {
    pub fn new(monitor: MonitorBounds, height_px: i32) -> Result<Self> {
        // SAFETY: class and window are created on the current UI thread; callback does not unwind.
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance.into(),
                lpszClassName: w!("ProtonSearchDockAppBar"),
                ..Default::default()
            };
            // Reusing an already registered class on this module is expected.
            if RegisterClassW(&class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
                return Err(windows::core::Error::from_win32().into());
            }
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("ProtonSearchDockAppBar"),
                w!("ProtonSearch dock work-area reservation"),
                WS_POPUP,
                monitor.left,
                monitor.bottom - 1,
                1,
                1,
                None,
                None,
                instance,
                None,
            )?;
            let state = Box::new(State {
                hwnd: Cell::new(hwnd),
                monitor: Cell::new(monitor),
                height: Cell::new(height_px),
                rect: Cell::new(RECT::default()),
                busy: Cell::new(false),
                registered: Cell::new(false),
                taskbar_created: RegisterWindowMessageW(w!("TaskbarCreated")),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*state as *const State) as isize);
            let bar = Self {
                state,
                _thread: PhantomData,
            };
            bar.state.register()?;
            bar.state.position()?;
            Ok(bar)
        }
    }
    pub fn rect(&self) -> RECT {
        self.state.rect.get()
    }
    pub fn update(&mut self, monitor: MonitorBounds, height_px: i32) -> Result<()> {
        // rcWork changes when this AppBar reserves space. It is output, not an
        // input geometry change, so comparing the entire MonitorBounds oscillates.
        let previous = self.state.monitor.get();
        if same_display(previous, monitor) && self.state.height.get() == height_px {
            self.state.monitor.set(monitor);
            return Ok(());
        }
        self.state.monitor.set(monitor);
        self.state.height.set(height_px);
        self.state.position()
    }
}
fn same_rect(a: RECT, b: RECT) -> bool {
    (a.left, a.top, a.right, a.bottom) == (b.left, b.top, b.right, b.bottom)
}
fn same_display(a: MonitorBounds, b: MonitorBounds) -> bool {
    (a.left, a.top, a.right, a.bottom, a.dpi) == (b.left, b.top, b.right, b.bottom, b.dpi)
}
impl Drop for AppBar {
    fn drop(&mut self) {
        // SAFETY: the !Send owner is destroyed on its creation thread. Clear callback pointer first.
        unsafe {
            SetWindowLongPtrW(self.state.hwnd.get(), GWLP_USERDATA, 0);
            if self.state.registered.replace(false) {
                let mut data = self.state.data();
                SHAppBarMessage(ABM_REMOVE, &mut data);
            }
            let _ = DestroyWindow(self.state.hwnd.get());
        }
    }
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: userdata is a stable boxed State owned until Drop clears it before DestroyWindow.
    unsafe {
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
        if !state.is_null() {
            let state = &*state;
            if state.taskbar_created != 0 && message == state.taskbar_created {
                state.registered.set(false);
                state.rect.set(RECT::default());
                state.refresh_monitor();
                if state.register().is_ok() {
                    let _ = state.position();
                }
                return LRESULT(0);
            }
            match message {
                CALLBACK if wparam.0 == ABN_POSCHANGED as usize => {
                    let _ = state.position();
                    return LRESULT(0);
                }
                WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
                    state.refresh_monitor();
                    let _ = state.position();
                    return LRESULT(0);
                }
                WM_WINDOWPOSCHANGED => {
                    if !state.busy.get() {
                        let mut data = state.data();
                        SHAppBarMessage(ABM_WINDOWPOSCHANGED, &mut data);
                    }
                }
                WM_CLOSE => {
                    return LRESULT(0);
                }
                WM_MOUSEACTIVATE => {
                    return LRESULT(MA_NOACTIVATE as isize);
                }
                _ => {}
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn work_area_changes_do_not_move_our_reservation() {
        let a = MonitorBounds { left: -1920, top: 0, right: 0, bottom: 1080,
            work_left: -1920, work_top: 0, work_right: 0, work_bottom: 1040, dpi: 96 };
        let mut b = a;
        b.work_bottom = 960;
        assert!(same_display(a, b));
        b.dpi = 144;
        assert!(!same_display(a, b));
    }
}
