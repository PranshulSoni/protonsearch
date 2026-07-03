use std::sync::OnceLock;
use image::codecs::png::PngEncoder;
use image::ExtendedColorType;
use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::*;

const LENS_BTN_SIZE: i32 = 20;
const OVERLAY_DIM_ALPHA: u8 = 120;

static LENS_ICON: OnceLock<HICON> = OnceLock::new();

pub fn get_lens_icon() -> HICON {
    *LENS_ICON.get_or_init(|| unsafe { load_lens_icon() })
}

unsafe fn load_lens_icon() -> HICON {
    let w: Vec<u16> = "imageres.dll\0".encode_utf16().collect();
    let mut hicon = HICON::default();
    let num = windows::Win32::UI::Shell::PrivateExtractIconsW(
        PCWSTR(w.as_ptr()),
        21,
        LENS_BTN_SIZE,
        LENS_BTN_SIZE,
        Some(&mut hicon as *mut HICON),
        None,
        1,
        0,
    );
    if num > 0 && !hicon.0.is_null() {
        hicon
    } else {
        HICON(std::ptr::null_mut())
    }
}

#[derive(Default)]
struct Selection {
    x1: i32, y1: i32,
    x2: i32, y2: i32,
    active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CapsuleMode {
    Image = 1,
    Text = 2,
    Translate = 3,
}

unsafe fn capture_screen() -> Option<(HBITMAP, i32, i32)> {
    let screen_dc = CreateDCW(windows::core::w!("DISPLAY"), None, None, None).ok()?;
    let w = GetDeviceCaps(screen_dc, HORZRES);
    let h = GetDeviceCaps(screen_dc, VERTRES);
    if w == 0 || h == 0 {
        let _ = DeleteDC(screen_dc);
        return None;
    }
    let bmp = CreateCompatibleBitmap(screen_dc, w, h);
    if bmp.is_invalid() {
        let _ = DeleteDC(screen_dc);
        return None;
    }
    let mem_dc = CreateCompatibleDC(screen_dc);
    if mem_dc.is_invalid() {
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(screen_dc);
        return None;
    }
    let old = SelectObject(mem_dc, bmp);
    let _ = BitBlt(mem_dc, 0, 0, w, h, screen_dc, 0, 0, SRCCOPY);
    let _ = SelectObject(mem_dc, old);
    let _ = DeleteDC(mem_dc);
    let _ = DeleteDC(screen_dc);
    Some((bmp, w, h))
}

unsafe fn create_overlay_window(
    hinst: HMODULE,
    bmp: HBITMAP,
    screen_w: i32,
    screen_h: i32,
) -> Option<HWND> {
    let class_name = "omnisearch-lens-overlay\0";
    let wname: Vec<u16> = class_name.encode_utf16().collect();

    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
        lpfnWndProc: Some(overlay_wndproc),
        hInstance: HINSTANCE(hinst.0),
        hCursor: HCURSOR(std::ptr::null_mut()),
        hbrBackground: HBRUSH(std::ptr::null_mut()),
        lpszClassName: PCWSTR(wname.as_ptr()),
        ..Default::default()
    };

    RegisterClassW(&wc);

    let data = Box::into_raw(Box::new(OverlayData {
        bmp,
        screen_w,
        screen_h,
        selection: Selection::default(),
        capsule_mode: CapsuleMode::Image,
        hovered_mode: None,
    }));

    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        PCWSTR(wname.as_ptr()),
        PCWSTR::null(),
        WS_POPUP | WS_VISIBLE,
        0, 0, screen_w, screen_h,
        None,
        None,
        HINSTANCE(hinst.0),
        Some(data as _),
    );

    if let Ok(h) = hwnd {
        SetWindowPos(h, HWND_TOPMOST, 0, 0, screen_w, screen_h, SWP_SHOWWINDOW);
        let _ = SetForegroundWindow(h);
        Some(h)
    } else {
        let _ = Box::from_raw(data);
        None
    }
}

struct OverlayData {
    bmp: HBITMAP,
    screen_w: i32,
    screen_h: i32,
    selection: Selection,
    capsule_mode: CapsuleMode,
    hovered_mode: Option<CapsuleMode>,
}

fn is_inside_capsule(screen_w: i32, screen_h: i32, x: i32, y: i32) -> bool {
    let cap_w = 330;
    let cap_h = 48;
    let cx = (screen_w - cap_w) / 2;
    let cy = screen_h - cap_h - 60;
    x >= cx && x < cx + cap_w && y >= cy && y < cy + cap_h
}

fn get_capsule_mode_at_point(screen_w: i32, screen_h: i32, x: i32, y: i32) -> Option<CapsuleMode> {
    let cap_w = 330;
    let cap_h = 48;
    let cx = (screen_w - cap_w) / 2;
    let cy = screen_h - cap_h - 60;
    if x >= cx + 6 && x < cx + 324 && y >= cy + 6 && y < cy + 42 {
        let bx = x - (cx + 6);
        let btn_w = 106;
        let idx = bx / btn_w;
        match idx {
            0 => Some(CapsuleMode::Image),
            1 => Some(CapsuleMode::Text),
            2 => Some(CapsuleMode::Translate),
            _ => None,
        }
    } else {
        None
    }
}

extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
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
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mem_dc = CreateCompatibleDC(hdc);
                let old = SelectObject(mem_dc, data.bmp);
                let _ = BitBlt(hdc, 0, 0, data.screen_w, data.screen_h, mem_dc, 0, 0, SRCCOPY);
                let _ = SelectObject(mem_dc, old);
                let _ = DeleteDC(mem_dc);

                let sel = &data.selection;
                let has_sel = sel.active
                    && (sel.x1 - sel.x2).abs() >= 5
                    && (sel.y1 - sel.y2).abs() >= 5;

                if has_sel {
                    let sx = sel.x1.min(sel.x2);
                    let sy = sel.y1.min(sel.y2);
                    let sw = (sel.x1 - sel.x2).abs();
                    let sh = (sel.y1 - sel.y2).abs();

                    dim_rects(hdc, data.screen_w, data.screen_h, sx, sy, sw, sh);

                    let pen = CreatePen(PS_SOLID, 2, RGB(255, 255, 255));
                    let old_pen = SelectObject(hdc, pen);
                    let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
                    let _ = Rectangle(hdc, sx, sy, sx + sw, sy + sh);
                    let _ = SelectObject(hdc, old_pen);
                    let _ = SelectObject(hdc, old_brush);
                    let _ = DeleteObject(pen);
                } else {
                    dim_rects(hdc, data.screen_w, data.screen_h, 0, 0, 0, 0);
                }

                // Draw floating capsule at the bottom center
                let cap_w = 330;
                let cap_h = 48;
                let cx = (data.screen_w - cap_w) / 2;
                let cy = data.screen_h - cap_h - 60;

                let cap_brush = CreateSolidBrush(RGB(0x22, 0x22, 0x22));
                let cap_pen = CreatePen(PS_SOLID, 1, RGB(0x44, 0x44, 0x44));
                let old_b = SelectObject(hdc, cap_brush);
                let old_p = SelectObject(hdc, cap_pen);
                let _ = RoundRect(hdc, cx, cy, cx + cap_w, cy + cap_h, 24, 24);
                let _ = SelectObject(hdc, old_b);
                let _ = SelectObject(hdc, old_p);
                let _ = DeleteObject(cap_brush);
                let _ = DeleteObject(cap_pen);

                let modes = [CapsuleMode::Image, CapsuleMode::Text, CapsuleMode::Translate];
                let mode_labels = ["Image", "Text", "Translate"];
                for (i, &mode) in modes.iter().enumerate() {
                    let bx = cx + 6 + (i as i32 * 106);
                    let by_btn = cy + 6;
                    let br_rect = RECT { left: bx, top: by_btn, right: bx + 106, bottom: by_btn + 36 };

                    let is_selected = mode == data.capsule_mode;
                    let is_hovered = Some(mode) == data.hovered_mode;

                    if is_selected {
                        let btn_brush = CreateSolidBrush(RGB(0x4F, 0x56, 0x66));
                        let btn_pen = CreatePen(PS_SOLID, 1, RGB(0x60, 0x68, 0x7A));
                        let old_b = SelectObject(hdc, btn_brush);
                        let old_p = SelectObject(hdc, btn_pen);
                        let _ = RoundRect(hdc, bx, by_btn, bx + 106, by_btn + 36, 14, 14);
                        let _ = SelectObject(hdc, old_b);
                        let _ = SelectObject(hdc, old_p);
                        let _ = DeleteObject(btn_brush);
                        let _ = DeleteObject(btn_pen);
                    } else if is_hovered {
                        let btn_brush = CreateSolidBrush(RGB(0x35, 0x39, 0x45));
                        let btn_pen = CreatePen(PS_SOLID, 1, RGB(0x44, 0x49, 0x57));
                        let old_b = SelectObject(hdc, btn_brush);
                        let old_p = SelectObject(hdc, btn_pen);
                        let _ = RoundRect(hdc, bx, by_btn, bx + 106, by_btn + 36, 14, 14);
                        let _ = SelectObject(hdc, old_b);
                        let _ = SelectObject(hdc, old_p);
                        let _ = DeleteObject(btn_brush);
                        let _ = DeleteObject(btn_pen);
                    }

                    let label = mode_labels[i];
                    let txt: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                    let mut tr = br_rect;
                    let _ = SetBkMode(hdc, TRANSPARENT);
                    let old_font = SelectObject(hdc, GetStockObject(DEFAULT_GUI_FONT));
                    let text_color = if is_selected {
                        RGB(255, 255, 255)
                    } else {
                        RGB(180, 180, 180)
                    };
                    let _ = SetTextColor(hdc, text_color);
                    let _ = DrawTextW(hdc, &txt, &mut tr, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
                    let _ = SelectObject(hdc, old_font);
                }

                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }

            WM_SETCURSOR => {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let _ = ScreenToClient(hwnd, &mut pt);

                if is_inside_capsule(data.screen_w, data.screen_h, pt.x, pt.y) {
                    if let Ok(cursor) = LoadCursorW(None, IDC_HAND) {
                        SetCursor(cursor);
                        return LRESULT(1);
                    }
                } else {
                    if let Ok(cursor) = LoadCursorW(None, IDC_CROSS) {
                        SetCursor(cursor);
                        return LRESULT(1);
                    }
                }
                DefWindowProcW(hwnd, msg, wp, lp)
            }

            WM_LBUTTONDOWN => {
                let x = LOWORD(lp.0 as u32) as i32;
                let y = HIWORD(lp.0 as u32) as i32;

                if is_inside_capsule(data.screen_w, data.screen_h, x, y) {
                    if let Some(mode) = get_capsule_mode_at_point(data.screen_w, data.screen_h, x, y) {
                        data.capsule_mode = mode;
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                    return LRESULT(0);
                }

                data.selection.x1 = x;
                data.selection.y1 = y;
                data.selection.x2 = x;
                data.selection.y2 = y;
                data.selection.active = true;
                let _ = SetCapture(hwnd);
                let _ = InvalidateRect(hwnd, None, FALSE);
                LRESULT(0)
            }

            WM_MOUSEMOVE => {
                let x = LOWORD(lp.0 as u32) as i32;
                let y = HIWORD(lp.0 as u32) as i32;

                if data.selection.active {
                    data.selection.x2 = x;
                    data.selection.y2 = y;
                    let _ = InvalidateRect(hwnd, None, FALSE);
                } else {
                    let hovered = get_capsule_mode_at_point(data.screen_w, data.screen_h, x, y);
                    if data.hovered_mode != hovered {
                        data.hovered_mode = hovered;
                        let _ = InvalidateRect(hwnd, None, FALSE);
                    }
                }
                LRESULT(0)
            }

            WM_LBUTTONUP => {
                if data.selection.active {
                    let _ = ReleaseCapture();
                    data.selection.x2 = LOWORD(lp.0 as u32) as i32;
                    data.selection.y2 = HIWORD(lp.0 as u32) as i32;
                    data.selection.active = false;

                    let sx = data.selection.x1.min(data.selection.x2);
                    let sy = data.selection.y1.min(data.selection.y2);
                    let sw = (data.selection.x1 - data.selection.x2).abs();
                    let sh = (data.selection.y1 - data.selection.y2).abs();

                    if sw < 5 || sh < 5 {
                        data.selection = Selection::default();
                        let _ = InvalidateRect(hwnd, None, FALSE);
                        return LRESULT(0);
                    }

                    let cropped = crop_bitmap(data.bmp, sx, sy, sw, sh, data.screen_w, data.screen_h);
                    let _ = DestroyWindow(hwnd);
                    PostQuitMessage(0);

                    if let Some(hbmp) = cropped {
                        process_selection(hbmp, data.capsule_mode as u32);
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

unsafe fn dim_rects(hdc: HDC, sw: i32, sh: i32, sx: i32, sy: i32, sw_sel: i32, sh_sel: i32) {
    let black_dc = CreateCompatibleDC(hdc);
    let black_bmp = CreateCompatibleBitmap(hdc, 1, 1);
    if black_bmp.is_invalid() {
        let _ = DeleteDC(black_dc);
        return;
    }
    let old = SelectObject(black_dc, black_bmp);
    let _ = PatBlt(black_dc, 0, 0, 1, 1, BLACKNESS);

    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER,
        BlendFlags: 0,
        SourceConstantAlpha: OVERLAY_DIM_ALPHA,
        AlphaFormat: 0,
    };

    if sw_sel == 0 || sh_sel == 0 {
        let _ = AlphaBlend(hdc, 0, 0, sw, sh, black_dc, 0, 0, 1, 1, blend);
    } else {
        if sy > 0 {
            let _ = AlphaBlend(hdc, 0, 0, sw, sy, black_dc, 0, 0, 1, 1, blend);
        }
        if sy + sh_sel < sh {
            let _ = AlphaBlend(hdc, 0, sy + sh_sel, sw, sh - sy - sh_sel, black_dc, 0, 0, 1, 1, blend);
        }
        if sx > 0 {
            let _ = AlphaBlend(hdc, 0, sy, sx, sh_sel, black_dc, 0, 0, 1, 1, blend);
        }
        if sx + sw_sel < sw {
            let _ = AlphaBlend(hdc, sx + sw_sel, sy, sw - sx - sw_sel, sh_sel, black_dc, 0, 0, 1, 1, blend);
        }
    }

    let _ = SelectObject(black_dc, old);
    let _ = DeleteObject(black_bmp);
    let _ = DeleteDC(black_dc);
}

unsafe fn crop_bitmap(
    src: HBITMAP,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    src_w: i32,
    src_h: i32,
) -> Option<HBITMAP> {
    let screen_dc = GetDC(None);
    let mem_dc = CreateCompatibleDC(screen_dc);
    let old = SelectObject(mem_dc, src);

    let cropped = CreateCompatibleBitmap(screen_dc, w, h);
    if cropped.is_invalid() {
        let _ = SelectObject(mem_dc, old);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
        return None;
    }

    let crop_dc = CreateCompatibleDC(screen_dc);
    let old2 = SelectObject(crop_dc, cropped);
    let _ = BitBlt(crop_dc, 0, 0, w, h, mem_dc, x, y, SRCCOPY);
    let _ = SelectObject(crop_dc, old2);
    let _ = DeleteDC(crop_dc);
    let _ = SelectObject(mem_dc, old);
    let _ = DeleteDC(mem_dc);
    let _ = ReleaseDC(None, screen_dc);
    Some(cropped)
}

fn process_selection(hbmp: HBITMAP, mode: u32) {
    unsafe {
        let pixels = bitmap_to_rgba(hbmp);
        let _ = DeleteObject(hbmp);
        let (data, w, h) = match pixels {
            Some(p) => p,
            None => return,
        };
        match mode {
            1 => process_image(&data, w, h),
            2 => process_text(&data, w, h),
            3 => process_translate(&data, w, h),
            _ => {}
        }
    }
}

unsafe fn bitmap_to_rgba(hbmp: HBITMAP) -> Option<(Vec<u8>, u32, u32)> {
    let screen_dc = GetDC(None);
    let mem_dc = CreateCompatibleDC(screen_dc);
    let old = SelectObject(mem_dc, hbmp);

    let mut bmp = BITMAP::default();
    if GetObjectW(hbmp, std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp as *mut _ as *mut _)) == 0 {
        let _ = SelectObject(mem_dc, old);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
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
    bmi.bmiHeader.biCompression = BI_RGB;

    let row_size = (w * 4) as usize;
    let total = (row_size * h as usize) as usize;
    let mut bgra = vec![0u8; total];

    let res = GetDIBits(
        mem_dc, hbmp, 0, h,
        Some(bgra.as_mut_ptr() as *mut _),
        &mut bmi, DIB_RGB_COLORS,
    );

    let _ = SelectObject(mem_dc, old);
    let _ = DeleteDC(mem_dc);
    let _ = ReleaseDC(None, screen_dc);

    if res == 0 {
        return None;
    }

    for chunk in bgra.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    Some((bgra, w, h))
}

unsafe fn process_image(data: &[u8], w: u32, h: u32) {
    let mut png = Vec::new();
    if PngEncoder::new(&mut png)
        .encode(data, w, h, ExtendedColorType::Rgba8)
        .is_err()
    {
        return;
    }

    let b64 = base64_encode(&png);

    let html = format!(
        r#"<!DOCTYPE html><html><body>
<form id="f" action="https://lens.google.com/v3/upload?hl=en" method="POST" enctype="multipart/form-data">
<input type="file" id="i" name="encoded_image" accept="image/*">
</form>
<script>
var raw = atob('{}');
var arr = new Uint8Array(raw.length);
for (var i = 0; i < raw.length; i++) arr[i] = raw.charCodeAt(i);
var blob = new Blob([arr], {{ type: 'image/png' }});
var file = new File([blob], 'screenshot.png', {{ type: 'image/png' }});
var dt = new DataTransfer();
dt.items.add(file);
var fi = document.getElementById('i');
fi.files = dt.files;
var ev = new Event('change', {{ bubbles: true }});
fi.dispatchEvent(ev);
document.getElementById('f').submit();
</script></body></html>"#,
        b64
    );

    let temp_dir = std::env::temp_dir();
    let html_path = temp_dir.join("omnisearch_lens.html");
    let _ = std::fs::write(&html_path, &html);

    let path_str: Vec<u16> = html_path.to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let _ = ShellExecuteW(
        None,
        &windows::core::w!("open"),
        PCWSTR(path_str.as_ptr()),
        None,
        None,
        SW_SHOWNORMAL,
    );

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(10));
        let _ = std::fs::remove_file(&html_path);
    });
}

unsafe fn process_text(data: &[u8], w: u32, h: u32) {
    if let Some(text) = ocr_from_rgba(data, w, h) {
        let encoded = urlencoding(&text);
        let url_str = format!("https://www.google.com/search?q={}", encoded);
        let url: Vec<u16> = url_str.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = ShellExecuteW(None, &windows::core::w!("open"), PCWSTR(url.as_ptr()), None, None, SW_SHOWNORMAL);
    }
}

unsafe fn process_translate(data: &[u8], w: u32, h: u32) {
    if let Some(text) = ocr_from_rgba(data, w, h) {
        let encoded = urlencoding(&text);
        let url_str = format!("https://translate.google.com/?sl=auto&text={}&op=translate", encoded);
        let url: Vec<u16> = url_str.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = ShellExecuteW(None, &windows::core::w!("open"), PCWSTR(url.as_ptr()), None, None, SW_SHOWNORMAL);
    }
}

unsafe fn ocr_from_rgba(rgba: &[u8], w: u32, h: u32) -> Option<String> {
    use windows::Graphics::Imaging::*;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::*;

    let mut png = Vec::new();
    PngEncoder::new(&mut png).encode(rgba, w, h, ExtendedColorType::Rgba8).ok()?;

    let stream = InMemoryRandomAccessStream::Create().ok()?;
    let writer = DataWriter::CreateDataWriter(&stream).ok()?;
    writer.WriteBytes(&png).ok()?;
    writer.FlushAsync().ok()?.get().ok()?;
    stream.Seek(0).ok()?;

    let decoder = BitmapDecoder::CreateAsync(&stream).ok()?.get().ok()?;

    let raw_bitmap = decoder.GetSoftwareBitmapAsync().ok()?.get().ok()?;

    let fmt_ok = raw_bitmap.BitmapPixelFormat().ok() == Some(BitmapPixelFormat::Bgra8);
    let alpha_ok = raw_bitmap.BitmapAlphaMode().ok() == Some(BitmapAlphaMode::Premultiplied);
    let sb = if fmt_ok && alpha_ok {
        raw_bitmap
    } else {
        SoftwareBitmap::ConvertWithAlpha(&raw_bitmap, BitmapPixelFormat::Bgra8, BitmapAlphaMode::Premultiplied).ok()?
    };

    let engine = OcrEngine::TryCreateFromUserProfileLanguages().ok()?;
    let result = engine.RecognizeAsync(&sb).ok()?.get().ok()?;
    let text = result.Text().ok()?;
    let s = text.to_string();
    if s.trim().is_empty() { None } else { Some(s) }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        if chunk.len() == 3 {
            let triple = (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8 | chunk[2] as u32;
            result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else if chunk.len() == 2 {
            let triple = (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8;
            result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
            result.push('=');
        } else {
            let triple = (chunk[0] as u32) << 16;
            result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            result.push('=');
            result.push('=');
        }
    }
    result
}

fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}

pub fn start_lens_search(hwnd: HWND, s: &mut crate::State) {
    unsafe {
        let hinst = GetModuleHandleW(None).unwrap();
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
    }
}

unsafe fn LOWORD(dw: u32) -> i32 {
    (dw & 0xFFFF) as i32
}

unsafe fn HIWORD(dw: u32) -> i32 {
    ((dw >> 16) & 0xFFFF) as i32
}
