#![windows_subsystem = "windows"]

mod launcher;
mod search;
mod indexer;
mod browser_indexer;
mod git_indexer;
mod voice;
mod ai;
mod markdown;

use std::ptr::null_mut;
use std::os::windows::process::CommandExt;
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
const RESULT_H: i32 = 72;
const MAX_RESULTS: usize = 30;
const VISIBLE_RESULTS: usize = 5;
const PAD_L: i32 = 24;
const ICON_W: i32 = 36;
const BADGE_W: i32 = 54;
const BADGE_H: i32 = 18;

// ── Win32 IDs ─────────────────────────────────────────────────────────────────
const HOTKEY_ID: i32 = 1;
const HOTKEY_VOICE_ID: i32 = 2;
const TIMER_DEBOUNCE: usize = 1;
const TIMER_CURSOR_BLINK: usize = 2;
const TIMER_VOICE_AUTOEXEC: usize = 3;
const TIMER_VOICE_ANIM: usize = 4;
const TIMER_AI_ANIM: usize = 5;
const CURSOR_BLINK_MS: u32 = 530;
const WM_ICON_LOADED: u32 = WM_USER + 1;
const WM_ENGINE_READY: u32 = WM_USER + 2;
const WM_SEARCH_RESULTS: u32 = WM_USER + 3;
const WM_START_EDITING: u32 = WM_USER + 4;
const WM_REFRESH_SEARCH: u32 = WM_USER + 5;
const WM_VOICE_QUERY_READY: u32 = WM_USER + 101;
const WM_AI_RESULT: u32 = WM_USER + 6;
// Hermes Runs API: a tool needs approval (lparam = boxed HermesApproval).
const WM_HERMES_APPROVAL: u32 = WM_USER + 7;
// Hermes Runs API: streaming output progress (lparam = boxed String).
const WM_AI_PROGRESS: u32 = WM_USER + 8;

// AI answer panel height (below the search bar) when showing an AI response.
const AI_PANEL_H: i32 = 360;

struct SearchRequest {
    query: String,
    query_id: usize,
}
// ── Animation ─────────────────────────────────────────────────────────────────
// const ANIM_TICK_MS: u32 = 1;
const ANIM_DURATION_SEC: f32 = 0.115; // 115ms
// const MAX_ALPHA: u8 = 255;

// ── Genie Morph Dimensions ────────────────────────────────────────────────────
// const PILL_H: i32 = 12; // Starting height at top center

// ── Colors (COLORREF = 0x00BBGGRR) ───────────────────────────────────────────
const BG: COLORREF        = COLORREF(0x00_1F_1D_1C);
const BG_SEL: COLORREF    = COLORREF(0x00_36_31_2F);
const CLR_DIV: COLORREF   = COLORREF(0x00_35_31_30);
const CLR_WHITE: COLORREF = COLORREF(0x00_FF_FF_FF);
const CLR_GRAY: COLORREF  = COLORREF(0x00_A7_A1_9F);
const CLR_PH: COLORREF    = COLORREF(0x00_70_6A_68);
const CLR_BDGBG: COLORREF = COLORREF(0x00_38_34_33);
const CLR_BDGTX: COLORREF = COLORREF(0x00_B4_AD_AA);
const COLOR_KEY: COLORREF = COLORREF(0x00_12_34_56);

#[derive(Debug, Clone, PartialEq)]
enum FormState {
    None,
    CreateSnippetName,
    CreateSnippetContent { name: String },
    CreateSnippetKeyword { name: String, content: String },
    CreateQuicklinkName,
    CreateQuicklinkUrl { name: String },
    CreateQuicklinkKeyword { name: String, url: String },
}

// ── App state ─────────────────────────────────────────────────────────────────
struct State {
    search_tx: Option<std::sync::mpsc::Sender<SearchRequest>>,
    icon_tx: Option<std::sync::mpsc::Sender<IconRequest>>,
    current_query_id: usize,
    db_path: std::path::PathBuf,
    query: String,
    cursor_pos: usize,
    chat_input: String,
    chat_cursor_pos: usize,
    chat_input_active: bool,
    results: Vec<SearchResult>,
    selected: usize,
    anim: Anim,
    cx: i32,
    cy: i32,
    font_q: HFONT,
    font_n: HFONT,
    font_c: HFONT,
    font_b: HFONT,
    font_mic: HFONT,
    font_code: HFONT, // monospace for inline code / code blocks
    font_h: HFONT,    // bold larger font for markdown headings
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
    submenu_active: bool,
    submenu_selected: usize,
    // Voice activation
    voice_listening: bool,   // true = currently recording query
    voice_triggered: bool,   // launcher opened via voice (auto-execute on result)
    voice_pending_exec: bool, // true = waiting for search results to auto-execute
    voice_dot_tick: u32,     // animation frame counter for pulsing mic dot
    voice_exec_deadline: Option<std::time::Instant>, // when the auto-exec countdown fires
    form_state: FormState,   // Phase 2 Quicklinks & Snippets creation form state
    color_picker_active: bool,
    color_picker_mx: i32,
    color_picker_my: i32,
    prev_foreground: HWND,  // Window that had focus before launcher appeared (for snippet auto-paste)
    // AI answer panel
    ai_pending: bool,            // true while waiting on the AI response
    ai_answer: Option<String>,   // the response text to render
    ai_title: String,            // command label shown above the answer
    ai_scroll: i32,              // vertical pixel scroll offset in the answer panel
    ai_follow_bottom: bool,      // true = keep the latest message pinned to the bottom (auto-scroll)
    ai_content_height: std::cell::Cell<i32>, // cached total rendered AI height (for max_scroll in input handlers)
    ai_view_height: std::cell::Cell<i32>,    // cached viewport height (content_bottom - content_top)
    ai_tick: u32,                 // lightweight activity indicator while AI is running
    active_chat_id: Option<i64>, // persistent chat thread ID in ai_chats table
    // Hermes Runs API: a pending tool approval (None = nothing to approve).
    hermes_approval: Option<ai::HermesApproval>,
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
        if self.ai_pending || self.ai_answer.is_some() {
            return SEARCH_H + 1 + AI_PANEL_H;
        }
        if self.form_state != FormState::None {
            return SEARCH_H + 24;
        }
        let n = self.results.len().min(VISIBLE_RESULTS) as i32;
        if n == 0 {
            SEARCH_H
        } else {
            let base_h = SEARCH_H + 1 + n * RESULT_H;
            if self.query.starts_with("clip:") || self.query.starts_with("clipboard:") {
                base_h + 24
            } else {
                base_h + 12
            }
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
    accept_speech_privacy();
    register_startup();
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE);
    }

    unsafe { run(); }
}

// Accept the Windows "Online speech recognition" privacy policy so the Dictation
// recognizer can run. Without this, RecognizeAsync fails with 0x80045509
// ("speech privacy policy was not accepted"). This is the same flag the
// Settings → Privacy → Speech toggle sets; the user can turn it back off there.
fn accept_speech_privacy() {
    use windows::Win32::System::Registry::*;
    use windows::core::PCWSTR;
    unsafe {
        let subkey: Vec<u16> =
            "Software\\Microsoft\\Speech_OneCore\\Settings\\OnlineSpeechPrivacy\0"
                .encode_utf16().collect();
        let value_name: Vec<u16> = "HasAccepted\0".encode_utf16().collect();
        let mut hkey = HKEY::default();
        // RegCreateKeyW creates the subkey if missing, or opens it if it exists.
        let r = RegCreateKeyW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()), &mut hkey);
        if r.is_ok() {
            let data: u32 = 1;
            let _ = RegSetValueExW(
                hkey,
                PCWSTR(value_name.as_ptr()),
                0,
                REG_DWORD,
                Some(&data.to_ne_bytes()),
            );
            let _ = RegCloseKey(hkey);
        }
    }
}

fn register_startup() {
    // Add to HKCU Run so it launches on login and listens for wake words
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy().to_string();
        let _ = (|| -> Result<(), Box<dyn std::error::Error>> {
            let hkcu = windows::Win32::System::Registry::HKEY_CURRENT_USER;
            let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
                .encode_utf16().collect();
            let value_name: Vec<u16> = "OpenSearchOS\0".encode_utf16().collect();
            let exe_wide: Vec<u16> = format!("{}\0", exe_str).encode_utf16().collect();
            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            unsafe {
                let err = windows::Win32::System::Registry::RegOpenKeyExW(
                    hkcu,
                    windows::core::PCWSTR(subkey.as_ptr()),
                    0,
                    windows::Win32::System::Registry::KEY_SET_VALUE,
                    &mut hkey,
                );
                if err.is_err() { return Err("open key failed".into()); }
                let _ = windows::Win32::System::Registry::RegSetValueExW(
                    hkey,
                    windows::core::PCWSTR(value_name.as_ptr()),
                    0,
                    windows::Win32::System::Registry::REG_SZ,
                    Some(std::slice::from_raw_parts(
                        exe_wide.as_ptr() as *const u8,
                        (exe_wide.len() - 1) * 2,
                    )),
                );
                let _ = windows::Win32::System::Registry::RegCloseKey(hkey);
            }
            Ok(())
        })();
    }
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

    let mic_face: Vec<u16> = "Segoe MDL2 Assets\0".encode_utf16().collect();
    let font_mic = CreateFontW(
        -20, 0, 0, 0, 400, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32, (DEFAULT_PITCH.0 | FF_SWISS.0) as u32, PCWSTR(mic_face.as_ptr()),
    );

    // Monospace + bold fonts for markdown code blocks and headings in the AI panel.
    let mono_face: Vec<u16> = "Consolas\0".encode_utf16().collect();
    let font_code = CreateFontW(
        -15, 0, 0, 0, 400, 0, 0, 0,
        DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32, (FIXED_PITCH.0 | FF_MODERN.0) as u32, PCWSTR(mono_face.as_ptr()),
    );
    // Heading font: slightly larger, bold, same Segoe UI Variable face.
    let font_h = mk_font(-22, 700);

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
        chat_input: String::new(),
        chat_cursor_pos: 0,
        chat_input_active: false,
        results: vec![],
        selected: 0,
        anim: Anim::Hidden,
        cx: sw / 2,
        cy: sh / 3,
        font_q: mk_font(-19, 400),
        font_n: mk_font(-17, 600),
        font_c: mk_font(-16, 400),
        font_b: mk_font(-11, 600),
        font_mic,
        font_code,
        font_h,
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
        submenu_active: false,
        submenu_selected: 0,
        voice_listening: false,
        voice_triggered: false,
        voice_pending_exec: false,
        voice_dot_tick: 0,
        voice_exec_deadline: None,
        form_state: FormState::None,
        color_picker_active: false,
        color_picker_mx: 0,
        color_picker_my: 0,
        prev_foreground: HWND(null_mut()),
        ai_pending: false,
        ai_answer: None,
        ai_title: String::new(),
        ai_scroll: 0,
        ai_follow_bottom: true,
        ai_content_height: std::cell::Cell::new(0),
        ai_view_height: std::cell::Cell::new(0),
        ai_tick: 0,
        active_chat_id: None,
        hermes_approval: None,
    });

    // Spawn background Hermes gateway status checker and auto-starter
    std::thread::spawn(|| {
        // Quick initial check and start if not running
        let running = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8642".parse().unwrap(),
            std::time::Duration::from_millis(500)
        ).is_ok();
        if !running {
            ai::start_hermes_gateway_daemon();
        }

        loop {
            let running = std::net::TcpStream::connect_timeout(
                &"127.0.0.1:8642".parse().unwrap(),
                std::time::Duration::from_millis(500)
            ).is_ok();
            ai::HERMES_GATEWAY_RUNNING.store(running, std::sync::atomic::Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });

    if let Ok(cfg) = ai::get_config() {
        configure_hermes_llm(&cfg.endpoint, &cfg.model, &cfg.api_key);
    }

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
                let file_icon_path = icon_file_path(&req.source, &req.key);
                let hicon = if let Some(path) = file_icon_path {
                    get_file_icon(&path)
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

    // Ctrl+Shift+Space starts voice dictation. (Ctrl+Alt is AltGr on many layouts and
    // gets eaten, so it's deliberately avoided.) Non-fatal: the launcher works without it.
    if RegisterHotKey(hwnd, HOTKEY_VOICE_ID, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, VK_SPACE.0 as u32).is_err() {
        voice::log("voice hotkey Ctrl+Shift+Space registration FAILED (already in use?)");
    } else {
        voice::log("voice hotkey Ctrl+Shift+Space registered");
    }

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).as_bool() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
    let _ = UnregisterHotKey(hwnd, HOTKEY_VOICE_ID);
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

        WM_SETCURSOR => {
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{LoadCursorW, SetCursor, IDC_ARROW, IDC_CROSS};
                use windows::Win32::Foundation::HINSTANCE;
                let idc = if !sp.is_null() && (*sp).color_picker_active {
                    IDC_CROSS
                } else {
                    IDC_ARROW
                };
                if let Ok(cursor) = LoadCursorW(HINSTANCE(std::ptr::null_mut()), idc) {
                    SetCursor(cursor);
                    return LRESULT(1);
                }
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }

        WM_HOTKEY if wp.0 as i32 == HOTKEY_ID => {
            let s = &mut *sp;
            match s.anim {
                Anim::Hidden | Anim::Hiding { .. } => do_show(hwnd, s),
                _ => start_hide(hwnd, s),
            }
            LRESULT(0)
        }

        WM_HOTKEY if wp.0 as i32 == HOTKEY_VOICE_ID => {
            if !sp.is_null() {
                start_voice_capture(hwnd, &mut *sp);
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
                // Don't dismiss while a voice flow is mid-setup — focus briefly bounces
                // when the launcher is summoned from the background.
                if s.voice_triggered || s.voice_pending_exec {
                    return LRESULT(0);
                }
                if !matches!(s.anim, Anim::Hidden | Anim::Hiding { .. }) {
                    start_hide(hwnd, s);
                }
            }
            LRESULT(0)
        }

        WM_ACTIVATEAPP | WM_ACTIVATE => {
            if !sp.is_null() {
                let s = &mut *sp;
                let app_inactive = msg == WM_ACTIVATEAPP && wp.0 == 0;
                let window_inactive = msg == WM_ACTIVATE && (wp.0 & 0xffff) == 0;
                if (app_inactive || window_inactive)
                    && !s.voice_triggered
                    && !s.voice_pending_exec
                    && !matches!(s.anim, Anim::Hidden | Anim::Hiding { .. })
                {
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
                    if s.voice_pending_exec && !s.results.is_empty() && s.voice_exec_deadline.is_none() {
                        // Results are in. Show them and count down ~3.5s before executing,
                        // so the user can press Esc to cancel or arrow/type to take over.
                        let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
                        let _ = SetTimer(hwnd, TIMER_VOICE_AUTOEXEC, 3500, None);
                        let _ = SetTimer(hwnd, TIMER_VOICE_ANIM, 100, None); // repaint countdown
                        s.voice_exec_deadline =
                            Some(std::time::Instant::now() + std::time::Duration::from_millis(3500));
                    }
                    // Clear stale WINDOW icon cache when new window results arrive
                    if s.results.iter().any(|r| r.entry.source == "WINDOW") {
                        s.app_icons.retain(|k, _| !k.starts_with("window:"));
                    }
                    trigger_icon_loading(hwnd, s);
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
            }
            LRESULT(0)
        }

        WM_AI_RESULT => {
            let payload = unsafe { *Box::from_raw(lp.0 as *mut (bool, String)) };
            if !sp.is_null() {
                let s = &mut *sp;
                let target_chat = wp.0 as i64;
                let active_chat = s.active_chat_id.unwrap_or(0);
                if target_chat == active_chat && (active_chat != 0 || s.ai_pending) {
                    let (ok, text) = payload;
                    let _ = KillTimer(hwnd, TIMER_AI_ANIM);
                    s.ai_pending = false;
                    s.ai_tick = 0;
                    s.ai_scroll = 0;
                    s.ai_follow_bottom = true;
                    s.ai_answer = Some(if ok { text } else { format!("⚠ {}", text) });
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
            }
            LRESULT(0)
        }

        WM_AI_PROGRESS => {
            if lp.0 != 0 {
                let text = unsafe { *Box::from_raw(lp.0 as *mut String) };
                if !sp.is_null() {
                    let s = &mut *sp;
                    let target_chat = wp.0 as i64;
                    let active_chat = s.active_chat_id.unwrap_or(0);
                    if target_chat == active_chat && (active_chat != 0 || s.ai_pending) {
                        s.ai_answer = Some(text);
                        s.ai_follow_bottom = true;
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
            }
            LRESULT(0)
        }

        WM_HERMES_APPROVAL => {
            if lp.0 != 0 {
                let approval = unsafe { *Box::from_raw(lp.0 as *mut ai::HermesApproval) };
                if !sp.is_null() {
                    let s = &mut *sp;
                    let target_chat = wp.0 as i64;
                    let active_chat = s.active_chat_id.unwrap_or(0);
                    if target_chat == active_chat && (active_chat != 0 || s.ai_pending) {
                        s.hermes_approval = Some(approval);
                        s.ai_follow_bottom = true;
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
            }
            LRESULT(0)
        }

        WM_VOICE_QUERY_READY => {
            if sp.is_null() {
                if wp.0 == 1 && lp.0 != 0 {
                    let _ = unsafe { Box::from_raw(lp.0 as *mut String) };
                }
                return LRESULT(0);
            }
            let s = &mut *sp;
            let text = if wp.0 == 1 && lp.0 != 0 {
                let text_box = unsafe { Box::from_raw(lp.0 as *mut String) };
                *text_box
            } else {
                String::new()
            };
            if s.voice_listening {
                s.voice_listening = false;
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let text = text.trim().to_string();
                if !text.is_empty() {
                    // Query already normalized in voice.rs. Type it out, search, and let
                    // WM_SEARCH_RESULTS arm the ~3.5s "Esc to cancel" auto-exec countdown.
                    s.query = text;
                    s.cursor_pos = s.query.len();
                    s.selected = 0;
                    s.scroll_offset = 0;
                    s.voice_pending_exec = s.voice_triggered;
                    s.voice_exec_deadline = None;
                    reset_cursor_blink(hwnd, s);
                    trigger_search(hwnd, s);
                } else {
                    s.voice_triggered = false;
                    s.voice_pending_exec = false;
                }
                let _ = InvalidateRect(hwnd, None, FALSE);
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
                TIMER_VOICE_ANIM => {
                    s.voice_dot_tick = (s.voice_dot_tick + 1) % 100;
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
                TIMER_AI_ANIM => {
                    s.ai_tick = (s.ai_tick + 1) % 60;
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
                TIMER_VOICE_AUTOEXEC => {
                    let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
                    let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                    s.voice_exec_deadline = None;
                    if s.voice_triggered || s.voice_pending_exec {
                        s.voice_triggered = false;
                        s.voice_pending_exec = false;
                        if !s.results.is_empty() {
                            execute_selected(hwnd, s);
                        }
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_CHAR => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            if s.form_state != FormState::None {
                if let Some(c) = char::from_u32(wp.0 as u32) {
                    if !c.is_control() {
                        if s.text_selected {
                            s.query.clear();
                            s.cursor_pos = 0;
                            s.text_selected = false;
                        }
                        s.query.insert(s.cursor_pos, c);
                        s.cursor_pos += c.len_utf8();
                        reset_cursor_blink(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
                return LRESULT(0);
            }
            if s.voice_listening {
                s.voice_listening = false;
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
            if s.voice_triggered {
                s.voice_triggered = false;
                s.voice_pending_exec = false;
                s.voice_exec_deadline = None;
                let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
            s.submenu_active = false;
            if let Some(c) = char::from_u32(wp.0 as u32) {
                if !c.is_control() {
                    if s.chat_input_active && s.ai_answer.is_some() {
                        s.chat_input.insert(s.chat_cursor_pos, c);
                        s.chat_cursor_pos += c.len_utf8();
                        reset_cursor_blink(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                        return LRESULT(0);
                    }
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
            let ctrl_down = (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;

            // A Hermes tool-approval is awaiting a decision. Intercept the
            // keys that resolve it before any other handling.
            if s.hermes_approval.is_some() && !ctrl_down {
                match vk {
                    VK_RETURN => { resolve_current_approval(hwnd, s, true); return LRESULT(0); }
                    VK_ESCAPE => { 
                        close_ai_panel(hwnd, s);
                        start_hide(hwnd, s);
                        return LRESULT(0); 
                    }
                    _ => {}
                }
                if let Some(c) = char::from_u32(wp.0 as u32) {
                    match c.to_ascii_lowercase() {
                        'a' => { resolve_current_approval(hwnd, s, true); return LRESULT(0); }
                        'd' => { resolve_current_approval(hwnd, s, false); return LRESULT(0); }
                        'v' => {
                            ai::ALWAYS_APPROVE.store(true, std::sync::atomic::Ordering::Relaxed);
                            if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
                                let _ = conn.execute("INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('always_approve', '1');", []);
                            }
                            resolve_current_approval(hwnd, s, true);
                            return LRESULT(0);
                        }
                        _ => {}
                    }
                }
            }

            if s.color_picker_active {
                if vk == VK_ESCAPE {
                    stop_color_picker(hwnd, s);
                }
                return LRESULT(0);
            }

            // AI answer panel captures keys: Esc/Backspace closes, Enter submits follow-up.
            if s.ai_pending {
                match vk {
                    VK_ESCAPE => { close_ai_panel(hwnd, s); start_hide(hwnd, s); return LRESULT(0); },
                    VK_DOWN => { ai_scroll_down(s, 40); let _ = InvalidateRect(hwnd, None, FALSE); }
                    VK_UP => { ai_scroll_up(s, 40); let _ = InvalidateRect(hwnd, None, FALSE); }
                    _ => {}
                }
            }

            if s.ai_answer.is_some() {
                if s.chat_input_active {
                    if ctrl_down {
                        match vk.0 as u32 {
                            0x43 => {
                                if !s.chat_input.is_empty() {
                                    copy_to_clipboard(hwnd, &s.chat_input);
                                }
                                return LRESULT(0);
                            }
                            0x56 => {
                                if let Some(text) = paste_from_clipboard(hwnd) {
                                    let clean_text: String = text.chars().filter(|c| !c.is_control()).collect();
                                    s.chat_input.insert_str(s.chat_cursor_pos, &clean_text);
                                    s.chat_cursor_pos += clean_text.len();
                                    reset_cursor_blink(hwnd, s);
                                    let _ = InvalidateRect(hwnd, None, FALSE);
                                }
                                return LRESULT(0);
                            }
                            _ => {}
                        }
                    }
                    match vk {
                        VK_ESCAPE => {
                            close_ai_panel(hwnd, s);
                            start_hide(hwnd, s);
                            return LRESULT(0);
                        }
                        VK_BACK => {
                            if s.chat_cursor_pos > 0 {
                                let mut p = s.chat_cursor_pos - 1;
                                while p > 0 && !s.chat_input.is_char_boundary(p) {
                                    p -= 1;
                                }
                                s.chat_input.remove(p);
                                s.chat_cursor_pos = p;
                                reset_cursor_blink(hwnd, s);
                                let _ = InvalidateRect(hwnd, None, FALSE);
                            }
                            return LRESULT(0);
                        }
                        VK_LEFT => {
                            if ctrl_down {
                                s.chat_cursor_pos = word_left(&s.chat_input, s.chat_cursor_pos);
                            } else if s.chat_cursor_pos > 0 {
                                let mut p = s.chat_cursor_pos - 1;
                                while p > 0 && !s.chat_input.is_char_boundary(p) {
                                    p -= 1;
                                }
                                s.chat_cursor_pos = p;
                            }
                            reset_cursor_blink(hwnd, s);
                            let _ = InvalidateRect(hwnd, None, FALSE);
                            return LRESULT(0);
                        }
                        VK_RIGHT => {
                            if ctrl_down {
                                s.chat_cursor_pos = word_right(&s.chat_input, s.chat_cursor_pos);
                            } else if s.chat_cursor_pos < s.chat_input.len() {
                                let mut p = s.chat_cursor_pos + 1;
                                while p < s.chat_input.len() && !s.chat_input.is_char_boundary(p) {
                                    p += 1;
                                }
                                s.chat_cursor_pos = p;
                            }
                            reset_cursor_blink(hwnd, s);
                            let _ = InvalidateRect(hwnd, None, FALSE);
                            return LRESULT(0);
                        }
                        VK_RETURN => {
                            let msg = s.chat_input.trim().to_string();
                            if !msg.is_empty() {
                                s.chat_input.clear();
                                s.chat_cursor_pos = 0;
                                start_follow_up_chat(hwnd, s, msg);
                            }
                            return LRESULT(0);
                        }
                        VK_DOWN => { ai_scroll_down(s, 40); let _ = InvalidateRect(hwnd, None, FALSE); return LRESULT(0); }
                        VK_UP => { ai_scroll_up(s, 40); let _ = InvalidateRect(hwnd, None, FALSE); return LRESULT(0); }
                        _ => {}
                    }
                }
                if ctrl_down && vk.0 as u32 == 0x43 { // Ctrl+C
                    if let Some(ans) = &s.ai_answer {
                        copy_to_clipboard(hwnd, ans);
                    }
                    return LRESULT(0);
                }
                match vk {
                    VK_ESCAPE => {
                        close_ai_panel(hwnd, s);
                        start_hide(hwnd, s);
                        return LRESULT(0);
                    }
                    VK_BACK => {
                        if s.query.is_empty() {
                            close_ai_panel(hwnd, s);
                            return LRESULT(0);
                        }
                        // Let it fall through to normal backspace handling if there is text to edit!
                    }
                    VK_RETURN => {
                        let q_trim = s.query.trim().to_string();
                        if q_trim.is_empty() {
                            if let Some(ans) = s.ai_answer.clone() {
                                copy_to_clipboard(hwnd, &ans);
                            }
                            close_ai_panel(hwnd, s);
                        } else {
                            start_follow_up_chat(hwnd, s, q_trim);
                        }
                        return LRESULT(0);
                    }
                    VK_DOWN => { ai_scroll_down(s, 40); let _ = InvalidateRect(hwnd, None, FALSE); return LRESULT(0); }
                    VK_UP => { ai_scroll_up(s, 40); let _ = InvalidateRect(hwnd, None, FALSE); return LRESULT(0); }
                    _ => {} // Let other keys fall through to let user type!
                }
            }

            if s.form_state != FormState::None {
                match vk {
                    VK_ESCAPE => {
                        s.form_state = FormState::None;
                        s.query.clear();
                        s.cursor_pos = 0;
                        s.results.clear();
                        s.selected = 0;
                        s.scroll_offset = 0;
                        reset_cursor_blink(hwnd, s);
                        trigger_search(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                        return LRESULT(0);
                    }
                    VK_RETURN => {
                        handle_form_enter(hwnd, s);
                        return LRESULT(0);
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
                        reset_cursor_blink(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                        return LRESULT(0);
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
                        return LRESULT(0);
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
                        return LRESULT(0);
                    }
                    _ => {
                        if ctrl_down {
                            match vk.0 as u32 {
                                0x41 => { // Ctrl + A
                                    if !s.query.is_empty() {
                                        s.text_selected = true;
                                        let _ = InvalidateRect(hwnd, None, FALSE);
                                    }
                                    return LRESULT(0);
                                }
                                0x43 => { // Ctrl + C
                                    if !s.query.is_empty() {
                                        copy_to_clipboard(hwnd, &s.query);
                                    }
                                    return LRESULT(0);
                                }
                                0x56 => { // Ctrl + V
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
                                        reset_cursor_blink(hwnd, s);
                                        let _ = InvalidateRect(hwnd, None, FALSE);
                                    }
                                    return LRESULT(0);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                return LRESULT(0);
            }
            if s.voice_listening {
                s.voice_listening = false;
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
            if s.voice_triggered {
                s.voice_triggered = false;
                s.voice_pending_exec = false;
                s.voice_exec_deadline = None;
                let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
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
                    if s.submenu_active {
                        s.submenu_active = false;
                        s.submenu_selected = 0;
                        let _ = InvalidateRect(hwnd, None, FALSE);
                        return LRESULT(0);
                    }
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
                    } else if !s.query.is_empty() {
                        s.query.clear();
                        s.cursor_pos = 0;
                        s.results.clear();
                        s.selected = 0;
                        s.scroll_offset = 0;
                        trigger_search(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    } else {
                        start_hide(hwnd, s);
                    }
                }
                VK_LEFT => {
                    if s.submenu_active {
                        s.submenu_active = false;
                        s.submenu_selected = 0;
                        let _ = InvalidateRect(hwnd, None, FALSE);
                        return LRESULT(0);
                    }
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
                    if s.submenu_active {
                        // ignore
                    } else if ctrl_down {
                        s.cursor_pos = word_right(&s.query, s.cursor_pos);
                    } else if s.cursor_pos < s.query.len() {
                        let mut p = s.cursor_pos + 1;
                        while p < s.query.len() && !s.query.is_char_boundary(p) {
                            p += 1;
                        }
                        s.cursor_pos = p;
                    } else {
                        if let Some(r) = s.results.get(s.selected) {
                            if r.entry.source == "app" {
                                s.submenu_active = true;
                                s.submenu_selected = 0;
                            }
                        }
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
                    } else {
                        if let Some(r) = s.results.get(s.selected) {
                            if r.entry.source == "app" {
                                s.submenu_active = !s.submenu_active;
                                s.submenu_selected = 0;
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
                    if s.submenu_active {
                        execute_submenu_action(hwnd, s);
                        return LRESULT(0);
                    }
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
                    } else {
                        execute_selected(hwnd, s);
                    }
                }
                VK_DOWN => {
                    if s.submenu_active {
                        s.submenu_selected = (s.submenu_selected + 1).min(2);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    } else if !s.results.is_empty() {
                        s.selected = (s.selected + 1).min(s.results.len() - 1);
                        if s.selected >= s.scroll_offset + VISIBLE_RESULTS {
                            s.scroll_offset = s.selected - (VISIBLE_RESULTS - 1);
                        }
                        reset_cursor_blink(hwnd, s);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
                VK_UP => {
                    if s.submenu_active {
                        s.submenu_selected = s.submenu_selected.saturating_sub(1);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    } else if s.selected > 0 {
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
            let in_chat_history_list = s.query.starts_with("chats:") || s.query.starts_with("agentchats:");
            if s.ai_answer.is_some() && !in_chat_history_list {
                let delta = (wp.0 >> 16) as i16;
                let step = (delta as i32).abs().max(40);
                if delta > 0 {
                    ai_scroll_up(s, step);
                } else {
                    ai_scroll_down(s, step);
                }
                let _ = InvalidateRect(hwnd, None, FALSE);
                return LRESULT(0);
            }
            if s.voice_listening {
                s.voice_listening = false;
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
            if s.voice_triggered {
                s.voice_triggered = false;
                s.voice_pending_exec = false;
                s.voice_exec_deadline = None;
                let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
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

            // Hermes approval buttons take priority over everything else while shown.
            if s.hermes_approval.is_some() {
                let mx = (lp.0 & 0xFFFF) as i16 as i32;
                let my = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
                // Recover the content_bottom/footer geometry the same way the footer
                // painter does, so the button rects line up exactly.
                let win_h = s.win_h();
                let y_start = s.cy - win_h / 2;
                let footer_h = if s.hermes_approval.is_some() { 76 } else if s.ai_pending { 30 } else { 62 };
                let content_bottom = y_start + SEARCH_H + 1 + AI_PANEL_H - footer_h;
                let _ = win_h;

                let btn_y = content_bottom + 2 + 36;
                let btn_h = 26;
                let approve_w = 96;
                let deny_w = 80;
                let always_w = 130;
                let gap = 8;
                // The footer x-origin matches the morphed result box x. Reuse the
                // same band the painter uses (the box is centered in the window).
                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);
                let band_w = (rc.right - rc.left).min(WIN_W);
                let bx = (rc.right - rc.left - band_w) / 2;
                let pad = 24;
                let approve_x = bx + pad;
                let deny_x = approve_x + approve_w + gap;
                let always_x = deny_x + deny_w + gap;

                if my >= btn_y && my < btn_y + btn_h {
                    if mx >= approve_x && mx < approve_x + approve_w {
                        resolve_current_approval(hwnd, s, true);
                        return LRESULT(0);
                    }
                    if mx >= deny_x && mx < deny_x + deny_w {
                        resolve_current_approval(hwnd, s, false);
                        return LRESULT(0);
                    }
                    if mx >= always_x && mx < always_x + always_w {
                        ai::ALWAYS_APPROVE.store(true, std::sync::atomic::Ordering::Relaxed);
                        if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
                            let _ = conn.execute("INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('always_approve', '1');", []);
                        }
                        resolve_current_approval(hwnd, s, true);
                        return LRESULT(0);
                    }
                }
            }

            if s.ai_answer.is_some() {
                let my = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
                let mx = (lp.0 & 0xFFFF) as i16 as i32;
                let win_h = s.win_h();
                let y_start = s.cy - win_h / 2;
                let mut rc_client = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc_client);
                let win_w = rc_client.right - rc_client.left;
                let x_start = (win_w - WIN_W) / 2;

                let body_top = y_start + SEARCH_H + 1;
                let footer_h = 62;
                let content_bottom = y_start + SEARCH_H + 1 + AI_PANEL_H - footer_h;
                let input_y = content_bottom + 8;
                let input_x = x_start + 24;
                let input_w = WIN_W - 48;

                if mx >= input_x && mx < input_x + input_w && my >= input_y && my < input_y + 34 {
                    s.chat_input_active = true;
                    s.chat_cursor_pos = s.chat_input.len();
                    s.text_selected = false;
                    reset_cursor_blink(hwnd, s);
                    let _ = InvalidateRect(hwnd, None, FALSE);
                    return LRESULT(0);
                }

                // Click inside the chat history area (above bottom input box) copies the chat text
                if my >= body_top && my < content_bottom {
                    if let Some(ans) = &s.ai_answer {
                        copy_to_clipboard(hwnd, ans);
                    }
                    return LRESULT(0);
                }
            }

            if s.color_picker_active {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let screen_dc = GetDC(HWND(null_mut()));
                let pixel = GetPixel(screen_dc, pt.x, pt.y);
                let _ = ReleaseDC(HWND(null_mut()), screen_dc);

                let r = (pixel.0 & 0xFF) as u8;
                let g = ((pixel.0 >> 8) & 0xFF) as u8;
                let b = ((pixel.0 >> 16) & 0xFF) as u8;
                let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);

                copy_to_clipboard(hwnd, &hex);
                stop_color_picker(hwnd, s);
                do_hide(hwnd, s);
                return LRESULT(0);
            }
            // Mic button (search bar's right corner) toggles voice dictation.
            {
                let cmy = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
                let cmx = (lp.0 & 0xFFFF) as i16 as i32;
                let mut rcc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rcc);
                let bx = (rcc.right - rcc.left - WIN_W) / 2;
                let by = s.cy - s.win_h() / 2;
                if cmx >= bx + WIN_W - 52 && cmx < bx + WIN_W - 4 && cmy >= by && cmy < by + SEARCH_H {
                    if s.voice_listening {
                        s.voice_listening = false;
                        s.voice_triggered = false;
                        s.voice_pending_exec = false;
                        s.voice_exec_deadline = None;
                        let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                        let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    } else {
                        start_voice_capture(hwnd, s);
                    }
                    return LRESULT(0);
                }
            }
            if s.voice_listening {
                s.voice_listening = false;
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
            if s.voice_triggered {
                s.voice_triggered = false;
                s.voice_pending_exec = false;
                s.voice_exec_deadline = None;
                let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
                let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
            reset_cursor_blink(hwnd, s);
            let my = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            let mx = (lp.0 & 0xFFFF) as i16 as i32;
            let mut rc_client = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc_client);
            let win_w = rc_client.right - rc_client.left;
            let _bx = (win_w - WIN_W) / 2;
            let by = s.cy - s.win_h() / 2;
            
            if my >= by && my < by + SEARCH_H {
                if s.ai_answer.is_some() || s.ai_pending {
                    s.chat_input_active = false;
                    close_ai_panel(hwnd, s);
                    return LRESULT(0);
                }
            }
            let win_w = rc_client.right - rc_client.left;
            let x_start = (win_w - WIN_W) / 2;
            
            if s.submenu_active && mx >= x_start + (WIN_W - 240) {
                let end_h = s.win_h();
                let end_y = s.cy - end_h / 2;
                let start_y = end_y + SEARCH_H + 16;
                let action_h = 44;
                for idx in 0..3 {
                    let ay = start_y + idx as i32 * (action_h + 8);
                    if my >= ay && my < ay + action_h {
                        s.submenu_selected = idx;
                        execute_submenu_action(hwnd, s);
                        return LRESULT(0);
                    }
                }
                return LRESULT(0);
            } else if s.submenu_active {
                s.submenu_active = false;
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
            let n = (s.results.len().saturating_sub(s.scroll_offset)).min(VISIBLE_RESULTS);
            for i in 0..n {
                let r = s.result_rect(i);
                if my >= r.top && my < r.bottom {
                    s.selected = s.scroll_offset + i;
                    execute_selected(hwnd, s);
                    break;
                }
            }
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;

            if s.color_picker_active {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let mut client_pt = pt;
                let _ = ScreenToClient(hwnd, &mut client_pt);
                s.color_picker_mx = client_pt.x;
                s.color_picker_my = client_pt.y;
                let _ = InvalidateRect(hwnd, None, FALSE);
                return LRESULT(0);
            }
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

        WM_RBUTTONDOWN => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            if s.color_picker_active {
                stop_color_picker(hwnd, s);
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, wp, lp)
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
                let _ = DeleteObject(s.font_mic);
                let _ = DeleteObject(s.font_code);
                let _ = DeleteObject(s.font_h);
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
        // Save the current foreground window so snippet auto-paste can restore focus to it
        s.prev_foreground = GetForegroundWindow();
        if !(s.ai_pending || s.ai_answer.is_some()) {
            s.query.clear();
            s.cursor_pos = 0;
            s.results.clear();
            s.selected = 0;
            s.scroll_offset = 0;
            s.ai_pending = false;
            s.ai_answer = None;
            s.ai_scroll = 0;
            s.ai_follow_bottom = true;
            trigger_search(hwnd, s);
        } else {
            s.selected = 0;
            s.scroll_offset = 0;
            if s.ai_pending {
                let _ = KillTimer(hwnd, TIMER_AI_ANIM);
                let _ = SetTimer(hwnd, TIMER_AI_ANIM, 180, None);
            }
        }

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        // Get active monitor work area (excludes taskbar)
        let hmonitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let (work_w, work_h, work_left, work_top) = if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
            (
                mi.rcWork.right - mi.rcWork.left,
                mi.rcWork.bottom - mi.rcWork.top,
                mi.rcWork.left,
                mi.rcWork.top,
            )
        } else {
            (
                GetSystemMetrics(SM_CXSCREEN),
                GetSystemMetrics(SM_CYSCREEN),
                0,
                0,
            )
        };

        let win_x = work_left + (work_w - WIN_W) / 2;
        let win_y = work_top;

        // Position and size the physical window to cover the entire work area vertically
        let _ = SetWindowPos(
            hwnd,
            HWND(null_mut()),
            win_x,
            win_y,
            WIN_W,
            work_h,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );

        s.cx = WIN_W / 2;
        s.cy = work_h / 2;
        s.last_mouse_x = pt.x;
        s.last_mouse_y = pt.y;

        s.anim = Anim::Appearing { start_time, start_p };

        let _ = SetLayeredWindowAttributes(hwnd, COLOR_KEY, 0, LWA_COLORKEY | LWA_ALPHA);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        force_foreground(hwnd);
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
                force_foreground(hwnd);
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

// AttachThreadInput trick: allows SetForegroundWindow to succeed even from background context.
// Needed when the launcher is summoned by a global hotkey while another app holds focus.
unsafe fn force_foreground(hwnd: HWND) {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    let fore = GetForegroundWindow();
    let fore_tid = GetWindowThreadProcessId(fore, None);
    let my_tid = GetCurrentThreadId();
    if fore_tid != 0 && fore_tid != my_tid {
        let _ = AttachThreadInput(fore_tid, my_tid, TRUE);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(hwnd);
        let _ = AttachThreadInput(fore_tid, my_tid, FALSE);
    } else {
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(hwnd);
    }
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

unsafe fn reset_launcher_window_position(hwnd: HWND, s: &mut State) {
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let hmonitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let (work_w, work_h, work_left, work_top) = if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
        (
            mi.rcWork.right - mi.rcWork.left,
            mi.rcWork.bottom - mi.rcWork.top,
            mi.rcWork.left,
            mi.rcWork.top,
        )
    } else {
        (
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
            0,
            0,
        )
    };

    let _ = SetWindowPos(
        hwnd,
        HWND(null_mut()),
        work_left + (work_w - WIN_W) / 2,
        work_top,
        WIN_W,
        work_h,
        SWP_NOACTIVATE | SWP_NOZORDER,
    );
    s.cx = WIN_W / 2;
    s.cy = work_h / 2;
    let _ = InvalidateRect(hwnd, None, FALSE);
}

// Hotkey / mic-button entry point: open the launcher, show "Listening…", and run one
// one-shot dictation. Mirrors the old wake-word flow (auto-exec the top result).
unsafe fn start_voice_capture(hwnd: HWND, s: &mut State) {
    if s.voice_listening {
        return;
    }
    match s.anim {
        Anim::Hidden | Anim::Hiding { .. } => do_show(hwnd, s),
        _ => {}
    }
    force_foreground(hwnd);
    s.query.clear();
    s.cursor_pos = 0;
    s.selected = 0;
    s.scroll_offset = 0;
    s.text_selected = false;
    s.voice_triggered = true;
    s.voice_listening = true;
    s.voice_pending_exec = false;
    s.voice_exec_deadline = None;
    s.voice_dot_tick = 0;
    let _ = SetTimer(hwnd, TIMER_VOICE_ANIM, 80, None);
    voice::start_query_listener(hwnd);
    let _ = InvalidateRect(hwnd, None, FALSE);
}

unsafe fn start_hide(hwnd: HWND, s: &mut State) {
    // Voice is one-shot (hotkey / mic button), so there's nothing to restart here.
    // Just clear voice flags so the window can dismiss normally.
    s.voice_listening = false;
    s.voice_triggered = false;
    s.voice_pending_exec = false;
    s.voice_exec_deadline = None;
    let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
    let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
    animate_window(hwnd, false);
    reset_visible_chat_view(hwnd, s);
}

unsafe fn reset_visible_chat_view(hwnd: HWND, s: &mut State) {
    // ponytail: hide only resets what is visible; workers keep updating ai_chats in the background.
    let _ = KillTimer(hwnd, TIMER_AI_ANIM);
    s.ai_pending = false;
    s.ai_answer = None;
    s.ai_title.clear();
    s.ai_scroll = 0;
    s.ai_follow_bottom = true;
    s.hermes_approval = None;
    s.ai_tick = 0;
    s.active_chat_id = None;
    s.query.clear();
    s.cursor_pos = 0;
    s.chat_input.clear();
    s.chat_cursor_pos = 0;
    s.chat_input_active = false;
    s.results.clear();
    s.selected = 0;
    s.scroll_offset = 0;
    s.text_selected = false;
}

unsafe fn start_color_picker(hwnd: HWND, s: &mut State) {
    s.color_picker_active = true;

    // Get active monitor full bounds
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let hmonitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let (monitor_w, monitor_h, monitor_left, monitor_top) = if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
        (
            mi.rcMonitor.right - mi.rcMonitor.left,
            mi.rcMonitor.bottom - mi.rcMonitor.top,
            mi.rcMonitor.left,
            mi.rcMonitor.top,
        )
    } else {
        (
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
            0,
            0,
        )
    };

    // Resize and position the window to cover the entire active monitor
    let _ = SetWindowPos(
        hwnd,
        HWND(null_mut()),
        monitor_left,
        monitor_top,
        monitor_w,
        monitor_h,
        SWP_NOACTIVATE | SWP_NOZORDER,
    );

    // Set mouse capture
    let _ = SetCapture(hwnd);
    s.color_picker_mx = pt.x - monitor_left;
    s.color_picker_my = pt.y - monitor_top;

    let _ = InvalidateRect(hwnd, None, FALSE);
}

unsafe fn stop_color_picker(hwnd: HWND, s: &mut State) {
    let _ = ReleaseCapture();
    s.color_picker_active = false;

    // Center the window on the active monitor's work area and reset size
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let hmonitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let (work_w, work_h, work_left, work_top) = if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
        (
            mi.rcWork.right - mi.rcWork.left,
            mi.rcWork.bottom - mi.rcWork.top,
            mi.rcWork.left,
            mi.rcWork.top,
        )
    } else {
        (
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
            0,
            0,
        )
    };

    let win_x = work_left + (work_w - WIN_W) / 2;
    let win_y = work_top;

    let _ = SetWindowPos(
        hwnd,
        HWND(null_mut()),
        win_x,
        win_y,
        WIN_W,
        work_h,
        SWP_NOACTIVATE | SWP_NOZORDER,
    );

    s.cx = WIN_W / 2;
    s.cy = work_h / 2;

    let _ = InvalidateRect(hwnd, None, FALSE);
}

unsafe fn do_hide(hwnd: HWND, s: &mut State) {
    let _ = KillTimer(hwnd, TIMER_DEBOUNCE);
    let _ = KillTimer(hwnd, TIMER_CURSOR_BLINK);
    let _ = KillTimer(hwnd, TIMER_VOICE_ANIM);
    let _ = KillTimer(hwnd, TIMER_VOICE_AUTOEXEC);
    s.voice_triggered = false;
    s.voice_listening = false;
    s.voice_pending_exec = false;
    s.voice_exec_deadline = None;
    s.form_state = FormState::None;
    let _ = ShowWindow(hwnd, SW_HIDE);
    s.anim = Anim::Hidden;
}

fn format_conversation(prompt: &str, response: &str) -> String {
    let prompts: Vec<&str> = prompt.split("\n---\n").map(|p| {
        p.strip_prefix("User: ").unwrap_or(p).trim()
    }).collect();
    let responses: Vec<&str> = response.split("\n\n---\n\n").collect();

    let mut conversation = String::new();
    for i in 0..prompts.len() {
        if i > 0 {
            conversation.push_str("\n\n---\n\n");
        }
        let p = prompts[i];
        if !p.is_empty() {
            conversation.push_str("User: ");
            conversation.push_str(p);
            conversation.push_str("\n\n");
        }
        if i < responses.len() {
            let r = responses[i].trim();
            if !r.is_empty() {
                conversation.push_str(r);
            }
        }
    }
    if responses.len() > prompts.len() {
        for i in prompts.len()..responses.len() {
            if i > 0 {
                conversation.push_str("\n\n---\n\n");
            }
            conversation.push_str(responses[i].trim());
        }
    }
    conversation
}

fn store_ai_chat(db_path: &std::path::Path, command: &str, title: &str, prompt: &str, response: &str) -> Option<i64> {
    if let Ok(conn) = rusqlite::Connection::open(db_path) {
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_chats (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, ts INTEGER, \
                command TEXT, title TEXT, prompt TEXT, response TEXT);",
            [],
        );
        let _ = conn.execute("ALTER TABLE ai_chats ADD COLUMN run_id TEXT;", []);
        let _ = conn.execute("ALTER TABLE ai_chats ADD COLUMN pending_approval TEXT;", []);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let _ = conn.execute(
            "INSERT INTO ai_chats (ts, command, title, prompt, response) VALUES (?,?,?,?,?);",
            rusqlite::params![now, command, title, prompt, response],
        );
        let id = conn.last_insert_rowid();
        return Some(id);
    }
    None
}

fn create_agent(db_path: &std::path::Path, name: &str, goal: &str) {
    if let Ok(conn) = rusqlite::Connection::open(db_path) {
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS agents (\
                id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, goal TEXT, \
                system_prompt TEXT, ts INTEGER);",
            [],
        );
        let system_prompt = if goal.is_empty() {
            format!("You are {name}, a helpful AI assistant. Be concise and proactive.")
        } else {
            format!("You are {name}, an AI assistant. Your goal: {goal}. Be concise, helpful, and proactive in pursuing this goal.")
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        let _ = conn.execute(
            "INSERT INTO agents (name, goal, system_prompt, ts) VALUES (?,?,?,?);",
            rusqlite::params![name, goal, system_prompt, now],
        );
    }
}

fn configure_hermes_llm(endpoint: &str, model: &str, api_key: &str) {
    let base_url = endpoint
        .replace("/chat/completions", "")
        .replace("/completions", "");
    let base_url = base_url.trim().to_string();
    let model = model.trim().to_string();
    let api_key = api_key.trim().to_string();

    if base_url.is_empty() || model.is_empty() || api_key.is_empty() {
        return;
    }

    std::thread::spawn(move || {
        let hermes_cmd = ai::get_hermes_executable();

        let _ = std::process::Command::new(&hermes_cmd)
            .args(["config", "set", "model.default", &model])
            .creation_flags(0x08000000)
            .status();

        let _ = std::process::Command::new(&hermes_cmd)
            .args(["config", "set", "model.provider", "custom"])
            .creation_flags(0x08000000)
            .status();

        let _ = std::process::Command::new(&hermes_cmd)
            .args(["config", "set", "model.base_url", &base_url])
            .creation_flags(0x08000000)
            .status();

        let _ = std::process::Command::new(&hermes_cmd)
            .args(["config", "set", "model.api_key", &api_key])
            .creation_flags(0x08000000)
            .status();

        let _ = std::process::Command::new(&hermes_cmd)
            .args(["config", "set", "agent.tool_use_enforcement", "true"])
            .creation_flags(0x08000000)
            .status();
    });
}

fn start_follow_up_chat(hwnd: HWND, s: &mut State, follow_up: String) {
    start_ai_activity(hwnd, s);
    let prev_ans = s.ai_answer.clone().unwrap_or_default();
    let new_prompt_str = follow_up.clone();
    if !prev_ans.is_empty() {
        s.ai_answer = Some(format!("{}\n\n---\n\nUser: {}\n\nExecuting...", prev_ans, new_prompt_str));
    } else {
        s.ai_answer = Some(format!("User: {}\n\nExecuting...", new_prompt_str));
    }
    s.ai_scroll = 0;
    s.ai_follow_bottom = true;
    s.results.clear();
    s.selected = 0;
    s.chat_input.clear();
    s.chat_cursor_pos = 0;
    s.chat_input_active = true;
    let _ = unsafe { InvalidateRect(hwnd, None, FALSE) };

    let hwnd_raw = hwnd.0 as isize;
    let db_path = s.db_path.clone();
    let chat_id = s.active_chat_id;
    let new_prompt = follow_up;

    std::thread::spawn(move || {
        let mut original_prompt = String::new();
        let mut original_response = String::new();
        let mut command = "ask".to_string();
        let mut title = "Follow-up Chat".to_string();

        if let Some(id) = chat_id {
            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                let _ = conn.query_row(
                    "SELECT command, title, prompt, response FROM ai_chats WHERE id = ?",
                    [id],
                    |row| {
                        let cmd_str: String = row.get(0)?;
                        let title_str: String = row.get(1)?;
                        let p_str: String = row.get(2)?;
                        let r_str: String = row.get(3)?;
                        command = cmd_str;
                        title = title_str;
                        original_prompt = p_str;
                        original_response = r_str;
                        Ok(())
                    }
                );
                // Immediately write the "Executing..." state to DB now that we have the original values.
                let updated_prompt = if original_prompt.is_empty() {
                    new_prompt.clone()
                } else {
                    format!("{}\n---\nUser: {}", original_prompt, new_prompt)
                };
                let updated_response = if original_response.is_empty() {
                    "Executing...".to_string()
                } else {
                    format!("{}\n\n---\n\nExecuting...", original_response)
                };
                let _ = conn.execute(
                    "UPDATE ai_chats SET prompt = ?, response = ? WHERE id = ?",
                    rusqlite::params![updated_prompt, updated_response, id],
                );
            }
        }

        let mut system_prompt = "You are a concise, helpful assistant. Answer directly in at most a few short paragraphs.".to_string();
        if command == "agent" {
            if let Some(name) = title.strip_prefix('@').and_then(|t| t.split_once(':')).map(|(n, _)| n.trim()) {
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    let _ = conn.query_row(
                        "SELECT system_prompt FROM agents WHERE lower(name) = lower(?)",
                        [name],
                        |row| {
                            let sp: String = row.get(0)?;
                            system_prompt = sp;
                            Ok(())
                        }
                    );
                }
            }
        } else {
            system_prompt = match command.as_str() {
                "explain" => "Explain the following clearly and simply for a general audience. Be concise.".to_string(),
                "grammar" => "Fix the spelling and grammar of the text. Output ONLY the corrected text, with no preamble or quotes.".to_string(),
                "translate" => "You are a translator. If the input names a target language (e.g. 'X to Spanish'), translate X into it; otherwise translate the text to English. Output ONLY the translation.".to_string(),
                "summarize" => "Summarize the following text concisely as a few short bullet points.".to_string(),
                "bugs" => "You are a code reviewer. List likely bugs and issues in the following code as short bullet points. Be specific.".to_string(),
                _ => system_prompt
            };
        }

        // Agent follow-ups: prefer the streaming Runs API so a real Approve/Deny
        // button can be shown. The blocking path below is the fallback.
        if command == "agent" && ai::supports_runs_api() {
            let cb_hwnd = SendHwnd(HWND(hwnd_raw as *mut _));
            let prev_history = format_conversation(&original_prompt, &original_response);
            let cb = UiRunCallbacks {
                hwnd: cb_hwnd,
                user: new_prompt.clone(),
                prev_history,
                db_path: db_path.clone(),
                chat_id,
                original_response: original_response.clone(),
            };
            let result = ai::run_agent_streaming(&system_prompt, &new_prompt, &cb);
            if let Err(e) = result {
                let prev_history = format_conversation(&original_prompt, &original_response);
                let formatted_err = format!("{}\n\n---\n\nUser: {}\n\n⚠ {}", prev_history, new_prompt, e);
                let payload = (false, formatted_err);
                let ptr = Box::into_raw(Box::new(payload)) as isize;
                unsafe {
                    let wp_chat_id = chat_id.unwrap_or(0) as usize;
                    let _ = PostMessageW(cb_hwnd.0, WM_AI_RESULT, WPARAM(wp_chat_id), LPARAM(ptr));
                }
            }
            // on_done (success) is delivered by the callback; nothing more to do.
            return;
        }

        let result = if command == "agent" {
            ai::complete_chat_agent(&system_prompt, &original_prompt, &original_response, &new_prompt)
        } else {
            ai::complete_chat(&system_prompt, &original_prompt, &original_response, &new_prompt)
        };

        if let Ok(ref new_response) = result {
            if let Some(id) = chat_id {
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    let updated_prompt = format!("{}\n---\nUser: {}", original_prompt, new_prompt);
                    let updated_response = format!("{}\n\n---\n\n{}", original_response, new_response);
                    let _ = conn.execute(
                        "UPDATE ai_chats SET prompt = ?, response = ? WHERE id = ?",
                        rusqlite::params![updated_prompt, updated_response, id],
                    );
                }
            }
        }

        let payload: (bool, String) = match result {
            Ok(ref new_response) => {
                let updated_prompt = format!("{}\n---\nUser: {}", original_prompt, new_prompt);
                let updated_response = format!("{}\n\n---\n\n{}", original_response, new_response);
                let full_history_resp = format_conversation(&updated_prompt, &updated_response);
                (true, full_history_resp)
            }
            Err(e) => (false, e.to_string()),
        };
        let ptr = Box::into_raw(Box::new(payload)) as isize;
        unsafe {
            let wp_chat_id = chat_id.unwrap_or(0) as usize;
            let _ = PostMessageW(HWND(hwnd_raw as *mut _), WM_AI_RESULT, WPARAM(wp_chat_id), LPARAM(ptr));
        }
    });
}

fn start_ai_activity(hwnd: HWND, s: &mut State) {
    s.ai_pending = true;
    s.ai_tick = 0;
    unsafe {
        let _ = KillTimer(hwnd, TIMER_AI_ANIM);
        let _ = SetTimer(hwnd, TIMER_AI_ANIM, 180, None);
    }
}

// ── Hermes Runs API plumbing ─────────────────────────────────────────────────
//
// RunCallbacks impl that forwards streaming events back to the UI thread via
// PostMessageW. Used by the agent: / follow-up paths when the gateway supports
// the Runs API, so a real Approve/Deny button can be shown instead of hanging.
struct UiRunCallbacks {
    hwnd: SendHwnd,
    user: String,
    prev_history: String,
    db_path: std::path::PathBuf,
    chat_id: Option<i64>,
    original_response: String,
}

impl ai::RunCallbacks for UiRunCallbacks {
    fn on_run_id(&self, run_id: &str) {
        if let Some(id) = self.chat_id {
            if let Ok(conn) = rusqlite::Connection::open(&self.db_path) {
                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                let _ = conn.execute(
                    "UPDATE ai_chats SET run_id = ? WHERE id = ?",
                    rusqlite::params![run_id, id],
                );
            }
        }
    }
    fn on_approval(&self, approval: ai::HermesApproval) {
        if let Some(id) = self.chat_id {
            if let Ok(conn) = rusqlite::Connection::open(&self.db_path) {
                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                let approval_json = serde_json::json!({
                    "run_id": approval.run_id,
                    "approval_id": approval.approval_id,
                    "tool": approval.tool,
                    "summary": approval.summary,
                }).to_string();
                let _ = conn.execute(
                    "UPDATE ai_chats SET pending_approval = ? WHERE id = ?",
                    rusqlite::params![approval_json, id],
                );
            }
        }

        if ai::ALWAYS_APPROVE.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::spawn(move || {
                let _ = ai::resolve_run_approval(&approval, true);
            });
            return;
        }
        let ptr = Box::into_raw(Box::new(approval)) as isize;
        unsafe {
            let wp_chat_id = self.chat_id.unwrap_or(0) as usize;
            let _ = PostMessageW(self.hwnd.0, WM_HERMES_APPROVAL, WPARAM(wp_chat_id), LPARAM(ptr));
        }
    }
    fn on_progress(&self, text: &str) {
        let formatted = if self.prev_history.is_empty() {
            format!("User: {}\n\n{}", self.user, text)
        } else {
            format!("{}\n\n---\n\nUser: {}\n\n{}", self.prev_history, self.user, text)
        };
        let ptr = Box::into_raw(Box::new(formatted)) as isize;
        unsafe {
            let wp_chat_id = self.chat_id.unwrap_or(0) as usize;
            let _ = PostMessageW(self.hwnd.0, WM_AI_PROGRESS, WPARAM(wp_chat_id), LPARAM(ptr));
        }
    }
    fn on_done(&self, ok: bool, text: &str) {
        if let Some(id) = self.chat_id {
            if let Ok(conn) = rusqlite::Connection::open(&self.db_path) {
                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                let updated_response = if self.original_response.is_empty() {
                    if ok { text.to_string() } else { format!("⚠ {}", text) }
                } else {
                    if ok {
                        format!("{}\n\n---\n\n{}", self.original_response, text)
                    } else {
                        format!("{}\n\n---\n\n⚠ {}", self.original_response, text)
                    }
                };
                let _ = conn.execute(
                    "UPDATE ai_chats SET response = ?, pending_approval = NULL WHERE id = ?",
                    rusqlite::params![updated_response, id],
                );
            }
        }
        let formatted = if self.prev_history.is_empty() {
            if ok {
                format!("User: {}\n\n{}", self.user, text)
            } else {
                format!("User: {}\n\n⚠ {}", self.user, text)
            }
        } else {
            if ok {
                format!("{}\n\n---\n\nUser: {}\n\n{}", self.prev_history, self.user, text)
            } else {
                format!("{}\n\n---\n\nUser: {}\n\n⚠ {}", self.prev_history, self.user, text)
            }
        };
        let payload = (ok, formatted);
        let ptr = Box::into_raw(Box::new(payload)) as isize;
        unsafe {
            let wp_chat_id = self.chat_id.unwrap_or(0) as usize;
            let _ = PostMessageW(self.hwnd.0, WM_AI_RESULT, WPARAM(wp_chat_id), LPARAM(ptr));
        }
    }
}

/// Run an agent turn through the streaming Runs API. Returns `false` if the
/// gateway doesn't support it (caller should fall back to blocking complete_agent).
fn run_agent_via_runs_api(
    hwnd: HWND,
    system: String,
    user: String,
    db_path: std::path::PathBuf,
    chat_id: Option<i64>,
) -> bool {
    if !ai::supports_runs_api() {
        return false;
    }
    let cb_hwnd = SendHwnd(hwnd);
    let user_clone = user.clone();
    let db_path_clone = db_path.clone();
    std::thread::spawn(move || {
        let cb = UiRunCallbacks {
            hwnd: cb_hwnd,
            user: user_clone,
            prev_history: String::new(),
            db_path: db_path_clone,
            chat_id,
            original_response: String::new(),
        };
        let result = ai::run_agent_streaming(&system, &user, &cb);
        if let Err(e) = result {
            let formatted_err = format!("User: {}\n\n⚠ {}", user, e);
            let payload = (false, formatted_err);
            let ptr = Box::into_raw(Box::new(payload)) as isize;
            unsafe {
                let wp_chat_id = chat_id.unwrap_or(0) as usize;
                let _ = PostMessageW(cb_hwnd.0, WM_AI_RESULT, WPARAM(wp_chat_id), LPARAM(ptr));
            }
        }
    });
    true
}

unsafe fn close_ai_panel(hwnd: HWND, s: &mut State) {
    let _ = KillTimer(hwnd, TIMER_AI_ANIM);
    s.ai_pending = false;
    s.ai_answer = None;
    s.ai_title.clear();
    s.ai_scroll = 0;
    s.ai_follow_bottom = true;
    s.hermes_approval = None;
    s.ai_tick = 0;
    s.active_chat_id = None;
    s.chat_input.clear();
    s.chat_cursor_pos = 0;
    s.chat_input_active = false;
    s.results.clear();
    trigger_search(hwnd, s); // restore normal results for the current query
    let _ = InvalidateRect(hwnd, None, FALSE);
}

// Resolve the currently-shown Hermes approval: POST the decision to the gateway
// (so the blocked run can continue or abort) and clear the UI state.
unsafe fn resolve_current_approval(hwnd: HWND, s: &mut State, approved: bool) {
    if let Some(ap) = s.hermes_approval.take() {
        if let Some(id) = s.active_chat_id {
            if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
                let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                let _ = conn.execute(
                    "UPDATE ai_chats SET pending_approval = NULL WHERE id = ?",
                    rusqlite::params![id],
                );
            }
        }
        std::thread::spawn(move || {
            let _ = ai::resolve_run_approval(&ap, approved);
        });
        // While awaiting the outcome, show a transient status so the user sees
        // their decision registered.
        s.ai_follow_bottom = true;
        let _ = InvalidateRect(hwnd, None, FALSE);
    }
}

// Scroll the AI panel up by `step` pixels. Scrolling up always leaves "follow the
// latest message" mode so the view stays where the user put it.
fn ai_scroll_up(s: &mut State, step: i32) {
    s.ai_follow_bottom = false;
    s.ai_scroll = (s.ai_scroll - step).max(0);
}

// Scroll the AI panel down by `step` pixels. If this lands at (or past) the bottom,
// re-enter follow-bottom mode so future messages auto-scroll again.
fn ai_scroll_down(s: &mut State, step: i32) {
    let total = s.ai_content_height.get();
    let view = s.ai_view_height.get();
    let max_scroll = (total - view).max(0);
    s.ai_scroll = (s.ai_scroll + step).min(max_scroll);
    if s.ai_scroll >= max_scroll {
        s.ai_follow_bottom = true;
    }
}

unsafe fn execute_selected(hwnd: HWND, s: &mut State) {
    if let Some(r) = s.results.get(s.selected) {
        let cmd = r.entry.launch_command.clone();
        let ctrl_name = r.entry.control_name.clone();
        let is_action_folder = r.entry.source == "FOLDER" && (
            cmd == "bookmarks:" || cmd == "history:" || cmd == "commits:" ||
            cmd == "todos:" || cmd == "clip:" || cmd == "file:" || cmd == "code:" ||
            cmd == "switch:" || cmd == "window:" || cmd == "ql:" || cmd == "snip:" || cmd == "img:" ||
            cmd == "chats:" || cmd == "agents:" || cmd == "agentchats:"
        );
        if is_action_folder {
            s.query = cmd;
            s.cursor_pos = s.query.len();
            s.selected = 0;
            s.scroll_offset = 0;
            s.text_selected = false;
            reset_cursor_blink(hwnd, s);
            trigger_search(hwnd, s);
        } else if let Some(rest) = cmd.strip_prefix("ai:") {
            // Run an AI command on a worker thread; show the answer in the AI panel.
            let (aicmd, inline) = rest.split_once(':').unwrap_or((rest, ""));
            let aicmd = aicmd.to_string();
            let mut input = if inline.trim().is_empty() {
                paste_from_clipboard(hwnd).unwrap_or_default()
            } else {
                inline.to_string()
            };
            if input.len() > 30000 {
                input = input.chars().take(30000).collect::<String>() + "\n\n[Truncated for length...]";
            }
            start_ai_activity(hwnd, s);
            s.ai_answer = Some(format!("User: {}\n\nExecuting...", input));
            s.ai_scroll = 0;
            s.ai_follow_bottom = true;
            s.ai_title = ctrl_name;
            s.results.clear();
            s.selected = 0;
            let _ = InvalidateRect(hwnd, None, FALSE);

            let hwnd_ai = SendHwnd(hwnd);
            let db_path = s.db_path.clone();
            let title = s.ai_title.clone();
            let aicmd_clone = aicmd.clone();
            let input_clone = input.clone();

            // Store chat in DB immediately to get a chat ID
            let chat_id = store_ai_chat(&db_path, &aicmd_clone, &title, &input_clone, "Executing...");
            s.active_chat_id = chat_id;
            s.chat_input.clear();
            s.chat_cursor_pos = 0;
            s.chat_input_active = true;

            std::thread::spawn(move || {
                let hwnd_ai = hwnd_ai;
                let result = ai::run(&aicmd_clone, &input_clone);
                if let Ok(ref text) = result {
                    if let Some(id) = chat_id {
                        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                            let _ = conn.execute(
                                "UPDATE ai_chats SET response = ? WHERE id = ?",
                                rusqlite::params![text, id],
                            );
                        }
                    }
                }
                let payload: (bool, String) = match result {
                    Ok(text) => (true, format_conversation(&input_clone, &text)),
                    Err(e) => (false, e.to_string()),
                };
                let ptr = Box::into_raw(Box::new(payload)) as isize;
                unsafe {
                    let wp_chat_id = chat_id.unwrap_or(0) as usize;
                    let _ = PostMessageW(hwnd_ai.0, WM_AI_RESULT, WPARAM(wp_chat_id), LPARAM(ptr));
                }
            });
            return;
        } else if let Some(id_str) = cmd.strip_prefix("aichat:") {
            // Reopen a stored chat in the panel, reconnecting to its run if still active.
            if let Ok(id) = id_str.parse::<i64>() {
                if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
                    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                    let _ = conn.execute("ALTER TABLE ai_chats ADD COLUMN run_id TEXT;", []);
                    let _ = conn.execute("ALTER TABLE ai_chats ADD COLUMN pending_approval TEXT;", []);
                    if let Ok((title, prompt, response, run_id, pending_approval_str)) = conn.query_row(
                        "SELECT title, prompt, response, run_id, pending_approval FROM ai_chats WHERE id = ?",
                        [id],
                        |row| Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?
                        )),
                    ) {
                        s.ai_title = title;
                        s.ai_scroll = 0;
                        s.ai_follow_bottom = true;
                        s.active_chat_id = Some(id);
                        s.chat_input.clear();
                        s.chat_cursor_pos = 0;
                        s.chat_input_active = true;
                        s.results.clear();
                        s.selected = 0;

                        let mut is_active = false;
                        let mut final_output: Option<String> = None;
                        let mut run_error: Option<String> = None;
                        let mut is_waiting_approval = false;

                        if let Some(ref rid) = run_id {
                            if let Ok(status_resp) = ai::get_run_status(rid) {
                                match status_resp.status.as_str() {
                                    "queued" | "running" => {
                                        is_active = true;
                                    }
                                    "waiting_for_approval" => {
                                        is_active = true;
                                        is_waiting_approval = true;
                                    }
                                    "completed" => {
                                        final_output = status_resp.output;
                                    }
                                    _ => {
                                        run_error = status_resp.error;
                                    }
                                }
                            }
                        }

                        if is_active {
                            s.ai_pending = true;
                            s.ai_tick = 0;
                            let _ = unsafe { KillTimer(hwnd, TIMER_AI_ANIM) };
                            let _ = unsafe { SetTimer(hwnd, TIMER_AI_ANIM, 180, None) };

                            if is_waiting_approval {
                                let mut restored_ap = None;
                                if let Some(ref json_str) = pending_approval_str {
                                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                                        restored_ap = Some(ai::HermesApproval {
                                            run_id: v.get("run_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                                            approval_id: v.get("approval_id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                                            tool: v.get("tool").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                                            summary: v.get("summary").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                                        });
                                    }
                                }
                                if restored_ap.is_none() {
                                    restored_ap = Some(ai::HermesApproval {
                                        run_id: run_id.clone().unwrap_or_default(),
                                        approval_id: "".to_string(),
                                        tool: "System command".to_string(),
                                        summary: "Hermes is waiting for your approval to run a command.".to_string(),
                                    });
                                }
                                s.hermes_approval = restored_ap;
                            } else {
                                s.hermes_approval = None;
                            }

                            s.ai_answer = Some(format_conversation(&prompt, &response));

                            let rid_clone = run_id.clone().unwrap_or_default();
                            let hwnd_ai = SendHwnd(hwnd);
                            let db_path_clone = s.db_path.clone();
                            let chat_id_opt = Some(id);
                            let response_clone = response.clone();

                            std::thread::spawn(move || {
                                let cb = UiRunCallbacks {
                                    hwnd: hwnd_ai,
                                    user: "".to_string(),
                                    prev_history: "".to_string(),
                                    db_path: db_path_clone,
                                    chat_id: chat_id_opt,
                                    original_response: response_clone,
                                };
                                let _ = ai::poll_and_stream_existing_run(&rid_clone, &cb);
                            });
                        } else {
                            s.ai_pending = false;
                            s.hermes_approval = None;
                            if let Some(out) = final_output {
                                if response.is_empty() || response.ends_with("Executing...") {
                                    let _ = conn.execute(
                                        "UPDATE ai_chats SET response = ?, pending_approval = NULL WHERE id = ?",
                                        rusqlite::params![out, id],
                                    );
                                    s.ai_answer = Some(format_conversation(&prompt, &out));
                                } else {
                                    s.ai_answer = Some(format_conversation(&prompt, &response));
                                }
                            } else if let Some(err) = run_error {
                                let updated_response = if response.is_empty() || response.ends_with("Executing...") {
                                    format!("⚠ {}", err)
                                } else {
                                    response.clone()
                                };
                                let _ = conn.execute(
                                    "UPDATE ai_chats SET response = ?, pending_approval = NULL WHERE id = ?",
                                    rusqlite::params![updated_response, id],
                                );
                                s.ai_answer = Some(format_conversation(&prompt, &updated_response));
                            } else {
                                s.ai_answer = Some(format_conversation(&prompt, &response));
                            }
                        }
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
            }
            return;
        } else if let Some(rest) = cmd.strip_prefix("mkagent:") {
            // Create a new agent, then jump to the agents list.
            let mut it = rest.splitn(2, '\u{1f}');
            let name = it.next().unwrap_or("").to_string();
            let goal = it.next().unwrap_or("").to_string();
            if !name.is_empty() {
                create_agent(&s.db_path, &name, &goal);
            }
            s.query = "agents:".to_string();
            s.cursor_pos = s.query.len();
            s.selected = 0;
            s.scroll_offset = 0;
            reset_cursor_blink(hwnd, s);
            trigger_search(hwnd, s);
            return;
        } else if let Some(rest) = cmd.strip_prefix("openagent:") {
            let mut parts = rest.splitn(2, '\u{1f}');
            let _agent_id: i64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(-1);
            let name = parts.next().unwrap_or("").to_string();
            if !name.is_empty() {
                s.query = format!("agentchats:@{}", name);
                s.cursor_pos = s.query.len();
                s.results.clear();
                s.selected = 0;
                s.scroll_offset = 0;
                s.text_selected = false;
                reset_cursor_blink(hwnd, s);
                trigger_search(hwnd, s);
                let _ = InvalidateRect(hwnd, None, FALSE);
            }
            return;
        } else if let Some(name) = cmd.strip_prefix("startnewagent:") {
            let db_path = s.db_path.clone();
            let new_title = format!("@{}: [New Conversation]", name);
            let chat_id = store_ai_chat(&db_path, "agent", &new_title, "", "");
            s.ai_pending = false;
            s.ai_answer = Some("Ask me anything! I will execute tasks on your PC using Hermes.".to_string());
            s.ai_title = new_title;
            s.ai_scroll = 0;
            s.ai_follow_bottom = true;
            s.active_chat_id = chat_id;
            s.chat_input.clear();
            s.chat_cursor_pos = 0;
            s.chat_input_active = true;
            s.results.clear();
            s.selected = 0;
            s.scroll_offset = 0;
            s.text_selected = false;
            reset_cursor_blink(hwnd, s);
            let _ = InvalidateRect(hwnd, None, FALSE);
            return;
        } else if let Some(rest) = cmd.strip_prefix("agent:") {
            // Message an agent: run the AI with the agent's persona, show in the panel.
            let mut it = rest.splitn(2, '\u{1f}');
            let id: i64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(-1);
            let msg = it.next().unwrap_or("").to_string();
            let (mut aname, mut sys) = (String::new(), String::new());
            if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
                if let Ok((n, sp)) = conn.query_row(
                    "SELECT name, system_prompt FROM agents WHERE id = ?",
                    [id],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                ) {
                    aname = n;
                    sys = sp;
                }
            }
            if sys.is_empty() || msg.is_empty() {
                return;
            }
            start_ai_activity(hwnd, s);
            s.ai_answer = Some(format!("User: {}\n\nExecuting...", msg));
            s.ai_scroll = 0;
            s.ai_follow_bottom = true;
            s.ai_title = format!("@{}: {}", aname, msg);
            s.results.clear();
            s.selected = 0;
            let _ = InvalidateRect(hwnd, None, FALSE);

            let hwnd_ai = SendHwnd(hwnd);
            let db_path = s.db_path.clone();
            let title = s.ai_title.clone();
            let msg_clone = msg.clone();

            // Store chat in DB immediately to get a chat ID
            let chat_id = store_ai_chat(&db_path, "agent", &title, &msg_clone, "Executing...");
            s.active_chat_id = chat_id;
            s.chat_input.clear();
            s.chat_cursor_pos = 0;
            s.chat_input_active = true;

            // Prefer the streaming Runs API so a real Approve/Deny button can be
            // shown when Hermes needs tool approval. Fall back to the blocking
            // chat-completions call if the gateway doesn't support runs/approval.
            if run_agent_via_runs_api(hwnd, sys.clone(), msg_clone.clone(), db_path.clone(), chat_id) {
                return;
            }

            std::thread::spawn(move || {
                let hwnd_ai = hwnd_ai;
                let result = ai::complete_agent(&sys, &msg_clone);
                if let Ok(ref text) = result {
                    if let Some(id) = chat_id {
                        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                            let _ = conn.execute(
                                "UPDATE ai_chats SET response = ? WHERE id = ?",
                                rusqlite::params![text, id],
                            );
                        }
                    }
                }
                let payload: (bool, String) = match result {
                    Ok(text) => (true, format_conversation(&msg_clone, &text)),
                    Err(e) => (false, e.to_string()),
                };
                let ptr = Box::into_raw(Box::new(payload)) as isize;
                unsafe {
                    let wp_chat_id = chat_id.unwrap_or(0) as usize;
                    let _ = PostMessageW(hwnd_ai.0, WM_AI_RESULT, WPARAM(wp_chat_id), LPARAM(ptr));
                }
            });
            return;
        } else {
            if let Some(new_query) = cmd.strip_prefix("query:") {
                s.query = new_query.to_string();
                s.cursor_pos = s.query.len();
                s.selected = 0;
                s.scroll_offset = 0;
                s.text_selected = false;
                reset_cursor_blink(hwnd, s);
                trigger_search(hwnd, s);
                let _ = InvalidateRect(hwnd, None, FALSE);
                return;
            } else if let Some(config_action) = cmd.strip_prefix("action:ai_config:") {
                let db_path = s.db_path.clone();
                if config_action == "reset" {
                    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                        let _ = conn.execute("DELETE FROM ai_settings", []);
                    }
                    s.query = "AI Config Reset!".to_string();
                } else if let Some((k, v)) = config_action.split_once(':') {
                    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                        let _ = conn.execute(
                            "CREATE TABLE IF NOT EXISTS ai_settings (key TEXT PRIMARY KEY, value TEXT);",
                            [],
                        );
                        if k == "preset" {
                            if v == "opencode" {
                                let _ = conn.execute("INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('endpoint', 'https://opencode.ai/zen/v1/chat/completions');", []);
                                let _ = conn.execute("INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('model', 'deepseek-v4-flash-free');", []);
                                s.query = "AI Configured for OpenCode Zen!".to_string();
                            } else if v == "deepseek" {
                                let _ = conn.execute("INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('endpoint', 'https://api.deepseek.com/chat/completions');", []);
                                let _ = conn.execute("INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('model', 'deepseek-chat');", []);
                                s.query = "AI Configured for DeepSeek!".to_string();
                            } else if v == "hermes" {
                                create_agent(&db_path, "Hermes", "Execute commands and run autonomous tasks on this Windows PC");
                                s.query = "@Hermes: ".to_string();
                            }
                        } else {
                            let db_key = if k == "key" { "api_key" } else { k };
                            let _ = conn.execute(
                                "INSERT OR REPLACE INTO ai_settings (key, value) VALUES (?, ?);",
                                rusqlite::params![db_key, v],
                            );
                            if db_key == "api_key" {
                                let current_model = conn.query_row(
                                    "SELECT value FROM ai_settings WHERE key = 'model'",
                                    [],
                                    |row| row.get::<_, String>(0),
                                ).unwrap_or_default();
                                if v.trim().starts_with("sk-oc-") || current_model == "hermes-agent" {
                                    let _ = conn.execute("INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('endpoint', 'https://opencode.ai/zen/v1/chat/completions');", []);
                                    let _ = conn.execute("INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('model', 'deepseek-v4-flash-free');", []);
                                }
                            }
                            s.query = format!("AI {} Saved!", k.to_uppercase());
                        }
                    }
                }
                if let Ok(cfg) = ai::get_config() {
                    configure_hermes_llm(&cfg.endpoint, &cfg.model, &cfg.api_key);
                }
                s.cursor_pos = s.query.len();
                s.results.clear();
                s.selected = 0;
                let _ = InvalidateRect(hwnd, None, FALSE);
                return;
            } else if let Some(hermes_action) = cmd.strip_prefix("action:hermes:") {
                if hermes_action == "start" {
                    std::thread::spawn(move || {
                        ai::start_hermes_gateway_daemon();
                    });
                    s.query = "Starting Hermes Gateway...".to_string();
                } else if hermes_action == "stop" {
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/IM", "hermes.exe"])
                        .creation_flags(0x08000000)
                        .spawn();
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/IM", "uv.exe"])
                        .creation_flags(0x08000000)
                        .spawn();
                    s.query = "Stopped Hermes Gateway!".to_string();
                } else if hermes_action == "install" {
                    let _ = std::process::Command::new("powershell")
                        .args(["-NoExit", "-Command", "iex (irm https://hermes-agent.nousresearch.com/install.ps1)"])
                        .spawn();
                    s.query = "Installing Hermes Agent...".to_string();
                }
                s.cursor_pos = s.query.len();
                s.results.clear();
                s.selected = 0;
                let _ = InvalidateRect(hwnd, None, FALSE);
                return;
            } else if let Some(text) = cmd.strip_prefix("copy:") {
                copy_to_clipboard(hwnd, text);
            } else if let Some(path) = cmd.strip_prefix("copy_image:") {
                let prev_hwnd = s.prev_foreground;
                if copy_image_to_clipboard(hwnd, path) {
                    do_hide(hwnd, s);
                    paste_into_window(prev_hwnd);
                } else {
                    s.query = "Could not copy image to clipboard".to_string();
                    s.cursor_pos = s.query.len();
                    s.results.clear();
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
                return;
            } else if cmd == "action:create_snippet" {
                s.form_state = FormState::CreateSnippetName;
                s.query.clear();
                s.cursor_pos = 0;
                s.results.clear();
                s.selected = 0;
                reset_cursor_blink(hwnd, s);
                let _ = InvalidateRect(hwnd, None, FALSE);
                return;
            } else if cmd == "action:create_quicklink" {
                s.form_state = FormState::CreateQuicklinkName;
                s.query.clear();
                s.cursor_pos = 0;
                s.results.clear();
                s.selected = 0;
                reset_cursor_blink(hwnd, s);
                let _ = InvalidateRect(hwnd, None, FALSE);
                return;
            } else if let Some(content) = cmd.strip_prefix("copy_snippet:") {
                copy_to_clipboard(hwnd, content);
                let prev_hwnd = s.prev_foreground;
                do_hide(hwnd, s);
                // Auto-paste into the previously focused window (Raycast-style snippet behavior)
                if !prev_hwnd.0.is_null() {
                    paste_into_window(prev_hwnd);
                }
                return;
            } else if let Some(url) = cmd.strip_prefix("open_quicklink:") {
                let url_w = format!("{}\0", url).encode_utf16().collect::<Vec<u16>>();
                let open_w = "open\0".encode_utf16().collect::<Vec<u16>>();
                windows::Win32::UI::Shell::ShellExecuteW(
                    None,
                    windows::core::PCWSTR(open_w.as_ptr()),
                    windows::core::PCWSTR(url_w.as_ptr()),
                    None,
                    None,
                    windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
                );
                do_hide(hwnd, s);
                return;
            } else if cmd == "action:export_snippets" {
                export_snippets(hwnd, s);
                do_hide(hwnd, s);
                return;
            } else if cmd == "action:import_snippets" {
                import_snippets(hwnd, s);
                do_hide(hwnd, s);
                return;
            } else if cmd == "action:export_quicklinks" {
                export_quicklinks(hwnd, s);
                do_hide(hwnd, s);
                return;
            } else if cmd == "action:import_quicklinks" {
                import_quicklinks(hwnd, s);
                do_hide(hwnd, s);
                return;
            } else if cmd == "action:color_picker" {
                start_color_picker(hwnd, s);
                return;
            } else if cmd == "action:reset_window_position" {
                reset_launcher_window_position(hwnd, s);
                return;
            } else if cmd == "action:quit_active_app" {
                let prev_hwnd = s.prev_foreground;
                do_hide(hwnd, s);
                if !prev_hwnd.0.is_null() {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                        prev_hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
                return;
            } else {
                launcher::launch(&cmd);
            }
            do_hide(hwnd, s);
        }
    }
}


unsafe fn kick_debounce(hwnd: HWND) {
    let _ = KillTimer(hwnd, TIMER_DEBOUNCE);
    let _ = SetTimer(hwnd, TIMER_DEBOUNCE, 55, None);
}

unsafe fn trigger_search(_hwnd: HWND, s: &mut State) {
    s.submenu_active = false;
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

// ── Markdown rendering for the AI panel ───────────────────────────────────────
//
// Both `measure_response` and `paint_response` walk the same parsed blocks so the
// measured height (used for scroll math) always matches the painted height.
//
// Layout vocabulary (pixels):
//   PARA_GAP     — space after a paragraph / between non-adjacent blocks
//   CODE_PAD     — inner padding of a fenced code block
//   LIST_INDENT  — left indent for list markers
const MD_PARA_GAP: i32 = 10;
const MD_CODE_PAD: i32 = 12;
const MD_CODE_BG: COLORREF = COLORREF(0x00_28_26_25);
const MD_CODE_BORDER: COLORREF = COLORREF(0x00_3A_36_35);
const MD_INLINE_CODE_BG: COLORREF = COLORREF(0x00_2E_2C_2B);
const MD_LINK: COLORREF = COLORREF(0x00_7A_B8_F5);
const MD_CODE_FG: COLORREF = COLORREF(0x00_E6_C0_7A);

/// Color used for the body font of an assistant response.
const MD_BODY: COLORREF = COLORREF(0x00_E7_E3_E1);
const MD_MUTED: COLORREF = COLORREF(0x00_8F_89_86);

/// Measure the rendered height of a Markdown response at the given pixel width.
unsafe fn measure_response(hdc: HDC, text: &str, s: &State, width: i32) -> i32 {
    let blocks = markdown::parse(text);
    measure_blocks(hdc, &blocks, s, width)
}

/// Paint a Markdown response at vertical offset `top`. Returns the total height
/// consumed (so the caller can advance its cursor).
unsafe fn paint_response(
    hdc: HDC,
    text: &str,
    s: &State,
    x: i32,
    width: i32,
    top: i32,
) -> i32 {
    let blocks = markdown::parse(text);
    paint_blocks(hdc, &blocks, s, x, width, top)
}

unsafe fn measure_blocks(hdc: HDC, blocks: &[markdown::MdBlock], s: &State, width: i32) -> i32 {
    let mut total = 0i32;
    let prev_font = SelectObject(hdc, s.font_c); // remember to restore
    for (i, b) in blocks.iter().enumerate() {
        let prev_kind = block_kind(blocks.get(i.wrapping_sub(1)));
        total += block_top_gap(b, prev_kind);
        total += measure_one(hdc, b, s, width);
    }
    let _ = SelectObject(hdc, prev_font);
    total
}

fn block_kind(b: Option<&markdown::MdBlock>) -> Option<&'static str> {
    match b {
        Some(markdown::MdBlock::Heading { .. }) => Some("h"),
        Some(markdown::MdBlock::Paragraph { .. }) => Some("p"),
        Some(markdown::MdBlock::Code { .. }) => Some("code"),
        Some(markdown::MdBlock::ListItem { .. }) => Some("li"),
        Some(markdown::MdBlock::Spacer) => Some("spacer"),
        None => None,
    }
}

/// Vertical gap to insert *above* this block based on what came before it.
fn block_top_gap(cur: &markdown::MdBlock, prev: Option<&str>) -> i32 {
    use markdown::MdBlock;
    match (cur, prev) {
        (_, None) => 0,
        (MdBlock::Spacer, _) | (_, Some("spacer")) => 0,
        (MdBlock::Heading { .. }, _) => 14,
        (_, Some("h")) => 10,
        (MdBlock::ListItem { .. }, Some("li")) => 3,
        (MdBlock::ListItem { .. }, _) => 8,
        (_, Some("li")) => MD_PARA_GAP,
        (MdBlock::Code { .. }, _) | (_, Some("code")) => MD_PARA_GAP,
        (_, Some("p")) => MD_PARA_GAP,
        _ => MD_PARA_GAP,
    }
}

unsafe fn measure_one(hdc: HDC, b: &markdown::MdBlock, s: &State, width: i32) -> i32 {
    use markdown::MdBlock;
    match b {
        MdBlock::Spacer => 6,
        MdBlock::Heading { level, .. } => {
            let _ = SelectObject(hdc, s.font_h);
            let plain = strip_inline_text(b);
            let h = wrap_text_height(hdc, &plain, width);
            (*level as i32 - 1).max(0) * 4 + h + 4
        }
        MdBlock::Paragraph { runs } => {
            let _ = SelectObject(hdc, s.font_c);
            measure_runs(hdc, runs, s, width).1
        }
        MdBlock::ListItem { runs, .. } => {
            let _ = SelectObject(hdc, s.font_c);
            // 22px reserved for the marker ("•"/"1.") on the left.
            measure_runs(hdc, runs, s, (width - 22).max(40)).1
        }
        MdBlock::Code { text, .. } => {
            let _ = SelectObject(hdc, s.font_code);
            let inner_w = (width - MD_CODE_PAD * 2).max(40);
            let mut h = 0;
            for line in text.split('\n') {
                let lh = if line.is_empty() {
                    18
                } else {
                    let mut wide: Vec<u16> = line.encode_utf16().collect();
                    let mut rc = RECT { left: 0, top: 0, right: inner_w, bottom: 0 };
                    let _ = DrawTextW(hdc, &mut wide, &mut rc, DT_LEFT | DT_WORDBREAK | DT_CALCRECT | DT_NOPREFIX);
                    (rc.bottom - rc.top).max(18)
                };
                h += lh;
            }
            // Empty code block still shows one line.
            if h == 0 { h = 18; }
            h + MD_CODE_PAD * 2
        }
    }
}

/// Extract the joined plain text of a block (used for heading measurement).
fn strip_inline_text(b: &markdown::MdBlock) -> String {
    use markdown::MdBlock;
    match b {
        MdBlock::Heading { runs, .. } | MdBlock::Paragraph { runs } | MdBlock::ListItem { runs, .. } => {
            runs.iter().map(|r| match r {
                markdown::MdInline::Plain(t) | markdown::MdInline::Bold(t) | markdown::MdInline::Italic(t)
                | markdown::MdInline::Code(t) => t.as_str(),
                markdown::MdInline::Link { label, .. } => label.as_str(),
            }).collect::<Vec<_>>().join("")
        }
        _ => String::new(),
    }
}

unsafe fn wrap_text_height(hdc: HDC, text: &str, width: i32) -> i32 {
    if text.is_empty() { return 16; }
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let mut rc = RECT { left: 0, top: 0, right: width, bottom: 0 };
    let _ = DrawTextW(hdc, &mut wide, &mut rc, DT_LEFT | DT_WORDBREAK | DT_CALCRECT | DT_NOPREFIX);
    (rc.bottom - rc.top).max(16)
}

/// Measure a run-list, returning `(max_inline_width_unused, total_height)`.
/// Height is what the caller actually needs; the width component is unused today
/// but kept to centralize run layout for future use.
unsafe fn measure_runs(hdc: HDC, runs: &[markdown::MdInline], s: &State, width: i32) -> (i32, i32) {
    // Render the runs into a single wrapped string is hard because of mixed
    // fonts; instead we lay runs out greedily into visual lines and sum heights.
    let layout = layout_runs(hdc, runs, s, width);
    let height = layout.line_heights.iter().sum::<i32>();
    (width, height)
}

/// Greedy line layout of inline runs. Returns positioned run fragments plus the
/// per-line heights. Both measure and paint use this so wrapping matches.
unsafe fn layout_runs(
    hdc: HDC,
    runs: &[markdown::MdInline],
    s: &State,
    width: i32,
) -> MdLayout {
    use markdown::MdInline;
    let mut lines: Vec<Vec<MdFrag>> = vec![vec![]];
    let mut cur_w = 0i32;
    let mut max_ascent = 0i32;
    let mut max_descent = 0i32;
    let mut line_heights: Vec<i32> = Vec::new();

    let space_w = text_width(hdc, " ", s.font_c);

    for run in runs {
        let (font, _color, text) = match run {
            MdInline::Plain(t) => (s.font_c, MD_BODY, t.as_str()),
            MdInline::Bold(t) => (s.font_n, MD_BODY, t.as_str()),
            MdInline::Italic(t) => (s.font_c, MD_BODY, t.as_str()),
            MdInline::Code(t) => (s.font_code, MD_CODE_FG, t.as_str()),
            MdInline::Link { label, .. } => (s.font_c, MD_LINK, label.as_str()),
        };
        let _ = _color;

        // Split into words and wrap greedily.
        let parts: Vec<&str> = text.split(' ').collect();
        for (wi, word) in parts.iter().enumerate() {
            if word.is_empty() && wi != 0 {
                // Consecutive spaces collapse to one; the space is handled below.
                continue;
            }
            let word_w = text_width(hdc, word, font);
            let need_space = !lines.last().map(|l| l.is_empty()).unwrap_or(true) && wi != 0;
            let add_w = word_w + if need_space { space_w } else { 0 };

            if cur_w + add_w > width && !lines.last().map(|l| l.is_empty()).unwrap_or(true) {
                // Wrap: flush the current line, then start a fresh line with this word.
                line_heights.push((max_ascent + max_descent).max(16));
                lines.push(vec![]);
                lines.last_mut().unwrap().push(MdFrag {
                    font,
                    text: word.to_string(),
                    leading_space: false,
                });
                cur_w = word_w;
                let metrics = font_metrics(hdc, font);
                max_ascent = metrics.0;
                max_descent = metrics.1;
            } else {
                let leading = need_space;
                lines.last_mut().unwrap().push(MdFrag {
                    font,
                    text: word.to_string(),
                    leading_space: leading,
                });
                cur_w += add_w;
                let metrics = font_metrics(hdc, font);
                max_ascent = max_ascent.max(metrics.0);
                max_descent = max_descent.max(metrics.1);
            }
        }
    }
    line_heights.push((max_ascent + max_descent).max(16));

    MdLayout { lines, line_heights }
}

#[derive(Default)]
struct MdLayout {
    lines: Vec<Vec<MdFrag>>,
    line_heights: Vec<i32>,
}

struct MdFrag {
    font: HFONT,
    text: String,
    leading_space: bool,
}

unsafe fn text_width(hdc: HDC, text: &str, font: HFONT) -> i32 {
    if text.is_empty() { return 0; }
    let _old = SelectObject(hdc, font);
    let wide: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    let _ = GetTextExtentPoint32W(hdc, &wide, &mut size);
    let _ = SelectObject(hdc, _old);
    size.cx
}

/// Returns (ascent, descent) in pixels for the current font.
unsafe fn font_metrics(hdc: HDC, font: HFONT) -> (i32, i32) {
    let _old = SelectObject(hdc, font);
    let mut tm = TEXTMETRICW::default();
    let _ = GetTextMetricsW(hdc, &mut tm);
    let _ = SelectObject(hdc, _old);
    (tm.tmAscent, tm.tmDescent)
}

unsafe fn paint_blocks(
    hdc: HDC,
    blocks: &[markdown::MdBlock],
    s: &State,
    x: i32,
    width: i32,
    top: i32,
) -> i32 {
    use markdown::MdBlock;
    let prev_font = SelectObject(hdc, s.font_c);
    let mut y = top;
    let _ = SetBkMode(hdc, TRANSPARENT);

    for (i, b) in blocks.iter().enumerate() {
        let prev_kind = block_kind(blocks.get(i.wrapping_sub(1)));
        y += block_top_gap(b, prev_kind);

        match b {
            MdBlock::Spacer => {
                y += 6;
            }
            MdBlock::Heading { level, runs } => {
                let _ = SelectObject(hdc, s.font_h);
                SetTextColor(hdc, CLR_WHITE);
                y += (*level as i32 - 1).max(0) * 4;
                y += paint_run_lines(hdc, runs, s, x, y, width, None);
            }
            MdBlock::Paragraph { runs } => {
                let _ = SelectObject(hdc, s.font_c);
                y += paint_run_lines(hdc, runs, s, x, y, width, None);
            }
            MdBlock::ListItem { runs, ordered, index } => {
                let _ = SelectObject(hdc, s.font_c);
                let marker = if *ordered { format!("{}.", index) } else { "•".to_string() };
                // Draw the marker.
                SetTextColor(hdc, MD_MUTED);
                let mut mwide: Vec<u16> = marker.encode_utf16().collect();
                let mut mrc = RECT { left: x, top: y, right: x + 22, bottom: y + 40 };
                let _ = DrawTextW(hdc, &mut mwide, &mut mrc, DT_LEFT | DT_TOP | DT_NOPREFIX);
                y += paint_run_lines(hdc, runs, s, x + 22, y, (width - 22).max(40), None);
            }
            MdBlock::Code { text, .. } => {
                let _ = SelectObject(hdc, s.font_code);
                let inner_w = (width - MD_CODE_PAD * 2).max(40);
                // Measure height to draw the background box.
                let mut h = 0;
                for line in text.split('\n') {
                    let lh = if line.is_empty() {
                        18
                    } else {
                        let mut wide: Vec<u16> = line.encode_utf16().collect();
                        let mut rc = RECT { left: 0, top: 0, right: inner_w, bottom: 0 };
                        let _ = DrawTextW(hdc, &mut wide, &mut rc, DT_LEFT | DT_WORDBREAK | DT_CALCRECT | DT_NOPREFIX);
                        (rc.bottom - rc.top).max(18)
                    };
                    h += lh;
                }
                if h == 0 { h = 18; }
                let box_h = h + MD_CODE_PAD * 2;
                fill_rounded(hdc, x, y, width, box_h, 8, MD_CODE_BG);
                // Subtle left accent border.
                fill(hdc, x, y, 2, box_h, MD_CODE_BORDER);

                SetTextColor(hdc, MD_CODE_FG);
                let mut ly = y + MD_CODE_PAD;
                for line in text.split('\n') {
                    let lh = if line.is_empty() {
                        18
                    } else {
                        let mut wide: Vec<u16> = line.encode_utf16().collect();
                        let mut measure = RECT { left: 0, top: 0, right: inner_w, bottom: 0 };
                        let _ = DrawTextW(hdc, &mut wide, &mut measure, DT_LEFT | DT_WORDBREAK | DT_CALCRECT | DT_NOPREFIX);
                        let lh = (measure.bottom - measure.top).max(18);
                        let mut rc = RECT { left: x + MD_CODE_PAD, top: ly, right: x + MD_CODE_PAD + inner_w, bottom: ly + lh };
                        let _ = DrawTextW(hdc, &mut wide, &mut rc, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);
                        lh
                    };
                    ly += lh;
                }
                y += box_h;
            }
        }
    }

    let _ = SelectObject(hdc, prev_font);
    y - top
}

/// Paint inline runs with word-wrap, advancing downward. Returns the height used.
/// `bg` optionally fills behind each line (used for inline-code chips below).
unsafe fn paint_run_lines(
    hdc: HDC,
    runs: &[markdown::MdInline],
    s: &State,
    x: i32,
    top: i32,
    width: i32,
    _bg: Option<COLORREF>,
) -> i32 {
    let layout = layout_runs(hdc, runs, s, width);
    let mut y = top;

    for (line_idx, line) in layout.lines.iter().enumerate() {
        let lh = layout.line_heights.get(line_idx).copied().unwrap_or(16);
        let mut cx = x;
        // Baseline = top + ascent of the dominant (body) font.
        let body_metrics = font_metrics(hdc, s.font_c);
        let baseline = y + body_metrics.0;

        for frag in line {
            if frag.leading_space {
                cx += text_width(hdc, " ", s.font_c);
            }
            let _ = SelectObject(hdc, frag.font);
            let color = if std::ptr::eq(frag.font.0, s.font_code.0) {
                MD_CODE_FG
            } else {
                MD_BODY
            };

            // Inline code chip: draw a rounded background behind the text.
            let is_code = std::ptr::eq(frag.font.0, s.font_code.0);
            if is_code && !frag.text.is_empty() {
                let tw = text_width(hdc, &frag.text, frag.font);
                let metrics = font_metrics(hdc, frag.font);
                let chip_h = metrics.0 + metrics.1 + 4;
                let chip_top = baseline - metrics.0 + 2;
                fill_rounded(hdc, cx, chip_top, tw + 10, chip_h, 4, MD_INLINE_CODE_BG);
            }

            if !frag.text.is_empty() {
                SetTextColor(hdc, color);
                let mut wide: Vec<u16> = frag.text.encode_utf16().collect();
                let mut rc = RECT { left: cx, top: baseline - (font_metrics(hdc, frag.font).0), right: cx + width, bottom: baseline + 40 };
                let _ = DrawTextW(hdc, &mut wide, &mut rc, DT_LEFT | DT_NOPREFIX | DT_SINGLELINE);
                cx += text_width(hdc, &frag.text, frag.font);
            }
        }
        y += lh;
    }
    y - top
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
        // For WINDOW source: fetch icon synchronously on the UI thread (fast, only called once
        // when results arrive — not on every paint frame) and cache it in app_icons.
        if source == "WINDOW" && !s.app_icons.contains_key(&key) {
            let hwnd_val = key.strip_prefix("window:")
                .and_then(|h| h.parse::<isize>().ok())
                .unwrap_or(0);
            let win_hwnd = HWND(hwnd_val as *mut std::ffi::c_void);
            let hicon = get_window_icon(win_hwnd);
            s.app_icons.insert(key.clone(), hicon);
            continue;
        }
        let is_kill_action = source == "ACTION" && key.starts_with("kill:");
        let needs_icon = (source == "app" || icon_file_path(source, &key).is_some() || is_kill_action)
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

fn known_folder_icon_path(key: &str) -> Option<String> {
    let folder_name = key
        .strip_prefix("folder:")
        .or_else(|| key.strip_prefix("action:folder:"))?;
    let folder_id = match folder_name {
        "downloads" => &windows::Win32::UI::Shell::FOLDERID_Downloads,
        "desktop" => &windows::Win32::UI::Shell::FOLDERID_Desktop,
        "documents" => &windows::Win32::UI::Shell::FOLDERID_Documents,
        "pictures" => &windows::Win32::UI::Shell::FOLDERID_Pictures,
        "music" => &windows::Win32::UI::Shell::FOLDERID_Music,
        "videos" => &windows::Win32::UI::Shell::FOLDERID_Videos,
        _ => return None,
    };
    crate::launcher::get_known_folder_path(folder_id)
}

fn icon_file_path(source: &str, key: &str) -> Option<String> {
    if source == "FOLDER" {
        if let Some(path) = known_folder_icon_path(key) {
            return Some(path);
        }
        if !key.ends_with(':') && std::path::Path::new(key).exists() {
            return Some(key.to_string());
        }
    } else if source == "ACTION" && key.starts_with("action:folder:") {
        return known_folder_icon_path(key);
    } else if source == "RECENT" || source == "FILE" || source == "CODE" {
        if std::path::Path::new(key).exists() {
            return Some(key.to_string());
        }
    } else if source == "PROJECT" && !key.is_empty() && !key.starts_with("http") && std::path::Path::new(key).exists() {
        return Some(key.to_string());
    }
    None
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

    if s.color_picker_active {
        // Draw the magnifier and picked color overlay under the cursor
        let screen_dc = GetDC(HWND(null_mut()));
        let mut pt_screen = POINT { x: s.color_picker_mx, y: s.color_picker_my };
        let _ = ClientToScreen(hwnd, &mut pt_screen);
        let pixel = GetPixel(screen_dc, pt_screen.x, pt_screen.y);

        let zoom_w = 117;
        let zoom_h = 117;
        let mut draw_x = s.color_picker_mx + 25;
        let mut draw_y = s.color_picker_my + 25;
        
        if draw_x + zoom_w + 20 > win_w {
            draw_x = s.color_picker_mx - zoom_w - 25;
        }
        if draw_y + zoom_h + 80 > win_h {
            draw_y = s.color_picker_my - zoom_h - 25;
        }

        let src_x = pt_screen.x - 6;
        let src_y = pt_screen.y - 6;

        let _ = StretchBlt(
            mdc,
            draw_x, draw_y, zoom_w, zoom_h,
            screen_dc,
            src_x, src_y, 13, 13,
            SRCCOPY,
        );

        // Draw magnifier border using fill lines
        fill(mdc, draw_x - 2, draw_y - 2, zoom_w + 4, 2, CLR_WHITE);
        fill(mdc, draw_x - 2, draw_y + zoom_h, zoom_w + 4, 2, CLR_WHITE);
        fill(mdc, draw_x - 2, draw_y - 2, 2, zoom_h + 4, CLR_WHITE);
        fill(mdc, draw_x + zoom_w, draw_y - 2, 2, zoom_h + 4, CLR_WHITE);

        // Draw central pixel highlight box (9x9)
        let cx_box = draw_x + 54;
        let cy_box = draw_y + 54;
        fill(mdc, cx_box - 1, cy_box - 1, 9 + 2, 1, CLR_WHITE);
        fill(mdc, cx_box - 1, cy_box + 9, 9 + 2, 1, CLR_WHITE);
        fill(mdc, cx_box - 1, cy_box, 1, 9, CLR_WHITE);
        fill(mdc, cx_box + 9, cy_box, 1, 9, CLR_WHITE);

        // Draw color info box below magnifier
        let info_y = draw_y + zoom_h + 6;
        fill_rounded(mdc, draw_x - 1, info_y - 1, zoom_w + 2, 44 + 2, 6, CLR_DIV);
        fill_rounded(mdc, draw_x, info_y, zoom_w, 44, 6, BG);

        let r_comp = (pixel.0 & 0xFF) as u8;
        let g_comp = ((pixel.0 >> 8) & 0xFF) as u8;
        let b_comp = ((pixel.0 >> 16) & 0xFF) as u8;
        fill_rounded(mdc, draw_x + 8, info_y + 8, 28, 28, 14, pixel);

        SelectObject(mdc, s.font_b);
        SetTextColor(mdc, CLR_WHITE);
        SetBkMode(mdc, TRANSPARENT);
        let hex_str = format!("#{:02X}{:02X}{:02X}", r_comp, g_comp, b_comp);
        let mut hex_wide: Vec<u16> = hex_str.encode_utf16().collect();
        let mut text_rect = RECT {
            left: draw_x + 42,
            top: info_y + 6,
            right: draw_x + zoom_w - 4,
            bottom: info_y + 22,
        };
        let _ = DrawTextW(mdc, &mut hex_wide, &mut text_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

        SelectObject(mdc, s.font_c);
        SetTextColor(mdc, CLR_GRAY);
        let rgb_str = format!("{},{},{}", r_comp, g_comp, b_comp);
        let mut rgb_wide: Vec<u16> = rgb_str.encode_utf16().collect();
        let mut rgb_rect = RECT {
            left: draw_x + 42,
            top: info_y + 22,
            right: draw_x + zoom_w - 4,
            bottom: info_y + 38,
        };
        let _ = DrawTextW(mdc, &mut rgb_wide, &mut rgb_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

        let _ = ReleaseDC(HWND(null_mut()), screen_dc);

        let _ = BitBlt(hdc, 0, 0, win_w, win_h, mdc, 0, 0, SRCCOPY);
        let _ = SelectObject(mdc, old);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(mdc);
        let _ = EndPaint(hwnd, &ps);
        return;
    }

    // Calculate dynamic shape coordinates
    let p = s.current_p();
    let t = ease_out(p);

    let pill_w = 96;
    let pill_h = SEARCH_H;
    let pill_r = 32;

    let end_w = WIN_W;
    let end_h = s.win_h();

    let w = (pill_w as f32 + (end_w - pill_w) as f32 * t) as i32;
    let h = (pill_h as f32 + (end_h - pill_h) as f32 * t) as i32;
    let x = (win_w - w) / 2;
    let y = s.cy - h / 2;
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
    let tw = w - (PAD_L + ICON_W + 8) - PAD_L - 36;
    let mut tr = RECT { left: tx, top: y, right: tx + tw, bottom: y + SEARCH_H };

    SelectObject(mdc, s.font_q);
    SetTextColor(mdc, CLR_WHITE);

    if s.voice_listening {
        let mut ph: Vec<u16> = "Listening...".encode_utf16().collect();
        SetTextColor(mdc, CLR_PH);
        let _ = DrawTextW(mdc, &mut ph, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SetTextColor(mdc, CLR_WHITE);
    } else if s.query.is_empty() {
        let ph_str = match &s.form_state {
            FormState::CreateSnippetName => "Create Snippet: Enter Name...",
            FormState::CreateSnippetContent { .. } => "Create Snippet: Enter Content...",
            FormState::CreateSnippetKeyword { .. } => "Create Snippet: Enter Keyword (optional)...",
            FormState::CreateQuicklinkName => "Create Quicklink: Enter Name...",
            FormState::CreateQuicklinkUrl { .. } => "Create Quicklink: Enter URL (use {query} placeholder)...",
            FormState::CreateQuicklinkKeyword { .. } => "Create Quicklink: Enter Keyword...",
            FormState::None => "Search Windows settings...",
        };
        let mut ph: Vec<u16> = ph_str.encode_utf16().collect();
        SetTextColor(mdc, CLR_PH);
        let _ = DrawTextW(mdc, &mut ph, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SetTextColor(mdc, CLR_WHITE);
    } else {
        let cur = floor_char_boundary(&s.query, s.cursor_pos);
        let before = &s.query[..cur];
        let dw_before: Vec<u16> = before.encode_utf16().collect();
        let mut size = SIZE::default();
        if !dw_before.is_empty() {
            let _ = GetTextExtentPoint32W(mdc, &dw_before, &mut size);
        }
        let max_w = w - 80;
        let mut scroll_x = 0;
        if size.cx > max_w {
            scroll_x = size.cx - max_w;
        }
        let mut dw_query: Vec<u16> = s.query.encode_utf16().collect();
        let mut text_rect = tr;
        text_rect.left -= scroll_x;
        text_rect.right += 2000; // prevent clipping the remaining text
        let _ = DrawTextW(mdc, &mut dw_query, &mut text_rect, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    }

    // Mic button at the search bar's right corner. Pulses red while listening, sits
    // muted otherwise. Click toggles dictation (hit-test in WM_LBUTTONDOWN).
    if w >= WIN_W - 8 {
        let mic_color = if s.voice_listening {
            let phase = (s.voice_dot_tick as f32 * 0.25).sin().abs();
            let lerp = |a: f32, b: f32| (a + (b - a) * phase) as u8;
            let r_val = lerp(0x60 as f32, 255.0);
            let g_val = lerp(0x24 as f32, 50.0);
            let b_val = lerp(0x24 as f32, 50.0);
            COLORREF(r_val as u32 | ((g_val as u32) << 8) | ((b_val as u32) << 16))
        } else {
            CLR_PH
        };
        SelectObject(mdc, s.font_mic);
        SetTextColor(mdc, mic_color);
        let mut glyph: Vec<u16> = "\u{E720}".encode_utf16().collect();
        let mut mr = RECT { left: x + w - 48, top: y, right: x + w - 12, bottom: y + SEARCH_H };
        let _ = DrawTextW(mdc, &mut glyph, &mut mr, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(mdc, s.font_q);
        SetTextColor(mdc, CLR_WHITE);
    }

    // Draw countdown hint while waiting to auto-execute a voice query.
    if (s.voice_triggered || s.voice_pending_exec) && !s.voice_listening {
        let hint_text = if let Some(deadline) = s.voice_exec_deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let secs = (remaining.as_millis() as f32 / 1000.0).ceil() as u32;
            format!("Esc to cancel · {}s", secs.max(1))
        } else {
            "Listening…".to_string()
        };
        SelectObject(mdc, s.font_c);
        SetTextColor(mdc, COLORREF(0x00_3C_B4_00)); // green
        let mut hint: Vec<u16> = hint_text.encode_utf16().collect();
        let mut hint_tr = RECT {
            left: x + w - 200,
            top: y,
            right: x + w - 52,
            bottom: y + SEARCH_H,
        };
        let _ = DrawTextW(mdc, &mut hint, &mut hint_tr, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SetTextColor(mdc, CLR_WHITE);
        SelectObject(mdc, s.font_q);
    }

    // Draw cursor
    if s.cursor_visible && !s.chat_input_active {
        let cur = floor_char_boundary(&s.query, s.cursor_pos);
        let before = &s.query[..cur];
        let dw_before: Vec<u16> = before.encode_utf16().collect();
        let mut size = SIZE::default();
        if !dw_before.is_empty() {
            let _ = GetTextExtentPoint32W(mdc, &dw_before, &mut size);
        }
        let max_w = w - 80;
        let mut scroll_x = 0;
        if size.cx > max_w {
            scroll_x = size.cx - max_w;
        }
        let cursor_x = tr.left - scroll_x + size.cx;
        
        let mut dummy_size = SIZE::default();
        let _ = GetTextExtentPoint32W(mdc, &['A' as u16], &mut dummy_size);
        let text_h = dummy_size.cy;
        let cursor_top = tr.top + (tr.bottom - tr.top - text_h) / 2;
        fill(mdc, cursor_x, cursor_top, 2, text_h, CLR_WHITE);
    }
    // ── AI answer panel ────────────────────────────────────────────────────
    if s.ai_pending || s.ai_answer.is_some() {
        let pad = 24;
        let body_top = y + SEARCH_H + 1;
        fill(mdc, x, y + SEARCH_H, w, 1, CLR_DIV);

        // Title (the command label)
        SelectObject(mdc, s.font_n);
        SetTextColor(mdc, CLR_WHITE);
        let mut title: Vec<u16> = s.ai_title.encode_utf16().collect();
        let mut title_rc = RECT { left: x + pad, top: body_top + 12, right: x + w - pad - 116, bottom: body_top + 42 };
        let _ = DrawTextW(mdc, &mut title, &mut title_rc, DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);

        if s.ai_pending {
            let dots = match s.ai_tick % 4 {
                0 => "",
                1 => ".",
                2 => "..",
                _ => "...",
            };
            fill_rounded(mdc, x + w - pad - 104, body_top + 11, 104, 24, 10, COLORREF(0x00_34_3C_32));
            SelectObject(mdc, s.font_b);
            SetTextColor(mdc, COLORREF(0x00_B8_D6_B4));
            let mut status: Vec<u16> = format!("Executing{}", dots).encode_utf16().collect();
            let mut status_rc = RECT {
                left: x + w - pad - 96,
                top: body_top + 11,
                right: x + w - pad - 8,
                bottom: body_top + 35,
            };
            let _ = DrawTextW(mdc, &mut status, &mut status_rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            SelectObject(mdc, s.font_n);
        }

        let content_top = body_top + 48;
        let footer_h = if s.hermes_approval.is_some() {
            76
        } else if s.ai_pending {
            30
        } else {
            62
        };
        let content_bottom = y + SEARCH_H + 1 + AI_PANEL_H - footer_h;

        let has_answer = s.ai_answer.is_some();
        if s.ai_pending && !has_answer {
            SelectObject(mdc, s.font_q);
            SetTextColor(mdc, CLR_GRAY);
            let mut th: Vec<u16> = "Thinking…".encode_utf16().collect();
            let mut th_rc = RECT { left: x + pad, top: content_top, right: x + w - pad, bottom: content_bottom };
            let _ = DrawTextW(mdc, &mut th, &mut th_rc, DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX);
        } else if let Some(ans) = &s.ai_answer {
            let parts: Vec<&str> = ans.split("\n\n---\n\n").collect();

            // 1. Measure Pass
            let mut total_h = 0;
            let card_inner_w = w - pad * 2 - 24;
            let resp_w = w - pad * 2;

            for part in &parts {
                let mut prompt = "";
                let mut response = "";
                if part.starts_with("User: ") {
                    let after_user = &part["User: ".len()..];
                    if let Some((p, r)) = after_user.split_once("\n\n") {
                        prompt = p.trim();
                        response = r.trim();
                    } else {
                        prompt = after_user.trim();
                    }
                } else {
                    response = part.trim();
                }

                if !prompt.is_empty() {
                    let mut p_wide: Vec<u16> = prompt.encode_utf16().collect();
                    let mut calc = RECT { left: 0, top: 0, right: card_inner_w, bottom: 0 };
                    SelectObject(mdc, s.font_c);
                    let _ = DrawTextW(mdc, &mut p_wide, &mut calc, DT_LEFT | DT_WORDBREAK | DT_CALCRECT | DT_NOPREFIX);
                    let prompt_h = calc.bottom - calc.top;
                    total_h += prompt_h + 16 + 16;
                }

                if !response.is_empty() {
                    let is_thinking = response == "Thinking..." || response == "Executing...";
                    let resp_h = if is_thinking {
                        // Plain single-line height for the animated status text.
                        let mut r_wide: Vec<u16> = response.encode_utf16().collect();
                        let mut calc = RECT { left: 0, top: 0, right: resp_w, bottom: 0 };
                        SelectObject(mdc, s.font_c);
                        let _ = DrawTextW(mdc, &mut r_wide, &mut calc, DT_LEFT | DT_WORDBREAK | DT_CALCRECT | DT_NOPREFIX);
                        calc.bottom - calc.top
                    } else {
                        // Markdown-rendered height (headings/code/lists/etc.).
                        measure_response(mdc, response, s, resp_w)
                    };
                    total_h += resp_h + 24;
                }
            }

            let view_h = content_bottom - content_top;
            let max_scroll = (total_h - view_h).max(0);
            // Cache these so input handlers (VK_DOWN/wheel) can decide whether the user
            // has scrolled back to the bottom without re-running the whole measure pass.
            s.ai_content_height.set(total_h);
            s.ai_view_height.set(view_h);
            // While pending OR when following the latest message, keep pinned to the bottom.
            let scroll = if s.ai_pending || s.ai_follow_bottom {
                max_scroll
            } else {
                s.ai_scroll.clamp(0, max_scroll)
            };

            // 2. Paint Pass
            let dc_state = SaveDC(mdc);
            let _ = IntersectClipRect(mdc, x + pad, content_top, x + w - pad, content_bottom);

            let mut current_y = content_top - scroll;
            let bg_user = COLORREF(0x00_2C_2B_2A);

            for part in &parts {
                let mut prompt = "";
                let mut response = "";
                if part.starts_with("User: ") {
                    let after_user = &part["User: ".len()..];
                    if let Some((p, r)) = after_user.split_once("\n\n") {
                        prompt = p.trim();
                        response = r.trim();
                    } else {
                        prompt = after_user.trim();
                    }
                } else {
                    response = part.trim();
                }

                if !prompt.is_empty() {
                    let mut p_wide: Vec<u16> = prompt.encode_utf16().collect();
                    let mut calc = RECT { left: 0, top: 0, right: card_inner_w, bottom: 0 };
                    SelectObject(mdc, s.font_c);
                    let _ = DrawTextW(mdc, &mut p_wide.clone(), &mut calc, DT_LEFT | DT_WORDBREAK | DT_CALCRECT | DT_NOPREFIX);
                    let prompt_h = calc.bottom - calc.top;
                    let bubble_h = prompt_h + 16;

                    fill_rounded(mdc, x + pad, current_y, w - pad * 2, bubble_h, 8, bg_user);

                    let mut body_rc = RECT {
                        left: x + pad + 12,
                        top: current_y + 8,
                        right: x + w - pad - 12,
                        bottom: current_y + 8 + prompt_h,
                    };
                    SetTextColor(mdc, COLORREF(0x00_D0_D0_D0));
                    let _ = DrawTextW(mdc, &mut p_wide, &mut body_rc, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);

                    current_y += bubble_h + 16;
                }

                if !response.is_empty() {
                    let is_thinking = response == "Thinking..." || response == "Executing...";
                    let response_text = if is_thinking && s.ai_pending {
                        let dots = match s.ai_tick % 4 {
                            0 => "",
                            1 => ".",
                            2 => "..",
                            _ => "...",
                        };
                        format!("Executing task{}", dots)
                    } else {
                        response.to_string()
                    };
                    let mut r_wide: Vec<u16> = response_text.encode_utf16().collect();
                    let mut calc = RECT { left: 0, top: 0, right: resp_w, bottom: 0 };
                    SelectObject(mdc, s.font_c);
                    let _ = DrawTextW(mdc, &mut r_wide.clone(), &mut calc, DT_LEFT | DT_WORDBREAK | DT_CALCRECT | DT_NOPREFIX);
                    let resp_h = calc.bottom - calc.top;

                    let mut body_rc = RECT {
                        left: x + pad,
                        top: current_y,
                        right: x + w - pad,
                        bottom: current_y + resp_h,
                    };
                    if is_thinking {
                        SetTextColor(mdc, CLR_GRAY);
                        let _ = DrawTextW(mdc, &mut r_wide, &mut body_rc, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);
                        current_y += resp_h + 24;
                    } else {
                        // Markdown: paint using the shared renderer so heights match
                        // the measure pass exactly.
                        let used = paint_response(mdc, &response_text, s, x + pad, resp_w, current_y);
                        current_y += used + 24;
                    }
                }
            }

            let _ = RestoreDC(mdc, dc_state);
        }

        // Footer / chat input (painted over any text overflow)
        fill(mdc, x, content_bottom, w, footer_h + 4, BG);
        fill(mdc, x, content_bottom, w, 1, CLR_DIV);

        // ── Hermes approval banner + Approve/Deny/Always Approve buttons ───────────
        if let Some(ap) = &s.hermes_approval {
            // Banner row describing what needs approval.
            let banner_y = content_bottom + 2;
            SelectObject(mdc, s.font_b);
            SetTextColor(mdc, COLORREF(0x00_F5_C8_7A)); // warm accent
            let label = if ap.tool.is_empty() {
                "Hermes wants to run a tool".to_string()
            } else {
                format!("Hermes wants to run: {}", ap.tool)
            };
            if !label.is_empty() {
                let mut bw: Vec<u16> = label.encode_utf16().collect();
                let mut brc = RECT { left: x + pad, top: banner_y, right: x + w - pad, bottom: banner_y + 16 };
                let _ = DrawTextW(mdc, &mut bw, &mut brc, DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
            }

            // Summary line (if any) in muted text.
            if !ap.summary.is_empty() {
                SelectObject(mdc, s.font_c);
                SetTextColor(mdc, CLR_GRAY);
                let mut sw: Vec<u16> = ap.summary.chars().take(140).collect::<String>().encode_utf16().collect();
                if !sw.is_empty() {
                    let mut src = RECT { left: x + pad, top: banner_y + 16, right: x + w - pad, bottom: banner_y + 32 };
                    let _ = DrawTextW(mdc, &mut sw, &mut src, DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
                }
            }

            // Three buttons: Approve (green), Deny (red), and Always Approve (blue). Hit-tested in WM_LBUTTONDOWN.
            let btn_y = banner_y + 36;
            let btn_h = 26;
            let approve_w = 96;
            let deny_w = 80;
            let always_w = 130;
            let gap = 8;
            let approve_x = x + pad;
            let deny_x = approve_x + approve_w + gap;
            let always_x = deny_x + deny_w + gap;

            // Approve button
            fill_rounded(mdc, approve_x, btn_y, approve_w, btn_h, 6, COLORREF(0x00_3A_6B_3A));
            SelectObject(mdc, s.font_b);
            SetTextColor(mdc, COLORREF(0x00_E6_F5_E6));
            let mut aw: Vec<u16> = "Approve".encode_utf16().collect();
            let mut arc = RECT { left: approve_x, top: btn_y, right: approve_x + approve_w, bottom: btn_y + btn_h };
            let _ = DrawTextW(mdc, &mut aw, &mut arc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

            // Deny button
            fill_rounded(mdc, deny_x, btn_y, deny_w, btn_h, 6, COLORREF(0x00_6B_3A_3A));
            SetTextColor(mdc, COLORREF(0x00_F5_E6_E6));
            let mut dw: Vec<u16> = "Deny".encode_utf16().collect();
            let mut drc = RECT { left: deny_x, top: btn_y, right: deny_x + deny_w, bottom: btn_y + btn_h };
            let _ = DrawTextW(mdc, &mut dw, &mut drc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

            // Always Approve button
            fill_rounded(mdc, always_x, btn_y, always_w, btn_h, 6, COLORREF(0x00_2B_5B_8B));
            SetTextColor(mdc, COLORREF(0x00_E6_EE_F5));
            let mut alw: Vec<u16> = "Always Approve".encode_utf16().collect();
            let mut alrc = RECT { left: always_x, top: btn_y, right: always_x + always_w, bottom: btn_y + btn_h };
            let _ = DrawTextW(mdc, &mut alw, &mut alrc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

            // Hint
            SelectObject(mdc, s.font_b);
            SetTextColor(mdc, CLR_GRAY);
            let mut hw: Vec<u16> = "A approve · D deny · V always".encode_utf16().collect();
            let mut hrc = RECT { left: always_x + always_w + 12, top: btn_y, right: x + w - pad, bottom: btn_y + btn_h };
            let _ = DrawTextW(mdc, &mut hw, &mut hrc, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        } else if s.ai_pending {
            SelectObject(mdc, s.font_b);
            SetTextColor(mdc, CLR_GRAY);
            let mut hint_w: Vec<u16> = "Esc: cancel".encode_utf16().collect();
            let mut hint_rc = RECT { left: x + pad, top: content_bottom + 2, right: x + w - pad, bottom: content_bottom + footer_h };
            let _ = DrawTextW(mdc, &mut hint_w, &mut hint_rc, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        } else {
            let input_y = content_bottom + 8;
            fill_rounded(mdc, x + pad, input_y, w - pad * 2, 34, 10, COLORREF(0x00_2B_29_28));
            SelectObject(mdc, s.font_c);
            let input_text = if s.chat_input.trim().is_empty() {
                SetTextColor(mdc, CLR_PH);
                "Message this chat...".to_string()
            } else {
                SetTextColor(mdc, CLR_WHITE);
                s.chat_input.clone()
            };
            let mut input_w: Vec<u16> = input_text.encode_utf16().collect();
            let mut input_rc = RECT {
                left: x + pad + 12,
                top: input_y,
                right: x + w - pad - 118,
                bottom: input_y + 34,
            };
            let _ = DrawTextW(mdc, &mut input_w, &mut input_rc, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);

            if s.cursor_visible && s.chat_input_active {
                let cur = floor_char_boundary(&s.chat_input, s.chat_cursor_pos);
                let before = &s.chat_input[..cur];
                let dw_before: Vec<u16> = before.encode_utf16().collect();
                let mut size = SIZE::default();
                if !dw_before.is_empty() {
                    let _ = GetTextExtentPoint32W(mdc, &dw_before, &mut size);
                }
                let cursor_x = input_rc.left + size.cx;
                let mut dummy_size = SIZE::default();
                let _ = GetTextExtentPoint32W(mdc, &['A' as u16], &mut dummy_size);
                let text_h = dummy_size.cy;
                let cursor_top = input_rc.top + (input_rc.bottom - input_rc.top - text_h) / 2;
                fill(mdc, cursor_x, cursor_top, 2, text_h, CLR_WHITE);
            }

            SelectObject(mdc, s.font_b);
            SetTextColor(mdc, CLR_GRAY);
            let mut hint_w: Vec<u16> = "Enter send".encode_utf16().collect();
            let mut hint_rc = RECT {
                left: x + w - pad - 104,
                top: input_y,
                right: x + w - pad - 12,
                bottom: input_y + 34,
            };
            let _ = DrawTextW(mdc, &mut hint_w, &mut hint_rc, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }

        SelectObject(mdc, s.font_q);
    }

    // ── Results ───────────────────────────────────────────────────────────
    let n = if s.ai_pending || s.ai_answer.is_some() {
        0
    } else {
        (s.results.len().saturating_sub(s.scroll_offset)).min(VISIBLE_RESULTS)
    };
    if n > 0 {
        let list_w = if s.submenu_active { w - 240 } else { w };
        fill(mdc, x, y + SEARCH_H, list_w, 1, CLR_DIV);

        for i in 0..n {
            let res_idx = s.scroll_offset + i;
            let res = &s.results[res_idx];
            let ry = y + SEARCH_H + 1 + i as i32 * RESULT_H;

            let is_checked = s.selected_clip_ids.contains(&res.entry.id);
            if res_idx == s.selected {
                if is_checked {
                    fill(mdc, x, ry, list_w, RESULT_H, COLORREF(0x00_4E_45_45));
                } else {
                    fill(mdc, x, ry, list_w, RESULT_H, BG_SEL);
                }
            } else if is_checked {
                fill(mdc, x, ry, list_w, RESULT_H, COLORREF(0x00_25_2A_2E));
            }
            if i > 0 { fill(mdc, x + PAD_L, ry, list_w - PAD_L * 2, 1, CLR_DIV); }

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
                // For WINDOW source: icon was pre-fetched into app_icons on result arrival.
                // For all other async-loaded sources, also use app_icons.
                let cached_icon = s.app_icons.get(&res.entry.launch_command)
                    .copied()
                    .filter(|h| !h.0.is_null());
                let icon_to_draw = if let Some(hicon) = cached_icon {
                    hicon
                } else if res.entry.source == "WINDOW" {
                    s.app_icons.get(&res.entry.launch_command)
                        .copied()
                        .filter(|h| !h.0.is_null())
                        .unwrap_or(s.icon_control_panel)
                } else if res.entry.source == "app" || res.entry.source == "RECENT" || res.entry.source == "FILE" || res.entry.source == "CODE"
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
                } else if res.entry.source == "AI" {
                    s.icon_memory
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
            let badge_left = x + list_w - PAD_L - BADGE_W;
            let mut r = RECT { left: tx, top: cy, right: badge_left - 14, bottom: cy + 22 };
            let _ = DrawTextW(mdc, &mut name, &mut r,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

            // Breadcrumb
            SelectObject(mdc, s.font_c);
            SetTextColor(mdc, CLR_GRAY);
            let mut crumb: Vec<u16> = res.entry.breadcrumb_path.encode_utf16().collect();
            let mut r2 = RECT { left: tx, top: cy + 24, right: badge_left - 14, bottom: cy + 40 };
            let _ = DrawTextW(mdc, &mut crumb, &mut r2,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

            // Badge
            let badge_source = if res.entry.id.starts_with("clip.pinned.") {
                "pinned_clip"
            } else {
                &res.entry.source
            };
            badge(mdc, s, badge_source, badge_left, ry + (RESULT_H - BADGE_H) / 2);
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
            let sb_x = x + list_w - 10;
            let sb_w = 4;
            fill(mdc, sb_x, track_top, sb_w, track_h, COLORREF(0x00_2A_2A_2A));
            // Draw thumb
            fill(mdc, sb_x, thumb_y, sb_w, thumb_h, CLR_GRAY);
        }

        if s.submenu_active {
            // Draw dividing line
            fill(mdc, x + list_w, y + SEARCH_H, 1, h - SEARCH_H, CLR_DIV);

            // Draw submenu background
            fill(mdc, x + list_w + 1, y + SEARCH_H + 1, 238, h - SEARCH_H - 1, COLORREF(0x00_15_15_15));

            let actions = [
                "Run as Administrator",
                "Open File Location",
                "Copy Path",
            ];

            let action_h = 44;
            let start_y = y + SEARCH_H + 16;
            for idx in 0..3 {
                let ay = start_y + idx as i32 * (action_h + 8);
                if s.submenu_selected == idx {
                    fill_rounded(mdc, x + list_w + 8, ay, 224, action_h, 8, BG_SEL);
                }

                SelectObject(mdc, s.font_n);
                if s.submenu_selected == idx {
                    SetTextColor(mdc, CLR_WHITE);
                } else {
                    SetTextColor(mdc, CLR_GRAY);
                }

                let mut text_wide: Vec<u16> = actions[idx].encode_utf16().collect();
                let mut r_action = RECT {
                    left: x + list_w + 16,
                    top: ay,
                    right: x + w - 16,
                    bottom: ay + action_h,
                };
                let _ = DrawTextW(mdc, &mut text_wide, &mut r_action, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);

                if s.submenu_selected == idx {
                    SelectObject(mdc, s.font_c);
                    SetTextColor(mdc, COLORREF(0x00_A0_A0_A0));
                    let mut hint_wide: Vec<u16> = "Enter".encode_utf16().collect();
                    let mut r_hint = RECT {
                        left: x + w - 60,
                        top: ay,
                        right: x + w - 16,
                        bottom: ay + action_h,
                    };
                    let _ = DrawTextW(mdc, &mut hint_wide, &mut r_hint, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
                }
            }
        }
    }

    // Draw footer instructions if showing clipboard
    if s.query.starts_with("clip:") || s.query.starts_with("clipboard:") {
        let footer_y = y + h - 24;
        fill(mdc, x, footer_y, w, 24, COLORREF(0x00_15_15_15));
        fill(mdc, x, footer_y, w, 1, CLR_DIV);
        
        if s.delete_confirm {
            badge(mdc, s, "confirm", x + PAD_L, footer_y + 2);
            SelectObject(mdc, s.font_c);
            SetTextColor(mdc, CLR_GRAY);
            let inst_text = format!(" Press Delete again to delete {} selected items, Escape to cancel", s.selected_clip_ids.len());
            let mut inst_wide: Vec<u16> = inst_text.encode_utf16().collect();
            let mut r_inst = RECT { left: x + PAD_L + 68, top: footer_y, right: x + w - PAD_L, bottom: y + h };
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
            let mut r_inst = RECT { left: x + PAD_L, top: footer_y, right: x + w - PAD_L, bottom: y + h };
            let _ = DrawTextW(mdc, &mut inst_wide, &mut r_inst, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
    }

    // Draw footer instructions if showing snippet/quicklink creation form
    if s.form_state != FormState::None {
        let footer_y = y + h - 24;
        fill(mdc, x, footer_y, w, 24, COLORREF(0x00_15_15_15));
        fill(mdc, x, footer_y, w, 1, CLR_DIV);
        
        SelectObject(mdc, s.font_c);
        SetTextColor(mdc, CLR_GRAY);
        let inst_text = " Enter: Next / Save  |  Escape: Cancel creation".to_string();
        let mut inst_wide: Vec<u16> = inst_text.encode_utf16().collect();
        let mut r_inst = RECT { left: x + PAD_L, top: footer_y, right: x + w - PAD_L, bottom: y + h };
        let _ = DrawTextW(mdc, &mut inst_wide, &mut r_inst, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
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
                Red: 0x4200,
                Green: 0x4a00,
                Blue: 0x5600,
                Alpha: 0x0000,
            },
            TRIVERTEX {
                x: x + w,
                y: y + h,
                Red: 0x3f00,
                Green: 0x5d00,
                Blue: 0x6200,
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
    let (label, bg_color, tx_color) = if src_lc == "window" {
        ("WIN", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "live" {
        ("LIVE", COLORREF(0x00_35_46_31), CLR_BDGTX)
    } else if src_lc == "project" {
        ("PROJ", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "action" {
        ("ACT", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "translated" {
        ("OK", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "web" {
        ("WEB", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "app" {
        ("APP", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "ai" {
        ("AI", COLORREF(0x00_3A_37_46), COLORREF(0x00_D6_D0_F0))
    } else if src_lc == "quicklink" {
        ("LINK", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "snippet" {
        ("SNIP", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "calc" {
        ("CALC", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "recent" {
        ("REC", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "file" {
        ("FILE", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "code" {
        ("CODE", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "clipboard" {
        ("CLIP", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "pinned_clip" {
        ("PIN", COLORREF(0x00_46_43_31), CLR_BDGTX)
    } else if src_lc == "confirm" {
        ("DEL", COLORREF(0x00_30_30_55), CLR_WHITE)
    } else if src_lc == "bookmark" {
        ("MARK", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "history" {
        ("HIST", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "folder" {
        ("DIR", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "commit" {
        ("GIT", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "todo" {
        ("TODO", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "memory" {
        ("MEM", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc == "browser" {
        ("BROW", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc.contains("legacy") {
        ("OLD", CLR_BDGBG, CLR_BDGTX)
    } else if src_lc.contains("native") {
        ("SYS", CLR_BDGBG, CLR_BDGTX)
    } else {
        ("SET", CLR_BDGBG, CLR_BDGTX)
    };
    fill_rounded(hdc, x, y, BADGE_W, BADGE_H, 8, bg_color);
    SelectObject(hdc, s.font_b);
    SetTextColor(hdc, tx_color);
    SetBkMode(hdc, TRANSPARENT);
    let mut t: Vec<u16> = label.encode_utf16().collect();
    let mut r = RECT { left: x, top: y, right: x + BADGE_W, bottom: y + BADGE_H };
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

unsafe fn paste_into_window(target: HWND) {
    if target.0.is_null() {
        return;
    }
    std::thread::sleep(std::time::Duration::from_millis(80));
    let _ = SetForegroundWindow(target);
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        VK_CONTROL, KEYEVENTF_KEYUP,
    };
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0x56),
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0x56),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

unsafe fn copy_image_to_clipboard(hwnd: HWND, file_path: &str) -> bool {
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
            let ok = SetClipboardData(2, HANDLE(hbitmap.0)).is_ok();
            let _ = CloseClipboard();
            return ok;
        }
    }
    false
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
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock, GlobalSize};
    
    let mut result = None;
    if OpenClipboard(hwnd).is_ok() {
        if let Ok(h_mem) = GetClipboardData(13) {
            if !h_mem.0.is_null() {
                let h_global = HGLOBAL(h_mem.0);
                let size = GlobalSize(h_global);
                let max_len = size / 2;
                let ptr = GlobalLock(h_global);
                if !ptr.is_null() {
                    let mut len = 0;
                    let ptr_u16 = ptr as *const u16;
                    while len < max_len && *ptr_u16.add(len) != 0 {
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


fn resolve_known_folder_path(path: &str) -> String {
    if path.starts_with('{') && path.contains('}') {
        if let Some(close_brace_idx) = path.find('}') {
            let guid_str = &path[0..=close_brace_idx];
            let guid_str_wide: Vec<u16> = guid_str.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                use windows::Win32::System::Com::CLSIDFromString;
                use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};
                use windows::Win32::Foundation::HANDLE;
                use windows::core::PCWSTR;
                if let Ok(guid) = CLSIDFromString(PCWSTR(guid_str_wide.as_ptr())) {
                    if let Ok(result) = SHGetKnownFolderPath(&guid, KF_FLAG_DEFAULT, HANDLE::default()) {
                        let mut len = 0;
                        while *result.0.add(len) != 0 { len += 1; }
                        let base_path = String::from_utf16_lossy(std::slice::from_raw_parts(result.0, len));
                        windows::Win32::System::Com::CoTaskMemFree(Some(result.0 as *const _));
                        
                        let remaining = &path[close_brace_idx + 1..];
                        let remaining = remaining.trim_start_matches('\\');
                        return format!("{}\\{}", base_path, remaining);
                    }
                }
            }
        }
    }
    path.to_string()
}

fn get_app_path(launch_command: &str) -> String {
    let clean = if let Some(rest) = launch_command.strip_prefix("shell:AppsFolder\\") {
        rest
    } else {
        launch_command
    };
    resolve_known_folder_path(clean)
}

unsafe fn get_window_icon(hwnd: HWND) -> HICON {
    use windows::Win32::UI::WindowsAndMessaging::{
        SendMessageW, WM_GETICON, ICON_BIG, ICON_SMALL, GCLP_HICON, GCLP_HICONSM, GetClassLongPtrW
    };
    use windows::Win32::Foundation::WPARAM;
    
    let mut hicon = HICON(SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_BIG as usize), None).0 as *mut std::ffi::c_void);
    if hicon.0.is_null() {
        hicon = HICON(SendMessageW(hwnd, WM_GETICON, WPARAM(ICON_SMALL as usize), None).0 as *mut std::ffi::c_void);
    }
    if hicon.0.is_null() {
        hicon = HICON(GetClassLongPtrW(hwnd, GCLP_HICON) as *mut std::ffi::c_void);
    }
    if hicon.0.is_null() {
        hicon = HICON(GetClassLongPtrW(hwnd, GCLP_HICONSM) as *mut std::ffi::c_void);
    }
    hicon
}

unsafe fn execute_submenu_action(hwnd: HWND, s: &mut State) {
    if let Some(r) = s.results.get(s.selected) {
        if r.entry.source == "app" {
            let launch_cmd = &r.entry.launch_command;
            let parsing_name = get_app_path(launch_cmd);
            
            let lnk_path = std::path::Path::new(&parsing_name);
            let resolved_path = if parsing_name.to_lowercase().ends_with(".lnk") {
                crate::search::resolve_lnk_path(lnk_path)
            } else {
                None
            };
            
            match s.submenu_selected {
                0 => {
                    // Run as Administrator
                    let run_cmd = if parsing_name.to_lowercase().ends_with(".lnk") {
                        resolved_path.unwrap_or_else(|| parsing_name.clone())
                    } else {
                        if parsing_name.contains('\\') {
                            parsing_name.clone()
                        } else {
                            format!("shell:AppsFolder\\{}", parsing_name)
                        }
                    };
                    
                    let run_cmd_wide: Vec<u16> = run_cmd.encode_utf16().chain(std::iter::once(0)).collect();
                    use windows::Win32::UI::Shell::ShellExecuteW;
                    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
                    use windows::core::{w, PCWSTR};
                    let _ = ShellExecuteW(
                        HWND::default(),
                        w!("runas"),
                        PCWSTR(run_cmd_wide.as_ptr()),
                        PCWSTR::null(),
                        PCWSTR::null(),
                        SW_SHOWNORMAL,
                    );
                    
                    s.submenu_active = false;
                    do_hide(hwnd, s);
                }
                1 => {
                    // Open File Location
                    let select_path = if parsing_name.to_lowercase().ends_with(".lnk") {
                        if let Some(ref res) = resolved_path {
                            res.clone()
                        } else {
                            parsing_name.clone()
                        }
                    } else {
                        parsing_name.clone()
                    };
                    
                    if std::path::Path::new(&select_path).exists() {
                        use std::os::windows::process::CommandExt;
                        let _ = std::process::Command::new("explorer.exe")
                            .arg(format!(r#"/select,"{}""#, select_path))
                            .creation_flags(0x08000000) // CREATE_NO_WINDOW
                            .spawn();
                    }
                    
                    s.submenu_active = false;
                    do_hide(hwnd, s);
                }
                2 => {
                    // Copy Path
                    let copy_val = if parsing_name.to_lowercase().ends_with(".lnk") {
                        resolved_path.unwrap_or_else(|| parsing_name.clone())
                    } else {
                        parsing_name.clone()
                    };
                    
                    copy_to_clipboard(hwnd, &copy_val);
                    s.submenu_active = false;
                    do_hide(hwnd, s);
                }
                _ => {}
            }
        }
    }
}

unsafe fn handle_form_enter(hwnd: HWND, s: &mut State) {
    let input = s.query.trim().to_string();
    match &s.form_state {
        FormState::CreateSnippetName => {
            if !input.is_empty() {
                s.form_state = FormState::CreateSnippetContent { name: input };
                s.query.clear();
                s.cursor_pos = 0;
            }
        }
        FormState::CreateSnippetContent { name } => {
            if !input.is_empty() {
                s.form_state = FormState::CreateSnippetKeyword { name: name.clone(), content: input };
                s.query.clear();
                s.cursor_pos = 0;
            }
        }
        FormState::CreateSnippetKeyword { name, content } => {
            let keyword = if input.is_empty() { None } else { Some(input) };
            let db_path = s.db_path.clone();
            let name = name.clone();
            let content = content.clone();
            std::thread::spawn(move || {
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO snippets (name, content, keyword) VALUES (?, ?, ?);",
                        rusqlite::params![name, content, keyword],
                    );
                }
            });
            s.form_state = FormState::None;
            s.query.clear();
            s.cursor_pos = 0;
            trigger_search(hwnd, s);
        }
        FormState::CreateQuicklinkName => {
            if !input.is_empty() {
                s.form_state = FormState::CreateQuicklinkUrl { name: input };
                s.query.clear();
                s.cursor_pos = 0;
            }
        }
        FormState::CreateQuicklinkUrl { name } => {
            if !input.is_empty() {
                s.form_state = FormState::CreateQuicklinkKeyword { name: name.clone(), url: input };
                s.query.clear();
                s.cursor_pos = 0;
            }
        }
        FormState::CreateQuicklinkKeyword { name, url } => {
            if !input.is_empty() {
                let db_path = s.db_path.clone();
                let name = name.clone();
                let url = url.clone();
                let keyword = input;
                std::thread::spawn(move || {
                    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO quicklinks (name, url, keyword) VALUES (?, ?, ?);",
                            rusqlite::params![name, url, keyword],
                        );
                    }
                });
                s.form_state = FormState::None;
                s.query.clear();
                s.cursor_pos = 0;
                trigger_search(hwnd, s);
            }
        }
        FormState::None => {}
    }
    let _ = InvalidateRect(hwnd, None, FALSE);
}

unsafe fn export_snippets(hwnd: HWND, s: &State) {
    if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
        let mut stmt = match conn.prepare("SELECT name, content, keyword FROM snippets") {
            Ok(st) => st,
            Err(_) => return,
        };
        let iter = match stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
        }) {
            Ok(it) => it,
            Err(_) => return,
        };
        let mut list = Vec::new();
        for item in iter {
            if let Ok((name, content, keyword)) = item {
                list.push(serde_json::json!({
                    "name": name,
                    "content": content,
                    "keyword": keyword,
                }));
            }
        }
        let json_data = serde_json::to_string_pretty(&list).unwrap_or_default();
        if let Some(desktop) = launcher::get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Desktop) {
            let path = std::path::PathBuf::from(desktop).join("snippets_export.json");
            if std::fs::write(&path, json_data).is_ok() {
                copy_to_clipboard(hwnd, &path.to_string_lossy().to_string());
                let msg = format!("Snippets exported successfully to:\n{:?}\n\nPath copied to clipboard.", path);
                let title = "Export Snippets\0".encode_utf16().collect::<Vec<u16>>();
                let msg_w = format!("{}\0", msg).encode_utf16().collect::<Vec<u16>>();
                windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    windows::core::PCWSTR(msg_w.as_ptr()),
                    windows::core::PCWSTR(title.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
    }
}

unsafe fn import_snippets(hwnd: HWND, s: &State) {
    if let Some(desktop) = launcher::get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Desktop) {
        let path = std::path::PathBuf::from(desktop).join("snippets_import.json");
        if !path.exists() {
            let msg = format!("Import file not found!\n\nPlease place a file named 'snippets_import.json' on your Desktop and try again.\n\nTemplate format:\n[\n  {{\n    \"name\": \"example\",\n    \"content\": \"text\",\n    \"keyword\": \"optional\"\n  }}\n]");
            let title = "Import Snippets Error\0".encode_utf16().collect::<Vec<u16>>();
            let msg_w = format!("{}\0", msg).encode_utf16().collect::<Vec<u16>>();
            windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                hwnd,
                windows::core::PCWSTR(msg_w.as_ptr()),
                windows::core::PCWSTR(title.as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
            );
            return;
        }

        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
                    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                    let mut count = 0;
                    for val in list {
                        let name = val.get("name").and_then(|v| v.as_str());
                        let content = val.get("content").and_then(|v| v.as_str());
                        let keyword = val.get("keyword").and_then(|v| v.as_str());
                        if let (Some(n), Some(c)) = (name, content) {
                            if conn.execute(
                                "INSERT OR REPLACE INTO snippets (name, content, keyword) VALUES (?, ?, ?);",
                                rusqlite::params![n, c, keyword],
                            ).is_ok() {
                                count += 1;
                            }
                        }
                    }
                    let msg = format!("Successfully imported {} snippets from snippets_import.json!", count);
                    let title = "Import Snippets\0".encode_utf16().collect::<Vec<u16>>();
                    let msg_w = format!("{}\0", msg).encode_utf16().collect::<Vec<u16>>();
                    windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                        hwnd,
                        windows::core::PCWSTR(msg_w.as_ptr()),
                        windows::core::PCWSTR(title.as_ptr()),
                        windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                    );
                }
            }
        }
    }
}

unsafe fn export_quicklinks(hwnd: HWND, s: &State) {
    if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
        let mut stmt = match conn.prepare("SELECT name, url, keyword FROM quicklinks") {
            Ok(st) => st,
            Err(_) => return,
        };
        let iter = match stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }) {
            Ok(it) => it,
            Err(_) => return,
        };
        let mut list = Vec::new();
        for item in iter {
            if let Ok((name, url, keyword)) = item {
                list.push(serde_json::json!({
                    "name": name,
                    "url": url,
                    "keyword": keyword,
                }));
            }
        }
        let json_data = serde_json::to_string_pretty(&list).unwrap_or_default();
        if let Some(desktop) = launcher::get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Desktop) {
            let path = std::path::PathBuf::from(desktop).join("quicklinks_export.json");
            if std::fs::write(&path, json_data).is_ok() {
                copy_to_clipboard(hwnd, &path.to_string_lossy().to_string());
                let msg = format!("Quicklinks exported successfully to:\n{:?}\n\nPath copied to clipboard.", path);
                let title = "Export Quicklinks\0".encode_utf16().collect::<Vec<u16>>();
                let msg_w = format!("{}\0", msg).encode_utf16().collect::<Vec<u16>>();
                windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    windows::core::PCWSTR(msg_w.as_ptr()),
                    windows::core::PCWSTR(title.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                );
            }
        }
    }
}

unsafe fn import_quicklinks(hwnd: HWND, s: &State) {
    if let Some(desktop) = launcher::get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Desktop) {
        let path = std::path::PathBuf::from(desktop).join("quicklinks_import.json");
        if !path.exists() {
            let msg = format!("Import file not found!\n\nPlease place a file named 'quicklinks_import.json' on your Desktop and try again.\n\nTemplate format:\n[\n  {{\n    \"name\": \"example\",\n    \"url\": \"https://example.com/?q={{query}}\",\n    \"keyword\": \"ex\"\n  }}\n]");
            let title = "Import Quicklinks Error\0".encode_utf16().collect::<Vec<u16>>();
            let msg_w = format!("{}\0", msg).encode_utf16().collect::<Vec<u16>>();
            windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                hwnd,
                windows::core::PCWSTR(msg_w.as_ptr()),
                windows::core::PCWSTR(title.as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
            );
            return;
        }

        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                if let Ok(conn) = rusqlite::Connection::open(&s.db_path) {
                    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
                    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
                    let mut count = 0;
                    for val in list {
                        let name = val.get("name").and_then(|v| v.as_str());
                        let url = val.get("url").and_then(|v| v.as_str());
                        let keyword = val.get("keyword").and_then(|v| v.as_str());
                        if let (Some(n), Some(u), Some(kw)) = (name, url, keyword) {
                            if conn.execute(
                                "INSERT OR REPLACE INTO quicklinks (name, url, keyword) VALUES (?, ?, ?);",
                                rusqlite::params![n, u, kw],
                            ).is_ok() {
                                count += 1;
                            }
                        }
                    }
                    let msg = format!("Successfully imported {} quicklinks from quicklinks_import.json!", count);
                    let title = "Import Quicklinks\0".encode_utf16().collect::<Vec<u16>>();
                    let msg_w = format!("{}\0", msg).encode_utf16().collect::<Vec<u16>>();
                    windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                        hwnd,
                        windows::core::PCWSTR(msg_w.as_ptr()),
                        windows::core::PCWSTR(title.as_ptr()),
                        windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
                    );
                }
            }
        }
    }
}
