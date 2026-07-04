slint::include_modules!();

use crate::settings::AppSettings;
use slint::{CloseRequestResponse, ComponentHandle, SharedString};
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use once_cell::sync::Lazy;
use std::sync::Mutex;

static UPDATE_URL: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static DOWNLOADED_PATH: Lazy<Mutex<Option<std::path::PathBuf>>> = Lazy::new(|| Mutex::new(None));
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DISPLAY_VERSION: &str = "1.1.0";

fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .map(|s| s.parse::<u32>().unwrap_or(0))
            .collect()
    };
    let cur_parts = parse(current);
    let lat_parts = parse(latest);
    for i in 0..cur_parts.len().max(lat_parts.len()) {
        let cur_val = cur_parts.get(i).cloned().unwrap_or(0);
        let lat_val = lat_parts.get(i).cloned().unwrap_or(0);
        if lat_val > cur_val {
            return true;
        } else if cur_val > lat_val {
            return false;
        }
    }
    false
}

thread_local! {
    static SETTINGS_WINDOW: std::cell::RefCell<Option<std::rc::Rc<SettingsWindow>>> = std::cell::RefCell::new(None);
}

fn find_launcher_hwnd() -> Option<HWND> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    let class_name: Vec<u16> = "protonsearch\0".encode_utf16().collect();
    if let Ok(hwnd) = unsafe { FindWindowW(PCWSTR(class_name.as_ptr()), None) } {
        if !hwnd.0.is_null() {
            return Some(hwnd);
        }
    }
    None
}

fn notify_launcher_settings_changed() {
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

fn normalize_ignored_folder_name(name: &str) -> Option<String> {
    let trimmed = name.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }
    let trimmed = trimmed.trim_end_matches(['/', '\\']);
    let leaf = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed).trim();
    if leaf.is_empty() {
        None
    } else {
        Some(leaf.to_string())
    }
}

fn ignored_folders_model() -> slint::ModelRc<slint::SharedString> {
    let folders: Vec<slint::SharedString> = crate::indexer::get_ignored_folder_names()
        .iter()
        .map(|f| slint::SharedString::from(f.clone()))
        .collect();
    slint::ModelRc::new(slint::VecModel::from(folders))
}

fn get_desktop_wallpaper_path() -> Option<String> {
    // 1. Try Roaming AppData themes transcoded wallpaper (most reliable on Win10/11 for active slideshows/custom wallpapers)
    if let Ok(app_data) = std::env::var("APPDATA") {
        let transcoded = std::path::Path::new(&app_data)
            .join("Microsoft")
            .join("Windows")
            .join("Themes")
            .join("TranscodedWallpaper");
        if transcoded.exists() {
            return Some(transcoded.to_string_lossy().to_string());
        }
    }

    // 2. Try SystemParametersInfoW as fallback
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
        let name: Vec<u16> = "Local\\ProtonSearchSettingsMutex\0"
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

    // Clean up .bak files on startup
    if let Ok(current_exe) = std::env::current_exe() {
        let backup_exe = current_exe.with_extension("bak");
        if backup_exe.exists() {
            let _ = std::fs::remove_file(backup_exe);
        }
    }

    std::env::set_var("SLINT_STYLE", "fluent-dark");

    let ui = match SettingsWindow::new() {
        Ok(u) => std::rc::Rc::new(u),
        Err(_) => return,
    };

    // Store in thread-local so it is never dropped during the event loop
    SETTINGS_WINDOW.with(|cell| {
        *cell.borrow_mut() = Some(std::rc::Rc::clone(&ui));
    });

    // Load current settings
    let settings = AppSettings::load();
    let (api_key, endpoint, model, always_approve) = load_ai_settings();

    // Keep the public About label compact while Cargo uses semver for update comparisons.
    ui.set_app_version(DISPLAY_VERSION.into());
    ui.set_run_on_startup(settings.run_on_startup);
    ui.set_hide_on_lose_focus(settings.hide_on_lose_focus);
    ui.set_show_taskbar(settings.show_taskbar);
    ui.set_window_location(SharedString::from(settings.window_location.clone()));
    ui.set_theme_mode(SharedString::from(settings.normalized_theme_mode()));
    ui.set_global_hotkey(SharedString::from(settings.global_hotkey.clone()));
    ui.set_hotkey_available(true);
    ui.set_hotkey_error(SharedString::from("Current hotkey is active."));

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
    ui.set_plugin_circle_search(settings.plugin_circle_search);
    ui.set_plugin_text_expansions(settings.plugin_text_expansions);
    ui.set_plugin_color_picker(settings.plugin_color_picker);
    ui.set_plugin_calculator(settings.plugin_calculator);
    ui.set_plugin_git_commits(settings.plugin_git_commits);
    ui.set_show_clipboard_image_text_action(settings.show_clipboard_image_text_action);
    load_snippets_into_ui(&ui);

    // Populate initial database status & statistics instantly on startup
    if let Some(conn) = get_db_conn() {
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

        ui.set_db_is_indexing(is_indexing);
        ui.set_db_status(slint::SharedString::from(progress));
        ui.set_db_last_indexed(slint::SharedString::from(last_time));
        ui.set_db_file_count(count);
    }

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
    ui.set_db_ignored_folders(ignored_folders_model());

    // Load Available Agents from DB
    let mut slint_agents = Vec::new();
    if let Some(conn) = get_db_conn() {
        if let Ok(mut stmt) = conn.prepare("SELECT id, name, goal FROM agents ORDER BY ts DESC") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(SlintAgent {
                    id: row.get::<_, i32>(0)?,
                    name: SharedString::from(row.get::<_, String>(1)?),
                    goal: SharedString::from(row.get::<_, String>(2)?),
                })
            }) {
                for agent in rows.filter_map(|r| r.ok()) {
                    slint_agents.push(agent);
                }
            }
        }
    }
    let agents_model = slint::ModelRc::new(slint::VecModel::from(slint_agents));
    ui.set_available_agents(agents_model);

    // Close = hide window, terminate event loop
    let ui_weak_close = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = ui_weak_close.upgrade() {
            if let Some(hwnd) = find_launcher_hwnd() {
                unsafe {
                    let _ = PostMessageW(
                        hwnd,
                        crate::WM_RELOAD_SETTINGS,
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

    // Launch agent callback
    let ui_weak_launch = ui.as_weak();
    ui.on_launch_agent(move |agent_id| {
        if let Some(ui) = ui_weak_launch.upgrade() {
            if let Some(hwnd) = find_launcher_hwnd() {
                unsafe {
                    let _ = PostMessageW(
                        hwnd,
                        crate::WM_LAUNCH_AGENT,
                        windows::Win32::Foundation::WPARAM(agent_id as usize),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
            ui.window().hide().ok();
            slint::quit_event_loop().ok();
        }
    });

    let ui_weak_add_snippet = ui.as_weak();
    ui.on_add_snippet(move || {
        let Some(ui) = ui_weak_add_snippet.upgrade() else {
            return;
        };
        let name = ui.get_snippet_name().to_string();
        let keyword = ui.get_snippet_keyword().to_string();
        let content = ui.get_snippet_content().to_string();
        let name = name.trim();
        let keyword_raw = keyword.trim();
        let keyword = if keyword_raw.is_empty() {
            name.to_lowercase()
        } else {
            keyword_raw.to_lowercase()
        };
        let content = content.trim();
        if name.is_empty() || content.is_empty() {
            return;
        }
        if let Some(conn) = get_db_conn() {
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO snippets (name, content, keyword) VALUES (?1, ?2, ?3);",
                rusqlite::params![name, content, keyword],
            ) {
                log_settings_ui(&format!("Failed to save snippet '{}': {:?}", name, e));
            }
        }
        ui.set_snippet_name(SharedString::from(""));
        ui.set_snippet_keyword(SharedString::from(""));
        ui.set_snippet_content(SharedString::from(""));
        load_snippets_into_ui(&ui);
    });

    let ui_weak_delete_snippet = ui.as_weak();
    ui.on_delete_snippet(move |snippet_id| {
        let Some(ui) = ui_weak_delete_snippet.upgrade() else {
            return;
        };
        if let Some(conn) = get_db_conn() {
            let _ = conn.execute("DELETE FROM snippets WHERE rowid = ?1;", [snippet_id]);
        }
        load_snippets_into_ui(&ui);
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
            if let Err(message) =
                crate::hotkey::validate_hotkey_unique(&next_hotkey, &s.global_hotkey, None)
            {
                ui.set_hotkey_available(false);
                ui.set_hotkey_error(SharedString::from(message));
                ui.set_global_hotkey(SharedString::from(s.global_hotkey.clone()));
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
            s.plugin_circle_search = ui.get_plugin_circle_search();
            s.plugin_text_expansions = ui.get_plugin_text_expansions();
            s.plugin_color_picker = ui.get_plugin_color_picker();
            s.plugin_calculator = ui.get_plugin_calculator();
            s.plugin_git_commits = ui.get_plugin_git_commits();
            s.show_clipboard_image_text_action = ui.get_show_clipboard_image_text_action();
            s.save();
            ui.set_hotkey_available(true);
            ui.set_hotkey_error(SharedString::from("Hotkey available and saved."));

            let run_on_startup = s.run_on_startup;
            let api_key = ui.get_agent_api_key().to_string();
            let endpoint = ui.get_agent_endpoint().to_string();
            let model = ui.get_agent_model().to_string();
            let always_approve = ui.get_agent_always_approve();

            std::thread::spawn(move || {
                crate::settings_startup::sync_run_on_startup(run_on_startup);
                save_ai_settings(
                    &api_key.trim(),
                    &endpoint.trim(),
                    &model.trim(),
                    always_approve,
                );
            });

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

    ui.on_open_url(move |url| {
        let url_wide: Vec<u16> = url
            .as_str()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        use windows::core::{w, PCWSTR};
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        unsafe {
            let _ = ShellExecuteW(
                HWND::default(),
                w!("open"),
                PCWSTR(url_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
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
        match crate::hotkey::assemble_hotkey(&[&s1, &s2, &s3, &s4], &current, None) {
            Ok(combo) => {
                if combo == current {
                    ui.set_hotkey_available(true);
                    ui.set_hotkey_error(SharedString::from("Current hotkey is active."));
                    return;
                }
                ui.set_hotkey_available(true);
                ui.set_hotkey_error(SharedString::from("Hotkey available and saved."));
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
                ui.set_hotkey_available(false);
                ui.set_hotkey_error(SharedString::from(message));
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
                    s.scan_folders.push(path_str.clone());
                    s.save();

                    // Immediately scan/index the newly added folder in the background
                    let path_str_clone = path_str.clone();
                    std::thread::spawn(move || {
                        let com_ok = unsafe {
                            windows::Win32::System::Com::CoInitializeEx(
                                None,
                                windows::Win32::System::Com::COINIT_MULTITHREADED,
                            )
                        }
                        .is_ok();
                        let appdata = std::env::var("APPDATA").unwrap_or_default();
                        let db_path = std::path::PathBuf::from(appdata)
                            .join("protonsearch")
                            .join("file_index.db");
                        let _ = crate::indexer::run_indexer_folders_force(
                            &db_path,
                            vec![std::path::PathBuf::from(path_str_clone)],
                        );
                        if com_ok {
                            unsafe { windows::Win32::System::Com::CoUninitialize() };
                        }
                    });
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

    let ui_weak_add_ignore = ui.as_weak();
    ui.on_add_ignored_folder(move || {
        if let Some(ui) = ui_weak_add_ignore.upgrade() {
            let input = ui.get_db_ignore_input().to_string();
            if let Some(name) = normalize_ignored_folder_name(&input) {
                let _ = crate::indexer::add_ignored_folder_name(&name);
                ui.set_db_ignored_folders(ignored_folders_model());
                notify_launcher_settings_changed();
                ui.set_db_ignore_input(SharedString::from(""));
            }
        }
    });

    let ui_weak_remove_ignore = ui.as_weak();
    ui.on_remove_ignored_folder(move |idx| {
        if let Some(ui) = ui_weak_remove_ignore.upgrade() {
            let folders = crate::indexer::get_ignored_folder_names();
            if idx >= 0 && (idx as usize) < folders.len() {
                let _ = crate::indexer::remove_ignored_folder_name(&folders[idx as usize]);
                ui.set_db_ignored_folders(ignored_folders_model());
                notify_launcher_settings_changed();
            }
        }
    });

    ui.on_rebuild_index(move || {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let db_path = std::path::PathBuf::from(appdata)
            .join("protonsearch")
            .join("file_index.db");
        std::thread::spawn(move || {
            let com_ok = unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED,
                )
            }
            .is_ok();
            let folders = crate::indexer::get_scan_folders();
            let _ = crate::indexer::run_indexer_folders(&db_path, folders);
            if com_ok {
                unsafe { windows::Win32::System::Com::CoUninitialize() };
            }
        });
    });

    let ui_weak_check = ui.as_weak();
    ui.on_check_for_updates(move || {
        let ui_weak = ui_weak_check.clone();
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_update_status("checking".into());
        }

        std::thread::spawn(move || {
            let res = ureq::get(
                "https://raw.githubusercontent.com/PranshulSoni/protonsearch/main/update.json",
            )
            .timeout(std::time::Duration::from_secs(10))
            .call();

            let status = match res {
                Ok(response) => match response.into_json::<serde_json::Value>() {
                    Ok(json) => {
                        let latest_version = json["version"].as_str().unwrap_or("0.0.0");
                        let download_url = json["url"].as_str().unwrap_or("");

                        if is_newer_version(CURRENT_VERSION, latest_version)
                            && !download_url.is_empty()
                        {
                            if let Ok(mut url_lock) = UPDATE_URL.lock() {
                                *url_lock = Some(download_url.to_string());
                            }
                            let ver = latest_version.to_string();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak.upgrade() {
                                    ui.set_update_status("available".into());
                                    ui.set_update_version(ver.into());
                                }
                            });
                            return;
                        } else {
                            "uptodate"
                        }
                    }
                    Err(_) => "error",
                },
                Err(ureq::Error::Status(404, _)) => "uptodate", // 404 means no updates published yet!
                Err(_) => "error",
            };

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_update_status(status.into());

                    if status == "uptodate" {
                        let ui_weak_reset = ui_weak.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs(4));
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak_reset.upgrade() {
                                    if ui.get_update_status() == "uptodate" {
                                        ui.set_update_status("idle".into());
                                    }
                                }
                            });
                        });
                    }
                }
            });
        });
    });

    let ui_weak_download = ui.as_weak();
    ui.on_download_update(move || {
        let ui_weak = ui_weak_download.clone();
        let url = match UPDATE_URL.lock().unwrap().clone() {
            Some(u) => u,
            None => return,
        };

        std::thread::spawn(move || {
            let _ = slint::invoke_from_event_loop({
                let ui_weak = ui_weak.clone();
                move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_update_status("downloading".into());
                        ui.set_update_progress(0);
                    }
                }
            });

            // Socket-level timeouts only: a total-request timeout caps the whole body
            // transfer and made multi-MB installer downloads fail on slow connections.
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(15))
                .timeout_read(std::time::Duration::from_secs(30))
                .build();
            let res = match agent.get(&url).call() {
                Ok(r) => r,
                Err(_) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_update_status("error".into());
                        }
                    });
                    return;
                }
            };

            let total_size = res
                .header("Content-Length")
                .and_then(|len| len.parse::<u64>().ok())
                .unwrap_or(0);

            let temp_dir = std::env::temp_dir();
            let temp_path = temp_dir.join("protonsearch_update.exe");

            let mut file = match std::fs::File::create(&temp_path) {
                Ok(f) => f,
                Err(_) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_update_status("error".into());
                        }
                    });
                    return;
                }
            };

            let mut reader = res.into_reader();
            let mut buffer = [0; 16384];
            let mut downloaded = 0u64;

            loop {
                use std::io::{Read, Write};
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if file.write_all(&buffer[..n]).is_err() {
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak.upgrade() {
                                    ui.set_update_status("error".into());
                                }
                            });
                            return;
                        }
                        downloaded += n as u64;
                        if total_size > 0 {
                            let percent = ((downloaded as f32 / total_size as f32) * 100.0) as i32;
                            let ui_weak = ui_weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_weak.upgrade() {
                                    ui.set_update_progress(percent);
                                }
                            });
                        }
                    }
                    Err(_) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_update_status("error".into());
                            }
                        });
                        return;
                    }
                }
            }

            if let Ok(mut path_lock) = DOWNLOADED_PATH.lock() {
                *path_lock = Some(temp_path);
            }

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_update_status("ready".into());
                }
            });
        });
    });

    ui.on_install_update(move || {
        let downloaded_path = match DOWNLOADED_PATH.lock().unwrap().clone() {
            Some(p) => p,
            None => return,
        };

        // 1. Ask the main launcher to close gracefully via WM_CLOSE
        let class_name: Vec<u16> = "protonsearch\0".encode_utf16().collect();
        if let Ok(hwnd) = unsafe {
            windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                windows::core::PCWSTR(class_name.as_ptr()),
                None,
            )
        } {
            if !hwnd.0.is_null() {
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
        }

        // 2. Poll-wait for the launcher window to disappear (up to 3s).
        //    This ensures the main process has fully exited and released the
        //    exe file handle before we spawn the installer.
        for _ in 0..30 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let gone = unsafe {
                windows::Win32::UI::WindowsAndMessaging::FindWindowW(
                    windows::core::PCWSTR(class_name.as_ptr()),
                    None,
                )
            };
            match gone {
                Ok(h) if h.0.is_null() => break,
                Err(_) => break,
                _ => {}
            }
        }

        // 3. Launch installer DETACHED so it survives the launcher shutdown.
        //    /SILENT skips the wizard UI since the user already clicked "Install".
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        let _ = std::process::Command::new(&downloaded_path)
            .args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn();

        // 4. Brief pause to let OS register the detached process, then quit
        std::thread::sleep(std::time::Duration::from_millis(500));
        slint::quit_event_loop().ok();
    });

    let ui_weak_status = ui.as_weak();
    std::thread::spawn(move || {
        log_settings_ui("Settings status thread started");

        // Open one persistent connection with WAL + a short busy timeout so we
        // never block the Slint event loop even while the indexer is writing.
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let db_path = std::path::PathBuf::from(&appdata)
            .join("protonsearch")
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
            // Poll at 3-second intervals — we don't need sub-second UI accuracy here.
            std::thread::sleep(std::time::Duration::from_secs(3));

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

            let ui_weak = ui_weak_status.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            let schedule_result = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_db_is_indexing(is_indexing);
                    ui.set_db_status(slint::SharedString::from(progress));
                    ui.set_db_last_indexed(slint::SharedString::from(last_time));
                    ui.set_db_file_count(count);
                    let _ = tx.send(true);
                } else {
                    let _ = tx.send(false);
                }
            });
            if schedule_result.is_err() {
                // The event loop itself is gone (not just busy) - no closure was ever
                // queued, so tx will never send and every future tick would time out
                // forever. This is the only case worth stopping for.
                log_settings_ui("Settings status thread: event loop gone, exiting");
                break;
            }

            match rx.recv_timeout(std::time::Duration::from_millis(500)) {
                Ok(true) => {} // UI still alive
                Ok(false) => {
                    // upgrade() inside the closure returned None: the window is confirmed gone.
                    log_settings_ui("Settings status thread UI is dead, exiting");
                    break;
                }
                Err(_) => {
                    // Timed out or disconnected: the round-trip didn't complete in 500ms, which
                    // just means the UI thread hasn't run the closure yet (e.g. it's blocked in
                    // a modal folder-picker dialog or the install-update flow's sleeps) - not
                    // proof the window is dead. Skip this tick and try again in 3s instead of
                    // permanently stopping status updates for the rest of the Settings session.
                    log_settings_ui("Settings status thread: UI busy, will retry next tick");
                }
            }
        }
    });

    // Show the window and run the event loop until it's closed.
    ui.window().show().ok();
    ui.window().set_minimized(false);
    slint::run_event_loop().ok();

    // Clear thread-local reference to allow it to be dropped
    SETTINGS_WINDOW.with(|cell| {
        *cell.borrow_mut() = None;
    });
    drop(ui);
}

fn log_settings_ui(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::PathBuf;
    let log_dir = match std::env::var("APPDATA") {
        Ok(d) => PathBuf::from(d).join("protonsearch"),
        Err(_) => PathBuf::from("."),
    };
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("settings_ui.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        if file
            .metadata()
            .map(|m| m.len() > 1024 * 1024)
            .unwrap_or(false)
        {
            let _ = file.set_len(0);
        }
        let _ = writeln!(file, "{}", msg);
    }
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
        .join("protonsearch")
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

fn load_snippets_into_ui(ui: &SettingsWindow) {
    let mut snippets = Vec::new();
    if let Some(conn) = get_db_conn() {
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS snippets (
                name TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                keyword TEXT
            );",
            [],
        );
        if let Ok(mut stmt) = conn.prepare(
            "SELECT rowid, name, COALESCE(NULLIF(keyword, ''), name), content FROM snippets ORDER BY name ASC",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(SlintSnippet {
                    id: row.get::<_, i32>(0)?,
                    name: SharedString::from(row.get::<_, String>(1)?),
                    keyword: SharedString::from(row.get::<_, String>(2)?),
                    content: SharedString::from(row.get::<_, String>(3)?),
                })
            }) {
                snippets.extend(rows.filter_map(|row| row.ok()));
            }
        }
    }
    ui.set_snippets(slint::ModelRc::new(slint::VecModel::from(snippets)));
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
    if let Some(mut conn) = get_db_conn() {
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        if let Ok(tx) = conn.transaction() {
            let _ = tx.execute(
                "CREATE TABLE IF NOT EXISTS ai_settings (key TEXT PRIMARY KEY, value TEXT);",
                [],
            );
            let _ = tx.execute(
                "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('api_key', ?);",
                [api_key],
            );
            let _ = tx.execute(
                "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('endpoint', ?);",
                [endpoint],
            );
            let _ = tx.execute(
                "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('model', ?);",
                [model],
            );
            let _ = tx.execute(
                "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('always_approve', ?);",
                [if always_approve { "1" } else { "0" }],
            );
            let _ = tx.commit();
        }
    }
}

fn pick_folder() -> Option<std::path::PathBuf> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
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
