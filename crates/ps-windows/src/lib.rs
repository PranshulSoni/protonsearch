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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardSnapshot {
    pub text: Option<String>,
    /// PNG encoded image, when the clipboard held a bitmap.
    pub png: Option<Vec<u8>>,
    /// Executable name (no path) of the app that was foreground when the
    /// clipboard changed, when it can be determined.
    pub source_app: Option<String>,
    /// Unix milliseconds at capture time.
    pub timestamp_ms: i64,
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