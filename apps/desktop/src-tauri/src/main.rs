//! ProtonSearch v2 Tauri shell. OWNER: L0 (lead).
//!
//! Wires the Rust engine to the WebView UI, owns window/hotkey/tray lifecycle
//! and executes structured actions. No shell string interpolation and no
//! network access live here.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ps_agent::{AgentClient, AgentEvent};
use ps_core::{Engine, EngineConfig};
use ps_ipc::{
    ActionSpec, ActivityItem, ActivityQuery, AgentApprovalResponse, AgentRunRequest, AgentState,
    AgentStatus, EngineStatus, IndexCommand, NativeCommand, OnboardingInput, OnboardingState,
    OpenOutcome, SearchRequest, SearchResponse, Settings, SettingsPatch,
};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const LAUNCHER_LABEL: &str = "launcher";
/// Incremented on every summon/dismiss; a pending safety-hide only runs when the
/// generation is unchanged (design: re-summon must never be hidden mid-open).
static LAUNCHER_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const MAIN_LABEL: &str = "main";
const LAUNCHER_CARD_WIDTH: f64 = 820.0;
const LAUNCHER_PREVIEW_CARD_WIDTH: f64 = 1180.0;
const LAUNCHER_DEFAULT_HEIGHT: f64 = 520.0;
/// Transparent breathing room around the launcher card so the CSS shadow is
/// never clipped by the native window edge. 28 logical px on every side.
const LAUNCHER_SHADOW_PAD: f64 = 28.0;
fn launcher_window_width(preview: bool) -> f64 {
    (if preview { LAUNCHER_PREVIEW_CARD_WIDTH } else { LAUNCHER_CARD_WIDTH }) + LAUNCHER_SHADOW_PAD * 2.0
}
fn launcher_window_height(content_height: f64) -> f64 {
    content_height + LAUNCHER_SHADOW_PAD * 2.0
}

struct AppState {
    engine: Arc<Engine>,
    data_dir: PathBuf,
    /// Optional Python agent sidecar. Created only when a workflow is started;
    /// never touched by search, startup or settings.
    agent: Mutex<Option<AgentClient>>,
}

/// Position the frameless launcher near the top-center of the monitor under the
/// cursor (multi-monitor correctness), clamped into that monitor's work area.
fn place_launcher(app: &AppHandle, win: &WebviewWindow, width: f64, height: f64) {
    let cursor = app.cursor_position().unwrap_or(tauri::PhysicalPosition::new(0.0, 0.0));
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| win.current_monitor().ok().flatten());
    let (mx, my, mw, mh, scale) = match monitor {
        Some(m) => {
            let pos = m.position();
            let size = m.size();
            (pos.x as f64, pos.y as f64, size.width as f64, size.height as f64, m.scale_factor())
        }
        None => (0.0, 0.0, 1920.0, 1080.0, 1.0),
    };
    let pw = width * scale;
    let ph = height * scale;
    let x = mx + ((mw - pw) / 2.0).max(12.0);
    let desired_y = my + (mh * 0.16).min(260.0 * scale).max(24.0);
    let y = desired_y.min(my + mh - ph - 12.0).max(my + 8.0);
    let _ = win.set_position(PhysicalPosition::new(x as i32, y as i32));
}

fn show_launcher_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window(LAUNCHER_LABEL) else {
        return;
    };
    // Cancel any pending dismissal from a previous close animation.
    LAUNCHER_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let scale = win.scale_factor().unwrap_or(1.0);
    let size = win
        .inner_size()
        .map(|s| (s.width as f64 / scale, s.height as f64 / scale))
        .unwrap_or((launcher_window_width(false), launcher_window_height(LAUNCHER_DEFAULT_HEIGHT)));
    place_launcher(app, &win, size.0, size.1);
    let _ = win.show();
    let _ = win.set_focus();
    let _ = win.emit("launcher:summoned", ());
}

/// Ask the launcher to play its exit animation; the frontend calls
/// hide_launcher once the animation finished (0 ms in reduced motion).
fn dismiss_launcher(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LAUNCHER_LABEL) {
        let generation = LAUNCHER_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let _ = win.emit("launcher:dismiss", ());
        // Safety net: if the renderer is wedged the window must still disappear,
        // but never hide a launcher that was re-summoned in the meantime.
        let handle = win.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(400));
            if LAUNCHER_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == generation {
                let _ = handle.hide();
                let _ = handle.emit("launcher:hidden", ());
            }
        });
    }
}
fn hide_launcher_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LAUNCHER_LABEL) {
        let _ = win.hide();
        let _ = win.emit("launcher:hidden", ());
    }
}

fn toggle_launcher(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LAUNCHER_LABEL) {
        let visible = win.is_visible().unwrap_or(false);
        let focused = win.is_focused().unwrap_or(false);
        if visible && focused {
            // Let the UI play its exit animation before the window disappears.
            dismiss_launcher(app);
        } else {
            show_launcher_window(app);
        }
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open ProtonSearch", true, None::<&str>)?;
    let search = MenuItem::with_id(app, "search", "Search (Alt+Space)", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause indexing", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Resume indexing", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit ProtonSearch", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &search, &pause, &resume, &sep, &quit])?;

    // Explicit tray icon: never depend on default_window_icon(), which can be
    // absent and would silently produce an invisible tray entry.
    let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
        .map_err(|err| tauri::Error::AssetNotFound(err.to_string()))?;
    let builder = TrayIconBuilder::with_id("protonsearch-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("ProtonSearch — Alt+Space to search")
        .icon(tray_icon);
    builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "search" => show_launcher_window(app),
            "pause" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = state.engine.index_command(IndexCommand::Pause);
                }
            }
            "resume" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = state.engine.index_command(IndexCommand::Resume);
                }
            }
            "quit" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = state.engine.index_command(IndexCommand::Shutdown);
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_launcher(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Test-only escape hatch: when PROTONSEARCH_CDP_PORT is set the WebView2
/// renderer opens a loopback DevTools socket so the E2E harness can drive the
/// real app (screenshots + latency). Not set in production; loopback only.
/// The embedded app icon (PNG). Used for both windows so the title bar and
/// taskbar entries are never blank, independent of bundle icon resolution.
fn window_icon() -> tauri::Result<tauri::image::Image<'static>> {
    tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
        .map_err(|err| tauri::Error::AssetNotFound(err.to_string()))
}

fn cdp_browser_args() -> Option<String> {
    // Debug builds only: a release binary must never expose a CDP socket.
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("PROTONSEARCH_CDP_PORT")
        .ok()
        .filter(|port| port.chars().all(|c| c.is_ascii_digit()) && !port.is_empty())
        .map(|port| format!("--remote-debugging-port={port}"))
}

/// When PROTONSEARCH_DATA_DIR is set (portable install / E2E sandbox), keep the
/// WebView2 profile inside it instead of %LOCALAPPDATA%\<identifier>.
fn webview_data_dir() -> Option<PathBuf> {
    std::env::var_os("PROTONSEARCH_DATA_DIR")
        .filter(|dir| !dir.is_empty())
        .map(|dir| PathBuf::from(dir).join("webview2"))
}

fn create_windows(app: &AppHandle) -> tauri::Result<()> {
    let cdp = cdp_browser_args();
    let webview_dir = webview_data_dir();
    if let Some(dir) = &webview_dir {
        let _ = std::fs::create_dir_all(dir);
    }
    if app.get_webview_window(MAIN_LABEL).is_none() {
        let mut main_builder = WebviewWindowBuilder::new(app, MAIN_LABEL, tauri::WebviewUrl::App("index.html".into()))
            .title("ProtonSearch")
            .inner_size(1180.0, 800.0)
            .min_inner_size(560.0, 480.0)
            .visible(false)
            .center();
        if let Some(args) = &cdp {
            main_builder = main_builder.additional_browser_args(args);
        }
        if let Some(dir) = &webview_dir {
            main_builder = main_builder.data_directory(dir.clone());
        }
        let main = main_builder.build()?;
        let _ = main.set_icon(window_icon()?);
        // Clamp the desktop window to the current monitor (design review L5):
        // never let the caption buttons or bottom edge fall outside the work area.
        if let Ok(Some(monitor)) = main.current_monitor() {
            let scale = monitor.scale_factor();
            let mw = monitor.size().width as f64 / scale;
            let mh = monitor.size().height as f64 / scale;
            let width = 1180.0_f64.min(mw - 48.0).max(560.0);
            let height = 800.0_f64.min(mh - 96.0).max(480.0);
            let _ = main.set_size(tauri::LogicalSize::new(width, height));
            let _ = main.center();
        }
        let win = main.clone();
        main.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Keep the app alive in the tray; explicit Quit exits.
                api.prevent_close();
                let _ = win.hide();
            }
        });
    }

    if app.get_webview_window(LAUNCHER_LABEL).is_none() {
        let mut launcher_builder = WebviewWindowBuilder::new(
            app,
            LAUNCHER_LABEL,
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("ProtonSearch")
        .inner_size(launcher_window_width(false), launcher_window_height(LAUNCHER_DEFAULT_HEIGHT))
        .min_inner_size(560.0, 160.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false);
        if let Some(args) = &cdp {
            launcher_builder = launcher_builder.additional_browser_args(args);
        }
        if let Some(dir) = &webview_dir {
            launcher_builder = launcher_builder.data_directory(dir.clone());
        }
        let launcher = launcher_builder.build()?;
        let _ = launcher.set_icon(window_icon()?);
        place_launcher(app, &launcher, launcher_window_width(false), launcher_window_height(LAUNCHER_DEFAULT_HEIGHT));
        let launcher_for_focus = launcher.clone();
        launcher.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = launcher_for_focus.emit("launcher:dismiss", ());
            }
        });
    }

    Ok(())
}

fn engine_config(data_dir: PathBuf) -> EngineConfig {
    let legacy = ps_core::paths::legacy_data_dir().join("file_index.db");
    EngineConfig {
        data_dir,
        legacy_db: if legacy.exists() { Some(legacy) } else { None },
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
async fn search(
    state: tauri::State<'_, AppState>,
    req: SearchRequest,
) -> Result<SearchResponse, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.search(&req))
        .await
        .map_err(|e| format!("search task failed: {e}"))
}

#[tauri::command]
async fn open_result(
    state: tauri::State<'_, AppState>,
    action: ActionSpec,
) -> Result<OpenOutcome, String> {
    let engine = state.engine.clone();
    Ok(tauri::async_runtime::spawn_blocking(move || engine.execute_action(action))
        .await
        .map_err(|e| format!("action task failed: {e}"))?)
}

/// Second step of the destructive-action confirmation handshake. The engine
/// only accepts the command it asked about within 15 seconds.
#[tauri::command]
fn confirm_native_action(
    state: tauri::State<'_, AppState>,
    command: NativeCommand,
) -> OpenOutcome {
    state.engine.confirm_destructive(command)
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, AppState>) -> Result<EngineStatus, String> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.status())
        .await
        .map_err(|e| format!("status task failed: {e}"))
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.engine.settings()
}

#[tauri::command]
fn update_settings(
    state: tauri::State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let settings = state
        .engine
        .update_settings(patch)
        .map_err(|e| e.to_string())?;
    // Settings may change roots/depth/pause; restarting is idempotent.
    if !settings.indexing.paused {
        std::sync::Arc::clone(&state.engine).spawn_background();
    }
    Ok(settings)
}

#[tauri::command]
async fn get_activity(
    state: tauri::State<'_, AppState>,
    query: Option<ActivityQuery>,
) -> Result<Vec<ActivityItem>, String> {
    let engine = state.engine.clone();
    let query = query.unwrap_or(ActivityQuery { limit: 50, since: None, kinds: Vec::new() });
    tauri::async_runtime::spawn_blocking(move || engine.activity(query))
        .await
        .map_err(|e| format!("activity task failed: {e}"))
}

#[tauri::command]
fn get_onboarding(state: tauri::State<'_, AppState>) -> OnboardingState {
    state.engine.onboarding()
}

#[tauri::command]
fn complete_onboarding(
    state: tauri::State<'_, AppState>,
    input: OnboardingInput,
) -> Result<Settings, String> {
    let settings = state
        .engine
        .complete_onboarding(input)
        .map_err(|e| e.to_string())?;
    std::sync::Arc::clone(&state.engine).spawn_background();
    Ok(settings)
}

#[tauri::command]
fn index_command(state: tauri::State<'_, AppState>, command: IndexCommand) -> Result<(), String> {
    state.engine.index_command(command).map_err(|e| e.to_string())?;
    if matches!(
        command,
        IndexCommand::Resume
            | IndexCommand::Rebuild
            | IndexCommand::RebuildContent
            | IndexCommand::RebuildOcr
    ) {
        std::sync::Arc::clone(&state.engine).spawn_background();
    }
    Ok(())
}

#[tauri::command]
fn set_launcher_geometry(
    app: AppHandle,
    window: WebviewWindow,
    height: f64,
    preview: bool,
) -> Result<(), String> {
    // `height` is the CARD content height reported by the UI; the native window
    // adds transparent padding for the shadow.
    let content_height = height.clamp(160.0, 760.0);
    let width = launcher_window_width(preview);
    let window_height = launcher_window_height(content_height);
    window
        .set_size(tauri::LogicalSize::new(width, window_height))
        .map_err(|e| e.to_string())?;
    place_launcher(&app, &window, width, window_height);
    Ok(())
}

#[tauri::command]
fn hide_launcher(app: AppHandle) {
    hide_launcher_window(&app);
}

/// The UI finished its close animation (or reduced motion is on).
#[tauri::command]
fn launcher_hidden(app: AppHandle) {
    hide_launcher_window(&app);
}

#[tauri::command]
fn show_launcher(app: AppHandle) {
    show_launcher_window(&app);
}

#[tauri::command]
fn show_main(app: AppHandle) {
    show_main_window(&app);
}

#[tauri::command]
fn quit_app(app: AppHandle, state: tauri::State<'_, AppState>) {
    let _ = state.engine.index_command(IndexCommand::Shutdown);
    if let Ok(mut slot) = state.agent.lock() {
        if let Some(client) = slot.take() {
            client.shutdown();
        }
    }
    app.exit(0);
}

/// Validate and live-apply a new global shortcut. Restores the previous one if
/// the new combination cannot be registered (e.g. another app owns it).
#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    hotkey: String,
) -> Result<Settings, String> {
    let candidate = hotkey.trim().to_string();
    if candidate.is_empty() || candidate.len() > 64 {
        return Err("Enter a shortcut such as Alt+Space.".into());
    }
    let previous = state.engine.settings().hotkey;
    let _ = app.global_shortcut().unregister_all();
    if let Err(err) = app.global_shortcut().register(candidate.as_str()) {
        let _ = app.global_shortcut().register(previous.as_str());
        return Err(format!("That shortcut is unavailable: {err}"));
    }
    let mut patch = SettingsPatch::default();
    patch.hotkey = Some(candidate);
    state.engine.update_settings(patch).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Optional Python agent sidecar (Gate 6). Never started for search.
// ---------------------------------------------------------------------------

fn probe_agent_status(state: &AppState) -> AgentStatus {
    let config = ps_agent::default_config(&state.data_dir);
    let running = state
        .agent
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().map(|client| client.is_alive()))
        .unwrap_or(false);
    match config {
        None => AgentStatus {
            state: AgentState::NotConfigured,
            runtime: None,
            model: None,
            detail: Some("Optional Python sidecar runtime is not installed.".into()),
        },
        Some(config) => AgentStatus {
            state: if running {
                AgentState::Running
            } else {
                AgentState::Stopped
            },
            runtime: Some("pydantic-ai (optional sidecar)".into()),
            model: config.model.clone(),
            detail: Some(if running {
                "Agent sidecar is running for the current workflow.".into()
            } else {
                "Starts only when you run a workflow.".into()
            }),
        },
    }
}

#[tauri::command]
fn agent_status(state: tauri::State<'_, AppState>) -> AgentStatus {
    probe_agent_status(&state)
}

#[tauri::command]
fn agent_start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    request: AgentRunRequest,
) -> Result<(), String> {
    if !state.engine.settings().agent.enabled {
        return Err("Agent workflows are disabled in Preferences.".into());
    }
    let mut slot = state.agent.lock().map_err(|e| e.to_string())?;
    let needs_spawn = slot
        .as_ref()
        .map(|client| !client.is_alive())
        .unwrap_or(true);
    if needs_spawn {
        let config = ps_agent::default_config(&state.data_dir).ok_or_else(|| {
            "The optional Python agent runtime is not installed on this machine.".to_string()
        })?;
        let handle = app.clone();
        let client = AgentClient::spawn(config, move |event: &AgentEvent| {
            let _ = handle.emit("agent:event", event);
        })
        .map_err(|e| e.to_string())?;
        *slot = Some(client);
    }
    slot.as_ref()
        .ok_or_else(|| "Agent sidecar is unavailable.".to_string())?
        .send_run(&request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn agent_cancel(state: tauri::State<'_, AppState>, run_id: Option<String>) -> Result<(), String> {
    let slot = state.agent.lock().map_err(|e| e.to_string())?;
    match slot.as_ref() {
        Some(client) => client.cancel(run_id.as_deref()).map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

#[tauri::command]
fn agent_approve(
    state: tauri::State<'_, AppState>,
    response: AgentApprovalResponse,
) -> Result<(), String> {
    let slot = state.agent.lock().map_err(|e| e.to_string())?;
    match slot.as_ref() {
        Some(client) => client.approve(&response).map_err(|e| e.to_string()),
        None => Err("No agent workflow is running.".into()),
    }
}

#[tauri::command]
fn agent_stop(state: tauri::State<'_, AppState>) {
    if let Ok(mut slot) = state.agent.lock() {
        if let Some(client) = slot.take() {
            client.shutdown();
        }
    }
}

#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({
        "name": "ProtonSearch",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": ps_ipc::PROTOCOL_VERSION,
        "dataSchema": ps_ipc::DATA_SCHEMA_VERSION,
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_launcher_window(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_launcher(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let verbose = std::env::var_os("PROTONSEARCH_VERBOSE").is_some();
            let step = |name: &str| {
                if verbose {
                    eprintln!("[setup] {name}");
                }
            };
            step("engine: opening");
            let data_dir = ps_core::paths::v2_data_dir();
            let config = engine_config(data_dir.clone());
            let engine = Arc::new(Engine::open(config).map_err(|e| format!("engine init: {e}"))?);
            app.manage(AppState { engine: engine.clone(), data_dir, agent: Mutex::new(None) });

            step("windows: creating");
            create_windows(app.handle())?;
            step("tray: building");
            build_tray(app.handle())?;

            step("hotkey: registering");
            let shortcut = app.state::<AppState>().engine.settings().hotkey;
            if let Err(err) = app.global_shortcut().register(shortcut.as_str()) {
                eprintln!("[setup] global shortcut '{shortcut}' could not be registered: {err}");
            }
            step("setup: complete");

            // Start scanners after windows exist so the launcher is usable while
            // indexing warms up. Never blocks the UI thread.
            let engine_for_bg = engine.clone();
            std::thread::spawn(move || engine_for_bg.spawn_background());

            // First run opens the desktop app; returning users get the tray only.
            if !engine.settings().onboarding_complete {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search,
            open_result,
            confirm_native_action,
            get_status,
            get_settings,
            update_settings,
            get_activity,
            get_onboarding,
            complete_onboarding,
            index_command,
            set_launcher_geometry,
            hide_launcher,
            launcher_hidden,
            show_launcher,
            show_main,
            quit_app,
            set_hotkey,
            agent_status,
            agent_start,
            agent_cancel,
            agent_approve,
            agent_stop,
            app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ProtonSearch");
}