//! Windows providers. Native handles never leave the calling thread.
use anyhow::{bail, Result};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::{Dwm::*, Gdi::*};
use windows::Win32::Media::Audio::{Endpoints::*, *};
use windows::Win32::System::{Com::*, Power::*, Threading::*};
use windows::Win32::UI::{HiDpi::*, WindowsAndMessaging::*};

#[derive(Clone, Debug)]
pub struct WindowEntry {
    pub hwnd: isize,
    pub process_id: u32,
    pub title: String,
    pub executable: Option<String>,
    pub identity: String,
}
#[derive(Clone, Debug)]
pub struct IconPixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub command: String,
    pub icon: Option<IconPixels>,
}
#[derive(Clone, Copy, Debug)]
pub struct AudioSnapshot {
    pub volume: u8,
    pub muted: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct PowerSnapshot {
    pub percentage: Option<u8>,
    pub charging: bool,
    pub ac_power: Option<bool>,
    pub remaining_minutes: Option<u32>,
}
#[derive(Clone, Debug)]
pub struct NetworkSnapshot {
    pub name: String,
    pub connected: bool,
    pub internet: bool,
    pub wireless: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorBounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub work_left: i32,
    pub work_top: i32,
    pub work_right: i32,
    pub work_bottom: i32,
    pub dpi: u32,
}

struct Apartment(bool);
impl Apartment {
    fn new() -> Result<Self> {
        // SAFETY: COM is initialized and balanced on this calling thread only.
        unsafe {
            let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if result.0 == 0x80010106u32 as i32 {
                return Ok(Self(false));
            }
            result.ok()?;
            Ok(Self(true))
        }
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            /* SAFETY: balances successful initialization on this thread. */
            unsafe { CoUninitialize() }
        }
    }
}

fn identity(executable: Option<&str>, pid: u32) -> String {
    executable
        .map(|p| p.replace('/', "\\").to_lowercase())
        .unwrap_or_else(|| format!("pid:{pid}"))
}

pub fn enumerate_windows() -> Vec<WindowEntry> {
    struct Collection {
        entries: Vec<WindowEntry>,
        visited: usize,
    }
    unsafe extern "system" fn collect(hwnd: HWND, param: LPARAM) -> BOOL {
        // SAFETY: EnumWindows invokes this synchronously with our valid collection pointer.
        unsafe {
            let data = &mut *(param.0 as *mut Collection);
            data.visited += 1;
            if data.visited > 4096 || data.entries.len() >= 256 {
                return FALSE;
            }
            if !IsWindowVisible(hwnd).as_bool()
                || GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.is_invalid())
                || GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW.0 as isize != 0
            {
                return TRUE;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 || pid == GetCurrentProcessId() {
                return TRUE;
            }
            let mut cloaked = 0u32;
            if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as _, 4).is_ok()
                && cloaked != 0
            {
                return TRUE;
            }
            let mut title = [0u16; 1024];
            let len = GetWindowTextW(hwnd, &mut title).max(0) as usize;
            let title = String::from_utf16_lossy(&title[..len]);
            if title.trim().is_empty() {
                return TRUE;
            }
            let executable = crate::get_process_path(pid);
            let identity = identity(executable.as_deref(), pid);
            data.entries.push(WindowEntry {
                hwnd: hwnd.0 as isize,
                process_id: pid,
                title,
                executable,
                identity,
            });
            TRUE
        }
    }
    let mut data = Collection {
        entries: Vec::new(),
        visited: 0,
    };
    // SAFETY: callback and collection remain alive throughout synchronous enumeration.
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut data as *mut _ as isize));
    }
    data.entries
}
fn validated(entry: &WindowEntry) -> Result<HWND> {
    let hwnd = HWND(entry.hwnd as _);
    // SAFETY: handle is checked before use; querying a stale HWND is supported by Win32.
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if !IsWindow(hwnd).as_bool() || pid != entry.process_id || pid == 0 {
            bail!("window is no longer available");
        }
    }
    Ok(hwnd)
}
pub fn activate_window(entry: &WindowEntry) -> Result<()> {
    let hwnd = validated(entry)?;
    // SAFETY: validated top-level window; Windows enforces foreground permissions.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if !SetForegroundWindow(hwnd).as_bool() {
            bail!("Windows denied foreground activation");
        }
    }
    Ok(())
}
pub fn close_window(entry: &WindowEntry) -> Result<()> {
    let hwnd = validated(entry)?;
    // SAFETY: validated window; WM_CLOSE allows the application to prompt for unsaved work.
    unsafe {
        PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0))?;
    }
    Ok(())
}

/// Renders an owned icon over black and white to recover alpha for legacy masked icons.
fn render_icon(icon: HICON, size: u32) -> Option<IconPixels> {
    let size = size.clamp(16, 256) as i32;
    // SAFETY: all GDI resources are owned locally and restored/deleted before returning.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(b) => b,
            Err(_) => {
                let _ = DeleteDC(dc);
                return None;
            }
        };
        let old = SelectObject(dc, bitmap);
        let len = size as usize * size as usize * 4;
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, len);
        pixels.fill(0);
        let first = DrawIconEx(dc, 0, 0, icon, size, size, 0, None, DI_NORMAL).is_ok();
        let _ = GdiFlush();
        let black = pixels.to_vec();
        pixels.fill(255);
        let second = DrawIconEx(dc, 0, 0, icon, size, size, 0, None, DI_NORMAL).is_ok();
        let _ = GdiFlush();
        let white = pixels.to_vec();
        let _ = SelectObject(dc, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(dc);
        if !first || !second {
            return None;
        }
        let mut rgba = Vec::with_capacity(len);
        for (b, w) in black.chunks_exact(4).zip(white.chunks_exact(4)) {
            let alpha = 255u8.saturating_sub(w[0].saturating_sub(b[0]));
            let channel = |v: u8| {
                if alpha == 0 {
                    0
                } else {
                    ((u32::from(v) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255) as u8
                }
            };
            rgba.extend_from_slice(&[channel(b[2]), channel(b[1]), channel(b[0]), alpha]);
        }
        Some(IconPixels {
            width: size as u32,
            height: size as u32,
            rgba,
        })
    }
}
pub fn icon_for_window(hwnd: isize, size: u32) -> Option<IconPixels> {
    // SAFETY: messages use bounded timeouts; borrowed icons are copied before rendering/destruction.
    unsafe {
        let hwnd = HWND(hwnd as _);
        if !IsWindow(hwnd).as_bool() {
            return None;
        }
        let mut value = 0usize;
        for kind in [ICON_BIG, ICON_SMALL2, ICON_SMALL] {
            let _ = SendMessageTimeoutW(
                hwnd,
                WM_GETICON,
                WPARAM(kind as usize),
                LPARAM(0),
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                75,
                Some(&mut value),
            );
            if value != 0 {
                break;
            }
        }
        if value == 0 {
            value = GetClassLongPtrW(hwnd, GCLP_HICON);
        }
        if value == 0 {
            value = GetClassLongPtrW(hwnd, GCLP_HICONSM);
        }
        if value == 0 {
            return None;
        }
        let owned = CopyIcon(HICON(value as _)).ok()?;
        let pixels = render_icon(owned, size);
        let _ = DestroyIcon(owned);
        pixels
    }
}
pub fn icon_for_path(path: &str, size: u32) -> Option<IconPixels> {
    let _apartment = Apartment::new().ok()?;
    // SAFETY: shell helpers return caller-owned icons; each icon is destroyed exactly once.
    unsafe {
        let mut icon = crate::get_app_icon(path);
        if icon.is_invalid() {
            icon = crate::get_file_icon(path);
        }
        if icon.is_invalid() {
            return None;
        }
        let pixels = render_icon(icon, size);
        let _ = DestroyIcon(icon);
        pixels
    }
}
pub fn installed_apps() -> Vec<InstalledApp> {
    let mut apps = crate::search::installed_apps();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.into_iter()
        .take(500)
        .map(|a| InstalledApp {
            id: identity(Some(&a.path), 0),
            name: a.name,
            command: a.path,
            icon: None,
        })
        .collect()
}
pub fn audio_snapshot() -> Result<AudioSnapshot> {
    let _apartment = Apartment::new()?;
    // SAFETY: interfaces are used and released within this initialized COM apartment.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        Ok(AudioSnapshot {
            volume: (endpoint.GetMasterVolumeLevelScalar()?.clamp(0., 1.) * 100.).round() as u8,
            muted: endpoint.GetMute()?.as_bool(),
        })
    }
}
pub fn set_volume(value: u8) -> Result<()> {
    let _apartment = Apartment::new()?;
    crate::launcher::set_master_volume(f32::from(value.min(100)) / 100.)?;
    Ok(())
}
pub fn set_mute(value: bool) -> Result<()> {
    let _apartment = Apartment::new()?;
    crate::launcher::set_mute(value)?;
    Ok(())
}
pub fn power_snapshot() -> Result<PowerSnapshot> {
    // SAFETY: the system writes a correctly sized initialized structure.
    unsafe {
        let mut s = SYSTEM_POWER_STATUS::default();
        GetSystemPowerStatus(&mut s)?;
        Ok(PowerSnapshot {
            percentage: (s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent),
            charging: s.BatteryFlag != 255 && s.BatteryFlag & 8 != 0,
            ac_power: match s.ACLineStatus {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            },
            remaining_minutes: (s.BatteryLifeTime != u32::MAX).then_some(s.BatteryLifeTime / 60),
        })
    }
}
pub fn network_snapshot() -> Result<NetworkSnapshot> {
    let _apartment = Apartment::new()?;
    use windows::Networking::Connectivity::{NetworkConnectivityLevel, NetworkInformation};
    let profile = match NetworkInformation::GetInternetConnectionProfile() {
        Ok(p) => p,
        Err(_) => {
            return Ok(NetworkSnapshot {
                name: "Offline".into(),
                connected: false,
                internet: false,
                wireless: false,
            })
        }
    };
    let level = profile.GetNetworkConnectivityLevel()?;
    Ok(NetworkSnapshot {
        name: profile.ProfileName()?.to_string(),
        connected: level != NetworkConnectivityLevel::None,
        internet: level == NetworkConnectivityLevel::InternetAccess,
        wireless: profile.IsWlanConnectionProfile()?,
    })
}
pub fn monitors() -> Vec<MonitorBounds> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        param: LPARAM,
    ) -> BOOL {
        // SAFETY: callback receives valid vector pointer for synchronous enumeration.
        unsafe {
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                let mut x = 96;
                let mut y = 96;
                let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y);
                let r = info.rcMonitor;
                let w = info.rcWork;
                (*(param.0 as *mut Vec<MonitorBounds>)).push(MonitorBounds {
                    left: r.left,
                    top: r.top,
                    right: r.right,
                    bottom: r.bottom,
                    work_left: w.left,
                    work_top: w.top,
                    work_right: w.right,
                    work_bottom: w.bottom,
                    dpi: x,
                });
            }
            TRUE
        }
    }
    let mut output = Vec::new();
    // SAFETY: callback and output are valid until enumeration returns.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut output as *mut _ as isize),
        );
    }
    output
}
pub fn primary_monitor() -> Option<MonitorBounds> {
    let all = monitors();
    all.iter()
        .find(|m| m.left == 0 && m.top == 0)
        .copied()
        .or_else(|| all.first().copied())
}
fn covers_monitor(r: RECT, m: MonitorBounds) -> bool {
    r.left <= m.left + 2 && r.top <= m.top + 2 && r.right >= m.right - 2 && r.bottom >= m.bottom - 2
}
pub fn fullscreen_on_monitor(m: MonitorBounds, exclude: &[isize]) -> bool {
    // SAFETY: foreground window is queried only; stale/invalid handles fail closed.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid()
            || exclude.contains(&(hwnd.0 as isize))
            || !IsWindowVisible(hwnd).as_bool()
        {
            return false;
        }
        let mut class = [0u16; 128];
        let n = GetClassNameW(hwnd, &mut class).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        if matches!(
            class.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return false;
        }
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).is_ok() && covers_monitor(rect, m)
    }
}
pub fn configure_window(hwnd: HWND, nonactivating: bool) -> Result<()> {
    // SAFETY: caller provides its live native window. Unsupported DWM attributes are optional.
    unsafe {
        if !IsWindow(hwnd).as_bool() {
            bail!("invalid dock window");
        }
        let mut style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) | WS_EX_TOOLWINDOW.0 as isize;
        style &= !(WS_EX_APPWINDOW.0 as isize);
        if nonactivating {
            style |= WS_EX_NOACTIVATE.0 as isize;
        } else {
            style &= !(WS_EX_NOACTIVATE.0 as isize);
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )?;
        let rounded = 2u32;
        let backdrop = 3u32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &rounded as *const _ as _,
            4,
        );
        let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(38), &backdrop as *const _ as _, 4);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_normalizes_path() {
        assert_eq!(identity(Some("C:/Apps/Test.EXE"), 12), "c:\\apps\\test.exe");
        assert_eq!(identity(None, 12), "pid:12");
    }
    #[test]
    fn fullscreen_supports_negative_monitor_coordinates() {
        let m = MonitorBounds {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1080,
            work_left: -1920,
            work_top: 0,
            work_right: 0,
            work_bottom: 1040,
            dpi: 96,
        };
        assert!(covers_monitor(
            RECT {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1080
            },
            m
        ));
        assert!(!covers_monitor(
            RECT {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1040
            },
            m
        ));
    }
}
