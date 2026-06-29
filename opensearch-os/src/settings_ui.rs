slint::include_modules!();

use crate::settings::AppSettings;
use slint::{CloseRequestResponse, ComponentHandle, SharedString};
use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{HWND, GetLastError};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

fn find_launcher_hwnd() -> Option<HWND> {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    use windows::core::PCWSTR;
    let class_name: Vec<u16> = "opensearch-os\0".encode_utf16().collect();
    if let Ok(hwnd) = unsafe { FindWindowW(PCWSTR(class_name.as_ptr()), None) } {
        if !hwnd.0.is_null() {
            return Some(hwnd);
        }
    }
    None
}

pub fn run_settings_window() {
    // single instance check for settings
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
    use windows::core::PCWSTR;

    unsafe {
        let name: Vec<u16> = "Local\\OpenSearchOSSettingsMutex\0".encode_utf16().collect();
        let handle = CreateMutexW(None, true, PCWSTR(name.as_ptr()));
        if let Ok(h) = handle {
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = windows::Win32::Foundation::CloseHandle(h);
                return; // Settings already open
            }
        }
    }

    std::env::set_var("SLINT_STYLE", "fluent-dark");

    let ui = match SettingsWindow::new() {
        Ok(u) => u,
        Err(_) => return,
    };

    // Load current settings
    let settings = AppSettings::load();
    let (api_key, endpoint, model, always_approve) = load_ai_settings();

    ui.set_run_on_startup(settings.run_on_startup);
    ui.set_hide_on_lose_focus(settings.hide_on_lose_focus);
    ui.set_show_taskbar(settings.show_taskbar);
    ui.set_window_location(SharedString::from(settings.window_location.clone()));
    ui.set_theme_mode(SharedString::from(settings.normalized_theme_mode()));
    ui.set_global_hotkey(SharedString::from(settings.global_hotkey.clone()));
    ui.set_voice_hotkey(SharedString::from(crate::hotkey::VOICE_DICTATION_HOTKEY));
    ui.set_hotkey_error(SharedString::from(""));
    ui.set_window_width(settings.window_width as i32);
    ui.set_item_height(settings.item_height as i32);
    ui.set_search_bar_height(settings.search_bar_height as i32);
    ui.set_query_font_family(SharedString::from(settings.query_font_family.clone()));
    ui.set_query_font_weight(SharedString::from(settings.query_font_weight.clone()));
    ui.set_query_font_size(settings.query_font_size as i32);
    ui.set_result_title_font_family(SharedString::from(settings.result_title_font_family.clone()));
    ui.set_result_title_font_weight(SharedString::from(settings.result_title_font_weight.clone()));
    ui.set_result_title_font_size(settings.result_title_font_size as i32);
    ui.set_result_subtitle_font_family(SharedString::from(settings.result_subtitle_font_family.clone()));
    ui.set_result_subtitle_font_weight(SharedString::from(settings.result_subtitle_font_weight.clone()));
    ui.set_result_subtitle_font_size(settings.result_subtitle_font_size as i32);
    ui.set_show_placeholder(settings.show_placeholder);

    // Load Agent properties
    ui.set_agent_api_key(SharedString::from(api_key));
    ui.set_agent_endpoint(SharedString::from(endpoint));
    ui.set_agent_model(SharedString::from(model));
    ui.set_agent_always_approve(always_approve);

    // Load Database folders
    let folders_vec: Vec<slint::SharedString> = settings
        .scan_folders
        .iter()
        .map(|f| slint::SharedString::from(f.clone()))
        .collect();
    let folders_model = slint::ModelRc::new(slint::VecModel::from(folders_vec));
    ui.set_db_folders(folders_model);

    // Close = hide window, terminate event loop
    let ui_weak_close = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = ui_weak_close.upgrade() {
            if let Some(hwnd) = find_launcher_hwnd() {
                unsafe {
                    let _ = PostMessageW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_USER + 11,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
            ui.window().hide().ok();
            slint::quit_event_loop().ok();
        }
        CloseRequestResponse::HideWindow
    });

        // Save settings callback
        let ui_weak_save = ui.as_weak();
        ui.on_save_settings(move || {
            if let Some(ui) = ui_weak_save.upgrade() {
                let mut s = AppSettings::load();
                s.run_on_startup = ui.get_run_on_startup();
                s.hide_on_lose_focus = ui.get_hide_on_lose_focus();
                s.show_taskbar = ui.get_show_taskbar();
                s.window_location = ui.get_window_location().to_string();
                s.theme_mode = ui.get_theme_mode().to_string();
                let next_hotkey = ui.get_global_hotkey().to_string();
                if let Err(message) = crate::hotkey::validate_hotkey(&next_hotkey, &s.global_hotkey)
                {
                    ui.set_hotkey_error(SharedString::from(message));
                    ui.set_global_hotkey(SharedString::from(s.global_hotkey));
                    return;
                }
                s.global_hotkey = next_hotkey;
                s.window_width = ui.get_window_width() as u32;
                s.item_height = ui.get_item_height() as u32;
                s.search_bar_height = ui.get_search_bar_height() as u32;
                s.query_font_family = ui.get_query_font_family().to_string();
                s.query_font_weight = ui.get_query_font_weight().to_string();
                s.query_font_size = ui.get_query_font_size() as u32;
                s.result_title_font_family = ui.get_result_title_font_family().to_string();
                s.result_title_font_weight = ui.get_result_title_font_weight().to_string();
                s.result_title_font_size = ui.get_result_title_font_size() as u32;
                s.result_subtitle_font_family = ui.get_result_subtitle_font_family().to_string();
                s.result_subtitle_font_weight = ui.get_result_subtitle_font_weight().to_string();
                s.result_subtitle_font_size = ui.get_result_subtitle_font_size() as u32;
                s.show_placeholder = ui.get_show_placeholder();
                s.save();
                ui.set_hotkey_error(SharedString::from(""));
                crate::settings_startup::set_run_on_startup(s.run_on_startup);

                // Save Agent properties
                save_ai_settings(
                    ui.get_agent_api_key().as_str(),
                    ui.get_agent_endpoint().as_str(),
                    ui.get_agent_model().as_str(),
                    ui.get_agent_always_approve(),
                );

                if let Some(hwnd) = find_launcher_hwnd() {
                    unsafe {
                        let _ = PostMessageW(
                            hwnd,
                            windows::Win32::UI::WindowsAndMessaging::WM_USER + 10,
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                }
            }
        });

        ui.on_format_hotkey(move |key, ctrl, alt, shift, win| {
            let Some(hotkey) =
                crate::hotkey::format_recorded_hotkey(key.as_str(), ctrl, alt, shift, win)
            else {
                return SharedString::from("");
            };
            SharedString::from(hotkey)
        });

        ui.on_validate_hotkey(move |hotkey| {
            let settings = AppSettings::load();
            match crate::hotkey::validate_hotkey(hotkey.as_str(), &settings.global_hotkey) {
                Ok(()) => SharedString::from("OK"),
                Err(message) => SharedString::from(message),
            }
        });

        ui.on_set_hotkey_recording(move |recording| {
            if let Some(hwnd) = find_launcher_hwnd() {
                unsafe {
                    let _ = PostMessageW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_USER + 11,
                        windows::Win32::Foundation::WPARAM(recording as usize),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
        });

        let ui_weak_add = ui.as_weak();
        ui.on_add_folder(move || {
            if let Some(path) = pick_folder() {
                let path_str = path.to_string_lossy().to_string();
                if let Some(ui) = ui_weak_add.upgrade() {
                    let mut s = AppSettings::load();
                    if s.scan_folders.is_empty() {
                        let defaults: Vec<String> = crate::indexer::get_default_scan_folders()
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect();
                        s.scan_folders = defaults;
                    }
                    if !s.scan_folders.contains(&path_str) {
                        s.scan_folders.push(path_str);
                        s.save();
                        let folders_vec: Vec<slint::SharedString> = s
                            .scan_folders
                            .iter()
                            .map(|f| slint::SharedString::from(f.clone()))
                            .collect();
                        let folders_model = slint::ModelRc::new(slint::VecModel::from(folders_vec));
                        ui.set_db_folders(folders_model);
                    }
                }
            }
        });

        let ui_weak_remove = ui.as_weak();
        ui.on_remove_folder(move |idx| {
            if let Some(ui) = ui_weak_remove.upgrade() {
                let mut s = AppSettings::load();
                if s.scan_folders.is_empty() {
                    let defaults: Vec<String> = crate::indexer::get_default_scan_folders()
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    s.scan_folders = defaults;
                }
                if idx >= 0 && (idx as usize) < s.scan_folders.len() {
                    s.scan_folders.remove(idx as usize);
                    s.save();
                    let folders_vec: Vec<slint::SharedString> = s
                        .scan_folders
                        .iter()
                        .map(|f| slint::SharedString::from(f.clone()))
                        .collect();
                    let folders_model = slint::ModelRc::new(slint::VecModel::from(folders_vec));
                    ui.set_db_folders(folders_model);
                }
            }
        });

        ui.on_rebuild_index(move || {
            let appdata = std::env::var("APPDATA").unwrap_or_default();
            let db_path = std::path::PathBuf::from(appdata)
                .join("opensearch-os")
                .join("file_index.db");
            std::thread::spawn(move || {
                let folders = crate::indexer::get_scan_folders();
                let _ = crate::indexer::run_indexer_folders(&db_path, folders);
            });
        });

        let ui_weak_status = ui.as_weak();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let ui_weak = ui_weak_status.clone();
                if ui_weak.upgrade().is_none() {
                    break;
                }
                let is_indexing = crate::indexer::IS_INDEXING.load(Ordering::Relaxed);
                let progress = {
                    if let Ok(g) = crate::indexer::INDEXING_PROGRESS.lock() {
                        g.clone()
                    } else {
                        "Idle".to_string()
                    }
                };
                let last_time = {
                    if let Ok(g) = crate::indexer::LAST_INDEX_TIME.lock() {
                        g.clone()
                    } else {
                        "Never".to_string()
                    }
                };
                let count = get_indexed_files_count();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_db_is_indexing(is_indexing);
                        ui.set_db_status(slint::SharedString::from(progress));
                        ui.set_db_last_indexed(slint::SharedString::from(last_time));
                        ui.set_db_file_count(count as i32);
                    }
                });
            }
        });

        // Show the window and run the event loop until it's closed
        ui.window().show().ok();

        ui.window().set_minimized(false);
        slint::run_event_loop().ok();
}

fn get_db_conn() -> Option<rusqlite::Connection> {
    let appdata = std::env::var("APPDATA").ok()?;
    let path = std::path::PathBuf::from(appdata)
        .join("opensearch-os")
        .join("file_index.db");
    let conn = rusqlite::Connection::open(&path).ok()?;
    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
    Some(conn)
}

fn load_ai_settings() -> (String, String, String, bool) {
    let mut api_key = String::new();
    let mut endpoint = String::new();
    let mut model = String::new();
    let mut always_approve = false;

    if let Some(conn) = get_db_conn() {
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_settings (key TEXT PRIMARY KEY, value TEXT);",
            [],
        );
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'api_key'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            api_key = val;
        }
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'endpoint'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            endpoint = val;
        }
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'model'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            model = val;
        }
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'always_approve'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            always_approve = val.trim() == "1";
        }
    }
    (api_key, endpoint, model, always_approve)
}

fn save_ai_settings(api_key: &str, endpoint: &str, model: &str, always_approve: bool) {
    if let Some(conn) = get_db_conn() {
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_settings (key TEXT PRIMARY KEY, value TEXT);",
            [],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('api_key', ?);",
            [api_key],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('endpoint', ?);",
            [endpoint],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('model', ?);",
            [model],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('always_approve', ?);",
            [if always_approve { "1" } else { "0" }],
        );
    }
}


fn get_indexed_files_count() -> u32 {
    if let Some(conn) = get_db_conn() {
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                extension TEXT NOT NULL,
                modified INTEGER NOT NULL,
                size INTEGER NOT NULL DEFAULT 0,
                is_dir INTEGER NOT NULL DEFAULT 0
            );",
            [],
        );
        if let Ok(count) = conn.query_row("SELECT COUNT(*) FROM files", [], |row| row.get::<_, u32>(0)) {
            return count;
        }
    }
    0
}

fn pick_folder() -> Option<std::path::PathBuf> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use windows::core::Interface;
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, CoInitializeEx, COINIT_APARTMENTTHREADED, CoUninitialize};
        use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, FOS_PICKFOLDERS};
        
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let mut path_res = None;
            
            if let Ok(dialog) = CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_ALL) {
                if let Ok(options) = dialog.GetOptions() {
                    let _ = dialog.SetOptions(options | FOS_PICKFOLDERS);
                    if dialog.Show(None).is_ok() {
                        if let Ok(result) = dialog.GetResult() {
                            if let Ok(display_name) = result.GetDisplayName(windows::Win32::UI::Shell::SIGDN_FILESYSPATH) {
                                if let Ok(s) = display_name.to_string() {
                                    path_res = Some(std::path::PathBuf::from(s));
                                }
                            }
                        }
                    }
                }
            }
            let _ = tx.send(path_res);
            CoUninitialize();
        }
    });
    rx.recv().unwrap_or(None)
}

