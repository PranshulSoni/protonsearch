//! Out-of-process native dock renderer; the launcher remains the recovery controller.
pub mod appbar;
pub mod platform;
pub mod supervisor;
use crate::ui::{DockItem, DockPanelWindow, DockWindow, PanelItem};
use anyhow::{bail, Result};
use proton_core::dock_model::{
    AppPin, DockConfig, FocusTimer, LayoutStore, WidgetInstance, WidgetKind,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{ComponentHandle, Image, ModelRc, SharedPixelBuffer, Timer, TimerMode, VecModel};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        System::{Com::*, SystemInformation::*, Threading::*},
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};

#[derive(Default)]
struct Snapshot {
    windows: Vec<platform::WindowEntry>,
    apps: Vec<platform::InstalledApp>,
    icons: HashMap<String, platform::IconPixels>,
    audio: Option<platform::AudioSnapshot>,
    power: Option<platform::PowerSnapshot>,
    network: Option<platform::NetworkSnapshot>,
}
struct State {
    config: DockConfig,
    store: LayoutStore,
    read_only: bool,
    snapshot: Snapshot,
    focus: FocusTimer,
    panel: String,
    query: String,
    status: String,
    controller: isize,
}
impl State {
    fn save(&mut self) {
        if self.read_only {
            self.status = "Layout is read-only because its saved file could not be loaded.".into();
            return;
        }
        if let Err(error) = self.store.save(&self.config) {
            self.status = error;
        }
    }
}
fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    std::rc::Rc::new(VecModel::from(items)).into()
}
fn image(pixels: &platform::IconPixels) -> Image {
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
        &pixels.rgba,
        pixels.width,
        pixels.height,
    ))
}
fn asset(name: &str) -> Image {
    Image::load_from_path(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("ui/icons/dock")
            .join(format!("{name}.svg")),
    )
    .unwrap_or_default()
}
fn native(window: &slint::Window) -> Option<HWND> {
    let provider = window.window_handle();
    match provider.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as _)),
        _ => None,
    }
}
fn controller_live(hwnd: isize, expected: u32) -> bool {
    // SAFETY: a borrowed HWND is only queried; no cross-process pointer is dereferenced.
    unsafe {
        let h = HWND(hwnd as _);
        let mut pid = 0;
        GetWindowThreadProcessId(h, Some(&mut pid));
        IsWindow(h).as_bool()
            && pid == expected
            && pid != 0
            && crate::get_process_path(pid).is_some_and(|p| {
                std::env::current_exe().is_ok_and(|e| e.to_string_lossy().eq_ignore_ascii_case(&p))
            })
    }
}
fn host(state: &State, message: u32) {
    // SAFETY: controller identity is validated at startup and periodically during the event loop.
    unsafe {
        let _ = PostMessageW(HWND(state.controller as _), message, WPARAM(0), LPARAM(0));
    }
}
fn clock() -> String {
    unsafe {
        let t = GetLocalTime();
        format!("{:02}:{:02}", t.wHour, t.wMinute)
    }
}
fn timer_text(state: &State) -> String {
    let s = state.focus.seconds(Instant::now());
    format!("{:02}:{:02}", s / 60, s % 60)
}
fn kind_icon(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Start | WidgetKind::Applications => "apps",
        WidgetKind::Agents => "agent",
        WidgetKind::Clock => "clock",
        WidgetKind::Weather => "weather",
        WidgetKind::Music => "music",
        WidgetKind::Focus => "focus",
        WidgetKind::Audio => "volume",
        WidgetKind::Power => "power",
        WidgetKind::Network => "network",
        WidgetKind::Files => "file",
        WidgetKind::SystemTray => "settings",
        _ => "search",
    }
}
fn refresh(state: &State, dock: &DockWindow, panel: &DockPanelWindow) {
    let profile = state.config.active();
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for widget in profile.widgets.iter().filter(|w| w.enabled) {
        if widget.kind == WidgetKind::Applications {
            for pin in &profile.pins {
                let running = state.snapshot.windows.iter().any(|w| {
                    w.identity == pin.id
                        || w.executable
                            .as_ref()
                            .is_some_and(|p| p.eq_ignore_ascii_case(&pin.command))
                });
                items.push(DockItem {
                    id: format!("app:{}", pin.id).into(),
                    kind: "app".into(),
                    title: pin.title.clone().into(),
                    icon: state
                        .snapshot
                        .icons
                        .get(&pin.id)
                        .map(image)
                        .unwrap_or_else(|| asset("apps")),
                    width: 64.,
                    running,
                    ..Default::default()
                });
                seen.insert(pin.id.clone());
                seen.insert(pin.command.replace('/', "\\").to_lowercase());
            }
            for window in &state.snapshot.windows {
                if !seen.insert(window.identity.clone()) {
                    continue;
                }
                items.push(DockItem {
                    id: format!("app:{}", window.identity).into(),
                    kind: "app".into(),
                    title: window.title.clone().into(),
                    icon: state
                        .snapshot
                        .icons
                        .get(&window.identity)
                        .map(image)
                        .unwrap_or_else(|| asset("apps")),
                    width: 64.,
                    running: true,
                    ..Default::default()
                });
            }
            continue;
        }
        let (value, subtitle) = match widget.kind {
            WidgetKind::Clock => (clock(), "Local time".into()),
            WidgetKind::Focus => (
                timer_text(state),
                if state.focus.running() {
                    "Focusing"
                } else {
                    "Focus timer"
                }
                .into(),
            ),
            WidgetKind::Audio => state
                .snapshot
                .audio
                .map(|a| {
                    (
                        if a.muted {
                            "Muted".into()
                        } else {
                            format!("{}%", a.volume)
                        },
                        "Sound".into(),
                    )
                })
                .unwrap_or(("Unavailable".into(), "Sound".into())),
            WidgetKind::Power => state
                .snapshot
                .power
                .map(|p| {
                    (
                        p.percentage
                            .map(|v| format!("{v}%"))
                            .unwrap_or("AC power".into()),
                        if p.charging { "Charging" } else { "Battery" }.into(),
                    )
                })
                .unwrap_or(("Unavailable".into(), "Power".into())),
            WidgetKind::Network => state
                .snapshot
                .network
                .as_ref()
                .map(|n| {
                    (
                        n.name.clone(),
                        if n.internet {
                            "Internet"
                        } else {
                            "No internet"
                        }
                        .into(),
                    )
                })
                .unwrap_or(("Unavailable".into(), "Network".into())),
            WidgetKind::Weather => ("Weather".into(), "Set location".into()),
            WidgetKind::Music => ("Media controls".into(), "System playback".into()),
            WidgetKind::CustomMetric => (widget.value.clone(), widget.detail.clone()),
            _ => (widget.kind.title().into(), String::new()),
        };
        items.push(DockItem {
            id: widget.id.clone().into(),
            kind: widget.kind.key().into(),
            title: if widget.label.is_empty() {
                widget.kind.title().into()
            } else {
                widget.label.clone().into()
            },
            subtitle: subtitle.into(),
            value: value.into(),
            icon: asset(kind_icon(widget.kind)),
            width: widget.width,
            ..Default::default()
        });
    }
    let width = (items.iter().map(|i| i.width + 5.).sum::<f32>() + 90.).max(250.);
    if let Some(m) = platform::primary_monitor() {
        dock.window().set_size(slint::LogicalSize::new(
            width.min((m.right - m.left) as f32 * 96. / m.dpi as f32 - 24.),
            profile.height,
        ));
    }
    dock.set_items(model(items));
    dock.set_profile(profile.name.clone().into());
    dock.set_dock_height(profile.height);
    dock.set_status(state.status.clone().into());
    panel.set_panel_kind(state.panel.clone().into());
    panel.set_panel_title(
        match state.panel.as_str() {
            "apps" => "Applications",
            "windows" => "Open windows",
            "customize" => "Customize dock",
            "focus" => "Focus",
            "audio" => "Sound",
            "agents" => "Agents",
            "music" => "Media controls",
            "weather" => "Weather",
            "network" => "Network",
            "power" => "Power",
            _ => "Proton dock",
        }
        .into(),
    );
    panel.set_subtitle(profile.name.clone().into());
    panel.set_status(state.status.clone().into());
    panel.set_timer_value(timer_text(state).into());
    panel.set_timer_running(state.focus.running());
    if let Some(audio) = state.snapshot.audio {
        panel.set_volume(audio.volume as f32);
        panel.set_muted(audio.muted);
    }
    let query = state.query.to_lowercase();
    let mut entries = Vec::new();
    let mut body = String::new();
    match state.panel.as_str() {
        "apps"=>for app in state.snapshot.apps.iter().filter(|a|a.name.to_lowercase().contains(&query)).take(150){entries.push(PanelItem{id:app.id.clone().into(),title:app.name.clone().into(),subtitle:app.command.clone().into(),icon:app.icon.as_ref().map(image).unwrap_or_else(||asset("apps")),selected:profile.pins.iter().any(|p|p.id==app.id),..Default::default()});},
        "windows"=>for w in state.snapshot.windows.iter().filter(|w|query.is_empty()||w.title.to_lowercase().contains(&query)||w.identity==query){entries.push(PanelItem{id:format!("window:{}",w.hwnd).into(),title:w.title.clone().into(),subtitle:w.executable.clone().unwrap_or_default().into(),icon:state.snapshot.icons.get(&w.identity).map(image).unwrap_or_else(||asset("apps")),..Default::default()});},
        "customize"=>for kind in WidgetKind::ALL { let w=profile.widgets.iter().find(|w|w.kind==kind);entries.push(PanelItem{id:kind.key().into(),title:kind.title().into(),subtitle:w.map(|w|format!("{} px",w.width)).unwrap_or("Add to this profile".into()).into(),selected:w.is_some_and(|w|w.enabled),icon:asset(kind_icon(kind)),..Default::default()});},
        "weather"=>body="Weather forecasts are not connected yet. Use Windows Weather from Settings, or add a location when a weather provider is available.".into(),
        "music"=>body="Controls the current Windows media session. Track information is not available from this provider.".into(),
        "agents"=>body="Open your existing ProtonSearch Agents page to manage and launch configured agents.".into(),
        "network"=>body=state.snapshot.network.as_ref().map(|n|format!("{}\n{} · {}",n.name,if n.wireless{"Wi-Fi"}else{"Network"},if n.internet{"Internet access"}else if n.connected{"Local connectivity"}else{"Disconnected"})).unwrap_or("Network information unavailable".into()),
        "power"=>body=state.snapshot.power.map(|p|format!("{}\n{}",p.percentage.map(|v|format!("Battery: {v}%")).unwrap_or("No battery reported".into()),if p.charging{"Charging"}else if p.ac_power==Some(true){"Connected to AC power"}else{"On battery"})).unwrap_or("Power information unavailable".into()),
        _=>{}
    }
    panel.set_body(body.into());
    panel.set_entries(model(entries));
}
fn launch(command: &str) {
    crate::launcher::launch(command);
}
fn action(state: &mut State, command: &str, panel: &DockPanelWindow) {
    if command == "hide" || command == "back" {
        let _ = panel.hide();
        return;
    }
    if let Some(query) = command.strip_prefix("query:") {
        state.query = query.into();
        return;
    }
    if let Some(v) = command.strip_prefix("volume:") {
        if let Ok(v) = v.parse::<f32>() {
            if let Err(e) = platform::set_volume(v.clamp(0., 100.) as u8) {
                state.status = e.to_string();
            } else if let Some(a) = state.snapshot.audio.as_mut() {
                a.volume = v as u8;
            }
        }
        return;
    }
    if command == "mute" {
        let value = !state.snapshot.audio.is_some_and(|a| a.muted);
        if let Err(e) = platform::set_mute(value) {
            state.status = e.to_string();
        } else if let Some(a) = state.snapshot.audio.as_mut() {
            a.muted = value;
        }
        return;
    }
    if let Some(id) = command.strip_prefix("pin:") {
        if let Some(app) = state.snapshot.apps.iter().find(|a| a.id == id) {
            let pin = AppPin {
                id: app.id.clone(),
                title: app.name.clone(),
                command: app.command.clone(),
            };
            let pins = &mut state.config.active_mut().pins;
            if pins.iter().any(|p| p.id == id) {
                pins.retain(|p| p.id != id);
            } else {
                pins.push(pin);
            }
            state.save();
        }
        return;
    }
    if let Some(id) = command.strip_prefix("close:") {
        if let Some(w) = state
            .snapshot
            .windows
            .iter()
            .find(|w| format!("window:{}", w.hwnd) == id)
            .cloned()
        {
            if let Err(e) = platform::close_window(&w) {
                state.status = e.to_string();
            }
        }
        return;
    }
    if let Some(id) = command.strip_prefix("window:") {
        if let Some(w) = state
            .snapshot
            .windows
            .iter()
            .find(|w| w.hwnd.to_string() == id)
            .cloned()
        {
            let _ = panel.hide();
            if let Err(e) = platform::activate_window(&w) {
                state.status = e.to_string();
            }
        }
        return;
    }
    if let Some(name) = command.strip_prefix("profile:") {
        state.config.select_profile(&name.to_lowercase());
        state.save();
        return;
    }
    if let Some(spec) = command.strip_prefix("customize:") {
        if let Some((id, op)) = spec.rsplit_once(':') {
            let p = state.config.active_mut();
            match op {
                "toggle" => {
                    if let Some(k) = WidgetKind::ALL.into_iter().find(|k| k.key() == id) {
                        p.ensure_kind(k);
                    }
                }
                "smaller" => {
                    p.resize(id, -20.);
                }
                "larger" => {
                    p.resize(id, 20.);
                }
                "left" => {
                    p.move_widget(id, -1);
                }
                "right" => {
                    p.move_widget(id, 1);
                }
                _ => {}
            }
            state.save();
        }
        return;
    }
    if command == "add-custom" {
        let parts: Vec<_> = state.query.split('|').map(str::trim).collect();
        if parts.len() >= 2 {
            let mut w = WidgetInstance::new(WidgetKind::CustomMetric);
            w.id = format!(
                "metric-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
            w.label = parts[0].into();
            w.value = parts[1].into();
            w.detail = parts.get(2).unwrap_or(&"").to_string();
            state.config.active_mut().widgets.push(w);
            state.save();
        } else {
            state.status = "Enter name | value | detail above, then Add widget.".into();
        }
        return;
    }
    if command == "start" {
        state.focus.start(Instant::now());
        return;
    }
    if command == "pause" {
        state.focus.pause(Instant::now());
        return;
    }
    if command == "reset" {
        state.focus.reset();
        return;
    }
    if command == "settings" {
        if state.panel == "agents" {
            host(state, WM_APP + 101);
        } else {
            launch(match state.panel.as_str() {
                "audio" => "ms-settings:sound",
                "network" => "ms-settings:network-status",
                "power" => "ms-settings:powersleep",
                "weather" => "msnweather:",
                _ => "ms-settings:",
            });
        }
        return;
    }
    if matches!(command, "play" | "next" | "previous") {
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        crate::launcher::send_media_key(match command {
            "next" => VK_MEDIA_NEXT_TRACK,
            "previous" => VK_MEDIA_PREV_TRACK,
            _ => VK_MEDIA_PLAY_PAUSE,
        });
        return;
    }
    if command == "refresh" || command == "submit" {
        return;
    }
    if let Some(id) = command.strip_prefix("context:") {
        state.panel = "apps".into();
        state.query = id
            .strip_prefix("app:")
            .and_then(|id| {
                state
                    .config
                    .active()
                    .pins
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.title.clone())
            })
            .unwrap_or_default();
        panel.set_query(state.query.clone().into());
        let _ = panel.show();
        return;
    }
    if let Some(id) = command.strip_prefix("dock:") {
        if let Some(identity) = id.strip_prefix("app:") {
            let wins: Vec<_> = state
                .snapshot
                .windows
                .iter()
                .filter(|w| w.identity == identity)
                .cloned()
                .collect();
            if wins.len() == 1 {
                if let Err(e) = platform::activate_window(&wins[0]) {
                    state.status = e.to_string();
                }
                return;
            }
            if wins.is_empty() {
                if let Some(pin) = state.config.active().pins.iter().find(|p| p.id == identity) {
                    launch(&pin.command);
                    return;
                }
            }
            state.panel = "windows".into();
            state.query = identity.into();
        } else {
            let kind = state
                .config
                .active()
                .widgets
                .iter()
                .find(|w| w.id == id)
                .map(|w| w.kind);
            match kind {
                Some(WidgetKind::Search) => {
                    host(state, WM_APP + 100);
                    return;
                }
                Some(WidgetKind::Files) => {
                    launch("explorer.exe");
                    return;
                }
                Some(WidgetKind::Agents) => state.panel = "agents".into(),
                Some(WidgetKind::Start | WidgetKind::Applications) => state.panel = "apps".into(),
                Some(WidgetKind::SystemTray) => {
                    launch("ms-settings:taskbar");
                    return;
                }
                Some(k) => state.panel = k.key().into(),
                None => state.panel = "customize".into(),
            }
            state.query.clear();
        }
        panel.set_query(state.query.clone().into());
        let _ = panel.show();
        return;
    }
    if command == "customize" {
        state.panel = "customize".into();
        state.query.clear();
        panel.set_query("".into());
        let _ = panel.show();
        return;
    }
    if let Some(app) = state.snapshot.apps.iter().find(|a| a.id == command) {
        launch(&app.command);
        let _ = panel.hide();
    }
}

pub fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let arg = |key: &str| args.windows(2).find(|a| a[0] == key).map(|a| a[1].clone());
    let pid = arg("--controller-pid")
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| anyhow::anyhow!("dock requires its controller"))?;
    let controller = arg("--controller-hwnd")
        .and_then(|v| v.parse::<isize>().ok())
        .ok_or_else(|| anyhow::anyhow!("dock requires its controller window"))?;
    if !controller_live(controller, pid) {
        bail!("dock controller is not available");
    }
    // SAFETY: process setup precedes native windows; mutex and COM lifetimes are balanced below.
    let mutex = unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let h = CreateMutexW(None, false, w!("Local\\ProtonDockMutex"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(h);
            CoUninitialize();
            return Ok(());
        }
        h
    };
    struct Guard(HANDLE);
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
                CoUninitialize();
            }
        }
    }
    let _guard = Guard(mutex);
    let store = LayoutStore::new(supervisor::layout_path());
    let loaded = store.load();
    let read_only = loaded.is_err();
    let status = loaded.as_ref().err().cloned().unwrap_or_default();
    let config = loaded.unwrap_or_default();
    if !config.enabled {
        return Ok(());
    }
    let mut state = State {
        config,
        store,
        read_only,
        snapshot: Snapshot::default(),
        focus: FocusTimer::new(25 * 60),
        panel: String::new(),
        query: String::new(),
        status,
        controller,
    };
    let dock = DockWindow::new()?;
    let panel = DockPanelWindow::new()?;
    let (tx, rx) = mpsc::channel::<String>();
    let send = |prefix: &'static str| {
        let tx = tx.clone();
        move |id: slint::SharedString| {
            let _ = tx.send(format!("{prefix}{id}"));
        }
    };
    dock.on_item_clicked(send("dock:"));
    dock.on_item_context(send("context:"));
    panel.on_action(send(""));
    panel.on_query_edited(send("query:"));
    {
        let t = tx.clone();
        dock.on_edit_toggled(move || {
            let _ = t.send("customize".into());
        });
    }
    {
        let t = tx.clone();
        panel.on_closed(move || {
            let _ = t.send("hide".into());
        });
    }
    {
        let t = tx.clone();
        panel.on_mute_toggled(move || {
            let _ = t.send("mute".into());
        });
    }
    {
        let t = tx.clone();
        panel.on_volume_edited(move |v| {
            let _ = t.send(format!("volume:{v}"));
        });
    }
    {
        let t = tx.clone();
        dock.on_close_requested(move || {
            let _ = t.send("disable".into());
        });
    }
    {
        let t = tx.clone();
        dock.on_profile_selected(move |v| {
            let _ = t.send(format!("profile:{v}"));
        });
    }
    {
        let t = tx.clone();
        dock.on_item_resized(move |id, v| {
            let _ = t.send(format!("resize:{id}:{v}"));
        });
    }
    {
        let t = tx.clone();
        dock.on_item_moved(move |id, v| {
            let _ = t.send(format!("move:{id}:{v}"));
        });
    }
    let (snapshot_tx, snapshot_rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut apps = platform::installed_apps();
        for app in apps.iter_mut().take(40) {
            app.icon = platform::icon_for_path(&app.command, 64);
        }
        loop {
            let windows = platform::enumerate_windows();
            let mut icons = HashMap::new();
            for app in apps.iter().filter(|a| a.icon.is_some()) {
                if let Some(icon) = &app.icon {
                    icons.insert(app.id.clone(), icon.clone());
                }
            }
            for w in windows.iter().take(24) {
                if let Some(icon) = platform::icon_for_window(w.hwnd, 64) {
                    icons.insert(w.identity.clone(), icon);
                }
            }
            let snapshot = Snapshot {
                windows,
                apps: apps.clone(),
                icons,
                audio: platform::audio_snapshot().ok(),
                power: platform::power_snapshot().ok(),
                network: platform::network_snapshot().ok(),
            };
            if snapshot_tx.send(snapshot).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
    refresh(&state, &dock, &panel);
    dock.show()?;
    let monitor = platform::primary_monitor().ok_or_else(|| anyhow::anyhow!("no monitor"))?;
    let mut bar = if state.config.active().reserve_space {
        Some(appbar::AppBar::new(
            monitor,
            (state.config.active().height * monitor.dpi as f32 / 96.) as i32,
        )?)
    } else {
        None
    };
    let weak = dock.as_weak();
    let weak_panel = panel.as_weak();
    let timer = Timer::default();
    let mut last = Instant::now() - Duration::from_secs(2);
    let mut configured = false;
    timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        let (Some(dock), Some(panel)) = (weak.upgrade(), weak_panel.upgrade()) else {
            return;
        };
        let mut dirty = false;
        while let Ok(cmd) = rx.try_recv() {
            if cmd == "disable" {
                state.config.enabled = false;
                state.save();
                let _ = slint::quit_event_loop();
                return;
            }
            if let Some(spec) = cmd.strip_prefix("resize:") {
                if let Some((id, v)) = spec.rsplit_once(':') {
                    if let Ok(v) = v.parse::<f32>() {
                        let old = state
                            .config
                            .active()
                            .widgets
                            .iter()
                            .find(|w| w.id == id)
                            .map(|w| w.width)
                            .unwrap_or(v);
                        state.config.active_mut().resize(id, v - old);
                        state.save();
                    }
                }
            } else if let Some(spec) = cmd.strip_prefix("move:") {
                if let Some((id, v)) = spec.rsplit_once(':') {
                    if let Ok(v) = v.parse() {
                        state.config.active_mut().move_widget(id, v);
                        state.save();
                    }
                }
            } else {
                action(&mut state, &cmd, &panel);
            }
            dirty = true;
        }
        if let Ok(snapshot) = snapshot_rx.try_recv() {
            state.snapshot = snapshot;
            dirty = true;
        }
        if last.elapsed() >= Duration::from_secs(1) {
            if !controller_live(controller, pid) {
                let _ = slint::quit_event_loop();
                return;
            }
            state.focus.tick(Instant::now());
            dirty = true;
            last = Instant::now();
        }
        if dirty {
            refresh(&state, &dock, &panel);
        }
        if let Some(hwnd) = native(dock.window()) {
            if !configured {
                if let Err(e) = platform::configure_window(hwnd, true) {
                    state.status = e.to_string();
                }
                configured = true;
            }
            if let Some(m) = platform::primary_monitor() {
                let profile = state.config.active();
                let height = (profile.height * m.dpi as f32 / 96.) as i32;
                if profile.reserve_space {
                    if let Some(b) = bar.as_mut() {
                        if let Err(e) = b.update(m, height) {
                            state.status = e.to_string();
                        }
                    } else {
                        bar = appbar::AppBar::new(m, height).ok();
                    }
                } else {
                    bar = None;
                }
                let rect = bar.as_ref().map(|b| b.rect()).unwrap_or(RECT {
                    left: m.work_left,
                    right: m.work_right,
                    top: m.work_bottom - height,
                    bottom: m.work_bottom,
                });
                let size = dock.window().size();
                let hidden = platform::fullscreen_on_monitor(m, &[hwnd.0 as isize, controller]);
                unsafe {
                    let _ = ShowWindow(hwnd, if hidden { SW_HIDE } else { SW_SHOWNOACTIVATE });
                    let _ = SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        rect.left + (rect.right - rect.left - size.width as i32) / 2,
                        rect.top,
                        size.width as i32,
                        height,
                        SWP_NOACTIVATE,
                    );
                }
                if let Some(ph) = native(panel.window()) {
                    let _ = platform::configure_window(ph, false);
                    let size = panel.window().size();
                    unsafe {
                        let _ = SetWindowPos(
                            ph,
                            HWND_TOPMOST,
                            (rect.left + (rect.right - rect.left - size.width as i32) / 2)
                                .max(m.left),
                            rect.top - size.height as i32 - 8,
                            size.width as i32,
                            size.height as i32,
                            SWP_NOACTIVATE,
                        );
                    }
                }
            }
        }
    });
    slint::run_event_loop()?;
    drop(timer);
    Ok(())
}
