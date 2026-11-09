//! Real Windows smoke tests for the ps-windows helpers.
//!
//! These run on the actual host: icon extraction from a system executable, a
//! clipboard round-trip, URL scheme rejection, foreground window query and the
//! AppsFolder enumeration. No GUI application is launched.

use std::path::Path;

#[test]
fn extracts_a_real_icon_png_from_system_executable() {
    let exe = Path::new(r"C:\Windows\System32\notepad.exe");
    if !exe.is_file() {
        eprintln!("SKIP: notepad.exe not present on this host");
        return;
    }
    let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ps-windows-test-icons");
    let out = ps_windows::extract_icon_png(exe, 32, &cache).expect("icon extraction");
    assert!(out.is_file(), "icon png missing: {}", out.display());
    let meta = std::fs::metadata(&out).unwrap();
    assert!(meta.len() > 100, "icon png suspiciously small: {meta:?}");
    let image = image::open(&out).expect("decode icon png");
    assert_eq!(image.width(), 32);
    assert_eq!(image.height(), 32);
    // A real icon must contain more than one colour; a blank RGBA buffer would fail.
    let rgba = image.to_rgba8();
    let first = rgba.get_pixel(0, 0);
    let distinct = rgba.pixels().any(|p| p != first);
    assert!(distinct, "icon decoded as a single flat colour");
}

#[test]
fn icon_cache_is_reused() {
    let exe = Path::new(r"C:\Windows\System32\notepad.exe");
    if !exe.is_file() {
        return;
    }
    let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ps-windows-test-icons-reuse");
    let a = ps_windows::extract_icon_png(exe, 32, &cache).unwrap();
    let b = ps_windows::extract_icon_png(exe, 32, &cache).unwrap();
    assert_eq!(a, b);
}

#[test]
fn rejects_non_http_urls() {
    for bad in [
        "file:///C:/Windows/System32/calc.exe",
        "javascript:alert(1)",
        "ms-settings:privacy",
        "cmd.exe /c calc",
        "http://",
        "",
    ] {
        assert!(
            ps_windows::open_url(bad).is_err(),
            "non-http url must be rejected: {bad:?}"
        );
    }
}

#[test]
fn clipboard_text_roundtrip() {
    let payload = format!("protonsearch clipboard test {}", std::process::id());
    ps_windows::set_clipboard_text(&payload).expect("set clipboard");
    let read = ps_windows::get_clipboard_text().expect("get clipboard");
    assert_eq!(read, payload);
}

#[test]
fn foreground_window_never_panics() {
    // Result depends on the desktop session; the contract is only that the query
    // is safe and any returned pid/title are structurally valid.
    let info = ps_windows::foreground_window();
    if let Some(info) = info {
        assert!(info.process_id > 0);
        assert!(!info.title.is_empty() || info.exe_path.is_some());
    }
}

#[test]
fn appsfolder_enumeration_reports_packaged_apps() {
    match ps_windows::list_appsfolder() {
        Ok(entries) => {
            eprintln!("AppsFolder entries with AUMID: {}", entries.len());
            for entry in entries.iter().take(5) {
                eprintln!("  {} -> {}", entry.name, entry.aumid);
            }
            assert!(
                !entries.is_empty(),
                "the AppsFolder namespace must expose packaged apps on a desktop session"
            );
            assert!(
                entries.iter().all(|e| !e.aumid.is_empty()),
                "entries must carry an AUMID"
            );
        }
        Err(error) => {
            // COM/shell namespace can be unavailable in stripped-down sessions.
            eprintln!("AppsFolder enumeration unavailable: {error}");
        }
    }
}


#[test]
fn extracts_icon_for_packaged_app_via_shell_item_factory() {
    let Ok(entries) = ps_windows::list_appsfolder() else {
        eprintln!("SKIP: AppsFolder enumeration unavailable");
        return;
    };
    let Some(entry) = entries.first() else {
        eprintln!("SKIP: no packaged apps on this host");
        return;
    };
    let shell_path = format!("shell:AppsFolder\\{}", entry.aumid);
    let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ps-windows-shell-icons");
    let out = ps_windows::extract_icon_png(std::path::Path::new(&shell_path), 32, &cache)
        .expect("packaged app icon extraction");
    assert!(out.is_file(), "icon png missing: {}", out.display());
    let image = image::open(&out).expect("decode packaged app icon");
    assert_eq!((image.width(), image.height()), (32, 32));
    let rgba = image.to_rgba8();
    let first = rgba.get_pixel(0, 0);
    assert!(
        rgba.pixels().any(|pixel| pixel != first),
        "packaged app icon decoded as a flat colour"
    );
    eprintln!("packaged app icon: {} (aumid {})", out.display(), entry.aumid);
}


#[test]
fn icon_cache_is_keyed_by_source_identity() {
    let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ps-windows-icon-identity");
    let a = Path::new(r"C:\Windows\System32\notepad.exe");
    let b = Path::new(r"C:\Windows\System32\cmd.exe");
    if !a.is_file() || !b.is_file() {
        eprintln!("SKIP: system executables unavailable");
        return;
    }
    let a32 = ps_windows::extract_icon_png(a, 32, &cache).expect("notepad icon");
    let b32 = ps_windows::extract_icon_png(b, 32, &cache).expect("cmd icon");
    assert_ne!(
        a32, b32,
        "two different icon sources must not share one cache file"
    );

    // Same source: cache reuse (one file).
    let a32_again = ps_windows::extract_icon_png(a, 32, &cache).expect("notepad icon again");
    assert_eq!(a32, a32_again, "same source must reuse the same cache file");

    // Case/path normalization is part of the identity (Windows paths are
    // case-insensitive), so an upper-case spelling reuses the same file.
    let upper = std::path::PathBuf::from(a.to_string_lossy().to_uppercase());
    let upper32 = ps_windows::extract_icon_png(&upper, 32, &cache).expect("upper-case icon");
    assert_eq!(
        a32, upper32,
        "case variants of the same source must share one cache file"
    );

    // Rendered size is part of the cache key.
    let a64 = ps_windows::extract_icon_png(a, 64, &cache).expect("notepad 64 icon");
    assert_ne!(a32, a64, "different icon sizes must use different cache files");
}

#[test]
fn packaged_app_icons_render_the_app_logo_not_a_generic_document() {
    let Ok(entries) = ps_windows::list_appsfolder() else {
        eprintln!("SKIP: AppsFolder enumeration unavailable");
        return;
    };
    let Some(amd) = entries.iter().find(|entry| entry.name == "AMD Software") else {
        eprintln!("SKIP: AMD Software is not installed on this host");
        return;
    };
    let shell_path = format!("shell:AppsFolder\\{}", amd.aumid);
    let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ps-windows-packaged-logo");
    let out = ps_windows::extract_icon_png(std::path::Path::new(&shell_path), 32, &cache)
        .expect("AMD Software icon");
    let image = image::open(&out).expect("decode AMD icon").to_rgba8();
    // The AMD Radeon logo is a saturated red square; the generic document icon
    // is greyscale. This regression catches SIIGBF_ICONONLY forcing the generic
    // fallback for packages that register only VisualElements logos.
    let has_colour = image.pixels().any(|pixel| {
        if pixel[3] < 16 {
            return false;
        }
        let r = pixel[0] as i32;
        let g = pixel[1] as i32;
        let b = pixel[2] as i32;
        r.max(g).max(b) - r.min(g).min(b) > 30
    });
    assert!(
        has_colour,
        "packaged app icon is greyscale/generic (blank document): {}",
        out.display()
    );
}

#[test]
fn resolves_a_generated_shortcut() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ps-windows-test-lnk");
    std::fs::create_dir_all(&dir).unwrap();
    let lnk = dir.join("notepad-link.lnk");
    if lnk.exists() {
        std::fs::remove_file(&lnk).unwrap();
    }
    if create_test_shortcut(&lnk).is_err() {
        eprintln!("SKIP: could not create a test shortcut on this host");
        return;
    }
    let info = ps_windows::resolve_shortcut(&lnk).expect("resolve shortcut");
    let target = info.target.expect("shortcut target");
    assert!(
        target.to_string_lossy().to_ascii_lowercase().contains("notepad"),
        "unexpected target: {}",
        target.display()
    );
    let _ = std::fs::remove_file(&lnk);
}

/// Create a .lnk through the same COM interface used in production.
fn create_test_shortcut(path: &Path) -> anyhow::Result<()> {
    use windows::core::{Interface, HSTRING, GUID};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::IShellLinkW;
    const CLSID_SHELLLINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&CLSID_SHELLLINK, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from("C:\\Windows\\System32\\notepad.exe"))?;
        link.SetArguments(&HSTRING::from("test.txt"))?;
        let persist: IPersistFile = link.cast()?;
        let target = HSTRING::from(path.to_string_lossy().as_ref());
        persist.Save(&target, true)?;
        if hr.is_ok() {
            CoUninitialize();
        }
    }
    Ok(())
}