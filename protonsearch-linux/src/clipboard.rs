//! Internal Linux clipboard monitoring and persistence.
//!
//! GDK owns the platform-specific clipboard integration. That means the same
//! service works through GTK's Wayland and X11 backends without requiring a
//! separate clipboard daemon such as cliphist, Klipper, or CopyQ.

use crate::system;
use crate::xdg::XdgPaths;
use anyhow::{Context, Result};
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 200;
const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardEntry {
    pub id: u64,
    pub kind: String,
    pub text: Option<String>,
    pub image_file: Option<String>,
    pub mime_type: Option<String>,
    pub fingerprint: String,
    pub created_at: u64,
}

impl ClipboardEntry {
    pub fn is_image(&self) -> bool {
        self.kind == "image"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardStatus {
    pub available: bool,
    pub backend: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    entries: Vec<ClipboardEntry>,
}

enum PendingClipboard {
    Text(String),
    Image(Vec<u8>),
}

pub struct ClipboardMonitor {
    _clipboard: gdk::Clipboard,
    _handlers: Vec<glib::SignalHandlerId>,
    _watchers: Vec<NativeWatcher>,
    _worker: Option<thread::JoinHandle<()>>,
}

struct NativeWatcher {
    child: Child,
}

impl Drop for NativeWatcher {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ClipboardMonitor {
    fn drop(&mut self) {
        // The callback owns the sender that keeps the storage worker alive.
        // Disconnecting it lets that worker finish as soon as any in-flight
        // GDK read callback returns, instead of leaving a monitor attached to
        // the display after the launcher exits.
        for handler in self._handlers.drain(..) {
            self._clipboard.disconnect(handler);
        }
    }
}

pub fn start(paths: XdgPaths) -> Result<ClipboardMonitor> {
    let display = gdk::Display::default()
        .context("no graphical display is available for clipboard monitoring")?;
    let clipboard = display.clipboard();
    let backend = display_backend_for(&display);
    let watchers = start_wayland_watchers();
    write_status(
        &paths,
        &ClipboardStatus {
            available: true,
            backend: backend.clone(),
            message: format!("Clipboard monitoring active through {backend}"),
        },
    )?;

    let (sender, receiver) = mpsc::channel::<PendingClipboard>();
    let worker_paths = paths.clone();
    let worker = thread::Builder::new()
        .name("protonsearch-clipboard-store".to_string())
        .spawn(move || {
            while let Ok(pending) = receiver.recv() {
                match persist_pending(&worker_paths, pending) {
                    Ok(()) => {
                        let _ = write_status(
                            &worker_paths,
                            &ClipboardStatus {
                                available: true,
                                backend: display_backend(),
                                message: "Clipboard monitoring active".to_string(),
                            },
                        );
                    }
                    Err(error) => {
                        let _ = write_status(
                            &worker_paths,
                            &ClipboardStatus {
                                available: false,
                                backend: display_backend(),
                                message: format!("Could not save clipboard history: {error}"),
                            },
                        );
                        eprintln!("ProtonSearch: clipboard history write failed: {error:#}");
                    }
                }
            }
        })?;

    let paths_for_signal = paths.clone();
    let sender_for_signal = sender.clone();
    let capture = move |clipboard: &gdk::Clipboard| {
        capture_current(
            clipboard.clone(),
            sender_for_signal.clone(),
            paths_for_signal.clone(),
        );
    };
    let changed_handler = clipboard.connect_changed(capture);
    let paths_for_notify = paths.clone();
    let sender_for_notify = sender.clone();
    let content_handler = clipboard.connect_content_notify(move |clipboard| {
        capture_current(
            clipboard.clone(),
            sender_for_notify.clone(),
            paths_for_notify.clone(),
        );
    });

    // Capture a clipboard item that was copied before ProtonSearch started.
    capture_current(clipboard.clone(), sender, paths);

    Ok(ClipboardMonitor {
        _clipboard: clipboard,
        _handlers: vec![changed_handler, content_handler],
        _watchers: watchers,
        _worker: Some(worker),
    })
}

pub fn ingest_text(paths: &XdgPaths, bytes: &[u8]) -> Result<()> {
    let text = String::from_utf8(bytes.to_vec()).context("clipboard text was not valid UTF-8")?;
    if text.is_empty() || text.len() > MAX_TEXT_BYTES {
        return Ok(());
    }
    persist_pending(paths, PendingClipboard::Text(text))
}

pub fn ingest_image(paths: &XdgPaths, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Ok(());
    }
    persist_pending(paths, PendingClipboard::Image(bytes.to_vec()))
}

fn start_wayland_watchers() -> Vec<NativeWatcher> {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() || !system::command_available("wl-paste") {
        return Vec::new();
    }
    let Ok(executable) = std::env::current_exe() else {
        return Vec::new();
    };
    [
        ("text/plain", "clipboard-capture-text"),
        ("image/png", "clipboard-capture-image"),
    ]
    .into_iter()
    .filter_map(|(mime_type, mode)| {
        Command::new("wl-paste")
            .args(["--type", mime_type, "--watch"])
            .arg(&executable)
            .arg(mode)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|child| NativeWatcher { child })
            .map_err(|error| {
                eprintln!("ProtonSearch: could not start {mime_type} clipboard watcher: {error}");
            })
            .ok()
    })
    .collect()
}

pub fn status(paths: &XdgPaths) -> ClipboardStatus {
    let path = status_path(paths);
    fs::read(&path)
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or_else(|| ClipboardStatus {
            available: false,
            backend: display_backend(),
            message: "Clipboard monitoring has not started".to_string(),
        })
}

pub fn mark_unavailable(paths: &XdgPaths, message: impl Into<String>) {
    let _ = write_status(
        paths,
        &ClipboardStatus {
            available: false,
            backend: display_backend(),
            message: message.into(),
        },
    );
}

pub fn history(paths: &XdgPaths) -> Result<Vec<ClipboardEntry>> {
    let path = history_path(paths);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    let file: HistoryFile = match serde_json::from_slice(&contents) {
        Ok(file) => file,
        Err(error) => {
            let backup = path.with_extension("json.corrupt");
            let _ = fs::rename(&path, backup);
            return Err(error).context("clipboard history database is corrupted");
        }
    };
    Ok(file.entries)
}

pub fn entry(paths: &XdgPaths, id: u64) -> Result<ClipboardEntry> {
    history(paths)?
        .into_iter()
        .find(|entry| entry.id == id)
        .with_context(|| format!("clipboard item {id} no longer exists"))
}

pub fn image_bytes(paths: &XdgPaths, id: u64) -> Result<Vec<u8>> {
    let item = entry(paths, id)?;
    let relative = item
        .image_file
        .context("clipboard item does not contain an image")?;
    let path = safe_image_path(paths, &relative)?;
    Ok(fs::read(path)?)
}

pub fn image_mime_type(paths: &XdgPaths, id: u64) -> Result<String> {
    Ok(entry(paths, id)?
        .mime_type
        .unwrap_or_else(|| "image/png".to_string()))
}

pub fn text(paths: &XdgPaths, id: u64) -> Result<String> {
    entry(paths, id)?
        .text
        .context("clipboard item does not contain text")
}

fn capture_current(
    clipboard: gdk::Clipboard,
    sender: mpsc::Sender<PendingClipboard>,
    paths: XdgPaths,
) {
    let clipboard_for_texture = clipboard.clone();
    let paths_for_text = paths.clone();
    clipboard.read_text_async(None::<&gio::Cancellable>, move |result| {
        let read_image = |sender: mpsc::Sender<PendingClipboard>, paths: XdgPaths| {
            let _ = write_status(
                &paths,
                &ClipboardStatus {
                    available: true,
                    backend: display_backend(),
                    message: "Clipboard changed; checking for image data".to_string(),
                },
            );
            clipboard_for_texture.read_texture_async(None::<&gio::Cancellable>, move |result| {
                match result {
                    Ok(Some(texture)) => {
                        let bytes = texture.save_to_png_bytes().to_vec();
                        if !bytes.is_empty() && bytes.len() <= MAX_IMAGE_BYTES {
                            let _ = sender.send(PendingClipboard::Image(bytes));
                        }
                    }
                    Ok(None) => {}
                    Err(error) => eprintln!("ProtonSearch: clipboard image read failed: {error}"),
                }
            });
        };
        match result {
            Ok(Some(text)) if !text.is_empty() && text.len() <= MAX_TEXT_BYTES => {
                let _ = sender.send(PendingClipboard::Text(text.to_string()));
            }
            Ok(Some(_)) => {}
            Ok(None) => {
                fallback_text(sender.clone(), paths_for_text.clone());
                read_image(sender.clone(), paths_for_text.clone());
            }
            Err(error) => {
                eprintln!("ProtonSearch: clipboard text read failed: {error}");
                fallback_text(sender.clone(), paths_for_text.clone());
                read_image(sender.clone(), paths_for_text.clone());
            }
        }
    });
}

fn fallback_text(sender: mpsc::Sender<PendingClipboard>, paths: XdgPaths) {
    thread::spawn(move || {
        let mut last_result = None;
        for attempt in 0..3 {
            // Some compositors emit the GDK change signal just before the
            // new offer is queryable. These three bounded attempts handle
            // that hand-off without creating a polling loop.
            if attempt > 0 {
                thread::sleep(std::time::Duration::from_millis(500));
            }
            let result = if std::env::var_os("WAYLAND_DISPLAY").is_some()
                && system::command_available("wl-paste")
            {
                system::run_with_timeout(
                    "wl-paste",
                    &["--no-newline"],
                    std::time::Duration::from_secs(2),
                )
            } else if std::env::var_os("DISPLAY").is_some() && system::command_available("xclip") {
                system::run_with_timeout(
                    "xclip",
                    &["-selection", "clipboard", "-o"],
                    std::time::Duration::from_secs(2),
                )
            } else {
                return;
            };
            let Ok(result) = result else { continue };
            if result.status == Some(0)
                && !result.timed_out
                && !result.stdout.is_empty()
                && result.stdout.len() <= MAX_TEXT_BYTES
            {
                let _ = write_status(
                    &paths,
                    &ClipboardStatus {
                        available: true,
                        backend: display_backend(),
                        message: "Clipboard text captured".to_string(),
                    },
                );
                let _ = sender.send(PendingClipboard::Text(result.stdout));
                return;
            }
            last_result = Some(result);
        }
        if last_result.is_some() {
            let _ = write_status(
                &paths,
                &ClipboardStatus {
                    available: true,
                    backend: display_backend(),
                    message: "Clipboard monitoring active".to_string(),
                },
            );
        }
    });
}

fn persist_pending(paths: &XdgPaths, pending: PendingClipboard) -> Result<()> {
    let mut entries = history(paths).unwrap_or_default();
    let (kind, text, image_bytes, mime_type, fingerprint) = match pending {
        PendingClipboard::Text(text) => {
            let fingerprint = format!("text:{:016x}", hash_bytes(text.as_bytes()));
            ("text", Some(text), None, None, fingerprint)
        }
        PendingClipboard::Image(bytes) => {
            let fingerprint = format!("image:{:016x}", hash_bytes(&bytes));
            (
                "image",
                None,
                Some(bytes),
                Some("image/png".to_string()),
                fingerprint,
            )
        }
    };

    let old_image_files = entries
        .iter()
        .filter(|entry| entry.fingerprint == fingerprint)
        .filter_map(|entry| entry.image_file.clone())
        .collect::<Vec<_>>();
    entries.retain(|entry| entry.fingerprint != fingerprint);

    let next_id = entries
        .iter()
        .map(|entry| entry.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let image_file = image_bytes
        .as_ref()
        .map(|_| format!("images/{next_id}.png"));
    let item = ClipboardEntry {
        id: next_id,
        kind: kind.to_string(),
        text,
        image_file: image_file.clone(),
        mime_type,
        fingerprint,
        created_at: now_millis(),
    };

    if let Some(bytes) = image_bytes {
        let relative = image_file.as_deref().context("image file name missing")?;
        let image_path = safe_image_path(paths, relative)?;
        let parent = image_path.parent().context("image directory missing")?;
        fs::create_dir_all(parent)?;
        let temporary = image_path.with_extension("png.new");
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, image_path)?;
    }

    entries.insert(0, item);
    let removed = if entries.len() > MAX_ENTRIES {
        entries.split_off(MAX_ENTRIES)
    } else {
        Vec::new()
    };
    for entry in old_image_files
        .into_iter()
        .chain(removed.into_iter().filter_map(|entry| entry.image_file))
    {
        if let Ok(path) = safe_image_path(paths, &entry) {
            let _ = fs::remove_file(path);
        }
    }
    write_history(paths, &entries)
}

fn write_history(paths: &XdgPaths, entries: &[ClipboardEntry]) -> Result<()> {
    let path = history_path(paths);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_vec_pretty(&HistoryFile {
        version: 1,
        entries: entries.to_vec(),
    })?;
    let temporary = path.with_extension("json.new");
    fs::write(&temporary, contents)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn write_status(paths: &XdgPaths, status: &ClipboardStatus) -> Result<()> {
    let path = status_path(paths);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_vec_pretty(status)?;
    let temporary = path.with_extension("json.new");
    fs::write(&temporary, contents)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn history_path(paths: &XdgPaths) -> PathBuf {
    paths.state_dir().join("clipboard/history.json")
}

fn status_path(paths: &XdgPaths) -> PathBuf {
    paths.state_dir().join("clipboard/status.json")
}

fn safe_image_path(paths: &XdgPaths, relative: &str) -> Result<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        anyhow::bail!("invalid clipboard image path");
    }
    Ok(paths.state_dir().join("clipboard").join(relative))
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn display_backend() -> String {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        "Wayland/GDK".to_string()
    } else if std::env::var_os("DISPLAY").is_some() {
        "X11/GDK".to_string()
    } else {
        "GDK".to_string()
    }
}

fn display_backend_for(display: &gdk::Display) -> String {
    if display.backend().is_wayland() {
        "Wayland/GDK".to_string()
    } else if display.backend().is_x11() {
        "X11/GDK".to_string()
    } else {
        "Other/GDK".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_paths(name: &str) -> XdgPaths {
        let root = std::env::temp_dir().join(format!(
            "protonsearch-clipboard-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        XdgPaths {
            home: root.join("home"),
            config: root.join("config"),
            data: root.join("data"),
            state: root.join("state"),
            cache: root.join("cache"),
            runtime: None,
        }
    }

    #[test]
    fn text_history_is_persistent_deduplicated_and_bounded() {
        let paths = test_paths("text");
        persist_pending(&paths, PendingClipboard::Text("hello".to_string())).unwrap();
        persist_pending(&paths, PendingClipboard::Text("world".to_string())).unwrap();
        persist_pending(&paths, PendingClipboard::Text("hello".to_string())).unwrap();

        let entries = history(&paths).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text.as_deref(), Some("hello"));
        assert_eq!(text(&paths, entries[1].id).unwrap(), "world");
        let _ = fs::remove_dir_all(paths.state);
    }

    #[test]
    fn image_history_persists_bytes_without_exposing_binary_metadata() {
        let paths = test_paths("image");
        let bytes = vec![137, 80, 78, 71, 13, 10, 26, 10, 1, 2, 3];
        persist_pending(&paths, PendingClipboard::Image(bytes.clone())).unwrap();

        let item = history(&paths).unwrap().pop().unwrap();
        assert!(item.is_image());
        assert!(item.text.is_none());
        assert_eq!(item.mime_type.as_deref(), Some("image/png"));
        assert_eq!(image_bytes(&paths, item.id).unwrap(), bytes);
        let _ = fs::remove_dir_all(paths.state);
    }

    #[test]
    fn corrupt_history_is_backed_up_and_reported() {
        let paths = test_paths("corrupt");
        let path = history_path(&paths);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not-json").unwrap();

        assert!(history(&paths).is_err());
        assert!(!path.exists());
        assert!(path.with_extension("json.corrupt").exists());
        let _ = fs::remove_dir_all(paths.state);
    }
}
