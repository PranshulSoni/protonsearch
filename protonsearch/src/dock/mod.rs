//! Out-of-process native dock renderer; the launcher remains the recovery controller.
pub mod appbar;
pub mod platform;
pub mod supervisor;
pub mod taskbar;
use crate::ui::{DockItem, DockPanelWindow, DockWindow, PanelItem};
use anyhow::{bail, Result};
use proton_core::dock_model::{
    AppPin, DockConfig, FocusTimer, LayoutStore, WidgetInstance, WidgetKind,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{
    ComponentHandle, Image, Model, ModelRc, SharedPixelBuffer, Timer, TimerMode, VecModel,
};
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
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
    panel_anchor: String,
}
impl State {
    fn save(&mut self) {
        if let Err(error) = self.config.normalize() {
            self.status = error;
            return;
        }
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
// Preserve delegates and keyboard focus when only a clock, volume, or timer value changes.
fn update_rows<T: Clone + 'static>(
    current: ModelRc<T>,
    rows: Vec<T>,
    id: impl Fn(&T) -> &slint::SharedString,
    equal: impl Fn(&T, &T) -> bool,
) -> Option<ModelRc<T>> {
    if current.row_count() != rows.len()
        || rows
            .iter()
            .enumerate()
            .any(|(i, row)| current.row_data(i).is_none_or(|old| id(&old) != id(row)))
    {
        return Some(model(rows));
    }
    for (i, row) in rows.into_iter().enumerate() {
        if current.row_data(i).is_some_and(|old| !equal(&old, &row)) {
            current.set_row_data(i, row);
        }
    }
    None
}
fn dock_equal(a: &DockItem, b: &DockItem) -> bool {
    a.id == b.id
        && a.kind == b.kind
        && a.title == b.title
        && a.subtitle == b.subtitle
        && a.value == b.value
        && a.icon == b.icon
        && a.width == b.width
        && a.running == b.running
        && a.active == b.active
        && a.progress == b.progress
}
fn panel_equal(a: &PanelItem, b: &PanelItem) -> bool {
    a.id == b.id
        && a.title == b.title
        && a.subtitle == b.subtitle
        && a.value == b.value
        && a.icon == b.icon
        && a.selected == b.selected
        && a.progress == b.progress
}
fn image(pixels: &platform::IconPixels) -> Image {
    // Slint images are reference counted; reuse decoded buffers across unchanged snapshots.
    thread_local! { static ICONS: std::cell::RefCell<HashMap<u64, Image>> = std::cell::RefCell::new(HashMap::new()); }
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    pixels.width.hash(&mut hash);
    pixels.height.hash(&mut hash);
    pixels.rgba.hash(&mut hash);
    let key = hash.finish();
    ICONS.with(|icons| {
        let mut icons = icons.borrow_mut();
        if let Some(icon) = icons.get(&key) {
            return icon.clone();
        }
        if icons.len() >= 256 {
            icons.clear();
        }
        let icon = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
            &pixels.rgba,
            pixels.width,
            pixels.height,
        ));
        icons.insert(key, icon.clone());
        icon
    })
}
/// Anchor a popup to its originating tile, with both edges inside the monitor work area.
fn popup_position(
    anchor: i32,
    dock_top: i32,
    width: i32,
    height: i32,
    monitor: platform::MonitorBounds,
) -> (i32, i32) {
    let left = monitor.work_left;
    let top = monitor.work_top;
    let right = monitor.work_right;
    (
        (anchor - width / 2).clamp(left, (right - width).max(left)),
        (dock_top - height - 10).max(top),
    )
}
fn position_panel(
    state: &State,
    dock: &DockWindow,
    panel: &DockPanelWindow,
    monitor: platform::MonitorBounds,
    rect: RECT,
) {
    let scale = monitor.dpi as f32 / 96.;
    let mut offset = 10. - dock.get_scroll_offset();
    let mut anchor = rect.left + (rect.right - rect.left) / 2;
    let dock_size = dock.window().size();
    let dock_left = rect.left + (rect.right - rect.left - dock_size.width as i32) / 2;
    let items = dock.get_items();
    for i in 0..items.row_count() {
        if let Some(item) = items.row_data(i) {
            if item.id.as_str() == state.panel_anchor {
                anchor = dock_left + ((offset + item.width / 2.) * scale) as i32;
                break;
            }
            offset += item.width + 4.;
        }
    }
    let size = panel.window().size();
    let (x, y) = popup_position(
        anchor,
        rect.top,
        size.width as i32,
        size.height as i32,
        monitor,
    );
    panel
        .window()
        .set_position(slint::PhysicalPosition::new(x, y));
}
fn asset(name: &str) -> Image {
    thread_local! { static CACHE: std::cell::RefCell<HashMap<String,Image>> = std::cell::RefCell::new(HashMap::new()); }
    if let Some(image) = CACHE.with(|cache| cache.borrow().get(name).cloned()) {
        return image;
    }
    let bytes: &[u8] = match name {
        "apps" => include_bytes!("../../ui/icons/dock/apps.svg"),
        "agent" => include_bytes!("../../ui/icons/dock/agent.svg"),
        "clock" => include_bytes!("../../ui/icons/dock/clock.svg"),
        "volume" => include_bytes!("../../ui/icons/dock/volume.svg"),
        "network" => include_bytes!("../../ui/icons/dock/network.svg"),
        "power" => include_bytes!("../../ui/icons/dock/power.svg"),
        "music" => include_bytes!("../../ui/icons/dock/music.svg"),
        "weather" => include_bytes!("../../ui/icons/dock/weather.svg"),
        "focus" => include_bytes!("../../ui/icons/dock/focus.svg"),
        "settings" => include_bytes!("../../ui/icons/dock/settings.svg"),
        "file" => include_bytes!("../../ui/icons/dock/file.svg"),
        _ => include_bytes!("../../ui/icons/dock/search.svg"),
    };
    let image = Image::load_from_svg_data(bytes).unwrap_or_default();
    CACHE.with(|cache| {
        cache.borrow_mut().insert(name.to_owned(), image.clone());
    });
    image
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
                    width: 60.,
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
                    width: 60.,
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
            WidgetKind::Weather => ("—".into(), "No forecast".into()),
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
            progress: match widget.kind {
                WidgetKind::Audio => state.snapshot.audio.map_or(0., |a| a.volume as f32 / 100.),
                WidgetKind::Focus => 1. - state.focus.seconds(Instant::now()) as f32 / 1500.,
                WidgetKind::Power => state
                    .snapshot
                    .power
                    .and_then(|p| p.percentage)
                    .map_or(0., |p| p as f32 / 100.),
                _ => 0.,
            },
            ..Default::default()
        });
    }
    let width = (items.iter().map(|i| i.width + 5.).sum::<f32>() + 90.).max(250.);
    if let Some(m) = platform::primary_monitor() {
        let target_width = width.min((m.right - m.left) as f32 * 96. / m.dpi as f32 - 24.);
        let scale = dock.window().scale_factor();
        let current = dock.window().size();
        if (current.width as f32 - target_width * scale).abs() > 1.
            || (current.height as f32 - profile.height * scale).abs() > 1.
        {
            dock.window()
                .set_size(slint::LogicalSize::new(target_width, profile.height));
        }
    }
    if let Some(items) = update_rows(dock.get_items(), items, |item| &item.id, dock_equal) {
        dock.set_items(items);
    }
    dock.set_profile(profile.name.clone().into());
    dock.set_editing(state.panel == "customize");
    dock.set_dock_height(profile.height);
    dock.set_icon_size(profile.icon_size);
    dock.set_status(state.status.clone().into());
    panel.set_taskbar_hidden(state.config.hide_windows_taskbar);
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
        "customize"=>{
            let mut gallery=profile.widgets.clone();
            for kind in WidgetKind::ALL {if !gallery.iter().any(|w|w.kind==kind){let mut widget=WidgetInstance::new(kind);widget.enabled=false;gallery.push(widget);}}
            for widget in gallery {entries.push(PanelItem{id:widget.id.into(),title:if widget.label.is_empty(){widget.kind.title().into()}else{widget.label.into()},subtitle:format!("{} px",widget.width).into(),selected:widget.enabled,icon:asset(kind_icon(widget.kind)),..Default::default()});}
        },
        "weather"=>body="Weather forecasts are not connected yet. Use Windows Weather from Settings, or add a location when a weather provider is available.".into(),
        "music"=>body="Controls the current Windows media session. Track information is not available from this provider.".into(),
        "agents"=>body="Open your existing ProtonSearch Agents page to manage and launch configured agents.".into(),
        "network"=>body=state.snapshot.network.as_ref().map(|n|format!("{}\n{} · {}",n.name,if n.wireless{"Wi-Fi"}else{"Network"},if n.internet{"Internet access"}else if n.connected{"Local connectivity"}else{"Disconnected"})).unwrap_or("Network information unavailable".into()),
        "clock"=>body=format!("{}\nLocal time", clock()),
        "custom-metric"=>body=profile.widgets.iter().find(|w|w.id==state.panel_anchor).map(|w|format!("{}\n{}\n{}",w.label,w.value,w.detail)).unwrap_or_default(),
        "power"=>body=state.snapshot.power.map(|p|format!("{}\n{}",p.percentage.map(|v|format!("Battery: {v}%")).unwrap_or("No battery reported".into()),if p.charging{"Charging"}else if p.ac_power==Some(true){"Connected to AC power"}else{"On battery"})).unwrap_or("Power information unavailable".into()),
        _=>{}
    }
    panel.set_body(body.into());
    if let Some(entries) = update_rows(panel.get_entries(), entries, |entry| &entry.id, panel_equal)
    {
        panel.set_entries(entries);
    }
}
fn launch(command: &str) {
    crate::launcher::launch(command);
}
fn action(state: &mut State, command: &str, panel: &DockPanelWindow) {
    if command == "hide" || command == "back" {
        state.panel.clear();
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
            state.panel.clear();
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
                    if p.widgets.iter().any(|w| w.id == id) {
                        p.toggle(id);
                    } else if let Some(k) = WidgetKind::ALL.into_iter().find(|k| k.key() == id) {
                        p.ensure_kind(k);
                    }
                }
                "smaller" => {
                    p.resize(id, -64.);
                }
                "larger" => {
                    p.resize(id, 64.);
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
    if command == "height-less" || command == "height-more" {
        let p = state.config.active_mut();
        p.height = (p.height + if command == "height-more" { 8. } else { -8. }).clamp(82., 112.);
        state.save();
        return;
    }
    if command == "toggle-taskbar" {
        state.config.hide_windows_taskbar = !state.config.hide_windows_taskbar;
        state.save();
        return;
    }
    if command == "reserve" {
        let p = state.config.active_mut();
        p.reserve_space = !p.reserve_space;
        state.save();
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
                "clock" => "ms-settings:dateandtime",
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
    if command == "refresh" {
        state.status = "System information refresh requested".into();
        return;
    }
    if command == "submit" {
        if state.panel == "apps" {
            let query = state.query.to_lowercase();
            if let Some(app) = state
                .snapshot
                .apps
                .iter()
                .find(|a| a.name.to_lowercase().contains(&query))
            {
                launch(&app.command);
                state.panel.clear();
            }
        }
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
        return;
    }
    if let Some(id) = command.strip_prefix("dock:") {
        if !state.panel.is_empty() && state.panel_anchor == id {
            state.panel.clear();
            return;
        }
        state.panel_anchor = id.to_owned();
        if let Some(identity) = id.strip_prefix("app:") {
            let wins: Vec<_> = state
                .snapshot
                .windows
                .iter()
                .filter(|w| {
                    w.identity == identity
                        || state.config.active().pins.iter().any(|p| {
                            p.id == identity
                                && w.executable
                                    .as_ref()
                                    .is_some_and(|e| e.eq_ignore_ascii_case(&p.command))
                        })
                })
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
            state.query = wins
                .first()
                .map(|w| w.identity.clone())
                .unwrap_or_else(|| identity.into());
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
                    state.panel.clear();
                    host(state, WM_APP + 100);
                    return;
                }
                Some(WidgetKind::Files) => {
                    state.panel.clear();
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
        return;
    }
    if command == "customize" {
        if state.panel == "customize" {
            state.panel.clear();
            return;
        }
        state.panel_anchor.clear();
        state.panel = "customize".into();
        state.query.clear();
        panel.set_query("".into());
        return;
    }
    if let Some(app) = state.snapshot.apps.iter().find(|a| a.id == command) {
        launch(&app.command);
        state.panel.clear();
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
        panel_anchor: String::new(),
    };
    let dock = DockWindow::new()?;
    let panel = DockPanelWindow::new()?;
    panel.window().set_size(slint::LogicalSize::new(404., 460.));
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
        dock.window().on_close_requested(move || {
            let _ = t.send("disable".into());
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
    {
        let t = tx.clone();
        panel.window().on_close_requested(move || {
            let _ = t.send("hide".into());
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
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
    let (request_tx, request_rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut apps = platform::installed_apps();
        let mut next_icon = 0;
        let mut cached_icons = HashMap::new();
        loop {
            let windows = platform::enumerate_windows();
            let mut icons = HashMap::new();
            if let Some(folder_icon) = platform::icon_for_path(
                &format!(
                    "{}\\explorer.exe",
                    std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into())
                ),
                64,
            ) {
                icons.insert("files".into(), folder_icon);
            }
            for app in apps.iter().filter(|a| a.icon.is_some()) {
                if let Some(icon) = &app.icon {
                    icons.insert(app.id.clone(), icon.clone());
                }
            }
            for w in windows.iter().take(24) {
                let icon = cached_icons.entry(w.identity.clone()).or_insert_with(|| {
                    w.executable
                        .as_ref()
                        .and_then(|path| platform::icon_for_path(path, 128))
                        .or_else(|| platform::icon_for_window(w.hwnd, 128))
                });
                if let Some(icon) = icon {
                    icons.insert(w.identity.clone(), icon.clone());
                }
            }
            if cached_icons.len() > 256 {
                cached_icons.retain(|id, _| windows.iter().any(|w| &w.identity == id));
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
            // Publish useful system/window data immediately; fill drawer artwork in bounded batches.
            let end = (next_icon + 6).min(apps.len());
            for app in &mut apps[next_icon..end] {
                app.icon = platform::icon_for_path(&app.command, 128);
            }
            next_icon = end;
            let _ = request_rx.recv_timeout(Duration::from_secs(2));
        }
    });
    refresh(&state, &dock, &panel);
    let monitor = platform::primary_monitor().ok_or_else(|| anyhow::anyhow!("no monitor"))?;
    let mut bar = if state.config.active().reserve_space {
        Some(appbar::AppBar::new(
            monitor,
            (state.config.active().height * monitor.dpi as f32 / 96.) as i32,
        )?)
    } else {
        None
    };
    // Set the initial location before mapping the window to avoid a visible jump from (0, 0).
    let initial_height = (state.config.active().height * monitor.dpi as f32 / 96.) as i32;
    let initial_rect = bar.as_ref().map(|b| b.rect()).unwrap_or(RECT {
        left: monitor.work_left,
        right: monitor.work_right,
        top: monitor.work_bottom - initial_height,
        bottom: monitor.work_bottom,
    });
    let initial_size = dock.window().size();
    dock.window().set_position(slint::PhysicalPosition::new(
        initial_rect.left
            + (initial_rect.right - initial_rect.left - initial_size.width as i32) / 2,
        initial_rect.top,
    ));
    if let Some(hwnd) = native(dock.window()) {
        platform::configure_window(hwnd, true)?;
    }
    dock.show()?;
    let mut taskbar_visibility = taskbar::TaskbarVisibility::new(supervisor::layout_path());
    let weak = dock.as_weak();
    let weak_panel = panel.as_weak();
    let timer = Timer::default();
    let mut last = Instant::now() - Duration::from_secs(2);
    let mut configured = false;
    let mut panel_configured = None;
    let mut bar_geometry = None;
    let mut native_geometry = None;
    let mut native_hidden = None;
    let mut visible_panel = String::new();
    let mut hide_panel_at: Option<Instant> = None;
    let mut focus_seen = false;
    let mut taskbar_setting = None;
    struct StopEvent(HANDLE);
    impl Drop for StopEvent {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let stop_name: Vec<u16> = format!("Local\\ProtonDockStop-{pid}")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: the controller owns the event; this process owns and closes its opened handle.
    let stop = unsafe {
        OpenEventW(
            SYNCHRONIZATION_SYNCHRONIZE,
            false,
            windows::core::PCWSTR(stop_name.as_ptr()),
        )
        .ok()
        .map(StopEvent)
    };
    timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        if stop
            .as_ref()
            .is_some_and(|event| unsafe { WaitForSingleObject(event.0, 0) == WAIT_OBJECT_0 })
        {
            let _ = slint::quit_event_loop();
            return;
        }
        let (Some(dock), Some(panel)) = (weak.upgrade(), weak_panel.upgrade()) else {
            return;
        };
        if hide_panel_at.is_some_and(|deadline| Instant::now() >= deadline) {
            let _ = panel.hide();
            hide_panel_at = None;
        }
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
                if cmd == "refresh" {
                    let _ = request_tx.try_send(());
                }
                action(&mut state, &cmd, &panel);
            }
            dirty = true;
        }
        if let Ok(snapshot) = snapshot_rx.try_recv() {
            state.snapshot = snapshot;
            dirty = true;
        }
        let heartbeat = last.elapsed() >= Duration::from_secs(1);
        if heartbeat {
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
        if !state.panel.is_empty() && panel.window().is_visible() {
            let focused =
                native(panel.window()).is_some_and(|hwnd| unsafe { GetForegroundWindow() == hwnd });
            if focused {
                focus_seen = true;
            }
            if focus_seen && !focused {
                state.panel.clear();
                dirty = true;
                focus_seen = false;
            }
        }
        if !dirty {
            return;
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
                if bar_geometry != Some((m, height, profile.reserve_space)) {
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
                    bar_geometry = Some((m, height, profile.reserve_space));
                }
                let rect = bar.as_ref().map(|b| b.rect()).unwrap_or(RECT {
                    left: m.work_left,
                    right: m.work_right,
                    top: m.work_bottom - height,
                    bottom: m.work_bottom,
                });
                let size = dock.window().size();
                let hidden = platform::fullscreen_on_monitor(m, &[hwnd.0 as isize, controller]);
                let geometry = (
                    rect.left + (rect.right - rect.left - size.width as i32) / 2,
                    rect.top,
                    size.width as i32,
                    height,
                );
                unsafe {
                    if native_geometry != Some(geometry) {
                        let _ = SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            geometry.0,
                            geometry.1,
                            geometry.2,
                            geometry.3,
                            SWP_NOACTIVATE,
                        );
                        native_geometry = Some(geometry);
                    }
                    if native_hidden != Some(hidden) {
                        let _ = ShowWindow(hwnd, if hidden { SW_HIDE } else { SW_SHOWNOACTIVATE });
                        native_hidden = Some(hidden);
                    }
                }
                if heartbeat || taskbar_setting != Some(state.config.hide_windows_taskbar) {
                    if let Err(error) =
                        taskbar_visibility.set_hidden(state.config.hide_windows_taskbar)
                    {
                        state.status = format!("Taskbar setting: {error}");
                    } else {
                        taskbar_setting = Some(state.config.hide_windows_taskbar);
                    }
                }
                if state.panel != visible_panel {
                    if state.panel.is_empty() {
                        panel.set_revealed(false);
                        hide_panel_at = Some(Instant::now() + Duration::from_millis(150));
                    } else {
                        hide_panel_at = None;
                        panel.set_revealed(false);
                        position_panel(&state, &dock, &panel, m, rect);
                        if let Some(ph) = native(panel.window()) {
                            if panel_configured != Some(ph.0 as isize) {
                                let _ = platform::configure_window(ph, false);
                                panel_configured = Some(ph.0 as isize);
                            }
                        }
                        let _ = panel.show();
                        // Some backends create the native handle only while mapping the window.
                        if let Some(ph) = native(panel.window()) {
                            if panel_configured != Some(ph.0 as isize) {
                                let _ = platform::configure_window(ph, false);
                                panel_configured = Some(ph.0 as isize);
                            }
                        }
                        position_panel(&state, &dock, &panel, m, rect);
                        let weak_reveal = panel.as_weak();
                        Timer::single_shot(Duration::from_millis(16), move || {
                            if let Some(panel) = weak_reveal.upgrade() {
                                panel.set_revealed(true);
                            }
                        });
                        focus_seen = false;
                    }
                    visible_panel = state.panel.clone();
                }
            }
        }
    });
    slint::run_event_loop()?;
    drop(timer);
    Ok(())
}

#[cfg(test)]
mod geometry_tests {
    use super::*;
    fn monitor() -> platform::MonitorBounds {
        platform::MonitorBounds {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1080,
            work_left: -1920,
            work_top: 0,
            work_right: 0,
            work_bottom: 1040,
            dpi: 144,
        }
    }
    #[test]
    fn panel_clamps_both_sides_on_negative_coordinate_monitor() {
        assert_eq!(
            popup_position(-1910, 950, 404, 460, monitor()),
            (-1920, 480)
        );
        assert_eq!(popup_position(-10, 950, 404, 460, monitor()), (-404, 480));
    }
    #[test]
    fn panel_anchors_above_origin_and_stays_below_work_area_top() {
        assert_eq!(popup_position(-960, 950, 404, 460, monitor()), (-1162, 480));
        assert_eq!(popup_position(-960, 300, 404, 460, monitor()), (-1162, 0));
    }
}
