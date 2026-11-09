//! Gate 0 audit: which clipboard image formats do the production ps-windows
//! APIs detect/decode? Run from the harness directory:
//!
//!   cargo run --bin format_audit -- --evidence ../../../docs/qa/clipboard-audit/evidence/decoded
//!
//! Modes:
//!   default        full synthetic format matrix + listener snapshot matrix
//!   --capture-only read the CURRENT clipboard once (for externally-set data such
//!                  as PowerShell System.Windows.Forms.Clipboard.SetImage)

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use clipboard_audit::{
    clear, enumerate_formats, ensure_dir, observe_clipboard, save_png, set_bitmap_only, set_dib,
    set_dib_plus_text, set_dibv5, set_hdrop_files, set_registered_format, set_text, R,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let evidence = args
        .iter()
        .position(|arg| arg == "--evidence")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from);
    if let Some(dir) = &evidence {
        let _ = ensure_dir(dir);
    }
    let capture_only = args.iter().any(|arg| arg == "--capture-only");
    let only = args
        .iter()
        .position(|arg| arg == "--only")
        .and_then(|index| args.get(index + 1))
        .cloned();

    if let Some(set_name) = args
        .iter()
        .position(|arg| arg == "--set")
        .and_then(|index| args.get(index + 1))
        .cloned()
    {
        let scenario = scenarios()
            .into_iter()
            .chain(listener_scenarios())
            .find(|scenario| scenario.name == set_name)
            .expect("unknown scenario name");
        match (scenario.apply)() {
            Ok(()) => println!("set {}: ok", scenario.name),
            Err(error) => println!("set {}: FAILED ({error})", scenario.name),
        }
        clipboard_audit::dump_raw_formats();
        report_current(&evidence, &format!("set-{}", scenario.name));
        return;
    }
    if args.iter().any(|arg| arg == "--dump-current") {
        clipboard_audit::dump_raw_formats();
        report_current(&evidence, "current-clipboard");
        return;
    }
    if capture_only {
        report_current(&evidence, "current-clipboard");
        return;
    }

    let listener = start_listener_probe();
    println!("# synthetic format matrix (production ps-windows API)");
    for scenario in scenarios() {
        if only.as_deref().map(|name| name != scenario.name).unwrap_or(false) {
            continue;
        }
        run_scenario(&scenario, &evidence, &listener);
    }
    println!("# listener snapshot matrix (production ps_windows::ClipboardListener)");
    for scenario in listener_scenarios() {
        if only.as_deref().map(|name| name != scenario.name).unwrap_or(false) {
            continue;
        }
        run_scenario(&scenario, &evidence, &listener);
    }
    println!("done");
}

struct Scenario {
    name: &'static str,
    note: &'static str,
    apply: Box<dyn Fn() -> R<()>>,
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "dib32_bottomup_alpha255",
            note: "typical 32bpp CF_DIB, opaque alpha",
            apply: Box::new(|| set_dib(64, 48, 32, true, 255)),
        },
        Scenario {
            name: "dib32_bottomup_alpha0",
            note: "32bpp CF_DIB with alpha=0 (GDI convention; decoder must force opaque)",
            apply: Box::new(|| set_dib(64, 48, 32, true, 0)),
        },
        Scenario {
            name: "dib24_bottomup",
            note: "24bpp CF_DIB (classic screenshot/browser format)",
            apply: Box::new(|| set_dib(64, 48, 24, true, 255)),
        },
        Scenario {
            name: "dib32_topdown",
            note: "32bpp CF_DIB with negative height (top-down rows)",
            apply: Box::new(|| set_dib(64, 48, 32, false, 255)),
        },
        Scenario {
            name: "dib24_odd_width",
            note: "24bpp CF_DIB width 7 (odd row stride)",
            apply: Box::new(|| set_dib(7, 5, 24, true, 255)),
        },
        Scenario {
            name: "dib8_palette",
            note: "8bpp indexed CF_DIB (palette bitmap)",
            apply: Box::new(|| {
                // Minimal 8bpp payload: header + 256-entry palette + pixels.
                let width = 8i32;
                let height = 4i32;
                let mut bytes = Vec::new();
                let header = windows::Win32::Graphics::Gdi::BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<windows::Win32::Graphics::Gdi::BITMAPINFOHEADER>()
                        as u32,
                    biWidth: width,
                    biHeight: height,
                    biPlanes: 1,
                    biBitCount: 8,
                    biCompression: 0,
                    biSizeImage: 0,
                    ..Default::default()
                };
                bytes.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(
                        &header as *const _ as *const u8,
                        std::mem::size_of_val(&header),
                    )
                });
                bytes.extend(std::iter::repeat(0u8).take(256 * 4));
                bytes.extend(std::iter::repeat(7u8).take(((width as usize).div_ceil(4)) * 4 * height as usize));
                clipboard_audit::set_registered_format("AUDIT_FAKE_DIB8", &bytes)
            }),
        },
        Scenario {
            name: "dibv5_bitfields",
            note: "BITMAPV5HEADER + BI_BITFIELDS 32bpp (modern apps)",
            apply: Box::new(|| set_dibv5(64, 48, true)),
        },
        Scenario {
            name: "dibv5_rgb",
            note: "BITMAPV5HEADER + BI_RGB 32bpp",
            apply: Box::new(|| set_dibv5(64, 48, false)),
        },
        Scenario {
            name: "bitmap_only",
            note: "CF_BITMAP handle only (no CF_DIB)",
            apply: Box::new(|| set_bitmap_only(48, 32)),
        },
        Scenario {
            name: "registered_png_only",
            note: "registered \"PNG\" format only (browsers/some editors)",
            apply: Box::new(|| {
                let png = clipboard_audit::png_bytes(80, 60);
                set_registered_format("PNG", &png)
            }),
        },
        Scenario {
            name: "registered_image_png_only",
            note: "registered \"image/png\" format only",
            apply: Box::new(|| {
                let png = clipboard_audit::png_bytes(80, 60);
                set_registered_format("image/png", &png)
            }),
        },
        Scenario {
            name: "dib_plus_text",
            note: "same copy exposes CF_DIB and CF_UNICODETEXT (mixed representation)",
            apply: Box::new(|| set_dib_plus_text("mixed image+text copy", 64, 48, 32)),
        },
        Scenario {
            name: "hdrop_files_plus_text",
            note: "Explorer-style CF_HDROP file list + text (must not be an image bitmap)",
            apply: Box::new(|| {
                set_hdrop_files(&[PathBuf::from(r"C:\Windows\win.ini")])
            }),
        },
        Scenario {
            name: "text_only",
            note: "control: plain text",
            apply: Box::new(|| set_text("plain audit text")),
        },
        Scenario {
            name: "empty_clipboard",
            note: "empty clipboard",
            apply: Box::new(|| clear()),
        },
    ]
}

/// Listener scenarios include the mixed/multiple-format cases that the
/// production snapshot logic must negotiate.
fn listener_scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "listener_dib32",
            note: "single CF_DIB copy",
            apply: Box::new(|| set_dib(64, 48, 32, true, 255)),
        },
        Scenario {
            name: "listener_dib_plus_text",
            note: "CF_DIB + text in one copy: snapshot prefers text (image lost?)",
            apply: Box::new(|| set_dib_plus_text("mixed image+text copy", 64, 48, 32)),
        },
        Scenario {
            name: "listener_dibv5",
            note: "CF_DIBV5 only",
            apply: Box::new(|| set_dibv5(64, 48, true)),
        },
        Scenario {
            name: "listener_bitmap_only",
            note: "CF_BITMAP only",
            apply: Box::new(|| set_bitmap_only(48, 32)),
        },
        Scenario {
            name: "listener_png_registered",
            note: "registered PNG only",
            apply: Box::new(|| set_registered_format("PNG", &clipboard_audit::png_bytes(80, 60))),
        },
        Scenario {
            name: "listener_hdrop",
            note: "CF_HDROP + text only",
            apply: Box::new(|| set_hdrop_files(&[PathBuf::from(r"C:\Windows\win.ini")])),
        },
    ]
}

fn run_scenario(scenario: &Scenario, evidence: &Option<PathBuf>, listener: &ListenerProbe) {
    // Drain any stale listener events first.
    while listener.rx.try_recv().is_ok() {}
    let apply = (scenario.apply)();
    println!("=== {}", scenario.name);
    println!("note: {}", scenario.note);
    match &apply {
        Ok(()) => println!("set: ok"),
        Err(error) => {
            println!("set: FAILED ({error})");
            return;
        }
    }
    // Wait for the listener snapshot first: the listener thread must open the
    // clipboard itself, so avoid contending with the main-thread reads below.
    let listener_event = listener.wait(Duration::from_millis(3000));
    let formats = enumerate_formats();
    println!(
        "formats: {}",
        formats
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let observed = observe_clipboard(evidence.as_deref(), scenario.name);
    match &observed.text {
        Some(text) => println!("text: Some({:?})", truncate(text, 60)),
        None => println!("text: None"),
    }
    match &observed.image {
        Some(image) => println!("get_image: Some({image})"),
        None => println!("get_image: None"),
    }
    match &observed.png {
        Some((bytes, dims)) => println!("get_image_png: Some(bytes={bytes}, {dims})"),
        None => println!("get_image_png: None"),
    }
    match listener_event {
        Some(snapshot) => {
            let text = snapshot
                .text
                .as_deref()
                .map(|value| format!("Some({:?})", truncate(value, 60)))
                .unwrap_or_else(|| "None".to_string());
            let png = snapshot
                .png
                .as_deref()
                .map(|bytes| {
                    let dims = image::load_from_memory(bytes)
                        .map(|decoded| format!("{}x{}", decoded.width(), decoded.height()))
                        .unwrap_or_else(|_| "decode-failed".to_string());
                    format!("Some(bytes={}, {dims})", bytes.len())
                })
                .unwrap_or_else(|| "None".to_string());
            println!("listener_snapshot: text={text} png={png}");
            if let (Some(evidence), Some(png)) = (evidence, snapshot.png.as_deref()) {
                let _ = save_png(&evidence.join(format!("listener-{}.png", scenario.name)), png);
            }
        }
        None => println!("listener_snapshot: NO WM_CLIPBOARDUPDATE within 1200ms"),
    }
    println!();
}

fn report_current(evidence: &Option<PathBuf>, label: &str) {
    println!("=== {label}");
    let formats = enumerate_formats();
    println!(
        "formats: {}",
        formats
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let observed = observe_clipboard(evidence.as_deref(), label);
    match &observed.text {
        Some(text) => println!("text: Some({:?})", truncate(text, 120)),
        None => println!("text: None"),
    }
    match &observed.image {
        Some(image) => println!("get_image: Some({image})"),
        None => println!("get_image: None"),
    }
    match &observed.png {
        Some((bytes, dims)) => println!("get_image_png: Some(bytes={bytes}, {dims})"),
        None => println!("get_image_png: None"),
    }
}

fn truncate(value: &str, limit: usize) -> String {
    let mut out: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        out.push('…');
    }
    out
}

struct ListenerProbe {
    rx: mpsc::Receiver<ps_windows::ClipboardSnapshot>,
    _listener: ps_windows::ClipboardListener,
}

impl ListenerProbe {
    fn wait(&self, timeout: Duration) -> Option<ps_windows::ClipboardSnapshot> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match self.rx.recv_timeout(remaining) {
                Ok(snapshot) => return Some(snapshot),
                Err(mpsc::RecvTimeoutError::Timeout) => return None,
                Err(mpsc::RecvTimeoutError::Disconnected) => return None,
            }
        }
    }
}

fn start_listener_probe() -> ListenerProbe {
    let (tx, rx) = mpsc::channel();
    let listener = ps_windows::ClipboardListener::start(move |snapshot| {
        let _ = tx.send(snapshot);
    })
    .expect("clipboard listener must start");
    ListenerProbe {
        rx,
        _listener: listener,
    }
}