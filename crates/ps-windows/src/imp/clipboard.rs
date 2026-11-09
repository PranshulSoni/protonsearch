//! Win32 clipboard access, format negotiation and the background listener.
//!
//! Gate 2 (task-39) design:
//! - One clipboard open per capture; format negotiation order is registered
//!   PNG/image-png -> CF_DIBV5 -> CF_DIB -> CF_BITMAP, so the best available
//!   image representation wins and mixed text+image copies keep the image.
//! - Decoders accept BI_RGB and BI_BITFIELDS, 32/24/16 bpp and palette
//!   (8/4/1 bpp) DIB payloads, bottom-up and top-down rows. Decoding is bounded
//!   (pixel count / long edge / payload bytes) and arithmetic is checked.
//! - The WM_CLIPBOARDUPDATE handler only signals a bounded worker channel; all
//!   clipboard reads, PNG encoding and callback work (DB/file IO) run on the
//!   capture worker thread so the window message loop can never block.
//! - The callback is invoked for every clipboard update that carries any
//!   supported content; unsupported images are no longer silently dropped.
//! - set_image() restores a bitmap to the clipboard as CF_DIB + CF_DIBV5 +
//!   registered PNG + CF_BITMAP for maximum target compatibility.

use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Mutex;
use std::thread;

use anyhow::{anyhow, bail, Context};
use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_BITFIELDS, BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BITMAPV5HEADER, CBM_INIT,
    CreateDIBitmap, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits, GetObjectW, HGDIOBJ, ReleaseDC,
};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, EnumClipboardFormats,
    GetClipboardData, GetClipboardFormatNameW, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Shell::DROPFILES;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    PostThreadMessageW, RegisterClassW, TranslateMessage, HMENU, MSG, WINDOW_EX_STYLE,
    WINDOW_STYLE, WNDCLASSW, WM_DESTROY, WM_QUIT,
};

use super::util::ComGuard;
use crate::{ClipboardImage, ClipboardListener, ClipboardSnapshot};

const CF_BITMAP: u32 = 2;
const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;
const CF_DIBV5: u32 = 17;
const WM_CLIPBOARDUPDATE: u32 = 0x031D;

/// Hard caps that keep a hostile/huge clipboard payload from exhausting memory.
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
const MAX_LONG_EDGE: u32 = 8192;
const MAX_CLIPBOARD_BYTES: usize = 256 * 1024 * 1024;
const MAX_RICH_FORMAT_BYTES: usize = 8 * 1024 * 1024;
const MAX_HDROP_ENTRIES: usize = 200;
const MAX_FORMAT_NAME: usize = 128;

/// Capture worker queue depth: a burst of clipboard updates coalesces instead of
/// growing without bound.
const CAPTURE_QUEUE: usize = 8;

/// The listener callback is process-global: only one listener may be active.
static CALLBACK: Mutex<Option<Box<dyn Fn(ClipboardSnapshot) + Send + 'static>>> = Mutex::new(None);
/// Signal from the window procedure to the capture worker.
static CAPTURE_SIGNAL: Mutex<Option<SyncSender<()>>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Clipboard session and low-level readers
// ---------------------------------------------------------------------------

struct ClipboardSession;

impl ClipboardSession {
    fn open() -> anyhow::Result<Self> {
        // Another process can hold the clipboard open briefly (copy/paste
        // handoff). Retry for ~250 ms total, never spin forever.
        for _ in 0..25 {
            if unsafe { OpenClipboard(HWND::default()) }.is_ok() {
                return Ok(Self);
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
        bail!("could not open the clipboard (held by another process)")
    }

    fn text(&self) -> Option<String> {
        unsafe {
            let raw = clipboard_bytes(CF_UNICODETEXT)?;
            let units: Vec<u16> = raw
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();
            let end = units.iter().position(|unit| *unit == 0).unwrap_or(units.len());
            let text = String::from_utf16_lossy(&units[..end]);
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
    }

    fn files(&self) -> Vec<String> {
        unsafe {
            let Some(raw) = clipboard_bytes(CF_HDROP) else {
                return Vec::new();
            };
            parse_hdrop(&raw)
        }
    }

    fn html(&self) -> Option<String> {
        let raw = registered_bytes("HTML Format")?;
        let text = String::from_utf8_lossy(&raw);
        Some(parse_html_fragment(&text).unwrap_or_else(|| truncate_string(&text, MAX_RICH_FORMAT_BYTES)))
    }

    fn rtf(&self) -> Option<String> {
        let raw = registered_bytes("Rich Text Format")?;
        Some(truncate_string(&String::from_utf8_lossy(&raw), MAX_RICH_FORMAT_BYTES))
    }

    fn image(&self) -> Option<CapturedImage> {
        // 1) Registered PNG formats (browsers/editors sometimes expose only this).
        for name in ["PNG", "image/png"] {
            if let Some(raw) = registered_bytes(name) {
                if let Some(image) = decode_png_bounded(&raw) {
                    return Some(CapturedImage {
                        png: raw,
                        width: image.width,
                        height: image.height,
                        format: "png",
                        mime: "image/png",
                    });
                }
            }
        }
        // 2) CF_DIBV5 (BITMAPV5HEADER; BITFIELDS/BITMAPV5 masks supported).
        if let Some(raw) = unsafe { clipboard_bytes(CF_DIBV5) } {
            if let Some(image) = decode_dib_image(&raw, true) {
                return encode_capture(image, "dibv5");
            }
        }
        // 3) CF_DIB (BITMAPINFOHEADER; BI_RGB / BI_BITFIELDS, palette).
        if let Some(raw) = unsafe { clipboard_bytes(CF_DIB) } {
            if let Some(image) = decode_dib_image(&raw, false) {
                return encode_capture(image, "dib");
            }
        }
        // 4) CF_BITMAP handle: convert through GetDIBits to a readable DIB.
        if let Some(image) = bitmap_handle_image() {
            return encode_capture(image, "bitmap");
        }
        None
    }

    fn format_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut current = 0u32;
        loop {
            current = unsafe { EnumClipboardFormats(current) };
            if current == 0 || names.len() >= 64 {
                break;
            }
            names.push(format_name(current));
        }
        names
    }
}

impl Drop for ClipboardSession {
    fn drop(&mut self) {
        unsafe { CloseClipboard().ok() };
    }
}

struct CapturedImage {
    png: Vec<u8>,
    width: u32,
    height: u32,
    format: &'static str,
    mime: &'static str,
}

fn encode_capture(image: ClipboardImage, format: &'static str) -> Option<CapturedImage> {
    let png = encode_png(&image)?;
    Some(CapturedImage {
        png,
        width: image.width,
        height: image.height,
        format,
        mime: "image/png",
    })
}

/// Read a clipboard format payload as bytes (HGLOBAL ownership: the system owns
/// the handle; we only lock/read/unlock and never free it).
unsafe fn clipboard_bytes(format: u32) -> Option<Vec<u8>> {
    let handle = GetClipboardData(format).ok()?;
    if handle.0.is_null() {
        return None;
    }
    let hglobal = HGLOBAL(handle.0);
    let size = GlobalSize(hglobal);
    if size == 0 || size > MAX_CLIPBOARD_BYTES {
        return None;
    }
    let pointer = GlobalLock(hglobal);
    if pointer.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(pointer as *const u8, size).to_vec();
    let _ = GlobalUnlock(hglobal);
    Some(bytes)
}

fn registered_format(name: &str) -> u32 {
    let wide = HSTRING::from(name);
    unsafe { RegisterClipboardFormatW(&wide) }
}

fn registered_bytes(name: &str) -> Option<Vec<u8>> {
    let format = registered_format(name);
    if format == 0 {
        return None;
    }
    unsafe { clipboard_bytes(format) }
}

fn format_name(format: u32) -> String {
    if format < 0xC000 {
        return match format {
            CF_BITMAP => "CF_BITMAP".to_string(),
            CF_DIB => "CF_DIB".to_string(),
            CF_UNICODETEXT => "CF_UNICODETEXT".to_string(),
            CF_HDROP => "CF_HDROP".to_string(),
            CF_DIBV5 => "CF_DIBV5".to_string(),
            other => format!("CF#{other}"),
        };
    }
    let mut buffer = [0u16; MAX_FORMAT_NAME];
    let written = unsafe { GetClipboardFormatNameW(format, &mut buffer) };
    if written <= 0 {
        format!("registered#{format}")
    } else {
        String::from_utf16_lossy(&buffer[..written as usize])
    }
}

fn parse_hdrop(raw: &[u8]) -> Vec<String> {
    if raw.len() < std::mem::size_of::<DROPFILES>() {
        return Vec::new();
    }
    let header = unsafe { &*(raw.as_ptr() as *const DROPFILES) };
    let wide = header.fWide.as_bool();
    let offset = header.pFiles as usize;
    if offset >= raw.len() {
        return Vec::new();
    }
    let mut files = Vec::new();
    if wide {
        let units: Vec<u16> = raw[offset..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        let mut start = 0usize;
        for (index, unit) in units.iter().enumerate() {
            if *unit == 0 {
                if index == start {
                    break;
                }
                files.push(String::from_utf16_lossy(&units[start..index]));
                start = index + 1;
                if files.len() >= MAX_HDROP_ENTRIES {
                    break;
                }
            }
        }
    } else {
        let mut start = offset;
        for index in offset..raw.len() {
            if raw[index] == 0 {
                if index == start {
                    break;
                }
                files.push(String::from_utf8_lossy(&raw[start..index]).to_string());
                start = index + 1;
                if files.len() >= MAX_HDROP_ENTRIES {
                    break;
                }
            }
        }
    }
    files
}

/// Extract the fragment from a CF_HTML payload using its byte offsets.
fn parse_html_fragment(raw: &str) -> Option<String> {
    let start = header_offset(raw, "StartFragment:")?;
    let end = header_offset(raw, "EndFragment:")?;
    if start >= end || end > raw.len() || start >= raw.len() {
        return None;
    }
    let fragment = String::from_utf8_lossy(&raw.as_bytes()[start..end]).to_string();
    if fragment.trim().is_empty() {
        None
    } else {
        Some(truncate_string(&fragment, MAX_RICH_FORMAT_BYTES))
    }
}

fn header_offset(raw: &str, field: &str) -> Option<usize> {
    let index = raw.find(field)?;
    let rest = &raw[index + field.len()..];
    let line = rest.split(['\r', '\n']).next()?.trim();
    line.parse::<usize>().ok()
}

fn truncate_string(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

// ---------------------------------------------------------------------------
// DIB decoding (CF_DIB / CF_DIBV5)
// ---------------------------------------------------------------------------

struct DibHeader {
    width: i32,
    height: i32,
    bit_count: u16,
    compression: u32,
    header_size: usize,
    palette_offset: usize,
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
    alpha_mask: u32,
    colors_used: u32,
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    bytes.get(offset..offset + 2).map(|slice| u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn parse_dib_header(bytes: &[u8], v5: bool) -> Option<DibHeader> {
    let header_size = read_u32(bytes, 0)? as usize;
    if header_size < 40 || header_size > bytes.len() {
        return None;
    }
    let width = read_i32(bytes, 4)?;
    let height = read_i32(bytes, 8)?;
    let bit_count = read_u16(bytes, 14)?;
    let compression = read_u32(bytes, 16)?;
    let colors_used = read_u32(bytes, 32)?;
    if width <= 0 || height == 0 {
        return None;
    }
    let mut palette_offset = header_size;
    let mut red_mask = 0u32;
    let mut green_mask = 0u32;
    let mut blue_mask = 0u32;
    let mut alpha_mask = 0u32;
    if v5 {
        if header_size < 124 {
            return None;
        }
        red_mask = read_u32(bytes, 40)?;
        green_mask = read_u32(bytes, 44)?;
        blue_mask = read_u32(bytes, 48)?;
        alpha_mask = read_u32(bytes, 52)?;
    } else if compression == BI_BITFIELDS.0 {
        // Three colour masks follow a plain BITMAPINFOHEADER (BI_BITFIELDS).
        red_mask = read_u32(bytes, header_size)?;
        green_mask = read_u32(bytes, header_size + 4)?;
        blue_mask = read_u32(bytes, header_size + 8)?;
        palette_offset = header_size + 12;
    }
    Some(DibHeader {
        width,
        height,
        bit_count,
        compression,
        header_size,
        palette_offset,
        red_mask,
        green_mask,
        blue_mask,
        alpha_mask,
        colors_used,
    })
}

fn decode_dib_image(bytes: &[u8], v5: bool) -> Option<ClipboardImage> {
    let header = parse_dib_header(bytes, v5)?;
    let width = header.width as u32;
    let abs_height = header.height.unsigned_abs();
    if width == 0 || abs_height == 0 {
        return None;
    }
    if width > MAX_LONG_EDGE || abs_height > MAX_LONG_EDGE {
        return None;
    }
    let pixels = width as u64 * abs_height as u64;
    if pixels > MAX_IMAGE_PIXELS {
        return None;
    }
    let bit_count = header.bit_count as u32;
    if !matches!(bit_count, 1 | 4 | 8 | 16 | 24 | 32) {
        return None;
    }
    if !matches!(header.compression, 0 | 3) {
        // BI_RGB and BI_BITFIELDS only; RLE/JPEG/PNG DIB payloads are rejected.
        return None;
    }
    let bpp = bit_count;
    let bytes_per_row = (width as u64 * bpp as u64).div_ceil(32) * 4;
    let required = (header.header_size as u64)
        .checked_add(
            if !v5 && header.compression == BI_BITFIELDS.0 { 12 } else { 0 },
        )?
        .checked_add(bytes_per_row.checked_mul(abs_height as u64)?)?;
    if required > bytes.len() as u64 {
        return None;
    }
    // Palette (needed for 1/4/8 bpp).
    let palette = if bpp <= 8 {
        let colors = if header.colors_used == 0 {
            1usize << bpp
        } else {
            header.colors_used as usize
        }
        .min(256);
        let mut entries = Vec::with_capacity(colors);
        for index in 0..colors {
            let offset = header.palette_offset + index * 4;
            let b = *bytes.get(offset)?;
            let g = *bytes.get(offset + 1)?;
            let r = *bytes.get(offset + 2)?;
            entries.push([r, g, b, 255]);
        }
        Some(entries)
    } else {
        None
    };
    let data_offset = header.palette_offset
        + if bpp <= 8 {
            let colors = if header.colors_used == 0 {
                1usize << bpp
            } else {
                header.colors_used as usize
            }
            .min(256);
            colors * 4
        } else {
            0
        };
    let data = bytes.get(data_offset..)?;
    let bottom_up = header.height > 0;
    let mut rgba = vec![0u8; width as usize * abs_height as usize * 4];
    for row in 0..abs_height as usize {
        let source_row = if bottom_up { abs_height as usize - 1 - row } else { row };
        let row_start = source_row * bytes_per_row as usize;
        for x in 0..width as usize {
            let pixel_offset = match bpp {
                32 => row_start + x * 4,
                24 => row_start + x * 3,
                16 => row_start + x * 2,
                8 => row_start + x,
                4 => row_start + x / 2,
                1 => row_start + x / 8,
                _ => return None,
            };
            let pixel = match bpp {
                32 => {
                    let value = *data.get(pixel_offset)? as u32
                        | (*data.get(pixel_offset + 1)? as u32) << 8
                        | (*data.get(pixel_offset + 2)? as u32) << 16
                        | (*data.get(pixel_offset + 3)? as u32) << 24;
                    (value, *data.get(pixel_offset + 3)?)
                }
                24 => {
                    let b = *data.get(pixel_offset)?;
                    let g = *data.get(pixel_offset + 1)?;
                    let r = *data.get(pixel_offset + 2)?;
                    ((r as u32) | ((g as u32) << 8) | ((b as u32) << 16), 255)
                }
                16 => {
                    let value = u16::from_le_bytes([
                        *data.get(pixel_offset)?,
                        *data.get(pixel_offset + 1)?,
                    ]) as u32;
                    (value, 255)
                }
                8 => (*data.get(pixel_offset)? as u32, 255),
                4 => {
                    let byte = *data.get(pixel_offset)?;
                    let value = if x % 2 == 0 { byte >> 4 } else { byte & 0x0F };
                    (value as u32, 255)
                }
                1 => {
                    let byte = *data.get(pixel_offset)?;
                    let value = (byte >> (7 - (x % 8))) & 1;
                    (value as u32, 255)
                }
                _ => return None,
            };
            let (value, raw_alpha) = pixel;
            let [r, g, b, alpha] = if bpp <= 8 {
                palette.as_ref()?.get(value as usize).copied()?
            } else if header.compression == BI_BITFIELDS.0
                || (v5 && (header.red_mask | header.green_mask | header.blue_mask) != 0)
            {
                [
                    extract_channel(value, header.red_mask, 8),
                    extract_channel(value, header.green_mask, 8),
                    extract_channel(value, header.blue_mask, 8),
                    if header.alpha_mask != 0 {
                        extract_channel(value, header.alpha_mask, 8)
                    } else if bpp == 32 {
                        raw_alpha
                    } else {
                        255
                    },
                ]
            } else if bpp == 16 {
                // BI_RGB 16bpp is 5-5-5.
                [
                    (((value >> 10) & 0x1F) as u8) << 3,
                    (((value >> 5) & 0x1F) as u8) << 3,
                    ((value & 0x1F) as u8) << 3,
                    255,
                ]
            } else if bpp == 32 {
                // BI_RGB 32bpp is B,G,R,A in memory.
                [
                    ((value >> 16) & 0xFF) as u8,
                    ((value >> 8) & 0xFF) as u8,
                    (value & 0xFF) as u8,
                    raw_alpha,
                ]
            } else if bpp == 24 {
                // 24bpp: value was assembled as R | G<<8 | B<<16.
                [
                    (value & 0xFF) as u8,
                    ((value >> 8) & 0xFF) as u8,
                    ((value >> 16) & 0xFF) as u8,
                    255,
                ]
            } else {
                [0, 0, 0, 255]
            };
            let destination = (row * width as usize + x) * 4;
            rgba[destination] = r;
            rgba[destination + 1] = g;
            rgba[destination + 2] = b;
            rgba[destination + 3] = alpha;
        }
    }
    normalize_alpha(&mut rgba);
    Some(ClipboardImage {
        width,
        height: abs_height,
        rgba,
    })
}

fn extract_channel(value: u32, mask: u32, target_bits: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let bits = mask.count_ones();
    if bits == 0 || bits > 8 || shift + bits > 32 {
        return 0;
    }
    let raw = (value & mask) >> shift;
    let scaled = if bits < target_bits {
        (raw * ((1u32 << target_bits) - 1)) / ((1u32 << bits) - 1)
    } else {
        raw >> (bits - target_bits)
    };
    scaled.min(255) as u8
}

fn normalize_alpha(rgba: &mut [u8]) {
    let any_alpha = rgba.chunks_exact(4).any(|pixel| pixel[3] != 0);
    if !any_alpha {
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }
}

fn decode_png_bounded(bytes: &[u8]) -> Option<ClipboardImage> {
    let image = image::load_from_memory(bytes).ok()?;
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 || width > MAX_LONG_EDGE || height > MAX_LONG_EDGE {
        return None;
    }
    if width as u64 * height as u64 > MAX_IMAGE_PIXELS {
        return None;
    }
    let rgba = image.to_rgba8();
    Some(ClipboardImage {
        width,
        height,
        rgba: rgba.into_raw(),
    })
}

fn bitmap_handle_image() -> Option<ClipboardImage> {
    unsafe {
        let handle = GetClipboardData(CF_BITMAP).ok()?;
        if handle.0.is_null() {
            return None;
        }
        let bitmap = windows::Win32::Graphics::Gdi::HBITMAP(handle.0);
        let mut info = BITMAP::default();
        let got = GetObjectW(
            HGDIOBJ(bitmap.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut info as *mut BITMAP as *mut _),
        );
        if got == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
            return None;
        }
        let width = info.bmWidth as u32;
        let height = info.bmHeight as u32;
        if width > MAX_LONG_EDGE || height > MAX_LONG_EDGE {
            return None;
        }
        if width as u64 * height as u64 > MAX_IMAGE_PIXELS {
            return None;
        }
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let dc = GetDC(HWND::default());
        let scanned = GetDIBits(
            dc,
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(HWND::default(), dc);
        if scanned == 0 {
            return None;
        }
        let mut rgba = vec![0u8; pixels.len()];
        for (source, destination) in pixels.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
            destination[0] = source[2];
            destination[1] = source[1];
            destination[2] = source[0];
            destination[3] = source[3];
        }
        normalize_alpha(&mut rgba);
        Some(ClipboardImage {
            width,
            height,
            rgba,
        })
    }
}

fn encode_png(image: &ClipboardImage) -> Option<Vec<u8>> {
    let buffer = image::RgbaImage::from_raw(image.width, image.height, image.rgba.clone())?;
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    image::DynamicImage::ImageRgba8(buffer)
        .write_to(&mut cursor, image::ImageOutputFormat::Png)
        .ok()?;
    Some(out)
}

// ---------------------------------------------------------------------------
// Paste-back (set clipboard image)
// ---------------------------------------------------------------------------

pub(crate) fn set_image(png_bytes: &[u8]) -> anyhow::Result<()> {
    let image = decode_png_bounded(png_bytes)
        .ok_or_else(|| anyhow!("cannot decode clipboard image payload as PNG"))?;
    let width = image.width as i32;
    let height = image.height as i32;
    let dib = build_dib_rgb(&image);
    let dibv5 = build_dibv5_bitfields(&image);
    let bitmap = unsafe { create_hbitmap(&image) };

    let _session = ClipboardSession::open()?;
    unsafe {
        EmptyClipboard().context("EmptyClipboard")?;
        let mut set_any = false;
        if let Some(id) = Some(registered_format("PNG")) {
            if id != 0 {
                let wide = png_bytes.to_vec();
                if set_global_format(id, &wide).is_ok() {
                    set_any = true;
                }
            }
        }
        if set_global_format(CF_DIB, &dib).is_ok() {
            set_any = true;
        }
        let _ = set_global_format(CF_DIBV5, &dibv5);
        if let Some(bitmap) = bitmap {
            if SetClipboardData(CF_BITMAP, HANDLE(bitmap.0)).is_ok() {
                set_any = true;
            } else {
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
            }
        }
        let _ = (width, height);
        if !set_any {
            bail!("could not place any image format on the clipboard");
        }
    }
    Ok(())
}

unsafe fn create_hbitmap(image: &ClipboardImage) -> Option<windows::Win32::Graphics::Gdi::HBITMAP> {
    let width = image.width as i32;
    let height = image.height as i32;
    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut pixels = Vec::with_capacity(image.rgba.len());
    for pixel in image.rgba.chunks_exact(4) {
        pixels.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
    let dc = GetDC(HWND::default());
    let bitmap = CreateDIBitmap(
        dc,
        Some(&bmi.bmiHeader),
        CBM_INIT as u32,
        Some(pixels.as_ptr() as *const _),
        Some(&bmi),
        DIB_RGB_COLORS,
    );
    ReleaseDC(HWND::default(), dc);
    if bitmap.is_invalid() {
        None
    } else {
        Some(bitmap)
    }
}

fn build_dib_rgb(image: &ClipboardImage) -> Vec<u8> {
    let width = image.width;
    let height = image.height;
    let stride = (width as usize * 4).div_ceil(4) * 4;
    let mut out = Vec::with_capacity(40 + stride * height as usize);
    let header = BITMAPINFOHEADER {
        biSize: 40,
        biWidth: width as i32,
        biHeight: height as i32,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        biSizeImage: (stride * height as usize) as u32,
        ..Default::default()
    };
    out.extend_from_slice(unsafe {
        std::slice::from_raw_parts(&header as *const _ as *const u8, 40)
    });
    for row in (0..height as usize).rev() {
        for x in 0..width as usize {
            let px = &image.rgba[(row * width as usize + x) * 4..][..4];
            out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    out
}

fn build_dibv5_bitfields(image: &ClipboardImage) -> Vec<u8> {
    let width = image.width;
    let height = image.height;
    let stride = (width as usize * 4).div_ceil(4) * 4;
    let mut header = BITMAPV5HEADER {
        bV5Size: std::mem::size_of::<BITMAPV5HEADER>() as u32,
        bV5Width: width as i32,
        bV5Height: height as i32,
        bV5Planes: 1,
        bV5BitCount: 32,
        bV5Compression: BI_BITFIELDS,
        bV5SizeImage: (stride * height as usize) as u32,
        bV5RedMask: 0x00FF_0000,
        bV5GreenMask: 0x0000_FF00,
        bV5BlueMask: 0x0000_00FF,
        bV5AlphaMask: 0xFF00_0000,
        ..Default::default()
    };
    header.bV5CSType = 0x7352_4742; // LCS_sRGB
    let mut out = Vec::with_capacity(124 + stride * height as usize);
    out.extend_from_slice(unsafe {
        std::slice::from_raw_parts(&header as *const _ as *const u8, 124)
    });
    for row in (0..height as usize).rev() {
        for x in 0..width as usize {
            let px = &image.rgba[(row * width as usize + x) * 4..][..4];
            out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    out
}

unsafe fn set_global_format(format: u32, bytes: &[u8]) -> anyhow::Result<()> {
    let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).context("GlobalAlloc(clipboard)")?;
    let pointer = GlobalLock(handle);
    if pointer.is_null() {
        let _ = GlobalFree(handle);
        bail!("GlobalLock failed");
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer as *mut u8, bytes.len());
    let _ = GlobalUnlock(handle);
    match SetClipboardData(format, HANDLE(handle.0)) {
        Ok(_) => Ok(()),
        Err(error) => {
            let _ = GlobalFree(handle);
            Err(anyhow!("SetClipboardData({format}) failed: {error}"))
        }
    }
}

// ---------------------------------------------------------------------------
// Public-ish reads used by the crate facade
// ---------------------------------------------------------------------------

pub(crate) fn set_text(text: &str) -> anyhow::Result<()> {
    let _session = ClipboardSession::open()?;
    unsafe {
        EmptyClipboard().context("EmptyClipboard")?;
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
        set_global_format(CF_UNICODETEXT, bytes)
    }
}

pub(crate) fn get_text() -> Option<String> {
    ClipboardSession::open().ok().and_then(|session| session.text())
}

pub(crate) fn get_image() -> Option<ClipboardImage> {
    ClipboardSession::open()
        .ok()
        .and_then(|session| session.image())
        .map(|captured| ClipboardImage {
            width: captured.width,
            height: captured.height,
            rgba: decode_captured_rgba(&captured.png),
        })
        .filter(|image| image.width > 0)
}

fn decode_captured_rgba(png: &[u8]) -> Vec<u8> {
    image::load_from_memory(png)
        .map(|decoded| decoded.to_rgba8().into_raw())
        .unwrap_or_default()
}

pub(crate) fn get_image_png() -> Option<Vec<u8>> {
    ClipboardSession::open()
        .ok()
        .and_then(|session| session.image())
        .map(|captured| captured.png)
}

/// Full clipboard snapshot used by the listener worker and capture_current().
pub(crate) fn snapshot() -> ClipboardSnapshot {
    let mut snapshot = ClipboardSnapshot {
        timestamp_ms: now_ms(),
        ..Default::default()
    };
    if let Ok(session) = ClipboardSession::open() {
        snapshot.text = session.text();
        snapshot.files = session.files();
        snapshot.html = session.html();
        snapshot.rtf = session.rtf();
        snapshot.formats = session.format_names();
        if let Some(captured) = session.image() {
            snapshot.width = Some(captured.width);
            snapshot.height = Some(captured.height);
            snapshot.image_format = Some(captured.format.to_string());
            snapshot.mime = Some(captured.mime.to_string());
            snapshot.png = Some(captured.png);
        }
    }
    snapshot.source_app = super::window::foreground_process_name();
    snapshot
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Clipboard listener: window thread + bounded capture worker
// ---------------------------------------------------------------------------

pub(crate) fn start_listener<F>(callback: F) -> anyhow::Result<ClipboardListener>
where
    F: Fn(ClipboardSnapshot) + Send + 'static,
{
    {
        let mut slot = CALLBACK
            .lock()
            .map_err(|_| anyhow!("clipboard listener lock poisoned"))?;
        if slot.is_some() {
            bail!("a clipboard listener is already running");
        }
        *slot = Some(Box::new(callback));
    }
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<anyhow::Result<u32>>();
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
        let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
    }
}

fn clear_callback() {
    if let Ok(mut slot) = CALLBACK.lock() {
        *slot = None;
    }
}

fn listener_thread(ready: std::sync::mpsc::Sender<anyhow::Result<u32>>) {
    let _com = ComGuard::new();
    let thread_id = unsafe { GetCurrentThreadId() };

    // Bounded capture worker: the window procedure only signals it.
    let (signal_tx, signal_rx) = sync_channel::<()>(CAPTURE_QUEUE);
    if let Ok(mut slot) = CAPTURE_SIGNAL.lock() {
        *slot = Some(signal_tx);
    }
    let worker = thread::Builder::new()
        .name("ps-clipboard-capture".into())
        .spawn(move || {
            while signal_rx.recv().is_ok() {
                let snapshot = snapshot();
                if let Ok(slot) = CALLBACK.lock() {
                    if let Some(callback) = slot.as_ref() {
                        callback(snapshot);
                    }
                }
            }
        })
        .ok();

    match unsafe { create_listener_window(thread_id) } {
        Ok(()) => {
            let _ = ready.send(Ok(thread_id));
        }
        Err(error) => {
            let _ = ready.send(Err(error));
            if let Ok(mut slot) = CAPTURE_SIGNAL.lock() {
                slot.take();
            }
            if let Some(worker) = worker {
                let _ = worker.join();
            }
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
    // Stop order: drop the signal sender so the worker drains and exits, then
    // join it, then clear the callback.
    if let Ok(mut slot) = CAPTURE_SIGNAL.lock() {
        slot.take();
    }
    if let Some(worker) = worker {
        let _ = worker.join();
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
        const ERROR_CLASS_ALREADY_EXISTS: i32 = 1410;
        let error = windows::core::Error::from_win32();
        let code = (error.code().0 as u32 & 0xFFFF) as i32;
        if code != ERROR_CLASS_ALREADY_EXISTS {
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
        windows::Win32::Foundation::HINSTANCE(instance.0),
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
            // Never decode here: just wake the capture worker (bounded queue,
            // drop on overflow so the message loop cannot stall).
            if let Ok(slot) = CAPTURE_SIGNAL.lock() {
                if let Some(signal) = slot.as_ref() {
                    let _ = signal.try_send(());
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

// ---------------------------------------------------------------------------
// Tests for the bounded decoders (in-memory fixtures, no clipboard needed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod decode_tests {
    use super::*;

    fn rgba_image(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&[
                    (x % 256) as u8,
                    (y % 256) as u8,
                    ((x + y) % 256) as u8,
                    255,
                ]);
            }
        }
        rgba
    }

    fn dib_rgb(width: i32, height: i32, bit_count: u16) -> Vec<u8> {
        let image = ClipboardImage {
            width: width as u32,
            height: height as u32,
            rgba: rgba_image(width as u32, height as u32),
        };
        match bit_count {
            32 => build_dib_rgb(&image),
            24 => {
                let mut out = Vec::new();
                let stride = (width as usize * 3).div_ceil(4) * 4;
                let header = BITMAPINFOHEADER {
                    biSize: 40,
                    biWidth: width,
                    biHeight: height,
                    biPlanes: 1,
                    biBitCount: 24,
                    biCompression: BI_RGB.0,
                    biSizeImage: (stride * height as usize) as u32,
                    ..Default::default()
                };
                out.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(&header as *const _ as *const u8, 40)
                });
                for row in (0..height as usize).rev() {
                    for x in 0..width as usize {
                        let px = &image.rgba[(row * width as usize + x) * 4..][..4];
                        out.extend_from_slice(&[px[2], px[1], px[0]]);
                    }
                    while out.len() % 4 != 0 {
                        out.push(0);
                    }
                }
                out
            }
            _ => unreachable!(),
        }
    }

    fn dib_bitfields(width: i32, height: i32) -> Vec<u8> {
        let image = ClipboardImage {
            width: width as u32,
            height: height as u32,
            rgba: rgba_image(width as u32, height as u32),
        };
        let stride = (width as usize * 4).div_ceil(4) * 4;
        let mut out = Vec::new();
        let header = BITMAPINFOHEADER {
            biSize: 40,
            biWidth: width,
            biHeight: height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_BITFIELDS.0,
            biSizeImage: (stride * height as usize) as u32,
            ..Default::default()
        };
        out.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&header as *const _ as *const u8, 40)
        });
        for mask in [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF] {
            out.extend_from_slice(&mask.to_le_bytes());
        }
        for row in (0..height as usize).rev() {
            for x in 0..width as usize {
                let px = &image.rgba[(row * width as usize + x) * 4..][..4];
                out.extend_from_slice(&[px[2], px[1], px[0], 0]);
            }
            while out.len() % 4 != 0 {
                out.push(0);
            }
        }
        out
    }

    #[test]
    fn decodes_rgb32_and_rgb24_dib() {
        for bit_count in [32u16, 24] {
            let pixels = dib_rgb(5, 3, bit_count);
            let decoded = decode_dib_image(&pixels, false).expect("decode");
            assert_eq!((decoded.width, decoded.height), (5, 3));
            // Bottom-up row 0 is the source image's last row (y = 2).
            // Source pixel (x=2, y=2) is r=2, g=2, b=4.
            let offset = (2 * 5 + 2) * 4;
            assert_eq!(
                &decoded.rgba[offset..offset + 4],
                &[2, 2, 4, 255],
                "24/32 bpp colour channels must decode correctly (bit_count={bit_count})"
            );
        }
    }

    #[test]
    fn decodes_bitfields_dib_with_zero_alpha() {
        let payload = dib_bitfields(6, 4);
        let decoded = decode_dib_image(&payload, false).expect("bitfields decode");
        assert_eq!((decoded.width, decoded.height), (6, 4));
        assert!(decoded.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
        let offset = (1 * 6 + 3) * 4;
        assert_eq!(&decoded.rgba[offset..offset + 3], &[3, 1, 4]);
    }

    #[test]
    fn decodes_top_down_dib() {
        let image = ClipboardImage {
            width: 4,
            height: 2,
            rgba: rgba_image(4, 2),
        };
        let mut payload = build_dib_rgb(&image);
        payload[8..12].copy_from_slice(&(-2i32).to_le_bytes());
        let decoded = decode_dib_image(&payload, false).expect("top-down decode");
        assert_eq!((decoded.width, decoded.height), (4, 2));
    }

    #[test]
    fn rejects_oversized_and_malformed_dib() {
        let mut payload = dib_rgb(4, 4, 32);
        payload[4..8].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(decode_dib_image(&payload, false).is_none());
        assert!(decode_dib_image(&payload[..20], false).is_none());
        assert!(decode_dib_image(&[], false).is_none());
    }

    #[test]
    fn decodes_8bpp_palette_dib() {
        let width = 4i32;
        let height = 2i32;
        let mut payload = Vec::new();
        let header = BITMAPINFOHEADER {
            biSize: 40,
            biWidth: width,
            biHeight: height,
            biPlanes: 1,
            biBitCount: 8,
            biCompression: BI_RGB.0,
            biClrUsed: 4,
            ..Default::default()
        };
        payload.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&header as *const _ as *const u8, 40)
        });
        for (r, g, b) in [(0u8, 0u8, 0u8), (255, 0, 0), (0, 255, 0), (0, 0, 255)] {
            payload.extend_from_slice(&[b, g, r, 0]);
        }
        let pixels = [1u8, 2, 3, 0, 3, 2, 1, 0];
        payload.extend_from_slice(&pixels);
        let decoded = decode_dib_image(&payload, false).expect("palette decode");
        assert_eq!((decoded.width, decoded.height), (4, 2));
        // Bottom-up DIB: decoded row 0 comes from the payload's last row.
        assert_eq!(&decoded.rgba[0..4], &[0, 0, 255, 255]);
        assert_eq!(&decoded.rgba[4..8], &[0, 255, 0, 255]);
        assert_eq!(&decoded.rgba[8..12], &[255, 0, 0, 255]);
    }

    #[test]
    fn decodes_dibv5_bitfields() {
        let image = ClipboardImage {
            width: 3,
            height: 2,
            rgba: rgba_image(3, 2),
        };
        let payload = build_dibv5_bitfields(&image);
        let decoded = decode_dib_image(&payload, true).expect("v5 decode");
        assert_eq!((decoded.width, decoded.height), (3, 2));
    }

    #[test]
    fn paste_payloads_round_trip_through_decoders() {
        let image = ClipboardImage {
            width: 9,
            height: 7,
            rgba: rgba_image(9, 7),
        };
        let rgb = build_dib_rgb(&image);
        let bitfields = build_dibv5_bitfields(&image);
        let decoded_rgb = decode_dib_image(&rgb, false).expect("rgb");
        let decoded_v5 = decode_dib_image(&bitfields, true).expect("v5");
        assert_eq!(decoded_rgb.rgba, image.rgba);
        assert_eq!(decoded_v5.rgba, image.rgba);
    }
}