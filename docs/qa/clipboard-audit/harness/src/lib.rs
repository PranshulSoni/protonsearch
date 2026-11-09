//! Shared helpers for the Gate 0 clipboard-image audit harness.
//!
//! The harness deliberately lives OUTSIDE the product crates: Gate 0 is
//! read-only with respect to crates/**, and the report/evidence lives under
//! docs/qa/clipboard-audit. It exercises the real public ps-windows /
//! ps-sources APIs and uses raw Win32 only to *simulate other apps* placing
//! data on the clipboard.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND, POINT, GlobalFree};
use windows::Win32::Graphics::Gdi::{
    BI_BITFIELDS, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BITMAPV5HEADER, CreateCompatibleBitmap,
    DIB_RGB_COLORS, DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SetDIBits,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardFormatNameW, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::UI::Shell::DROPFILES;

pub const CF_TEXT: u32 = 1;
pub const CF_BITMAP: u32 = 2;
pub const CF_DIB: u32 = 8;
pub const CF_UNICODETEXT: u32 = 13;
pub const CF_HDROP: u32 = 15;
pub const CF_DIBV5: u32 = 17;

pub type R<T> = Result<T, String>;

fn werr<E: std::fmt::Display>(what: &str, error: E) -> String {
    format!("{what}: {error}")
}

pub fn open_clipboard() -> R<()> {
    unsafe { OpenClipboard(HWND::default()) }.map_err(|e| werr("OpenClipboard", e))
}

pub fn close_clipboard() {
    unsafe { CloseClipboard().ok() };
}

pub fn empty_clipboard() -> R<()> {
    unsafe { EmptyClipboard() }.map_err(|e| werr("EmptyClipboard", e))
}

/// Allocate a movable HGLOBAL, copy bytes into it, return the handle.
fn global_from_bytes(bytes: &[u8]) -> R<HGLOBAL> {
    let handle =
        unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }.map_err(|e| werr("GlobalAlloc", e))?;
    let pointer = unsafe { GlobalLock(handle) };
    if pointer.is_null() {
        unsafe { GlobalFree(handle).ok() };
        return Err("GlobalLock returned null".to_string());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer as *mut u8, bytes.len());
        GlobalUnlock(handle).ok();
    }
    Ok(handle)
}

fn set_format_raw(format: u32, bytes: &[u8]) -> R<()> {
    let handle = global_from_bytes(bytes)?;
    let result = unsafe { SetClipboardData(format, HANDLE(handle.0)) };
    match result {
        Ok(_) => Ok(()),
        Err(error) => {
            unsafe { GlobalFree(handle).ok() };
            Err(werr(&format!("SetClipboardData(format={format})"), error))
        }
    }
}

fn set_handle_format(format: u32, handle: HANDLE) -> R<()> {
    match unsafe { SetClipboardData(format, handle) } {
        Ok(_) => Ok(()),
        Err(error) => {
            unsafe { GlobalFree(HGLOBAL(handle.0)).ok() };
            Err(werr(&format!("SetClipboardData(format={format})"), error))
        }
    }
}

// ---------------------------------------------------------------------------
// Synthetic bitmap payloads (the "other application" side of the clipboard)
// ---------------------------------------------------------------------------

pub fn dib_variant_pixels(width: u32, height: u32, bpp: u16, alpha: u8) -> Vec<u8> {
    let bytes_per_pixel = (bpp / 8) as usize;
    let stride = ((width as usize * bytes_per_pixel).div_ceil(4)) * 4;
    let mut pixels = vec![0u8; stride * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let r = (x * 255 / width.max(1) as usize) as u8;
            let g = (y * 255 / height.max(1) as usize) as u8;
            let b = ((x + y) % 256) as u8;
            let a = if bytes_per_pixel == 4 { alpha } else { 255 };
            let offset = y * stride + x * bytes_per_pixel;
            pixels[offset] = b;
            pixels[offset + 1] = g;
            pixels[offset + 2] = r;
            if bytes_per_pixel == 4 {
                pixels[offset + 3] = a;
            }
        }
    }
    pixels
}

pub fn dib_bytes(width: i32, height: i32, bpp: u16, bottom_up: bool, alpha: u8) -> Vec<u8> {
    let width_usize = width.unsigned_abs() as usize;
    let height_usize = height.unsigned_abs() as usize;
    let bytes_per_pixel = (bpp / 8) as usize;
    let stride = ((width_usize * bytes_per_pixel).div_ceil(4)) * 4;
    let pixels = dib_variant_pixels(width_usize as u32, height_usize as u32, bpp, alpha);
    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: if bottom_up { height } else { -height },
        biPlanes: 1,
        biBitCount: bpp,
        biCompression: BI_RGB.0,
        biSizeImage: (stride * height_usize) as u32,
        ..Default::default()
    };
    let mut out = Vec::with_capacity(std::mem::size_of::<BITMAPINFOHEADER>() + pixels.len());
    out.extend_from_slice(unsafe {
        std::slice::from_raw_parts(&header as *const _ as *const u8, std::mem::size_of::<BITMAPINFOHEADER>())
    });
    out.extend_from_slice(&pixels);
    out
}

pub fn dibv5_bytes(width: i32, height: i32, bitfields: bool) -> Vec<u8> {
    let width_usize = width.unsigned_abs() as usize;
    let height_usize = height.unsigned_abs() as usize;
    let stride = ((width_usize * 4).div_ceil(4)) * 4;
    let pixels = dib_variant_pixels(width_usize as u32, height_usize as u32, 32, 255);
    let mut header = BITMAPV5HEADER {
        bV5Size: std::mem::size_of::<BITMAPV5HEADER>() as u32,
        bV5Width: width,
        bV5Height: height,
        bV5Planes: 1,
        bV5BitCount: 32,
        bV5Compression: if bitfields { BI_BITFIELDS } else { BI_RGB },
        bV5SizeImage: (stride * height_usize) as u32,
        bV5RedMask: if bitfields { 0x00FF_0000 } else { 0 },
        bV5GreenMask: if bitfields { 0x0000_FF00 } else { 0 },
        bV5BlueMask: if bitfields { 0x0000_00FF } else { 0 },
        bV5AlphaMask: if bitfields { 0xFF00_0000 } else { 0 },
        ..Default::default()
    };
    header.bV5CSType = 0x7352_4742; // LCS_sRGB
    let mut out = Vec::with_capacity(std::mem::size_of::<BITMAPV5HEADER>() + pixels.len());
    out.extend_from_slice(unsafe {
        std::slice::from_raw_parts(&header as *const _ as *const u8, std::mem::size_of::<BITMAPV5HEADER>())
    });
    out.extend_from_slice(&pixels);
    out
}

pub fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let pixels = dib_variant_pixels(width, height, 32, 255);
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for pixel in pixels.chunks_exact(4) {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
    let buffer = image::RgbaImage::from_raw(width, height, rgba).expect("rgba buffer");
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    image::DynamicImage::ImageRgba8(buffer)
        .write_to(&mut cursor, image::ImageOutputFormat::Png)
        .expect("png encode");
    out
}

// ---------------------------------------------------------------------------
// Clipboard setters (simulate different source applications)
// ---------------------------------------------------------------------------

pub fn clear() -> R<()> {
    open_clipboard()?;
    let result = empty_clipboard();
    close_clipboard();
    result
}

pub fn set_dib(width: i32, height: i32, bpp: u16, bottom_up: bool, alpha: u8) -> R<()> {
    set_dib_inner(width, height, bpp, bottom_up, alpha, false)
}

pub fn set_dib_plus_text(text: &str, width: i32, height: i32, bpp: u16) -> R<()> {
    open_clipboard()?;
    let result = (|| -> R<()> {
        empty_clipboard()?;
        set_format_raw(CF_DIB, &dib_bytes(width, height, bpp, true, 255))?;
        set_unicode_text(text)?;
        Ok(())
    })();
    close_clipboard();
    result
}

fn set_dib_inner(width: i32, height: i32, bpp: u16, bottom_up: bool, alpha: u8, _v5: bool) -> R<()> {
    open_clipboard()?;
    let result = (|| -> R<()> {
        empty_clipboard()?;
        set_format_raw(CF_DIB, &dib_bytes(width, height, bpp, bottom_up, alpha))
    })();
    close_clipboard();
    result
}

pub fn set_dibv5(width: i32, height: i32, bitfields: bool) -> R<()> {
    open_clipboard()?;
    let result = (|| -> R<()> {
        empty_clipboard()?;
        set_format_raw(CF_DIBV5, &dibv5_bytes(width, height, bitfields))
    })();
    close_clipboard();
    result
}

pub fn set_registered_format(name: &str, bytes: &[u8]) -> R<()> {
    let wide = windows::core::HSTRING::from(name);
    let format = unsafe { RegisterClipboardFormatW(&wide) };
    if format == 0 {
        return Err(format!("RegisterClipboardFormatW({name}) failed"));
    }
    open_clipboard()?;
    let result = (|| -> R<()> {
        empty_clipboard()?;
        set_format_raw(format, bytes)
    })();
    close_clipboard();
    result
}

pub fn set_unicode_text(text: &str) -> R<()> {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = unsafe {
        std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2)
    };
    set_format_raw(CF_UNICODETEXT, bytes)
}

pub fn set_text(text: &str) -> R<()> {
    open_clipboard()?;
    let result = (|| -> R<()> {
        empty_clipboard()?;
        set_unicode_text(text)
    })();
    close_clipboard();
    result
}

pub fn set_bitmap_only(width: i32, height: i32) -> R<()> {
    open_clipboard()?;
    let result = (|| -> R<()> {
        empty_clipboard()?;
        let pixels = dib_variant_pixels(width as u32, height as u32, 32, 255);
        let stride = ((width as usize * 4).div_ceil(4)) * 4;
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: (stride * height as usize) as u32,
            ..Default::default()
        };
        unsafe {
            let dc = GetDC(HWND::default());
            let bitmap = CreateCompatibleBitmap(dc, width, height);
            // SetDIBits requires the bitmap NOT to be selected into a DC.
            let scanned = SetDIBits(
                dc,
                bitmap,
                0,
                height as u32,
                pixels.as_ptr() as *const _,
                &bmi,
                DIB_RGB_COLORS,
            );
            ReleaseDC(HWND::default(), dc);
            if scanned == 0 {
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
                return Err("SetDIBits failed".to_string());
            }
            set_handle_format(CF_BITMAP, HANDLE(bitmap.0))
        }
    })();
    close_clipboard();
    result
}

pub fn set_hdrop_files(paths: &[PathBuf]) -> R<()> {
    let mut payload = Vec::new();
    let dropfiles = DROPFILES {
        pFiles: std::mem::size_of::<DROPFILES>() as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: windows::Win32::Foundation::BOOL(0),
        fWide: windows::Win32::Foundation::BOOL(1),
    };
    payload.extend_from_slice(unsafe {
        std::slice::from_raw_parts(&dropfiles as *const _ as *const u8, std::mem::size_of::<DROPFILES>())
    });
    for path in paths {
        for unit in path.to_string_lossy().encode_utf16() {
            payload.extend_from_slice(&unit.to_le_bytes());
        }
        payload.extend_from_slice(&0u16.to_le_bytes());
    }
    payload.extend_from_slice(&0u16.to_le_bytes());
    open_clipboard()?;
    let result = (|| -> R<()> {
        empty_clipboard()?;
        set_format_raw(CF_HDROP, &payload)?;
        let joined = paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\r\n");
        set_unicode_text(&joined)?;
        Ok(())
    })();
    close_clipboard();
    result
}

// ---------------------------------------------------------------------------
// Observation helpers
// ---------------------------------------------------------------------------

pub fn format_name(format: u32) -> String {
    if format < 0xC000 {
        return match format {
            CF_TEXT => "CF_TEXT".to_string(),
            CF_BITMAP => "CF_BITMAP".to_string(),
            CF_DIB => "CF_DIB".to_string(),
            CF_UNICODETEXT => "CF_UNICODETEXT".to_string(),
            CF_HDROP => "CF_HDROP".to_string(),
            CF_DIBV5 => "CF_DIBV5".to_string(),
            other => format!("CF#{other}"),
        };
    }
    let mut buffer = [0u16; 256];
    let written = unsafe { GetClipboardFormatNameW(format, &mut buffer) };
    if written <= 0 {
        return format!("registered#{format}");
    }
    String::from_utf16_lossy(&buffer[..written as usize])
}

pub fn enumerate_formats() -> Vec<(u32, String)> {
    // EnumClipboardFormats requires the clipboard to be open.
    if open_clipboard().is_err() {
        return Vec::new();
    }
    let mut formats = Vec::new();
    let mut current = 0u32;
    loop {
        current = unsafe { EnumClipboardFormats(current) };
        if current == 0 {
            break;
        }
        formats.push((current, format_name(current)));
    }
    close_clipboard();
    formats
}

pub fn describe_image(image: &ps_windows::ClipboardImage) -> String {
    let mut checksum: u64 = 0;
    for (index, byte) in image.rgba.iter().enumerate().take(4096) {
        checksum = checksum.wrapping_add((*byte as u64).wrapping_mul(index as u64 + 1));
    }
    format!(
        "{}x{} rgba_bytes={} checksum={checksum:016x}",
        image.width,
        image.height,
        image.rgba.len()
    )
}

pub fn save_png(path: &Path, bytes: &[u8]) -> R<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| werr("create_dir_all", e))?;
    }
    std::fs::write(path, bytes).map_err(|e| werr("write png", e))
}

/// Read the current clipboard through the public ps-windows API.
pub struct ObservedClipboard {
    pub text: Option<String>,
    pub image: Option<String>,
    pub png: Option<(usize, String)>,
}

pub fn observe_clipboard(evidence_dir: Option<&Path>, label: &str) -> ObservedClipboard {
    let text = ps_windows::get_clipboard_text();
    let image = ps_windows::get_clipboard_image().map(|image| describe_image(&image));
    let png = ps_windows::get_clipboard_image_png().map(|bytes| {
        let decoded = image::load_from_memory(&bytes)
            .map(|decoded| format!("{}x{}", decoded.width(), decoded.height()))
            .unwrap_or_else(|_| "decode-failed".to_string());
        if let Some(dir) = evidence_dir {
            let _ = save_png(&dir.join(format!("{label}.png")), &bytes);
        }
        (bytes.len(), decoded)
    });
    ObservedClipboard { text, image, png }
}

pub fn ensure_dir(path: &Path) -> R<()> {
    std::fs::create_dir_all(path).map_err(|e| werr("create_dir_all", e))
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub fn hstring(value: &str) -> windows::core::HSTRING {
    windows::core::HSTRING::from(value)
}

pub fn pwstr_or_null(value: &Option<windows::core::HSTRING>) -> PCWSTR {
    match value {
        Some(value) => PCWSTR(value.as_ptr()),
        None => PCWSTR::null(),
    }
}

pub const _UNUSED: PCWSTR = PCWSTR::null();
pub const _UNUSED_W: windows::core::PCWSTR = w!("clipboard-audit");
// ---------------------------------------------------------------------------
// Raw clipboard inspection (diagnostic evidence)
// ---------------------------------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(" ")
}

/// Dump the raw clipboard formats and the header of any DIB/DIBV5/PNG payload.
/// Used to explain decoder rejections (e.g. synthesized CF_DIB with
/// BI_BITFIELDS compression).
pub fn dump_raw_formats() {
    if open_clipboard().is_err() {
        println!("raw: could not open clipboard");
        return;
    }
    let mut current = 0u32;
    loop {
        current = unsafe { EnumClipboardFormats(current) };
        if current == 0 {
            break;
        }
        let name = format_name(current);
        let handle = unsafe { windows::Win32::System::DataExchange::GetClipboardData(current) };
        let handle = match handle {
            Ok(handle) => handle,
            Err(error) => {
                println!("raw {name} ({current}): GetClipboardData failed ({error})");
                continue;
            }
        };
        if current == CF_BITMAP {
            println!("raw CF_BITMAP (2): HBITMAP handle={:?} (no size)", handle.0);
            continue;
        }
        let hglobal = HGLOBAL(handle.0);
        let size = unsafe { windows::Win32::System::Memory::GlobalSize(hglobal) };
        let pointer = unsafe { GlobalLock(hglobal) };
        if pointer.is_null() {
            println!("raw {name} ({current}): GlobalSize={size} bytes, lock failed");
            continue;
        }
        let bytes = unsafe { std::slice::from_raw_parts(pointer as *const u8, size.min(24)) };
        print!("raw {name} ({current}): globalsize={size} first={}", hex(bytes));
        if current == CF_DIB || current == CF_DIBV5 || name.eq_ignore_ascii_case("png") || name.eq_ignore_ascii_case("image/png") {
            let header_size = unsafe { *(pointer as *const u32) } as usize;
            if current == CF_DIBV5 {
                let header = unsafe { &*(pointer as *const BITMAPV5HEADER) };
                print!(
                    " | V5 header_size={header_size} {}x{} bpp={} compression={} masks={:08x}/{:08x}/{:08x}/{:08x}",
                    header.bV5Width,
                    header.bV5Height,
                    header.bV5BitCount,
                    header.bV5Compression.0,
                    header.bV5RedMask,
                    header.bV5GreenMask,
                    header.bV5BlueMask,
                    header.bV5AlphaMask
                );
            } else if current == CF_DIB {
                let header = unsafe { &*(pointer as *const BITMAPINFOHEADER) };
                print!(
                    " | DIB header_size={header_size} {}x{} bpp={} compression={} size_image={}",
                    header.biWidth,
                    header.biHeight,
                    header.biBitCount,
                    header.biCompression,
                    header.biSizeImage
                );
            } else {
                let png_magic = bytes.starts_with(&[0x89, b'P', b'N', b'G']);
                print!(" | payload png_magic={png_magic}");
            }
        }
        println!();
        unsafe { GlobalUnlock(hglobal).ok() };
    }
    close_clipboard();
}
