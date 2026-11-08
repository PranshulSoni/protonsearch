use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use protonsearch::search::SearchResult;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{CloseHandle, FILETIME, HANDLE, HWND, LPARAM, WPARAM},
        Graphics::Gdi::*,
        System::Threading::{
            GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::*,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Identity {
    hwnd: isize,
    pid: u32,
    created: u64,
    title: String,
    class: String,
}
#[derive(Default)]
struct State {
    next: u64,
    entries: HashMap<String, Identity>,
    icons: HashMap<(isize, u32, u64), Option<String>>,
}
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State::default()))
}
fn issued_id(state: &mut State, identity: &Identity) -> String {
    if let Some((id, _)) = state.entries.iter().find(|(_, value)| *value == identity) {
        return id.clone();
    }
    state.next = state.next.wrapping_add(1);
    format!("running-{}", state.next)
}

fn identity(hwnd: HWND) -> Option<Identity> {
    unsafe {
        if !IsWindow(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 || pid == std::process::id() {
            return None;
        }
        let mut title = [0u16; 512];
        let mut class = [0u16; 256];
        let title_len = GetWindowTextW(hwnd, &mut title) as usize;
        let class_len = GetClassNameW(hwnd, &mut class) as usize;
        if title_len == 0 || class_len == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let (mut created, mut exit, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        let result = GetProcessTimes(process, &mut created, &mut exit, &mut kernel, &mut user);
        let _ = CloseHandle(process);
        result.ok()?;
        Some(Identity {
            hwnd: hwnd.0 as isize,
            pid,
            created: ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64,
            title: String::from_utf16_lossy(&title[..title_len]),
            class: String::from_utf16_lossy(&class[..class_len]),
        })
    }
}

fn icon(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut handle = 0usize;
        for kind in [ICON_BIG, ICON_SMALL2, ICON_SMALL] {
            let _ = SendMessageTimeoutW(
                hwnd,
                WM_GETICON,
                WPARAM(kind as usize),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                40,
                Some(&mut handle),
            );
            if handle != 0 {
                break;
            }
        }
        if handle == 0 {
            handle = GetClassLongPtrW(hwnd, GCLP_HICON);
        }
        if handle == 0 {
            handle = GetClassLongPtrW(hwnd, GCLP_HICONSM);
        }
        let window = (handle != 0).then(|| HICON(handle as *mut _));
        let window_px = window.and_then(icon_size);
        let path = process_path(hwnd);
        let prefer = path
            .as_deref()
            .is_some_and(|p| prefer_executable_icon(window_px, p));
        if prefer {
            if let Some(i) = path.as_deref().and_then(executable_icon) {
                return Some(i);
            }
        }
        if let Some(i) = window.and_then(render_icon) {
            return Some(i);
        }
        if prefer {
            None
        } else {
            path.as_deref().and_then(executable_icon)
        }
    }
}

// SAFETY: hwnd is passed only to Win32 queries; the process handle is closed on every path.
unsafe fn process_path(hwnd: HWND) -> Option<Vec<u16>> {
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut path = [0u16; 32768];
    let mut length = path.len() as u32;
    let queried = QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_WIN32,
        PWSTR(path.as_mut_ptr()),
        &mut length,
    )
    .is_ok();
    let _ = CloseHandle(process);
    if !queried || length as usize >= path.len() {
        return None;
    }
    path[length as usize] = 0;
    Some(path[..=length as usize].to_vec())
}

fn prefer_executable_icon(window_px: Option<u32>, exe: &[u16]) -> bool {
    let text = String::from_utf16_lossy(exe);
    let file = text
        .trim_end_matches('\0')
        .rsplit('\\')
        .next()
        .unwrap_or_default();
    // ApplicationFrameHost hosts UWP apps; its exe icon is generic, so keep the window icon.
    if file.eq_ignore_ascii_case("applicationframehost.exe") {
        return false;
    }
    window_px.map_or(true, |px| px < 64)
}

fn executable_icon(path: &[u16]) -> Option<String> {
    // SAFETY: callers provide a NUL-terminated path; the extracted icon is destroyed before return.
    unsafe {
        let end = path.iter().position(|&unit| unit == 0)?;
        // ponytail: PrivateExtractIconsW takes a MAX_PATH array; longer paths fall back to no icon.
        let mut file = [0u16; 260];
        if end >= file.len() {
            return None;
        }
        file[..end].copy_from_slice(&path[..end]);
        let mut handle = HICON::default();
        if PrivateExtractIconsW(
            &file,
            0,
            256,
            256,
            Some(std::slice::from_mut(&mut handle)),
            None,
            0,
        ) == 0
            || handle.0.is_null()
        {
            return None;
        }
        let result = render_icon(handle);
        let _ = DestroyIcon(handle);
        result
    }
}

fn icon_size(handle: HICON) -> Option<u32> {
    // SAFETY: GetIconInfo copies the icon bitmaps; both copies are released before return.
    unsafe {
        let mut info = ICONINFO::default();
        if GetIconInfo(handle, &mut info).is_err() {
            return None;
        }
        let mut bitmap = BITMAP::default();
        let read = GetObjectW(
            info.hbmColor,
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bitmap as *mut _ as *mut std::ffi::c_void),
        );
        let _ = DeleteObject(info.hbmColor);
        let _ = DeleteObject(info.hbmMask);
        if read == 0 || bitmap.bmWidth <= 0 {
            return None;
        }
        Some(bitmap.bmWidth.clamp(16, 256) as u32)
    }
}

fn render_icon(handle: HICON) -> Option<String> {
    render_icon_sized(handle, icon_size(handle).unwrap_or(48))
}

fn render_icon_sized(handle: HICON, size: u32) -> Option<String> {
    // SAFETY: handle is borrowed; all temporary GDI resources are released before return.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.0.is_null() {
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size as i32,
                biHeight: -(size as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap =
            match CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, HANDLE::default(), 0) {
                Ok(bitmap) => bitmap,
                Err(_) => {
                    let _ = DeleteDC(dc);
                    return None;
                }
            };
        if bits.is_null() {
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(dc);
            return None;
        }
        let old = SelectObject(dc, bitmap);
        if old.0.is_null() || old.0 as isize == -1 {
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(dc);
            return None;
        }
        let bytes = size as usize * size as usize * 4;
        std::ptr::write_bytes(bits, 0, bytes);
        let drawn = DrawIconEx(
            dc,
            0,
            0,
            handle,
            size as i32,
            size as i32,
            0,
            None,
            DI_NORMAL,
        )
        .is_ok();
        if !drawn || !GdiFlush().as_bool() {
            let _ = SelectObject(dc, old);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(dc);
            return None;
        }
        let mut rgba = std::slice::from_raw_parts(bits as *const u8, bytes).to_vec();
        let _ = SelectObject(dc, old);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(dc);
        for p in rgba.chunks_exact_mut(4) {
            p.swap(0, 2);
            if p[3] > 0 {
                for c in 0..3 {
                    p[c] = ((p[c] as u32 * 255 / p[3] as u32).min(255)) as u8;
                }
            }
        }
        if !rgba.chunks_exact(4).any(|p| p[3] != 0) {
            return None;
        }
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(&rgba, size, size, ColorType::Rgba8)
            .ok()?;
        Some(format!("data:image/png;base64,{}", STANDARD.encode(png)))
    }
}

pub fn snapshot(results: Vec<SearchResult>) -> Result<Value, String> {
    let mut state = state()
        .lock()
        .map_err(|_| "Running applications unavailable")?;
    let mut entries = HashMap::new();
    let mut items = Vec::new();
    let foreground = unsafe { GetForegroundWindow() };
    for result in results {
        if items.len() == 12 {
            break;
        }
        let Some(raw) = result
            .entry
            .launch_command
            .strip_prefix("window:")
            .and_then(|s| s.parse::<isize>().ok())
        else {
            continue;
        };
        let hwnd = HWND(raw as *mut _);
        let Some(identity) = identity(hwnd) else {
            continue;
        };
        let key = (identity.hwnd, identity.pid, identity.created);
        let image = state
            .icons
            .get(&key)
            .cloned()
            .flatten()
            .or_else(|| icon(hwnd));
        if self::identity(hwnd).as_ref() != Some(&identity) {
            continue;
        }
        if image.is_some() {
            state.icons.insert(key, image.clone());
        }
        let id = issued_id(&mut state, &identity);
        let title = identity.title.chars().take(256).collect::<String>();
        entries.insert(id.clone(), identity);
        items.push(json!({"id":id,"title":title,"icon":image,"active":hwnd == foreground}));
    }
    state
        .icons
        .retain(|key, _| entries.values().any(|v| (v.hwnd, v.pid, v.created) == *key));
    state.entries = entries;
    Ok(
        json!({"status":if items.is_empty() {"empty"} else {"live"},"data":{"items":items},"updatedAt":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64}),
    )
}

pub fn activate(id: &str) -> Result<(), String> {
    let target = state()
        .lock()
        .map_err(|_| "Running applications unavailable")?
        .entries
        .get(id)
        .cloned()
        .ok_or("Window expired; try again")?;
    let hwnd = HWND(target.hwnd as *mut _);
    if identity(hwnd) != Some(target) {
        return Err("Window expired; try again".into());
    }
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if !SetForegroundWindow(hwnd).as_bool() {
            return Err("Windows could not activate this window".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_window_ids() {
        assert!(activate("running-not-issued").is_err());
    }
    #[test]
    fn excludes_own_process_and_invalid_window() {
        assert!(identity(HWND::default()).is_none());
    }
    #[test]
    fn stale_issued_window_is_rejected() {
        let target = Identity {
            hwnd: 0,
            pid: 1,
            created: 1,
            title: "Fixture".into(),
            class: "Fixture".into(),
        };
        state()
            .lock()
            .unwrap()
            .entries
            .insert("running-stale".into(), target);
        assert!(activate("running-stale").is_err());
    }
    #[test]
    fn reused_window_with_changed_identity_is_distinct() {
        let target = Identity {
            hwnd: 10,
            pid: 2,
            created: 3,
            title: "Old".into(),
            class: "A".into(),
        };
        let mut replacement = target.clone();
        replacement.title = "New".into();
        assert_ne!(target, replacement);
        replacement = target.clone();
        replacement.created += 1;
        assert_ne!(target, replacement);
    }
    #[test]
    fn stable_identity_keeps_id_across_refresh() {
        let mut state = State::default();
        let target = Identity {
            hwnd: 10,
            pid: 2,
            created: 3,
            title: "Fixture".into(),
            class: "A".into(),
        };
        let first = issued_id(&mut state, &target);
        state.entries.insert(first.clone(), target.clone());
        assert_eq!(issued_id(&mut state, &target), first);
        let mut replacement = target;
        replacement.created += 1;
        assert_ne!(issued_id(&mut state, &replacement), first);
    }
    #[test]
    fn renders_windows_shared_icon_as_png() {
        let icon = unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap();
        let png = render_icon(icon).unwrap();
        assert!(png.starts_with("data:image/png;base64,"));
        let side = icon_size(icon).unwrap();
        assert_eq!(png_dimensions(&png), (side, side));
    }
    fn png_dimensions(data: &str) -> (u32, u32) {
        let bytes = STANDARD
            .decode(data.trim_start_matches("data:image/png;base64,"))
            .unwrap();
        assert_eq!(&bytes[12..16], b"IHDR");
        (
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        )
    }
    #[test]
    fn extracts_high_resolution_executable_icon() {
        let path = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("notepad.exe");
        let wide: Vec<u16> = path
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let (width, height) = png_dimensions(&executable_icon(&wide).unwrap());
        assert!(
            width >= 48 && height >= 48,
            "expected at least 48x48, got {width}x{height}"
        );
    }
    #[test]
    fn rejects_missing_executable_path() {
        let wide: Vec<u16> = "C:\\protondock-missing-icon.exe\0".encode_utf16().collect();
        assert!(executable_icon(&wide).is_none());
    }
    #[test]
    fn prefers_executable_icon_below_64px() {
        let wide: Vec<u16> = "C:\\Apps\\tool.exe\0".encode_utf16().collect();
        assert!(prefer_executable_icon(Some(32), &wide));
        assert!(prefer_executable_icon(None, &wide));
    }
    #[test]
    fn keeps_window_icon_at_64px_and_above() {
        let wide: Vec<u16> = "C:\\Apps\\tool.exe\0".encode_utf16().collect();
        assert!(!prefer_executable_icon(Some(64), &wide));
        assert!(!prefer_executable_icon(Some(256), &wide));
    }
    #[test]
    fn never_prefers_application_frame_host_icon() {
        let wide: Vec<u16> = "C:\\Windows\\System32\\ApplicationFrameHost.exe\0"
            .encode_utf16()
            .collect();
        assert!(!prefer_executable_icon(Some(16), &wide));
        let lower: Vec<u16> = "C:\\Windows\\System32\\applicationframehost.exe\0"
            .encode_utf16()
            .collect();
        assert!(!prefer_executable_icon(Some(16), &lower));
    }
}
