slint::include_modules!();

use crate::settings::AppSettings;
use slint::{CloseRequestResponse, ComponentHandle, SharedString};
use std::sync::atomic::Ordering;
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

fn find_launcher_hwnd() -> Option<HWND> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    let class_name: Vec<u16> = "opensearch-os\0".encode_utf16().collect();
    if let Ok(hwnd) = unsafe { FindWindowW(PCWSTR(class_name.as_ptr()), None) } {
        if !hwnd.0.is_null() {
            return Some(hwnd);
        }
    }
    None
}

fn get_desktop_wallpaper_path() -> Option<String> {
    let mut buffer = [0u16; 512];
    let success = unsafe {
        windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows::Win32::UI::WindowsAndMessaging::SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Some(buffer.as_mut_ptr() as *mut std::ffi::c_void),
            Default::default(),
        )
    };
    if success.is_ok() {
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        let path_str = String::from_utf16_lossy(&buffer[..len]);
        if !path_str.trim().is_empty() && std::path::Path::new(&path_str).exists() {
            return Some(path_str);
        }
    }
    None
}

pub fn run_settings_window() {
    // single instance check for settings
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
    use windows::Win32::System::Threading::CreateMutexW;

    unsafe {
        let name: Vec<u16> = "Local\\OpenSearchOSSettingsMutex\0"
            .encode_utf16()
            .collect();
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
    ui.set_voice_hotkey(SharedString::from(settings.voice_hotkey.clone()));
    ui.set_hotkey_error(SharedString::from(""));
    ui.set_voice_hotkey_error(SharedString::from(""));

    // Populate the four hotkey dropdowns and pre-select the saved combo.
    let (slot1_model, slot2_model, slot34_model) = crate::hotkey::slot_models();
    let to_model = |v: Vec<String>| {
        slint::ModelRc::new(slint::VecModel::from(
            v.into_iter().map(SharedString::from).collect::<Vec<_>>(),
        ))
    };
    ui.set_slot1_model(to_model(slot1_model));
    ui.set_slot2_model(to_model(slot2_model));
    ui.set_slot34_model(to_model(slot34_model));
    let slots = crate::hotkey::hotkey_to_slots(&settings.global_hotkey);
    ui.set_slot1_val(SharedString::from(slots[0].clone()));
    ui.set_slot2_val(SharedString::from(slots[1].clone()));
    ui.set_slot3_val(SharedString::from(slots[2].clone()));
    ui.set_slot4_val(SharedString::from(slots[3].clone()));
    let voice_slots = crate::hotkey::hotkey_to_slots(&settings.voice_hotkey);
    ui.set_vslot1_val(SharedString::from(voice_slots[0].clone()));
    ui.set_vslot2_val(SharedString::from(voice_slots[1].clone()));
    ui.set_vslot3_val(SharedString::from(voice_slots[2].clone()));
    ui.set_vslot4_val(SharedString::from(voice_slots[3].clone()));
    ui.set_window_width(settings.window_width as i32);
    ui.set_item_height(settings.item_height as i32);
    ui.set_search_bar_height(settings.search_bar_height as i32);
    ui.set_query_font_family(SharedString::from(settings.query_font_family.clone()));
    ui.set_query_font_weight(SharedString::from(settings.query_font_weight.clone()));
    ui.set_query_font_size(settings.query_font_size as i32);
    ui.set_result_title_font_family(SharedString::from(
        settings.result_title_font_family.clone(),
    ));
    ui.set_result_title_font_weight(SharedString::from(
        settings.result_title_font_weight.clone(),
    ));
    ui.set_result_title_font_size(settings.result_title_font_size as i32);
    ui.set_result_subtitle_font_family(SharedString::from(
        settings.result_subtitle_font_family.clone(),
    ));
    ui.set_result_subtitle_font_weight(SharedString::from(
        settings.result_subtitle_font_weight.clone(),
    ));
    ui.set_result_subtitle_font_size(settings.result_subtitle_font_size as i32);
    ui.set_show_placeholder(settings.show_placeholder);

    if let Some(w_path) = get_desktop_wallpaper_path() {
        if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&w_path)) {
            ui.set_desktop_wallpaper(img);
            ui.set_has_desktop_wallpaper(true);
        }
    }

    // Load Agent properties
    ui.set_agent_api_key(SharedString::from(api_key));
    ui.set_agent_endpoint(SharedString::from(endpoint));
    ui.set_agent_model(SharedString::from(model));
    ui.set_agent_always_approve(always_approve);

    // Load Database folders
    let folders_vec: Vec<slint::SharedString> = crate::indexer::get_scan_folders()
        .iter()
        .map(|f| slint::SharedString::from(f.to_string_lossy().to_string()))
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
            let next_voice_hotkey = ui.get_voice_hotkey().to_string();
            if let Err(message) = crate::hotkey::validate_hotkey_unique(
                &next_hotkey,
                &s.global_hotkey,
                Some(&next_voice_hotkey),
            ) {
                ui.set_hotkey_error(SharedString::from(message));
                ui.set_global_hotkey(SharedString::from(s.global_hotkey));
                return;
            }
            if let Err(message) = crate::hotkey::validate_hotkey_unique(
                &next_voice_hotkey,
                &s.voice_hotkey,
                Some(&next_hotkey),
            ) {
                ui.set_voice_hotkey_error(SharedString::from(message));
                ui.set_voice_hotkey(SharedString::from(s.voice_hotkey));
                return;
            }
            s.global_hotkey = next_hotkey;
            s.voice_hotkey = next_voice_hotkey;
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
            ui.set_voice_hotkey_error(SharedString::from(""));
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

    // Re-assemble + validate the hotkey from the 4 dropdowns; apply, save and notify the
    // launcher whenever the selection forms a valid combo. Invalid intermediate states
    // (e.g. only a modifier picked) just show an error and leave the saved hotkey untouched.
    let ui_weak_apply = ui.as_weak();
    ui.on_apply_hotkey(move || {
        let Some(ui) = ui_weak_apply.upgrade() else {
            return;
        };
        let s1 = ui.get_slot1_val().to_string();
        let s2 = ui.get_slot2_val().to_string();
        let s3 = ui.get_slot3_val().to_string();
        let s4 = ui.get_slot4_val().to_string();
        let settings = AppSettings::load();
        let current = settings.global_hotkey;
        let other = settings.voice_hotkey;
        match crate::hotkey::assemble_hotkey(&[&s1, &s2, &s3, &s4], &current, Some(&other)) {
            Ok(combo) => {
                if combo == current {
                    ui.set_hotkey_error(SharedString::from(""));
                    return;
                }
                ui.set_hotkey_error(SharedString::from(""));
                ui.set_global_hotkey(SharedString::from(combo.clone()));
                let mut s = AppSettings::load();
                s.global_hotkey = combo;
                s.save();
                if let Some(hwnd) = find_launcher_hwnd() {
                    unsafe {
                        let _ = PostMessageW(
                            hwnd,
                            windows::Win32::UI::WindowsAndMessaging::WM_USER + 10,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            }
            Err(message) => {
                ui.set_hotkey_error(SharedString::from(message));
            }
        }
    });

    let ui_weak_apply_voice = ui.as_weak();
    ui.on_apply_voice_hotkey(move || {
        let Some(ui) = ui_weak_apply_voice.upgrade() else {
            return;
        };
        let s1 = ui.get_vslot1_val().to_string();
        let s2 = ui.get_vslot2_val().to_string();
        let s3 = ui.get_vslot3_val().to_string();
        let s4 = ui.get_vslot4_val().to_string();
        let settings = AppSettings::load();
        let current = settings.voice_hotkey;
        let other = settings.global_hotkey;
        match crate::hotkey::assemble_hotkey(&[&s1, &s2, &s3, &s4], &current, Some(&other)) {
            Ok(combo) => {
                if combo == current {
                    ui.set_voice_hotkey_error(SharedString::from(""));
                    return;
                }
                ui.set_voice_hotkey_error(SharedString::from(""));
                ui.set_voice_hotkey(SharedString::from(combo.clone()));
                let mut s = AppSettings::load();
                s.voice_hotkey = combo;
                s.save();
                if let Some(hwnd) = find_launcher_hwnd() {
                    unsafe {
                        let _ = PostMessageW(
                            hwnd,
                            windows::Win32::UI::WindowsAndMessaging::WM_USER + 10,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            }
            Err(message) => {
                ui.set_voice_hotkey_error(SharedString::from(message));
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

                    // Notify launcher to reload settings and update watches
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

                // Notify launcher to reload settings and update watches
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
        }
    });

    ui.on_rebuild_index(move || {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let db_path = std::path::PathBuf::from(appdata)
            .join("opensearch-os")
            .join("file_index.db");
        std::thread::spawn(move || {
            let _ = unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED,
                )
            };
            let folders = crate::indexer::get_scan_folders();
            let _ = crate::indexer::run_indexer_folders(&db_path, folders);
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        });
    });

    let ui_weak_status = ui.as_weak();
    std::thread::spawn(move || {
        log_settings_ui("Settings status thread started");

        // Open one persistent connection with WAL + a short busy timeout so we
        // never block the Slint event loop even while the indexer is writing.
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let db_path = std::path::PathBuf::from(&appdata)
            .join("opensearch-os")
            .join("file_index.db");

        let conn = match rusqlite::Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                log_settings_ui(&format!("Status thread: failed to open DB: {:?}", e));
                return;
            }
        };
        // WAL lets readers and writers proceed concurrently; 300ms timeout
        // means we give up quickly rather than stalling the UI thread.
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        let _ = conn.busy_timeout(std::time::Duration::from_millis(300));

        loop {
            // Poll at 1-second intervals — we don't need sub-second UI accuracy here.
            std::thread::sleep(std::time::Duration::from_secs(1));

            let ui_weak = ui_weak_status.clone();
            if ui_weak.upgrade().is_none() {
                log_settings_ui("Settings status thread UI upgrade is None, exiting");
                break;
            }

            // Ensure required tables exist (no-op if already there).
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS indexer_state (key TEXT PRIMARY KEY, value TEXT);
                 CREATE TABLE IF NOT EXISTS files (
                    path TEXT PRIMARY KEY, name TEXT NOT NULL,
                    extension TEXT NOT NULL, modified INTEGER NOT NULL,
                    size INTEGER NOT NULL DEFAULT 0, is_dir INTEGER NOT NULL DEFAULT 0);",
            );

            let is_indexing = conn
                .query_row(
                    "SELECT value FROM indexer_state WHERE key = 'is_indexing'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "0".to_string())
                == "1";

            let progress = conn
                .query_row(
                    "SELECT value FROM indexer_state WHERE key = 'progress'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "Idle".to_string());

            let last_time = conn
                .query_row(
                    "SELECT value FROM indexer_state WHERE key = 'last_index_time'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "Never".to_string());

            let count: i32 = conn
                .query_row("SELECT COUNT(*) FROM files", [], |row| row.get::<_, i64>(0))
                .unwrap_or(0) as i32;

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_db_is_indexing(is_indexing);
                    ui.set_db_status(slint::SharedString::from(progress));
                    ui.set_db_last_indexed(slint::SharedString::from(last_time));
                    ui.set_db_file_count(count);
                }
            });
        }
    });

    // Show the window and run the event loop until it's closed.
    ui.window().show().ok();
    ui.window().set_minimized(false);
    slint::run_event_loop().ok();
}

fn log_settings_ui(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::PathBuf;
    let log_dir = match std::env::var("APPDATA") {
        Ok(d) => PathBuf::from(d).join("opensearch-os"),
        Err(_) => PathBuf::from("."),
    };
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("settings_ui.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(file, "{}", msg);
    }
}

fn get_indexer_state_from_db(conn: &rusqlite::Connection, key: &str, default: &str) -> String {
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS indexer_state (
            key TEXT PRIMARY KEY,
            value TEXT
        );",
        [],
    );
    conn.query_row(
        "SELECT value FROM indexer_state WHERE key = ?",
        [key],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| default.to_string())
}

fn get_db_conn() -> Option<rusqlite::Connection> {
    let appdata = match std::env::var("APPDATA") {
        Ok(val) => val,
        Err(e) => {
            log_settings_ui(&format!("APPDATA env var read failed: {:?}", e));
            return None;
        }
    };
    let path = std::path::PathBuf::from(appdata)
        .join("opensearch-os")
        .join("file_index.db");
    match rusqlite::Connection::open(&path) {
        Ok(conn) => {
            let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
            Some(conn)
        }
        Err(e) => {
            log_settings_ui(&format!("Failed to open DB at {:?}: {:?}", path, e));
            None
        }
    }
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

fn pick_folder() -> Option<std::path::PathBuf> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use windows::core::Interface;
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
        };
        use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, FOS_PICKFOLDERS};

        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let mut path_res = None;

            if let Ok(dialog) =
                CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_ALL)
            {
                if let Ok(options) = dialog.GetOptions() {
                    let _ = dialog.SetOptions(options | FOS_PICKFOLDERS);
                    if dialog.Show(None).is_ok() {
                        if let Ok(result) = dialog.GetResult() {
                            if let Ok(display_name) =
                                result.GetDisplayName(windows::Win32::UI::Shell::SIGDN_FILESYSPATH)
                            {
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
