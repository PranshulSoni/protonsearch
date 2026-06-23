#![windows_subsystem = "windows"]

mod launcher;
mod search;
mod indexer;
mod browser_indexer;
mod git_indexer;

use std::ptr::null_mut;
use search::{SearchEngine, SearchResult};
use windows::{
    core::{PCWSTR, Interface},
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::*,
            Input::KeyboardAndMouse::*,
            WindowsAndMessaging::*,
            Shell::*,
        },
    },
};

// ── Layout ────────────────────────────────────────────────────────────────────
const WIN_W: i32 = 720;
const SEARCH_H: i32 = 64;
const RESULT_H: i32 = 76;
const MAX_RESULTS: usize = 30;
const VISIBLE_RESULTS: usize = 5;
const PAD_L: i32 = 24;
const ICON_W: i32 = 36;

// ── Win32 IDs ─────────────────────────────────────────────────────────────────
const HOTKEY_ID: i32 = 1;
const TIMER_DEBOUNCE: usize = 1;
const TIMER_ANIM: usize = 2;
const WM_ICON_LOADED: u32 = WM_USER + 1;
const WM_ENGINE_READY: u32 = WM_USER + 2;

// ── Animation ─────────────────────────────────────────────────────────────────
const ANIM_TICK_MS: u32 = 16;
const APPEAR_FRAMES: f32 = 8.0;  // ~130ms
const HIDE_FRAMES: f32 = 8.0;    // ~130ms
const MAX_ALPHA: u8 = 255;       // 100% opacity for solid backdrop
const SLIDE_PX: i32 = 14;

// ── Pill Dimensions ───────────────────────────────────────────────────────────
const PILL_W: i32 = 150;
const PILL_H: i32 = 12;
const PILL_Y: i32 = 8;

// ── Colors (COLORREF = 0x00BBGGRR) ───────────────────────────────────────────
const BG: COLORREF        = COLORREF(0x00_24_21_21);
const BG_SEL: COLORREF    = COLORREF(0x00_3E_38_38);
const CLR_DIV: COLORREF   = COLORREF(0x00_40_3A_3A);
const CLR_WHITE: COLORREF = COLORREF(0x00_FF_FF_FF);
const CLR_GRAY: COLORREF  = COLORREF(0x00_9A_94_94);
const CLR_PH: COLORREF    = COLORREF(0x00_5A_55_55);
const CLR_BDGBG: COLORREF = COLORREF(0x00_50_48_48);
const CLR_BDGTX: COLORREF = COLORREF(0x00_C0_BB_BB);

// ── App state ─────────────────────────────────────────────────────────────────
struct State {
    engine: Option<SearchEngine>,
    query: String,
    results: Vec<SearchResult>,
    selected: usize,
    anim: Anim,
    cx: i32,
    cy: i32,
    font_q: HFONT,
    font_n: HFONT,
    font_c: HFONT,
    font_b: HFONT,
    icon_settings: HICON,
    icon_control_panel: HICON,
    icon_search: HICON,
    icon_web: HICON,
    icon_bookmark: HICON,
    icon_folder: HICON,
    icon_commit: HICON,
    icon_todo: HICON,
    icon_clipboard: HICON,
    text_selected: bool,
    scroll_offset: usize,
    last_mouse_x: i32,
    last_mouse_y: i32,
    app_icons: std::collections::HashMap<String, HICON>,
}

#[derive(PartialEq)]
enum Anim { Pill, Appearing(i32), Visible, Hiding(i32) }

#[derive(Clone, Copy)]
struct SendHwnd(HWND);
unsafe impl Send for SendHwnd {}
unsafe impl Sync for SendHwnd {}

impl State {
    fn win_h(&self) -> i32 {
        let n = self.results.len().min(VISIBLE_RESULTS) as i32;
        if n == 0 { SEARCH_H } else { SEARCH_H + 1 + n * RESULT_H }
    }
    fn result_rect(&self, i: usize) -> RECT {
        let y = SEARCH_H + 1 + i as i32 * RESULT_H;
        RECT { left: 0, top: y, right: WIN_W, bottom: y + RESULT_H }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────
fn main() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE);
    }

    unsafe { run(); }
}

unsafe fn run() {
    let hinst = GetModuleHandleW(PCWSTR::null()).unwrap();
    let face: Vec<u16> = "Segoe UI Variable\0".encode_utf16().collect();
    let fp = PCWSTR(face.as_ptr());

    // CreateFontW takes u32 for the font attribute params in windows 0.58.
    let mk_font = |h, w| CreateFontW(
        h, 0, 0, 0, w, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32, (DEFAULT_PITCH.0 | FF_SWISS.0) as u32, fp,
    );

    let sw = GetSystemMetrics(SM_CXSCREEN);
    let sh = GetSystemMetrics(SM_CYSCREEN);

    const SETTINGS_ICO: &[u8] = include_bytes!("../../assets/logo/settings.ico");
    const CONTROL_PANEL_ICO: &[u8] = include_bytes!("../../assets/logo/control_panel.ico");
    const SEARCH_ICO: &[u8] = include_bytes!("../../assets/logo/search.ico");
    const WEB_ICO: &[u8] = include_bytes!("../../assets/logo/web.ico");

    let icon_settings = load_icon_from_memory(SETTINGS_ICO, 32);
    let icon_control_panel = load_icon_from_memory(CONTROL_PANEL_ICO, 32);
    let icon_search = load_icon_from_memory(SEARCH_ICO, 24);
    let icon_web = load_icon_from_memory(WEB_ICO, 32);
    let icon_bookmark = load_icon_from_dll("shell32.dll", 43, 32);
    let icon_folder = load_icon_from_dll("shell32.dll", 3, 32);
    let icon_commit = load_icon_from_dll("shell32.dll", 22, 32);
    let icon_todo = load_icon_from_dll("shell32.dll", 270, 32);
    let icon_clipboard = load_icon_from_dll("shell32.dll", 260, 32);

    let state = Box::new(State {
        engine: None,
        query: String::new(),
        results: vec![],
        selected: 0,
        anim: Anim::Pill,
        cx: sw / 2,
        cy: sh / 3,
        font_q: mk_font(-19, 400),
        font_n: mk_font(-17, 600),
        font_c: mk_font(-13, 400),
        font_b: mk_font(-11, 600),
        icon_settings,
        icon_control_panel,
        icon_search,
        icon_web,
        icon_bookmark,
        icon_folder,
        icon_commit,
        icon_todo,
        icon_clipboard,
        text_selected: false,
        scroll_offset: 0,
        last_mouse_x: -1,
        last_mouse_y: -1,
        app_icons: std::collections::HashMap::new(),
    });

    let class: Vec<u16> = "opensearch-os\0".encode_utf16().collect();
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wnd_proc),
        hInstance: hinst.into(),
        hCursor: LoadCursorW(HINSTANCE(null_mut()), IDC_ARROW).unwrap(),
        hbrBackground: HBRUSH(null_mut()),
        lpszClassName: PCWSTR(class.as_ptr()),
        ..Default::default()
    };
    RegisterClassExW(&wc);

    let sw = GetSystemMetrics(SM_CXSCREEN);
    let pill_x = sw / 2 - PILL_W / 2;
    let pill_y = PILL_Y;

    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        PCWSTR(class.as_ptr()),
        PCWSTR::null(),
        WS_POPUP,
        pill_x, pill_y, PILL_W, PILL_H,
        HWND(null_mut()), HMENU(null_mut()), hinst,
        Some(Box::into_raw(state) as _),
    ).unwrap();

    let _ = unsafe { windows::Win32::System::DataExchange::AddClipboardFormatListener(hwnd) };

    SetLayeredWindowAttributes(hwnd, COLORREF(0), 240, LWA_ALPHA).unwrap();
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

    // DWM rounded corners (Windows 11)
    let corner = DWMWCP_ROUND;
    let _ = DwmSetWindowAttribute(
        hwnd, DWMWA_WINDOW_CORNER_PREFERENCE,
        &corner as *const _ as _, 4,
    );

    // Disable DWM Acrylic backdrop (make it solid)
    let backdrop = 1i32; // DWMSBT_NONE (None)
    let _ = DwmSetWindowAttribute(
        hwnd, DWMWA_SYSTEMBACKDROP_TYPE,
        &backdrop as *const _ as _, 4,
    );

    // Load the search engine in a background thread so the window appears instantly.
    let hwnd_usize = hwnd.0 as usize;
    std::thread::spawn(move || {
        let db_path = match std::env::var("APPDATA") {
            Ok(d) => {
                let path = std::path::PathBuf::from(d).join("opensearch-os");
                let _ = std::fs::create_dir_all(&path);
                path.join("file_index.db")
            }
            Err(_) => std::path::PathBuf::from("file_index.db"),
        };
        indexer::start_indexer(db_path.clone());
        browser_indexer::start_browser_indexer(db_path.clone());
        git_indexer::start_git_indexer(db_path.clone());

        let model_path = std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|d| d.join("model_int8.onnx")));
        let db_path_for_engine = db_path.clone();
        let result = match model_path {
            Some(p) => SearchEngine::new(&p, db_path_for_engine),
            None => Err(anyhow::anyhow!("cannot locate exe directory")),
        };
        let hwnd_bg = HWND(hwnd_usize as *mut std::ffi::c_void);
        unsafe {
            match result {
                Ok(engine) => {
                    // Import Windows Clipboard History in background
                    let db_path_clone = db_path.clone();
                    std::thread::spawn(move || {
                        let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED) };
                        unsafe { import_windows_clipboard_history(&db_path_clone); }
                        unsafe { windows::Win32::System::Com::CoUninitialize(); }
                    });

                    let ptr = Box::into_raw(Box::new(engine)) as isize;
                    let _ = PostMessageW(hwnd_bg, WM_ENGINE_READY, WPARAM(1), LPARAM(ptr));
                }
                Err(e) => {
                    let msg = Box::into_raw(Box::new(e.to_string())) as isize;
                    let _ = PostMessageW(hwnd_bg, WM_ENGINE_READY, WPARAM(0), LPARAM(msg));
                }
            }
        }
    });

    // Win+Space is reserved by Windows IME; Alt+Space is the conventional launcher hotkey.
    if RegisterHotKey(hwnd, HOTKEY_ID, MOD_ALT | MOD_NOREPEAT, VK_SPACE.0 as u32).is_err() {
        use windows::Win32::UI::WindowsAndMessaging::MessageBoxW;
        let msg: Vec<u16> = "Failed to register Alt+Space hotkey.\nAnother app may be using it.\0"
            .encode_utf16().collect();
        let title: Vec<u16> = "OpenSearch OS\0".encode_utf16().collect();
        MessageBoxW(HWND(null_mut()), PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
        return;
    }

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).as_bool() {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    UnregisterHotKey(hwnd, HOTKEY_ID);
}

// ── WndProc ───────────────────────────────────────────────────────────────────
unsafe extern "system" fn wnd_proc(
    hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM,
) -> LRESULT {
    let sp = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;

    match msg {
        WM_CREATE => {
            let cs = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as _);
            LRESULT(0)
        }

        WM_HOTKEY if wp.0 as i32 == HOTKEY_ID => {
            let s = &mut *sp;
            match s.anim {
                Anim::Pill | Anim::Hiding(_) => do_show(hwnd, s),
                _ => start_hide(hwnd, s),
            }
            LRESULT(0)
        }

        WM_KILLFOCUS => {
            if !sp.is_null() {
                let s = &mut *sp;
                if !matches!(s.anim, Anim::Pill | Anim::Hiding(_)) {
                    start_hide(hwnd, s);
                }
            }
            LRESULT(0)
        }

        WM_ICON_LOADED => {
            if sp.is_null() {
                unsafe {
                    let _ = Box::from_raw(lp.0 as *mut String);
                }
                return LRESULT(0);
            }
            let s = &mut *sp;
            let hicon = HICON(wp.0 as *mut std::ffi::c_void);
            let key_box = unsafe { Box::from_raw(lp.0 as *mut String) };
            let key = *key_box;
            
            // Insert the loaded HICON into the map
            if let Some(old_hicon) = s.app_icons.insert(key, hicon) {
                if !old_hicon.0.is_null() && old_hicon != hicon {
                    unsafe { let _ = DestroyIcon(old_hicon); }
                }
            }
            
            unsafe { let _ = InvalidateRect(hwnd, None, FALSE); }
            LRESULT(0)
        }

        WM_CLIPBOARDUPDATE => {
            if sp.is_null() { return LRESULT(0); }
            let s = &*sp;

            // Check if foreground window is the launcher itself to prevent duplicates
            let hwnd_fg = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
            if hwnd_fg == hwnd {
                return LRESULT(0);
            }

            let db_path = s.engine.as_ref().map(|e| e.db_path());
            if let Some(db_path) = db_path {
                let app_name = unsafe { get_active_app_name() };

                // Try text format
                if let Some(text) = unsafe { paste_from_clipboard(hwnd) } {
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        let db_path_clone = db_path.clone();
                        let app_name_clone = app_name.clone();
                        std::thread::spawn(move || {
                            if let Ok(conn) = rusqlite::Connection::open(&db_path_clone) {
                                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs() as i64;
                                let _ = conn.execute(
                                    "INSERT OR REPLACE INTO clipboard_history (content, timestamp, source_app, is_image) VALUES (?, ?, ?, 0);",
                                    rusqlite::params![trimmed, now, app_name_clone],
                                );
                                let _ = conn.execute(
                                    "DELETE FROM clipboard_history WHERE id NOT IN (SELECT id FROM clipboard_history ORDER BY timestamp DESC LIMIT 500);",
                                    [],
                                );
                            }
                        });
                    }
                } else {
                    // Try image format (CF_BITMAP)
                    unsafe {
                        if let Some((buf, bih)) = capture_clipboard_image_data(hwnd) {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;
                            let filename = format!("image_{}.bmp", now);
                            let img_dir = db_path.parent().unwrap().join("clipboard_images");
                            let _ = std::fs::create_dir_all(&img_dir);
                            let img_path = img_dir.join(&filename);
                            let img_path_str = img_path.to_string_lossy().to_string();
                            
                            let db_path_clone = db_path.clone();
                            std::thread::spawn(move || {
                                if write_bmp_file(&img_path, &buf, bih).is_ok() {
                                    if let Ok(conn) = rusqlite::Connection::open(&db_path_clone) {
                                        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                                        let _ = conn.execute(
                                            "INSERT OR REPLACE INTO clipboard_history (content, timestamp, source_app, is_image) VALUES (?, ?, ?, 1);",
                                            rusqlite::params![img_path_str, now, app_name],
                                        );
                                        let _ = conn.execute(
                                            "DELETE FROM clipboard_history WHERE id NOT IN (SELECT id FROM clipboard_history ORDER BY timestamp DESC LIMIT 500);",
                                            [],
                                        );
                                    }
                                }
                            });
                        }
                    }
                }
            }
            LRESULT(0)
        }

        WM_ENGINE_READY => {
            if wp.0 == 1 {
                let engine = *Box::from_raw(lp.0 as *mut SearchEngine);
                if !sp.is_null() {
                    let s = &mut *sp;
                    s.engine = Some(engine);
                    s.results = s.engine.as_mut().unwrap().search(&s.query, MAX_RESULTS);
                    trigger_icon_loading(hwnd, s);
                    InvalidateRect(hwnd, None, FALSE);
                }
            } else {
                let err = *Box::from_raw(lp.0 as *mut String);
                let mut msg: Vec<u16> = format!("Engine error:\n{err}\0").encode_utf16().collect();
                let mut title: Vec<u16> = "OpenSearch OS\0".encode_utf16().collect();
                MessageBoxW(HWND(null_mut()), PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()), MB_ICONERROR | MB_OK);
                let _ = (&mut msg, &mut title);
            }
            LRESULT(0)
        }

        WM_TIMER => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            match wp.0 {
                TIMER_DEBOUNCE => {
                    KillTimer(hwnd, TIMER_DEBOUNCE);
                    s.results = if let Some(ref mut engine) = s.engine {
                        engine.search(&s.query, MAX_RESULTS)
                    } else {
                        vec![]
                    };
                    trigger_icon_loading(hwnd, s);
                    s.selected = 0;
                    s.scroll_offset = 0;
                    reposition_animated(hwnd, s, 1.0);
                    InvalidateRect(hwnd, None, FALSE);
                }
                TIMER_ANIM => tick(hwnd, s),
                _ => {}
            }
            LRESULT(0)
        }

        WM_CHAR => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            if let Some(c) = char::from_u32(wp.0 as u32) {
                if !c.is_control() {
                    if s.text_selected {
                        s.query.clear();
                        s.text_selected = false;
                    }
                    s.query.push(c);
                    s.selected = 0;
                    kick_debounce(hwnd);
                    InvalidateRect(hwnd, None, FALSE);
                }
            }
            LRESULT(0)
        }

        WM_KEYDOWN => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            let vk = VIRTUAL_KEY(wp.0 as u16);
            
            // Check if Ctrl is pressed
            let ctrl_down = (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            
            if ctrl_down {
                match vk.0 as u32 {
                    0x41 => { // Ctrl + A (Select All)
                        if !s.query.is_empty() {
                            s.text_selected = true;
                            InvalidateRect(hwnd, None, FALSE);
                        }
                        return LRESULT(0);
                    }
                    0x43 => { // Ctrl + C (Copy)
                        if !s.query.is_empty() {
                            copy_to_clipboard(hwnd, &s.query);
                        }
                        return LRESULT(0);
                    }
                    0x56 => { // Ctrl + V (Paste)
                        if let Some(text) = paste_from_clipboard(hwnd) {
                            let clean_text: String = text.chars().filter(|c| !c.is_control()).collect();
                            if s.text_selected {
                                s.query = clean_text;
                                s.text_selected = false;
                            } else {
                                s.query.push_str(&clean_text);
                            }
                            s.selected = 0;
                            kick_debounce(hwnd);
                            InvalidateRect(hwnd, None, FALSE);
                        }
                        return LRESULT(0);
                    }
                    _ => {}
                }
            }

            match vk {
                VK_ESCAPE => {
                    if s.text_selected {
                        s.text_selected = false;
                        InvalidateRect(hwnd, None, FALSE);
                    } else {
                        start_hide(hwnd, s);
                    }
                }
                VK_BACK => {
                    if s.text_selected {
                        s.query.clear();
                        s.text_selected = false;
                    } else {
                        s.query.pop();
                    }
                    s.selected = 0;
                    kick_debounce(hwnd);
                    InvalidateRect(hwnd, None, FALSE);
                }
                VK_RETURN => {
                    let is_shift = (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
                    let is_ctrl = (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
                    if (is_shift || is_ctrl) && !s.query.is_empty() {
                        let encoded = search::url_encode(&s.query);
                        let url = format!("https://www.google.com/search?q={}", encoded);
                        launcher::launch(&url);
                        do_hide(hwnd, s);
                    } else if let Some(r) = s.results.get(s.selected) {
                        let cmd = r.entry.launch_command.clone();
                        let is_action_folder = r.entry.source == "FOLDER" && (
                            cmd == "bookmarks:" || cmd == "history:" || cmd == "commits:" ||
                            cmd == "todos:" || cmd == "clip:" || cmd == "file:" || cmd == "code:"
                        );
                        if is_action_folder {
                            s.query = cmd;
                            s.selected = 0;
                            s.scroll_offset = 0;
                            s.text_selected = false;
                            s.results = if let Some(ref mut engine) = s.engine {
                                engine.search(&s.query, MAX_RESULTS)
                            } else {
                                vec![]
                            };
                            trigger_icon_loading(hwnd, s);
                            reposition_animated(hwnd, s, 1.0);
                            InvalidateRect(hwnd, None, FALSE);
                        } else {
                            if let Some(text) = cmd.strip_prefix("copy:") {
                                copy_to_clipboard(hwnd, text);
                            } else if let Some(path) = cmd.strip_prefix("copy_image:") {
                                copy_image_to_clipboard(hwnd, path);
                            } else {
                                launcher::launch(&cmd);
                            }
                            do_hide(hwnd, s);
                        }
                    }
                }
                VK_DOWN => {
                    if !s.results.is_empty() {
                        s.selected = (s.selected + 1).min(s.results.len() - 1);
                        if s.selected >= s.scroll_offset + VISIBLE_RESULTS {
                            s.scroll_offset = s.selected - (VISIBLE_RESULTS - 1);
                        }
                        InvalidateRect(hwnd, None, FALSE);
                    }
                }
                VK_UP => {
                    if s.selected > 0 {
                        s.selected -= 1;
                        if s.selected < s.scroll_offset {
                            s.scroll_offset = s.selected;
                        }
                        InvalidateRect(hwnd, None, FALSE);
                    }
                }
                _ => return DefWindowProcW(hwnd, msg, wp, lp),
            }
            LRESULT(0)
        }

        WM_MOUSEWHEEL => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            if !s.results.is_empty() {
                let delta = (wp.0 >> 16) as i16;
                if delta > 0 {
                    // Scroll up
                    if s.scroll_offset > 0 {
                        s.scroll_offset -= 1;
                        if s.selected >= s.scroll_offset + VISIBLE_RESULTS {
                            s.selected = s.scroll_offset + VISIBLE_RESULTS - 1;
                        }
                        InvalidateRect(hwnd, None, FALSE);
                    }
                } else {
                    // Scroll down
                    if s.scroll_offset + VISIBLE_RESULTS < s.results.len() {
                        s.scroll_offset += 1;
                        if s.selected < s.scroll_offset {
                            s.selected = s.scroll_offset;
                        }
                        InvalidateRect(hwnd, None, FALSE);
                    }
                }
            }
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            if s.anim == Anim::Pill {
                do_show(hwnd, s);
                return LRESULT(0);
            }
            let my = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            let n = s.results.len().min(VISIBLE_RESULTS);
            for i in 0..n {
                let r = s.result_rect(i);
                if my >= r.top && my < r.bottom {
                    let actual_idx = s.scroll_offset + i;
                    let cmd = s.results[actual_idx].entry.launch_command.clone();
                    let is_action_folder = s.results[actual_idx].entry.source == "FOLDER" && (
                        cmd == "bookmarks:" || cmd == "history:" || cmd == "commits:" ||
                        cmd == "todos:" || cmd == "clip:" || cmd == "file:" || cmd == "code:"
                    );
                    if is_action_folder {
                        s.query = cmd;
                        s.selected = 0;
                        s.scroll_offset = 0;
                        s.text_selected = false;
                        s.results = if let Some(ref mut engine) = s.engine {
                            engine.search(&s.query, MAX_RESULTS)
                        } else {
                            vec![]
                        };
                        trigger_icon_loading(hwnd, s);
                        reposition_animated(hwnd, s, 1.0);
                        InvalidateRect(hwnd, None, FALSE);
                    } else {
                        if let Some(text) = cmd.strip_prefix("copy:") {
                            copy_to_clipboard(hwnd, text);
                        } else if let Some(path) = cmd.strip_prefix("copy_image:") {
                            copy_image_to_clipboard(hwnd, path);
                        } else {
                            launcher::launch(&cmd);
                        }
                        do_hide(hwnd, s);
                    }
                    break;
                }
            }
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            let _mx = (lp.0 & 0xFFFF) as i16 as i32;
            let my = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            
            if pt.x != s.last_mouse_x || pt.y != s.last_mouse_y {
                s.last_mouse_x = pt.x;
                s.last_mouse_y = pt.y;
                
                let n = s.results.len().min(VISIBLE_RESULTS);
                for i in 0..n {
                    let r = s.result_rect(i);
                    if my >= r.top && my < r.bottom {
                        let actual_idx = s.scroll_offset + i;
                        if s.selected != actual_idx {
                            s.selected = actual_idx;
                            let _ = InvalidateRect(hwnd, None, FALSE);
                        }
                        break;
                    }
                }
            }
            LRESULT(0)
        }

        WM_ERASEBKGND => LRESULT(1),

        WM_PAINT => {
            if sp.is_null() { return DefWindowProcW(hwnd, msg, wp, lp); }
            paint(hwnd, &*sp);
            LRESULT(0)
        }

        WM_DESTROY => {
            let _ = unsafe { windows::Win32::System::DataExchange::RemoveClipboardFormatListener(hwnd) };
            if !sp.is_null() {
                let s = Box::from_raw(sp);
                if !s.icon_clipboard.0.is_null() { let _ = DestroyIcon(s.icon_clipboard); }
                DeleteObject(s.font_q);
                DeleteObject(s.font_n);
                DeleteObject(s.font_c);
                DeleteObject(s.font_b);
                if !s.icon_settings.0.is_null() { let _ = DestroyIcon(s.icon_settings); }
                if !s.icon_control_panel.0.is_null() { let _ = DestroyIcon(s.icon_control_panel); }
                if !s.icon_search.0.is_null() { let _ = DestroyIcon(s.icon_search); }
                if !s.icon_web.0.is_null() { let _ = DestroyIcon(s.icon_web); }
                if !s.icon_bookmark.0.is_null() { let _ = DestroyIcon(s.icon_bookmark); }
                if !s.icon_folder.0.is_null() { let _ = DestroyIcon(s.icon_folder); }
                if !s.icon_commit.0.is_null() { let _ = DestroyIcon(s.icon_commit); }
                if !s.icon_todo.0.is_null() { let _ = DestroyIcon(s.icon_todo); }
                for &hicon in s.app_icons.values() {
                    if !hicon.0.is_null() {
                        let _ = DestroyIcon(hicon);
                    }
                }
            }
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ── Window lifecycle ──────────────────────────────────────────────────────────
unsafe fn do_show(hwnd: HWND, s: &mut State) {
    s.query.clear();
    s.selected = 0;
    s.scroll_offset = 0;
    s.results = if let Some(ref mut engine) = s.engine {
        engine.search("", MAX_RESULTS)
    } else {
        vec![]
    };
    trigger_icon_loading(hwnd, s);
    
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    s.last_mouse_x = pt.x;
    s.last_mouse_y = pt.y;
    
    s.anim = Anim::Appearing(0);
    reposition_animated(hwnd, s, 0.0);
    let _ = SetForegroundWindow(hwnd);
    let _ = SetFocus(hwnd);
    let _ = SetTimer(hwnd, TIMER_ANIM, ANIM_TICK_MS, None);
    let _ = InvalidateRect(hwnd, None, FALSE);
}

unsafe fn do_hide(hwnd: HWND, s: &mut State) {
    KillTimer(hwnd, TIMER_ANIM);
    KillTimer(hwnd, TIMER_DEBOUNCE);
    
    // Hide the window so Windows naturally restores focus to the previously active window
    let _ = ShowWindow(hwnd, SW_HIDE);
    
    s.anim = Anim::Pill;
    
    // Reposition to pill size and position at top center
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let pill_x = sw / 2 - PILL_W / 2;
    let pill_y = PILL_Y;
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, pill_x, pill_y, PILL_W, PILL_H, SWP_NOACTIVATE);
    
    // Reset to pill opacity
    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 240, LWA_ALPHA);
    
    // Show window as pill without focus
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
}

unsafe fn start_hide(hwnd: HWND, s: &mut State) {
    s.anim = Anim::Hiding(0);
    let _ = SetTimer(hwnd, TIMER_ANIM, ANIM_TICK_MS, None);
}

unsafe fn reposition_animated(hwnd: HWND, s: &State, t: f32) {
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let pill_x = sw / 2 - PILL_W / 2;
    let pill_y = PILL_Y;
    let pill_w = PILL_W;
    let pill_h = PILL_H;

    let search_w = WIN_W;
    let search_h = s.win_h();
    let search_x = s.cx - WIN_W / 2;
    let search_y = s.cy - search_h / 2;

    let x = (pill_x as f32 + (search_x - pill_x) as f32 * t) as i32;
    let y = (pill_y as f32 + (search_y - pill_y) as f32 * t) as i32;
    let w = (pill_w as f32 + (search_w - pill_w) as f32 * t) as i32;
    let h = (pill_h as f32 + (search_h - pill_h) as f32 * t) as i32;

    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE | SWP_NOCOPYBITS);
}

unsafe fn kick_debounce(hwnd: HWND) {
    KillTimer(hwnd, TIMER_DEBOUNCE);
    let _ = SetTimer(hwnd, TIMER_DEBOUNCE, 120, None);
}

// ── Animation tick ────────────────────────────────────────────────────────────
unsafe fn tick(hwnd: HWND, s: &mut State) {
    match &mut s.anim {
        Anim::Appearing(f) => {
            *f += 1;
            let progress = (*f as f32 / APPEAR_FRAMES).min(1.0);
            let t = ease_out(progress);
            let alpha = (240.0 + (255.0 - 240.0) * t) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
            reposition_animated(hwnd, s, t);
            if progress >= 1.0 {
                s.anim = Anim::Visible;
                KillTimer(hwnd, TIMER_ANIM);
                let _ = SetForegroundWindow(hwnd);
                let _ = SetFocus(hwnd);
            }
        }
        Anim::Hiding(f) => {
            *f += 1;
            let progress = (*f as f32 / HIDE_FRAMES).min(1.0);
            let t = 1.0 - ease_in(progress);
            let alpha = (240.0 + (255.0 - 240.0) * t) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
            reposition_animated(hwnd, s, t);
            if progress >= 1.0 {
                do_hide(hwnd, s);
            }
        }
        _ => { KillTimer(hwnd, TIMER_ANIM); }
    }
    let _ = InvalidateRect(hwnd, None, FALSE);
}

fn ease_out(t: f32) -> f32 { 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3) }
fn ease_in(t: f32) -> f32 { t.clamp(0.0, 1.0).powi(3) }

unsafe fn fill_rounded(hdc: HDC, x: i32, y: i32, w: i32, h: i32, r: i32, c: COLORREF) {
    let br = CreateSolidBrush(c);
    let old_brush = SelectObject(hdc, br);
    let pen = CreatePen(PS_NULL, 0, COLORREF(0));
    let old_pen = SelectObject(hdc, pen);
    
    let _ = RoundRect(hdc, x, y, x + w + 1, y + h + 1, r, r);
    
    let _ = SelectObject(hdc, old_pen);
    let _ = DeleteObject(pen);
    let _ = SelectObject(hdc, old_brush);
    let _ = DeleteObject(br);
}

// ── Painting ──────────────────────────────────────────────────────────────────
unsafe fn resolve_lnk(path: &str) -> Option<String> {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, IPersistFile, STGM_READ};
    use windows::Win32::UI::Shell::{ShellLink, IShellLinkW, SLGP_UNCPRIORITY};

    let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
    let persist: IPersistFile = link.cast().ok()?;
    let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    persist.Load(PCWSTR(path_wide.as_ptr()), STGM_READ).ok()?;
    let mut buffer = [0u16; 260];
    link.GetPath(&mut buffer, std::ptr::null_mut(), SLGP_UNCPRIORITY.0 as u32).ok()?;
    let target = String::from_utf16_lossy(&buffer);
    let trimmed = target.trim_matches('\0').trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

unsafe fn get_app_icon(path: &str) -> HICON {
    let mut hicon = HICON(null_mut());
    let mut log_msg = format!("get_app_icon input: {}\n", path);

    // Resolve shortcut if it ends in .lnk to bypass shortcut arrow overlay
    let mut target_path = path.to_string();
    if target_path.to_lowercase().ends_with(".lnk") {
        if let Some(resolved) = resolve_lnk(&target_path) {
            target_path = resolved;
            log_msg.push_str(&format!("  Resolved shortcut to: {}\n", target_path));
        }
    }

    // Format virtual paths properly for SHCreateItemFromParsingName
    let parsing_path = if target_path.starts_with("shell:AppsFolder\\") {
        target_path.clone()
    } else if !target_path.contains(":\\") && !target_path.starts_with("\\\\") {
        format!("shell:AppsFolder\\{}", target_path)
    } else {
        target_path.clone()
    };
    log_msg.push_str(&format!("  Parsing path: {}\n", parsing_path));

    let path_wide: Vec<u16> = parsing_path.encode_utf16().chain(std::iter::once(0)).collect();

    // Try parsing as a shell item to get icon from virtual Applications folder or normal file
    let shell_item: Option<windows::Win32::UI::Shell::IShellItem> = windows::Win32::UI::Shell::SHCreateItemFromParsingName(
        PCWSTR(path_wide.as_ptr()),
        None,
    ).ok();

    if let Some(item) = &shell_item {
        log_msg.push_str("  SHCreateItemFromParsingName: SUCCESS\n");
        
        // 1. Try modern IShellItemImageFactory first
        let factory: Option<windows::Win32::UI::Shell::IShellItemImageFactory> = item.cast().ok();
        if let Some(f) = factory {
            let res = f.GetImage(
                windows::Win32::Foundation::SIZE { cx: 32, cy: 32 },
                windows::Win32::UI::Shell::SIIGBF_ICONONLY,
            );
            match res {
                Ok(hbitmap) => {
                    let hbm_mask = windows::Win32::Graphics::Gdi::CreateBitmap(32, 32, 1, 1, None);
                    if !hbm_mask.is_invalid() {
                        let mut ii = windows::Win32::UI::WindowsAndMessaging::ICONINFO {
                            fIcon: windows::Win32::Foundation::TRUE,
                            xHotspot: 0,
                            yHotspot: 0,
                            hbmMask: hbm_mask,
                            hbmColor: hbitmap,
                        };
                        if let Ok(hi) = windows::Win32::UI::WindowsAndMessaging::CreateIconIndirect(&mut ii) {
                            hicon = hi;
                            log_msg.push_str(&format!("  IShellItemImageFactory SUCCESS: {:?}\n", hicon.0));
                        }
                        let _ = windows::Win32::Graphics::Gdi::DeleteObject(hbm_mask);
                    }
                    let _ = windows::Win32::Graphics::Gdi::DeleteObject(hbitmap);
                }
                Err(e) => {
                    log_msg.push_str(&format!("  IShellItemImageFactory GetImage FAILED: {:?}\n", e));
                }
            }
        } else {
            log_msg.push_str("  IShellItemImageFactory cast FAILED\n");
        }

        // 2. Fall back to legacy SHGetFileInfoW with PIDL
        if hicon.0.is_null() {
            if let Ok(pidl) = windows::Win32::UI::Shell::SHGetIDListFromObject(item) {
                log_msg.push_str("  SHGetIDListFromObject: SUCCESS\n");
                let mut shfi = windows::Win32::UI::Shell::SHFILEINFOW::default();
                let flags = windows::Win32::UI::Shell::SHGFI_ICON 
                    | windows::Win32::UI::Shell::SHGFI_LARGEICON 
                    | windows::Win32::UI::Shell::SHGFI_PIDL;
                let res = windows::Win32::UI::Shell::SHGetFileInfoW(
                    PCWSTR(pidl as *const u16),
                    windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                    Some(&mut shfi),
                    std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
                    flags,
                );
                hicon = shfi.hIcon;
                log_msg.push_str(&format!("  SHGetFileInfoW res: {}, hicon: {:?}\n", res, hicon.0));
                windows::Win32::UI::Shell::ILFree(Some(pidl));
            } else {
                log_msg.push_str("  SHGetIDListFromObject: FAILED\n");
            }
        }
    } else {
        log_msg.push_str("  SHCreateItemFromParsingName: FAILED\n");
    }

    // Fallback directly using path
    if hicon.0.is_null() {
        log_msg.push_str("  Entering fallback\n");
        let mut shfi = windows::Win32::UI::Shell::SHFILEINFOW::default();
        let flags = windows::Win32::UI::Shell::SHGFI_ICON | windows::Win32::UI::Shell::SHGFI_LARGEICON;
        let fallback_wide: Vec<u16> = target_path.encode_utf16().chain(std::iter::once(0)).collect();
        let res = windows::Win32::UI::Shell::SHGetFileInfoW(
            PCWSTR(fallback_wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
            flags,
        );
        hicon = shfi.hIcon;
        log_msg.push_str(&format!("  Fallback SHGetFileInfoW res: {}, hicon: {:?}\n", res, hicon.0));
    }

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("C:\\Users\\Pranshul Soni\\.gemini\\antigravity\\brain\\63a8f76b-b4b2-431b-9719-18e67f5a0652\\scratch\\icon_log.txt")
    {
        use std::io::Write;
        let _ = write!(file, "{}\n", log_msg);
    }

    hicon
}

unsafe fn get_file_icon(path: &str) -> HICON {
    let mut shfi = windows::Win32::UI::Shell::SHFILEINFOW::default();
    let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let flags = windows::Win32::UI::Shell::SHGFI_ICON | windows::Win32::UI::Shell::SHGFI_LARGEICON;
    let res = windows::Win32::UI::Shell::SHGetFileInfoW(
        PCWSTR(path_wide.as_ptr()),
        windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
        Some(&mut shfi),
        std::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
        flags,
    );
    if res != 0 { shfi.hIcon } else { HICON(null_mut()) }
}

unsafe fn trigger_icon_loading(hwnd: HWND, s: &mut State) {
    for res in &s.results {
        let (source, key) = (res.entry.source.as_str(), res.entry.launch_command.clone());
        // For FOLDER source: only load icon if it's a real filesystem path (not virtual folders like bookmarks:)
        let is_real_folder = source == "FOLDER" && !key.ends_with(':') && std::path::Path::new(&key).exists();
        let needs_icon = (source == "app" || source == "RECENT" || source == "FILE" || source == "CODE" || is_real_folder)
            && !s.app_icons.contains_key(&key);
        if needs_icon {
            // Placeholder so we don't spawn multiple threads for same path
            s.app_icons.insert(key.clone(), HICON(std::ptr::null_mut()));
            let is_recent = source == "RECENT" || source == "FILE" || source == "CODE" || is_real_folder;
            let hwnd_clone = SendHwnd(hwnd);
            std::thread::spawn(move || {
                let hwnd_raw = hwnd_clone;
                unsafe {
                    let _ = windows::Win32::System::Com::CoInitializeEx(
                        None,
                        windows::Win32::System::Com::COINIT_APARTMENTTHREADED | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE
                    );
                    let hicon = if is_recent {
                        get_file_icon(&key)
                    } else {
                        get_app_icon(&key)
                    };
                    if !hicon.0.is_null() {
                        let key_ptr = Box::into_raw(Box::new(key));
                        if PostMessageW(
                            hwnd_raw.0,
                            WM_ICON_LOADED,
                            WPARAM(hicon.0 as usize),
                            LPARAM(key_ptr as isize),
                        ).is_err() {
                            let _ = Box::from_raw(key_ptr);
                            let _ = DestroyIcon(hicon);
                        }
                    }
                }
            });
        }
    }
}

unsafe fn paint(hwnd: HWND, s: &State) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    let win_w = rc.right - rc.left;
    let win_h = rc.bottom - rc.top;

    // Double-buffer
    let mdc = CreateCompatibleDC(hdc);
    let bmp = CreateCompatibleBitmap(hdc, win_w, win_h);
    let old = SelectObject(mdc, bmp);

    if win_h < SEARCH_H {
        // Draw simple rounded notch/pill for Dynamic Island state
        fill_rounded(mdc, 0, 0, win_w, win_h, 8, COLORREF(0x00_15_12_12));
        let _ = BitBlt(hdc, 0, 0, win_w, win_h, mdc, 0, 0, SRCCOPY);
        let _ = SelectObject(mdc, old);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mdc);
        EndPaint(hwnd, &ps);
        return;
    }

    // Fill background / Draw Glowing Border
    let has_results = s.results.len().min(MAX_RESULTS) > 0;
    if has_results {
        // Draw vibrant gradient border
        let vertices = [
            TRIVERTEX {
                x: 0,
                y: 0,
                Red: 0x0000,
                Green: 0xb400,
                Blue: 0xdb00,
                Alpha: 0x0000,
            },
            TRIVERTEX {
                x: win_w,
                y: win_h,
                Red: 0x7f00,
                Green: 0x0000,
                Blue: 0xff00,
                Alpha: 0x0000,
            },
        ];
        let g_rect = [GRADIENT_RECT {
            UpperLeft: 0,
            LowerRight: 1,
        }];
        let _ = GradientFill(mdc, &vertices, g_rect.as_ptr() as *const _, 1, GRADIENT_FILL(0)); // Horizontal gradient
        fill(mdc, 1, 1, win_w - 2, win_h - 2, BG);
    } else {
        // Draw subtle solid gray border
        fill(mdc, 0, 0, win_w, win_h, CLR_DIV);
        fill(mdc, 1, 1, win_w - 2, win_h - 2, BG);
    }

    // ── Search row ────────────────────────────────────────────────────────
    SetBkMode(mdc, TRANSPARENT);

    // Draw Search Icon
    if !s.icon_search.0.is_null() {
        let icon_y = (SEARCH_H - 24) / 2;
        let _ = DrawIconEx(mdc, PAD_L, icon_y, s.icon_search, 24, 24, 0, HBRUSH(null_mut()), DI_NORMAL);
    }

    // Text / placeholder
    let tx = PAD_L + ICON_W + 8;
    let ty = (SEARCH_H - 26) / 2;
    let tw = win_w - tx - PAD_L;
    let mut tr = RECT { left: tx, top: ty, right: tx + tw, bottom: ty + 26 };

    SelectObject(mdc, s.font_q);

    if s.query.is_empty() {
        let mut ph: Vec<u16> = "Search Windows settings...".encode_utf16().collect();
        SetTextColor(mdc, CLR_PH);
        let _ = DrawTextW(mdc, &mut ph, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    } else {
        let mut dw: Vec<u16> = s.query.encode_utf16().collect();
        if s.text_selected {
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(mdc, &dw, &mut size);
            let text_h = size.cy;
            let sel_top = tr.top + (tr.bottom - tr.top - text_h) / 2;
            fill(mdc, tx, sel_top, size.cx, text_h, COLORREF(0x00_C5_6A_00)); // Accent blue (#006AC5)
            
            SetTextColor(mdc, CLR_WHITE);
            let _ = DrawTextW(mdc, &mut dw, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        } else {
            let display = format!("{}_", s.query);
            let mut dw_cursor: Vec<u16> = display.encode_utf16().collect();
            SetTextColor(mdc, CLR_WHITE);
            let _ = DrawTextW(mdc, &mut dw_cursor, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
    }

    // ── Results ───────────────────────────────────────────────────────────
    let n = s.results.len().min(VISIBLE_RESULTS);
    if n > 0 {
        fill(mdc, 0, SEARCH_H, win_w, 1, CLR_DIV);

        for i in 0..n {
            let res_idx = s.scroll_offset + i;
            let res = &s.results[res_idx];
            let ry = SEARCH_H + 1 + i as i32 * RESULT_H;

            if res_idx == s.selected { fill(mdc, 0, ry, win_w, RESULT_H, BG_SEL); }
            if i > 0 { fill(mdc, PAD_L, ry, win_w - PAD_L * 2, 1, CLR_DIV); }

            let cy = ry + (RESULT_H - 40) / 2;

            // Draw Icon
            let icon_to_draw = if res.entry.source == "app" || res.entry.source == "RECENT" || res.entry.source == "FILE" || res.entry.source == "CODE" {
                s.app_icons.get(&res.entry.launch_command)
                    .copied()
                    .filter(|h| !h.0.is_null())
                    .unwrap_or(s.icon_control_panel)
            } else if res.entry.launch_command.starts_with("ms-settings:") {
                s.icon_settings
            } else if res.entry.source == "web" || res.entry.source == "HISTORY" {
                s.icon_web
            } else if res.entry.source == "BOOKMARK" {
                s.icon_bookmark
            } else if res.entry.source == "FOLDER" {
                s.icon_folder
            } else if res.entry.source == "COMMIT" {
                s.icon_commit
            } else if res.entry.source == "TODO" {
                s.icon_todo
            } else if res.entry.source == "CLIPBOARD" {
                s.icon_clipboard
            } else {
                s.icon_control_panel
            };

            if !icon_to_draw.0.is_null() {
                let icon_y = ry + (RESULT_H - 32) / 2;
                let _ = DrawIconEx(mdc, PAD_L, icon_y, icon_to_draw, 32, 32, 0, HBRUSH(null_mut()), DI_NORMAL);
            }

            // Name
            SelectObject(mdc, s.font_n);
            SetTextColor(mdc, CLR_WHITE);
            let mut name: Vec<u16> = res.entry.control_name.encode_utf16().collect();
            let mut r = RECT { left: tx, top: cy, right: win_w - 96, bottom: cy + 22 };
            let _ = DrawTextW(mdc, &mut name, &mut r,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

            // Breadcrumb
            SelectObject(mdc, s.font_c);
            SetTextColor(mdc, CLR_GRAY);
            let mut crumb: Vec<u16> = res.entry.breadcrumb_path.encode_utf16().collect();
            let mut r2 = RECT { left: tx, top: cy + 24, right: win_w - 96, bottom: cy + 40 };
            let _ = DrawTextW(mdc, &mut crumb, &mut r2,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

            // Badge
            badge(mdc, s, &res.entry.source, win_w - 88, ry + (RESULT_H - 20) / 2);
        }

        // Draw scrollbar if there are more results than visible
        let total_results = s.results.len();
        if total_results > VISIBLE_RESULTS {
            let track_top = SEARCH_H + 8;
            let track_bottom = win_h - 8;
            let track_h = track_bottom - track_top;
            
            // Thumb height proportional to ratio of visible results, capped at min 24px
            let thumb_h = ((VISIBLE_RESULTS as f32 / total_results as f32) * track_h as f32) as i32;
            let thumb_h = thumb_h.max(24);
            
            // Thumb position proportional to scroll_offset
            let max_offset = total_results - VISIBLE_RESULTS;
            let thumb_y = track_top + (s.scroll_offset as f32 / max_offset as f32 * (track_h - thumb_h) as f32) as i32;
            
            // Draw subtle track
            let sb_x = win_w - 10;
            let sb_w = 4;
            fill(mdc, sb_x, track_top, sb_w, track_h, COLORREF(0x00_2A_2A_2A));
            
            // Draw thumb
            fill(mdc, sb_x, thumb_y, sb_w, thumb_h, CLR_GRAY);
        }
    }

    let _ = BitBlt(hdc, 0, 0, win_w, win_h, mdc, 0, 0, SRCCOPY);
    let _ = SelectObject(mdc, old);
    let _ = DeleteObject(bmp);
    let _ = DeleteDC(mdc);
    EndPaint(hwnd, &ps);
}

unsafe fn fill(hdc: HDC, x: i32, y: i32, w: i32, h: i32, c: COLORREF) {
    let br = CreateSolidBrush(c);
    FillRect(hdc, &RECT { left: x, top: y, right: x + w, bottom: y + h }, br);
    DeleteObject(br);
}

unsafe fn badge(hdc: HDC, s: &State, source: &str, x: i32, y: i32) {
    let src_lc = source.to_lowercase();
    let (label, bg_color, tx_color) = if src_lc == "live" {
        ("LIVE", COLORREF(0x00_1F_A6_0A), CLR_WHITE)
    } else if src_lc == "action" {
        ("ACTION", COLORREF(0x00_B5_25_9E), CLR_WHITE)
    } else if src_lc == "translated" {
        ("RESOLVED", COLORREF(0x00_00_7F_FF), CLR_WHITE)
    } else if src_lc == "web" {
        ("WEB", COLORREF(0x00_C5_6A_00), CLR_WHITE)
    } else if src_lc == "app" {
        ("APP", COLORREF(0x00_A6_8F_0A), CLR_WHITE)
    } else if src_lc == "calc" {
        ("CALC", COLORREF(0x00_9B_4D_00), CLR_WHITE)
    } else if src_lc == "recent" {
        ("RECENT", COLORREF(0x00_7A_1F_7A), CLR_WHITE)
    } else if src_lc == "file" {
        ("FILE", COLORREF(0x00_90_40_00), CLR_WHITE)
    } else if src_lc == "code" {
        ("CODE", COLORREF(0x00_70_20_70), CLR_WHITE)
    } else if src_lc == "clipboard" {
        ("CLIP", COLORREF(0x00_A6_6A_0A), CLR_WHITE)
    } else if src_lc == "bookmark" {
        ("BOOKMARK", COLORREF(0x00_00_A5_D6), CLR_WHITE)
    } else if src_lc == "history" {
        ("HISTORY", COLORREF(0x00_90_60_20), CLR_WHITE)
    } else if src_lc == "folder" {
        ("FOLDER", COLORREF(0x00_13_45_8B), CLR_WHITE)
    } else if src_lc == "commit" {
        ("COMMIT", COLORREF(0x00_20_7A_D6), CLR_WHITE)
    } else if src_lc == "todo" {
        ("TODO", COLORREF(0x00_2A_3E_E6), CLR_WHITE)
    } else if src_lc == "browser" {
        ("BROWSER", COLORREF(0x00_2A_8F_C6), CLR_WHITE)
    } else if src_lc.contains("legacy") {
        ("LEGACY", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc.contains("native") {
        ("NATIVE", CLR_BDGBG, CLR_BDGTX)
    } else {
        ("MODERN", CLR_BDGBG, CLR_BDGTX)
    };
    fill(hdc, x, y, 64, 20, bg_color);
    SelectObject(hdc, s.font_b);
    SetTextColor(hdc, tx_color);
    SetBkMode(hdc, TRANSPARENT);
    let mut t: Vec<u16> = label.encode_utf16().collect();
    let mut r = RECT { left: x, top: y, right: x + 64, bottom: y + 20 };
    DrawTextW(hdc, &mut t, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
}

unsafe fn load_icon_from_dll(dll_name: &str, index: i32, size: i32) -> HICON {
    let mut filename_buf = [0u16; 260];
    let dll_wide: Vec<u16> = dll_name.encode_utf16().collect();
    for (dest, src) in filename_buf.iter_mut().zip(dll_wide.iter()) {
        *dest = *src;
    }
    
    let mut phicon = [HICON(std::ptr::null_mut())];
    let mut piconid_val = 0u32;
    
    let num = PrivateExtractIconsW(
        &filename_buf,
        index,
        size,
        size,
        Some(&mut phicon),
        Some(&mut piconid_val as *mut u32),
        1,
    );
    if num > 0 && !phicon[0].0.is_null() {
        phicon[0]
    } else {
        HICON(std::ptr::null_mut())
    }
}

unsafe fn load_icon_from_memory(bytes: &[u8], size: i32) -> HICON {
    if bytes.len() < 6 { return HICON(null_mut()); }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let mut best_idx = 0;
    let mut best_diff = i32::MAX;
    
    for i in 0..count {
        let offset = 6 + i * 16;
        if offset + 16 > bytes.len() { break; }
        let mut w = bytes[offset] as i32;
        let mut h = bytes[offset + 1] as i32;
        if w == 0 { w = 256; }
        if h == 0 { h = 256; }
        let diff = (w - size).abs() + (h - size).abs();
        if diff < best_diff {
            best_diff = diff;
            best_idx = i;
        }
    }
    
    let entry_offset = 6 + best_idx * 16;
    if entry_offset + 16 <= bytes.len() {
        let img_size = u32::from_le_bytes([
            bytes[entry_offset + 8],
            bytes[entry_offset + 9],
            bytes[entry_offset + 10],
            bytes[entry_offset + 11],
        ]) as usize;
        let img_offset = u32::from_le_bytes([
            bytes[entry_offset + 12],
            bytes[entry_offset + 13],
            bytes[entry_offset + 14],
            bytes[entry_offset + 15],
        ]) as usize;
        
        if img_offset + img_size <= bytes.len() {
            let img_bytes = &bytes[img_offset .. img_offset + img_size];
            let hicon = CreateIconFromResourceEx(
                img_bytes,
                TRUE,
                0x00030000,
                size,
                size,
                IMAGE_FLAGS(0),
            );
            if let Ok(h) = hicon {
                return h;
            }
        }
    }
    HICON(null_mut())
}

unsafe fn get_active_app_name() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_NAME_WIN32};
    use windows::Win32::Foundation::{CloseHandle, BOOL};
    use windows::core::PWSTR;

    let hwnd_fg = GetForegroundWindow();
    if hwnd_fg.0.is_null() {
        return "Unknown".to_string();
    }

    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd_fg, Some(&mut process_id));
    if process_id == 0 {
        return "Unknown".to_string();
    }

    if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, BOOL(0), process_id) {
        let mut buffer = [0u16; 512];
        let mut size = buffer.len() as u32;
        let res = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);

        if res.is_ok() && size > 0 {
            let path_str = String::from_utf16_lossy(&buffer[..size as usize]);
            if let Some(filename) = std::path::Path::new(&path_str).file_name() {
                return filename.to_string_lossy().into_owned();
            }
            return path_str;
        }
    }

    "Unknown".to_string()
}

unsafe fn copy_image_to_clipboard(hwnd: HWND, file_path: &str) {
    use windows::Win32::System::DataExchange::{OpenClipboard, CloseClipboard, EmptyClipboard, SetClipboardData};
    use windows::Win32::UI::WindowsAndMessaging::{LoadImageW, IMAGE_BITMAP, LR_LOADFROMFILE, LR_CREATEDIBSECTION};
    use windows::Win32::Foundation::HANDLE;
    use windows::core::PCWSTR;

    let wide_path: Vec<u16> = file_path.encode_utf16().chain(std::iter::once(0)).collect();
    let h_img = LoadImageW(
        None,
        PCWSTR(wide_path.as_ptr()),
        IMAGE_BITMAP,
        0,
        0,
        LR_LOADFROMFILE | LR_CREATEDIBSECTION,
    );

    if let Ok(hbitmap) = h_img {
        if OpenClipboard(hwnd).is_ok() {
            let _ = EmptyClipboard();
            let _ = SetClipboardData(2, HANDLE(hbitmap.0));
            let _ = CloseClipboard();
        }
    }
}

unsafe fn capture_clipboard_image_data(hwnd: HWND) -> Option<(Vec<u8>, windows::Win32::Graphics::Gdi::BITMAPINFOHEADER)> {
    use windows::Win32::System::DataExchange::{OpenClipboard, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable};
    use windows::Win32::Graphics::Gdi::{
        GetDC, ReleaseDC, GetObjectW, GetDIBits, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP
    };
    use windows::Win32::Foundation::HWND;

    // CF_BITMAP is 2
    if IsClipboardFormatAvailable(2).is_err() {
        return None;
    }

    if OpenClipboard(hwnd).is_err() {
        return None;
    }

    let mut result = None;
    if let Ok(h_mem) = GetClipboardData(2) {
        if !h_mem.0.is_null() {
            let hbitmap = HBITMAP(h_mem.0);
            let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
            if !hdc_screen.is_invalid() {
                let mut bmp: BITMAP = std::mem::zeroed();
                let size = std::mem::size_of::<BITMAP>() as i32;
                if GetObjectW(hbitmap, size, Some(&mut bmp as *mut BITMAP as *mut _)) != 0 {
                    let bih = BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: bmp.bmWidth,
                        biHeight: bmp.bmHeight,
                        biPlanes: 1,
                        biBitCount: 32, // Convert to 32-bit BGRA
                        biCompression: 0, // BI_RGB
                        biSizeImage: (bmp.bmWidth * bmp.bmHeight * 4) as u32,
                        biXPelsPerMeter: 0,
                        biYPelsPerMeter: 0,
                        biClrUsed: 0,
                        biClrImportant: 0,
                    };

                    let mut bi = BITMAPINFO {
                        bmiHeader: bih,
                        bmiColors: [std::mem::zeroed(); 1],
                    };

                    let mut buf = vec![0u8; (bmp.bmWidth * bmp.bmHeight * 4) as usize];
                    
                    let res = GetDIBits(
                        hdc_screen,
                        hbitmap,
                        0,
                        bmp.bmHeight as u32,
                        Some(buf.as_mut_ptr() as *mut _),
                        &mut bi,
                        DIB_RGB_COLORS,
                    );

                    if res != 0 {
                        result = Some((buf, bih));
                    }
                }
                let _ = ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            }
        }
    }
    let _ = CloseClipboard();
    result
}

fn write_bmp_file(path: &std::path::Path, buf: &[u8], bih: windows::Win32::Graphics::Gdi::BITMAPINFOHEADER) -> Result<(), String> {
    use std::fs::File;
    use std::io::Write;

    let file_size = 54 + buf.len();
    let mut file_header = [0u8; 14];
    file_header[0] = b'B';
    file_header[1] = b'M';
    file_header[2..6].copy_from_slice(&(file_size as u32).to_le_bytes());
    file_header[10..14].copy_from_slice(&54u32.to_le_bytes());

    let mut info_header = [0u8; 40];
    info_header[0..4].copy_from_slice(&bih.biSize.to_le_bytes());
    info_header[4..8].copy_from_slice(&bih.biWidth.to_le_bytes());
    info_header[8..12].copy_from_slice(&bih.biHeight.to_le_bytes());
    info_header[12..14].copy_from_slice(&bih.biPlanes.to_le_bytes());
    info_header[14..16].copy_from_slice(&bih.biBitCount.to_le_bytes());
    info_header[16..20].copy_from_slice(&bih.biCompression.to_le_bytes());
    info_header[20..24].copy_from_slice(&bih.biSizeImage.to_le_bytes());

    let mut file = File::create(path).map_err(|e| e.to_string())?;
    file.write_all(&file_header).map_err(|e| e.to_string())?;
    file.write_all(&info_header).map_err(|e| e.to_string())?;
    file.write_all(buf).map_err(|e| e.to_string())?;

    Ok(())
}

unsafe fn import_windows_clipboard_history(db_path: &std::path::Path) {
    use windows::ApplicationModel::DataTransfer::{Clipboard, StandardDataFormats};
    
    if Clipboard::IsHistoryEnabled().unwrap_or(false) {
        if let Ok(op) = Clipboard::GetHistoryItemsAsync() {
            if let Ok(result) = op.get() {
                if let Ok(items) = result.Items() {
                    if let Ok(conn) = rusqlite::Connection::open(db_path) {
                        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                        let mut time_offset = 0;
                        
                        for item in items {
                            if let Ok(content) = item.Content() {
                                let is_text = if let Ok(fmt) = StandardDataFormats::Text() {
                                    content.Contains(&fmt).unwrap_or(false)
                                } else {
                                    false
                                };
                                
                                if is_text {
                                    if let Ok(text_op) = content.GetTextAsync() {
                                        if let Ok(text) = text_op.get() {
                                            let trimmed = text.to_string().trim().to_string();
                                            if !trimmed.is_empty() {
                                                // Decrement timestamp to preserve original sorting order of items
                                                let timestamp = now - time_offset;
                                                time_offset += 1;
                                                let _ = conn.execute(
                                                    "INSERT OR IGNORE INTO clipboard_history (content, timestamp, source_app, is_image) VALUES (?, ?, 'Windows History', 0);",
                                                    rusqlite::params![trimmed, timestamp],
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    let is_bitmap = if let Ok(fmt) = StandardDataFormats::Bitmap() {
                                        content.Contains(&fmt).unwrap_or(false)
                                    } else {
                                        false
                                    };
                                    
                                    if is_bitmap {
                                        if let Ok(bitmap_op) = content.GetBitmapAsync() {
                                            if let Ok(stream_ref) = bitmap_op.get() {
                                                if let Ok(open_op) = stream_ref.OpenReadAsync() {
                                                    if let Ok(stream) = open_op.get() {
                                                        let size = stream.Size().unwrap_or(0);
                                                        if size > 0 && size < 50 * 1024 * 1024 {
                                                            use windows::Storage::Streams::DataReader;
                                                            if let Ok(reader) = DataReader::CreateDataReader(&stream) {
                                                                if reader.LoadAsync(size as u32).and_then(|l| l.get()).is_ok() {
                                                                    let mut buf = vec![0u8; size as usize];
                                                                    if reader.ReadBytes(&mut buf).is_ok() {
                                                                        let timestamp = now - time_offset;
                                                                        time_offset += 1;
                                                                        let filename = format!("image_{}.bmp", timestamp);
                                                                        let img_dir = db_path.parent().unwrap().join("clipboard_images");
                                                                        let _ = std::fs::create_dir_all(&img_dir);
                                                                        let img_path = img_dir.join(&filename);
                                                                        let img_path_str = img_path.to_string_lossy().to_string();
                                                                        
                                                                        if std::fs::write(&img_path, &buf).is_ok() {
                                                                            let _ = conn.execute(
                                                                                "INSERT OR IGNORE INTO clipboard_history (content, timestamp, source_app, is_image) VALUES (?, ?, 'Windows History', 1);",
                                                                                rusqlite::params![img_path_str, timestamp],
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

unsafe fn copy_to_clipboard(hwnd: HWND, text: &str) {
    use windows::Win32::System::DataExchange::{OpenClipboard, CloseClipboard, EmptyClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    
    if OpenClipboard(hwnd).is_ok() {
        let _ = EmptyClipboard();
        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let size = utf16.len() * 2;
        if let Ok(h_mem) = GlobalAlloc(GMEM_MOVEABLE, size) {
            let ptr = GlobalLock(h_mem);
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(utf16.as_ptr() as *const u8, ptr as *mut u8, size);
                let _ = GlobalUnlock(h_mem);
                let _ = SetClipboardData(13, HANDLE(h_mem.0));
            }
        }
        let _ = CloseClipboard();
    }
}

unsafe fn paste_from_clipboard(hwnd: HWND) -> Option<String> {
    use windows::Win32::System::DataExchange::{OpenClipboard, CloseClipboard, GetClipboardData};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    
    let mut result = None;
    if OpenClipboard(hwnd).is_ok() {
        if let Ok(h_mem) = GetClipboardData(13) {
            if !h_mem.0.is_null() {
                let h_global = HGLOBAL(h_mem.0);
                let ptr = GlobalLock(h_global);
                if !ptr.is_null() {
                    let mut len = 0;
                    let ptr_u16 = ptr as *const u16;
                    while *ptr_u16.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(ptr_u16, len);
                    if let Ok(s) = String::from_utf16(slice) {
                        result = Some(s);
                    }
                    let _ = GlobalUnlock(h_global);
                }
            }
        }
        let _ = CloseClipboard();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winrt_clipboard() {
        unsafe {
            let res = windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED
            );
            if res.is_ok() {
                use windows::ApplicationModel::DataTransfer::Clipboard;
                if let Ok(enabled) = Clipboard::IsHistoryEnabled() {
                    println!("Clipboard history enabled status: {}", enabled);
                }
                windows::Win32::System::Com::CoUninitialize();
            }
        }
    }

    #[test]
    fn test_antigravity_icons() {
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE
            );
            let h1 = get_app_icon("electron.app.Antigravity");
            let h2 = get_app_icon("Google.Antigravity");
            let h3 = get_app_icon("Google.AntigravityIDE");
            println!("electron.app.Antigravity HICON: {:?}", h1.0);
            println!("Google.Antigravity HICON: {:?}", h2.0);
            println!("Google.AntigravityIDE HICON: {:?}", h3.0);
            assert!(!h1.0.is_null(), "electron.app.Antigravity icon was null");
            assert!(!h2.0.is_null(), "Google.Antigravity icon was null");
            assert!(!h3.0.is_null(), "Google.AntigravityIDE icon was null");
        }
    }

    #[test]
    fn test_image_factory_cast() {
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE
            );

            unsafe fn get_icon_via_image_factory(parsing_path: &str) -> Option<HICON> {
                use windows::Win32::UI::Shell::IShellItemImageFactory;
                use windows::Win32::UI::Shell::SIIGBF_ICONONLY;
                use windows::Win32::Foundation::SIZE;
                use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
                use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, ICONINFO};
                use windows::Win32::UI::Shell::SHCreateItemFromParsingName;
                use windows::core::{Interface, PCWSTR};
                
                let path_wide: Vec<u16> = parsing_path.encode_utf16().chain(std::iter::once(0)).collect();
                let item: windows::Win32::UI::Shell::IShellItem = SHCreateItemFromParsingName(
                    PCWSTR(path_wide.as_ptr()),
                    None,
                ).ok()?;
                
                let factory: IShellItemImageFactory = item.cast().ok()?;
                let hbitmap = factory.GetImage(
                    SIZE { cx: 32, cy: 32 },
                    SIIGBF_ICONONLY,
                ).ok()?;
                
                let hbm_mask = CreateBitmap(32, 32, 1, 1, None);
                if hbm_mask.is_invalid() {
                    let _ = DeleteObject(hbitmap);
                    return None;
                }
                
                let mut ii = ICONINFO {
                    fIcon: windows::Win32::Foundation::TRUE,
                    xHotspot: 0,
                    yHotspot: 0,
                    hbmMask: hbm_mask,
                    hbmColor: hbitmap,
                };
                
                let hicon = CreateIconIndirect(&mut ii).ok();
                
                let _ = DeleteObject(hbitmap);
                let _ = DeleteObject(hbm_mask);
                
                hicon
            }
            
            for app_id in &["electron.app.Antigravity", "Google.Antigravity", "Google.AntigravityIDE"] {
                let parsing_path = format!("shell:AppsFolder\\{}", app_id);
                let hicon = get_icon_via_image_factory(&parsing_path);
                println!("App: {}, ImageFactory HICON: {:?}", app_id, hicon.map(|h| h.0));
                if let Some(h) = hicon {
                    let _ = windows::Win32::UI::WindowsAndMessaging::DestroyIcon(h);
                }
            }
        }
    }
}
