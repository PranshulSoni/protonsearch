#![windows_subsystem = "windows"]
use protonsearch::search::{SearchEngine, SearchResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
mod clipboard_widget;
mod dock_lifecycle;
mod known_folders;
mod local_calendar;
mod running_apps;
mod widget_data;
mod window_geometry;
use window_geometry::{DockGeometry, Region};

struct Engine {
    engine: Option<SearchEngine>,
    results: HashMap<String, SearchResult>,
    generation: u64,
    latest_request: u64,
}
struct NativeState {
    engine: Arc<Mutex<Engine>>,
    persistence: Mutex<()>,
    dock_geometry: Mutex<Option<DockGeometry>>,
    search_height: Mutex<f64>,
    search_focus_epoch: AtomicU64,
    widget_card: Mutex<Option<WidgetCardState>>,
    widget_focus_epoch: AtomicU64,
    dock_lifecycle: Mutex<dock_lifecycle::State>,
    lifecycle_running: Arc<AtomicBool>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WidgetCardState {
    id: String,
    size: String,
    settings: Value,
}
#[derive(Clone, Deserialize)]
struct Anchor {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultRow {
    id: String,
    title: String,
    detail: String,
    r#type: String,
    actions: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    request_id: u64,
    results: Vec<ResultRow>,
    elapsed_ms: f64,
}
const WIDGET_CARD_IDS: [&str; 14] = [
    "windows", "search", "apps", "spotify", "weather", "system", "claude", "chatgpt", "agent",
    "files", "folders", "trash", "volume", "clock",
];
const WIDGET_CARD_SIZES: [&str; 3] = ["compact", "standard", "wide"];
const WIDGET_SETTINGS_LIMIT: usize = 64 * 1024;

fn valid_widget_card_id(id: &str) -> bool {
    WIDGET_CARD_IDS.contains(&id)
}
fn valid_widget_card_size(size: &str) -> bool {
    WIDGET_CARD_SIZES.contains(&size)
}
fn widget_card_settings(settings: Option<Value>) -> Value {
    let Some(settings) = settings else {
        return json!({});
    };
    if !settings.is_object() {
        return json!({});
    }
    match serde_json::to_vec(&settings) {
        Ok(bytes) if bytes.len() <= WIDGET_SETTINGS_LIMIT => settings,
        _ => json!({}),
    }
}
fn widget_card_position(
    work: [i32; 4],
    dock_position: (i32, i32),
    dock_size: (u32, u32),
    card_size: (u32, u32),
    anchor: Option<&Anchor>,
    scale: f64,
) -> [i32; 4] {
    let [left, top, right, bottom] = work;
    let card_w = card_size.0.max(1) as i32;
    let card_h = card_size.1.max(1) as i32;
    if right <= left || bottom <= top {
        return [0, 0, card_w, card_h];
    }
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let anchor = anchor.filter(|anchor| {
        [anchor.x, anchor.y, anchor.width, anchor.height]
            .iter()
            .all(|value| value.is_finite())
            && anchor.width >= 0.0
            && anchor.height >= 0.0
    });
    let center_x = anchor.map_or_else(
        || dock_position.0 as f64 + dock_size.0 as f64 / 2.0,
        |anchor| dock_position.0 as f64 + (anchor.x + anchor.width / 2.0) * scale,
    );
    let desired_y = dock_position.1 as f64 - card_h as f64 - 10.0 * scale;
    let max_x = (right - card_w).max(left);
    let max_y = (bottom - card_h).max(top);
    let x = (center_x - card_w as f64 / 2.0)
        .round()
        .clamp(left as f64, max_x as f64)
        .max(0.0) as i32;
    let y = desired_y.round().clamp(top as f64, max_y as f64).max(0.0) as i32;
    [x, y, card_w, card_h]
}

impl Engine {
    fn publish_results(&mut self, found: Vec<SearchResult>, limit: usize) -> Vec<ResultRow> {
        self.generation += 1;
        let generation = self.generation;
        self.results.clear();
        found
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(index, result)| {
                let id = format!("{generation}:{index}");
                let row = ResultRow {
                    id: id.clone(),
                    title: result.entry.control_name.clone(),
                    detail: result.entry.breadcrumb_path.clone(),
                    r#type: result.entry.source.clone(),
                    actions: vec!["open".into()],
                };
                self.results.insert(id, result);
                row
            })
            .collect()
    }
}
fn data_dir() -> Result<PathBuf, String> {
    let path = PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA unavailable")?)
        .join("protonsearch");
    let old = path.with_file_name("omnisearch");
    if !path.exists() && old.exists() {
        std::fs::rename(&old, &path).map_err(|e| format!("Legacy data migration failed: {e}"))?;
    }
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}
#[tauri::command]
async fn search(
    query: String,
    limit: Option<usize>,
    request_id: u64,
    state: tauri::State<'_, NativeState>,
) -> Result<SearchResponse, String> {
    if query.len() > 4096 {
        return Err("Query too long".into());
    }
    let shared = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let mut state = shared.lock().map_err(|e| e.to_string())?;
        if request_id < state.latest_request {
            return Ok(SearchResponse {
                request_id,
                results: Vec::new(),
                elapsed_ms: 0.0,
            });
        }
        state.latest_request = request_id;
        if state.engine.is_none() {
            state.engine = Some(
                SearchEngine::new(data_dir()?.join("file_index.db"), true)
                    .map_err(|e| e.to_string())?,
            );
        }
        let limit = limit.unwrap_or(15).clamp(1, 100);
        let found = state
            .engine
            .as_mut()
            .ok_or("Engine unavailable")?
            .search(&query, limit);
        let results = state.publish_results(found, limit);
        Ok(SearchResponse {
            request_id,
            results,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[derive(Debug, PartialEq)]
enum SavedSearchAction<'a> {
    CopySnippet(&'a str),
    OpenQuicklink(tauri::Url),
}

fn saved_search_action(command: &str) -> Result<Option<SavedSearchAction<'_>>, String> {
    if let Some(text) = command.strip_prefix("copy_snippet:") {
        return Ok(Some(SavedSearchAction::CopySnippet(text)));
    }
    if let Some(target) = command.strip_prefix("open_quicklink:") {
        let url = tauri::Url::parse(target).map_err(|_| "Invalid quicklink URL")?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("Only HTTP and HTTPS quicklinks are supported".into());
        }
        return Ok(Some(SavedSearchAction::OpenQuicklink(url)));
    }
    Ok(None)
}

#[tauri::command]
async fn execute_search_result(
    result_id: String,
    action_id: Option<String>,
    state: tauri::State<'_, NativeState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if action_id.as_deref().is_some_and(|id| id != "open") {
        return Err("Unsupported action".into());
    }
    let result = state
        .engine
        .lock()
        .map_err(|e| e.to_string())?
        .results
        .get(&result_id)
        .cloned()
        .ok_or("Result expired; search again")?;
    // Commands originate only from the engine; frontend paths and command strings are never accepted.
    let command = &result.entry.launch_command;
    match saved_search_action(command)? {
        Some(SavedSearchAction::CopySnippet(text)) => return copy_text(text),
        Some(SavedSearchAction::OpenQuicklink(url)) => {
            tauri::async_runtime::spawn_blocking(move || {
                widget_data::open_uri(url.as_str()).map_err(|error| error.to_string())
            })
            .await
            .map_err(|error| error.to_string())??;
            return hide_search(app);
        }
        None => {}
    }
    if [
        "bookmarks:",
        "history:",
        "commits:",
        "todos:",
        "clip:",
        "file:",
        "folder:",
        "code:",
        "switch:",
        "window:",
        "ql:",
        "snip:",
        "img:",
        "memory:",
        "chats:",
        "agents:",
        "agentchats:",
        "notes:",
    ]
    .contains(&command.as_str())
    {
        return show_search(&app, command);
    }
    if let Some(text) = command.strip_prefix("copy:") {
        copy_text(text)?;
        return hide_search(app);
    }
    if let Some(path) = command.strip_prefix("open_note:") {
        std::process::Command::new("notepad.exe")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
        return hide_search(app);
    }
    let supported = std::path::Path::new(command).exists()
        || [
            "https://",
            "http://",
            "ms-settings:",
            "shell:",
            "control.exe",
            "window:",
            "vscode:",
        ]
        .iter()
        .any(|prefix| command.starts_with(prefix))
        || command.ends_with(".msc")
        || command.ends_with(".exe");
    if !supported {
        return Err(
            "This action requires the legacy ProtonSearch launcher during migration".into(),
        );
    }
    protonsearch::launcher::launch(command);
    hide_search(app)
}
fn show_search(app: &tauri::AppHandle, query: &str) -> Result<(), String> {
    app.state::<NativeState>()
        .search_focus_epoch
        .fetch_add(1, Ordering::AcqRel);
    let window = app
        .get_webview_window("search")
        .ok_or("Search window unavailable")?;
    let height = *app
        .state::<NativeState>()
        .search_height
        .lock()
        .map_err(|e| e.to_string())?;
    position_search(app, &window, height)?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    // Tauri's window focus does not focus the hosted WebView's keyboard input.
    let webview: &tauri::Webview = window.as_ref();
    webview.set_focus().map_err(|e| e.to_string())?;
    protonsearch::applog::log("Search activation requested for native window and WebView");
    window
        .emit("search-query", query)
        .map_err(|e| e.to_string())?;
    window.eval("requestAnimationFrame(()=>requestAnimationFrame(()=>document.querySelector('input')?.focus()))").map_err(|e|e.to_string())
}
#[tauri::command]
fn open_search(app: tauri::AppHandle, query: Option<String>) -> Result<(), String> {
    show_search(&app, query.as_deref().unwrap_or(""))
}
#[tauri::command]
fn hide_search(app: tauri::AppHandle) -> Result<(), String> {
    app.state::<NativeState>()
        .search_focus_epoch
        .fetch_add(1, Ordering::AcqRel);
    app.get_webview_window("search")
        .ok_or("Search window unavailable")?
        .hide()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn fit_dock(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, NativeState>,
    width: f64,
    height: f64,
    regions: Vec<Region>,
) -> Result<(), String> {
    if window.label() != "dock" {
        return Err("Dock window required".into());
    }
    let geometry = DockGeometry {
        width,
        height,
        regions,
    };
    let first_fit = state
        .dock_geometry
        .lock()
        .map_err(|e| e.to_string())?
        .is_none();
    let reveal = {
        let mut lifecycle = state.dock_lifecycle.lock().map_err(|e| e.to_string())?;
        if first_fit && lifecycle.suppressed {
            lifecycle.restore_visible = true;
        }
        !lifecycle.suppressed
    };
    window_geometry::fit(&window, &geometry, reveal).map_err(|error| {
        protonsearch::applog::log(&format!("Cannot fit dock: {error}"));
        if let Err(fallback_error) = window_geometry::show_fallback(&window, reveal) {
            protonsearch::applog::log(&format!("Cannot show dock fallback: {fallback_error}"));
        }
        error
    })?;
    *state.dock_geometry.lock().map_err(|e| e.to_string())? = Some(geometry);
    Ok(())
}
fn position_search(
    app: &tauri::AppHandle,
    search: &tauri::WebviewWindow,
    height: f64,
) -> Result<(), String> {
    let dock = app
        .get_webview_window("dock")
        .ok_or("Dock window unavailable")?;
    let (area, scale) = window_geometry::work_area(&dock)?;
    window_geometry::place(
        search,
        window_geometry::bounds(area, scale, 638.0, height, false)?,
    )
}
#[tauri::command]
fn fit_search(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, NativeState>,
    height: f64,
) -> Result<(), String> {
    if window.label() != "search" || !height.is_finite() {
        return Err("Invalid search geometry".into());
    }
    let height = height.clamp(174.0, 500.0);
    position_search(window.app_handle(), &window, height)?;
    *state.search_height.lock().map_err(|e| e.to_string())? = height;
    Ok(())
}
#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("settings")
        .ok_or("Settings unavailable")?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())
}
#[tauri::command]
fn hide_settings(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("settings")
        .ok_or("Settings unavailable")?
        .hide()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn open_start() -> Result<(), String> {
    // Installed-apps fallback; AppsFolder is not the native Start menu.
    protonsearch::launcher::launch("shell:AppsFolder");
    Ok(())
}
#[tauri::command]
fn load_dock_settings(state: tauri::State<'_, NativeState>) -> Result<Value, String> {
    let _guard = state.persistence.lock().map_err(|e| e.to_string())?;
    let path = data_dir()?.join("protondock-layout.json");
    if !path.exists() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_dock_settings(
    value: Value,
    state: tauri::State<'_, NativeState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let _guard = state.persistence.lock().map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
    if !value.is_object() || bytes.len() > 1024 * 1024 {
        return Err("Invalid layout".into());
    }
    let path = data_dir()?.join("protondock-layout.json");
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    unsafe {
        use windows::{
            core::PCWSTR,
            Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            },
        };
        let from: Vec<u16> = tmp
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let to: Vec<u16> = path
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|e| e.to_string())?;
    }
    app.emit("settings-updated", ())
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
async fn provider_snapshot(provider_id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || provider_snapshot_data(provider_id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn known_folders_snapshot() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(known_folders::snapshot)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn widget_action(action_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || widget_data::action(&action_id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn open_widget_card(
    app: tauri::AppHandle,
    state: tauri::State<'_, NativeState>,
    id: String,
    size: String,
    settings: Option<Value>,
    anchor: Option<Anchor>,
) -> Result<(), String> {
    if !valid_widget_card_id(&id) {
        return Err(format!("Invalid widget card id: {id}"));
    }
    if !valid_widget_card_size(&size) {
        return Err(format!("Invalid widget card size: {size}"));
    }
    let card = WidgetCardState {
        id,
        size,
        settings: widget_card_settings(settings),
    };
    *state.widget_card.lock().map_err(|e| e.to_string())? = Some(card.clone());

    let dock = app
        .get_webview_window("dock")
        .ok_or("Dock window unavailable")?;
    let widget = app
        .get_webview_window("widget")
        .ok_or("Widget window unavailable")?;
    let (work_area, scale) = window_geometry::work_area(&dock)?;
    let dock_position = dock.outer_position().map_err(|e| e.to_string())?;
    let dock_size = dock.outer_size().map_err(|e| e.to_string())?;
    let fallback_size = (
        (520.0 * scale).round().max(1.0) as u32,
        (430.0 * scale).round().max(1.0) as u32,
    );
    let initial_size = widget
        .outer_size()
        .ok()
        .filter(|size| size.width > 0 && size.height > 0)
        .map(|size| (size.width, size.height))
        .unwrap_or(fallback_size);
    let placement = |card_size| {
        widget_card_position(
            work_area,
            (dock_position.x, dock_position.y),
            (dock_size.width, dock_size.height),
            card_size,
            anchor.as_ref(),
            scale,
        )
    };
    let position = placement(initial_size);
    widget
        .set_position(tauri::PhysicalPosition::new(position[0], position[1]))
        .map_err(|e| e.to_string())?;
    state.widget_focus_epoch.fetch_add(1, Ordering::AcqRel);
    widget.show().map_err(|e| e.to_string())?;
    // Moving a window between monitors can change its physical DPI size.
    if let Ok(actual) = widget.outer_size() {
        let actual = (actual.width.max(1), actual.height.max(1));
        if actual != initial_size {
            let position = placement(actual);
            widget
                .set_position(tauri::PhysicalPosition::new(position[0], position[1]))
                .map_err(|e| e.to_string())?;
        }
    }
    if let Err(error) = app.emit("widget-card:show", &card) {
        let _ = widget.hide();
        return Err(error.to_string());
    }
    widget.set_focus().map_err(|e| e.to_string())
}
#[tauri::command]
fn hide_widget_card(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("widget")
        .ok_or("Widget window unavailable")?
        .hide()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn widget_card_state(
    state: tauri::State<'_, NativeState>,
) -> Result<Option<WidgetCardState>, String> {
    Ok(state.widget_card.lock().map_err(|e| e.to_string())?.clone())
}
#[tauri::command]
async fn clipboard_snapshot(state: tauri::State<'_, NativeState>) -> Result<Value, String> {
    let shared = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let engine = shared.lock().map_err(|e| e.to_string())?;
        clipboard_snapshot_data(&engine)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn running_apps_snapshot(state: tauri::State<'_, NativeState>) -> Result<Value, String> {
    let shared = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let results = {
            let engine = shared.lock().map_err(|e| e.to_string())?;
            engine
                .engine
                .as_ref()
                .ok_or("Search engine not ready")?
                .search_windows("")
        };
        running_apps::snapshot(results)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn running_apps_activate(id: String) -> Result<(), String> {
    if id.len() > 128 {
        return Err("Invalid window ID".into());
    }
    tauri::async_runtime::spawn_blocking(move || running_apps::activate(&id))
        .await
        .map_err(|e| e.to_string())?
}
fn clipboard_snapshot_data(engine: &Engine) -> Result<Value, String> {
    Ok(clipboard_widget::snapshot(
        engine.engine.as_ref().ok_or("Search engine not ready")?,
    ))
}
#[tauri::command]
async fn weather_snapshot() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        protonsearch::weather::snapshot(&data_dir()?.join("protondock-layout.json"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn calendar_select_source(
    window: tauri::WebviewWindow,
    profile: String,
) -> Result<Option<Value>, String> {
    if window.label() != "settings" {
        return Err("Open calendar settings to select a source".into());
    }
    let directory = data_dir()?;
    let owner = window.hwnd().map_err(|error| error.to_string())?.0 as isize;
    tauri::async_runtime::spawn_blocking(move || {
        local_calendar::select_source(directory, profile, owner)
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
async fn calendar_snapshot(profile: String, token: String) -> Result<Value, String> {
    let directory = data_dir()?;
    tauri::async_runtime::spawn_blocking(move || {
        local_calendar::snapshot(&directory, &profile, &token)
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
async fn calendar_forget_source(
    window: tauri::WebviewWindow,
    profile: String,
    token: String,
) -> Result<(), String> {
    if window.label() != "settings" {
        return Err("Open calendar settings to unlink a source".into());
    }
    let directory = data_dir()?;
    tauri::async_runtime::spawn_blocking(move || {
        local_calendar::forget_source(&directory, &profile, &token)
    })
    .await
    .map_err(|error| error.to_string())?
}
fn provider_snapshot_data(provider_id: String) -> Result<Value, String> {
    if matches!(
        provider_id.as_str(),
        "network" | "media" | "volume" | "trash"
    ) {
        return widget_data::snapshot(&provider_id);
    }
    let updated = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    let (status, data, message) = match provider_id.as_str() {
        "system" => unsafe {
            use windows::Win32::System::{
                Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
                SystemInformation::{GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX},
            };
            let mut memory = MEMORYSTATUSEX {
                dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                ..Default::default()
            };
            GlobalMemoryStatusEx(&mut memory).map_err(|e| e.to_string())?;
            let mut power = SYSTEM_POWER_STATUS::default();
            let (battery, charging) = if GetSystemPowerStatus(&mut power).is_ok() {
                (
                    (power.BatteryLifePercent != 255).then_some(power.BatteryLifePercent),
                    battery_charging(power.BatteryLifePercent, power.BatteryFlag),
                )
            } else {
                (None, None)
            };
            let connectivity = widget_data::snapshot("network")
                .ok()
                .and_then(|snapshot| snapshot.get("data").cloned())
                .unwrap_or(Value::Null);
            (
                "live",
                json!({
                    "ram":memory.dwMemoryLoad,
                    "battery":battery,
                    "batteryCharging":charging,
                    "cpu":cpu_usage(),
                    "temperatureC":Value::Null,
                    "uptimeSeconds":GetTickCount64() / 1000,
                    "connectivity":connectivity
                }),
                "",
            )
        },
        "storage" => unsafe {
            use windows::{core::PCWSTR, Win32::Storage::FileSystem::GetDiskFreeSpaceExW};
            let root = format!("{}\\", std::env::var("SystemDrive").unwrap_or("C:".into()));
            let wide: Vec<u16> = root.encode_utf16().chain(Some(0)).collect();
            let (mut available, mut total, mut free) = (0, 0, 0);
            GetDiskFreeSpaceExW(
                PCWSTR(wide.as_ptr()),
                Some(&mut available),
                Some(&mut total),
                Some(&mut free),
            )
            .map_err(|e| e.to_string())?;
            (
                "live",
                json!({"totalBytes":total,"freeBytes":free,"mountPoint":root}),
                "",
            )
        },
        _ => return Err("Unknown provider".into()),
    };
    Ok(json!({"status":status,"data":data,"updatedAt":updated,"message":message}))
}
fn cpu_usage() -> Option<f64> {
    use windows::Win32::{Foundation::FILETIME, System::Threading::GetSystemTimes};
    static PREVIOUS: std::sync::OnceLock<Mutex<Option<(u64, u64)>>> = std::sync::OnceLock::new();
    let (mut idle, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    unsafe {
        GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()?;
    }
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    let current = (ticks(idle), ticks(kernel) + ticks(user));
    let mut previous = PREVIOUS.get_or_init(|| Mutex::new(None)).lock().ok()?;
    let result = previous.and_then(|old| cpu_percent(old, current));
    *previous = Some(current);
    result
}
fn cpu_percent(previous: (u64, u64), current: (u64, u64)) -> Option<f64> {
    let total = current.1.checked_sub(previous.1)?;
    let idle = current.0.checked_sub(previous.0)?;
    (total > 0).then(|| 100.0 * (total.saturating_sub(idle)) as f64 / total as f64)
}
fn battery_charging(percent: u8, flags: u8) -> Option<bool> {
    // BatteryFlag: 8 = charging, 128 = no system battery, 255 = unknown.
    if percent == 255 || flags == 255 || flags & 0x80 != 0 {
        None
    } else {
        Some(flags & 0x08 != 0)
    }
}
fn copy_text(text: &str) -> Result<(), String> {
    use windows::Win32::{
        Foundation::{GlobalFree, HANDLE, HWND},
        System::{
            DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
        },
    };
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let _guard = protonsearch::clipboard_lock()
        .lock()
        .map_err(|e| e.to_string())?;
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2).map_err(|e| e.to_string())?;
        let pointer = GlobalLock(memory);
        if pointer.is_null() {
            let _ = GlobalFree(memory);
            return Err("Clipboard allocation failed".into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), pointer.cast::<u16>(), wide.len());
        let _ = GlobalUnlock(memory);
        if let Err(e) = OpenClipboard(HWND::default()) {
            let _ = GlobalFree(memory);
            return Err(e.to_string());
        }
        let result =
            EmptyClipboard().and_then(|_| SetClipboardData(13, HANDLE(memory.0)).map(|_| ()));
        let _ = CloseClipboard();
        if result.is_err() {
            let _ = GlobalFree(memory);
        }
        result.map_err(|e| e.to_string())
    }
}
fn position_dock(dock: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let state = dock.app_handle().state::<NativeState>();
    let geometry = state
        .dock_geometry
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    if let Some(geometry) = geometry {
        let reveal = !state
            .dock_lifecycle
            .lock()
            .map_err(|e| e.to_string())?
            .suppressed;
        window_geometry::fit(dock, &geometry, reveal)?;
    } else {
        let (area, scale) = window_geometry::work_area(dock)?;
        window_geometry::place(
            dock,
            window_geometry::bounds(area, scale, 1200.0, 132.0, true)?,
        )?;
    }
    Ok(())
}
fn refresh_dock_lifecycle(app: &tauri::AppHandle, monitor_changed: bool) -> Result<(), String> {
    let dock = app.get_webview_window("dock").ok_or("Dock unavailable")?;
    let snapshot = dock_lifecycle::sample(windows::Win32::Foundation::HWND(
        dock.hwnd().map_err(|e| e.to_string())?.0,
    ))?;
    let state = app.state::<NativeState>();
    let visible = dock.is_visible().map_err(|e| e.to_string())?;
    let (previous, action) = {
        let mut lifecycle = state.dock_lifecycle.lock().map_err(|e| e.to_string())?;
        let previous = *lifecycle;
        (
            previous,
            lifecycle.update(snapshot.fullscreen, visible, monitor_changed),
        )
    };
    let apply = || -> Result<(), String> {
        match action {
            dock_lifecycle::Action::Hide => dock.hide().map_err(|e| e.to_string())?,
            dock_lifecycle::Action::Restore => {
                position_dock(&dock).map_err(|e| e.to_string())?;
                dock.show().map_err(|e| e.to_string())?;
            }
            dock_lifecycle::Action::Reposition => {
                position_dock(&dock).map_err(|e| e.to_string())?
            }
            dock_lifecycle::Action::None => {}
        };
        Ok(())
    };
    if let Err(error) = apply() {
        *state.dock_lifecycle.lock().map_err(|e| e.to_string())? = previous;
        return Err(error);
    }
    if monitor_changed {
        if let Some(search) = app
            .get_webview_window("search")
            .filter(|w| w.is_visible().unwrap_or(false))
        {
            let height = *state.search_height.lock().map_err(|e| e.to_string())?;
            position_search(app, &search, height)?;
        }
    }
    Ok(())
}
fn search_owns_foreground(window: &tauri::WebviewWindow) -> bool {
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
    let Ok(hwnd) = window.hwnd() else {
        return false;
    };
    // SAFETY: these APIs only inspect the current foreground window and its owner.
    unsafe {
        let foreground = GetForegroundWindow();
        foreground == HWND(hwnd.0)
            || (!foreground.0.is_null() && GetAncestor(foreground, GA_ROOTOWNER) == HWND(hwnd.0))
    }
}
fn blur_is_current(scheduled: u64, current: u64, owns_foreground: bool) -> bool {
    scheduled == current && !owns_foreground
}
fn defer_search_blur(window: &tauri::Window) {
    let epoch = window
        .app_handle()
        .state::<NativeState>()
        .search_focus_epoch
        .load(Ordering::Acquire);
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(120));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let state = handle.state::<NativeState>();
            if state.search_focus_epoch.load(Ordering::Acquire) != epoch
                || !protonsearch::settings::AppSettings::load().hide_on_lose_focus
            {
                return;
            }
            if let Some(search) = handle.get_webview_window("search") {
                if blur_is_current(
                    epoch,
                    state.search_focus_epoch.load(Ordering::Acquire),
                    search_owns_foreground(&search),
                ) && search.is_visible().unwrap_or(false)
                {
                    let _ = search.hide();
                    protonsearch::applog::log(
                        "Search dismissed after native foreground moved away",
                    );
                }
            }
        });
    });
}
fn defer_widget_card_hide(window: &tauri::Window) {
    let epoch = window
        .app_handle()
        .state::<NativeState>()
        .widget_focus_epoch
        .load(Ordering::Acquire);
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(80));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let state = handle.state::<NativeState>();
            if state.widget_focus_epoch.load(Ordering::Acquire) != epoch {
                return;
            }
            if let Some(widget) = handle.get_webview_window("widget") {
                if widget.is_visible().unwrap_or(false) && !widget.is_focused().unwrap_or(true) {
                    let _ = widget.hide();
                }
            }
        });
    });
}
fn recover_taskbar(exiting: bool) -> Result<(), String> {
    let layout = PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA unavailable")?)
        .join("protonsearch")
        .join("dock-layout.json");
    protonsearch::taskbar_recovery::restore_after_exit(
        &layout,
        exiting.then_some(std::process::id()),
    )
    .map_err(|error| format!("Taskbar recovery failed: {error:#}"))
}
fn log_taskbar_recovery(exiting: bool) {
    if let Err(error) = recover_taskbar(exiting) {
        protonsearch::applog::log(&error);
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Runs before WebView/shortcut initialization, including when the UI cannot start.
    if args.get(1).is_some_and(|arg| arg == "--restore-taskbar") {
        if let Err(error) = recover_taskbar(false) {
            protonsearch::applog::log(&error);
            std::process::exit(1);
        }
        return;
    }
    if let Some(pos) = args.iter().position(|a| a == "--extract-content") {
        if let Some(path) = args.get(pos + 1) {
            protonsearch::indexer::extract_content_subprocess(path);
        }
        return;
    }
    if let Some(query) = std::env::args()
        .nth(1)
        .filter(|arg| arg == "--smoke-search")
        .and_then(|_| std::env::args().nth(2))
    {
        let mut engine = SearchEngine::new(
            data_dir().expect("data directory").join("file_index.db"),
            true,
        )
        .expect("real engine");
        let results = engine.search(&query, 15);
        let rows: Vec<_> = results
            .iter()
            .map(|r| json!({"title":r.entry.control_name,"source":r.entry.source}))
            .collect();
        std::fs::write(
            "native-search-smoke.json",
            serde_json::to_vec_pretty(&json!({"query":query,"results":rows})).unwrap(),
        )
        .unwrap();
        assert!(!rows.is_empty(), "real search must return results");
        return;
    }
    log_taskbar_recovery(false);
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = show_search(app, "");
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        let _ = show_search(app, "");
                    }
                })
                .build(),
        )
        .manage(NativeState {
            engine: Arc::new(Mutex::new(Engine {
                engine: None,
                results: HashMap::new(),
                generation: 0,
                latest_request: 0,
            })),
            persistence: Mutex::new(()),
            dock_geometry: Mutex::new(None),
            search_height: Mutex::new(174.0),
            search_focus_epoch: AtomicU64::new(0),
            widget_card: Mutex::new(None),
            widget_focus_epoch: AtomicU64::new(0),
            dock_lifecycle: Mutex::new(dock_lifecycle::State::default()),
            lifecycle_running: Arc::new(AtomicBool::new(true)),
        })
        .invoke_handler(tauri::generate_handler![
            search,
            execute_search_result,
            open_search,
            hide_search,
            open_start,
            open_settings,
            hide_settings,
            load_dock_settings,
            save_dock_settings,
            provider_snapshot,
            known_folders_snapshot,
            fit_dock,
            fit_search,
            open_widget_card,
            hide_widget_card,
            widget_card_state,
            widget_action,
            clipboard_snapshot,
            running_apps_snapshot,
            running_apps_activate,
            weather_snapshot,
            calendar_select_source,
            calendar_snapshot,
            calendar_forget_source
        ])
        .setup(|app| {
            app.global_shortcut().register("Alt+Space")?;
            let configured = protonsearch::settings::AppSettings::load().global_hotkey;
            if configured != "Alt+Space" {
                app.global_shortcut().register(configured.as_str())?;
            }
            // Initialize schema before the indexers acquire write transactions.
            let shared = app.state::<NativeState>();
            shared.engine.lock().map_err(|e| e.to_string())?.engine =
                Some(SearchEngine::new(data_dir()?.join("file_index.db"), true)?);
            let dock = app.get_webview_window("dock").ok_or("dock missing")?;
            position_dock(&dock)?;
            if let Err(error) = refresh_dock_lifecycle(app.handle(), true) {
                protonsearch::applog::log(&format!("Initial dock lifecycle inspection: {error}"));
            }
            dock_lifecycle::start(
                app.handle().clone(),
                dock.hwnd()?.0 as isize,
                shared.lifecycle_running.clone(),
            )?;
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let search = MenuItem::with_id(app, "search", "Search", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Customize", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit ProtonDock", true, None::<&str>)?;
            let restore = MenuItem::with_id(
                app,
                "restore-taskbar",
                "Restore Windows taskbar",
                true,
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&search, &settings, &restore, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("ProtonDock")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "search" => {
                        let _ = show_search(app, "");
                    }
                    "settings" => {
                        let _ = open_settings(app.clone());
                    }
                    "quit" => app.exit(0),
                    "restore-taskbar" => log_taskbar_recovery(false),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            let db = data_dir()?.join("file_index.db");
            protonsearch::indexer::start_indexer(db.clone());
            protonsearch::indexer::start_watcher(db.clone());
            protonsearch::browser_indexer::start_browser_indexer(db.clone());
            if protonsearch::settings::AppSettings::load().plugin_git_commits {
                protonsearch::git_indexer::start_git_indexer(db);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::ScaleFactorChanged { .. } if window.label() == "dock" => {
                if let Some(dock) = window.app_handle().get_webview_window("dock") {
                    if dock.is_visible().unwrap_or(false) {
                        let _ = position_dock(&dock);
                    }
                }
            }
            tauri::WindowEvent::Focused(true) if window.label() == "widget" => {
                window
                    .app_handle()
                    .state::<NativeState>()
                    .widget_focus_epoch
                    .fetch_add(1, Ordering::AcqRel);
            }
            tauri::WindowEvent::Focused(false)
                if window.label() == "widget" && window.is_visible().unwrap_or(false) =>
            {
                defer_widget_card_hide(window);
            }
            tauri::WindowEvent::Focused(true) if window.label() == "search" => {
                window
                    .app_handle()
                    .state::<NativeState>()
                    .search_focus_epoch
                    .fetch_add(1, Ordering::AcqRel);
            }
            tauri::WindowEvent::Focused(false)
                if window.label() == "search"
                    && protonsearch::settings::AppSettings::load().hide_on_lose_focus =>
            {
                defer_search_blur(window);
            }
            _ => {}
        })
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<NativeState>()
                    .lifecycle_running
                    .store(false, Ordering::Release);
                log_taskbar_recovery(true);
            }
            if matches!(event, tauri::RunEvent::Resumed) {
                if let Err(error) = refresh_dock_lifecycle(app, true) {
                    protonsearch::applog::log(&format!("Dock resume: {error}"));
                }
            }
        }),
        Err(error) => {
            log_taskbar_recovery(true);
            protonsearch::applog::log(&format!("ProtonDock native host failed: {error}"));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_snippets_preserve_exact_content_and_do_not_classify_other_commands() {
        for text in ["", "hello", " नमस्ते 🦀\nsecond:line\r\n "] {
            assert_eq!(
                saved_search_action(&format!("copy_snippet:{text}")).unwrap(),
                Some(SavedSearchAction::CopySnippet(text))
            );
        }
        for command in [
            "copy:text",
            "snip:",
            "notes:",
            "open_note:C:\\note.txt",
            "action:create_snippet",
        ] {
            assert_eq!(saved_search_action(command).unwrap(), None);
        }
    }

    #[test]
    fn saved_quicklinks_accept_only_parsed_web_urls() {
        for target in [
            "http://example.com/",
            "https://example.com/path?q=a:b#part",
            "HTTPS://example.com:8443/नमस्ते",
            "https://chatgpt.com/?q=test",
        ] {
            assert_eq!(
                saved_search_action(&format!("open_quicklink:{target}")).unwrap(),
                Some(SavedSearchAction::OpenQuicklink(
                    tauri::Url::parse(target).unwrap()
                ))
            );
        }
        for target in [
            "",
            "example.com",
            "https://",
            "http://[",
            "javascript:alert(1)",
            "file:///C:/test.exe",
            "ms-settings:display",
            "shell:AppsFolder",
            "cmd.exe /c echo test",
            "https://example.com:invalid/",
        ] {
            assert!(
                saved_search_action(&format!("open_quicklink:{target}")).is_err(),
                "{target}"
            );
        }
    }

    #[test]
    fn overdelivered_candidates_are_bounded_before_action_id_publication() {
        use protonsearch::search::CatalogEntry;
        let mut engine = Engine {
            engine: None,
            results: HashMap::new(),
            generation: 0,
            latest_request: 8,
        };
        let candidates = (0..921)
            .map(|i| SearchResult {
                entry: CatalogEntry {
                    id: format!("candidate-{i}"),
                    control_name: format!("Rank {i}"),
                    breadcrumb_path: String::new(),
                    launch_command: format!("fixture-{i}"),
                    source: "fixture".into(),
                    description: String::new(),
                    synonyms: String::new(),
                },
                score: (921 - i) as f32,
            })
            .collect();
        let rows = engine.publish_results(candidates, 30);
        assert_eq!(rows.len(), 30);
        assert_eq!(engine.results.len(), 30);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.title, format!("Rank {i}"));
            assert_eq!(engine.results[&row.id].entry.id, format!("candidate-{i}"));
        }
        assert!(!engine.results.contains_key("1:30"));
        let next = engine.publish_results(vec![], 30);
        assert!(next.is_empty());
        assert!(engine.results.is_empty());
        assert_eq!((engine.generation, engine.latest_request), (2, 8));
    }
    #[test]
    fn clipboard_snapshot_preserves_spotlight_action_generation() {
        use protonsearch::search::CatalogEntry;
        let result = SearchResult {
            entry: CatalogEntry {
                id: "preserved".into(),
                control_name: "Fixture".into(),
                breadcrumb_path: String::new(),
                launch_command: "fixture".into(),
                source: "fixture".into(),
                description: String::new(),
                synonyms: String::new(),
            },
            score: 1.0,
        };
        let engine = Engine {
            engine: Some(SearchEngine::new(":memory:".into(), false).unwrap()),
            results: HashMap::from([("action-7".into(), result)]),
            generation: 7,
            latest_request: 9,
        };
        let snapshot = clipboard_snapshot_data(&engine).unwrap();
        assert_eq!(snapshot["status"], "empty");
        assert_eq!((engine.generation, engine.latest_request), (7, 9));
        assert_eq!(engine.results.len(), 1);
        assert_eq!(engine.results["action-7"].entry.id, "preserved");
    }
    #[test]
    fn delayed_blur_does_not_hide_reopened_or_still_active_search() {
        assert!(!blur_is_current(1, 2, false));
        assert!(!blur_is_current(2, 2, true));
        assert!(blur_is_current(2, 2, false));
    }
    #[test]
    fn cpu_sampling_uses_deltas_and_handles_first_interval() {
        assert_eq!(cpu_percent((10, 100), (30, 200)), Some(80.0));
        assert_eq!(cpu_percent((10, 100), (10, 100)), None);
        assert_eq!(cpu_percent((10, 100), (0, 0)), None);
    }
    #[test]
    fn response_matches_typescript_contract() {
        let response = SearchResponse {
            request_id: 7,
            results: vec![ResultRow {
                id: "1:0".into(),
                title: "Notepad".into(),
                detail: "Application".into(),
                r#type: "app".into(),
                actions: vec!["open".into()],
            }],
            elapsed_ms: 2.0,
        };
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["requestId"], 7);
        assert_eq!(value["results"][0]["type"], "app");
        assert_eq!(value["results"][0]["actions"][0], "open");
        assert_eq!(value["elapsedMs"], 2.0);
    }
    #[test]
    fn unknown_provider_is_denied() {
        assert_eq!(
            provider_snapshot_data("shell".into()).unwrap_err(),
            "Unknown provider"
        );
    }
    #[test]
    fn storage_provider_returns_native_capacity() {
        let snapshot = provider_snapshot_data("storage".into()).unwrap();
        assert_eq!(snapshot["status"], "live");
        assert!(snapshot["data"]["totalBytes"].as_u64().unwrap() > 0);
        assert!(
            snapshot["data"]["freeBytes"].as_u64().unwrap()
                <= snapshot["data"]["totalBytes"].as_u64().unwrap()
        );
    }
    #[test]
    fn battery_charging_requires_a_readable_battery() {
        assert_eq!(battery_charging(80, 8), Some(true));
        assert_eq!(battery_charging(80, 1), Some(false));
        assert_eq!(battery_charging(255, 8), None);
        assert_eq!(battery_charging(80, 255), None);
        assert_eq!(battery_charging(80, 128), None);
        assert_eq!(battery_charging(80, 136), None);
    }
    #[test]
    fn system_provider_exposes_extended_fields_without_dropping_existing_ones() {
        let snapshot = provider_snapshot_data("system".into()).unwrap();
        assert_eq!(snapshot["status"], "live");
        for key in [
            "cpu",
            "ram",
            "battery",
            "batteryCharging",
            "temperatureC",
            "uptimeSeconds",
            "connectivity",
        ] {
            assert!(snapshot["data"].get(key).is_some(), "missing {key}");
        }
        assert!(snapshot["data"]["ram"].as_u64().unwrap() <= 100);
        assert!(snapshot["data"]["uptimeSeconds"].as_u64().unwrap() > 0);
        assert!(snapshot["data"]["temperatureC"].is_null());
        assert!(
            snapshot["data"]["batteryCharging"].is_boolean()
                || snapshot["data"]["batteryCharging"].is_null()
        );
        let connectivity = &snapshot["data"]["connectivity"];
        assert!(connectivity.is_object() || connectivity.is_null());
        if connectivity.is_object() {
            assert!(["wifi", "ethernet", "vpn", "none"]
                .contains(&connectivity["connectionType"].as_str().unwrap()));
            assert!(connectivity["wifi"]["state"].is_string());
        }
        println!(
            "System provider: cpu={}, ram={}, battery={}, charging={}, uptime={}s, connectivity={}",
            snapshot["data"]["cpu"],
            snapshot["data"]["ram"],
            snapshot["data"]["battery"],
            snapshot["data"]["batteryCharging"],
            snapshot["data"]["uptimeSeconds"],
            snapshot["data"]["connectivity"]["connectionType"]
        );
    }
    #[test]
    fn volume_and_trash_providers_are_routed_and_shaped() {
        let volume = provider_snapshot_data("volume".into()).unwrap();
        assert!(["live", "empty", "unsupported", "permission-denied"]
            .contains(&volume["status"].as_str().unwrap()));
        if volume["status"] == "live" {
            assert!(volume["data"]["level"].as_u64().unwrap() <= 100);
            assert!(volume["data"]["muted"].is_boolean());
            assert!(volume["data"]["device"].is_string());
        }
        let trash = provider_snapshot_data("trash".into()).unwrap();
        assert!(["live", "unsupported", "permission-denied"]
            .contains(&trash["status"].as_str().unwrap()));
        if trash["status"] == "live" {
            assert!(trash["data"]["itemCount"].is_u64());
            assert!(trash["data"]["sizeBytes"].is_u64());
            assert!(trash["data"]["lastEmptiedAt"].is_null());
        }
        println!(
            "Volume provider: status={}, level={}, muted={}, device={}; trash: status={}, items={}, bytes={}",
            volume["status"],
            volume["data"]["level"],
            volume["data"]["muted"],
            volume["data"]["device"],
            trash["status"],
            trash["data"]["itemCount"],
            trash["data"]["sizeBytes"]
        );
    }
    #[test]
    fn known_folders_provider_has_safe_shape() {
        let snapshot = known_folders::snapshot();
        assert!(["live", "empty", "unsupported", "permission-denied"]
            .contains(&snapshot["status"].as_str().unwrap()));
        assert!(snapshot["updatedAt"].as_u64().unwrap() > 0);
        assert!(snapshot["data"]["folders"].is_array());
        assert!(snapshot["data"]["recent"].is_array());
        for folder in snapshot["data"]["folders"].as_array().unwrap() {
            assert!(folder["id"].is_string());
            assert!(folder["name"].is_string());
            let path = folder["path"].as_str().unwrap();
            assert!(known_folders::is_local_absolute_path(std::path::Path::new(
                path
            )));
        }
        assert!(snapshot["data"]["recent"].as_array().unwrap().len() <= 12);
        println!(
            "Known folders: status={}, folders={}, recent={}",
            snapshot["status"],
            snapshot["data"]["folders"].as_array().unwrap().len(),
            snapshot["data"]["recent"].as_array().unwrap().len()
        );
    }
    #[test]
    fn widget_card_ids_and_sizes_are_allowlisted() {
        for id in WIDGET_CARD_IDS {
            assert!(valid_widget_card_id(id));
        }
        assert!(!valid_widget_card_id(""));
        assert!(!valid_widget_card_id("calendar"));
        assert!(!valid_widget_card_id("../dock"));
        for size in WIDGET_CARD_SIZES {
            assert!(valid_widget_card_size(size));
        }
        assert!(!valid_widget_card_size("large"));
        assert!(!valid_widget_card_size(""));
    }
    #[test]
    fn widget_card_settings_are_objects_under_the_size_cap() {
        assert_eq!(widget_card_settings(None), json!({}));
        assert_eq!(
            widget_card_settings(Some(json!({"theme": "dark"}))),
            json!({"theme": "dark"})
        );
        assert_eq!(widget_card_settings(Some(json!("dark"))), json!({}));
        assert_eq!(widget_card_settings(Some(json!(["dark"]))), json!({}));
        let oversized = json!({"payload": "x".repeat(WIDGET_SETTINGS_LIMIT)});
        assert_eq!(widget_card_settings(Some(oversized)), json!({}));
    }
    #[test]
    fn widget_card_position_centers_above_anchor_and_clamps_to_work_area() {
        let anchor = Anchor {
            x: 800.0,
            y: 40.0,
            width: 40.0,
            height: 40.0,
        };
        assert_eq!(
            widget_card_position(
                [0, 0, 1920, 1080],
                (0, 948),
                (1200, 132),
                (520, 430),
                Some(&anchor),
                1.0
            ),
            [560, 508, 520, 430]
        );
        let left_edge = Anchor {
            x: 100.0,
            y: 40.0,
            width: 40.0,
            height: 40.0,
        };
        assert_eq!(
            widget_card_position(
                [0, 0, 1920, 1080],
                (0, 948),
                (1200, 132),
                (520, 430),
                Some(&left_edge),
                1.0
            )[0],
            0
        );
        let right_edge = Anchor {
            x: 20.0,
            y: 40.0,
            width: 20.0,
            height: 20.0,
        };
        assert_eq!(
            widget_card_position(
                [0, 0, 1920, 1080],
                (1700, 948),
                (1200, 132),
                (520, 430),
                Some(&right_edge),
                1.0
            )[0],
            1400
        );
        assert_eq!(
            widget_card_position(
                [0, 0, 1920, 1080],
                (0, 100),
                (1200, 132),
                (520, 430),
                None,
                1.0
            ),
            [340, 0, 520, 430]
        );
    }
}
