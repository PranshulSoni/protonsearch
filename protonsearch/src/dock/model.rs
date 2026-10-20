//! Validated, versioned layouts. Loading never changes the legacy search settings.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_WIDGETS: usize = 32;
pub const MAX_PINS: usize = 32;
pub const MAX_CONFIG_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WidgetKind {
    Start, Search, Applications, Agents, Weather, Music, Focus, Audio, Power, Network,
    Clock, Files, SystemTray, CustomMetric,
}

impl WidgetKind {
    pub const ALL: [Self; 14] = [Self::Start, Self::Search, Self::Applications, Self::Agents,
        Self::Weather, Self::Music, Self::Focus, Self::Audio, Self::Power, Self::Network,
        Self::Clock, Self::Files, Self::SystemTray, Self::CustomMetric];

    pub fn key(self) -> &'static str {
        match self {
            Self::Start => "start", Self::Search => "search", Self::Applications => "applications",
            Self::Agents => "agents", Self::Weather => "weather", Self::Music => "music",
            Self::Focus => "focus", Self::Audio => "audio", Self::Power => "power",
            Self::Network => "network", Self::Clock => "clock", Self::Files => "files",
            Self::SystemTray => "system-tray", Self::CustomMetric => "custom-metric",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Start => "Start", Self::Search => "Proton Search", Self::Applications => "Applications",
            Self::Agents => "Agents", Self::Weather => "Weather", Self::Music => "Now playing",
            Self::Focus => "Focus", Self::Audio => "Sound", Self::Power => "Battery",
            Self::Network => "Network", Self::Clock => "Clock", Self::Files => "Files",
            Self::SystemTray => "Windows tray", Self::CustomMetric => "Custom metric",
        }
    }

    pub fn width_bounds(self) -> (f32, f32, f32) {
        match self {
            Self::Start | Self::Applications => (56.0, 64.0, 96.0),
            Self::Audio | Self::Power | Self::Network | Self::SystemTray => (64.0, 88.0, 220.0),
            Self::Weather | Self::Music => (96.0, 184.0, 380.0),
            Self::Search => (72.0, 136.0, 260.0),
            Self::Clock | Self::Focus => (96.0, 124.0, 260.0),
            _ => (88.0, 152.0, 380.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetInstance {
    pub id: String,
    pub kind: WidgetKind,
    pub width: f32,
    pub enabled: bool,
    pub label: String,
    pub value: String,
    pub detail: String,
}

impl Default for WidgetInstance {
    fn default() -> Self { Self::new(WidgetKind::Search) }
}

impl WidgetInstance {
    pub fn new(kind: WidgetKind) -> Self {
        Self { id: kind.key().into(), kind, width: kind.width_bounds().1, enabled: true,
            label: String::new(), value: String::new(), detail: String::new() }
    }

    fn normalize(&mut self) {
        self.id = clean_text(&self.id, 64);
        let (min, preferred, max) = self.kind.width_bounds();
        self.width = if self.width.is_finite() { self.width.clamp(min, max) } else { preferred };
        self.label = clean_text(&self.label, 40);
        self.value = clean_text(&self.value, 40);
        self.detail = clean_text(&self.detail, 120);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPin {
    pub id: String,
    pub title: String,
    pub command: String,
}

impl Default for AppPin {
    fn default() -> Self { Self { id: "files".into(), title: "File Explorer".into(), command: "explorer.exe".into() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub widgets: Vec<WidgetInstance>,
    pub pins: Vec<AppPin>,
    pub height: f32,
    pub icon_size: f32,
    pub reserve_space: bool,
    pub presentation: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self { id: "work".into(), name: "Work".into(),
            widgets: [WidgetKind::Start, WidgetKind::Search, WidgetKind::Applications,
                WidgetKind::Weather, WidgetKind::Focus, WidgetKind::Audio, WidgetKind::Clock,
                WidgetKind::Agents].into_iter().map(WidgetInstance::new).collect(),
            pins: vec![AppPin::default()], height: 96.0, icon_size: 52.0,
            reserve_space: true, presentation: false }
    }
}

impl Profile {
    pub fn normalize(&mut self) {
        self.id = clean_text(&self.id, 48);
        self.name = clean_text(&self.name, 40);
        self.height = if self.height.is_finite() { self.height.clamp(80.0, 144.0) } else { 96.0 };
        self.icon_size = if self.icon_size.is_finite() { self.icon_size.clamp(40.0, (self.height - 30.0).min(80.0)) } else { 52.0 };
        self.widgets.truncate(MAX_WIDGETS);
        let mut ids = HashSet::new();
        self.widgets.retain_mut(|widget| {
            widget.normalize(); !widget.id.is_empty() && ids.insert(widget.id.clone())
        });
        self.pins.truncate(MAX_PINS);
        let mut commands = HashSet::new();
        self.pins.retain_mut(|pin| {
            pin.id = clean_text(&pin.id, 128); pin.title = clean_text(&pin.title, 128);
            pin.command = clean_text(&pin.command, 8192);
            !pin.id.is_empty() && !pin.command.is_empty() && commands.insert(pin.command.to_lowercase())
        });
    }

    pub fn resize(&mut self, id: &str, delta: f32) -> bool {
        let Some(widget) = self.widgets.iter_mut().find(|widget| widget.id == id) else { return false; };
        if !delta.is_finite() { return false; }
        widget.width += delta; widget.normalize(); true
    }

    pub fn move_widget(&mut self, id: &str, delta: i32) -> bool {
        let Some(index) = self.widgets.iter().position(|widget| widget.id == id) else { return false; };
        let next = (index as i64 + i64::from(delta)).clamp(0, self.widgets.len().saturating_sub(1) as i64) as usize;
        if index == next { return false; }
        let widget = self.widgets.remove(index); self.widgets.insert(next, widget); true
    }

    pub fn toggle(&mut self, id: &str) -> bool {
        if let Some(widget) = self.widgets.iter_mut().find(|widget| widget.id == id) {
            widget.enabled = !widget.enabled; return true;
        }
        false
    }

    pub fn ensure_kind(&mut self, kind: WidgetKind) -> bool {
        if let Some(widget) = self.widgets.iter_mut().find(|widget| widget.kind == kind) {
            widget.enabled = !widget.enabled; return widget.enabled;
        }
        self.widgets.push(WidgetInstance::new(kind)); self.normalize(); true
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WeatherLocation {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl WeatherLocation {
    pub fn valid(&self) -> bool {
        !self.name.trim().is_empty() && self.latitude.is_finite() && self.longitude.is_finite()
            && (-90.0..=90.0).contains(&self.latitude) && (-180.0..=180.0).contains(&self.longitude)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DockConfig {
    pub version: u32,
    pub enabled: bool,
    pub active_profile: String,
    pub profiles: Vec<Profile>,
    pub weather: Option<WeatherLocation>,
}

impl Default for DockConfig {
    fn default() -> Self {
        let work = Profile::default();
        let mut home = Profile { id: "home".into(), name: "Home".into(), ..work.clone() };
        home.widgets = [WidgetKind::Start, WidgetKind::Search, WidgetKind::Applications,
            WidgetKind::Weather, WidgetKind::Music, WidgetKind::Audio, WidgetKind::Power,
            WidgetKind::Clock].into_iter().map(WidgetInstance::new).collect();
        let mut presentation = Profile { id: "presentation".into(), name: "Presentation".into(),
            presentation: true, ..work.clone() };
        presentation.widgets = [WidgetKind::Start, WidgetKind::Applications, WidgetKind::Audio,
            WidgetKind::Clock].into_iter().map(WidgetInstance::new).collect();
        Self { version: SCHEMA_VERSION, enabled: true, active_profile: "work".into(),
            profiles: vec![work, home, presentation], weather: None }
    }
}

impl DockConfig {
    pub fn normalize(&mut self) -> Result<(), String> {
        if self.version != SCHEMA_VERSION { return Err(format!("unsupported dock layout version {}", self.version)); }
        self.profiles.truncate(12);
        let mut ids = HashSet::new();
        self.profiles.retain_mut(|profile| { profile.normalize(); !profile.id.is_empty() && ids.insert(profile.id.clone()) });
        if self.profiles.is_empty() { self.profiles = Self::default().profiles; }
        if !self.profiles.iter().any(|profile| profile.id == self.active_profile) { self.active_profile = self.profiles[0].id.clone(); }
        if self.weather.as_ref().is_some_and(|location| !location.valid()) { self.weather = None; }
        Ok(())
    }

    pub fn active(&self) -> &Profile {
        self.profiles.iter().find(|profile| profile.id == self.active_profile).unwrap_or(&self.profiles[0])
    }

    pub fn active_mut(&mut self) -> &mut Profile {
        let index = self.profiles.iter().position(|profile| profile.id == self.active_profile).unwrap_or(0);
        &mut self.profiles[index]
    }

    pub fn select_profile(&mut self, id: &str) -> bool {
        if !self.profiles.iter().any(|profile| profile.id == id) { return false; }
        self.active_profile = id.into(); true
    }
}

pub fn clean_text(value: &str, max_chars: usize) -> String {
    value.chars().filter(|ch| !ch.is_control()).take(max_chars).collect::<String>().trim().to_owned()
}

pub struct LayoutStore { path: PathBuf }

impl LayoutStore {
    pub fn new(path: impl Into<PathBuf>) -> Self { Self { path: path.into() } }
    pub fn path(&self) -> &Path { &self.path }

    pub fn load(&self) -> Result<DockConfig, String> {
        match std::fs::metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(DockConfig::default()),
            Err(error) => return Err(format!("cannot inspect dock layout: {error}")),
            Ok(metadata) if metadata.len() > MAX_CONFIG_BYTES => return Err("dock layout is too large".into()),
            _ => {}
        }
        let bytes = std::fs::read(&self.path).map_err(|error| format!("cannot read dock layout: {error}"))?;
        let mut config: DockConfig = serde_json::from_slice(&bytes).map_err(|error| format!("cannot parse dock layout: {error}"))?;
        config.normalize()?; Ok(config)
    }

    pub fn save(&self, config: &DockConfig) -> Result<(), String> {
        use std::io::Write;
        let mut normalized = config.clone(); normalized.normalize()?;
        let bytes = serde_json::to_vec_pretty(&normalized).map_err(|error| error.to_string())?;
        if bytes.len() as u64 > MAX_CONFIG_BYTES { return Err("dock layout is too large".into()); }
        let parent = self.path.parent().ok_or("dock layout has no parent directory")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        if self.path.exists() { std::fs::copy(&self.path, self.path.with_extension("json.bak")).map_err(|error| format!("cannot back up dock layout: {error}"))?; }
        let temp = self.path.with_extension(format!("{}.tmp", std::process::id()));
        let result = (|| {
            let mut file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temp).map_err(|error| error.to_string())?;
            file.write_all(&bytes).and_then(|()| file.sync_all()).map_err(|error| error.to_string())?;
            drop(file);
            std::fs::rename(&temp, &self.path).map_err(|error| format!("cannot replace dock layout: {error}"))
        })();
        if result.is_err() { let _ = std::fs::remove_file(&temp); }
        result
    }
}

#[derive(Debug, Clone)]
pub struct FocusTimer { duration: Duration, remaining: Duration, deadline: Option<Instant> }

impl FocusTimer {
    pub fn new(seconds: u64) -> Self { let duration = Duration::from_secs(seconds.clamp(1, 24 * 60 * 60)); Self { duration, remaining: duration, deadline: None } }
    pub fn running(&self) -> bool { self.deadline.is_some() }
    pub fn seconds(&self, now: Instant) -> u64 { self.deadline.map_or(self.remaining, |end| end.saturating_duration_since(now)).as_secs() }
    pub fn start(&mut self, now: Instant) { if self.running() { return; } if self.remaining.is_zero() { self.remaining = self.duration; } self.deadline = now.checked_add(self.remaining); }
    pub fn pause(&mut self, now: Instant) { if let Some(end) = self.deadline.take() { self.remaining = end.saturating_duration_since(now); } }
    pub fn reset(&mut self) { self.deadline = None; self.remaining = self.duration; }
    pub fn tick(&mut self, now: Instant) -> bool { if self.deadline.is_some_and(|end| now >= end) { self.remaining = Duration::ZERO; self.deadline = None; return true; } false }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn defaults_have_three_independent_profiles() {
        let mut config = DockConfig::default(); config.active_mut().widgets[0].width = 80.0;
        assert_eq!(config.profiles.len(), 3); assert_eq!(config.profiles[1].widgets[0].width, 64.0);
        assert!(config.profiles[2].presentation);
    }
    #[test] fn invalid_and_duplicate_geometry_is_normalized() {
        let mut config = DockConfig::default(); let profile = config.active_mut();
        profile.height = f32::NAN; profile.icon_size = 800.0; profile.widgets[0].width = f32::INFINITY;
        profile.widgets.push(profile.widgets[0].clone()); profile.normalize();
        assert_eq!(profile.height, 96.0); assert_eq!(profile.icon_size, 66.0);
        assert_eq!(profile.widgets[0].width, 64.0); assert_eq!(profile.widgets.len(), 8);
    }
    #[test] fn resize_and_reorder_obey_widget_constraints() {
        let mut profile = Profile::default(); assert!(profile.resize("weather", 900.0));
        assert_eq!(profile.widgets.iter().find(|w| w.id == "weather").unwrap().width, 380.0);
        assert!(profile.move_widget("search", -99)); assert_eq!(profile.widgets[0].id, "search");
        assert!(!profile.resize("search", f32::NAN)); assert!(!profile.move_widget("absent", 1));
    }
    #[test] fn unknown_profile_and_future_version_do_not_silently_migrate() {
        let mut config = DockConfig::default(); assert!(!config.select_profile("unknown"));
        config.version += 1; assert!(config.normalize().is_err());
    }
    #[test] fn text_limits_are_unicode_safe_and_remove_control_characters() {
        assert_eq!(clean_text("  界🙂\0hello\n", 4), "界🙂");
        assert_eq!(clean_text("abc\r\ndef", 6), "abcdef");
    }
    #[test] fn malformed_weather_location_is_not_used() {
        let mut config = DockConfig::default(); config.weather = Some(WeatherLocation { name: "City".into(), latitude: 999.0, longitude: 0.0 });
        config.normalize().unwrap(); assert!(config.weather.is_none());
    }
    #[test] fn focus_timer_pause_resume_completion_and_reset_use_elapsed_time() {
        let now = Instant::now(); let mut timer = FocusTimer::new(10); timer.start(now);
        timer.pause(now + Duration::from_secs(3)); assert_eq!(timer.seconds(now), 7);
        timer.start(now + Duration::from_secs(100)); assert_eq!(timer.seconds(now + Duration::from_secs(104)), 3);
        assert!(timer.tick(now + Duration::from_secs(107))); assert!(!timer.running());
        assert_eq!(timer.seconds(now), 0); timer.reset(); assert_eq!(timer.seconds(now), 10);
    }
    #[test] fn missing_layout_loads_defaults_and_corrupt_layout_is_preserved() {
        let directory = std::env::temp_dir().join(format!("proton-dock-model-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&directory).unwrap(); let store = LayoutStore::new(directory.join("layout.json"));
        assert!(store.load().is_ok()); std::fs::write(store.path(), "invalid").unwrap();
        assert!(store.load().is_err()); assert_eq!(std::fs::read_to_string(store.path()).unwrap(), "invalid");
        std::fs::remove_file(store.path()).unwrap(); std::fs::remove_dir(directory).unwrap();
    }
    #[test] fn saved_layout_round_trips_and_retains_previous_backup() {
        let directory = std::env::temp_dir().join(format!("proton-dock-store-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&directory).unwrap(); let store = LayoutStore::new(directory.join("layout.json"));
        let mut config = DockConfig::default(); store.save(&config).unwrap(); config.select_profile("home"); store.save(&config).unwrap();
        assert_eq!(store.load().unwrap().active_profile, "home");
        let backup: DockConfig = serde_json::from_slice(&std::fs::read(store.path().with_extension("json.bak")).unwrap()).unwrap();
        assert_eq!(backup.active_profile, "work"); std::fs::remove_file(store.path()).unwrap();
        std::fs::remove_file(store.path().with_extension("json.bak")).unwrap(); std::fs::remove_dir(directory).unwrap();
    }
}
