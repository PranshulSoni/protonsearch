//! ProtonSearch v2 Tauri shell. OWNER: L0 (lead).
//!
//! Wires the Rust engine to the WebView UI, owns window/hotkey/tray lifecycle
//! and executes structured actions. No shell string interpolation and no
//! network access live here.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod monitors;

use ps_agent::{AgentClient, AgentEvent, PiAgentClient, PiAgentConfig};
use ps_ipc::agent_task::DetectedAgent;
use ps_core::{Engine, EngineConfig};
use ps_ipc::{
    ActionSpec, ActivityItem, ActivityQuery, AgentApprovalResponse, AgentRunRequest, AgentRuntime, AgentState,
    AgentStatus, EngineStatus, IndexCommand, LauncherMode, MonitorInfo, NativeCommand, OnboardingInput,
    OnboardingState,
    OpenOutcome, SearchRequest, SearchResponse, Settings, SettingsPatch,
};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const LAUNCHER_LABEL: &str = "launcher";
const ISLAND_LABEL: &str = "island";
const ISLAND_DEFAULT_WIDTH: f64 = 520.0;
const ISLAND_DEFAULT_HEIGHT: f64 = 66.0;
/// Transparent room around the island surface so the V9 layered shadow is not
/// clipped by native window edges (design G0-B gap: L0).
const ISLAND_SHADOW_PAD: f64 = 112.0;

/// Performance instrumentation is opt-in (zero cost when unset). Used by the
/// benchmark harness to split Win32-visible, first-pixel and paint timings.
static PERF_INVOKE_EPOCH_MS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
/// Cancels a pending island safety-hide when the island is re-summoned.
static ISLAND_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn perf_hooks_enabled() -> bool {
    std::env::var("PROTONSEARCH_PERF_HOOKS")
        .map(|value| value == "1")
        .unwrap_or(false)
}

fn epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn perf_log_app(app: &AppHandle, label: &str, epoch: i64) {
    if !perf_hooks_enabled() {
        return;
    }
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let path = state.data_dir.join("perf-marks.jsonl");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = writeln!(
            file,
            r#"{{"label":"{label}","epochMs":{epoch},"writtenAt":{}}}"#,
            epoch_ms()
        );
    }
}

#[tauri::command]
fn perf_mark(app: AppHandle, label: String, epoch_ms: i64) -> Result<(), String> {
    perf_log_app(&app, &label, epoch_ms);
    Ok(())
}
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
    /// Pi CLI agent (default runtime). Spawned only when a workflow starts.
    pi_agent: Mutex<Option<PiAgentClient>>,
    /// Cached agent discovery for the Workflows Agents section.
    agents: Mutex<Option<Vec<DetectedAgent>>>,
    /// Persisted runtime-agnostic agent task history/state (Agents card).
    agent_tasks: Arc<ps_agent::AgentTaskService>,
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
    // Mark the Win32-visible instant before focus/placement bookkeeping so the
    // benchmark harness can measure hotkey->first-pixel honestly.
    let invoke = PERF_INVOKE_EPOCH_MS.load(std::sync::atomic::Ordering::SeqCst);
    if perf_hooks_enabled() {
        perf_log_app(app, "launcher-window-shown", epoch_ms());
        let _ = win.emit(
            "perf:show",
            serde_json::json!({ "mode": "launcher", "invokeEpochMs": invoke }),
        );
    }
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

fn place_island(app: &AppHandle, win: &WebviewWindow, width: f64, height: f64) {
    let settings = app.state::<AppState>().engine.settings();
    let placement = settings.launcher.island.placement.clone();
    let monitors = monitors::enumerate();
    let monitor = monitors
        .iter()
        .find(|m| placement.monitor.as_deref() == Some(m.id.as_str()))
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first());
    let (mx, my, mw, mh, scale) = match monitor {
        Some(m) => (
            m.work_x as f64,
            m.work_y as f64,
            m.work_width as f64,
            m.work_height as f64,
            m.scale_factor,
        ),
        None => {
            let cursor = app.cursor_position().unwrap_or(PhysicalPosition::new(0.0, 0.0));
            (0.0, 0.0, cursor.x.max(1920.0), (cursor.y + 1080.0).max(1080.0), 1.0)
        }
    };
    let pad = ISLAND_SHADOW_PAD;
    // The surface keeps its V9 size; the native window adds transparent padding
    // around it so the shadow renders instead of being cut at the window edge.
    // Clamp the surface into the monitor so an oversized card cannot escape.
    let max_surface_w = (mw / scale - 16.0).max(240.0);
    let max_surface_h = (mh / scale - 16.0).max(40.0);
    let surface_w = width.clamp(240.0, max_surface_w);
    let surface_h = height.clamp(40.0, max_surface_h);
    let pw = surface_w * scale;
    let ph = surface_h * scale;
    let (fx, fy) = match placement.anchor {
        ps_ipc::IslandAnchor::TopLeft => (0.0, 0.0),
        ps_ipc::IslandAnchor::TopCenter => (0.5, 0.0),
        ps_ipc::IslandAnchor::TopRight => (1.0, 0.0),
        ps_ipc::IslandAnchor::CenterLeft => (0.0, 0.5),
        ps_ipc::IslandAnchor::Center => (0.5, 0.5),
        ps_ipc::IslandAnchor::CenterRight => (1.0, 0.5),
        ps_ipc::IslandAnchor::BottomLeft => (0.0, 1.0),
        ps_ipc::IslandAnchor::BottomCenter => (0.5, 1.0),
        ps_ipc::IslandAnchor::BottomRight => (1.0, 1.0),
    };
    let off_x = placement.offset_x as f64 * scale;
    let off_y = placement.offset_y as f64 * scale;
    let mut surface_x = mx + ((mw - pw) * fx) + off_x;
    let mut surface_y = my + ((mh - ph) * fy) + off_y;
    // Always keep the surface reachable: with keepInWorkArea the taskbar is
    // excluded; without it the monitor bounds still apply (never unreachable).
    let margin = if placement.keep_in_work_area { 8.0 } else { 0.0 };
    let min_x = mx + margin;
    let max_x = (mx + mw - pw - margin).max(min_x);
    let min_y = my + margin;
    let max_y = (my + mh - ph - margin).max(min_y);
    surface_x = surface_x.clamp(min_x, max_x);
    surface_y = surface_y.clamp(min_y, max_y);
    let pad_px = pad * scale;
    let window_w = surface_w + pad * 2.0;
    let window_h = surface_h + pad * 2.0;
    let _ = win.set_size(LogicalSize::new(window_w, window_h));
    let _ = win.set_position(PhysicalPosition::new(
        (surface_x - pad_px) as i32,
        (surface_y - pad_px) as i32,
    ));
}

fn hide_island_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(ISLAND_LABEL) {
        let _ = win.hide();
        let _ = win.emit("island:hidden", ());
    }
}

fn show_island_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window(ISLAND_LABEL) else {
        return;
    };
    ISLAND_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    hide_launcher_window(app);
    let scale = win.scale_factor().unwrap_or(1.0);
    let window_size = win
        .inner_size()
        .map(|s| (s.width as f64 / scale, s.height as f64 / scale))
        .unwrap_or((
            ISLAND_DEFAULT_WIDTH + ISLAND_SHADOW_PAD * 2.0,
            ISLAND_DEFAULT_HEIGHT + ISLAND_SHADOW_PAD * 2.0,
        ));
    place_island(
        app,
        &win,
        (window_size.0 - ISLAND_SHADOW_PAD * 2.0).max(240.0),
        (window_size.1 - ISLAND_SHADOW_PAD * 2.0).max(40.0),
    );
    let _ = win.show();
    // Mark the Win32-visible instant before focus bookkeeping.
    let invoke = PERF_INVOKE_EPOCH_MS.load(std::sync::atomic::Ordering::SeqCst);
    if perf_hooks_enabled() {
        perf_log_app(app, "island-window-shown", epoch_ms());
        let _ = win.emit(
            "perf:show",
            serde_json::json!({ "mode": "island", "invokeEpochMs": invoke }),
        );
    }
    let _ = win.set_focus();
    let _ = win.emit("island:shown", ());
}

fn toggle_island(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(ISLAND_LABEL) {
        let visible = win.is_visible().unwrap_or(false);
        let focused = win.is_focused().unwrap_or(false);
        if visible && focused {
            // Frontend plays the exit animation then calls hide_island.
            let _ = win.emit("island:dismiss", ());
            let generation =
                ISLAND_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let handle = win.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(320));
                if ISLAND_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == generation {
                    let _ = handle.hide();
                    let _ = handle.emit("island:hidden", ());
                }
            });
        } else {
            show_island_window(app);
        }
    }
}

fn toggle_primary_launcher(app: &AppHandle) {
    PERF_INVOKE_EPOCH_MS.store(epoch_ms(), std::sync::atomic::Ordering::SeqCst);
    let settings = app.state::<AppState>().engine.settings();
    if settings.launcher.primary_mode == LauncherMode::DynamicIsland
        && settings.launcher.island.enabled
    {
        toggle_island(app);
    } else {
        toggle_launcher(app);
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

    if app.get_webview_window(ISLAND_LABEL).is_none() {
        let mut island_builder = WebviewWindowBuilder::new(
            app,
            ISLAND_LABEL,
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("ProtonSearch Island")
        .inner_size(
            ISLAND_DEFAULT_WIDTH + ISLAND_SHADOW_PAD * 2.0,
            ISLAND_DEFAULT_HEIGHT + ISLAND_SHADOW_PAD * 2.0,
        )
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .focused(false);
        if let Some(args) = &cdp {
            island_builder = island_builder.additional_browser_args(args);
        }
        if let Some(dir) = &webview_dir {
            island_builder = island_builder.data_directory(dir.clone());
        }
        let island = island_builder.build()?;
        let _ = island.set_icon(window_icon()?);
        place_island(app, &island, ISLAND_DEFAULT_WIDTH, ISLAND_DEFAULT_HEIGHT);
        let app_for_focus = app.clone();
        let island_for_focus = island.clone();
        island.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let hide = app_for_focus
                    .try_state::<AppState>()
                    .map(|state| state.engine.settings().launcher.island.hide_on_focus_loss)
                    .unwrap_or(true);
                if hide {
                    let _ = island_for_focus.hide();
                    let _ = island_for_focus.emit("island:hidden", ());
                }
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

fn validate_settings(settings: &Settings) -> Result<(), String> {
    use std::collections::HashSet;
    let island = &settings.launcher.island;
    if island.cards.len() != 8 {
        return Err("The island must keep exactly the 8 V9 cards.".into());
    }
    let mut seen = HashSet::new();
    for card in &island.cards {
        if !seen.insert(card.id) {
            return Err("Duplicate island card in the card order.".into());
        }
    }
    if !island
        .cards
        .iter()
        .any(|card| card.id == island.default_card && card.enabled)
    {
        return Err("The default island card must be enabled.".into());
    }
    if island.placement.offset_x.abs() > 2000 || island.placement.offset_y.abs() > 2000 {
        return Err("Island offsets must stay within +/-2000 px.".into());
    }
    let clipboard = &settings.clipboard;
    if clipboard.retention_days > 3650 {
        return Err("Clipboard retention must be 0-3650 days.".into());
    }
    if clipboard.max_items < 10 || clipboard.max_items > 100_000 {
        return Err("Clipboard history must keep between 10 and 100000 items.".into());
    }
    Ok(())
}

#[tauri::command]
fn update_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let previous = state.engine.settings();
    let mut candidate = previous.clone();
    candidate.apply(&patch);
    validate_settings(&candidate)?;
    let settings = state
        .engine
        .update_settings(patch)
        .map_err(|e| e.to_string())?;
    // The Pi launcher path may have changed; drop the cached probe and agent list.
    ps_agent::clear_probe_cache();
    if let Ok(mut cache) = state.agents.lock() {
        *cache = None;
    }
    // Settings may change roots/depth/pause; restarting is idempotent.
    if !settings.indexing.paused {
        std::sync::Arc::clone(&state.engine).spawn_background();
    }
    // Launcher mode / island placement changes take effect without a restart.
    if previous.launcher != settings.launcher {
        if let Some(win) = app.get_webview_window(ISLAND_LABEL) {
            let scale = win.scale_factor().unwrap_or(1.0);
            let size = win
                .inner_size()
                .map(|s| (s.width as f64 / scale, s.height as f64 / scale))
                .unwrap_or((
                    ISLAND_DEFAULT_WIDTH + ISLAND_SHADOW_PAD * 2.0,
                    ISLAND_DEFAULT_HEIGHT + ISLAND_SHADOW_PAD * 2.0,
                ));
            place_island(
                &app,
                &win,
                (size.0 - ISLAND_SHADOW_PAD * 2.0).max(240.0),
                (size.1 - ISLAND_SHADOW_PAD * 2.0).max(40.0),
            );
        }
        if settings.launcher.primary_mode != previous.launcher.primary_mode {
            hide_launcher_window(&app);
            hide_island_window(&app);
        }
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
    let _ = state.agent_tasks.stop_all("app quit");
    if let Ok(mut slot) = state.pi_agent.lock() {
        if let Some(client) = slot.take() {
            client.shutdown();
        }
    }
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
    let settings = state.engine.settings();
    if settings.agent.runtime == AgentRuntime::Pi {
        let probed = ps_agent::probe_pi_cached(
            settings.agent.pi_path.as_deref().map(std::path::Path::new),
        );
        let running = state
            .pi_agent
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|client| client.is_running()))
            .unwrap_or(false);
        return match probed {
            Some(info) => AgentStatus {
                state: if running {
                    AgentState::Running
                } else {
                    AgentState::Stopped
                },
                runtime: Some(format!("Pi {}", info.version)),
                model: settings.agent.model.clone(),
                detail: Some(format!(
                    "Read-only tools. Launcher: {} ({}).",
                    info.path.display(),
                    match info.source {
                        ps_agent::PiSource::Explicit => "explicit",
                        ps_agent::PiSource::Env => "environment",
                        ps_agent::PiSource::Managed => "managed install",
                        ps_agent::PiSource::Path => "PATH (least trusted)",
                    }
                )),
            },
            None => AgentStatus {
                state: AgentState::NotConfigured,
                runtime: Some("Pi".into()),
                model: None,
                detail: Some(
                    "The Pi CLI agent was not found. Install it from pi.dev or set its path in Preferences."
                        .into(),
                ),
            },
        };
    }
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

fn locate_agent_on_path(exe: &str) -> Option<std::path::PathBuf> {
    let mut cmd = std::process::Command::new("where");
    cmd.arg(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_file())
        .max_by_key(|path| match path.extension().and_then(|value| value.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("exe") => 3,
            Some(ext) if ext.eq_ignore_ascii_case("cmd") => 2,
            Some(ext) if ext.eq_ignore_ascii_case("bat") => 1,
            _ => 0,
        })
}

/// Run the agent CLI version command with a hard 2.5 s bound and a 64 KiB cap.
fn bounded_agent_version(path: &std::path::Path) -> Option<String> {
    let text = path.to_string_lossy();
    if text
        .chars()
        .any(|c| matches!(c, '&' | '|' | '<' | '>' | '^' | '"' | '%' | '\r' | '\n'))
    {
        return None;
    }
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut cmd = if matches!(ext.as_str(), "cmd" | "bat") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/d", "/s", "/c", &format!("\"{text}\" --version")]);
        c
    } else {
        let mut c = std::process::Command::new(path);
        c.arg("--version");
        c
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().ok()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2500);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            Err(_) => {
                let _ = child.kill();
                return None;
            }
        }
    }
    use std::io::Read;
    let mut buffer = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.take(64 * 1024).read_to_string(&mut buffer);
    }
    if buffer.trim().is_empty() {
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.take(64 * 1024).read_to_string(&mut buffer);
        }
    }
    let first = buffer
        .lines()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())?;
    Some(first.chars().take(80).collect())
}

/// Detected local agent CLIs (Pi first, then other installed coding agents).
#[tauri::command]
fn detect_agents(state: tauri::State<'_, AppState>) -> Vec<DetectedAgent> {
    if let Ok(cache) = state.agents.lock() {
        if let Some(agents) = cache.as_ref() {
            return agents.clone();
        }
    }
    let settings = state.engine.settings();
    let mut agents = Vec::new();
    if let Some(info) =
        ps_agent::probe_pi_cached(settings.agent.pi_path.as_deref().map(std::path::Path::new))
    {
        agents.push(DetectedAgent {
            id: "pi".into(),
            name: "Pi".into(),
            kind: "agent".into(),
            version: Some(info.version.clone()),
            path: Some(info.path.to_string_lossy().into_owned()),
            available: true,
            detail: Some("Default agent runtime. Read-only in this alpha.".into()),
        });
    }
    for (exe, name, detail) in [
        ("codex", "Codex CLI", "OpenAI Codex coding agent"),
        ("opencode", "OpenCode", "OpenCode coding agent"),
        ("cline", "Cline", "Cline coding agent"),
        ("agy", "Antigravity", "Antigravity agent"),
    ] {
        if let Some(path) = locate_agent_on_path(exe) {
            let version = bounded_agent_version(&path);
            agents.push(DetectedAgent {
                id: exe.into(),
                name: name.into(),
                kind: "agent".into(),
                version,
                path: Some(path.to_string_lossy().into_owned()),
                available: true,
                detail: Some(detail.into()),
            });
        }
    }
    if let Ok(mut cache) = state.agents.lock() {
        *cache = Some(agents.clone());
    }
    agents
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
    let settings = state.engine.settings();
    if !settings.agent.enabled {
        return Err("Agent workflows are disabled in Preferences.".into());
    }
    if settings.agent.runtime == AgentRuntime::Pi {
        let mut config =
            PiAgentConfig::from_settings(&settings.agent, &state.data_dir).map_err(|_| {
            "The Pi CLI agent is not available. Install it from pi.dev or set its path in Preferences."
                .to_string()
            })?;
        // ALPHA SAFETY GATE (security re-review RR-02): Pi runs read-only until
        // a server-side per-run consent token exists; persisted settings can
        // never silently enable edits/shell.
        config.allow_writes = false;
        let task = state
            .agent_tasks
            .begin_pi(
                &request,
                &config.pi,
                settings.agent.model.clone(),
                config.allow_writes,
                None,
            )
            .map_err(|_| "The workflow could not be recorded.".to_string())?;
        // The canonical task id becomes the runtime run id so events map 1:1.
        let mut run_request = request.clone();
        run_request.run_id = task.id.clone();
        let mut slot = state.pi_agent.lock().map_err(|e| e.to_string())?;
        if slot.is_none() {
            let handle = app.clone();
            let tasks = Arc::clone(&state.agent_tasks);
            let task_id = task.id.clone();
            let client = PiAgentClient::spawn(config, move |event: &AgentEvent| {
                let _ = tasks.apply_event(&task_id, event);
                let _ = handle.emit("agent:event", event);
            })
            .map_err(|_| "The Pi CLI agent could not be started.".to_string())?;
            *slot = Some(client);
        }
        return slot
            .as_ref()
            .ok_or_else(|| "The Pi CLI agent is unavailable.".to_string())?
            .send_run(&run_request)
            .map_err(|err| match err {
                ps_agent::AgentError::Busy => "A workflow is already running.".to_string(),
                _ => "The Pi workflow could not be started.".to_string(),
            });
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
    if state.engine.settings().agent.runtime == AgentRuntime::Pi {
        for task in state.agent_tasks.tasks(10) {
            if !task.state.is_terminal() {
                let _ = state.agent_tasks.cancel(&task.id, "user cancelled");
            }
        }
        let slot = state.pi_agent.lock().map_err(|e| e.to_string())?;
        return match slot.as_ref() {
            Some(client) => client
                .cancel()
                .map_err(|_| "The Pi workflow could not be cancelled.".to_string()),
            None => Ok(()),
        };
    }
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
    if state.engine.settings().agent.runtime == AgentRuntime::Pi {
        return Err(
            "Pi runs do not ask for per-tool approval; file edits and shell access are opted in before the run."
                .into(),
        );
    }
    let slot = state.agent.lock().map_err(|e| e.to_string())?;
    match slot.as_ref() {
        Some(client) => client.approve(&response).map_err(|e| e.to_string()),
        None => Err("No agent workflow is running.".into()),
    }
}

#[tauri::command]
fn agent_stop(state: tauri::State<'_, AppState>) {
    if let Ok(mut slot) = state.pi_agent.lock() {
        if let Some(client) = slot.take() {
            client.shutdown();
        }
    }
    if let Ok(mut slot) = state.agent.lock() {
        if let Some(client) = slot.take() {
            client.shutdown();
        }
    }
}

#[tauri::command]
fn get_preview(
    state: tauri::State<'_, AppState>,
    req: ps_ipc::PreviewRequest,
) -> ps_ipc::PreviewPayload {
    state.engine.preview(&req)
}

#[tauri::command]
fn agent_tasks(
    state: tauri::State<'_, AppState>,
    limit: Option<usize>,
) -> Vec<ps_agent::AgentTask> {
    state.agent_tasks.tasks(limit.unwrap_or(50).min(200))
}

#[tauri::command]
fn agent_task_tree(
    state: tauri::State<'_, AppState>,
    limit: Option<usize>,
) -> Vec<ps_agent::AgentTaskNode> {
    state.agent_tasks.tree(limit.unwrap_or(50).min(200))
}

#[tauri::command]
fn agent_task(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Option<ps_agent::AgentTask> {
    state.agent_tasks.task(&task_id)
}

#[tauri::command]
fn agent_permissions(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Option<ps_agent::AgentPermissionState> {
    state.agent_tasks.permissions(&task_id)
}

#[tauri::command]
fn agent_stop_all(state: tauri::State<'_, AppState>) -> Vec<ps_agent::AgentTask> {
    let tasks = state.agent_tasks.stop_all("user stopped all");
    if let Ok(mut slot) = state.pi_agent.lock() {
        if let Some(client) = slot.take() {
            client.shutdown();
        }
    }
    tasks
}

#[tauri::command]
fn agent_pause(state: tauri::State<'_, AppState>, task_id: String) -> Result<(), String> {
    state
        .agent_tasks
        .pause(&task_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn agent_resume(state: tauri::State<'_, AppState>, task_id: String) -> Result<(), String> {
    state
        .agent_tasks
        .resume(&task_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_monitors() -> Vec<MonitorInfo> {
    monitors::enumerate()
}

#[tauri::command]
fn show_island(app: AppHandle) {
    show_island_window(&app);
}

#[tauri::command]
fn hide_island(app: AppHandle) {
    hide_island_window(&app);
}

#[tauri::command]
fn set_island_geometry(
    app: AppHandle,
    window: WebviewWindow,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let width = width.clamp(240.0, 900.0);
    let height = height.clamp(40.0, 760.0);
    place_island(&app, &window, width, height);
    Ok(())
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
                        toggle_primary_launcher(app);
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
            let agent_task_app = app.handle().clone();
            let mut agent_redactions: Vec<String> = Vec::new();
            for key in [
                "OPENAI_API_KEY",
                "ANTHROPIC_API_KEY",
                "GEMINI_API_KEY",
                "GOOGLE_API_KEY",
                "OPENROUTER_API_KEY",
                "DEEPSEEK_API_KEY",
                "AZURE_OPENAI_API_KEY",
                "XAI_API_KEY",
                "GROQ_API_KEY",
            ] {
                if let Ok(value) = std::env::var(key) {
                    if value.len() >= 8 {
                        agent_redactions.push(value);
                    }
                }
            }
            let agent_tasks = Arc::new(
                ps_agent::AgentTaskService::open(&data_dir, agent_redactions)
                    .map_err(|e| format!("agent task store: {e}"))?
                    .with_listener(Arc::new(move |task: &ps_agent::AgentTask| {
                        let _ = agent_task_app.emit("agent:task", task);
                    })),
            );
            app.manage(AppState {
                engine: engine.clone(),
                data_dir,
                agent: Mutex::new(None),
                pi_agent: Mutex::new(None),
                agents: Mutex::new(None),
                agent_tasks,
            });

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
            list_monitors,
            show_island,
            hide_island,
            set_island_geometry,
            hide_launcher,
            launcher_hidden,
            show_launcher,
            show_main,
            quit_app,
            set_hotkey,
            agent_status,
            detect_agents,
            agent_tasks,
            agent_task_tree,
            agent_task,
            agent_permissions,
            agent_stop_all,
            agent_pause,
            agent_resume,
            agent_start,
            agent_cancel,
            agent_approve,
            agent_stop,
            perf_mark,
            get_preview,
            app_info,
        ])
        .build(tauri::generate_context!())
        .expect("error while building ProtonSearch")
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Ok(mut slot) = state.pi_agent.lock() {
                        if let Some(client) = slot.take() {
                            client.shutdown();
                        }
                    }
                    if let Ok(mut slot) = state.agent.lock() {
                        if let Some(client) = slot.take() {
                            client.shutdown();
                        }
                    }
                }
            }
        });
}