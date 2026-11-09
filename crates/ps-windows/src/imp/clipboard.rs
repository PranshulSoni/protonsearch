//! Win32 clipboard access and the background clipboard-change listener.

use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;

use anyhow::{anyhow, bail, Context};
use windows::core::w;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard,
    SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    PostThreadMessageW, RegisterClassW, TranslateMessage, HMENU, MSG, WINDOW_EX_STYLE,
    WINDOW_STYLE, WNDCLASSW, WM_DESTROY, WM_QUIT,
};

use super::util::ComGuard;
use crate::{ClipboardImage, ClipboardListener, ClipboardSnapshot};

/// CF_UNICODETEXT — stable Win32 clipboard format id.
const CF_UNICODETEXT: u32 = 13;
/// CF_DIB — device-independent bitmap.
const CF_DIB: u32 = 8;
const WM_CLIPBOARDUPDATE: u32 = 0x031D;

/// The listener callback is process-global: only one listener may be active.
static CALLBACK: Mutex<Option<Box<dyn Fn(ClipboardSnapshot) + Send + 'static>>> = Mutex::new(None);

fn open_clipboard_retry() -> anyhow::Result<()> {
    // Another process can hold the clipboard open for a few milliseconds.
    for _ in 0..12 {
        if unsafe { OpenClipboard(HWND::default()) }.is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    bail!("could not open the clipboard (it is held by another process)")
}

pub(crate) fn set_text(text: &str) -> anyhow::Result<()> {
    open_clipboard_retry()?;
    let result = set_text_inner(text);
    unsafe { CloseClipboard() }.ok();
    result
}

fn set_text_inner(text: &str) -> anyhow::Result<()> {
    unsafe {
        EmptyClipboard().context("EmptyClipboard")?;
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = wide.len() * 2;
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes).context("GlobalAlloc(clipboard text)")?;
        let pointer = GlobalLock(handle);
        if pointer.is_null() {
            let _ = GlobalFree(handle);
            bail!("GlobalLock failed for clipboard text");
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, pointer as *mut u8, bytes);
        let _ = GlobalUnlock(handle);
        if let Err(error) = SetClipboardData(CF_UNICODETEXT, HANDLE(handle.0)) {
            let _ = GlobalFree(handle);
            return Err(anyhow!("SetClipboardData failed: {error}"));
        }
    }
    Ok(())
}

pub(crate) fn get_text() -> Option<String> {
    open_clipboard_retry().ok()?;
    let result = get_text_inner();
    unsafe { CloseClipboard() }.ok();
    result
}

fn get_text_inner() -> Option<String> {
    unsafe {
        let handle = GetClipboardData(CF_UNICODETEXT).ok()?;
        if handle.0.is_null() {
            return None;
        }
        let hglobal = HGLOBAL(handle.0);
        let pointer = GlobalLock(hglobal);
        if pointer.is_null() {
            return None;
        }
        let size = GlobalSize(hglobal);
        let units = if size >= 2 { size / 2 } else { 0 };
        let slice = std::slice::from_raw_parts(pointer as *const u16, units);
        let end = slice.iter().position(|&c| c == 0).unwrap_or(units);
        let text = String::from_utf16_lossy(&slice[..end]);
        let _ = GlobalUnlock(hglobal);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

pub(crate) fn get_image() -> Option<ClipboardImage> {
    open_clipboard_retry().ok()?;
    let result = get_image_inner();
    unsafe { CloseClipboard() }.ok();
    result
}

fn get_image_inner() -> Option<ClipboardImage> {
    unsafe {
        let handle = GetClipboardData(CF_DIB).ok()?;
        if handle.0.is_null() {
            return None;
        }
        let hglobal = HGLOBAL(handle.0);
        let pointer = GlobalLock(hglobal);
        if pointer.is_null() {
            return None;
        }
        let size = GlobalSize(hglobal);
        let decoded = decode_dib(pointer as *const u8, size);
        let _ = GlobalUnlock(hglobal);
        decoded
    }
}

/// Decode a packed CF_DIB (BITMAPINFOHEADER + uncompressed pixels) to RGBA.
fn decode_dib(pointer: *const u8, size: usize) -> Option<ClipboardImage> {
    if pointer.is_null() || size < std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAPINFOHEADER>() {
        return None;
    }
    unsafe {
        let header = &*(pointer as *const windows::Win32::Graphics::Gdi::BITMAPINFOHEADER);
        let header_size = header.biSize as usize;
        if header_size < std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAPINFOHEADER>() {
            return None;
        }
        let width = header.biWidth;
        let height_field = header.biHeight;
        let bit_count = header.biBitCount;
        let compression = header.biCompression;
        // Only uncompressed RGB/RGBA bitmaps are supported (screenshots, Snipping Tool).
        if width <= 0 || height_field == 0 || compression != 0 {
            return None;
        }
        if !matches!(bit_count, 24 | 32) {
            return None;
        }
        let abs_height = height_field.unsigned_abs() as usize;
        let width_usize = width as usize;
        let bytes_per_pixel = (bit_count / 8) as usize;
        let stride = (width_usize * bytes_per_pixel).div_ceil(4) * 4;
        let required = header_size.checked_add(stride.checked_mul(abs_height)?)?;
        if required > size {
            return None;
        }
        let pixels = pointer.add(header_size);
        let mut rgba = vec![0u8; width_usize * abs_height * 4];
        let bottom_up = height_field > 0;
        let mut any_alpha = false;
        for y in 0..abs_height {
            let src_row = if bottom_up { abs_height - 1 - y } else { y };
            let src = pixels.add(src_row * stride);
            for x in 0..width_usize {
                let sp = src.add(x * bytes_per_pixel);
                let (b, g, r, a) = if bytes_per_pixel == 4 {
                    let a = *sp.add(3);
                    if a != 0 {
                        any_alpha = true;
                    }
                    (*sp, *sp.add(1), *sp.add(2), a)
                } else {
                    (*sp, *sp.add(1), *sp.add(2), 255)
                };
                let dst = (y * width_usize + x) * 4;
                rgba[dst] = r;
                rgba[dst + 1] = g;
                rgba[dst + 2] = b;
                rgba[dst + 3] = a;
            }
        }
        if bytes_per_pixel == 4 && !any_alpha {
            for px in rgba.chunks_exact_mut(4) {
                px[3] = 255;
            }
        }
        Some(ClipboardImage {
            width: width_usize as u32,
            height: abs_height as u32,
            rgba,
        })
    }
}

pub(crate) fn get_image_png() -> Option<Vec<u8>> {
    let image = get_image()?;
    encode_png(&image)
}

fn encode_png(image: &ClipboardImage) -> Option<Vec<u8>> {
    let buffer =
        image::RgbaImage::from_raw(image.width, image.height, image.rgba.clone())?;
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    image::DynamicImage::ImageRgba8(buffer)
        .write_to(&mut cursor, image::ImageOutputFormat::Png)
        .ok()?;
    Some(out)
}

/// Capture one clipboard snapshot (text and/or image + source app).
pub(crate) fn snapshot() -> ClipboardSnapshot {
    let text = get_text();
    let png = if text.is_none() {
        get_image_png()
    } else {
        None
    };
    ClipboardSnapshot {
        text,
        png,
        source_app: super::window::foreground_process_name(),
        timestamp_ms: now_ms(),
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Clipboard listener
// ---------------------------------------------------------------------------

pub(crate) fn start_listener<F>(callback: F) -> anyhow::Result<ClipboardListener>
where
    F: Fn(ClipboardSnapshot) + Send + 'static,
{
    {
        let mut slot = CALLBACK.lock().map_err(|_| anyhow!("clipboard listener lock poisoned"))?;
        if slot.is_some() {
            bail!("a clipboard listener is already running");
        }
        *slot = Some(Box::new(callback));
    }
    let (ready_tx, ready_rx) = mpsc::channel::<anyhow::Result<u32>>();
    let join = thread::Builder::new()
        .name("ps-clipboard-listener".into())
        .spawn(move || listener_thread(ready_tx))
        .context("spawning clipboard listener thread")?;
    match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(thread_id)) => Ok(ClipboardListener {
            thread_id,
            join: Some(join),
        }),
        Ok(Err(error)) => {
            clear_callback();
            let _ = join.join();
            Err(error)
        }
        Err(_) => {
            clear_callback();
            Err(anyhow!("clipboard listener did not start in time"))
        }
    }
}

pub(crate) fn stop_listener(thread_id: u32) {
    unsafe {
        let _ = PostThreadMessageW(
            thread_id,
            WM_QUIT,
            WPARAM(0),
            LPARAM(0),
        );
    }
}

fn clear_callback() {
    if let Ok(mut slot) = CALLBACK.lock() {
        *slot = None;
    }
}

fn listener_thread(ready: mpsc::Sender<anyhow::Result<u32>>) {
    let _com = ComGuard::new();
    let thread_id = unsafe { GetCurrentThreadId() };
    let result = unsafe { create_listener_window(thread_id) };
    match result {
        Ok(()) => {
            let _ = ready.send(Ok(thread_id));
        }
        Err(error) => {
            let _ = ready.send(Err(error));
            clear_callback();
            return;
        }
    }
    unsafe {
        let mut message = MSG::default();
        while GetMessageW(&mut message, HWND::default(), 0, 0).0 > 0 {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    clear_callback();
}

unsafe fn create_listener_window(_thread_id: u32) -> anyhow::Result<()> {
    let instance = GetModuleHandleW(None).context("GetModuleHandleW")?;
    let class_name = w!("ProtonSearchClipboardListener");
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: instance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    let atom = RegisterClassW(&window_class);
    if atom == 0 {
        // May already be registered from a previous listener; continue.
        let error = windows::core::Error::from_win32();
        if error.code().0 & 0xFFFF != 0x0583 {
            // ERROR_CLASS_ALREADY_EXISTS (1410) is tolerable, anything else is not.
            return Err(anyhow!("RegisterClassW failed: {error}"));
        }
    }
    let window = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class_name,
        w!("ProtonSearch Clipboard Listener"),
        WINDOW_STYLE(0),
        0,
        0,
        0,
        0,
        HWND::default(),
        HMENU::default(),
        HINSTANCE(instance.0),
        None,
    )
    .context("CreateWindowExW(clipboard listener)")?;
    if window.is_invalid() {
        bail!("clipboard listener window was not created");
    }
    AddClipboardFormatListener(window).context("AddClipboardFormatListener")?;
    Ok(())
}

unsafe extern "system" fn wndproc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CLIPBOARDUPDATE => {
            let snapshot = snapshot();
            if snapshot.text.is_some() || snapshot.png.is_some() {
                if let Ok(slot) = CALLBACK.lock() {
                    if let Some(callback) = slot.as_ref() {
                        callback(snapshot);
                    }
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}
