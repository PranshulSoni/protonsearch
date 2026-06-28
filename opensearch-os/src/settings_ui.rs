slint::include_modules!();

use crate::settings::AppSettings;
use slint::{SharedString, ComponentHandle, CloseRequestResponse};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Keep a global reference to the Slint UI so we can show/hide it repeatedly
static SETTINGS_UI: Lazy<Mutex<Option<slint::Weak<SettingsWindow>>>> = Lazy::new(|| Mutex::new(None));

pub fn init_settings_window(hwnd: HWND) {
    std::env::set_var("SLINT_STYLE", "fluent-dark");
    let ui = SettingsWindow::new().unwrap();
    let ui_weak = ui.as_weak();
    
    // Store in global static
    if let Ok(mut guard) = SETTINGS_UI.lock() {
        *guard = Some(ui_weak.clone());
    }

    // Load current settings
    let mut settings = AppSettings::load();

    // Initialize UI properties
    ui.set_run_on_startup(settings.run_on_startup);
    ui.set_hide_on_lose_focus(settings.hide_on_lose_focus);
    ui.set_theme_mode(SharedString::from(settings.theme_mode.clone()));
    ui.set_global_hotkey(SharedString::from(settings.global_hotkey.clone()));
    ui.set_window_width(settings.window_width as i32);
    ui.set_item_height(settings.item_height as i32);

    // Intercept Close to Hide instead of Destroy
    let ui_weak_close = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = ui_weak_close.upgrade() {
            ui.window().hide().unwrap();
        }
        CloseRequestResponse::KeepWindowShown
    });

    // Callback when UI wants to save settings
    ui.on_save_settings(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let mut current_settings = AppSettings::load();
            current_settings.run_on_startup = ui.get_run_on_startup();
            current_settings.hide_on_lose_focus = ui.get_hide_on_lose_focus();
            current_settings.theme_mode = ui.get_theme_mode().to_string();
            current_settings.global_hotkey = ui.get_global_hotkey().to_string();
            current_settings.window_width = ui.get_window_width() as u32;
            current_settings.item_height = ui.get_item_height() as u32;
            current_settings.save();
            
            // Sync with Windows Registry
            crate::settings_startup::set_run_on_startup(current_settings.run_on_startup);
            
            unsafe {
                let _ = PostMessageW(hwnd, windows::Win32::UI::WindowsAndMessaging::WM_USER + 10, windows::Win32::Foundation::WPARAM(0), windows::Win32::Foundation::LPARAM(0));
            }
        }
    });

    let ui_weak_hotkey = ui.as_weak();
    ui.on_record_hotkey(move || {
        let weak_clone = ui_weak_hotkey.clone();
        std::thread::spawn(move || {
            if let Some(recorded) = crate::hotkey::record_hotkey_blocking() {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak_clone.upgrade() {
                        ui.set_global_hotkey(SharedString::from(recorded));
                        ui.invoke_save_settings(); // Automatically save the newly recorded hotkey
                    }
                });
            } else {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak_clone.upgrade() {
                        // Revert to old hotkey
                        let current_settings = AppSettings::load();
                        ui.set_global_hotkey(SharedString::from(current_settings.global_hotkey));
                    }
                });
            }
        });
    });

    // Do not run here! If we run here, we block the thread. Wait, we are spawned in a background thread in main.rs, so blocking here is correct!
    // But we need to make sure the window is initially HIDDEN. 
    // In Slint, the window is shown automatically when `run` is called unless we hide it before.
    let _ = ui.window().hide();
    
    // We run the event loop. This blocks this background thread forever.
    let _ = ui.run();
}

pub fn show_settings_window() {
    // Always dispatch into the Slint event loop - don't check weak outside
    if let Ok(guard) = SETTINGS_UI.lock() {
        if let Some(weak) = guard.as_ref() {
            let weak_clone = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak_clone.upgrade() {
                    // Refresh data
                    let settings = AppSettings::load();
                    ui.set_run_on_startup(settings.run_on_startup);
                    ui.set_hide_on_lose_focus(settings.hide_on_lose_focus);
                    ui.set_theme_mode(SharedString::from(settings.theme_mode.clone()));
                    ui.set_global_hotkey(SharedString::from(settings.global_hotkey.clone()));
                    ui.set_window_width(settings.window_width as i32);
                    ui.set_item_height(settings.item_height as i32);
                    ui.window().show().unwrap();
                    // Bring to front
                    ui.window().set_minimized(false);
                }
            });
        }
    }
}
