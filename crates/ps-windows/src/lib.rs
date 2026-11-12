//! Win32 helpers shared by the engine. OWNER: sources-content.
//!
//! Every operation here is safe for untrusted paths: structured arguments only,
//! no shell string interpolation, no elevation by default. All Windows-specific
//! code lives in `imp`; on other platforms the functions fail gracefully so the
//! workspace still type-checks for CI.

use std::path::{Path, PathBuf};

#[cfg(windows)]
mod imp;

/// Information about the current foreground window (used by window switching
/// and activity tracking).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub title: String,
    pub process_id: u32,
    pub exe_path: Option<PathBuf>,
}

/// Resolved contents of a Windows .lnk shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutInfo {
    /// Absolute target path (may be empty for shell-only shortcuts).
    pub target: Option<PathBuf>,
    pub arguments: Option<String>,
    /// Icon location from the shortcut, when it has an explicit override.
    pub icon_location: Option<PathBuf>,
    pub icon_index: i32,
}

/// One entry discovered under the shell AppsFolder namespace (packaged/UWP apps
/// and classic Start Menu apps exposed through the shell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppsFolderEntry {
    pub name: String,
    /// Application User Model ID; empty for non-packaged entries.
    pub aumid: String,
}

/// A decoded clipboard image (straight RGBA8, top-down rows).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// One clipboard change captured by the listener.
///
/// A single copy event can expose several representations (for example text +
/// bitmap). All of them are reported here so the persistence layer can keep one
/// history item without losing the image, files, HTML or RTF alternatives.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClipboardSnapshot {
    pub text: Option<String>,
    /// Best image representation, PNG encoded.
    pub png: Option<Vec<u8>>,
    /// Original image dimensions (before PNG encoding).
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Which clipboard format produced the image: "png" | "dibv5" | "dib" | "bitmap".
    pub image_format: Option<String>,
    /// MIME of the stored image payload (currently image/png).
    pub mime: Option<String>,
    /// File list from CF_HDROP (Explorer-style copy).
    pub files: Vec<String>,
    /// HTML fragment from the registered "HTML Format" clipboard format.
    pub html: Option<String>,
    /// RTF from the registered "Rich Text Format" clipboard format.
    pub rtf: Option<String>,
    /// Clipbord format names present for this copy (diagnostics/debugging).
    pub formats: Vec<String>,
    /// Executable name (no path) of the app that was foreground when the
    /// clipboard changed, when it can be determined.
    pub source_app: Option<String>,
    /// Unix milliseconds at capture time.
    pub timestamp_ms: i64,
}

impl ClipboardSnapshot {
    /// True when the copy event carried any content we can persist or preview.
    pub fn has_content(&self) -> bool {
        self.text.is_some()
            || self.png.is_some()
            || !self.files.is_empty()
            || self.html.is_some()
            || self.rtf.is_some()
    }
}

/// Handle to the background clipboard listener thread.
pub struct ClipboardListener {
    thread_id: u32,
    join: Option<std::thread::JoinHandle<()>>,
}

impl ClipboardListener {
    /// Register a callback invoked (on the listener thread) for every clipboard
    /// update that changes text or image content. Only one listener may run at a
    /// time per process.
    pub fn start<F>(callback: F) -> anyhow::Result<Self>
    where
        F: Fn(ClipboardSnapshot) + Send + 'static,
    {
        #[cfg(windows)]
        {
            imp::clipboard::start_listener(callback)
        }
        #[cfg(not(windows))]
        {
            let _ = callback;
            anyhow::bail!("clipboard listener is only supported on Windows")
        }
    }

    /// Stop the listener and wait for its thread to exit.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        if self.join.is_some() {
            #[cfg(windows)]
            imp::clipboard::stop_listener(self.thread_id);
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }
}

impl Drop for ClipboardListener {
    /// Dropping the listener also stops the hook thread, so forgetting an
    /// explicit stop cannot leak a background thread that keeps writing to the
    /// clipboard history after the engine is gone.
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Open a file or folder with the OS default handler.
pub fn open_path(path: &Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::shell::open_path(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        anyhow::bail!("open_path is only supported on Windows")
    }
}

/// Open a file in an editor at a specific line/column. Falls back to plain open
/// when no supported editor is detected.
pub fn open_path_at(path: &Path, line: Option<u32>, column: Option<u32>) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::shell::open_path_at(path, line, column)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, line, column);
        anyhow::bail!("open_path_at is only supported on Windows")
    }
}

/// Reveal a path in File Explorer with the item selected.
pub fn reveal_path(path: &Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::shell::reveal_path(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        anyhow::bail!("reveal_path is only supported on Windows")
    }
}

/// Open an http(s) URL in the default browser. Rejects non-http schemes.
pub fn open_url(url: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::shell::open_url(url)
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        anyhow::bail!("open_url is only supported on Windows")
    }
}

/// Launch a packaged app (AppsFolder) or executable target.
pub fn launch_app(app_id: &str, target: Option<&str>) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::shell::launch_app(app_id, target)
    }
    #[cfg(not(windows))]
    {
        let _ = (app_id, target);
        anyhow::bail!("launch_app is only supported on Windows")
    }
}

/// Resolve a .lnk shortcut to its target/arguments (COM IShellLink).
pub fn resolve_shortcut(lnk: &Path) -> anyhow::Result<ShortcutInfo> {
    #[cfg(windows)]
    {
        imp::shell::resolve_shortcut(lnk)
    }
    #[cfg(not(windows))]
    {
        let _ = lnk;
        anyhow::bail!("resolve_shortcut is only supported on Windows")
    }
}

/// Enumerate packaged/classic apps in the shell AppsFolder namespace.
pub fn list_appsfolder() -> anyhow::Result<Vec<AppsFolderEntry>> {
    #[cfg(windows)]
    {
        imp::shell::list_appsfolder()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("list_appsfolder is only supported on Windows")
    }
}

/// Extract a real Windows icon to a cached PNG and return its path.
///
/// Icons are resolved from the target of a .lnk (so no shortcut overlay arrow)
/// and written atomically into `cache_dir`. Repeated calls are cheap.
pub fn extract_icon_png(source: &Path, size: u32, cache_dir: &Path) -> anyhow::Result<PathBuf> {
    #[cfg(windows)]
    {
        imp::icons::extract_icon_png(source, size, cache_dir)
    }
    #[cfg(not(windows))]
    {
        let _ = (source, size, cache_dir);
        anyhow::bail!("icon extraction is only supported on Windows")
    }
}

/// Current foreground window, skipping our own process.
pub fn foreground_window() -> Option<WindowInfo> {
    #[cfg(windows)]
    {
        imp::window::foreground_window()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Executable name (file stem) of the foreground window's process.
pub fn foreground_process_name() -> Option<String> {
    #[cfg(windows)]
    {
        imp::window::foreground_process_name()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Set the OS clipboard text (used by copy-path / calculator).
pub fn set_clipboard_text(text: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::clipboard::set_text(text)
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        anyhow::bail!("clipboard is only supported on Windows")
    }
}

/// Read the current clipboard as a full multi-representation snapshot
/// (text, image, files, HTML, RTF). Used to seed clipboard history.
pub fn read_clipboard_snapshot() -> ClipboardSnapshot {
    #[cfg(windows)]
    {
        imp::clipboard::snapshot()
    }
    #[cfg(not(windows))]
    {
        ClipboardSnapshot::default()
    }
}

/// Put a PNG image back on the clipboard as CF_DIB + CF_DIBV5 + registered PNG
/// + CF_BITMAP, so Paint, Office and Snipping-style targets can paste it.
pub fn set_clipboard_image(png_bytes: &[u8]) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::clipboard::set_image(png_bytes)
    }
    #[cfg(not(windows))]
    {
        let _ = png_bytes;
        anyhow::bail!("clipboard is only supported on Windows")
    }
}

/// Read clipboard text, if any.
pub fn get_clipboard_text() -> Option<String> {
    #[cfg(windows)]
    {
        imp::clipboard::get_text()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Read the clipboard bitmap as RGBA, if any.
pub fn get_clipboard_image() -> Option<ClipboardImage> {
    #[cfg(windows)]
    {
        imp::clipboard::get_image()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Read the clipboard bitmap encoded as PNG, if any.
pub fn get_clipboard_image_png() -> Option<Vec<u8>> {
    #[cfg(windows)]
    {
        imp::clipboard::get_image_png()
    }
    #[cfg(not(windows))]
    {
        None
    }
}
// ---------------------------------------------------------------------------
// Reviewed native commands (destructive ones are confirmed by the UI/engine)
// ---------------------------------------------------------------------------

/// Lock the workstation.
pub fn lock_workstation() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::lock_workstation()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("not supported on this platform")
    }
}

/// Suspend (sleep) the machine.
pub fn sleep_pc() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::sleep_pc()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("not supported on this platform")
    }
}

/// Shut down the machine (caller must have confirmed the destructive action).
pub fn shutdown_pc() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::shutdown_pc()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("not supported on this platform")
    }
}

/// Restart the machine (caller must have confirmed the destructive action).
pub fn restart_pc() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::restart_pc()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("not supported on this platform")
    }
}

/// Empty the recycle bin (caller must have confirmed the destructive action).
pub fn empty_recycle_bin() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::empty_recycle_bin()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("not supported on this platform")
    }
}

/// Open the Windows Settings app (fixed target, no user input).
pub fn open_settings() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::open_settings()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("not supported on this platform")
    }
}

/// Open Task Manager.
pub fn open_task_manager() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::open_task_manager()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("not supported on this platform")
    }
}

/// Open Windows Terminal, falling back to PowerShell.
pub fn open_terminal() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::open_terminal()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("not supported on this platform")
    }
}

/// Broadcast WM_SETTINGCHANGE after a user setting (theme) change.
pub fn broadcast_setting_change() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::native::broadcast_setting_change()
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}
// ---------------------------------------------------------------------------
// Native adapter layer (task-35): capabilities, Core Audio, brightness,
// notifications. Every API is user-level, bounded and honest about support.
// ---------------------------------------------------------------------------

/// Result of one capability probe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Capability {
    pub supported: bool,
    /// Why the capability is unavailable (None when supported unless a note).
    pub reason: Option<String>,
    /// Honest "open Windows settings" fallback for unsupported OS toggles.
    #[serde(default)]
    pub fallback_uri: Option<String>,
}

impl Capability {
    pub fn supported() -> Self {
        Self {
            supported: true,
            reason: None,
            fallback_uri: None,
        }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            supported: false,
            reason: Some(reason.into()),
            fallback_uri: None,
        }
    }

    /// Unsupported state with an allow-listed ms-settings fallback page.
    pub fn unsupported_with_fallback(reason: impl Into<String>, page: SettingsPage) -> Self {
        Self {
            supported: false,
            reason: Some(reason.into()),
            fallback_uri: Some(page.uri().to_string()),
        }
    }

    pub fn is_supported(&self) -> bool {
        self.supported
    }

    pub fn with_fallback(mut self, page: SettingsPage) -> Self {
        self.fallback_uri = Some(page.uri().to_string());
        self
    }
}

/// Allow-listed Windows Settings pages used as honest fallbacks instead of dead
/// toggles. No user-controlled URI strings are accepted anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsPage {
    Wifi,
    Bluetooth,
    Display,
    Sound,
    Notifications,
    AirplaneMode,
    MobileHotspot,
    NightLight,
    EnergySaver,
    Project,
    WindowsUpdate,
    PrivacyLocation,
}

impl SettingsPage {
    pub fn uri(self) -> &'static str {
        match self {
            SettingsPage::Wifi => "ms-settings:network-wifi",
            SettingsPage::Bluetooth => "ms-settings:bluetooth",
            SettingsPage::Display => "ms-settings:display",
            SettingsPage::Sound => "ms-settings:sound",
            SettingsPage::Notifications => "ms-settings:notifications",
            SettingsPage::AirplaneMode => "ms-settings:network-airplanemode",
            SettingsPage::MobileHotspot => "ms-settings:network-mobilehotspot",
            SettingsPage::NightLight => "ms-settings:nightlight",
            SettingsPage::EnergySaver => "ms-settings:batterysaver",
            SettingsPage::Project => "ms-settings:display",
            SettingsPage::WindowsUpdate => "ms-settings:windowsupdate",
            SettingsPage::PrivacyLocation => "ms-settings:privacy-location",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SettingsPage::Wifi => "Open Wi-Fi settings",
            SettingsPage::Bluetooth => "Open Bluetooth settings",
            SettingsPage::Display => "Open display settings",
            SettingsPage::Sound => "Open sound settings",
            SettingsPage::Notifications => "Open notification settings",
            SettingsPage::AirplaneMode => "Open airplane mode settings",
            SettingsPage::MobileHotspot => "Open mobile hotspot settings",
            SettingsPage::NightLight => "Open night light settings",
            SettingsPage::EnergySaver => "Open battery saver settings",
            SettingsPage::Project => "Open display settings",
            SettingsPage::WindowsUpdate => "Open Windows Update",
            SettingsPage::PrivacyLocation => "Open location privacy settings",
        }
    }
}

/// Capability matrix for the native integrations surfaced in Settings/cards.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NativeCapabilities {
    pub media_sessions: Capability,
    pub audio_output: Capability,
    pub audio_volume: Capability,
    pub audio_output_switch: Capability,
    pub wifi: Capability,
    pub bluetooth: Capability,
    pub brightness: Capability,
    pub local_notifications: Capability,
    pub toast_notifications: Capability,
    pub airplane_mode: Capability,
    pub energy_saver: Capability,
    pub hotspot: Capability,
    pub cast: Capability,
    pub night_light: Capability,
}

impl NativeCapabilities {
    /// Probe every integration. Bounded (per-probe timeout) and side-effect free;
    /// call from a background thread if the caller cannot tolerate up to a few
    /// hundred milliseconds of first-call latency.
    pub fn probe() -> Self {
        #[cfg(windows)]
        {
            imp::capabilities::probe_all()
        }
        #[cfg(not(windows))]
        {
            let unsupported = || Capability::unsupported("native adapters are only supported on Windows");
            NativeCapabilities {
                media_sessions: unsupported(),
                audio_output: unsupported(),
                audio_volume: unsupported(),
                audio_output_switch: unsupported(),
                wifi: unsupported(),
                bluetooth: unsupported(),
                brightness: unsupported(),
                local_notifications: unsupported(),
                toast_notifications: unsupported(),
                airplane_mode: unsupported(),
                energy_saver: unsupported(),
                hotspot: unsupported(),
                cast: unsupported(),
                night_light: unsupported(),
            }
        }
    }
}

/// One render (output) audio endpoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Master volume/mute state of the default render endpoint.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioState {
    /// 0.0 ..= 1.0
    pub volume: f32,
    pub muted: bool,
}

/// Probe the native capability matrix (see NativeCapabilities::probe).
pub fn native_capabilities() -> NativeCapabilities {
    NativeCapabilities::probe()
}

/// Enumerate active output endpoints (friendly name + id + default marker).
pub fn list_audio_outputs() -> anyhow::Result<Vec<AudioOutputDevice>> {
    #[cfg(windows)]
    {
        imp::audio::list_output_devices()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("audio adapters are only supported on Windows")
    }
}

/// Current default output endpoint.
pub fn default_audio_output() -> anyhow::Result<AudioOutputDevice> {
    #[cfg(windows)]
    {
        imp::audio::default_output_device()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("audio adapters are only supported on Windows")
    }
}

/// Switch the default output endpoint (may be unsupported on some builds; the
/// error message tells the UI to open Windows sound settings).
pub fn set_audio_output(device_id: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::audio::set_default_output(device_id)
    }
    #[cfg(not(windows))]
    {
        let _ = device_id;
        anyhow::bail!("audio adapters are only supported on Windows")
    }
}

pub fn get_audio_volume() -> anyhow::Result<f32> {
    #[cfg(windows)]
    {
        imp::audio::get_volume()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("audio adapters are only supported on Windows")
    }
}

/// Set master volume of the default output endpoint (clamped 0.0..=1.0).
pub fn set_audio_volume(volume: f32) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::audio::set_volume(volume)
    }
    #[cfg(not(windows))]
    {
        let _ = volume;
        anyhow::bail!("audio adapters are only supported on Windows")
    }
}

pub fn get_audio_mute() -> anyhow::Result<bool> {
    #[cfg(windows)]
    {
        imp::audio::get_mute()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("audio adapters are only supported on Windows")
    }
}

pub fn set_audio_mute(muted: bool) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::audio::set_mute(muted)
    }
    #[cfg(not(windows))]
    {
        let _ = muted;
        anyhow::bail!("audio adapters are only supported on Windows")
    }
}

/// Volume + mute snapshot of the default output endpoint.
pub fn audio_state() -> anyhow::Result<AudioState> {
    #[cfg(windows)]
    {
        imp::audio::state()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("audio adapters are only supported on Windows")
    }
}

/// True when at least one monitor exposes user-level brightness control.
pub fn brightness_supported() -> bool {
    #[cfg(windows)]
    {
        imp::brightness::supported()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Descriptions of monitors that expose brightness control.
pub fn brightness_monitors() -> Vec<String> {
    #[cfg(windows)]
    {
        imp::brightness::monitors()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Current brightness (0..=100) of the first brightness-capable monitor.
pub fn get_brightness() -> anyhow::Result<u8> {
    #[cfg(windows)]
    {
        imp::brightness::get_brightness()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("brightness is only supported on Windows")
    }
}

/// Set brightness (0..=100) on every brightness-capable monitor.
pub fn set_brightness(percent: u8) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::brightness::set_brightness(percent)
    }
    #[cfg(not(windows))]
    {
        let _ = percent;
        anyhow::bail!("brightness is only supported on Windows")
    }
}

/// True when Windows accepts ProtonSearch app toasts on this OS/identity.
pub fn toast_supported() -> bool {
    #[cfg(windows)]
    {
        imp::toast::supported()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Show a local ProtonSearch toast (never reads other apps' notifications).
pub fn show_toast(title: &str, body: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::toast::show(title, body)
    }
    #[cfg(not(windows))]
    {
        let _ = (title, body);
        anyhow::bail!("toast notifications are only supported on Windows")
    }
}

/// What the system's current media session is playing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MediaSession {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    /// App user model id of the player (for example "Spotify.exe").
    pub app_id: Option<String>,
    pub playing: bool,
    pub position_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    /// The player lets other apps change its playback position.
    #[serde(default)]
    pub can_seek: bool,
    /// Session thumbnail as a `data:` URL; `None` when absent or over 512 KB.
    #[serde(default)]
    pub artwork_data_url: Option<String>,
}

/// Transport commands a media session accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    Play,
    Pause,
    PlayPause,
    Next,
    Previous,
    /// Jump to this many milliseconds from the start of the track.
    Seek(u64),
}

/// The current system media session, or `None` when nothing is playing.
pub fn media_session() -> anyhow::Result<Option<MediaSession>> {
    #[cfg(windows)]
    {
        imp::media::now_playing()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("media sessions are only supported on Windows")
    }
}

/// Send a transport command to the current system media session.
pub fn media_command(command: MediaCommand) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::media::command(command)
    }
    #[cfg(not(windows))]
    {
        let _ = command;
        anyhow::bail!("media sessions are only supported on Windows")
    }
}

/// One entry of the Windows notification feed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SystemNotification {
    /// Windows' own id for the entry (used to dismiss it).
    pub id: u32,
    /// Display name of the app that raised it; empty when Windows has none.
    pub app: String,
    pub title: String,
    pub body: String,
    /// Unix epoch milliseconds.
    pub created_ms: i64,
}

/// The newest entries of the Windows notification feed, read live.
pub fn system_notifications(limit: usize) -> anyhow::Result<Vec<SystemNotification>> {
    #[cfg(windows)]
    {
        imp::notifications::list(limit)
    }
    #[cfg(not(windows))]
    {
        let _ = limit;
        anyhow::bail!("the notification feed is only supported on Windows")
    }
}

/// Remove one entry from the Windows notification feed.
pub fn dismiss_system_notification(id: u32) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::notifications::dismiss(id)
    }
    #[cfg(not(windows))]
    {
        let _ = id;
        anyhow::bail!("the notification feed is only supported on Windows")
    }
}

/// Clear the Windows notification feed.
pub fn clear_system_notifications() -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::notifications::clear()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("the notification feed is only supported on Windows")
    }
}
// ---------------------------------------------------------------------------
// Milestone 2 native controls: Wi-Fi, Bluetooth, brightness details, settings
// fallbacks, and a ready-to-map ControlsStatus snapshot.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RadioStateInfo {
    pub available: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WifiNetworkInfo {
    pub ssid: String,
    /// 0..=100 signal quality.
    pub signal: u32,
    /// Open / WPA / WPA2 / WPA3 / Secured(...)
    pub security: String,
    pub connected: bool,
    /// A saved Windows profile exists, so connect-without-prompt is possible.
    pub has_profile: bool,
    pub connectable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WifiConnectOutcome {
    pub ssid: String,
    pub connected: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BluetoothDeviceInfo {
    pub id: String,
    pub name: String,
    pub connected: bool,
    pub paired: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrightnessMethod {
    Dxva2,
    /// DDC/CI fallback (external monitors).
    DdcCi,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BrightnessMonitorInfo {
    pub index: usize,
    pub name: String,
    pub current: u8,
    pub minimum: u32,
    pub maximum: u32,
    pub method: BrightnessMethod,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WifiState {
    pub radio: RadioStateInfo,
    pub networks: Vec<WifiNetworkInfo>,
    /// Honest failure when the scan/list was denied (e.g. location permission).
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BluetoothState {
    pub radio: RadioStateInfo,
    pub devices: Vec<BluetoothDeviceInfo>,
    /// Honest failure when device enumeration failed.
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BrightnessState {
    pub supported: bool,
    pub monitors: Vec<BrightnessMonitorInfo>,
}

/// Snapshot for the shell/UI Quick Settings card; every section is optional and
/// bounded (None = timed out or unavailable, capability explains why).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ControlsStatus {
    pub capabilities: NativeCapabilities,
    pub wifi: Option<WifiState>,
    pub bluetooth: Option<BluetoothState>,
    pub brightness: Option<BrightnessState>,
}

fn bounded_control<T, F>(probe: F, timeout: std::time::Duration) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    let (sender, receiver) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("ps-control-probe".into())
        .spawn(move || {
            let _ = sender.send(probe());
        });
    if spawned.is_err() {
        return None;
    }
    match receiver.recv_timeout(timeout) {
        Ok(Ok(value)) => Some(value),
        _ => None,
    }
}

/// Wi-Fi radio state (WinRT Radio).
pub fn wifi_radio_state() -> anyhow::Result<RadioStateInfo> {
    #[cfg(windows)]
    {
        imp::wifi::radio_state()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("Wi-Fi controls are only supported on Windows")
    }
}

/// Turn the Wi-Fi radio on/off (user-level; RadioAccessStatus is surfaced as an
/// error with the Wi-Fi settings fallback when Windows denies it).
pub fn set_wifi_radio(enabled: bool) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::wifi::set_radio(enabled)
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        anyhow::bail!("Wi-Fi controls are only supported on Windows")
    }
}

/// Available Wi-Fi networks (cached Windows scan list; may be empty briefly after
/// turning the radio on). Never triggers a blocking scan.
pub fn list_wifi_networks() -> anyhow::Result<Vec<WifiNetworkInfo>> {
    #[cfg(windows)]
    {
        imp::wifi::list_networks()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("Wi-Fi controls are only supported on Windows")
    }
}

/// Connect using an EXISTING saved Windows profile. Never creates a profile and
/// never prompts for keys: missing profiles return an error with the settings
/// fallback (ms-settings:network-wifi).
pub fn connect_wifi(ssid: &str) -> anyhow::Result<WifiConnectOutcome> {
    #[cfg(windows)]
    {
        imp::wifi::connect(ssid)
    }
    #[cfg(not(windows))]
    {
        let _ = ssid;
        anyhow::bail!("Wi-Fi controls are only supported on Windows")
    }
}

/// WLAN adapter descriptions (diagnostics/UI detail line).
pub fn wlan_adapters() -> Vec<String> {
    #[cfg(windows)]
    {
        imp::wifi::interface_descriptions()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

pub fn bluetooth_radio_state() -> anyhow::Result<RadioStateInfo> {
    #[cfg(windows)]
    {
        imp::bluetooth::radio_state()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("Bluetooth controls are only supported on Windows")
    }
}

/// Turn the Bluetooth radio on/off (user-level; denials surface as errors).
pub fn set_bluetooth_radio(enabled: bool) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::bluetooth::set_radio(enabled)
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        anyhow::bail!("Bluetooth controls are only supported on Windows")
    }
}

/// Paired Bluetooth devices (read-only; pairing/connection stays in Windows).
pub fn list_bluetooth_devices() -> anyhow::Result<Vec<BluetoothDeviceInfo>> {
    #[cfg(windows)]
    {
        imp::bluetooth::paired_devices()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("Bluetooth controls are only supported on Windows")
    }
}

/// Per-monitor brightness details incl. control method (Dxva2 or DDC/CI).
pub fn list_brightness_monitors() -> Vec<BrightnessMonitorInfo> {
    #[cfg(windows)]
    {
        imp::brightness::list_monitors()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Open one of the allow-listed Windows Settings pages.
pub fn open_settings_page(page: SettingsPage) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        imp::settings_pages::open(page)
    }
    #[cfg(not(windows))]
    {
        let _ = page;
        anyhow::bail!("Windows Settings pages are only available on Windows")
    }
}

/// Bounded Quick Settings snapshot for the shell/UI.
pub fn controls_status() -> ControlsStatus {
    let capabilities = NativeCapabilities::probe();
    let timeout = std::time::Duration::from_secs(8);
    let wifi = if capabilities.wifi.supported {
        bounded_control(
            || {
                let radio = wifi_radio_state()?;
                match list_wifi_networks() {
                    Ok(networks) => Ok(WifiState {
                        radio,
                        networks,
                        error: None,
                    }),
                    Err(error) => Ok(WifiState {
                        radio,
                        networks: Vec::new(),
                        error: Some(format!("{error:#}")),
                    }),
                }
            },
            timeout,
        )
    } else {
        None
    };
    let bluetooth = if capabilities.bluetooth.supported {
        bounded_control(
            || {
                let radio = bluetooth_radio_state()?;
                match list_bluetooth_devices() {
                    Ok(devices) => Ok(BluetoothState {
                        radio,
                        devices,
                        error: None,
                    }),
                    Err(error) => Ok(BluetoothState {
                        radio,
                        devices: Vec::new(),
                        error: Some(format!("{error:#}")),
                    }),
                }
            },
            timeout,
        )
    } else {
        None
    };
    let brightness = if capabilities.brightness.supported {
        bounded_control(
            || {
                Ok(BrightnessState {
                    supported: true,
                    monitors: list_brightness_monitors(),
                })
            },
            timeout,
        )
    } else {
        None
    };
    ControlsStatus {
        capabilities,
        wifi,
        bluetooth,
        brightness,
    }
}
