//! GTK launcher window for the Linux implementation.
//!
//! Wayland deliberately leaves global shortcut ownership to the compositor.
//! On Hyprland the window is opened with a normal `bind` command documented in
//! `docs/linux/LAUNCHER.md`; the same desktop entry can be assigned a shortcut
//! by other desktop environments.

use crate::providers::{self, Item, Target};
use crate::settings;
use crate::xdg::XdgPaths;
use anyhow::Result;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType, CheckButton,
    ComboBoxText, Entry, EventControllerKey, Image, Label, ListBox, ListBoxRow, MessageDialog,
    MessageType, Orientation, PolicyType, PropagationPhase, ResponseType, Revealer,
    RevealerTransitionType, ScrolledWindow, SelectionMode, SpinButton, Stack, StackSidebar,
    StackTransitionType, TextView, WrapMode,
};
use std::cell::{Cell, RefCell};
use std::fs;
use std::io::{BufRead, Cursor, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

const APPLICATION_ID: &str = "com.protonsearch.Linux";
const LAUNCHER_CSS: &str = r#"
window.proton-window {
    background-color: rgba(31, 32, 34, 0.98);
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 12px;
    font-family: sans;
}

.launcher-root {
    background-color: transparent;
}

.search-shell {
    background-color: #2b2c2e;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    padding: 0 10px;
}

.search-icon {
    color: #c6c9cc;
}

entry.search-entry {
    min-height: 42px;
    background-color: transparent;
    color: #f4f5f6;
    caret-color: #f4f5f6;
    border: none;
    box-shadow: none;
    padding: 0 6px;
    font-size: 15px;
}

entry.search-entry:focus {
    border: none;
    box-shadow: none;
}

entry.error {
    border: 1px solid #ef6b73;
}

.category-row {
    margin-top: 2px;
    margin-bottom: 2px;
}

.category-chip {
    color: #979b9f;
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
}

.category-chip.active {
    color: #f2f3f4;
    background-color: #4b4d50;
}

.status-label {
    color: #85898d;
    font-size: 11px;
}

list.result-list {
    background-color: transparent;
}

row.result-row {
    background-color: transparent;
    border-radius: 7px;
    margin: 1px 0;
}

row.result-row:hover {
    background-color: #3b3d40;
}

row.result-row:selected {
    background-color: #4b4d50;
}

.result-icon {
    margin-right: 10px;
}

.result-thumbnail {
    min-width: 48px;
    min-height: 48px;
    border-radius: 6px;
}

.image-preview {
    background-color: rgba(17, 18, 19, 0.96);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    padding: 10px;
    margin-bottom: 4px;
}

.preview-image {
    min-width: 180px;
    min-height: 120px;
    border-radius: 6px;
}

.preview-title {
    color: #f1f2f3;
    font-size: 12px;
    font-weight: 600;
}

.preview-hint, .empty-state-hint {
    color: #85898d;
    font-size: 10px;
}

.empty-state-title {
    color: #c6c9cc;
    font-size: 14px;
    font-weight: 600;
}

.asset-icon {
    min-width: 32px;
    min-height: 32px;
}

.result-title {
    color: #f1f2f3;
    font-size: 13px;
    font-weight: 600;
}

.result-subtitle {
    color: #8f9498;
    font-size: 11px;
}

.source-badge {
    color: #bfc3c6;
    background-color: rgba(255, 255, 255, 0.09);
    border-radius: 999px;
    padding: 3px 8px;
    font-size: 9px;
    font-weight: 700;
}

.footer-hint {
    color: #777c80;
    font-size: 10px;
}

window.proton-window.light {
    background-color: #f6f7f8;
    border-color: rgba(32, 35, 38, 0.18);
}

window.proton-window.light .launcher-root {
    background-color: #f6f7f8;
}

window.proton-window.light list.result-list {
    background-color: #f6f7f8;
}

window.proton-window.light .search-shell {
    background-color: #ffffff;
    border-color: rgba(32, 35, 38, 0.16);
}

window.proton-window.light .search-icon {
    color: #4b535b;
}

window.proton-window.light entry.search-entry,
window.proton-window.light .result-title,
window.proton-window.light .empty-state-title {
    color: #202326;
}

window.proton-window.light entry.search-entry {
    caret-color: #202326;
}

window.proton-window.light entry.search-entry placeholder,
window.proton-window.light entry.search-entry text.placeholder,
window.proton-window.light entry.search-entry > text > placeholder {
    color: #202326;
    opacity: 1;
}

window.proton-window.light entry.search-entry text,
window.proton-window.light entry.search-entry selection {
    color: #202326;
}

window.proton-window.light entry.search-entry selection {
    background-color: #b9d7d0;
}

window.proton-window.light .category-chip {
    color: #687078;
    background-color: transparent;
}

window.proton-window.light button.category-chip label {
    color: #4f5861;
}

window.proton-window.light button.category-chip.active,
window.proton-window.light button.category-chip:checked {
    color: #202326;
    background-color: #d7dce1;
}

window.proton-window.light button.category-chip.active label,
window.proton-window.light button.category-chip:checked label {
    color: #202326;
}

window.proton-window.light .result-subtitle,
window.proton-window.light .status-label,
window.proton-window.light .footer-hint,
window.proton-window.light .preview-hint,
window.proton-window.light .empty-state-hint {
    color: #5e646a;
}

window.proton-window.light row.result-row {
    background-color: transparent;
}

window.proton-window.light row.result-row:hover {
    background-color: #e9edf1;
}

window.proton-window.light row.result-row:focus,
window.proton-window.light row.result-row:selected {
    background-color: #d7e5f5;
}

window.proton-window.light row.result-row:hover label.result-title,
window.proton-window.light row.result-row:focus label.result-title,
window.proton-window.light row.result-row:selected label.result-title {
    color: #17202a;
}

window.proton-window.light row.result-row:hover label.result-subtitle,
window.proton-window.light row.result-row:focus label.result-subtitle,
window.proton-window.light row.result-row:selected label.result-subtitle {
    color: #435363;
}

window.proton-window.light row.result-row:hover label.source-badge,
window.proton-window.light row.result-row:focus label.source-badge,
window.proton-window.light row.result-row:selected label.source-badge {
    color: #263746;
    background-color: rgba(38, 55, 70, 0.12);
}

window.proton-window.light row.result-row:hover label,
window.proton-window.light row.result-row:focus label,
window.proton-window.light row.result-row:selected label {
    color: #202326;
}

/* Keep the native ListBox selected-row foreground from reintroducing the
 * dark-theme accent color in light mode. The extra list/row qualifiers are
 * intentional: GTK's theme gives selected descendants a more specific rule
 * than a plain label class. */
window.proton-window.light list.result-list row.result-row label.result-title {
    color: #17202a;
}

window.proton-window.light list.result-list row.result-row label.result-subtitle {
    color: #435363;
}

window.proton-window.light list.result-list row.result-row label.source-badge {
    color: #263746;
}

window.proton-window.light row.result-row:hover .result-icon,
window.proton-window.light row.result-row:focus .result-icon,
window.proton-window.light row.result-row:selected .result-icon,
window.proton-window.light row.result-row:hover .asset-icon,
window.proton-window.light row.result-row:focus .asset-icon,
window.proton-window.light row.result-row:selected .asset-icon {
    color: #34424f;
}

window.proton-window.light .image-preview {
    background-color: #ffffff;
    border-color: rgba(32, 35, 38, 0.16);
}

window.proton-window.light .preview-title {
    color: #202326;
}

window.proton-window.light .source-badge {
    color: #4f5861;
    background-color: rgba(32, 35, 38, 0.09);
}

window.proton-window.light .result-icon,
window.proton-window.light .asset-icon {
    color: #4b535b;
}

.settings-window {
    background-color: #202122;
}

.settings-shell {
    background-color: #202122;
}

.settings-sidebar {
    background-color: #18191a;
    padding: 18px 10px;
}

.settings-sidebar-title {
    color: #f1f2f3;
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 12px;
}

stacksidebar row {
    color: #aeb3b8;
    border-radius: 6px;
    padding: 7px 10px;
}

stacksidebar row:hover {
    background-color: #2d3033;
}

stacksidebar row:selected {
    color: #ffffff;
    background-color: #45494d;
}

.settings-heading {
    color: #f1f2f3;
    font-size: 24px;
    font-weight: 700;
}

.settings-heading-row {
    min-height: 42px;
}

.settings-label {
    color: #f1f2f3;
    font-size: 13px;
    font-weight: 600;
}

.settings-help {
    color: #9da1a5;
    font-size: 11px;
}

.settings-section-title {
    color: #cfd2d5;
    font-size: 15px;
    font-weight: 700;
    margin-top: 10px;
}

textview.agent-transcript {
    background-color: #18191a;
    color: #f1f2f3;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    padding: 12px;
}

textview.agent-transcript text {
    background-color: transparent;
    color: #f1f2f3;
}

window.settings-window.settings-theme-light textview.agent-transcript,
window.proton-window.light textview.agent-transcript {
    background-color: #ffffff;
    color: #202225;
    border-color: rgba(32, 35, 38, 0.18);
}

window.settings-window.settings-theme-light textview.agent-transcript text,
window.proton-window.light textview.agent-transcript text {
    background-color: transparent;
    color: #202225;
}

window.settings-window.settings-theme-light {
    background-color: #f4f5f6;
}

window.settings-window.settings-theme-light .settings-heading,
window.settings-window.settings-theme-light .settings-label,
window.settings-window.settings-theme-light .settings-section-title {
    color: #202225;
}

window.settings-window.settings-theme-light .settings-help,
window.settings-window.settings-theme-light .result-subtitle {
    color: #5f6469;
}

window.settings-window.settings-theme-system {
    background-color: #25272a;
}
"#;

pub fn run(paths: XdgPaths) -> Result<()> {
    let Some((socket_guard, commands)) = start_ipc(&paths, true) else {
        return Ok(());
    };
    run_application(paths, socket_guard, commands)
}

pub fn run_resident(paths: XdgPaths) -> Result<()> {
    loop {
        if let Some((socket_guard, commands)) = start_ipc(&paths, false) {
            return run_application(paths, socket_guard, commands);
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn run_application(
    paths: XdgPaths,
    socket_guard: SocketGuard,
    commands: async_channel::Receiver<UiCommand>,
) -> Result<()> {
    let _tray = crate::tray::ProtonTray::start(&paths);
    let application = Application::builder()
        .application_id(APPLICATION_ID)
        // A compositor keybind may launch a fresh window while another
        // launcher invocation is still closing. Each invocation must receive
        // its own window instead of being forwarded as an unsupported file
        // open request to an existing instance.
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let commands = Rc::new(RefCell::new(Some(commands)));
    let commands_for_activate = commands.clone();
    application.connect_activate(move |application| {
        if let Some(commands) = commands_for_activate.borrow_mut().take() {
            build_window(application, paths.clone(), commands);
        }
    });
    // Do not pass the CLI subcommand (`gui`) into GApplication. GTK treats
    // positional arguments as files to open, which would bypass activation
    // and immediately exit with an "cannot open files" warning.
    let program = std::env::args()
        .next()
        .unwrap_or_else(|| "protonsearch-linux".to_string());
    application.run_with_args(&[program]);
    drop(socket_guard);
    Ok(())
}

fn settings_page(title: &str, description: &str) -> (GtkBox, ScrolledWindow) {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_top(24);
    page.set_margin_bottom(24);
    page.set_margin_start(28);
    page.set_margin_end(28);

    let heading = Label::new(Some(title));
    heading.set_halign(Align::Start);
    heading.add_css_class("settings-heading");
    page.append(&heading);

    let intro = Label::new(Some(description));
    intro.set_halign(Align::Start);
    intro.add_css_class("result-subtitle");
    page.append(&intro);

    let scroll = ScrolledWindow::builder()
        .child(&page)
        .vexpand(true)
        .hexpand(true)
        .build();
    (page, scroll)
}

fn open_agent_window(parent: &ApplicationWindow, session: Option<String>) {
    let Some(application) = parent.application() else {
        return;
    };
    let window = ApplicationWindow::builder()
        .application(&application)
        .title("ProtonSearch Agent")
        .default_width(720)
        .default_height(560)
        .build();
    window.set_transient_for(Some(parent));
    window.set_modal(false);
    window.add_css_class("settings-window");

    let root = GtkBox::new(Orientation::Vertical, 12);
    root.set_margin_top(22);
    root.set_margin_bottom(18);
    root.set_margin_start(22);
    root.set_margin_end(22);

    let heading = Label::new(Some("ProtonSearch Agent"));
    heading.set_halign(Align::Start);
    heading.add_css_class("settings-heading");
    root.append(&heading);

    let description = Label::new(Some(
        "Hermes Agent runs in a worker so the launcher and this conversation remain responsive.",
    ));
    description.set_wrap(true);
    description.set_halign(Align::Start);
    description.add_css_class("settings-help");
    root.append(&description);

    let transcript = TextView::new();
    transcript.set_editable(false);
    transcript.set_cursor_visible(false);
    transcript.set_wrap_mode(WrapMode::WordChar);
    transcript.set_vexpand(true);
    transcript.set_hexpand(true);
    transcript.add_css_class("agent-transcript");
    let transcript_scroll = ScrolledWindow::builder()
        .child(&transcript)
        .vexpand(true)
        .hexpand(true)
        .build();
    root.append(&transcript_scroll);

    let status = Label::new(Some(if session.is_some() {
        "Ready to continue this Hermes session"
    } else {
        "Ready"
    }));
    status.set_halign(Align::Start);
    status.add_css_class("settings-help");
    root.append(&status);

    let prompt = Entry::new();
    prompt.set_hexpand(true);
    prompt.set_placeholder_text(Some("Ask Hermes Agent something…"));
    prompt.set_activates_default(true);
    let send = Button::with_label("Send");
    send.set_receives_default(true);
    let input_row = GtkBox::new(Orientation::Horizontal, 8);
    input_row.set_hexpand(true);
    input_row.append(&prompt);
    input_row.append(&send);
    root.append(&input_row);
    window.set_child(Some(&root));

    let (sender, receiver) = mpsc::channel::<Result<String, String>>();
    let status_for_receiver = status.clone();
    let transcript_for_receiver = transcript.clone();
    let send_for_receiver = send.clone();
    let weak_window = window.downgrade();
    glib::timeout_add_local(Duration::from_millis(80), move || {
        let Some(_window) = weak_window.upgrade() else {
            return glib::ControlFlow::Break;
        };
        while let Ok(result) = receiver.try_recv() {
            send_for_receiver.set_sensitive(true);
            match result {
                Ok(response) => {
                    status_for_receiver.set_text("Response received");
                    let buffer = transcript_for_receiver.buffer();
                    let previous = buffer
                        .text(&buffer.start_iter(), &buffer.end_iter(), false)
                        .to_string();
                    let text = if previous.trim().is_empty() {
                        format!("Hermes Agent\n\n{response}")
                    } else {
                        format!("{previous}\n\n{response}")
                    };
                    buffer.set_text(&text);
                }
                Err(error) => {
                    status_for_receiver.set_text(&format!("Agent error: {error}"));
                }
            }
        }
        glib::ControlFlow::Continue
    });

    let session_for_prompt = session;
    send.connect_clicked(move |button| {
        let value = prompt.text().trim().to_string();
        if value.is_empty() || !button.is_sensitive() {
            return;
        }
        button.set_sensitive(false);
        status.set_text("Hermes is thinking…");
        let sender_for_worker = sender.clone();
        let session = session_for_prompt.clone();
        thread::spawn(move || {
            let command = if crate::system::command_available("hermes") {
                "hermes"
            } else if crate::system::command_available("hermes-agent") {
                "hermes-agent"
            } else {
                let _ = sender_for_worker.send(Err(
                    "Hermes Agent is not installed. Install Hermes, then retry.".to_string(),
                ));
                return;
            };
            let mut args = Vec::new();
            if let Some(session) = session.as_deref() {
                args.extend(["--resume", session]);
            }
            args.extend(["-z", value.as_str()]);
            let result = crate::system::run_with_timeout(command, &args, Duration::from_secs(90));
            let response = match result {
                Ok(output) if output.status == Some(0) && !output.stdout.trim().is_empty() => {
                    Ok(output.stdout)
                }
                Ok(output) if output.timed_out => Err(
                    "Hermes did not respond within 90 seconds. Check Hermes status and retry."
                        .to_string(),
                ),
                Ok(output) => Err(if output.stderr.is_empty() {
                    format!("Hermes exited with status {:?}.", output.status)
                } else {
                    format!("Hermes: {}", output.stderr)
                }),
                Err(error) => Err(format!("Could not run Hermes: {error}")),
            };
            let _ = sender_for_worker.send(response);
        });
        prompt.set_text("");
    });
    window.present();
}

pub fn run_settings(paths: XdgPaths) -> Result<()> {
    let application = Application::builder()
        .application_id("com.protonsearch.Linux.Settings")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    application.connect_activate(move |application| {
        let current = settings::load(&paths);
        let window = ApplicationWindow::builder()
            .application(application)
            .title("ProtonSearch Settings")
            .default_width(760)
            .default_height(700)
            .build();
        window.add_css_class("settings-window");
        install_css();
        let root = GtkBox::new(Orientation::Horizontal, 0);
        root.add_css_class("settings-shell");

        let sidebar = GtkBox::new(Orientation::Vertical, 10);
        sidebar.add_css_class("settings-sidebar");
        sidebar.set_width_request(190);
        let logo = crate::icons::protonsearch(48);
        logo.set_halign(Align::Center);
        sidebar.append(&logo);
        let sidebar_title = Label::new(Some("ProtonSearch"));
        sidebar_title.add_css_class("settings-sidebar-title");
        sidebar_title.set_halign(Align::Center);
        sidebar.append(&sidebar_title);

        let stack = Stack::new();
        stack.set_vexpand(true);
        stack.set_hexpand(true);
        stack.set_transition_type(StackTransitionType::SlideLeftRight);
        let stack_sidebar = StackSidebar::new();
        stack_sidebar.set_stack(&stack);
        stack_sidebar.set_vexpand(true);

        let (general_page, general_scroll) = settings_page(
            "General",
            "Startup and launcher visibility.",
        );
        let (appearance_page, appearance_scroll) = settings_page(
            "Appearance and layout",
            "Theme and dimensions applied to the running launcher.",
        );
        let (search_page, search_scroll) = settings_page(
            "Search",
            "Search roots, privacy boundaries, and application visibility.",
        );
        let (providers_page, providers_scroll) = settings_page(
            "Providers",
            "Enable the Linux-native providers you want available.",
        );
        let (hotkey_page, hotkey_scroll) = settings_page(
            "Hotkey",
            "The compositor-owned shortcut used to toggle ProtonSearch.",
        );
        let (safety_page, safety_scroll) = settings_page(
            "Safety and Linux",
            "Power-action confirmation, diagnostics, and platform notes.",
        );
        let (indexing_page, indexing_scroll) = settings_page(
            "Indexing and database",
            "Understand how Linux search stays responsive and bounded.",
        );

        stack.add_titled(&general_scroll, Some("general"), "General");
        stack.add_titled(&appearance_scroll, Some("appearance"), "Appearance");
        stack.add_titled(&search_scroll, Some("search"), "Search");
        stack.add_titled(&providers_scroll, Some("providers"), "Providers");
        stack.add_titled(&hotkey_scroll, Some("hotkey"), "Hotkey");
        stack.add_titled(&safety_scroll, Some("safety"), "Safety & Linux");
        stack.add_titled(&indexing_scroll, Some("indexing"), "Indexing & Database");

        let startup = CheckButton::with_label("Run ProtonSearch in the background at login");
        startup.set_active(current.run_on_startup);
        startup.set_tooltip_text(Some(
            "Uses the ProtonSearch systemd user service when it is installed",
        ));
        general_page.append(&startup);
        let show_taskbar = CheckButton::with_label("Show a taskbar/dock entry when supported");
        show_taskbar.set_active(current.show_taskbar);
        show_taskbar.set_tooltip_text(Some(
            "Wayland compositors and desktop shells decide whether this is available",
        ));
        general_page.append(&show_taskbar);
        let show_placeholder = CheckButton::with_label("Show the launcher search placeholder");
        show_placeholder.set_active(current.show_placeholder);
        general_page.append(&show_placeholder);

        let appearance_label = Label::new(Some("Appearance and layout"));
        appearance_label.set_halign(Align::Start);
        appearance_label.add_css_class("settings-section-title");
        appearance_page.append(&appearance_label);
        let theme = ComboBoxText::new();
        theme.append(Some("system"), "System theme");
        theme.append(Some("dark"), "Dark");
        theme.append(Some("light"), "Light");
        let theme_id = match current.theme_mode.to_ascii_lowercase().as_str() {
            "system" => "system",
            "light" => "light",
            _ => "dark",
        };
        theme.set_active_id(Some(theme_id));
        theme.set_tooltip_text(Some(
            "Uses GTK colors; exact appearance follows the current desktop theme",
        ));
        appearance_page.append(&theme);

        let width_label = Label::new(Some("Launcher width"));
        width_label.set_halign(Align::Start);
        width_label.add_css_class("settings-label");
        appearance_page.append(&width_label);
        let width = SpinButton::with_range(480.0, 1600.0, 10.0);
        width.set_value(f64::from(current.window_width.clamp(480, 1600)));
        appearance_page.append(&width);

        let height_label = Label::new(Some("Launcher height"));
        height_label.set_halign(Align::Start);
        height_label.add_css_class("settings-label");
        appearance_page.append(&height_label);
        let height = SpinButton::with_range(420.0, 1200.0, 10.0);
        height.set_value(f64::from(current.window_height.clamp(420, 1200)));
        appearance_page.append(&height);

        let item_height_label = Label::new(Some("Result row height"));
        item_height_label.set_halign(Align::Start);
        item_height_label.add_css_class("settings-label");
        appearance_page.append(&item_height_label);
        let item_height = SpinButton::with_range(52.0, 120.0, 4.0);
        item_height.set_value(f64::from(current.item_height.clamp(52, 120)));
        appearance_page.append(&item_height);

        let search_bar_height_label = Label::new(Some("Search bar height"));
        search_bar_height_label.set_halign(Align::Start);
        search_bar_height_label.add_css_class("settings-label");
        appearance_page.append(&search_bar_height_label);
        let search_bar_height = SpinButton::with_range(42.0, 100.0, 2.0);
        search_bar_height.set_value(f64::from(current.search_bar_height.clamp(42, 100)));
        appearance_page.append(&search_bar_height);

        let search_label = Label::new(Some("Search"));
        search_label.set_halign(Align::Start);
        search_label.add_css_class("settings-section-title");
        search_page.append(&search_label);
        let include_hidden = CheckButton::with_label("Include hidden files in search");
        include_hidden.set_active(current.include_hidden);
        search_page.append(&include_hidden);
        let terminal_apps = CheckButton::with_label("Include terminal applications");
        terminal_apps.set_active(current.show_terminal_apps);
        search_page.append(&terminal_apps);

        let roots_label = Label::new(Some("Additional search roots (comma or newline separated)"));
        roots_label.set_halign(Align::Start);
        roots_label.add_css_class("settings-label");
        search_page.append(&roots_label);
        let roots = Entry::builder()
            .placeholder_text("/home/user/Projects, /mnt/data")
            .text(current.search_roots.join("\n"))
            .build();
        search_page.append(&roots);

        let ignored_label = Label::new(Some("Ignored directory names (comma separated)"));
        ignored_label.set_halign(Align::Start);
        ignored_label.add_css_class("settings-label");
        search_page.append(&ignored_label);
        let ignored = Entry::builder()
            .placeholder_text("node_modules, target, .cache")
            .text(current.ignored_names.join(", "))
            .build();
        search_page.append(&ignored);

        let providers_label = Label::new(Some("Providers"));
        providers_label.set_halign(Align::Start);
        providers_label.add_css_class("settings-section-title");
        providers_page.append(&providers_label);
        let system_actions = CheckButton::with_label("Enable system actions");
        system_actions.set_active(current.enable_system_actions);
        providers_page.append(&system_actions);
        let hyprland = CheckButton::with_label("Enable Hyprland providers");
        hyprland.set_active(current.enable_hyprland);
        providers_page.append(&hyprland);
        let calculator = CheckButton::with_label("Enable calculator");
        calculator.set_active(current.enable_calculator);
        providers_page.append(&calculator);
        let git_commits = CheckButton::with_label("Enable Git commit search");
        git_commits.set_active(current.enable_git_commits);
        providers_page.append(&git_commits);
        let clipboard_history = CheckButton::with_label("Enable clipboard history");
        clipboard_history.set_active(current.enable_clipboard_history);
        providers_page.append(&clipboard_history);
        let ocr = CheckButton::with_label("OCR image search (coming soon)");
        ocr.set_active(current.enable_ocr);
        ocr.set_sensitive(false);
        ocr.set_tooltip_text(Some(
            "OCR is intentionally deferred. Use Images for image filename search.",
        ));
        providers_page.append(&ocr);
        let browser_history = CheckButton::with_label("Enable browser history search");
        browser_history.set_active(current.enable_browser_history);
        providers_page.append(&browser_history);
        let hermes = CheckButton::with_label("Enable Hermes Agent integration");
        hermes.set_active(current.enable_hermes);
        hermes.set_tooltip_text(Some(
            "Uses the installed Hermes Agent desktop app when the hermes command is available",
        ));
        providers_page.append(&hermes);
        let agent_history = CheckButton::with_label("Enable Hermes Agent history");
        agent_history.set_active(current.enable_agent_history);
        providers_page.append(&agent_history);

        let safety_label = Label::new(Some("Safety and diagnostics"));
        safety_label.set_halign(Align::Start);
        safety_label.add_css_class("settings-section-title");
        safety_page.append(&safety_label);
        let confirm_power = CheckButton::with_label("Ask before power and session actions");
        confirm_power.set_active(current.confirm_power_actions);
        confirm_power.set_tooltip_text(Some(
            "Explicit confirmation remains required for destructive CLI actions",
        ));
        safety_page.append(&confirm_power);
        let log_level = ComboBoxText::new();
        for level in ["error", "warn", "info", "debug"] {
            log_level.append(Some(level), level);
        }
        let log_id = match current.log_level.as_str() {
            "error" | "warn" | "info" | "debug" => current.log_level.as_str(),
            _ => "info",
        };
        log_level.set_active_id(Some(log_id));
        log_level.set_tooltip_text(Some("Controls diagnostic verbosity for future providers"));
        safety_page.append(&log_level);

        let native_note = Label::new(Some(
            "Windows-only Registry, taskbar, wallpaper, Win32 window placement, agent API, and updater controls are intentionally not shown here. Wi-Fi, Bluetooth, audio, brightness, display, power, keyboard, mouse, notifications, privacy, date/time, users, region, and software settings are available from the launcher’s Linux-native commands.",
        ));
        native_note.set_wrap(true);
        native_note.set_halign(Align::Start);
        native_note.add_css_class("settings-help");
        safety_page.append(&native_note);

        let indexing_title = Label::new(Some("Linux search model"));
        indexing_title.set_halign(Align::Start);
        indexing_title.add_css_class("settings-section-title");
        indexing_page.append(&indexing_title);
        let indexing_note = Label::new(Some(
            "Linux uses bounded on-demand providers instead of a continuously polling background index. File and folder results are searched only when a query needs them; Git, clipboard, browser, image, OCR, and agent providers are independently bounded. This keeps the resident launcher idle when it is hidden and avoids a permanent CPU or memory watcher.",
        ));
        indexing_note.set_wrap(true);
        indexing_note.set_halign(Align::Start);
        indexing_note.add_css_class("settings-help");
        indexing_page.append(&indexing_note);

        let roots_title = Label::new(Some("Authoritative search roots"));
        roots_title.set_halign(Align::Start);
        roots_title.add_css_class("settings-section-title");
        indexing_page.append(&roots_title);
        let roots_text = paths
            .search_roots(
                &current
                    .search_roots
                    .iter()
                    .map(std::path::PathBuf::from)
                    .collect::<Vec<_>>(),
            )
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n");
        let roots_value = Label::new(Some(if roots_text.is_empty() {
            "No readable search roots detected"
        } else {
            &roots_text
        }));
        roots_value.set_selectable(true);
        roots_value.set_wrap(true);
        roots_value.set_halign(Align::Start);
        roots_value.add_css_class("settings-help");
        indexing_page.append(&roots_value);

        let data_title = Label::new(Some("ProtonSearch data"));
        data_title.set_halign(Align::Start);
        data_title.add_css_class("settings-section-title");
        indexing_page.append(&data_title);
        let data_value = Label::new(Some(&format!(
            "Settings: {}\nState: {}\nCache: {}\n\nNo database rebuild is required for the current Linux provider model.",
            paths.settings_file().display(),
            paths.state_dir().display(),
            paths.cache_dir().display(),
        )));
        data_value.set_selectable(true);
        data_value.set_wrap(true);
        data_value.set_halign(Align::Start);
        data_value.add_css_class("settings-help");
        indexing_page.append(&data_value);

        let hotkey_title = Label::new(Some("Launcher hotkey"));
        hotkey_title.set_halign(Align::Start);
        hotkey_title.add_css_class("settings-label");
        hotkey_page.append(&hotkey_title);
        let hotkey_help = Label::new(Some(
            "Hyprland notation. Default: ALT,SPACE. The installer manages the global binding safely.",
        ));
        hotkey_help.set_halign(Align::Start);
        hotkey_help.add_css_class("settings-help");
        hotkey_page.append(&hotkey_help);
        let hotkey = Entry::builder()
            .placeholder_text("ALT,SPACE")
            .text(&current.hotkey)
            .build();
        hotkey_page.append(&hotkey);

        let save = Button::with_label("Save settings");
        save.set_halign(Align::End);
        let save_status = Label::new(None);
        save_status.set_hexpand(true);
        save_status.set_halign(Align::Start);
        save_status.add_css_class("settings-help");
        let footer = GtkBox::new(Orientation::Horizontal, 8);
        footer.set_margin_top(10);
        footer.set_margin_bottom(12);
        footer.set_margin_start(18);
        footer.set_margin_end(18);
        footer.append(&save_status);
        footer.append(&save);

        sidebar.append(&stack_sidebar);
        root.append(&sidebar);
        let content = GtkBox::new(Orientation::Vertical, 0);
        content.set_hexpand(true);
        content.set_vexpand(true);
        content.append(&stack);
        content.append(&footer);
        root.append(&content);
        window.set_child(Some(&root));

        let paths_for_save = paths.clone();
        save.connect_clicked(move |_| {
            let mut next = current.clone();
            next.run_on_startup = startup.is_active();
            next.show_taskbar = show_taskbar.is_active();
            next.show_placeholder = show_placeholder.is_active();
            next.theme_mode = theme
                .active_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "dark".to_string());
            next.window_width = width.value_as_int().clamp(480, 1600) as u32;
            next.window_height = height.value_as_int().clamp(420, 1200) as u32;
            next.item_height = item_height.value_as_int().clamp(52, 120) as u32;
            next.search_bar_height = search_bar_height.value_as_int().clamp(42, 100) as u32;
            next.include_hidden = include_hidden.is_active();
            next.show_terminal_apps = terminal_apps.is_active();
            next.enable_system_actions = system_actions.is_active();
            next.enable_hyprland = hyprland.is_active();
            next.enable_calculator = calculator.is_active();
            next.enable_git_commits = git_commits.is_active();
            next.enable_clipboard_history = clipboard_history.is_active();
            next.enable_ocr = ocr.is_active();
            next.enable_browser_history = browser_history.is_active();
            next.enable_hermes = hermes.is_active();
            next.enable_agent_history = agent_history.is_active();
            next.confirm_power_actions = confirm_power.is_active();
            next.log_level = log_level
                .active_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "info".to_string());
            let appearance_changed = next.theme_mode != current.theme_mode
                || next.window_width != current.window_width
                || next.window_height != current.window_height
                || next.item_height != current.item_height
                || next.search_bar_height != current.search_bar_height
                || next.show_placeholder != current.show_placeholder;
            let hotkey_text = hotkey.text();
            let requested_hotkey = if hotkey_text.trim().is_empty() {
                "ALT,SPACE"
            } else {
                hotkey_text.trim()
            };
            let Some(normalized_hotkey) = normalize_hotkey(requested_hotkey) else {
                hotkey.add_css_class("error");
                hotkey.set_tooltip_text(Some(
                    "Use compositor notation such as ALT,SPACE or SUPER,ENTER",
                ));
                return;
            };
            hotkey.remove_css_class("error");
            next.hotkey = normalized_hotkey;
            next.search_roots = roots
                .text()
                .split([',', '\n'])
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(str::to_string)
                .collect();
            next.ignored_names = ignored
                .text()
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect();
            if let Err(error) = settings::save(&paths_for_save, &next) {
                eprintln!("ProtonSearch: could not save settings: {error:#}");
                return;
            }
            if next.run_on_startup != current.run_on_startup {
                if let Err(error) = sync_startup_service(next.run_on_startup) {
                    eprintln!("ProtonSearch: could not update startup service: {error:#}");
                }
            }
            if next.enable_hyprland {
                apply_hyprland_hotkey(&current.hotkey, &next.hotkey);
            } else {
                unbind_hyprland_hotkey(&current.hotkey);
            }
            notify_launcher(&paths_for_save, "reload");
            if appearance_changed {
                restart_launcher_service_if_active();
            }
            save_status.set_text("Settings saved — ProtonSearch updated.");
        });
        window.present();
    });
    let program = std::env::args()
        .next()
        .unwrap_or_else(|| "protonsearch-linux".to_string());
    application.run_with_args(&[program]);
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum UiCommand {
    Toggle,
    Reload,
}

struct SocketGuard {
    path: std::path::PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // Wake a non-blocking listener so shutdown does not leave a worker
        // thread behind when the GTK application exits.
        let _ = UnixStream::connect(&self.path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn start_ipc(
    paths: &XdgPaths,
    notify_existing: bool,
) -> Option<(SocketGuard, async_channel::Receiver<UiCommand>)> {
    let socket = ipc_socket(paths);
    let directory = socket.parent()?.to_path_buf();
    let _ = fs::create_dir_all(&directory);
    if socket.exists() {
        match UnixStream::connect(&socket) {
            Ok(mut stream) => {
                if notify_existing {
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                    let _ = stream.write_all(b"toggle\n");
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
                return None;
            }
            Err(_) => {
                let _ = fs::remove_file(&socket);
            }
        }
    }
    let listener = match UnixListener::bind(&socket) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            if notify_existing {
                if let Ok(mut stream) = UnixStream::connect(&socket) {
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                    let _ = stream.write_all(b"toggle\n");
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
            }
            return None;
        }
        Err(_) => return None,
    };
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop.clone();
    let (sender, receiver) = async_channel::unbounded();
    let thread = thread::spawn(move || {
        while !stop_for_thread.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if stop_for_thread.load(Ordering::Acquire) {
                        break;
                    }
                    let mut message = String::new();
                    let _ = std::io::BufReader::new(stream).read_line(&mut message);
                    for line in message.lines().map(str::trim) {
                        let command = match line {
                            "toggle" => Some(UiCommand::Toggle),
                            "reload" => Some(UiCommand::Reload),
                            _ => None,
                        };
                        if let Some(command) = command {
                            let _ = sender.send_blocking(command);
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
    Some((
        SocketGuard {
            path: socket,
            stop,
            thread: Some(thread),
        },
        receiver,
    ))
}

fn ipc_socket(paths: &XdgPaths) -> std::path::PathBuf {
    paths
        .runtime
        .as_ref()
        .map(|runtime| runtime.join("protonsearch/launcher.sock"))
        .unwrap_or_else(|| paths.state_dir().join("launcher.sock"))
}

fn notify_launcher(paths: &XdgPaths, command: &str) {
    let Ok(mut stream) = UnixStream::connect(ipc_socket(paths)) else {
        return;
    };
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    let _ = stream.write_all(format!("{command}\n").as_bytes());
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

fn sync_startup_service(enabled: bool) -> anyhow::Result<()> {
    let verb = if enabled { "enable" } else { "disable" };
    let result = crate::system::run(
        "systemctl",
        &["--user", verb, "--now", "protonsearch.service"],
    )?;
    if result.status == Some(0) {
        Ok(())
    } else {
        anyhow::bail!(
            "systemd user service could not be {verb}d: {}",
            result.stderr.trim()
        )
    }
}

fn restart_launcher_service_if_active() {
    let Ok(active) = crate::system::run(
        "systemctl",
        &["--user", "is-active", "--quiet", "protonsearch.service"],
    ) else {
        return;
    };
    if active.status != Some(0) {
        return;
    }
    if let Err(error) =
        crate::system::run("systemctl", &["--user", "restart", "protonsearch.service"])
    {
        eprintln!("ProtonSearch: could not apply launcher appearance: {error:#}");
    }
}

fn confirmation_target(target: &Target) -> Option<(Target, &'static str)> {
    let Target::Action {
        id,
        args,
        confirmed: false,
    } = target
    else {
        return None;
    };
    let label = match id.as_str() {
        "power-suspend" => "Suspend computer",
        "power-reboot" => "Restart computer",
        "poweroff" => "Power off computer",
        "power-logout" => "Log out",
        _ => return None,
    };
    Some((
        Target::Action {
            id: id.clone(),
            args: args.clone(),
            confirmed: true,
        },
        label,
    ))
}

fn set_launcher_status(status: &Label, message: &str) {
    status.set_text(message);
    status.set_tooltip_text(None);
}

fn activate_target(
    paths: &XdgPaths,
    window: &ApplicationWindow,
    status: &Label,
    animation: &Rc<RefCell<Option<glib::SourceId>>>,
    action_sender: &async_channel::Sender<anyhow::Result<Option<String>>>,
    target: Target,
) {
    if matches!(&target, Target::Action { .. }) {
        let paths_for_worker = paths.clone();
        let target_for_worker = target.clone();
        let sender_for_worker = action_sender.clone();
        set_launcher_status(status, "Running action…");
        thread::spawn(move || {
            let result = providers::activate(&paths_for_worker, &target_for_worker);
            let _ = sender_for_worker.send_blocking(result);
        });
        return;
    }
    let keep_launcher_open = matches!(&target, Target::Action { .. });
    match providers::activate(paths, &target) {
        Ok(Some(feedback)) => {
            set_launcher_status(status, feedback.lines().next().unwrap_or(feedback.as_str()));
        }
        Ok(None) if keep_launcher_open => {
            set_launcher_status(status, "Action completed");
        }
        Ok(None) => {
            animate_hide(window, animation);
        }
        Err(error) => {
            set_launcher_status(status, &format!("Action failed: {error}"));
            status.set_tooltip_text(Some(&format!("{error:#}")));
            eprintln!("ProtonSearch: {error:#}");
        }
    }
}

fn activate_item(
    paths: &XdgPaths,
    window: &ApplicationWindow,
    entry: &Entry,
    status: &Label,
    animation: &Rc<RefCell<Option<glib::SourceId>>>,
    action_sender: &async_channel::Sender<anyhow::Result<Option<String>>>,
    item: Item,
) {
    match item.target {
        Target::Query(query) => {
            entry.set_text(&query);
            entry.grab_focus();
        }
        Target::Notice(message) => set_launcher_status(status, &message),
        target => {
            if let Target::Action { id, args, .. } = &target {
                if id == "open-agent" {
                    open_agent_window(window, args.first().cloned());
                    return;
                }
            }
            if let Some((confirmed_target, label)) = confirmation_target(&target) {
                set_launcher_status(status, &format!("Confirmation required: {label}"));
                let dialog = MessageDialog::builder()
                    .transient_for(window)
                    .modal(true)
                    .message_type(MessageType::Warning)
                    .buttons(ButtonsType::Cancel)
                    .text("Confirm session action")
                    .secondary_text(format!("{label} now? This action cannot be undone."))
                    .build();
                dialog.set_title(Some(label));
                dialog
                    .add_button(label, ResponseType::Accept)
                    .add_css_class("destructive-action");
                dialog.set_default_response(ResponseType::Cancel);

                let paths_for_confirmation = paths.clone();
                let window_for_confirmation = window.clone();
                let entry_for_confirmation = entry.clone();
                let status_for_confirmation = status.clone();
                let animation_for_confirmation = animation.clone();
                let action_sender_for_confirmation = action_sender.clone();
                dialog.connect_response(move |dialog, response| {
                    dialog.close();
                    if response == ResponseType::Accept {
                        activate_target(
                            &paths_for_confirmation,
                            &window_for_confirmation,
                            &status_for_confirmation,
                            &animation_for_confirmation,
                            &action_sender_for_confirmation,
                            confirmed_target.clone(),
                        );
                    } else {
                        set_launcher_status(&status_for_confirmation, "Action cancelled");
                        entry_for_confirmation.grab_focus();
                    }
                });
                dialog.present();
            } else {
                activate_target(paths, window, status, animation, action_sender, target);
            }
        }
    }
}

fn build_window(
    application: &Application,
    paths: XdgPaths,
    commands: async_channel::Receiver<UiCommand>,
) {
    let linux_settings = settings::load(&paths);
    let settings_state = Rc::new(RefCell::new(linux_settings.clone()));
    let window = ApplicationWindow::builder()
        .application(application)
        .title("ProtonSearch")
        .default_width(linux_settings.window_width.clamp(480, 1600) as i32)
        .default_height(linux_settings.window_height.clamp(420, 1200) as i32)
        .build();
    // Keep an explicit application-owned reference. This matters when the
    // launcher is started directly from a compositor keybind: the local
    // window variable is otherwise dropped when this builder function returns.
    application.add_window(&window);
    window.set_decorated(false);
    window.set_resizable(false);
    window.add_css_class("proton-window");
    window.add_css_class(theme_class(&linux_settings.theme_mode));
    install_css();

    let root = GtkBox::new(Orientation::Vertical, 10);
    root.add_css_class("launcher-root");
    root.set_margin_top(18);
    root.set_margin_bottom(14);
    root.set_margin_start(18);
    root.set_margin_end(18);

    let search_shell = GtkBox::new(Orientation::Horizontal, 6);
    search_shell.add_css_class("search-shell");
    search_shell.set_hexpand(true);
    search_shell.set_height_request(linux_settings.search_bar_height.clamp(42, 100) as i32);

    let search_icon = crate::icons::protonsearch(34);
    search_icon.add_css_class("search-icon");
    search_shell.append(&search_icon);

    let entry = Entry::builder().hexpand(true).build();
    if linux_settings.show_placeholder {
        entry.set_placeholder_text(Some("Search files, code, PDFs..."));
    }
    entry.add_css_class("search-entry");
    entry.set_tooltip_text(Some(
        "Type to search; Enter opens the selected result; Escape closes",
    ));
    search_shell.append(&entry);
    root.append(&search_shell);

    let category_row = GtkBox::new(Orientation::Horizontal, 2);
    category_row.add_css_class("category-row");
    for (label, prefix, active) in [
        ("All", "", true),
        ("Files", "file:", false),
        ("Folders", "folder:", false),
        ("Content", "content:", false),
        ("Images", "images:", false),
        ("OCR", "ocr:", false),
        ("Code", "code:", false),
        ("Settings", "settings:", false),
        ("Commands", "commands:", false),
    ] {
        let chip = Button::with_label(label);
        chip.set_has_frame(false);
        chip.add_css_class("category-chip");
        if active {
            chip.add_css_class("active");
        }
        category_row.append(&chip);
        let entry_for_chip = entry.clone();
        chip.connect_clicked(move |_| {
            entry_for_chip.set_text(prefix);
            entry_for_chip.grab_focus();
        });
    }
    let status = Label::new(Some("Quick Search"));
    status.set_halign(Align::End);
    status.set_hexpand(true);
    status.add_css_class("status-label");
    category_row.append(&status);
    root.append(&category_row);

    let preview_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .transition_duration(140)
        .reveal_child(false)
        .build();
    let preview_box = GtkBox::new(Orientation::Horizontal, 10);
    preview_box.add_css_class("image-preview");
    preview_box.set_hexpand(true);
    preview_revealer.set_child(Some(&preview_box));
    root.append(&preview_revealer);

    let list = ListBox::new();
    list.add_css_class("result-list");
    list.set_selection_mode(SelectionMode::Multiple);
    list.set_activate_on_single_click(true);
    list.set_vexpand(true);

    let scroll = ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .build();
    root.append(&scroll);

    let footer = Label::new(Some(
        "↑↓ navigate   •   Ctrl+Space multi-select   •   Enter copy/open   •   Esc close",
    ));
    footer.set_halign(Align::End);
    footer.add_css_class("footer-hint");
    root.append(&footer);
    window.set_child(Some(&root));

    let items = Rc::new(RefCell::new(Vec::<Item>::new()));
    let animation = Rc::new(RefCell::new(None::<glib::SourceId>));
    let generation = Rc::new(Cell::new(0_u64));
    let (sender, receiver) = async_channel::unbounded::<(u64, String, Vec<Item>)>();
    let (action_sender, action_receiver) =
        async_channel::unbounded::<anyhow::Result<Option<String>>>();
    let (request_sender, request_receiver) =
        mpsc::channel::<(u64, String, settings::LinuxSettings)>();
    let worker_paths = paths.clone();
    thread::spawn(move || {
        while let Ok((mut request_generation, mut query, mut worker_settings)) =
            request_receiver.recv()
        {
            // If typing produced multiple requests while a search was in
            // flight, only compute the newest one after the current search.
            while let Ok((next_generation, next_query, next_settings)) = request_receiver.try_recv()
            {
                request_generation = next_generation;
                query = next_query;
                worker_settings = next_settings;
            }
            let results = providers::collect(&worker_paths, &worker_settings, &query);
            if sender
                .send_blocking((request_generation, query, results))
                .is_err()
            {
                break;
            }
        }
    });

    let row_height = Rc::new(Cell::new(linux_settings.item_height.clamp(52, 120)));
    let update = |list: &ListBox,
                  status: &Label,
                  items: &[Item],
                  query: &str,
                  row_height: u32,
                  light_theme: bool| {
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        for item in items {
            list.append(&result_row(item, row_height, light_theme));
        }
        if items.is_empty() {
            let message = empty_state_message(query);
            list.append(&empty_state_row(&message));
            status.set_text(&message);
        } else {
            let source_count = items
                .iter()
                .map(|item| item.source.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len();
            status.set_text(&format!("{source_count} sources · {} results", items.len()));
        }
        list.unselect_all();
        if let Some(row) = list.row_at_index(0) {
            list.select_row(Some(&row));
        } else {
            list.select_row(None::<&ListBoxRow>);
        }
    };

    let initial_items = providers::collect(&paths, &linux_settings, "");
    *items.borrow_mut() = initial_items.clone();
    update(
        &list,
        &status,
        &initial_items,
        "",
        row_height.get(),
        theme_class(&linux_settings.theme_mode) == "light",
    );

    let generation_for_changed = generation.clone();
    let request_sender_for_changed = request_sender.clone();
    let settings_for_changed = settings_state.clone();
    let debounce_source = Rc::new(RefCell::new(None::<glib::SourceId>));
    entry.connect_changed(move |entry| {
        let next_generation = generation_for_changed.get().saturating_add(1);
        generation_for_changed.set(next_generation);
        let query = entry.text().to_string();
        if let Some(source) = debounce_source.borrow_mut().take() {
            source.remove();
        }
        let request_sender = request_sender_for_changed.clone();
        let settings = settings_for_changed.borrow().clone();
        let debounce_source_for_timeout = debounce_source.clone();
        let source = glib::timeout_add_local_once(Duration::from_millis(90), move || {
            debounce_source_for_timeout.borrow_mut().take();
            let _ = request_sender.send((next_generation, query, settings));
        });
        *debounce_source.borrow_mut() = Some(source);
    });

    let generation_for_receiver = generation.clone();
    let items_for_receiver = items.clone();
    let list_for_receiver = list.clone();
    let status_for_receiver = status.clone();
    let row_height_for_receiver = row_height.clone();
    let settings_for_receiver = settings_state.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Ok((result_generation, query, results)) = receiver.recv().await {
            if result_generation == generation_for_receiver.get() {
                *items_for_receiver.borrow_mut() = results.clone();
                update(
                    &list_for_receiver,
                    &status_for_receiver,
                    &results,
                    &query,
                    row_height_for_receiver.get(),
                    theme_class(&settings_for_receiver.borrow().theme_mode) == "light",
                );
            }
        }
    });

    let action_status = status.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Ok(result) = action_receiver.recv().await {
            match result {
                Ok(Some(feedback)) => set_launcher_status(
                    &action_status,
                    feedback.lines().next().unwrap_or(feedback.as_str()),
                ),
                Ok(None) => set_launcher_status(&action_status, "Action completed"),
                Err(error) => {
                    set_launcher_status(&action_status, &format!("Action failed: {error}"));
                    action_status.set_tooltip_text(Some(&format!("{error:#}")));
                    eprintln!("ProtonSearch: {error:#}");
                }
            }
        }
    });

    let window_for_commands = window.clone();
    let entry_for_commands = entry.clone();
    let search_shell_for_commands = search_shell.clone();
    let row_height_for_commands = row_height.clone();
    let animation_for_commands = animation.clone();
    let paths_for_commands = paths.clone();
    let settings_for_commands = settings_state.clone();
    let request_sender_for_commands = request_sender.clone();
    let generation_for_commands = generation.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Ok(command) = commands.recv().await {
            match command {
                UiCommand::Toggle if window_for_commands.is_visible() => {
                    animate_hide(&window_for_commands, &animation_for_commands);
                }
                UiCommand::Toggle => {
                    animate_show(
                        &window_for_commands,
                        &entry_for_commands,
                        &animation_for_commands,
                    );
                }
                UiCommand::Reload => {
                    let next_settings = settings::load(&paths_for_commands);
                    window_for_commands.remove_css_class("dark");
                    window_for_commands.remove_css_class("light");
                    window_for_commands.remove_css_class("system");
                    window_for_commands.add_css_class(theme_class(&next_settings.theme_mode));
                    window_for_commands.set_default_size(
                        next_settings.window_width.clamp(480, 1600) as i32,
                        next_settings.window_height.clamp(420, 1200) as i32,
                    );
                    search_shell_for_commands
                        .set_height_request(next_settings.search_bar_height.clamp(42, 100) as i32);
                    row_height_for_commands.set(next_settings.item_height.clamp(52, 120));
                    if next_settings.show_placeholder {
                        entry_for_commands
                            .set_placeholder_text(Some("Search files, code, PDFs..."));
                    } else {
                        entry_for_commands.set_placeholder_text(None);
                    }
                    *settings_for_commands.borrow_mut() = next_settings.clone();
                    let next_generation = generation_for_commands.get().saturating_add(1);
                    generation_for_commands.set(next_generation);
                    let _ = request_sender_for_commands.send((
                        next_generation,
                        entry_for_commands.text().to_string(),
                        next_settings,
                    ));
                }
            }
        }
    });

    let list_for_enter = list.clone();
    let items_for_enter = items.clone();
    let paths_for_enter = paths.clone();
    let window_for_enter = window.clone();
    let entry_for_enter = entry.clone();
    let status_for_enter = status.clone();
    let animation_for_enter = animation.clone();
    let action_sender_for_enter = action_sender.clone();
    entry.connect_activate(move |_| {
        let selected_items = list_for_enter
            .selected_rows()
            .into_iter()
            .filter_map(|row| items_for_enter.borrow().get(row.index() as usize).cloned())
            .collect::<Vec<_>>();
        if selected_items.len() > 1
            && selected_items.iter().all(|item| item.source == "Clipboard")
        {
            match providers::activate_clipboard_batch(&selected_items) {
                Ok((text_count, image_count)) => {
                    let message = if image_count > 1 {
                        format!(
                            "Copied {text_count} text item(s) and combined {image_count} images into one clipboard image"
                        )
                    } else if image_count > 0 {
                        format!(
                            "Copied {text_count} text item(s) and {image_count} image"
                        )
                    } else {
                        format!("Copied {text_count} clipboard items together")
                    };
                    set_launcher_status(&status_for_enter, &message);
                    animate_hide(&window_for_enter, &animation_for_enter);
                }
                Err(error) => {
                    set_launcher_status(&status_for_enter, &format!("Clipboard action failed: {error}"));
                }
            }
            return;
        }
        if let Some(row) = list_for_enter
            .selected_row()
            .or_else(|| list_for_enter.row_at_index(0))
        {
            let index = row.index();
            let Some(item) = items_for_enter.borrow().get(index as usize).cloned() else {
                return;
            };
            activate_item(
                &paths_for_enter,
                &window_for_enter,
                &entry_for_enter,
                &status_for_enter,
                &animation_for_enter,
                &action_sender_for_enter,
                item,
            );
        }
    });

    let paths_for_activation = paths.clone();
    let items_for_activation = items.clone();
    let list_for_activation = list.clone();
    let window_for_activation = window.clone();
    let entry_for_activation = entry.clone();
    let status_for_activation = status.clone();
    let animation_for_activation = animation.clone();
    let action_sender_for_activation = action_sender.clone();
    let preview_revealer_for_activation = preview_revealer.clone();
    let preview_box_for_activation = preview_box.clone();
    list.connect_row_activated(move |_, row| {
        let selected_items = list_for_activation
            .selected_rows()
            .into_iter()
            .filter_map(|selected| {
                items_for_activation
                    .borrow()
                    .get(selected.index() as usize)
                    .cloned()
            })
            .collect::<Vec<_>>();
        if selected_items.len() > 1
            && selected_items.iter().all(|item| item.source == "Clipboard")
        {
            match providers::activate_clipboard_batch(&selected_items) {
                Ok((text_count, image_count)) => {
                    let message = if image_count > 1 {
                        format!(
                            "Copied {text_count} text item(s) and combined {image_count} images into one clipboard image"
                        )
                    } else if image_count > 0 {
                        format!(
                            "Copied {text_count} text item(s) and {image_count} image"
                        )
                    } else {
                        format!("Copied {text_count} clipboard items together")
                    };
                    set_launcher_status(&status_for_activation, &message);
                    animate_hide(&window_for_activation, &animation_for_activation);
                }
                Err(error) => {
                    eprintln!("ProtonSearch: clipboard action failed: {error:#}");
                    set_launcher_status(
                        &status_for_activation,
                        &format!("Clipboard action failed: {error}"),
                    );
                }
            }
            return;
        }
        let index = row.index();
        let Some(item) = items_for_activation.borrow().get(index as usize).cloned() else {
            return;
        };
        if item.kind == "IMAGE" {
            show_image_preview(
                &preview_revealer_for_activation,
                &preview_box_for_activation,
                Some(&item),
            );
            return;
        }
        activate_item(
            &paths_for_activation,
            &window_for_activation,
            &entry_for_activation,
            &status_for_activation,
            &animation_for_activation,
            &action_sender_for_activation,
            item,
        );
    });

    let key_controller = EventControllerKey::new();
    let window_for_escape = window.clone();
    let animation_for_escape = animation.clone();
    let list_for_navigation = list.clone();
    let items_for_preview = items.clone();
    let preview_revealer_for_key = preview_revealer.clone();
    let preview_box_for_key = preview_box.clone();
    key_controller.connect_key_pressed(move |_, key, _, state| {
        if matches!(key, gdk::Key::Alt_L | gdk::Key::Alt_R) {
            let selected = list_for_navigation
                .selected_row()
                .or_else(|| list_for_navigation.row_at_index(0))
                .and_then(|row| {
                    items_for_preview
                        .borrow()
                        .get(row.index() as usize)
                        .cloned()
                });
            show_image_preview(
                &preview_revealer_for_key,
                &preview_box_for_key,
                selected.as_ref(),
            );
            return glib::Propagation::Proceed;
        }
        if key == gdk::Key::Escape {
            animate_hide(&window_for_escape, &animation_for_escape);
            return glib::Propagation::Stop;
        }
        if key == gdk::Key::space && state.contains(gdk::ModifierType::CONTROL_MASK) {
            if let Some(row) = list_for_navigation
                .selected_row()
                .or_else(|| list_for_navigation.row_at_index(0))
            {
                let selected = list_for_navigation
                    .selected_rows()
                    .iter()
                    .any(|selected| selected.index() == row.index());
                if selected {
                    list_for_navigation.unselect_row(&row);
                } else {
                    list_for_navigation.select_row(Some(&row));
                }
            }
            return glib::Propagation::Stop;
        }
        if matches!(
            key,
            gdk::Key::Down
                | gdk::Key::Up
                | gdk::Key::Page_Down
                | gdk::Key::Page_Up
                | gdk::Key::Home
                | gdk::Key::End
        ) {
            let count = list_for_navigation.observe_children().n_items() as i32;
            if count == 0 {
                return glib::Propagation::Stop;
            }
            let current = list_for_navigation
                .selected_row()
                .map(|row| row.index())
                .unwrap_or(0);
            let next = match key {
                gdk::Key::Down => (current + 1).min(count - 1),
                gdk::Key::Up => current.saturating_sub(1),
                gdk::Key::Page_Down => (current + 6).min(count - 1),
                gdk::Key::Page_Up => current.saturating_sub(6),
                gdk::Key::Home => 0,
                _ => count - 1,
            };
            if let Some(row) = list_for_navigation.row_at_index(next) {
                // Multiple selection is available for clipboard workflows,
                // but ordinary arrow navigation must behave like a single
                // active cursor. Holding Ctrl intentionally preserves the
                // existing selection set.
                if !state.contains(gdk::ModifierType::CONTROL_MASK) {
                    list_for_navigation.unselect_all();
                }
                list_for_navigation.select_row(Some(&row));
                row.grab_focus();
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    let preview_revealer_for_release = preview_revealer.clone();
    key_controller.connect_key_released(move |_, key, _, _| {
        if matches!(key, gdk::Key::Alt_L | gdk::Key::Alt_R) {
            preview_revealer_for_release.set_reveal_child(false);
        }
    });
    key_controller.set_propagation_phase(PropagationPhase::Capture);
    window.add_controller(key_controller);
    // The resident service starts hidden. The compositor shortcut sends a
    // toggle over the IPC socket and reveals the launcher on demand.
    window.hide();
}

fn cancel_animation(animation: &Rc<RefCell<Option<glib::SourceId>>>) {
    if let Some(source) = animation.borrow_mut().take() {
        source.remove();
    }
}

fn animate_show(
    window: &ApplicationWindow,
    entry: &Entry,
    animation: &Rc<RefCell<Option<glib::SourceId>>>,
) {
    cancel_animation(animation);
    window.set_opacity(0.0);
    window.present();
    entry.grab_focus();
    let weak_window = window.downgrade();
    let animation_for_tick = animation.clone();
    let started = Instant::now();
    let source = glib::timeout_add_local(Duration::from_millis(16), move || {
        let progress = (started.elapsed().as_secs_f64() / 0.16).min(1.0);
        let Some(window) = weak_window.upgrade() else {
            *animation_for_tick.borrow_mut() = None;
            return glib::ControlFlow::Break;
        };
        window.set_opacity(progress);
        if progress >= 1.0 {
            *animation_for_tick.borrow_mut() = None;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
    *animation.borrow_mut() = Some(source);
}

fn animate_hide(window: &ApplicationWindow, animation: &Rc<RefCell<Option<glib::SourceId>>>) {
    if !window.is_visible() {
        return;
    }
    cancel_animation(animation);
    let weak_window = window.downgrade();
    let animation_for_tick = animation.clone();
    let started = Instant::now();
    let source = glib::timeout_add_local(Duration::from_millis(16), move || {
        let progress = (started.elapsed().as_secs_f64() / 0.12).min(1.0);
        let Some(window) = weak_window.upgrade() else {
            *animation_for_tick.borrow_mut() = None;
            return glib::ControlFlow::Break;
        };
        window.set_opacity(1.0 - progress);
        if progress >= 1.0 {
            window.hide();
            window.set_opacity(1.0);
            *animation_for_tick.borrow_mut() = None;
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
    *animation.borrow_mut() = Some(source);
}

fn result_row(item: &Item, row_height: u32, light_theme: bool) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_height_request(row_height as i32);
    row.set_hexpand(true);
    row.add_css_class("result-row");
    let content = GtkBox::new(Orientation::Horizontal, 8);
    content.set_hexpand(true);
    content.set_margin_top(8);
    content.set_margin_bottom(8);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let icon = result_icon(item, light_theme);
    content.append(&icon);

    let text = GtkBox::new(Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.set_halign(Align::Fill);

    let title = Label::new(Some(&item.title));
    title.set_halign(Align::Start);
    title.set_xalign(0.0);
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title.set_single_line_mode(true);
    if light_theme {
        title.set_markup(&format!(
            "<span foreground=\"#17202a\">{}</span>",
            glib::markup_escape_text(&item.title)
        ));
    }
    title.add_css_class("result-title");
    text.append(&title);

    let subtitle = Label::new(Some(&item.subtitle));
    subtitle.set_halign(Align::Start);
    subtitle.set_xalign(0.0);
    subtitle.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    subtitle.set_single_line_mode(true);
    if light_theme {
        subtitle.set_markup(&format!(
            "<span foreground=\"#435363\">{}</span>",
            glib::markup_escape_text(&item.subtitle)
        ));
    }
    subtitle.add_css_class("result-subtitle");
    text.append(&subtitle);
    content.append(&text);

    let badge = Label::new(Some(&item.kind));
    badge.set_halign(Align::End);
    badge.set_hexpand(false);
    if light_theme {
        badge.set_markup(&format!(
            "<span foreground=\"#263746\">{}</span>",
            glib::markup_escape_text(&item.kind)
        ));
    }
    badge.add_css_class("source-badge");
    badge.set_tooltip_text(Some(&item.source));
    content.append(&badge);

    let revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .transition_duration(120)
        .build();
    revealer.set_hexpand(true);
    revealer.set_halign(Align::Fill);
    revealer.set_child(Some(&content));
    revealer.set_reveal_child(true);
    row.set_child(Some(&revealer));
    row
}

fn empty_state_message(query: &str) -> String {
    let query = query.trim().to_ascii_lowercase();
    if query.starts_with("images:") || query.starts_with("image:") || query.starts_with("ocr:") {
        return "No image files found".to_string();
    }
    if query.starts_with("clipboard:") || query.starts_with("clip:") {
        return "No clipboard history found".to_string();
    }
    if query.starts_with("git:") || query.starts_with("commits:") {
        return "No Git commits found".to_string();
    }
    "No results found".to_string()
}

fn empty_state_row(message: &str) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_selectable(false);
    row.set_activatable(false);
    let box_ = GtkBox::new(Orientation::Vertical, 5);
    box_.set_valign(Align::Center);
    box_.set_vexpand(true);
    box_.set_margin_top(48);
    box_.set_margin_bottom(48);
    let title = Label::new(Some(message));
    title.add_css_class("empty-state-title");
    title.set_halign(Align::Center);
    let hint = Label::new(Some("Try another search or choose a different category"));
    hint.add_css_class("empty-state-hint");
    hint.set_halign(Align::Center);
    box_.append(&title);
    box_.append(&hint);
    row.set_child(Some(&box_));
    row
}

fn show_image_preview(revealer: &Revealer, preview_box: &GtkBox, item: Option<&Item>) {
    let Some(item) = item else {
        revealer.set_reveal_child(false);
        return;
    };
    let Some(image) = preview_image_for_item(item) else {
        revealer.set_reveal_child(false);
        return;
    };
    while let Some(child) = preview_box.first_child() {
        preview_box.remove(&child);
    }
    image.set_pixel_size(360);
    image.set_tooltip_text(Some("Image preview · release Alt to close"));
    image.add_css_class("preview-image");
    preview_box.append(&image);
    let text = GtkBox::new(Orientation::Vertical, 3);
    text.set_valign(Align::Center);
    let title = Label::new(Some(&item.title));
    title.set_halign(Align::Start);
    title.add_css_class("preview-title");
    let hint = Label::new(Some("Release Alt to close preview · Enter opens image"));
    hint.set_halign(Align::Start);
    hint.add_css_class("preview-hint");
    text.append(&title);
    text.append(&hint);
    preview_box.append(&text);
    revealer.set_reveal_child(true);
}

fn preview_image_for_item(item: &Item) -> Option<Image> {
    match &item.target {
        Target::Path(path) if crate::search::is_image_path(path) && path.is_file() => {
            scaled_image(path, 720, 480)
        }
        Target::Cliphist(line) if item.kind == "IMAGE" => scaled_clipboard_image(line, 720, 480),
        _ => None,
    }
}

fn scaled_clipboard_image(line: &str, max_width: i32, max_height: i32) -> Option<Image> {
    if !crate::system::command_available("cliphist") {
        return None;
    }
    let mut child = Command::new("cliphist")
        .args(["decode"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(format!("{line}\n").as_bytes()).is_err() {
            let _ = child.kill();
            return None;
        }
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() || output.stdout.len() > 16 * 1024 * 1024 {
        return None;
    }
    let pixbuf = gdk_pixbuf::Pixbuf::from_read(Cursor::new(output.stdout)).ok()?;
    let scale = (max_width as f64 / pixbuf.width() as f64)
        .min(max_height as f64 / pixbuf.height() as f64)
        .min(1.0);
    let width = (pixbuf.width() as f64 * scale).round().max(1.0) as i32;
    let height = (pixbuf.height() as f64 * scale).round().max(1.0) as i32;
    let pixbuf = pixbuf.scale_simple(width, height, gdk_pixbuf::InterpType::Bilinear)?;
    let texture = gdk::Texture::for_pixbuf(&pixbuf);
    Some(Image::from_paintable(Some(&texture)))
}

fn result_icon(item: &Item, light_theme: bool) -> Image {
    if let Some(name) = result_asset_name(item) {
        if let Some(image) = crate::icons::source_for_theme(name, 32, light_theme) {
            return image;
        }
    }

    if item.kind == "IMAGE" && matches!(&item.target, Target::Cliphist(_)) {
        if let Target::Cliphist(line) = &item.target {
            if let Some(image) = scaled_clipboard_image(line, 64, 64) {
                image.set_pixel_size(48);
                image.add_css_class("result-thumbnail");
                return image;
            }
        }
        let image = Image::from_icon_name("image-x-generic-symbolic");
        image.set_pixel_size(48);
        image.add_css_class("result-thumbnail");
        return image;
    }

    let image = match &item.target {
        Target::Application(entry) if entry.name.eq_ignore_ascii_case("ProtonSearch") => {
            crate::icons::protonsearch(32)
        }
        Target::Application(entry) => entry
            .icon
            .as_deref()
            .filter(|icon| icon.starts_with('/'))
            .map(Image::from_file)
            .unwrap_or_else(|| {
                Image::from_icon_name(
                    entry
                        .icon
                        .as_deref()
                        .unwrap_or("application-x-executable-symbolic"),
                )
            }),
        Target::Path(path) if path.is_dir() => Image::from_icon_name("folder-symbolic"),
        Target::Path(path) if crate::search::is_image_path(path) => {
            let Some(image) = scaled_image(path, 64, 64) else {
                return Image::from_icon_name("image-x-generic-symbolic");
            };
            image.set_pixel_size(48);
            image.add_css_class("result-thumbnail");
            return image;
        }
        Target::Url(_) => Image::from_icon_name("web-browser-symbolic"),
        Target::Copy(_) => Image::from_icon_name("edit-copy-symbolic"),
        Target::Cliphist(_) => Image::from_icon_name("edit-copy-symbolic"),
        Target::Query(_) => Image::from_icon_name("folder-open-symbolic"),
        Target::Notice(_) => Image::from_icon_name("dialog-information-symbolic"),
        Target::Action { .. } => Image::from_icon_name("system-run-symbolic"),
        Target::Path(_) => Image::from_icon_name("text-x-generic-symbolic"),
    };
    image.set_pixel_size(32);
    image.add_css_class("result-icon");
    image
}

fn scaled_image(path: &std::path::Path, width: i32, height: i32) -> Option<Image> {
    let pixbuf = gdk_pixbuf::Pixbuf::from_file_at_scale(path, width, height, true).ok()?;
    let texture = gdk::Texture::for_pixbuf(&pixbuf);
    Some(Image::from_paintable(Some(&texture)))
}

fn result_asset_name(item: &Item) -> Option<&'static str> {
    if item.kind == "SETTING" || item.source == "Settings" {
        return Some("settings");
    }
    if item.kind == "COMMAND" || item.source == "Commands" {
        return Some("commands");
    }
    if item.kind != "SOURCE" {
        return None;
    }
    match item.title.as_str() {
        "Browser Bookmarks" => Some("browser-bookmarks"),
        "Browser History" => Some("browser-history"),
        "Git Commits" => Some("git-commits"),
        "Clipboard History" => Some("clipboard-history"),
        "Local Files" => Some("local-files"),
        "Agents" => Some("agents"),
        "Agent History" => Some("agent-history"),
        _ => Some("all"),
    }
}

fn install_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(LAUNCHER_CSS);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn theme_class(theme_mode: &str) -> &'static str {
    match theme_mode.trim().to_ascii_lowercase().as_str() {
        "light" => "light",
        "system" => "system",
        _ => "dark",
    }
}

fn apply_hyprland_hotkey(previous: &str, next: &str) {
    let Some(next) = normalize_hotkey(next) else {
        eprintln!("ProtonSearch: invalid hotkey; expected comma-separated Hyprland keys");
        return;
    };
    if !crate::system::command_available("hyprctl") {
        return;
    }
    if hyprland_uses_lua_config() {
        eprintln!(
            "ProtonSearch: Hyprland Lua bindings are managed by packaging/install-linux.sh; no runtime binding was changed"
        );
        return;
    }
    let previous = normalize_hotkey(previous).unwrap_or_else(|| "ALT,SPACE".to_string());
    let _ = crate::system::run("hyprctl", &["keyword", "unbind", &previous]);
    let Ok(executable) = std::env::current_exe() else {
        eprintln!("ProtonSearch: could not resolve the launcher executable");
        return;
    };
    let executable = executable.to_string_lossy().replace('\'', "'\\''");
    let binding = format!("{next},exec,'{executable}' gui");
    if let Ok(result) = crate::system::run("hyprctl", &["keyword", "bind", &binding]) {
        if result.status != Some(0) {
            eprintln!("ProtonSearch: Hyprland rejected the launcher hotkey");
        }
    }
}

fn unbind_hyprland_hotkey(value: &str) {
    if !crate::system::command_available("hyprctl") {
        return;
    }
    if hyprland_uses_lua_config() {
        eprintln!(
            "ProtonSearch: Hyprland Lua bindings are installer-managed; no runtime binding was removed"
        );
        return;
    }
    let value = normalize_hotkey(value).unwrap_or_else(|| "ALT,SPACE".to_string());
    let _ = crate::system::run("hyprctl", &["keyword", "unbind", &value]);
}

fn hyprland_uses_lua_config() -> bool {
    let mut candidates = Vec::new();
    if let Some(config) = std::env::var_os("HYPRLAND_CONFIG") {
        candidates.push(std::path::PathBuf::from(config));
    }
    if let Ok(paths) = XdgPaths::discover() {
        candidates.push(paths.config.join("hypr/hyprland.lua"));
    }
    candidates
        .iter()
        .any(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "lua"))
}

fn normalize_hotkey(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_uppercase();
    if value.is_empty() || value.len() > 80 || value.contains(';') || value.contains('\n') {
        return None;
    }
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    if parts
        .iter()
        .flat_map(|part| part.chars())
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '+' | '-' | ' ')))
    {
        return None;
    }
    Some(parts.join(","))
}
