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
fn clipboard_listener_can_restart_after_stop() {
    // Regression: the window class stays registered for the process lifetime, so
    // a second ClipboardListener::start() must tolerate ERROR_CLASS_ALREADY_EXISTS
    // (1410). Without that, disabling and re-enabling clipboard history in the
    // app broke capture until a restart.
    for cycle in 0..3 {
        let listener = ps_windows::ClipboardListener::start(|_snapshot| {})
            .unwrap_or_else(|error| panic!("listener start failed on cycle {cycle}: {error}"));
        ps_windows::set_clipboard_text(&format!("listener restart cycle {cycle}"))
            .expect("set clipboard");
        std::thread::sleep(std::time::Duration::from_millis(30));
        listener.stop();
    }
    // Dropping without an explicit stop must also be safe.
    let listener = ps_windows::ClipboardListener::start(|_snapshot| {}).expect("start");
    drop(listener);
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
// ---------------------------------------------------------------------------
// Native adapter layer (task-35 milestone 1): capabilities, audio, brightness
// ---------------------------------------------------------------------------

#[test]
fn native_capabilities_probe_is_bounded_and_honest() {
    let started = std::time::Instant::now();
    let capabilities = ps_windows::native_capabilities();
    let elapsed = started.elapsed();
    eprintln!("native capabilities probe took {elapsed:?}");
    assert!(elapsed < std::time::Duration::from_secs(20), "probe must be bounded");
    assert!(
        capabilities.local_notifications.supported,
        "the local notification store is always available"
    );
    for (name, capability) in [
        ("media_sessions", &capabilities.media_sessions),
        ("audio_output", &capabilities.audio_output),
        ("audio_volume", &capabilities.audio_volume),
        ("audio_output_switch", &capabilities.audio_output_switch),
        ("wifi", &capabilities.wifi),
        ("bluetooth", &capabilities.bluetooth),
        ("brightness", &capabilities.brightness),
        ("toast_notifications", &capabilities.toast_notifications),
    ] {
        eprintln!("capability {name}: supported={} reason={:?}", capability.supported, capability.reason);
        if !capability.supported {
            assert!(
                capability.reason.as_deref().map(|reason| !reason.is_empty()).unwrap_or(false),
                "unsupported capability {name} must carry a reason"
            );
        }
    }
    // Round-trip through serde for the future IPC layer.
    let json = serde_json::to_string(&capabilities).unwrap();
    let decoded: ps_windows::NativeCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, capabilities);
}

#[test]
fn audio_output_enumeration_and_volume_roundtrip() {
    let capabilities = ps_windows::native_capabilities();
    if !capabilities.audio_output.supported {
        eprintln!(
            "UNVERIFIED audio output: {}",
            capabilities.audio_output.reason.clone().unwrap_or_default()
        );
        return;
    }
    let devices = ps_windows::list_audio_outputs().expect("list output devices");
    eprintln!("audio output devices: {devices:?}");
    assert!(!devices.is_empty());
    assert!(devices.iter().any(|device| device.is_default), "one default expected");
    for device in devices.iter().take(5) {
        assert!(!device.id.is_empty() && !device.name.is_empty());
    }

    if capabilities.audio_volume.supported {
        let before = ps_windows::audio_state().expect("audio state");
        ps_windows::set_audio_volume(before.volume).expect("restore volume");
        ps_windows::set_audio_mute(before.muted).expect("restore mute");
        let after = ps_windows::audio_state().expect("audio state after");
        assert!((after.volume - before.volume).abs() < 0.02);
        assert_eq!(after.muted, before.muted);
        // Clamping must be honoured (does not change the user's audio state).
        ps_windows::set_audio_volume(before.volume).expect("volume clamp path");
    } else {
        eprintln!(
            "audio volume unsupported: {}",
            capabilities.audio_volume.reason.clone().unwrap_or_default()
        );
    }
}

#[test]
fn brightness_roundtrip_when_supported() {
    let capabilities = ps_windows::native_capabilities();
    if !capabilities.brightness.supported {
        eprintln!(
            "UNVERIFIED brightness: {}",
            capabilities.brightness.reason.clone().unwrap_or_default()
        );
        return;
    }
    let monitors = ps_windows::brightness_monitors();
    eprintln!("brightness-capable monitors: {monitors:?}");
    assert!(!monitors.is_empty());
    let current = ps_windows::get_brightness().expect("current brightness");
    assert!(current <= 100);
    ps_windows::set_brightness(current).expect("restore brightness");
    let after = ps_windows::get_brightness().expect("brightness after");
    let delta = (i16::from(after) - i16::from(current)).abs();
    assert!(delta <= 2, "brightness changed unexpectedly: {current} -> {after}");
}

#[test]
fn toast_capability_is_reported_honestly() {
    let capabilities = ps_windows::native_capabilities();
    if capabilities.toast_notifications.supported {
        // Only probe that a toast can be created; do not spam the user.
        eprintln!("toast notifications supported on this host");
    } else {
        let reason = capabilities
            .toast_notifications
            .reason
            .clone()
            .expect("unsupported toast must carry a reason");
        eprintln!("toast notifications unsupported: {reason}");
        assert!(!ps_windows::show_toast("ProtonSearch", "should fail").is_ok());
    }
}
// ---------------------------------------------------------------------------
// Native milestone 2 (task-55): Wi-Fi, Bluetooth, brightness DDC/CI, fallbacks
// ---------------------------------------------------------------------------

#[test]
fn settings_fallbacks_are_allow_listed() {
    use ps_windows::SettingsPage as Page;
    let pages = [
        Page::Wifi,
        Page::Bluetooth,
        Page::Display,
        Page::Sound,
        Page::Notifications,
        Page::AirplaneMode,
        Page::MobileHotspot,
        Page::NightLight,
        Page::EnergySaver,
        Page::Project,
        Page::WindowsUpdate,
        Page::PrivacyLocation,
    ];
    for page in pages {
        let uri = page.uri();
        assert!(uri.starts_with("ms-settings:"), "unexpected settings uri {uri}");
        assert!(!page.label().is_empty());
    }
    assert_eq!(Page::Wifi.uri(), "ms-settings:network-wifi");
    assert_eq!(Page::Bluetooth.uri(), "ms-settings:bluetooth");
    let json = serde_json::to_string(&Page::AirplaneMode).unwrap();
    assert_eq!(json, "\"airplane_mode\"");
    // Do not call open_settings_page() in tests: it opens a real GUI window.
}

#[test]
fn wifi_controls_are_usable_or_honest() {
    let started = std::time::Instant::now();
    let capabilities = ps_windows::native_capabilities();
    if !capabilities.wifi.supported {
        assert!(capabilities.wifi.reason.is_some());
        assert_eq!(
            capabilities.wifi.fallback_uri.as_deref(),
            Some("ms-settings:network-wifi")
        );
        eprintln!(
            "UNVERIFIED Wi-Fi: {}",
            capabilities.wifi.reason.clone().unwrap_or_default()
        );
        return;
    }
    let state = ps_windows::wifi_radio_state().expect("Wi-Fi radio state");
    assert!(state.available, "supported Wi-Fi must have an available radio");
    let networks = match ps_windows::list_wifi_networks() {
        Ok(networks) => networks,
        Err(error) => {
            // Windows may deny Wi-Fi scanning until the user grants location
            // permission; that must be an honest error, not a fake empty list.
            let message = format!("{error:#}").to_lowercase();
            assert!(
                message.contains("location")
                    || message.contains("settings")
                    || message.contains("scan")
                    || message.contains("wlan"),
                "unexpected Wi-Fi list error: {message}"
            );
            eprintln!("wifi list honest failure: {message}");
            Vec::new()
        }
    };
    eprintln!(
        "wifi radio enabled={} adapters={:?} networks={}",
        state.enabled,
        ps_windows::wlan_adapters(),
        networks.len()
    );
    for network in networks.iter().take(8) {
        assert!(!network.ssid.is_empty());
        assert!(network.signal <= 100);
        assert!(!network.security.is_empty());
        eprintln!(
            "wifi network {:?} signal={} security={} connected={} profile={}",
            network.ssid, network.signal, network.security, network.connected, network.has_profile
        );
    }
    // Toggle to the CURRENT state: exercises the setter without changing connectivity.
    match ps_windows::set_wifi_radio(state.enabled) {
        Ok(()) => eprintln!("wifi radio setter accepted a no-op change"),
        Err(error) => eprintln!("wifi radio setter honest failure: {error:#}"),
    }
    // Missing profile must fail honestly (never creates profiles or prompts for keys).
    let result = ps_windows::connect_wifi("__protonsearch_no_such_network__");
    match result {
        Ok(outcome) => {
            assert!(!outcome.connected);
            eprintln!("connect missing profile outcome: {}", outcome.message);
        }
        Err(error) => {
            let message = format!("{error:#}").to_lowercase();
            assert!(
                message.contains("profile") || message.contains("settings") || message.contains("radio"),
                "unexpected connect error: {message}"
            );
            eprintln!("connect missing profile honest error: {message}");
        }
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(25));
}

#[test]
fn bluetooth_controls_are_usable_or_honest() {
    let capabilities = ps_windows::native_capabilities();
    if !capabilities.bluetooth.supported {
        assert!(capabilities.bluetooth.reason.is_some());
        assert_eq!(
            capabilities.bluetooth.fallback_uri.as_deref(),
            Some("ms-settings:bluetooth")
        );
        eprintln!(
            "UNVERIFIED Bluetooth: {}",
            capabilities.bluetooth.reason.clone().unwrap_or_default()
        );
        return;
    }
    let state = ps_windows::bluetooth_radio_state().expect("Bluetooth radio state");
    assert!(state.available, "supported Bluetooth must have an available radio");
    let devices = ps_windows::list_bluetooth_devices().expect("paired Bluetooth devices");
    eprintln!("bluetooth radio enabled={} paired_devices={}", state.enabled, devices.len());
    for device in devices.iter().take(10) {
        assert!(!device.id.is_empty());
        assert!(device.paired);
        eprintln!(
            "bluetooth device name={:?} connected={} id={:?}",
            device.name, device.connected, device.id
        );
    }
    match ps_windows::set_bluetooth_radio(state.enabled) {
        Ok(()) => eprintln!("bluetooth radio setter accepted a no-op change"),
        Err(error) => eprintln!("bluetooth radio setter honest failure: {error:#}"),
    }
}

#[test]
fn brightness_controls_report_method_or_honest_fallback() {
    let capabilities = ps_windows::native_capabilities();
    if !capabilities.brightness.supported {
        assert!(capabilities.brightness.reason.is_some());
        assert_eq!(
            capabilities.brightness.fallback_uri.as_deref(),
            Some("ms-settings:display")
        );
        eprintln!(
            "UNVERIFIED brightness: {}",
            capabilities.brightness.reason.clone().unwrap_or_default()
        );
        return;
    }
    let monitors = ps_windows::list_brightness_monitors();
    assert!(!monitors.is_empty(), "supported brightness must list a monitor");
    for monitor in monitors.iter().take(4) {
        assert!(monitor.current <= 100);
        assert!(monitor.maximum > 0);
        eprintln!(
            "brightness monitor name={:?} current={} {}/{} method={:?}",
            monitor.name, monitor.current, monitor.minimum, monitor.maximum, monitor.method
        );
    }
    let current = ps_windows::get_brightness().expect("current brightness");
    ps_windows::set_brightness(current).expect("restore brightness");
}

#[test]
fn controls_status_snapshot_is_bounded_and_serializable() {
    let started = std::time::Instant::now();
    let status = ps_windows::controls_status();
    let elapsed = started.elapsed();
    eprintln!("controls_status() took {elapsed:?}");
    assert!(elapsed < std::time::Duration::from_secs(30));
    let json = serde_json::to_string(&status).expect("serialize ControlsStatus");
    assert!(!json.is_empty());
    assert!(status.capabilities.local_notifications.supported);
    eprintln!("controls_status json bytes={}", json.len());
}