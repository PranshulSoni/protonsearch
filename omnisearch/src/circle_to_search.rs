use crate::{ReleaseCapture, SetCapture, VK_ESCAPE};
use image::{codecs::png::PngEncoder, ColorType};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
struct Selection {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    active: bool,
}

struct OverlayData {
    bmp: HBITMAP,
    screen_w: i32,
    screen_h: i32,
    selection: Selection,
}

pub fn start_lens_search_async() {
    std::thread::spawn(|| unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else {
            return;
        };
        if let Some((bmp, w, h)) = capture_screen() {
            if let Some(overlay) = create_overlay_window(hinst, bmp, w, h) {
                let _ = ShowWindow(overlay, SW_SHOW);
                let _ = UpdateWindow(overlay);
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
                }
            } else {
                let _ = DeleteObject(bmp);
            }
        }
    });
}

unsafe fn capture_screen() -> Option<(HBITMAP, i32, i32)> {
    let screen_dc = GetDC(HWND::default());
    if screen_dc.is_invalid() {
        return None;
    }
    let w = GetSystemMetrics(SM_CXSCREEN);
    let h = GetSystemMetrics(SM_CYSCREEN);
    let bmp = CreateCompatibleBitmap(screen_dc, w, h);
    if bmp.is_invalid() {
        let _ = ReleaseDC(HWND::default(), screen_dc);
        return None;
    }
    let mem_dc = CreateCompatibleDC(screen_dc);
    if mem_dc.is_invalid() {
        let _ = DeleteObject(bmp);
        let _ = ReleaseDC(HWND::default(), screen_dc);
        return None;
    }
    let old = SelectObject(mem_dc, bmp);
    let _ = BitBlt(mem_dc, 0, 0, w, h, screen_dc, 0, 0, SRCCOPY);
    let _ = SelectObject(mem_dc, old);
    let _ = DeleteDC(mem_dc);
    let _ = ReleaseDC(HWND::default(), screen_dc);
    Some((bmp, w, h))
}

unsafe fn create_overlay_window(
    hinst: HMODULE,
    bmp: HBITMAP,
    screen_w: i32,
    screen_h: i32,
) -> Option<HWND> {
    let class: Vec<u16> = "omnisearch-circle-search\0".encode_utf16().collect();
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(overlay_wndproc),
        hInstance: HINSTANCE(hinst.0),
        hCursor: LoadCursorW(None, IDC_CROSS).unwrap_or_default(),
        hbrBackground: HBRUSH::default(),
        lpszClassName: PCWSTR(class.as_ptr()),
        ..Default::default()
    };
    RegisterClassW(&wc);

    let data = Box::into_raw(Box::new(OverlayData {
        bmp,
        screen_w,
        screen_h,
        selection: Selection::default(),
    }));

    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        PCWSTR(class.as_ptr()),
        PCWSTR::null(),
        WS_POPUP | WS_VISIBLE,
        0,
        0,
        screen_w,
        screen_h,
        None,
        None,
        HINSTANCE(hinst.0),
        Some(data as _),
    );
    match hwnd {
        Ok(h) => {
            let _ = SetWindowPos(h, HWND_TOPMOST, 0, 0, screen_w, screen_h, SWP_SHOWWINDOW);
            let _ = SetForegroundWindow(h);
            Some(h)
        }
        Err(_) => {
            let _ = Box::from_raw(data);
            None
        }
    }
}

extern "system" fn overlay_wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_CREATE {
            let cs = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            return LRESULT(0);
        }

        let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayData;
        if data_ptr.is_null() {
            return DefWindowProcW(hwnd, msg, wp, lp);
        }
        let data = &mut *data_ptr;

        match msg {
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                paint_overlay(hwnd, data);
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                let x = loword(lp.0 as u32);
                let y = hiword(lp.0 as u32);
                data.selection = Selection {
                    x1: x,
                    y1: y,
                    x2: x,
                    y2: y,
                    active: true,
                };
                let _ = SetCapture(hwnd);
                let _ = InvalidateRect(hwnd, None, FALSE);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                if data.selection.active {
                    data.selection.x2 = loword(lp.0 as u32);
                    data.selection.y2 = hiword(lp.0 as u32);
                    let _ = InvalidateRect(hwnd, None, FALSE);
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                if data.selection.active {
                    let _ = ReleaseCapture();
                    data.selection.x2 = loword(lp.0 as u32);
                    data.selection.y2 = hiword(lp.0 as u32);
                    data.selection.active = false;
                    let sx = data.selection.x1.min(data.selection.x2).max(0);
                    let sy = data.selection.y1.min(data.selection.y2).max(0);
                    let sw = (data.selection.x1 - data.selection.x2)
                        .abs()
                        .min(data.screen_w - sx);
                    let sh = (data.selection.y1 - data.selection.y2)
                        .abs()
                        .min(data.screen_h - sy);
                    if sw >= 8 && sh >= 8 {
                        if let Some(cropped) = crop_bitmap(data.bmp, sx, sy, sw, sh) {
                            process_image(cropped);
                        }
                        let _ = DestroyWindow(hwnd);
                        PostQuitMessage(0);
                    } else {
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                if wp.0 as u32 == VK_ESCAPE.0 as u32 {
                    let _ = DestroyWindow(hwnd);
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                let data = Box::from_raw(data_ptr);
                let _ = DeleteObject(data.bmp);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

unsafe fn paint_overlay(hwnd: HWND, data: &OverlayData) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mem_dc = CreateCompatibleDC(hdc);
    let old = SelectObject(mem_dc, data.bmp);
    let _ = BitBlt(
        hdc,
        0,
        0,
        data.screen_w,
        data.screen_h,
        mem_dc,
        0,
        0,
        SRCCOPY,
    );
    let _ = SelectObject(mem_dc, old);
    let _ = DeleteDC(mem_dc);

    let shade = CreateSolidBrush(rgb(0, 0, 0));
    let old_brush = SelectObject(hdc, shade);
    let _ = PatBlt(hdc, 0, 0, data.screen_w, data.screen_h, PATINVERT);
    let _ = SelectObject(hdc, old_brush);
    let _ = DeleteObject(shade);

    let sel = &data.selection;
    let sx = sel.x1.min(sel.x2);
    let sy = sel.y1.min(sel.y2);
    let sw = (sel.x1 - sel.x2).abs();
    let sh = (sel.y1 - sel.y2).abs();
    if sw > 0 && sh > 0 {
        let pen = CreatePen(PS_SOLID, 2, rgb(255, 255, 255));
        let old_pen = SelectObject(hdc, pen);
        let old_b = SelectObject(hdc, GetStockObject(NULL_BRUSH));
        let _ = Rectangle(hdc, sx, sy, sx + sw, sy + sh);
        let _ = SelectObject(hdc, old_pen);
        let _ = SelectObject(hdc, old_b);
        let _ = DeleteObject(pen);
    }

    let mut text: Vec<u16> = "Drag to search with Google Lens. Esc cancels."
        .encode_utf16()
        .collect();
    let mut rect = RECT {
        left: 24,
        top: 24,
        right: data.screen_w - 24,
        bottom: 64,
    };
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, rgb(255, 255, 255));
    let _ = DrawTextW(
        hdc,
        &mut text,
        &mut rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    let _ = EndPaint(hwnd, &ps);
}

unsafe fn crop_bitmap(src: HBITMAP, x: i32, y: i32, w: i32, h: i32) -> Option<HBITMAP> {
    let screen_dc = GetDC(HWND::default());
    let src_dc = CreateCompatibleDC(screen_dc);
    let old_src = SelectObject(src_dc, src);
    let dst = CreateCompatibleBitmap(screen_dc, w, h);
    if dst.is_invalid() {
        let _ = SelectObject(src_dc, old_src);
        let _ = DeleteDC(src_dc);
        let _ = ReleaseDC(HWND::default(), screen_dc);
        return None;
    }
    let dst_dc = CreateCompatibleDC(screen_dc);
    let old_dst = SelectObject(dst_dc, dst);
    let _ = BitBlt(dst_dc, 0, 0, w, h, src_dc, x, y, SRCCOPY);
    let _ = SelectObject(dst_dc, old_dst);
    let _ = SelectObject(src_dc, old_src);
    let _ = DeleteDC(dst_dc);
    let _ = DeleteDC(src_dc);
    let _ = ReleaseDC(HWND::default(), screen_dc);
    Some(dst)
}

unsafe fn process_image(hbmp: HBITMAP) {
    let Some((rgba, w, h)) = bitmap_to_rgba(hbmp) else {
        let _ = DeleteObject(hbmp);
        return;
    };
    let _ = DeleteObject(hbmp);
    let mut png = Vec::new();
    if PngEncoder::new(&mut png)
        .encode(&rgba, w, h, ColorType::Rgba8)
        .is_err()
    {
        return;
    }
    upload_to_lens(png);
}

unsafe fn bitmap_to_rgba(hbmp: HBITMAP) -> Option<(Vec<u8>, u32, u32)> {
    let screen_dc = GetDC(HWND::default());
    let mem_dc = CreateCompatibleDC(screen_dc);
    let old = SelectObject(mem_dc, hbmp);
    let mut bmp = BITMAP::default();
    if GetObjectW(
        hbmp,
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bmp as *mut _ as *mut _),
    ) == 0
    {
        let _ = SelectObject(mem_dc, old);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(HWND::default(), screen_dc);
        return None;
    }
    let w = bmp.bmWidth as u32;
    let h = bmp.bmHeight as u32;
    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w as i32;
    bmi.bmiHeader.biHeight = -(h as i32);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;
    let mut bgra = vec![0u8; (w * h * 4) as usize];
    let ok = GetDIBits(
        mem_dc,
        hbmp,
        0,
        h,
        Some(bgra.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    let _ = SelectObject(mem_dc, old);
    let _ = DeleteDC(mem_dc);
    let _ = ReleaseDC(HWND::default(), screen_dc);
    if ok == 0 {
        return None;
    }
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    Some((bgra, w, h))
}

fn upload_to_lens(png: Vec<u8>) {
    std::thread::spawn(move || {
        let boundary = "----OmniSearchLensBoundary";
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"encoded_image\"; filename=\"selection.png\"\r\n");
        body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
        body.extend_from_slice(&png);
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let res = ureq::AgentBuilder::new()
            .redirects(0)
            .build()
            .post("https://lens.google.com/v3/upload?hl=en")
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={}", boundary),
            )
            .set("User-Agent", "Mozilla/5.0 OmniSearch")
            .send_bytes(&body);

        let redirect = match res {
            Err(ureq::Error::Status(_, resp)) => resp.header("Location").map(str::to_string),
            Ok(resp) => resp.header("Location").map(str::to_string),
            Err(_) => None,
        };
        if let Some(url) = redirect {
            unsafe { open_url(&url) };
        }
    });
}

unsafe fn open_url(url: &str) {
    let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    let _ = ShellExecuteW(
        HWND::default(),
        w!("open"),
        PCWSTR(wide.as_ptr()),
        PCWSTR::null(),
        PCWSTR::null(),
        SW_SHOWNORMAL,
    );
}

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16))
}

fn loword(dw: u32) -> i32 {
    (dw & 0xFFFF) as i16 as i32
}

fn hiword(dw: u32) -> i32 {
    ((dw >> 16) & 0xFFFF) as i16 as i32
}
