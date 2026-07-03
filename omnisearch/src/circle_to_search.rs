use crate::{ReleaseCapture, SetCapture, VK_ESCAPE};
use image::{codecs::png::PngEncoder, ColorType};
use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::*;

struct OverlayData {
    bmp: HBITMAP,
    screen_w: i32,
    screen_h: i32,
    points: Vec<POINT>,
    active: bool,
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
        points: Vec::new(),
        active: false,
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
                data.points.clear();
                data.points.push(POINT { x, y });
                data.active = true;
                let _ = SetCapture(hwnd);
                let _ = InvalidateRect(hwnd, None, FALSE);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                if data.active {
                    let x = loword(lp.0 as u32);
                    let y = hiword(lp.0 as u32);
                    let last = data.points.last().cloned();
                    if let Some(l) = last {
                        let dx = (l.x - x).abs();
                        let dy = (l.y - y).abs();
                        // Only add point if mouse moved at least 2 pixels to keep polygon size sane
                        if dx > 2 || dy > 2 {
                            data.points.push(POINT { x, y });
                            let _ = InvalidateRect(hwnd, None, FALSE);
                        }
                    }
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                if data.active {
                    let _ = ReleaseCapture();
                    data.active = false;
                    let x = loword(lp.0 as u32);
                    let y = hiword(lp.0 as u32);
                    data.points.push(POINT { x, y });

                    let mut min_x = i32::MAX;
                    let mut min_y = i32::MAX;
                    let mut max_x = i32::MIN;
                    let mut max_y = i32::MIN;
                    for p in &data.points {
                        min_x = min_x.min(p.x);
                        min_y = min_y.min(p.y);
                        max_x = max_x.max(p.x);
                        max_y = max_y.max(p.y);
                    }
                    let sx = min_x.max(0);
                    let sy = min_y.max(0);
                    let sw = (max_x - min_x).min(data.screen_w - sx);
                    let sh = (max_y - min_y).min(data.screen_h - sy);

                    if sw >= 8 && sh >= 8 && data.points.len() >= 3 {
                        if let Some(cropped) = crop_bitmap(data.bmp, sx, sy, sw, sh) {
                            process_image_masked(cropped, &data.points, sx, sy);
                        }
                        let _ = DestroyWindow(hwnd);
                        PostQuitMessage(0);
                    } else {
                        data.points.clear();
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
    
    // Draw the bright screenshot base
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

    // Create a darken shade overlay
    let shade_dc = CreateCompatibleDC(hdc);
    let shade_bmp = CreateCompatibleBitmap(hdc, data.screen_w, data.screen_h);
    let old_shade = SelectObject(shade_dc, shade_bmp);
    let shade = CreateSolidBrush(rgb(0, 0, 0));
    let old_brush = SelectObject(shade_dc, shade);
    let _ = PatBlt(shade_dc, 0, 0, data.screen_w, data.screen_h, BLACKNESS);
    let _ = SelectObject(shade_dc, old_brush);
    
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 118,
        AlphaFormat: 0,
    };
    
    let _ = AlphaBlend(
        hdc,
        0,
        0,
        data.screen_w,
        data.screen_h,
        shade_dc,
        0,
        0,
        data.screen_w,
        data.screen_h,
        blend,
    );
    let _ = SelectObject(shade_dc, old_shade);
    let _ = DeleteObject(shade);
    let _ = DeleteObject(shade_bmp);
    let _ = DeleteDC(shade_dc);

    // If the user has drawn a path, highlight the interior and draw the outline
    if data.points.len() > 1 {
        // Create region for polygon
        let hrgn = CreatePolygonRgn(&data.points, WINDING);
        if !hrgn.is_invalid() {
            // Clip hdc drawing to only the interior of the polygon
            let _ = SelectClipRgn(hdc, hrgn);
            // Redraw the bright screenshot inside the polygon
            let _ = BitBlt(hdc, 0, 0, data.screen_w, data.screen_h, mem_dc, 0, 0, SRCCOPY);
            // Restore clipping region
            let _ = SelectClipRgn(hdc, HRGN::default());
            let _ = DeleteObject(hrgn);
        }

        // Draw the glowing lasso pen outline
        let pen = CreatePen(PS_SOLID, 4, rgb(83, 189, 255));
        let old_pen = SelectObject(hdc, pen);
        let _ = Polyline(hdc, &data.points);
        let _ = SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen);
    }

    // Draw the instructions top pill
    let pill_w = 480.min(data.screen_w - 32);
    let pill_h = 42;
    let pill_x = (data.screen_w - pill_w) / 2;
    let pill_y = 24;
    let pill_bg = CreateSolidBrush(rgb(22, 24, 28));
    let old_pill_brush = SelectObject(hdc, pill_bg);
    let old_pill_pen = SelectObject(hdc, GetStockObject(NULL_PEN));
    let _ = RoundRect(
        hdc,
        pill_x,
        pill_y,
        pill_x + pill_w,
        pill_y + pill_h,
        18,
        18,
    );
    let _ = SelectObject(hdc, old_pill_pen);
    let _ = SelectObject(hdc, old_pill_brush);
    let _ = DeleteObject(pill_bg);

    let mut text: Vec<u16> = "Draw a circle or lasso around anything to search. Esc cancels."
        .encode_utf16()
        .collect();
    let mut rect = RECT {
        left: pill_x + 18,
        top: pill_y,
        right: pill_x + pill_w - 18,
        bottom: pill_y + pill_h,
    };
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, rgb(238, 238, 238));
    let _ = DrawTextW(
        hdc,
        &mut text,
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );

    let _ = SelectObject(mem_dc, old);
    let _ = DeleteDC(mem_dc);
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

unsafe fn process_image_masked(hbmp: HBITMAP, points: &[POINT], min_x: i32, min_y: i32) {
    let Some((rgba, w, h)) = bitmap_to_rgba_masked(hbmp, points, min_x, min_y) else {
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

unsafe fn bitmap_to_rgba_masked(
    hbmp: HBITMAP,
    points: &[POINT],
    min_x: i32,
    min_y: i32,
) -> Option<(Vec<u8>, u32, u32)> {
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
    if ok == 0 {
        let _ = SelectObject(mem_dc, old);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(HWND::default(), screen_dc);
        return None;
    }

    // 2. Create GDI polygon mask
    let mask_dc = CreateCompatibleDC(screen_dc);
    let mask_bmp = CreateCompatibleBitmap(screen_dc, w as i32, h as i32);
    let old_mask = SelectObject(mask_dc, mask_bmp);
    let _ = PatBlt(mask_dc, 0, 0, w as i32, h as i32, BLACKNESS);

    let white_brush = CreateSolidBrush(rgb(255, 255, 255));
    let old_brush = SelectObject(mask_dc, white_brush);
    let old_pen = SelectObject(mask_dc, GetStockObject(NULL_PEN));

    let local_points: Vec<POINT> = points
        .iter()
        .map(|p| POINT {
            x: p.x - min_x,
            y: p.y - min_y,
        })
        .collect();

    let _ = SetPolyFillMode(mask_dc, WINDING);
    let _ = Polygon(mask_dc, &local_points);

    // 3. Read mask bits
    let mut mask_bgra = vec![0u8; (w * h * 4) as usize];
    let _ = GetDIBits(
        mask_dc,
        mask_bmp,
        0,
        h,
        Some(mask_bgra.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );

    let _ = SelectObject(mask_dc, old_pen);
    let _ = SelectObject(mask_dc, old_brush);
    let _ = DeleteObject(white_brush);
    let _ = SelectObject(mask_dc, old_mask);
    let _ = DeleteObject(mask_bmp);
    let _ = DeleteDC(mask_dc);

    let _ = SelectObject(mem_dc, old);
    let _ = DeleteDC(mem_dc);
    let _ = ReleaseDC(HWND::default(), screen_dc);

    // 4. Apply mask: make pixels outside lasso transparent
    for i in 0..(w * h) as usize {
        let offset = i * 4;
        let mask_val = mask_bgra[offset]; // Blue channel of mask
        if mask_val < 128 {
            bgra[offset + 3] = 0; // Transparent
        } else {
            bgra[offset + 3] = 255; // Opaque
        }
        bgra.swap(offset, offset + 2); // BGRA -> RGBA
    }

    Some((bgra, w, h))
}

fn upload_to_lens(png: Vec<u8>) {
    let html_path =
        std::env::temp_dir().join(format!("omnisearch_lens_{}.html", std::process::id()));
    let html = lens_upload_html(&base64_encode(&png));
    if std::fs::write(&html_path, html).is_err() {
        return;
    }
    unsafe {
        open_path(&html_path);
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let _ = std::fs::remove_file(html_path);
    });
}

fn lens_upload_html(b64_png: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
<body>
<form id="f" action="https://lens.google.com/v3/upload?hl=en" method="POST" enctype="multipart/form-data">
  <input id="i" name="encoded_image" type="file" accept="image/*">
</form>
<script>
const raw = atob("{}");
const bytes = new Uint8Array(raw.length);
for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
const file = new File([new Blob([bytes], {{ type: "image/png" }})], "selection.png", {{ type: "image/png" }});
const dt = new DataTransfer();
dt.items.add(file);
document.getElementById("i").files = dt.files;
document.getElementById("f").submit();
</script>
</body>
</html>"#,
        b64_png
    )
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            CHARS[((triple >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[(triple & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

unsafe fn open_path(path: &std::path::Path) {
    let wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let _ = ShellExecuteW(
        HWND::default(),
        windows::core::w!("open"),
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
