#![windows_subsystem = "windows"]

mod launcher;
mod search;

use std::ptr::null_mut;
use search::{SearchEngine, SearchResult};
use windows::{
    core::PCWSTR,
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
const WIN_W: i32 = 640;
const SEARCH_H: i32 = 56;
const RESULT_H: i32 = 68;
const MAX_RESULTS: usize = 8;
const PAD_L: i32 = 20;
const ICON_W: i32 = 28;

// ── Win32 IDs ─────────────────────────────────────────────────────────────────
const HOTKEY_ID: i32 = 1;
const TIMER_DEBOUNCE: usize = 1;
const TIMER_ANIM: usize = 2;

// ── Animation ─────────────────────────────────────────────────────────────────
const ANIM_TICK_MS: u32 = 16;
const APPEAR_FRAMES: f32 = 9.0;  // ~144ms
const HIDE_FRAMES: f32 = 6.0;   // ~96ms
const MAX_ALPHA: u8 = 242;       // 95% opacity
const SLIDE_PX: i32 = 14;

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
    engine: SearchEngine,
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
}

#[derive(PartialEq)]
enum Anim { Hidden, Appearing(i32), Visible, Hiding(i32) }

impl State {
    fn win_h(&self) -> i32 {
        let n = self.results.len().min(MAX_RESULTS) as i32;
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
    }

    let engine = match SearchEngine::new() {
        Ok(e) => e,
        Err(e) => {
            unsafe {
                let mut msg: Vec<u16> = format!("Engine error:\n{e}\0").encode_utf16().collect();
                let mut title: Vec<u16> = "OpenSearch OS\0".encode_utf16().collect();
                MessageBoxW(HWND(null_mut()), PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()),
                            MB_ICONERROR | MB_OK);
                let _ = (&mut msg, &mut title); // suppress unused-mut
            }
            return;
        }
    };

    unsafe { run(engine); }
}

unsafe fn run(engine: SearchEngine) {
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

    let icon_settings = load_icon_from_memory(SETTINGS_ICO, 24);
    let icon_control_panel = load_icon_from_memory(CONTROL_PANEL_ICO, 24);

    let state = Box::new(State {
        engine,
        query: String::new(),
        results: vec![],
        selected: 0,
        anim: Anim::Hidden,
        cx: sw / 2,
        cy: sh / 3,
        font_q: mk_font(-17, 400),
        font_n: mk_font(-15, 600),
        font_c: mk_font(-12, 400),
        font_b: mk_font(-10, 600),
        icon_settings,
        icon_control_panel,
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

    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        PCWSTR(class.as_ptr()),
        PCWSTR::null(),
        WS_POPUP,
        -4000, -4000, WIN_W, SEARCH_H,
        HWND(null_mut()), HMENU(null_mut()), hinst,
        Some(Box::into_raw(state) as _),
    ).unwrap();

    SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA).unwrap();

    // DWM rounded corners (Windows 11)
    let corner = DWMWCP_ROUND;
    let _ = DwmSetWindowAttribute(
        hwnd, DWMWA_WINDOW_CORNER_PREFERENCE,
        &corner as *const _ as _, 4,
    );

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
                Anim::Hidden | Anim::Hiding(_) => do_show(hwnd, s),
                _ => do_hide(hwnd, s),
            }
            LRESULT(0)
        }

        WM_KILLFOCUS => {
            if !sp.is_null() {
                let s = &mut *sp;
                if !matches!(s.anim, Anim::Hidden | Anim::Hiding(_)) {
                    start_hide(hwnd, s);
                }
            }
            LRESULT(0)
        }

        WM_TIMER => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            match wp.0 {
                TIMER_DEBOUNCE => {
                    KillTimer(hwnd, TIMER_DEBOUNCE);
                    s.results = s.engine.search(&s.query, MAX_RESULTS);
                    s.selected = 0;
                    reposition(hwnd, s, 0);
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
            match vk {
                VK_ESCAPE => start_hide(hwnd, s),
                VK_BACK => {
                    s.query.pop();
                    s.selected = 0;
                    kick_debounce(hwnd);
                    InvalidateRect(hwnd, None, FALSE);
                }
                VK_RETURN => {
                    if let Some(r) = s.results.get(s.selected) {
                        let cmd = r.entry.launch_command.clone();
                        launcher::launch(&cmd);
                        do_hide(hwnd, s);
                    }
                }
                VK_DOWN => {
                    if !s.results.is_empty() {
                        s.selected = (s.selected + 1).min(s.results.len() - 1);
                        InvalidateRect(hwnd, None, FALSE);
                    }
                }
                VK_UP => {
                    if s.selected > 0 { s.selected -= 1; }
                    InvalidateRect(hwnd, None, FALSE);
                }
                _ => return DefWindowProcW(hwnd, msg, wp, lp),
            }
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            let my = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            for i in 0..s.results.len().min(MAX_RESULTS) {
                let r = s.result_rect(i);
                if my >= r.top && my < r.bottom {
                    let cmd = s.results[i].entry.launch_command.clone();
                    launcher::launch(&cmd);
                    do_hide(hwnd, s);
                    break;
                }
            }
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            if sp.is_null() { return LRESULT(0); }
            let s = &mut *sp;
            let my = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            for i in 0..s.results.len().min(MAX_RESULTS) {
                let r = s.result_rect(i);
                if my >= r.top && my < r.bottom && s.selected != i {
                    s.selected = i;
                    InvalidateRect(hwnd, None, FALSE);
                    break;
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
            if !sp.is_null() {
                let s = Box::from_raw(sp);
                DeleteObject(s.font_q);
                DeleteObject(s.font_n);
                DeleteObject(s.font_c);
                DeleteObject(s.font_b);
                if !s.icon_settings.0.is_null() { let _ = DestroyIcon(s.icon_settings); }
                if !s.icon_control_panel.0.is_null() { let _ = DestroyIcon(s.icon_control_panel); }
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
    s.results.clear();
    s.selected = 0;
    s.anim = Anim::Appearing(0);
    reposition(hwnd, s, SLIDE_PX);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    SetForegroundWindow(hwnd);
    SetTimer(hwnd, TIMER_ANIM, ANIM_TICK_MS, None);
    InvalidateRect(hwnd, None, FALSE);
}

unsafe fn do_hide(hwnd: HWND, s: &mut State) {
    KillTimer(hwnd, TIMER_ANIM);
    KillTimer(hwnd, TIMER_DEBOUNCE);
    SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA).unwrap();
    ShowWindow(hwnd, SW_HIDE);
    s.anim = Anim::Hidden;
}

unsafe fn start_hide(hwnd: HWND, s: &mut State) {
    s.anim = Anim::Hiding(0);
    SetTimer(hwnd, TIMER_ANIM, ANIM_TICK_MS, None);
}

unsafe fn reposition(hwnd: HWND, s: &State, y_up: i32) {
    let h = s.win_h();
    let x = s.cx - WIN_W / 2;
    let y = s.cy - h / 2 - y_up;
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, WIN_W, h, SWP_NOACTIVATE);
}

unsafe fn kick_debounce(hwnd: HWND) {
    KillTimer(hwnd, TIMER_DEBOUNCE);
    SetTimer(hwnd, TIMER_DEBOUNCE, 80, None);
}

// ── Animation tick ────────────────────────────────────────────────────────────
unsafe fn tick(hwnd: HWND, s: &mut State) {
    match &mut s.anim {
        Anim::Appearing(f) => {
            *f += 1;
            let progress = (*f as f32 / APPEAR_FRAMES).min(1.0);
            let t = ease_out(progress);
            let alpha = (t * MAX_ALPHA as f32) as u8;
            let slide = ((1.0 - t) * SLIDE_PX as f32) as i32;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA).unwrap();
            reposition(hwnd, s, slide);
            if progress >= 1.0 {
                s.anim = Anim::Visible;
                KillTimer(hwnd, TIMER_ANIM);
            }
        }
        Anim::Hiding(f) => {
            *f += 1;
            let progress = (*f as f32 / HIDE_FRAMES).min(1.0);
            let t = 1.0 - ease_out(progress);
            let alpha = (t * MAX_ALPHA as f32) as u8;
            let slide = ((1.0 - t) * SLIDE_PX as f32) as i32;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA).unwrap();
            reposition(hwnd, s, slide);
            if progress >= 1.0 {
                do_hide(hwnd, s);
            }
        }
        _ => { KillTimer(hwnd, TIMER_ANIM); }
    }
    InvalidateRect(hwnd, None, FALSE);
}

fn ease_out(t: f32) -> f32 { 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3) }

// ── Painting ──────────────────────────────────────────────────────────────────
unsafe fn paint(hwnd: HWND, s: &State) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let h = s.win_h();

    // Double-buffer
    let mdc = CreateCompatibleDC(hdc);
    let bmp = CreateCompatibleBitmap(hdc, WIN_W, h);
    let old = SelectObject(mdc, bmp);

    // Fill background
    fill(mdc, 0, 0, WIN_W, h, BG);

    // ── Search row ────────────────────────────────────────────────────────
    SetBkMode(mdc, TRANSPARENT);

    // Icon
    let icon: Vec<u16> = "⌕".encode_utf16().collect();
    SelectObject(mdc, s.font_q);
    SetTextColor(mdc, CLR_GRAY);
    TextOutW(mdc, PAD_L, (SEARCH_H - 20) / 2, &icon);

    // Text / placeholder
    let tx = PAD_L + ICON_W + 8;
    let ty = (SEARCH_H - 22) / 2;
    let tw = WIN_W - tx - PAD_L;
    let mut tr = RECT { left: tx, top: ty, right: tx + tw, bottom: ty + 22 };

    if s.query.is_empty() {
        let mut ph: Vec<u16> = "Search Windows settings...".encode_utf16().collect();
        SetTextColor(mdc, CLR_PH);
        DrawTextW(mdc, &mut ph, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    } else {
        let display = format!("{}_", s.query);
        let mut dw: Vec<u16> = display.encode_utf16().collect();
        SetTextColor(mdc, CLR_WHITE);
        DrawTextW(mdc, &mut dw, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    }

    // ── Results ───────────────────────────────────────────────────────────
    let n = s.results.len().min(MAX_RESULTS);
    if n > 0 {
        fill(mdc, 0, SEARCH_H, WIN_W, 1, CLR_DIV);

        for (i, res) in s.results.iter().take(n).enumerate() {
            let ry = SEARCH_H + 1 + i as i32 * RESULT_H;

            if i == s.selected { fill(mdc, 0, ry, WIN_W, RESULT_H, BG_SEL); }
            if i > 0 { fill(mdc, PAD_L, ry, WIN_W - PAD_L * 2, 1, CLR_DIV); }

            let cy = ry + (RESULT_H - 34) / 2;

            // Draw Icon
            let icon_to_draw = if res.entry.launch_command.starts_with("ms-settings:") {
                s.icon_settings
            } else {
                s.icon_control_panel
            };

            if !icon_to_draw.0.is_null() {
                let icon_x = PAD_L + (ICON_W - 24) / 2;
                let icon_y = ry + (RESULT_H - 24) / 2;
                let _ = DrawIconEx(mdc, icon_x, icon_y, icon_to_draw, 24, 24, 0, HBRUSH(null_mut()), DI_NORMAL);
            }

            // Name
            SelectObject(mdc, s.font_n);
            SetTextColor(mdc, CLR_WHITE);
            let mut name: Vec<u16> = res.entry.control_name.encode_utf16().collect();
            let mut r = RECT { left: PAD_L + ICON_W, top: cy, right: WIN_W - 88, bottom: cy + 20 };
            DrawTextW(mdc, &mut name, &mut r,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

            // Breadcrumb
            SelectObject(mdc, s.font_c);
            SetTextColor(mdc, CLR_GRAY);
            let mut crumb: Vec<u16> = res.entry.breadcrumb_path.encode_utf16().collect();
            let mut r2 = RECT { left: PAD_L + ICON_W, top: cy + 19, right: WIN_W - 88, bottom: cy + 34 };
            DrawTextW(mdc, &mut crumb, &mut r2,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);

            // Badge
            badge(mdc, s, &res.entry.source, WIN_W - 80, ry + (RESULT_H - 18) / 2);
        }
    }

    BitBlt(hdc, 0, 0, WIN_W, h, mdc, 0, 0, SRCCOPY);
    SelectObject(mdc, old);
    DeleteObject(bmp);
    DeleteDC(mdc);
    EndPaint(hwnd, &ps);
}

unsafe fn fill(hdc: HDC, x: i32, y: i32, w: i32, h: i32, c: COLORREF) {
    let br = CreateSolidBrush(c);
    FillRect(hdc, &RECT { left: x, top: y, right: x + w, bottom: y + h }, br);
    DeleteObject(br);
}

unsafe fn badge(hdc: HDC, s: &State, source: &str, x: i32, y: i32) {
    let src_lc = source.to_lowercase();
    let label = if src_lc.contains("legacy") || src_lc.contains("native") {
        if src_lc.contains("legacy") { "LEGACY" } else { "NATIVE" }
    } else {
        "MODERN"
    };
    fill(hdc, x, y, 64, 18, CLR_BDGBG);
    SelectObject(hdc, s.font_b);
    SetTextColor(hdc, CLR_BDGTX);
    SetBkMode(hdc, TRANSPARENT);
    let mut t: Vec<u16> = label.encode_utf16().collect();
    let mut r = RECT { left: x, top: y, right: x + 64, bottom: y + 18 };
    DrawTextW(hdc, &mut t, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
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
