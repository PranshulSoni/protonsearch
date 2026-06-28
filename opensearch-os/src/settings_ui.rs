slint::include_modules!();

use crate::settings::AppSettings;
use slint::{CloseRequestResponse, ComponentHandle, SharedString};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

// Signal channel: main thread sends () to ask settings window to show
static SHOW_REQUEST: Lazy<Mutex<Option<std::sync::mpsc::SyncSender<()>>>> = Lazy::new(|| Mutex::new(None));
// Track whether the settings thread is alive at all
static SETTINGS_READY: AtomicBool = AtomicBool::new(false);

pub fn init_settings_window(hwnd: HWND) {
    std::env::set_var("SLINT_STYLE", "fluent-dark");
    
    // Channel: main thread sends () → settings thread shows the window
    let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);
    if let Ok(mut guard) = SHOW_REQUEST.lock() {
        *guard = Some(tx);
    }
    SETTINGS_READY.store(true, Ordering::SeqCst);
    
    // Wait for show requests in a loop. Each request creates a fresh window.
    loop {
        // Block until someone calls show_settings_window()
        match rx.recv() {
            Ok(_) => {},
            Err(_) => break, // channel closed, exit thread
        }
        
        // Create fresh Slint window each time (avoids all hide/show state issues)
        let ui = match SettingsWindow::new() {
            Ok(u) => u,
            Err(_) => continue,
        };

        // Load and apply current settings
        let settings = AppSettings::load();
        ui.set_run_on_startup(settings.run_on_startup);
        ui.set_hide_on_lose_focus(settings.hide_on_lose_focus);
        ui.set_theme_mode(SharedString::from(settings.normalized_theme_mode()));
        ui.set_global_hotkey(SharedString::from(settings.global_hotkey.clone()));
        ui.set_window_width(settings.window_width as i32);
        ui.set_item_height(settings.item_height as i32);

        // Close = hide window, then stop the inner run()
        let ui_weak_close = ui.as_weak();
        ui.window().on_close_requested(move || {
            if let Some(ui) = ui_weak_close.upgrade() {
                ui.invoke_set_hotkey_recording(false);
                // Quit the inner event loop to unblock us
                slint::quit_event_loop().ok();
                ui.window().hide().ok();
            }
            CloseRequestResponse::KeepWindowShown
        });

        // Save settings callback
        let ui_weak_save = ui.as_weak();
        ui.on_save_settings(move || {
            if let Some(ui) = ui_weak_save.upgrade() {
                let mut s = AppSettings::load();
                s.run_on_startup = ui.get_run_on_startup();
                s.hide_on_lose_focus = ui.get_hide_on_lose_focus();
                s.theme_mode = ui.get_theme_mode().to_string();
                s.global_hotkey = ui.get_global_hotkey().to_string();
                s.window_width = ui.get_window_width() as u32;
                s.item_height = ui.get_item_height() as u32;
                s.save();
                crate::settings_startup::set_run_on_startup(s.run_on_startup);
                unsafe {
                    let _ = PostMessageW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_USER + 10,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
        });

        ui.on_format_hotkey(move |key, ctrl, alt, shift, win| {
            let Some(hotkey) = crate::hotkey::format_recorded_hotkey(
                key.as_str(),
                ctrl,
                alt,
                shift,
                win,
            ) else {
                return SharedString::from("");
            };
            if crate::hotkey::hotkey_available(&hotkey) {
                SharedString::from(hotkey)
            } else {
                SharedString::from("")
            }
        });

        ui.on_set_hotkey_recording(move |recording| {
            unsafe {
                let _ = PostMessageW(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::WM_USER + 11,
                    windows::Win32::Foundation::WPARAM(recording as usize),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
        });

        // Show the window and run the event loop until it's closed
        ui.window().show().ok();
        ui.window().set_minimized(false);
        slint::run_event_loop().ok();
        // Window was closed — loop back and wait for next show request
    }
}

pub fn show_settings_window() {
    if !SETTINGS_READY.load(Ordering::SeqCst) {
        // Settings thread not yet ready — spawn it now lazily
        // (This path shouldn't normally be hit since init is called at startup)
        return;
    }
    if let Ok(guard) = SHOW_REQUEST.lock() {
        if let Some(tx) = guard.as_ref() {
            let _ = tx.try_send(());
        }
    }
}
