//! Linux StatusNotifierItem tray integration.
//!
//! The tray is intentionally separate from GTK. Wayland desktops such as
//! Hyprland commonly expose the freedesktop StatusNotifierItem protocol
//! through Waybar, while the launcher itself remains a resident GTK process.

use crate::system;
use crate::xdg::XdgPaths;
use gdk_pixbuf::Pixbuf;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, Tray};
use std::io::Cursor;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

const PROTONSEARCH_ICON: &[u8] = include_bytes!("../assets/branding/protonsearch.png");

pub struct ProtonTray {
    socket: PathBuf,
    executable: PathBuf,
}

impl ProtonTray {
    pub fn start(paths: &XdgPaths) -> Option<Handle<Self>> {
        let tray = Self {
            socket: ipc_socket(paths),
            executable: std::env::current_exe().ok()?,
        };
        match tray.assume_sni_available(true).spawn() {
            Ok(handle) => Some(handle),
            Err(error) => {
                eprintln!("ProtonSearch: tray unavailable: {error}");
                None
            }
        }
    }

    fn open(&self) {
        notify(&self.socket, "toggle");
    }

    fn open_settings(&self) {
        if let Err(error) = system::spawn_detached(&self.executable, &["settings-ui"]) {
            eprintln!("ProtonSearch: could not open settings: {error:#}");
        }
    }
}

impl Tray for ProtonTray {
    fn id(&self) -> String {
        "protonsearch-linux".to_string()
    }

    fn title(&self) -> String {
        "ProtonSearch".to_string()
    }

    fn icon_name(&self) -> String {
        String::new()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let Ok(pixbuf) = Pixbuf::from_read(Cursor::new(PROTONSEARCH_ICON)) else {
            return Vec::new();
        };
        let width = pixbuf.width();
        let height = pixbuf.height();
        let channels = pixbuf.n_channels();
        let rowstride = pixbuf.rowstride() as usize;
        let bytes = pixbuf.read_pixel_bytes();
        let pixels = bytes.as_ref();
        let mut data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height as usize {
            let row = &pixels[y * rowstride..];
            for x in 0..width as usize {
                let offset = x * channels as usize;
                let red = row[offset];
                let green = row[offset + 1];
                let blue = row[offset + 2];
                let alpha = if channels >= 4 { row[offset + 3] } else { 255 };
                data.extend_from_slice(&[alpha, red, green, blue]);
            }
        }

        vec![Icon {
            width,
            height,
            data,
        }]
    }

    fn icon_theme_path(&self) -> String {
        std::env::var_os("HOME")
            .map(|home| {
                std::path::PathBuf::from(home)
                    .join(".local/share/icons")
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_default()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.open();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let open = StandardItem {
            label: "Open".to_string(),
            icon_name: "protonsearch".to_string(),
            activate: Box::new(|tray: &mut Self| tray.open()),
            ..Default::default()
        };
        let settings = StandardItem {
            label: "Open Settings".to_string(),
            icon_name: "preferences-system-symbolic".to_string(),
            activate: Box::new(|tray: &mut Self| tray.open_settings()),
            ..Default::default()
        };
        vec![open.into(), settings.into()]
    }
}

fn ipc_socket(paths: &XdgPaths) -> PathBuf {
    paths
        .runtime
        .as_ref()
        .map(|runtime| runtime.join("protonsearch/launcher.sock"))
        .unwrap_or_else(|| paths.state_dir().join("launcher.sock"))
}

fn notify(socket: &PathBuf, command: &str) {
    let Ok(mut stream) = UnixStream::connect(socket) else {
        return;
    };
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    let _ = stream.write_all(format!("{command}\n").as_bytes());
    let _ = stream.shutdown(std::net::Shutdown::Both);
}
