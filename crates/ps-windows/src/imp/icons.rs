//! Real Windows icon extraction to cached PNG files.
//!
//! Two sources are supported:
//! - filesystem targets (.exe/.dll/.ico, and .lnk targets resolved first so the
//!   shortcut overlay arrow is not baked into the icon);
//! - shell namespace items such as packaged/UWP apps addressed by
//!   `shell:AppsFolder\\<AUMID>`, rendered through IShellItemImageFactory.
//!
//! All icons are written atomically (temp file + rename) and cached by
//! (source, size). Failures return an error or None; a cache path is only
//! returned when the PNG really exists.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};
use windows::Win32::Foundation::{HWND, SIZE};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::System::Com::IBindCtx;
use windows::Win32::UI::Shell::{
    ExtractIconExW, IShellItemImageFactory, SHCreateItemFromParsingName, SHGetFileInfoW,
    SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
    SIIGBF_RESIZETOFIT,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

use super::util::{hstr, ComGuard};

/// FNV-1a: stable cache key independent of the toolchain's DefaultHasher.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn cache_path(cache_dir: &Path, source: &Path, size: u32) -> PathBuf {
    let key = format!("{}#{}", source.to_string_lossy().to_lowercase(), size);
    let hash = fnv1a(key.as_bytes());
    cache_dir.join(format!("{hash:016x}-{size}.png"))
}

pub(crate) fn extract_icon_png(
    source: &Path,
    size: u32,
    cache_dir: &Path,
) -> anyhow::Result<PathBuf> {
    let size = size.clamp(16, 256);
    let out = cache_path(cache_dir, source, size);
    if out.is_file() {
        return Ok(out);
    }
    let display = source.to_string_lossy().to_string();
    let image = if display.starts_with("shell:") {
        // Packaged/UWP apps live in the shell namespace; SHGetFileInfo cannot
        // resolve them, so render through IShellItemImageFactory.
        match shell_item_image(&display, size) {
            Ok(image) => image,
            Err(shell_error) => {
                let icon = load_hicon(source, size).with_context(|| {
                    format!("shell item icon failed ({shell_error}); fallback failed")
                })?;
                let image = hicon_to_rgba(icon, size);
                let _ = unsafe { DestroyIcon(icon) };
                image?
            }
        }
    } else {
        let icon = load_hicon(source, size)?;
        let image = hicon_to_rgba(icon, size);
        let _ = unsafe { DestroyIcon(icon) };
        image?
    };
    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating icon cache {}", cache_dir.display()))?;
    let tmp = out.with_extension("png.tmp");
    image
        .save_with_format(&tmp, image::ImageFormat::Png)
        .with_context(|| format!("writing icon {}", tmp.display()))?;
    std::fs::rename(&tmp, &out).with_context(|| format!("committing icon {}", out.display()))?;
    Ok(out)
}

/// Render a shell namespace item (e.g. a packaged app AUMID) to RGBA.
fn shell_item_image(display: &str, size: u32) -> anyhow::Result<image::RgbaImage> {
    let _com = ComGuard::new();
    let item: IShellItemImageFactory = unsafe {
        SHCreateItemFromParsingName(&hstr(display), None::<&IBindCtx>)
            .with_context(|| format!("SHCreateItemFromParsingName({display})"))?
    };
    let bitmap = unsafe {
        item.GetImage(
            SIZE {
                cx: size as i32,
                cy: size as i32,
            },
            SIIGBF_RESIZETOFIT | SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
        )
        .with_context(|| format!("IShellItemImageFactory::GetImage({display})"))?
    };
    let raw = unsafe { read_bitmap_bgra(bitmap) };
    let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.0)) };
    let raw = raw?;
    // Shell image bitmaps are premultiplied BGRA.
    Ok(raw_to_rgba(raw, size, true))
}

/// Load an HICON for a filesystem path (a .lnk target when possible).
fn load_hicon(source: &Path, size: u32) -> anyhow::Result<HICON> {
    let display = source.to_string_lossy().to_string();
    if display.starts_with("shell:") {
        return sh_get_icon(&display);
    }
    // Resolve .lnk targets first to avoid the shortcut overlay glyph.
    let effective = if source
        .extension()
        .map(|e| e.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false)
    {
        super::shell::resolve_shortcut(source)
            .ok()
            .and_then(|s| s.target)
            .filter(|p| p.is_file())
    } else {
        None
    };
    let icon_source = effective.as_deref().unwrap_or(source);
    if let Some(icon) = extract_icon_ex(icon_source, size) {
        return Ok(icon);
    }
    sh_get_icon(&icon_source.to_string_lossy())
        .with_context(|| format!("no icon available for {}", source.display()))
}

/// ExtractIconExW gives proper per-file icons for .exe/.dll/.ico.
fn extract_icon_ex(path: &Path, size: u32) -> Option<HICON> {
    let wide = hstr(&path.to_string_lossy());
    let mut large = HICON::default();
    let mut small = HICON::default();
    let count = unsafe { ExtractIconExW(&wide, 0, Some(&mut large), Some(&mut small), 1) };
    if count == 0 {
        return None;
    }
    let want_large = size > 16;
    let chosen = if want_large && !large.is_invalid() {
        large
    } else if !small.is_invalid() {
        small
    } else if !large.is_invalid() {
        large
    } else {
        return None;
    };
    // Free the icon we are not returning.
    unsafe {
        if chosen.0 != large.0 && !large.is_invalid() {
            let _ = DestroyIcon(large);
        }
        if chosen.0 != small.0 && !small.is_invalid() {
            let _ = DestroyIcon(small);
        }
    }
    Some(chosen)
}

/// SHGetFileInfo fallback (works for a few shell-name strings and files).
fn sh_get_icon(display: &str) -> anyhow::Result<HICON> {
    let wide = hstr(display);
    let mut info = SHFILEINFOW::default();
    let flags = SHGFI_ICON | SHGFI_LARGEICON;
    let result = unsafe {
        SHGetFileInfoW(
            &wide,
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };
    if result == 0 || info.hIcon.is_invalid() {
        bail!("SHGetFileInfo returned no icon");
    }
    Ok(info.hIcon)
}

struct RawBitmap {
    width: u32,
    height: u32,
    /// Top-down BGRA rows.
    bgra: Vec<u8>,
}

/// Read a 32 bpp HBITMAP as top-down BGRA bytes.
unsafe fn read_bitmap_bgra(bitmap: HBITMAP) -> anyhow::Result<RawBitmap> {
    if bitmap.is_invalid() {
        bail!("no bitmap");
    }
    let mut info = BITMAP::default();
    let got = GetObjectW(
        HGDIOBJ(bitmap.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut info as *mut BITMAP as *mut _),
    );
    if got == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
        bail!("could not query bitmap dimensions");
    }
    let width = info.bmWidth as u32;
    let height = info.bmHeight as u32;
    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32), // top-down rows
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut bgra = vec![0u8; width as usize * height as usize * 4];
    let dc: HDC = GetDC(HWND::default());
    let scanned = GetDIBits(
        dc,
        bitmap,
        0,
        height,
        Some(bgra.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    ReleaseDC(HWND::default(), dc);
    if scanned == 0 {
        bail!("GetDIBits failed for bitmap");
    }
    Ok(RawBitmap {
        width,
        height,
        bgra,
    })
}

fn raw_to_rgba(mut raw: RawBitmap, target: u32, un_premultiply: bool) -> image::RgbaImage {
    let mut any_alpha = false;
    for px in raw.bgra.chunks_exact(4) {
        if px[3] != 0 {
            any_alpha = true;
            break;
        }
    }
    if !any_alpha {
        // Opaque bitmap: keep alpha at 255 and do not un-premultiply.
        for px in raw.bgra.chunks_exact_mut(4) {
            px[3] = 255;
        }
    } else if un_premultiply {
        for px in raw.bgra.chunks_exact_mut(4) {
            let alpha = px[3] as u32;
            if alpha > 0 && alpha < 255 {
                for channel in 0..3 {
                    let value = (px[channel] as u32 * 255 + alpha / 2) / alpha;
                    px[channel] = value.min(255) as u8;
                }
            }
        }
    }
    // BGRA -> RGBA
    for px in raw.bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let image = image::RgbaImage::from_raw(raw.width, raw.height, raw.bgra)
        .unwrap_or_else(|| image::RgbaImage::new(target, target));
    if raw.width == target && raw.height == target {
        image
    } else {
        image::imageops::resize(&image, target, target, image::imageops::FilterType::Lanczos3)
    }
}

fn hicon_to_rgba(icon: HICON, target: u32) -> anyhow::Result<image::RgbaImage> {
    let mut icon_info = ICONINFO::default();
    unsafe { GetIconInfo(icon, &mut icon_info) }.map_err(|e| anyhow!("GetIconInfo: {e}"))?;
    let guard = IconInfoGuard(icon_info);
    let color: HBITMAP = guard.0.hbmColor;
    if color.is_invalid() {
        bail!("icon has no color bitmap");
    }
    let raw = unsafe { read_bitmap_bgra(color) }?;
    let mask = guard.0.hbmMask;
    let raw = if raw.bgra.chunks_exact(4).all(|px| px[3] == 0) && !mask.is_invalid() {
        // Older icons store transparency only in the AND-mask.
        apply_mask_alpha(mask, raw)?
    } else {
        raw
    };
    Ok(raw_to_rgba(raw, target, false))
}

/// Materialise the AND-mask as alpha for icons without an alpha channel.
fn apply_mask_alpha(mask: HBITMAP, mut raw: RawBitmap) -> anyhow::Result<RawBitmap> {
    let width = raw.width;
    let height = raw.height;
    let stride = width.div_ceil(32) * 4;
    let mut mask_buf = vec![0u8; stride as usize * height as usize];
    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32),
        biPlanes: 1,
        biBitCount: 1,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let dc = unsafe { GetDC(HWND::default()) };
    let scanned = unsafe {
        GetDIBits(
            dc,
            mask,
            0,
            height,
            Some(mask_buf.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };
    unsafe { ReleaseDC(HWND::default(), dc) };
    if scanned == 0 {
        return Ok(raw);
    }
    for y in 0..height as usize {
        for x in 0..width as usize {
            let byte = mask_buf[y * stride as usize + x / 8];
            let transparent = (byte >> (7 - (x % 8))) & 1 == 1;
            raw.bgra[(y * width as usize + x) * 4 + 3] = if transparent { 0 } else { 255 };
        }
    }
    Ok(raw)
}

/// Frees the two bitmaps GetIconInfo allocates.
struct IconInfoGuard(ICONINFO);

impl Drop for IconInfoGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.0.hbmColor.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(self.0.hbmColor.0));
            }
            if !self.0.hbmMask.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(self.0.hbmMask.0));
            }
        }
    }
}
