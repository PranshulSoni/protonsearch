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
const TIMER_CURSOR_BLINK: usize = 2;
const CURSOR_BLINK_MS: u32 = 530;
const WM_ICON_LOADED: u32 = WM_USER + 1;
const WM_ENGINE_READY: u32 = WM_USER + 2;
const WM_SEARCH_RESULTS: u32 = WM_USER + 3;
const WM_START_EDITING: u32 = WM_USER + 4;
const WM_REFRESH_SEARCH: u32 = WM_USER + 5;

struct SearchRequest {
    query: String,
    query_id: usize,
}
// ── Animation ─────────────────────────────────────────────────────────────────
// const ANIM_TICK_MS: u32 = 1;
const ANIM_DURATION_SEC: f32 = 0.160; // 160ms
// const MAX_ALPHA: u8 = 255;

// ── Genie Morph Dimensions ────────────────────────────────────────────────────
// const PILL_H: i32 = 12; // Starting height at top center

// ── Colors (COLORREF = 0x00BBGGRR) ───────────────────────────────────────────
const BG: COLORREF        = COLORREF(0x00_24_21_21);
const BG_SEL: COLORREF    = COLORREF(0x00_3E_38_38);
const CLR_DIV: COLORREF   = COLORREF(0x00_40_3A_3A);
const CLR_WHITE: COLORREF = COLORREF(0x00_FF_FF_FF);
const CLR_GRAY: COLORREF  = COLORREF(0x00_9A_94_94);
const CLR_PH: COLORREF    = COLORREF(0x00_5A_55_55);
const CLR_BDGBG: COLORREF = COLORREF(0x00_50_48_48);
const CLR_BDGTX: COLORREF = COLORREF(0x00_C0_BB_BB);
const COLOR_KEY: COLORREF = COLORREF(0x00_12_34_56);

// ── App state ─────────────────────────────────────────────────────────────────
struct State {
    search_tx: Option<std::sync::mpsc::Sender<SearchRequest>>,
    icon_tx: Option<std::sync::mpsc::Sender<IconRequest>>,
    current_query_id: usize,
    db_path: std::path::PathBuf,
    query: String,
    cursor_pos: usize,
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
    icon_memory: HICON,
    text_selected: bool,
    cursor_visible: bool,
    scroll_offset: usize,
    last_mouse_x: i32,
    last_mouse_y: i32,
    app_icons: std::collections::HashMap<String, HICON>,
    clipboard_thumbnails: std::cell::RefCell<std::collections::HashMap<String, HBITMAP>>,
    selected_clip_ids: std::collections::HashSet<String>,
    delete_confirm: bool,
    editing_item: Option<String>,
}

#[derive(PartialEq)]
enum Anim {
    Hidden,
    Appearing { start_time: std::time::Instant, start_p: f32 },
    Visible,
    Hiding { start_time: std::time::Instant, start_p: f32 },
}

#[derive(Clone, Copy)]
struct SendHwnd(HWND);
unsafe impl Send for SendHwnd {}
unsafe impl Sync for SendHwnd {}

struct IconRequest {
    key: String,
    source: String,
}

impl State {
    fn win_h(&self) -> i32 {
        let n = self.results.len().min(VISIBLE_RESULTS) as i32;
        let base_h = if n == 0 { SEARCH_H } else { SEARCH_H + 1 + n * RESULT_H };
        if self.query.starts_with("clip:") || self.query.starts_with("clipboard:") {
            base_h + 24
        } else {
            base_h
        }
    }
    fn result_rect(&self, i: usize) -> RECT {
        let end_h = self.win_h();
        let end_y = self.cy - end_h / 2;
        let y = end_y + SEARCH_H + 1 + i as i32 * RESULT_H;
        RECT { left: 0, top: y, right: WIN_W, bottom: y + RESULT_H }
    }
    fn current_p(&self) -> f32 {
        match self.anim {
            Anim::Hidden => 0.0,
            Anim::Visible => 1.0,
            Anim::Appearing { start_time, start_p } => {
                let elapsed = start_time.elapsed().as_secs_f32();
                (start_p + elapsed / ANIM_DURATION_SEC).min(1.0)
            }
            Anim::Hiding { start_time, start_p } => {
                let elapsed = start_time.elapsed().as_secs_f32();
                (start_p - elapsed / ANIM_DURATION_SEC).max(0.0)
            }
        }
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

    let db_path = match std::env::var("APPDATA") {
        Ok(d) => {
            let path = std::path::PathBuf::from(d).join("opensearch-os");
            let _ = std::fs::create_dir_all(&path);
            path.join("file_index.db")
        }
        Err(_) => std::path::PathBuf::from("file_index.db"),
    };

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
    let icon_memory = load_icon_from_dll("shell32.dll", 238, 32);

    let (icon_tx, icon_rx) = std::sync::mpsc::channel::<IconRequest>();

    let state = Box::new(State {
        search_tx: None,
        icon_tx: Some(icon_tx),
        current_query_id: 0,
        db_path: db_path.clone(),
        query: String::new(),
        cursor_pos: 0,
        results: vec![],
        selected: 0,
        anim: Anim::Hidden,
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
        icon_memory,
        text_selected: false,
        cursor_visible: true,
        scroll_offset: 0,
        last_mouse_x: -1,
        last_mouse_y: -1,
        app_icons: std::collections::HashMap::new(),
        clipboard_thumbnails: std::cell::RefCell::new(std::collections::HashMap::new()),
        selected_clip_ids: std::collections::HashSet::new(),
        delete_confirm: false,
        editing_item: None,
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
    let win_x = (sw - WIN_W) / 2;
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        PCWSTR(class.as_ptr()),
        PCWSTR::null(),
        WS_POPUP,
        win_x, 0, WIN_W, 800,
        HWND(null_mut()), HMENU(null_mut()), hinst,
        Some(Box::into_raw(state) as _),
    ).unwrap();

    let hwnd_icon = SendHwnd(hwnd);
    std::thread::spawn(move || {
        let hwnd_raw = hwnd_icon;
        let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE
        ) };
        while let Ok(req) = icon_rx.recv() {
            unsafe {
                let is_real_folder = req.source == "FOLDER" && !req.key.ends_with(':') && std::path::Path::new(&req.key).exists();
                let is_real_project = req.source == "PROJECT" && !req.key.is_empty() && !req.key.starts_with("http") && std::path::Path::new(&req.key).exists();
                let is_file_icon = req.source == "RECENT" || req.source == "FILE" || req.source == "CODE" || is_real_folder || is_real_project;
                let hicon = if is_file_icon {
                    get_file_icon(&req.key)
                } else if req.source == "ACTION" && req.key.starts_with("kill:") {
                    let pid_str = req.key.strip_prefix("kill:").unwrap_or("");
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        if let Some(path) = get_process_path(pid) {
                            get_app_icon(&path)
                        } else {
                            get_app_icon("C:\\Windows\\System32\\cmd.exe")
                        }
                    } else {
                        HICON(std::ptr::null_mut())
                    }
                } else {
                    get_app_icon(&req.key)
                };
                if !hicon.0.is_null() {
                    let key_ptr = Box::into_raw(Box::new(req.key));
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
        }
    });

    let _ = unsafe { windows::Win32::System::DataExchange::AddClipboardFormatListener(hwnd) };

    SetLayeredWindowAttributes(hwnd, COLOR_KEY, 255, LWA_COLORKEY).unwrap();

    // DWM rounded corners (Windows 11) - Do not round the transparent box
    let corner = DWMWCP_DONOTROUND;
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
    let db_path_for_thread = db_path.clone();
    std::thread::spawn(move || {
        let db_path = db_path_for_thread;
        indexer::start_indexer(db_path.clone());
        browser_indexer::start_browser_indexer(db_path.clone());
        git_indexer::start_git_indexer(db_path.clone());

        let db_path_for_timeline = db_path.clone();
        let hwnd_for_timeline = SendHwnd(HWND(hwnd_usize as *mut std::ffi::c_void));
        std::thread::spawn(move || {
            let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED) };
            unsafe { start_timeline_tracker(db_path_for_timeline, hwnd_for_timeline); }
            unsafe { windows::Win32::System::Com::CoUninitialize(); }
        });

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
                Ok(mut engine) => {
                    // Import Windows Clipboard History in background
                    let db_path_clone = db_path.clone();
                    std::thread::spawn(move || {
                        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
                        import_windows_clipboard_history(&db_path_clone);
                        windows::Win32::System::Com::CoUninitialize();
                    });

                    // Spawn worker channels
                    let (tx, rx) = std::sync::mpsc::channel::<SearchRequest>();
                    let hwnd_worker = SendHwnd(hwnd_bg);

                    // Spawn search worker thread
                    std::thread::spawn(move || {
                        let hwnd_target = hwnd_worker;
                        while let Ok(req) = rx.recv() {
                            // Drain queued requests to keep only the latest one
                            let mut latest_req = req;
                            while let Ok(next_req) = rx.try_recv() {
                                latest_req = next_req;
                            }

                            let results = engine.search(&latest_req.query, MAX_RESULTS);
                            let results_ptr = Box::into_raw(Box::new(results)) as isize;
                            let _ = PostMessageW(
                                hwnd_target.0,
                                WM_SEARCH_RESULTS,
                                WPARAM(latest_req.query_id),
                                LPARAM(results_ptr),
                            );
                        }
                    });

                    let tx_ptr = Box::into_raw(Box::new(tx)) as isize;
                    let _ = PostMessageW(hwnd_bg, WM_ENGINE_READY, WPARAM(1), LPARAM(tx_ptr));
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
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
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
                Anim::Hidden | Anim::Hiding { .. } => do_show(hwnd, s),
                _ => start_hide(hwnd, s),
            }
            LRESULT(0)
        }

        WM_START_EDITING => {
            let ptr = lp.0 as *mut (String, String);
            let (id, content) = unsafe { *Box::from_raw(ptr) };
            if !sp.is_null() {
                let s = unsafe { &mut *sp };
                s.editing_item = Some(id);
                s.query = content;
                s.cursor_pos = s.query.len();
                s.selected = 0;
                s.scroll_offset = 0;
                let _ = unsafe { InvalidateRect(hwnd, None, FALSE) };
            }
            LRESULT(0)
        }

        WM_REFRESH_SEARCH => {
            if !sp.is_null() {
                let s = unsafe { &mut *sp };
                unsafe { trigger_search(hwnd, s); }
            }
            LRESULT(0)
        }

        WM_KILLFOCUS => {
            if !sp.is_null() {
                let s = &mut *sp;
                if !matches!(s.anim, Anim::Hidden | Anim::Hiding { .. }) {
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

            let db_path = s.db_path.clone();
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
                            let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;
                            let _ = conn.execute(
                                "INSERT INTO clipboard_history (content, timestamp, source_app, is_image, pinned) \
                                 VALUES (?, ?, ?, 0, 0) \
                                 ON CONFLICT(content) DO UPDATE SET \
                                     timestamp = excluded.timestamp, \
                                     source_app = excluded.source_app, \
                                     is_image = excluded.is_image;",
                                rusqlite::params![trimmed, now, app_name_clone],
                            );
                            let _ = conn.execute(
                                "DELETE FROM clipboard_history WHERE pinned = 0 AND id NOT IN (SELECT id FROM clipboard_history ORDER BY pinned DESC, timestamp DESC LIMIT 500);",
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
                                    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                                    let _ = conn.execute(
                                        "INSERT INTO clipboard_history (content, timestamp, source_app, is_image, pinned) \
                                         VALUES (?, ?, ?, 1, 0) \
                                         ON CONFLICT(content) DO UPDATE SET \
                                             timestamp = excluded.timestamp, \
                                             source_app = excluded.source_app, \
                                             is_image = excluded.is_image;",
                                        rusqlite::params![img_path_str, now, app_name],
                                    );
                                    let _ = conn.execute(
                                        "DELETE FROM clipboard_history WHERE pinned = 0 AND id NOT IN (SELECT id FROM clipboard_history ORDER BY pinned DESC, timestamp DESC LIMIT 500);",
                                        [],
                                    );
                                }
                            }
                        });
                    }
                }
            }
            LRESULT(0)
        }

        WM_ENGINE_READY => {
            if wp.0 == 1 {
                let tx = unsafe { *Box::from_raw(lp.0 as *mut std::sync::mpsc::Sender<SearchRequest>) };
                if !sp.is_null() {
                    let s = &mut *sp;
                    s.search_tx = Some(tx);
                    trigger_search(hwnd, s);
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

        WM_SEARCH_RESULTS => {
            let query_id = wp.0;
            let results_ptr = lp.0 as *mut Vec<SearchResult>;
            let results = unsafe { *Box::from_raw(results_ptr) };
            if !sp.is_null() {
                let s = &mut *sp;
                if query_id == s.current_query_id {
                    s.results = results;
                    if s.results.is_empty() {
                        s.selected = 0;
                        s.scroll_offset = 0;
                    } else {
                        s.selected = s.selected.min(s.results.len() - 1);
                        s.scroll_offset = s.scroll_offset.min(s.results.len().saturating_sub(VISIBLE_RESULTS));
                    }
                    trigger_icon_loading(hwnd, s);
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
            }
            LRESULT(0)
        }

        WM_TIMER => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            match wp.0 {
                TIMER_DEBOUNCE => {
                    let _ = KillTimer(hwnd, TIMER_DEBOUNCE);
                    trigger_search(hwnd, s);
                }
                TIMER_CURSOR_BLINK => {
                    s.cursor_visible = !s.cursor_visible;
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
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
                        s.cursor_pos = 0;
                        s.text_selected = false;
                    }
                    s.query.insert(s.cursor_pos, c);
                    s.cursor_pos += c.len_utf8();
                    s.selected = 0;
                    s.scroll_offset = 0;
                    kick_debounce(hwnd);
                    reset_cursor_blink(hwnd, s);
                    let _ = InvalidateRect(hwnd, None, FALSE);
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
                            let _ = InvalidateRect(hwnd, None, FALSE);
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
                                s.cursor_pos = s.query.len();
                                s.text_selected = false;
                            } else {
                                s.query.insert_str(s.cursor_pos, &clean_text);
                                s.cursor_pos += clean_text.len();
                            }
                            s.selected = 0;
                            s.scroll_offset = 0;
                            kick_debounce(hwnd);
                            reset_cursor_blink(hwnd, s);
                            let _ = InvalidateRect(hwnd, None, FALSE);
                        }
                        return LRESULT(0);
                    }
                    0x50 => { // Ctrl + P (Pin/Unpin toggle)
                        if let Some(r) = s.results.get(s.selected) {
                            if r.entry.source == "CLIPBOARD" {
                                let id = r.entry.id.clone();
                                let parts: Vec<&str> = id.split('.').collect();
                                if let Some(ts_str) = parts.last() {
                                    if let Ok(ts) = ts_str.parse::<i64>() {
                                        let db_path = s.db_path.clone();
                                        let is_pinned = id.starts_with("clip.pinned.");
                                        let new_id = if is_pinned {
                                            format!("clip.{}", ts)
                                        } else {
                                            format!("clip.pinned.{}", ts)
                                        };
                                        if s.selected_clip_ids.contains(&id) {
                                            s.selected_clip_ids.remove(&id);
                                            s.selected_clip_ids.insert(new_id.clone());
                                        }
                                        if let Some(r_mut) = s.results.get_mut(s.selected) {
                                            r_mut.entry.id = new_id;
                                        }
                                        let _ = InvalidateRect(hwnd, None, FALSE);

                                        let hwnd_notify = SendHwnd(hwnd);
                                        std::thread::spawn(move || {
                                            let hwnd_notify = hwnd_notify;
                                            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                                                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                                                let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                                                if conn.execute(
                                                    "UPDATE clipboard_history SET pinned = (CASE WHEN pinned = 1 THEN 0 ELSE 1 END) WHERE timestamp = ?;",
                                                    [ts],
                                                ).is_ok() {
                                                    let _ = PostMessageW(
                                                        hwnd_notify.0,
                                                        WM_REFRESH_SEARCH,
                                                        WPARAM(0),
                                                        LPARAM(0),
                                                    );
                                                }
                                            }
                                        });
                                    }
                                }
                             }
                         }
                         return LRESULT(0);
                    }
                    0x45 => { // Ctrl + E (Edit selected clipboard item)
                         if s.editing_item.is_some() {
                             s.editing_item = None;
                             s.query = "clip:".to_string();
                             s.cursor_pos = s.query.len();
                             trigger_search(hwnd, s);
                         } else if let Some(r) = s.results.get(s.selected) {
                             if r.entry.source == "CLIPBOARD" && !r.entry.launch_command.starts_with("copy_image:") {
                                 let id = r.entry.id.clone();
                                 let parts: Vec<&str> = id.split('.').collect();
                                 if let Some(ts_str) = parts.last() {
                                     if let Ok(ts) = ts_str.parse::<i64>() {
                                         let db_path = s.db_path.clone();
                                         let hwnd_notify = SendHwnd(hwnd);
                                         std::thread::spawn(move || {
                                             let hwnd_notify = hwnd_notify;
                                             if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                                                 if let Ok(content) = conn.query_row(
                                                     "SELECT content FROM clipboard_history WHERE timestamp = ?;",
                                                     [ts],
                                                     |row| row.get::<_, String>(0),
                                                 ) {
                                                     let content_ptr = Box::into_raw(Box::new((id, content)));
                                                     let _ = PostMessageW(
                                                         hwnd_notify.0,
                                                         WM_START_EDITING,
                                                         WPARAM(0),
                                                         LPARAM(content_ptr as isize),
                                                     );
                                                 }
                                             }
                                         });
                                     }
                                 }
                             }
                         }
                         return LRESULT(0);
                    }
                    _ => {}
                }
            }

            match vk {
                VK_ESCAPE => {
                    if s.delete_confirm {
                        s.delete_confirm = false;
                        s.selected_clip_ids.clear();
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    } else if s.editing_item.is_some() {
                        s.editing_item = None;
                        s.query = "clip:".to_string();
                        s.cursor_pos = s.query.len();
                        trigger_search(hwnd, s);
                    } else if s.text_selected {
                        s.text_selected = false;
                        reset_cursor_blink(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    } else {
                        start_hide(hwnd, s);
                    }
                }
                VK_LEFT => {
                    if ctrl_down {
                        s.cursor_pos = word_left(&s.query, s.cursor_pos);
                    } else if s.cursor_pos > 0 {
                        let mut p = s.cursor_pos - 1;
                        while p > 0 && !s.query.is_char_boundary(p) {
                            p -= 1;
                        }
                        s.cursor_pos = p;
                    }
                    reset_cursor_blink(hwnd, s);
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
                VK_RIGHT => {
                    if ctrl_down {
                        s.cursor_pos = word_right(&s.query, s.cursor_pos);
                    } else if s.cursor_pos < s.query.len() {
                        let mut p = s.cursor_pos + 1;
                        while p < s.query.len() && !s.query.is_char_boundary(p) {
                            p += 1;
                        }
                        s.cursor_pos = p;
                    }
                    reset_cursor_blink(hwnd, s);
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
                VK_BACK => {
                    if ctrl_down {
                        if s.text_selected {
                            s.query.clear();
                            s.cursor_pos = 0;
                            s.text_selected = false;
                        } else {
                            delete_word_before(s);
                        }
                    } else if s.text_selected {
                        s.query.clear();
                        s.cursor_pos = 0;
                        s.text_selected = false;
                    } else if s.cursor_pos > 0 {
                        let mut p = s.cursor_pos - 1;
                        while p > 0 && !s.query.is_char_boundary(p) {
                            p -= 1;
                        }
                        s.query.remove(p);
                        s.cursor_pos = p;
                    }
                    s.selected = 0;
                    s.scroll_offset = 0;
                    kick_debounce(hwnd);
                    reset_cursor_blink(hwnd, s);
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
                VK_TAB => {
                    let is_clip_view = s.query.starts_with("clip:") || s.query.starts_with("clipboard:");
                    if is_clip_view {
                        if let Some(r) = s.results.get(s.selected) {
                            if r.entry.source == "CLIPBOARD" {
                                let id = r.entry.id.clone();
                                if s.selected_clip_ids.contains(&id) {
                                    s.selected_clip_ids.remove(&id);
                                } else {
                                    s.selected_clip_ids.insert(id);
                                }
                                s.delete_confirm = false;
                                let _ = InvalidateRect(hwnd, None, FALSE);
                            }
                        }
                    }
                }
                VK_DELETE => {
                    let is_clip_view = s.query.starts_with("clip:") || s.query.starts_with("clipboard:");
                    if is_clip_view {
                        if s.delete_confirm {
                            // Second Delete confirms the deletion!
                            s.delete_confirm = false;
                            let db_path = s.db_path.clone();
                            let selected_ids: Vec<String> = s.selected_clip_ids.iter().cloned().collect();
                            let selected_set: std::collections::HashSet<String> = selected_ids.iter().cloned().collect();
                            s.selected_clip_ids.clear();
                            let hwnd_notify = SendHwnd(hwnd);
                            std::thread::spawn(move || {
                                let hwnd_notify = hwnd_notify;
                                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                                    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                                    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                                    let mut deleted_any = false;
                                    for id in &selected_ids {
                                        let parts: Vec<&str> = id.split('.').collect();
                                        if let Some(ts_str) = parts.last() {
                                            if let Ok(ts) = ts_str.parse::<i64>() {
                                                if conn.execute("DELETE FROM clipboard_history WHERE timestamp = ?;", [ts]).is_ok() {
                                                    deleted_any = true;
                                                }
                                            }
                                        }
                                    }
                                    if deleted_any {
                                        let _ = PostMessageW(
                                            hwnd_notify.0,
                                            WM_REFRESH_SEARCH,
                                            WPARAM(0),
                                            LPARAM(0),
                                        );
                                    }
                                }
                            });
                            s.results.retain(|r| !selected_set.contains(&r.entry.id));
                            let _ = InvalidateRect(hwnd, None, FALSE);
                        } else {
                            if s.selected_clip_ids.is_empty() {
                                if let Some(r) = s.results.get(s.selected) {
                                    if r.entry.source == "CLIPBOARD" {
                                        s.selected_clip_ids.insert(r.entry.id.clone());
                                    }
                                }
                            }
                            if !s.selected_clip_ids.is_empty() {
                                s.delete_confirm = true;
                                let _ = InvalidateRect(hwnd, None, FALSE);
                            }
                        }
                        return LRESULT(0);
                    } else if s.cursor_pos < s.query.len() {
                        s.query.remove(s.cursor_pos);
                        s.selected = 0;
                        s.scroll_offset = 0;
                        kick_debounce(hwnd);
                        reset_cursor_blink(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
                VK_RETURN => {
                    if s.delete_confirm {
                        s.delete_confirm = false;
                        let db_path = s.db_path.clone();
                        let selected_ids: Vec<String> = s.selected_clip_ids.iter().cloned().collect();
                        let selected_set: std::collections::HashSet<String> = selected_ids.iter().cloned().collect();
                        s.selected_clip_ids.clear();
                        let hwnd_notify = SendHwnd(hwnd);
                        std::thread::spawn(move || {
                            let hwnd_notify = hwnd_notify;
                            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                                let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                                let mut deleted_any = false;
                                for id in &selected_ids {
                                    let parts: Vec<&str> = id.split('.').collect();
                                    if let Some(ts_str) = parts.last() {
                                        if let Ok(ts) = ts_str.parse::<i64>() {
                                            if conn.execute("DELETE FROM clipboard_history WHERE timestamp = ?;", [ts]).is_ok() {
                                                deleted_any = true;
                                            }
                                        }
                                    }
                                }
                                if deleted_any {
                                    let _ = PostMessageW(
                                        hwnd_notify.0,
                                        WM_REFRESH_SEARCH,
                                        WPARAM(0),
                                        LPARAM(0),
                                    );
                                }
                            }
                        });
                        s.results.retain(|r| !selected_set.contains(&r.entry.id));
                        let _ = InvalidateRect(hwnd, None, FALSE);
                        return LRESULT(0);
                    }
                    if let Some(ref id) = s.editing_item {
                        let parts: Vec<&str> = id.split('.').collect();
                        if let Some(ts_str) = parts.last() {
                            if let Ok(ts) = ts_str.parse::<i64>() {
                                let db_path = s.db_path.clone();
                                let new_content = s.query.clone();
                                let new_content_for_thread = new_content.clone();
                                let hwnd_notify = SendHwnd(hwnd);
                                std::thread::spawn(move || {
                                    let hwnd_notify = hwnd_notify;
                                    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                                        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                                        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                                        if conn.execute(
                                            "UPDATE clipboard_history SET content = ? WHERE timestamp = ?;",
                                            rusqlite::params![new_content_for_thread, ts],
                                        ).is_ok() {
                                            let _ = PostMessageW(
                                                hwnd_notify.0,
                                                WM_REFRESH_SEARCH,
                                                WPARAM(0),
                                                LPARAM(0),
                                            );
                                        }
                                    }
                                });
                                copy_to_clipboard(hwnd, &new_content);
                            }
                        }
                        s.editing_item = None;
                        s.query = "clip:".to_string();
                        s.cursor_pos = s.query.len();
                        trigger_search(hwnd, s);
                        return LRESULT(0);
                    }
                    if !s.selected_clip_ids.is_empty() {
                        let db_path = s.db_path.clone();
                        let selected_ids: Vec<String> = s.selected_clip_ids.iter().cloned().collect();
                        s.selected_clip_ids.clear();
                        let hwnd_copy = SendHwnd(hwnd);
                        std::thread::spawn(move || {
                            let hwnd_copy = hwnd_copy;
                            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                                let mut contents = Vec::new();
                                let mut timestamps = Vec::new();
                                for id in &selected_ids {
                                    let parts: Vec<&str> = id.split('.').collect();
                                    if let Some(ts_str) = parts.last() {
                                        if let Ok(ts) = ts_str.parse::<i64>() {
                                            timestamps.push(ts);
                                        }
                                    }
                                }
                                timestamps.sort();
                                for ts in timestamps {
                                    if let Ok(content) = conn.query_row(
                                        "SELECT content FROM clipboard_history WHERE timestamp = ?;",
                                        [ts],
                                        |row| row.get::<_, String>(0),
                                    ) {
                                        contents.push(content);
                                    }
                                }
                                if !contents.is_empty() {
                                    let combined = contents.join("\r\n");
                                    copy_to_clipboard(hwnd_copy.0, &combined);
                                }
                            }
                        });
                        do_hide(hwnd, s);
                        return LRESULT(0);
                    }

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
                            s.cursor_pos = s.query.len();
                            s.selected = 0;
                            s.scroll_offset = 0;
                            s.text_selected = false;
                            reset_cursor_blink(hwnd, s);
                            trigger_search(hwnd, s);
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
                        reset_cursor_blink(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
                VK_UP => {
                    if s.selected > 0 {
                        s.selected -= 1;
                        if s.selected < s.scroll_offset {
                            s.scroll_offset = s.selected;
                        }
                        reset_cursor_blink(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
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
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                } else {
                    // Scroll down
                    if s.scroll_offset + VISIBLE_RESULTS < s.results.len() {
                        s.scroll_offset += 1;
                        if s.selected < s.scroll_offset {
                            s.selected = s.scroll_offset;
                        }
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
            }
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            reset_cursor_blink(hwnd, s);
            let my = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            let n = (s.results.len().saturating_sub(s.scroll_offset)).min(VISIBLE_RESULTS);
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
                        s.cursor_pos = s.query.len();
                        s.selected = 0;
                        s.scroll_offset = 0;
                        s.text_selected = false;
                        trigger_search(hwnd, s);
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
                
                let n = (s.results.len().saturating_sub(s.scroll_offset)).min(VISIBLE_RESULTS);
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
                if !s.icon_memory.0.is_null() { let _ = DestroyIcon(s.icon_memory); }
                let _ = DeleteObject(s.font_q);
                let _ = DeleteObject(s.font_n);
                let _ = DeleteObject(s.font_c);
                let _ = DeleteObject(s.font_b);
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
                for &hbmp in s.clipboard_thumbnails.borrow().values() {
                    if !hbmp.0.is_null() {
                        let _ = DeleteObject(hbmp);
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
unsafe fn animate_window(hwnd: HWND, appearing: bool) {
    let sp = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
    if sp.is_null() { return; }
    let s = &mut *sp;

    static mut IN_ANIMATION: bool = false;
    if IN_ANIMATION {
        return;
    }
    IN_ANIMATION = true;

    let start_time = std::time::Instant::now();
    let duration = ANIM_DURATION_SEC;
    let start_p = if appearing { 0.0 } else { 1.0 };

    if appearing {
        s.query.clear();
        s.cursor_pos = 0;
        s.selected = 0;
        s.scroll_offset = 0;
        trigger_search(hwnd, s);

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        s.cx = sw / 2;
        s.cy = (sh as f32 / 2.5) as i32;
        s.last_mouse_x = pt.x;
        s.last_mouse_y = pt.y;

        s.anim = Anim::Appearing { start_time, start_p };

        let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, 0, LWA_COLORKEY | LWA_ALPHA);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(hwnd);
    } else {
        let _ = KillTimer(hwnd, TIMER_DEBOUNCE);
        s.anim = Anim::Hiding { start_time, start_p };
    }

    loop {
        match s.anim {
            Anim::Appearing { .. } if appearing => {}
            Anim::Hiding { .. } if !appearing => {}
            _ => break,
        }

        let elapsed = start_time.elapsed().as_secs_f32();
        let p = if appearing {
            (start_p + elapsed / duration).min(1.0)
        } else {
            (start_p - elapsed / duration).max(0.0)
        };

        let t = ease_out(p);
        let alpha = (t * 255.0) as u8;
        let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, alpha, LWA_COLORKEY | LWA_ALPHA);
        
        let _ = InvalidateRect(hwnd, None, FALSE);
        let _ = UpdateWindow(hwnd);

        let is_finished = if appearing { p >= 1.0 } else { p <= 0.0 };
        if is_finished {
            if appearing {
                s.anim = Anim::Visible;
                let _ = SetForegroundWindow(hwnd);
                let _ = SetFocus(hwnd);
            } else {
                s.anim = Anim::Hidden;
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            break;
        }

        let _ = DwmFlush();

        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
            if msg.message == WM_QUIT {
                IN_ANIMATION = false;
                PostQuitMessage(0);
                return;
            }
        }
    }

    IN_ANIMATION = false;
}

unsafe fn reset_cursor_blink(hwnd: HWND, s: &mut State) {
    s.cursor_visible = true;
    let _ = KillTimer(hwnd, TIMER_CURSOR_BLINK);
    let _ = SetTimer(hwnd, TIMER_CURSOR_BLINK, CURSOR_BLINK_MS, None);
}

unsafe fn do_show(hwnd: HWND, s: &mut State) {
    reset_cursor_blink(hwnd, s);
    animate_window(hwnd, true);
}

unsafe fn start_hide(hwnd: HWND, _s: &mut State) {
    animate_window(hwnd, false);
}

unsafe fn do_hide(hwnd: HWND, s: &mut State) {
    let _ = KillTimer(hwnd, TIMER_DEBOUNCE);
    let _ = KillTimer(hwnd, TIMER_CURSOR_BLINK);
    let _ = ShowWindow(hwnd, SW_HIDE);
    s.anim = Anim::Hidden;
}

unsafe fn kick_debounce(hwnd: HWND) {
    let _ = KillTimer(hwnd, TIMER_DEBOUNCE);
    let _ = SetTimer(hwnd, TIMER_DEBOUNCE, 120, None);
}

unsafe fn trigger_search(_hwnd: HWND, s: &mut State) {
    if s.editing_item.is_some() {
        return;
    }
    s.current_query_id += 1;
    let req = SearchRequest {
        query: s.query.clone(),
        query_id: s.current_query_id,
    };
    if let Some(ref tx) = s.search_tx {
        let _ = tx.send(req);
    }
}

fn ease_out(t: f32) -> f32 { 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(4) }
// fn ease_in(t: f32) -> f32 { t.clamp(0.0, 1.0).powi(4) }

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

fn word_left(s: &str, pos: usize) -> usize {
    let i = pos.min(s.len());
    if i == 0 {
        return 0;
    }

    let mut chars = s[..i].char_indices().rev();
    
    let mut next_char = chars.next();
    while let Some((_, c)) = next_char {
        if c.is_whitespace() {
            next_char = chars.next();
        } else {
            break;
        }
    }
    
    let (idx, c) = match next_char {
        Some(pair) => pair,
        None => return 0,
    };
    
    let start_class = if c.is_alphanumeric() || c == '_' {
        1 // Word
    } else {
        2 // Punctuation
    };
    
    let mut last_idx = idx;
    for (idx, c) in chars {
        let class = if c.is_alphanumeric() || c == '_' {
            1
        } else if c.is_whitespace() {
            break; // Stop at whitespace
        } else {
            2
        };
        
        if class == start_class {
            last_idx = idx;
        } else {
            break; // Stop when class changes
        }
    }
    
    last_idx
}

fn word_right(s: &str, pos: usize) -> usize {
    let len = s.len();
    let i = pos.min(len);
    if i >= len {
        return len;
    }

    let mut chars = s[i..].char_indices();
    let (_, first_char) = chars.next().unwrap();
    
    if first_char.is_whitespace() {
        for (idx, c) in chars {
            if !c.is_whitespace() {
                return i + idx;
            }
        }
        return len;
    }
    
    let start_class = if first_char.is_alphanumeric() || first_char == '_' {
        1
    } else {
        2
    };
    
    let mut next_pos = len;
    let mut chars_loop = s[i..].char_indices();
    let _ = chars_loop.next();
    
    for (idx, c) in chars_loop {
        let class = if c.is_alphanumeric() || c == '_' {
            1
        } else if c.is_whitespace() {
            next_pos = i + idx;
            break;
        } else {
            2
        };
        
        if class != start_class {
            next_pos = i + idx;
            break;
        }
    }
    
    if next_pos < len {
        let follow_chars = s[next_pos..].char_indices();
        for (idx, c) in follow_chars {
            if !c.is_whitespace() {
                return next_pos + idx;
            }
        }
    }
    
    len
}

fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn delete_word_before(s: &mut State) {
    let cur = floor_char_boundary(&s.query, s.cursor_pos);
    let new_pos = word_left(&s.query, cur);
    let rest = s.query[cur..].to_string();
    s.query.truncate(new_pos);
    s.query.push_str(&rest);
    s.cursor_pos = new_pos;
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

unsafe fn get_process_path(pid: u32) -> Option<String> {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::System::Threading::{QueryFullProcessImageNameW, PROCESS_NAME_WIN32};
    use windows::Win32::Foundation::CloseHandle;
    use windows::core::PWSTR;

    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    let res = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut size);
    let _ = CloseHandle(handle);

    if res.is_ok() && size > 0 {
        Some(String::from_utf16_lossy(&buffer[..size as usize]))
    } else {
        None
    }
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

unsafe fn trigger_icon_loading(_hwnd: HWND, s: &mut State) {
    if s.icon_tx.is_none() { return; }
    let tx = s.icon_tx.as_ref().unwrap();
    for res in &s.results {
        let (source, key) = (res.entry.source.as_str(), res.entry.launch_command.clone());
        // For FOLDER source: only load icon if it's a real filesystem path (not virtual folders like bookmarks:)
        let is_real_folder = source == "FOLDER" && !key.ends_with(':');
        // For PROJECT source: load folder icon if the launch_command is a real filesystem path
        let is_real_project = source == "PROJECT" && !key.is_empty() && !key.starts_with("http");
        let is_kill_action = source == "ACTION" && key.starts_with("kill:");
        let needs_icon = (source == "app" || source == "RECENT" || source == "FILE" || source == "CODE" || is_real_folder || is_real_project || is_kill_action)
            && !s.app_icons.contains_key(&key);
        if needs_icon {
            // Placeholder so we don't spawn multiple threads for same path
            s.app_icons.insert(key.clone(), HICON(std::ptr::null_mut()));
            let _ = tx.send(IconRequest {
                key,
                source: source.to_string(),
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

    // Clear background with COLOR_KEY (completely transparent)
    fill(mdc, 0, 0, win_w, win_h, COLOR_KEY);

    // Calculate dynamic shape coordinates
    let p = s.current_p();
    let t = ease_out(p);

    let pill_w = 150;
    let pill_h = 28;
    let pill_y = 8;
    let pill_r = 28;

    let end_w = WIN_W;
    let end_h = s.win_h();
    let end_y = s.cy - end_h / 2;

    let w = (pill_w as f32 + (end_w - pill_w) as f32 * t) as i32;
    let h = (pill_h as f32 + (end_h - pill_h) as f32 * t) as i32;
    let x = (win_w - w) / 2;
    let y = (pill_y as f32 + (end_y - pill_y) as f32 * t) as i32;
    let r = (pill_r as f32 + (12 - pill_r) as f32 * t) as i32;

    // Fill background / Draw Glowing Border around the morphing rounded rect
    let has_results = s.results.len().min(MAX_RESULTS) > 0;
    draw_rounded_border_and_bg(mdc, x, y, w, h, r, has_results && s.anim != Anim::Hidden);

    // Create rounded clipping region matching the inner background area of the morphing shape
    let clip_rgn = CreateRoundRectRgn(x + 1, y + 1, x + w - 1, y + h - 1, r - 1, r - 1);
    let _ = SelectClipRgn(mdc, clip_rgn);

    // ── Search row ────────────────────────────────────────────────────────
    SetBkMode(mdc, TRANSPARENT);

    // Draw Search Icon
    if !s.icon_search.0.is_null() {
        let icon_y = y + (SEARCH_H - 24) / 2;
        let _ = DrawIconEx(mdc, x + PAD_L, icon_y, s.icon_search, 24, 24, 0, HBRUSH(null_mut()), DI_NORMAL);
    }

    // Text / placeholder
    let tx = x + PAD_L + ICON_W + 8;
    let tw = w - (PAD_L + ICON_W + 8) - PAD_L;
    let mut tr = RECT { left: tx, top: y, right: tx + tw, bottom: y + SEARCH_H };

    SelectObject(mdc, s.font_q);
    SetTextColor(mdc, CLR_WHITE);

    if s.query.is_empty() {
        let mut ph: Vec<u16> = "Search Windows settings...".encode_utf16().collect();
        SetTextColor(mdc, CLR_PH);
        let _ = DrawTextW(mdc, &mut ph, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SetTextColor(mdc, CLR_WHITE);
    } else {
        let mut dw_query: Vec<u16> = s.query.encode_utf16().collect();
        let mut text_rect = tr;
        let _ = DrawTextW(mdc, &mut dw_query, &mut text_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    }

    // Draw cursor
    if s.cursor_visible {
        let cur = floor_char_boundary(&s.query, s.cursor_pos);
        let before = &s.query[..cur];
        let dw_before: Vec<u16> = before.encode_utf16().collect();
        let mut size = SIZE::default();
        if !dw_before.is_empty() {
            let _ = GetTextExtentPoint32W(mdc, &dw_before, &mut size);
        }
        let cursor_x = tr.left + size.cx;
        
        let mut dummy_size = SIZE::default();
        let _ = GetTextExtentPoint32W(mdc, &['A' as u16], &mut dummy_size);
        let text_h = dummy_size.cy;
        let cursor_top = tr.top + (tr.bottom - tr.top - text_h) / 2;
        fill(mdc, cursor_x, cursor_top, 2, text_h, CLR_WHITE);
    }

    // ── Results ───────────────────────────────────────────────────────────
    let n = (s.results.len().saturating_sub(s.scroll_offset)).min(VISIBLE_RESULTS);
    if n > 0 {
        fill(mdc, x, y + SEARCH_H, w, 1, CLR_DIV);

        for i in 0..n {
            let res_idx = s.scroll_offset + i;
            let res = &s.results[res_idx];
            let ry = y + SEARCH_H + 1 + i as i32 * RESULT_H;

            if res_idx == s.selected { fill(mdc, x, ry, w, RESULT_H, BG_SEL); }
            if i > 0 { fill(mdc, x + PAD_L, ry, w - PAD_L * 2, 1, CLR_DIV); }

            let cy = ry + (RESULT_H - 40) / 2;

            // Draw Icon
            let mut drawn_custom_thumbnail = false;
            if res.entry.source == "CLIPBOARD" {
                if let Some(path) = res.entry.launch_command.strip_prefix("copy_image:") {
                    let icon_y = ry + (RESULT_H - 32) / 2;
                    let mut cache = s.clipboard_thumbnails.borrow_mut();
                    if let Some(&hbitmap) = cache.get(path) {
                        unsafe { draw_cached_bmp(mdc, x + PAD_L, icon_y, 32, 32, hbitmap); }
                        drawn_custom_thumbnail = true;
                    } else {
                        unsafe {
                            if let Some(hbitmap) = load_bmp_file(path) {
                                draw_cached_bmp(mdc, x + PAD_L, icon_y, 32, 32, hbitmap);
                                cache.insert(path.to_string(), hbitmap);
                                drawn_custom_thumbnail = true;
                            }
                        }
                    }
                }
            }

            if !drawn_custom_thumbnail {
                let icon_to_draw = if res.entry.source == "app" || res.entry.source == "RECENT" || res.entry.source == "FILE" || res.entry.source == "CODE"
                    || (res.entry.source == "ACTION" && res.entry.launch_command.starts_with("kill:")) {
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
                } else if res.entry.source == "MEMORY" {
                    s.icon_memory
                } else {
                    s.icon_control_panel
                };

                if !icon_to_draw.0.is_null() {
                    let icon_y = ry + (RESULT_H - 32) / 2;
                    let _ = unsafe { DrawIconEx(mdc, x + PAD_L, icon_y, icon_to_draw, 32, 32, 0, HBRUSH(null_mut()), DI_NORMAL) };
                }
            }

            // Name
            SelectObject(mdc, s.font_n);
            SetTextColor(mdc, CLR_WHITE);
            let has_selections = !s.selected_clip_ids.is_empty();
            let display_name = if s.selected_clip_ids.contains(&res.entry.id) {
                format!(" [✓] {}", res.entry.control_name)
            } else if has_selections && res.entry.source == "CLIPBOARD" {
                format!(" [ ] {}", res.entry.control_name)
            } else {
                res.entry.control_name.clone()
            };
            let mut name: Vec<u16> = display_name.encode_utf16().collect();
            let mut r = RECT { left: tx, top: cy, right: x + w - 96, bottom: cy + 22 };
            let _ = DrawTextW(mdc, &mut name, &mut r,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

            // Breadcrumb
            SelectObject(mdc, s.font_c);
            SetTextColor(mdc, CLR_GRAY);
            let mut crumb: Vec<u16> = res.entry.breadcrumb_path.encode_utf16().collect();
            let mut r2 = RECT { left: tx, top: cy + 24, right: x + w - 96, bottom: cy + 40 };
            let _ = DrawTextW(mdc, &mut crumb, &mut r2,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

            // Badge
            let badge_source = if res.entry.id.starts_with("clip.pinned.") {
                "pinned_clip"
            } else {
                &res.entry.source
            };
            badge(mdc, s, badge_source, x + w - 88, ry + (RESULT_H - 20) / 2);
        }

        // Draw scrollbar if there are more results than visible
        let total_results = s.results.len();
        if total_results > VISIBLE_RESULTS {
            let track_top = y + SEARCH_H + 8;
            let track_bottom = y + h - 8;
            let track_h = track_bottom - track_top;
            
            // Thumb height proportional to ratio of visible results, capped at min 24px
            let thumb_h = ((VISIBLE_RESULTS as f32 / total_results as f32) * track_h as f32) as i32;
            let thumb_h = thumb_h.max(24);
            
            // Thumb position proportional to scroll_offset
            let max_offset = total_results - VISIBLE_RESULTS;
            let thumb_y = track_top + (s.scroll_offset as f32 / max_offset as f32 * (track_h - thumb_h) as f32) as i32;
            
            // Draw subtle track
            let sb_x = x + w - 10;
            let sb_w = 4;
            fill(mdc, sb_x, track_top, sb_w, track_h, COLORREF(0x00_2A_2A_2A));
            
            // Draw thumb
            fill(mdc, sb_x, thumb_y, sb_w, thumb_h, CLR_GRAY);
        }
    }

    // Draw footer instructions if showing clipboard
    if s.query.starts_with("clip:") || s.query.starts_with("clipboard:") {
        let footer_y = win_h - 24;
        fill(mdc, 0, footer_y, win_w, 24, COLORREF(0x00_15_15_15));
        fill(mdc, 0, footer_y, win_w, 1, CLR_DIV);
        
        if s.delete_confirm {
            badge(mdc, s, "confirm", PAD_L, footer_y + 2);
            SelectObject(mdc, s.font_c);
            SetTextColor(mdc, CLR_GRAY);
            let inst_text = format!(" Press Delete again to delete {} selected items, Escape to cancel", s.selected_clip_ids.len());
            let mut inst_wide: Vec<u16> = inst_text.encode_utf16().collect();
            let mut r_inst = RECT { left: PAD_L + 68, top: footer_y, right: win_w - PAD_L, bottom: win_h };
            let _ = DrawTextW(mdc, &mut inst_wide, &mut r_inst, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        } else {
            SelectObject(mdc, s.font_c);
            SetTextColor(mdc, CLR_GRAY);
            let inst_text = if s.editing_item.is_some() {
                " 📝 Editing snippet: Press Enter to save to database & clipboard, Escape to cancel".to_string()
            } else {
                let sel_count = s.selected_clip_ids.len();
                if sel_count > 0 {
                    format!(" Tab: Deselect  |  Enter: Paste combined ({})  |  Delete: Bulk Delete  |  Ctrl+P: Pin/Unpin", sel_count)
                } else {
                    " Tab: Select  |  Enter: Copy & Paste  |  Ctrl+P: Pin/Unpin  |  Ctrl+E: Edit  |  Delete: Delete".to_string()
                }
            };
            let mut inst_wide: Vec<u16> = inst_text.encode_utf16().collect();
            let mut r_inst = RECT { left: PAD_L, top: footer_y, right: win_w - PAD_L, bottom: win_h };
            let _ = DrawTextW(mdc, &mut inst_wide, &mut r_inst, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
    }

    // Restore clipping
    let _ = SelectClipRgn(mdc, HRGN(null_mut()));
    let _ = DeleteObject(clip_rgn);

    let _ = BitBlt(hdc, 0, 0, win_w, win_h, mdc, 0, 0, SRCCOPY);
    let _ = SelectObject(mdc, old);
    let _ = DeleteObject(bmp);
    let _ = DeleteDC(mdc);
    let _ = EndPaint(hwnd, &ps);
}

unsafe fn fill(hdc: HDC, x: i32, y: i32, w: i32, h: i32, c: COLORREF) {
    let br = CreateSolidBrush(c);
    let _ = FillRect(hdc, &RECT { left: x, top: y, right: x + w, bottom: y + h }, br);
    let _ = DeleteObject(br);
}

unsafe fn draw_rounded_border_and_bg(hdc: HDC, x: i32, y: i32, w: i32, h: i32, r: i32, gradient: bool) {
    if gradient {
        // Create a rounded region for the border
        let rgn = CreateRoundRectRgn(x, y, x + w + 1, y + h + 1, r, r);
        let _ = SelectClipRgn(hdc, rgn);
        
        // Draw horizontal gradient over the outer bounds
        let vertices = [
            TRIVERTEX {
                x,
                y,
                Red: 0x0000,
                Green: 0xb400,
                Blue: 0xdb00,
                Alpha: 0x0000,
            },
            TRIVERTEX {
                x: x + w,
                y: y + h,
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
        let _ = GradientFill(hdc, &vertices, g_rect.as_ptr() as *const _, 1, GRADIENT_FILL(0));
        
        // Restore clipping
        let _ = SelectClipRgn(hdc, HRGN(null_mut()));
        let _ = DeleteObject(rgn);
        
        // Draw the inner background
        fill_rounded(hdc, x + 1, y + 1, w - 2, h - 2, r - 1, BG);
    } else {
        // Draw subtle solid gray border
        fill_rounded(hdc, x, y, w, h, r, CLR_DIV);
        fill_rounded(hdc, x + 1, y + 1, w - 2, h - 2, r - 1, BG);
    }
}

unsafe fn badge(hdc: HDC, s: &State, source: &str, x: i32, y: i32) {
    let src_lc = source.to_lowercase();
    let (label, bg_color, tx_color) = if src_lc == "live" {
        ("LIVE", COLORREF(0x00_1F_A6_0A), CLR_WHITE)
    } else if src_lc == "project" {
        ("PROJECT", COLORREF(0x00_B5_25_9E), CLR_WHITE)
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
    } else if src_lc == "pinned_clip" {
        ("PINNED", COLORREF(0x00_00_C5_D6), CLR_WHITE)
    } else if src_lc == "confirm" {
        ("CONFIRM", COLORREF(0x00_00_00_00), CLR_WHITE)
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
    } else if src_lc == "memory" {
        ("MEMORY", COLORREF(0x00_0B_5C_2C), CLR_WHITE)
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
            return String::from_utf16_lossy(&buffer[..size as usize]);
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

unsafe fn start_timeline_tracker(db_path: std::path::PathBuf, launcher_hwnd: SendHwnd) {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW, PROCESS_NAME_WIN32};
    use windows::Win32::Foundation::{CloseHandle, BOOL, HWND};
    use windows::core::PWSTR;

    let launcher_hwnd = launcher_hwnd.0;

    let mut last_hwnd = HWND::default();
    let mut last_title = String::new();
    let mut last_app = String::new();
    let mut focus_start = std::time::Instant::now();
    let mut focus_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            continue;
        }

        // Skip our own launcher window
        if fg == launcher_hwnd {
            continue;
        }

        // Get window title
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(fg, &mut title_buf);
        let title = if len > 0 {
            String::from_utf16_lossy(&title_buf[..len as usize])
        } else {
            String::new()
        };

        // Get app name (process filename)
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        
        let app = if pid != 0 {
            if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, BOOL(0), pid) {
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
                    String::from_utf16_lossy(&buffer[..size as usize])
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        // Check if focus changed
        if fg != last_hwnd || title != last_title || app != last_app {
            let duration = focus_start.elapsed().as_secs() as i64;
            if (!last_title.is_empty() || !last_app.is_empty()) && duration >= 1 {
                log_timeline_event(&db_path, focus_timestamp, duration, &last_app, &last_title);
            }
            last_hwnd = fg;
            last_title = title;
            last_app = app;
            focus_start = std::time::Instant::now();
            focus_timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
        }
    }
}

fn log_timeline_event(db_path: &std::path::Path, timestamp: i64, duration: i64, app_name: &str, window_title: &str) {
    if let Ok(conn) = rusqlite::Connection::open(db_path) {
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute(
            "INSERT INTO timeline_events (timestamp, duration, app_name, window_title) VALUES (?, ?, ?, ?);",
            rusqlite::params![timestamp, duration, app_name, window_title],
        );
        let _ = conn.execute(
            "DELETE FROM timeline_events WHERE id NOT IN (SELECT id FROM timeline_events ORDER BY timestamp DESC LIMIT 10000);",
            [],
        );
    }
}

unsafe fn load_bmp_file(path: &str) -> Option<HBITMAP> {
    use windows::Win32::UI::WindowsAndMessaging::{LoadImageW, IMAGE_BITMAP, LR_LOADFROMFILE, LR_CREATEDIBSECTION};
    use windows::core::PCWSTR;

    let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let h_img = LoadImageW(
        None,
        PCWSTR(wide_path.as_ptr()),
        IMAGE_BITMAP,
        0,
        0,
        LR_LOADFROMFILE | LR_CREATEDIBSECTION,
    );

    h_img.ok().map(|h| HBITMAP(h.0))
}

unsafe fn draw_cached_bmp(hdc: HDC, x: i32, y: i32, w: i32, h: i32, hbitmap: HBITMAP) {
    use windows::Win32::Graphics::Gdi::{CreateCompatibleDC, DeleteDC, SelectObject, StretchBlt, GetObjectW, BITMAP, COLORONCOLOR, SetStretchBltMode};

    let mem_dc = CreateCompatibleDC(hdc);
    if !mem_dc.is_invalid() {
        let mut bmp: BITMAP = std::mem::zeroed();
        let size = std::mem::size_of::<BITMAP>() as i32;
        if GetObjectW(hbitmap, size, Some(&mut bmp as *mut BITMAP as *mut _)) != 0 {
            let old_obj = SelectObject(mem_dc, hbitmap);
            let old_mode = SetStretchBltMode(hdc, COLORONCOLOR);
            let _ = StretchBlt(
                hdc,
                x,
                y,
                w,
                h,
                mem_dc,
                0,
                0,
                bmp.bmWidth,
                bmp.bmHeight,
                SRCCOPY,
            );
            let _ = SetStretchBltMode(hdc, STRETCH_BLT_MODE(old_mode));
            let _ = SelectObject(mem_dc, old_obj);
        }
        let _ = DeleteDC(mem_dc);
    }
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
