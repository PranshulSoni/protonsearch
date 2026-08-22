//! GTK launcher window for the Linux implementation.
//!
//! Wayland deliberately leaves global shortcut ownership to the compositor.
//! On Hyprland the window is opened with a normal `bind` command documented in
//! `docs/linux/LAUNCHER.md`; the same desktop entry can be assigned a shortcut
//! by other desktop environments.

use crate::providers::{self, Item, Target};
use crate::settings;
use crate::update;
use crate::xdg::XdgPaths;
use anyhow::Result;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType, CheckButton,
    ComboBoxText, Entry, EventControllerFocus, EventControllerKey, EventControllerScroll,
    EventControllerScrollFlags, Image, Label, ListBox, ListBoxRow, MessageDialog, MessageType,
    Orientation, PolicyType, PropagationPhase, ResponseType, Revealer, RevealerTransitionType,
    ScrolledWindow, SelectionMode, SpinButton, Stack, StackSidebar, StackTransitionType,
};
use std::cell::{Cell, RefCell};
use std::fs;
use std::io::{BufRead, Cursor, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const APPLICATION_ID: &str = "com.protonsearch.Linux";
const LAUNCHER_CSS: &str = r#"
window.proton-window {
    background-color: #17191c;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 16px;
    font-family: sans;
}

.launcher-root {
    background-color: #202327;
    border-radius: 16px;
}

.search-header {
    min-height: 48px;
}

.brand-slot {
    min-width: 44px;
    min-height: 44px;
    background-color: #252a2f;
    border: 1px solid #3b434b;
    border-radius: 12px;
}

.brand-logo {
    min-width: 38px;
    min-height: 38px;
}

.search-shell {
    background-color: #2a2e33;
    border: 1px solid #3b434b;
    border-radius: 12px;
    padding: 0 12px;
}

.search-icon {
    color: #f3f5f7;
}

entry.search-entry {
    min-height: 44px;
    background-color: transparent;
    color: #f3f5f7;
    caret-color: #82c7bb;
    border: none;
    box-shadow: none;
    padding: 0 8px;
    font-size: 17px;
}

entry.search-entry:focus {
    border: none;
    box-shadow: inset 0 0 0 1px #82c7bb;
}

entry.search-entry placeholder,
entry.search-entry text.placeholder,
entry.search-entry > text > placeholder {
    color: #a7b0b8;
    opacity: 1;
}

entry.search-entry selection {
    color: #172027;
    background-color: #82c7bb;
}

.shortcut-badge {
    color: #a7b0b8;
    background-color: #363c42;
    border: 1px solid #4a535b;
    border-radius: 7px;
    padding: 5px 8px;
    font-size: 10px;
    font-weight: 700;
}

entry.error {
    border: 1px solid #ef6b73;
}

.category-row {
    margin-top: 4px;
    margin-bottom: 4px;
}

.category-chip {
    color: #a7b0b8;
    background-color: transparent;
    border: none;
    box-shadow: none;
    font-size: 11px;
    font-weight: 600;
    padding: 5px 8px;
    border-radius: 999px;
}

.category-chip > box {
    min-height: 18px;
}

.category-icon {
    min-width: 16px;
    min-height: 16px;
    color: #a7b0b8;
}

.category-chip.active {
    color: #e8f5f2;
    background-color: #38504d;
}

button.category-chip:hover {
    color: #f3f5f7;
    background-color: #2d353a;
}

button.category-chip.active label,
button.category-chip.active:hover label {
    color: #e8f5f2;
}

button.category-chip.active .category-icon,
button.category-chip.active:hover .category-icon,
button.category-chip:hover .category-icon {
    color: #e8f5f2;
}

.status-label {
    color: #737d86;
    font-size: 11px;
}

list.result-list {
    background-color: transparent;
    padding: 2px 0;
}

row.result-row {
    background-color: transparent;
    border-radius: 10px;
    margin: 2px 0;
}

row.compact-row {
    border: 1px solid transparent;
}

row.source-row {
    background-color: #252a2f;
    border: 1px solid #30373d;
    margin: 4px 0;
}

row.source-row:hover {
    background-color: #2d353a;
    border-color: #465159;
}

row.source-row .result-title {
    font-size: 14px;
}

row.source-row .result-subtitle {
    color: #b4bec4;
}

row.source-row .asset-icon {
    min-width: 38px;
    min-height: 38px;
}

row.result-row:hover {
    background-color: #2d3339;
}

list.result-list > row.result-row:selected {
    background-color: #30383e;
    border: 1px solid #53616a;
}

list.result-list > row.result-row.cursor-row,
list.result-list > row.result-row.cursor-row:selected,
list.result-list > row.result-row.cursor-row:focus {
    background-color: #38444f;
    border: 1px solid #82c7bb;
    box-shadow: inset 3px 0 0 #82c7bb, 0 0 0 1px rgba(130, 199, 187, 0.16);
}

.result-icon {
    margin-right: 0;
}

.result-thumbnail {
    min-width: 44px;
    min-height: 44px;
    border-radius: 8px;
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
    color: #f3f5f7;
    font-size: 14px;
    font-weight: 600;
}

.asset-icon {
    min-width: 34px;
    min-height: 34px;
}

.result-title {
    color: #f3f5f7;
    font-size: 14px;
    font-weight: 600;
}

.result-subtitle {
    color: #a7b0b8;
    font-size: 12px;
}

.source-badge {
    color: #b8c8c7;
    background-color: #313941;
    border-radius: 999px;
    padding: 4px 9px;
    font-size: 9px;
    font-weight: 700;
}

.badge-file, .badge-folder, .badge-image, .badge-code, .badge-command,
.badge-setting, .badge-source, .badge-clipboard {
    background-color: #313941;
}

row.result-row.cursor-row .source-badge,
row.result-row.cursor-row:selected .source-badge {
    color: #e8f5f2;
    background-color: #49635e;
}

list.result-list > row.result-row:selected label.result-title,
list.result-list > row.result-row.cursor-row label.result-title {
    color: #f8fbfc;
}

list.result-list > row.result-row:selected label.result-subtitle,
list.result-list > row.result-row.cursor-row label.result-subtitle {
    color: #c4d0d4;
}

.footer-hint {
    color: #737d86;
    font-size: 10px;
}

window.proton-image-preview {
    background-color: #151617;
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 12px;
}

.preview-shell {
    background-color: #151617;
    border-radius: 12px;
    padding: 12px;
}

.preview-toolbar {
    min-height: 32px;
}

.preview-window-title {
    color: #f1f2f3;
    font-size: 13px;
    font-weight: 700;
}

.preview-close {
    min-width: 30px;
    min-height: 30px;
    padding: 0;
    border-radius: 7px;
    color: #c9cdd1;
}

.preview-close:hover {
    background-color: #3b3d40;
    color: #ffffff;
}

.preview-surface {
    background-color: #0d0e0f;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    min-width: 420px;
    min-height: 300px;
}

.preview-loading,
.preview-error {
    color: #9da3a8;
    font-size: 12px;
}

window.proton-image-preview.light {
    background-color: #f4f5f6;
    border-color: rgba(32, 35, 38, 0.18);
}

window.proton-image-preview.light .preview-shell {
    background-color: #f4f5f6;
}

window.proton-image-preview.light .preview-window-title {
    color: #202326;
}

window.proton-image-preview.light .preview-close {
    color: #4b535b;
}

window.proton-image-preview.light .preview-close:hover {
    background-color: #dfe4e8;
    color: #202326;
}

window.proton-image-preview.light .preview-surface {
    background-color: #ffffff;
    border-color: rgba(32, 35, 38, 0.14);
}

window.proton-image-preview.light .preview-loading,
window.proton-image-preview.light .preview-error {
    color: #5e646a;
}

window.proton-window.light {
    background-color: #eef1f3;
    border-color: rgba(23, 32, 39, 0.18);
}

window.proton-window.light .launcher-root {
    background-color: #ffffff;
}

window.proton-window.light .brand-slot {
    background-color: #f8fafb;
    border-color: #cbd5d9;
}

window.proton-window.light list.result-list {
    background-color: #ffffff;
}

window.proton-window.light .search-shell {
    background-color: #f8fafb;
    border-color: #cbd5d9;
}

window.proton-window.light .search-icon {
    color: #26343d;
}

window.proton-window.light entry.search-entry,
window.proton-window.light .result-title,
window.proton-window.light .empty-state-title {
    color: #172027;
}

window.proton-window.light entry.search-entry {
    caret-color: #26796d;
}

window.proton-window.light .shortcut-badge {
    color: #38515b;
    background-color: #e4eceb;
    border-color: #c1d5d1;
}

window.proton-window.light entry.search-entry placeholder,
window.proton-window.light entry.search-entry text.placeholder,
window.proton-window.light entry.search-entry > text > placeholder {
    color: #53636d;
    opacity: 1;
}

window.proton-window.light entry.search-entry text,
window.proton-window.light entry.search-entry selection {
    color: #172027;
}

window.proton-window.light entry.search-entry selection {
    background-color: #b8ded7;
}

window.proton-window.light .category-chip {
    color: #53636d;
    background-color: transparent;
}

window.proton-window.light button.category-chip label {
    color: #53636d;
}

window.proton-window.light button.category-chip:hover {
    color: #172027;
    background-color: #e8eef1;
}

window.proton-window.light button.category-chip.active,
window.proton-window.light button.category-chip.active:hover {
    color: #155a51;
    background-color: #d7eae6;
}

window.proton-window.light button.category-chip.active label,
window.proton-window.light button.category-chip.active:hover label {
    color: #155a51;
}

window.proton-window.light .category-icon {
    color: #53636d;
}

window.proton-window.light button.category-chip:hover .category-icon,
window.proton-window.light button.category-chip.active .category-icon,
window.proton-window.light button.category-chip.active:hover .category-icon {
    color: #155a51;
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

window.proton-window.light row.source-row {
    background-color: #f7f9fa;
    border-color: #e0e6e9;
}

window.proton-window.light row.source-row:hover {
    background-color: #e9edf1;
    border-color: #cbd5d9;
}

window.proton-window.light row.source-row .result-subtitle {
    color: #53636d;
}

window.proton-window.light row.result-row:hover {
    background-color: #e9edf1;
}

window.proton-window.light list.result-list > row.result-row:selected {
    background-color: #e4ecef;
    border-color: #b6c9cc;
}

window.proton-window.light list.result-list > row.result-row.cursor-row,
window.proton-window.light list.result-list > row.result-row.cursor-row:selected,
window.proton-window.light list.result-list > row.result-row.cursor-row:focus {
    background-color: #d7eae6;
    border-color: #26796d;
    box-shadow: inset 3px 0 0 #26796d, 0 0 0 1px rgba(38, 121, 109, 0.16);
}

window.proton-window.light list.result-list > row.result-row:selected label.result-title,
window.proton-window.light list.result-list > row.result-row.cursor-row label.result-title {
    color: #17202a;
}

window.proton-window.light list.result-list > row.result-row:selected label.result-subtitle,
window.proton-window.light list.result-list > row.result-row.cursor-row label.result-subtitle {
    color: #38515b;
}

window.proton-window.light list.result-list > row.result-row:selected label.source-badge,
window.proton-window.light list.result-list > row.result-row.cursor-row label.source-badge {
    color: #155a51;
    background-color: #c1dfd9;
}

window.proton-window.light .source-badge,
window.proton-window.light .badge-file,
window.proton-window.light .badge-folder,
window.proton-window.light .badge-image,
window.proton-window.light .badge-code,
window.proton-window.light .badge-command,
window.proton-window.light .badge-setting,
window.proton-window.light .badge-source,
window.proton-window.light .badge-clipboard {
    color: #53636d;
    background-color: #e4eceb;
}

window.proton-window.light row.result-row:hover .result-icon,
window.proton-window.light row.result-row.cursor-row .result-icon,
window.proton-window.light row.result-row:hover .asset-icon,
window.proton-window.light row.result-row.cursor-row .asset-icon {
    color: #1f655c;
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

.agent-root {
    background-color: #202327;
    min-height: 420px;
}

.agent-sidebar {
    background-color: #181a1d;
    min-width: 190px;
    padding: 16px 10px;
}

.agent-content {
    background-color: #202327;
}

.agent-header {
    min-height: 42px;
}

.agent-status {
    color: #9da6af;
    font-size: 11px;
}

.agent-history-list row,
.agent-new-chat,
.agent-back {
    color: #d9dde1;
    border-radius: 8px;
}

.agent-history-list row {
    padding: 9px 8px;
    margin: 2px 0;
}

.agent-history-list row:hover,
.agent-history-list row:selected {
    background-color: #30363d;
    color: #ffffff;
}

.agent-chat {
    background-color: transparent;
    padding: 8px 2px;
}

.agent-message {
    padding: 11px 14px;
    margin: 5px 4px;
    border-radius: 12px;
    font-size: 13px;
}

.agent-user {
    background-color: #3a5361;
    color: #ffffff;
}

.agent-assistant {
    background-color: #2a2e33;
    color: #f1f2f3;
}

.agent-empty {
    color: #969da5;
    padding: 30px;
}

.agent-input-row {
    background-color: #292e34;
    border: 1px solid #3b434b;
    border-radius: 11px;
    padding: 6px;
}

entry.agent-prompt {
    background-color: transparent;
    color: #f1f2f3;
    border: none;
    box-shadow: none;
}

.quick-side-preview {
    min-width: 300px;
    background-color: #181a1d;
    border-left: 1px solid #3b434b;
    padding: 14px;
}

.quick-side-preview-title {
    color: #f1f2f3;
    font-size: 13px;
    font-weight: 700;
}

.quick-side-preview-surface {
    background-color: #111315;
    border-radius: 10px;
    padding: 10px;
}

window.proton-window.light .quick-side-preview {
    background-color: #f2f3f4;
    border-left-color: #d2d6da;
}

window.proton-window.light .quick-side-preview-title {
    color: #202225;
}

window.proton-window.light .quick-side-preview-surface {
    background-color: #ffffff;
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
    let clipboard_monitor = Rc::new(RefCell::new(None::<crate::clipboard::ClipboardMonitor>));
    let clipboard_monitor_for_startup = clipboard_monitor.clone();
    let clipboard_paths = paths.clone();
    application.connect_startup(
        move |_| match crate::clipboard::start(clipboard_paths.clone()) {
            Ok(monitor) => {
                *clipboard_monitor_for_startup.borrow_mut() = Some(monitor);
            }
            Err(error) => {
                crate::clipboard::mark_unavailable(
                    &clipboard_paths,
                    format!("Clipboard monitoring is unavailable: {error}"),
                );
                eprintln!("ProtonSearch: clipboard monitoring unavailable: {error:#}");
            }
        },
    );
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

#[derive(Clone)]
struct AgentUi {
    history: ListBox,
    chat: ListBox,
    prompt: Entry,
    send: Button,
    status: Label,
    back: Button,
    new_chat: Button,
    paths: XdgPaths,
    conversations: Rc<RefCell<Vec<crate::agent::Conversation>>>,
    active: Rc<RefCell<Option<String>>>,
}

fn build_agent_view(paths: &XdgPaths) -> (GtkBox, AgentUi) {
    let conversations = Rc::new(RefCell::new(crate::agent::load_history(paths)));
    let active = Rc::new(RefCell::new(None::<String>));
    let history = ListBox::new();
    history.set_selection_mode(SelectionMode::Single);
    history.add_css_class("agent-history-list");
    history.set_width_request(190);

    let sidebar = GtkBox::new(Orientation::Vertical, 10);
    sidebar.add_css_class("agent-sidebar");
    let sidebar_title = Label::new(Some("Conversations"));
    sidebar_title.set_halign(Align::Start);
    sidebar_title.add_css_class("settings-section-title");
    sidebar.append(&sidebar_title);
    let new_chat = Button::with_label("＋ New chat");
    new_chat.add_css_class("agent-new-chat");
    sidebar.append(&new_chat);
    let history_scroll = ScrolledWindow::builder()
        .child(&history)
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .build();
    sidebar.append(&history_scroll);

    let content = GtkBox::new(Orientation::Vertical, 12);
    content.add_css_class("agent-content");
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);
    let header = GtkBox::new(Orientation::Horizontal, 10);
    header.add_css_class("agent-header");
    let back = Button::with_label("‹ Back");
    back.add_css_class("agent-back");
    header.append(&back);
    let heading = Label::new(Some("ProtonSearch Agent"));
    heading.set_halign(Align::Start);
    heading.set_hexpand(true);
    heading.add_css_class("settings-heading");
    header.append(&heading);
    let hermes_readiness = crate::hermes::readiness();
    let status_text = match hermes_readiness.command {
        None => "Hermes Agent is not installed".to_string(),
        Some(command) if hermes_readiness.ready => format!(
            "{} · local gateway ready",
            hermes_readiness
                .version
                .unwrap_or_else(|| command.program().to_string())
        ),
        Some(command) => format!(
            "{} installed · local gateway not running",
            hermes_readiness
                .version
                .unwrap_or_else(|| command.program().to_string())
        ),
    };
    let status = Label::new(Some(&status_text));
    status.set_halign(Align::End);
    status.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    status.set_max_width_chars(34);
    status.add_css_class("agent-status");
    header.append(&status);
    content.append(&header);

    let chat = ListBox::new();
    chat.set_selection_mode(SelectionMode::None);
    chat.add_css_class("agent-chat");
    let chat_scroll = ScrolledWindow::builder()
        .child(&chat)
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .build();
    content.append(&chat_scroll);

    let prompt = Entry::new();
    prompt.set_hexpand(true);
    prompt.set_placeholder_text(Some("Ask Hermes Agent anything…"));
    prompt.set_activates_default(true);
    prompt.add_css_class("agent-prompt");
    let send = Button::with_label("Send");
    send.add_css_class("suggested-action");
    send.set_receives_default(true);
    let input_row = GtkBox::new(Orientation::Horizontal, 8);
    input_row.add_css_class("agent-input-row");
    input_row.append(&prompt);
    input_row.append(&send);
    content.append(&input_row);
    let hint = Label::new(Some(
        "Enter sends · Esc returns to search · responses run in the background",
    ));
    hint.set_halign(Align::End);
    hint.add_css_class("footer-hint");
    content.append(&hint);

    let root = GtkBox::new(Orientation::Horizontal, 0);
    root.add_css_class("agent-root");
    root.append(&sidebar);
    root.append(&content);
    let ui = AgentUi {
        history,
        chat,
        prompt,
        send,
        status,
        back,
        new_chat,
        paths: paths.clone(),
        conversations,
        active,
    };
    agent_render_history(&ui);
    agent_render_chat(&ui);
    (root, ui)
}

fn agent_render_history(ui: &AgentUi) {
    while let Some(child) = ui.history.first_child() {
        ui.history.remove(&child);
    }
    for conversation in ui.conversations.borrow().iter().rev() {
        let label = Label::new(Some(&conversation.title));
        label.set_halign(Align::Start);
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        label.set_max_width_chars(22);
        let row = ListBoxRow::new();
        row.set_child(Some(&label));
        row.set_tooltip_text(Some(&format!("{} messages", conversation.messages.len())));
        row.set_widget_name(&conversation.id);
        ui.history.append(&row);
    }
}

fn agent_render_chat(ui: &AgentUi) {
    while let Some(child) = ui.chat.first_child() {
        ui.chat.remove(&child);
    }
    let active = ui.active.borrow().clone();
    let Some(active) = active else {
        let empty = Label::new(Some("Start a new conversation with Hermes Agent."));
        empty.add_css_class("agent-empty");
        empty.set_vexpand(true);
        ui.chat.append(&empty);
        return;
    };
    let Some(conversation) = ui
        .conversations
        .borrow()
        .iter()
        .find(|conversation| conversation.id == active)
        .cloned()
    else {
        return;
    };
    if conversation.messages.is_empty() {
        let empty = Label::new(Some(
            "Ask Hermes anything. Your conversation will be saved here.",
        ));
        empty.add_css_class("agent-empty");
        ui.chat.append(&empty);
    }
    for message in conversation.messages {
        let bubble = Label::new(Some(&message.text));
        bubble.set_wrap(true);
        bubble.set_selectable(true);
        bubble.set_xalign(0.0);
        bubble.set_halign(if message.role == "user" {
            Align::End
        } else {
            Align::Start
        });
        bubble.set_hexpand(false);
        bubble.set_max_width_chars(90);
        bubble.add_css_class("agent-message");
        bubble.add_css_class(if message.role == "user" {
            "agent-user"
        } else {
            "agent-assistant"
        });
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_activatable(false);
        row.set_child(Some(&bubble));
        ui.chat.append(&row);
    }
}

fn agent_new_conversation(ui: &AgentUi, session: Option<String>) {
    let id = session.clone().unwrap_or_else(|| {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        format!("chat-{stamp}")
    });
    if !ui.conversations.borrow().iter().any(|item| item.id == id) {
        ui.conversations
            .borrow_mut()
            .push(crate::agent::Conversation {
                id: id.clone(),
                title: "New conversation".to_string(),
                hermes_session: session,
                messages: Vec::new(),
            });
        let _ = crate::agent::save_history(&ui.paths, &ui.conversations.borrow());
    } else if let Some(session) = session {
        if let Some(conversation) = ui
            .conversations
            .borrow_mut()
            .iter_mut()
            .find(|conversation| conversation.id == id)
        {
            conversation.hermes_session = Some(session);
        }
    }
    *ui.active.borrow_mut() = Some(id);
    agent_render_history(ui);
    agent_render_chat(ui);
    ui.prompt.grab_focus();
}

fn connect_agent_actions(ui: &AgentUi) {
    let (sender, receiver) = async_channel::unbounded::<(String, crate::hermes::HermesEvent)>();
    let ui_for_receiver = ui.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Ok((id, event)) = receiver.recv().await {
            match event {
                crate::hermes::HermesEvent::Output(line) => {
                    if let Some(conversation) = ui_for_receiver
                        .conversations
                        .borrow_mut()
                        .iter_mut()
                        .find(|conversation| conversation.id == id)
                    {
                        if let Some(last) = conversation.messages.last_mut() {
                            if last.role == "assistant" {
                                if !last.text.is_empty() {
                                    last.text.push('\n');
                                }
                                last.text.push_str(&line);
                            } else {
                                conversation.messages.push(crate::agent::Message {
                                    role: "assistant".to_string(),
                                    text: line,
                                });
                            }
                        } else {
                            conversation.messages.push(crate::agent::Message {
                                role: "assistant".to_string(),
                                text: line,
                            });
                        }
                    }
                    ui_for_receiver.status.set_text("Hermes is responding…");
                    agent_render_chat(&ui_for_receiver);
                }
                crate::hermes::HermesEvent::Delta(fragment) => {
                    if let Some(conversation) = ui_for_receiver
                        .conversations
                        .borrow_mut()
                        .iter_mut()
                        .find(|conversation| conversation.id == id)
                    {
                        if let Some(last) = conversation.messages.last_mut() {
                            if last.role == "assistant" {
                                last.text.push_str(&fragment);
                            } else {
                                conversation.messages.push(crate::agent::Message {
                                    role: "assistant".to_string(),
                                    text: fragment,
                                });
                            }
                        } else {
                            conversation.messages.push(crate::agent::Message {
                                role: "assistant".to_string(),
                                text: fragment,
                            });
                        }
                    }
                    ui_for_receiver.status.set_text("Hermes is responding…");
                    agent_render_chat(&ui_for_receiver);
                }
                crate::hermes::HermesEvent::Diagnostic(line) => {
                    if !line.trim().is_empty() {
                        ui_for_receiver.status.set_text(&format!("Hermes: {line}"));
                    }
                }
                crate::hermes::HermesEvent::Approval { tool, summary } => {
                    ui_for_receiver
                        .status
                        .set_text(&format!("Approval needed for {tool}: {summary}"));
                }
                crate::hermes::HermesEvent::Finished(exit) => {
                    ui_for_receiver.send.set_sensitive(true);
                    if exit.success {
                        let _ = crate::agent::save_history(
                            &ui_for_receiver.paths,
                            &ui_for_receiver.conversations.borrow(),
                        );
                        ui_for_receiver.status.set_text("Response received");
                    } else {
                        ui_for_receiver
                            .status
                            .set_text("Hermes could not complete the request");
                    }
                    agent_render_chat(&ui_for_receiver);
                    agent_render_history(&ui_for_receiver);
                }
            }
        }
    });

    let submit: Rc<dyn Fn()> = {
        let ui = ui.clone();
        let sender = sender.clone();
        Rc::new(move || {
            let value = ui.prompt.text().trim().to_string();
            if value.is_empty() || !ui.send.is_sensitive() {
                return;
            }
            if ui.active.borrow().is_none() {
                agent_new_conversation(&ui, None);
            }
            let Some(id) = ui.active.borrow().clone() else {
                return;
            };
            let session = ui
                .conversations
                .borrow()
                .iter()
                .find(|conversation| conversation.id == id)
                .and_then(|conversation| conversation.hermes_session.clone());
            if let Some(conversation) = ui
                .conversations
                .borrow_mut()
                .iter_mut()
                .find(|conversation| conversation.id == id)
            {
                if conversation.title == "New conversation" {
                    conversation.title = value.chars().take(42).collect();
                }
                conversation.messages.push(crate::agent::Message {
                    role: "user".to_string(),
                    text: value.clone(),
                });
                let _ = crate::agent::save_history(&ui.paths, &ui.conversations.borrow());
            }
            agent_render_chat(&ui);
            agent_render_history(&ui);
            ui.prompt.set_text("");
            ui.send.set_sensitive(false);
            ui.status.set_text("Hermes is thinking…");
            let sender_for_worker = sender.clone();
            let sender_for_error = sender.clone();
            thread::spawn(move || {
                let id_for_callback = id.clone();
                let result = crate::hermes::run_prompt_best_effort_async(
                    value,
                    session.as_deref(),
                    move |event| {
                        let _ = sender_for_worker.send_blocking((id_for_callback.clone(), event));
                    },
                );
                if let Err(error) = result {
                    let _ = sender_for_error.send_blocking((
                        id,
                        crate::hermes::HermesEvent::Diagnostic(format!("{error}")),
                    ));
                    let _ = sender_for_error.send_blocking((
                        String::new(),
                        crate::hermes::HermesEvent::Finished(crate::hermes::HermesExit {
                            status: None,
                            success: false,
                        }),
                    ));
                }
            });
        })
    };
    let submit_for_click = submit.clone();
    ui.send.connect_clicked(move |_| submit_for_click());
    let submit_for_enter = submit.clone();
    ui.prompt.connect_activate(move |_| submit_for_enter());

    let ui_for_new = ui.clone();
    ui.new_chat
        .connect_clicked(move |_| agent_new_conversation(&ui_for_new, None));
    let ui_for_history = ui.clone();
    ui.history.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            *ui_for_history.active.borrow_mut() = Some(row.widget_name().to_string());
            agent_render_chat(&ui_for_history);
            ui_for_history.prompt.grab_focus();
        }
    });
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
        let (agent_page, agent_scroll) = settings_page(
            "Agent",
            "Use the installed Hermes Agent and reopen its saved sessions.",
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
        let (updates_page, updates_scroll) = settings_page(
            "Updates",
            "Check official ProtonSearch releases and install verified Linux updates safely.",
        );

        stack.add_titled(&general_scroll, Some("general"), "General");
        stack.add_titled(&appearance_scroll, Some("appearance"), "Appearance");
        stack.add_titled(&search_scroll, Some("search"), "Search");
        stack.add_titled(&providers_scroll, Some("providers"), "Providers");
        stack.add_titled(&agent_scroll, Some("agent"), "Agent");
        stack.add_titled(&hotkey_scroll, Some("hotkey"), "Hotkey");
        stack.add_titled(&safety_scroll, Some("safety"), "Safety & Linux");
        stack.add_titled(&indexing_scroll, Some("indexing"), "Indexing & Database");
        stack.add_titled(&updates_scroll, Some("updates"), "Updates");

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
        let image_preview_label = Label::new(Some("Image quick preview"));
        image_preview_label.set_halign(Align::Start);
        image_preview_label.add_css_class("settings-label");
        search_page.append(&image_preview_label);
        let image_preview_mode = ComboBoxText::new();
        image_preview_mode.append(Some("floating"), "Floating preview");
        image_preview_mode.append(Some("side"), "Expand preview to the right");
        let image_preview_mode_id = if current.image_preview_mode == "side" {
            "side"
        } else {
            "floating"
        };
        image_preview_mode.set_active_id(Some(image_preview_mode_id));
        image_preview_mode.set_tooltip_text(Some(
            "Choose the temporary preview shown while holding Alt on an image",
        ));
        search_page.append(&image_preview_mode);
        let include_hidden = CheckButton::with_label("Include hidden files in search");
        include_hidden.set_active(current.include_hidden);
        search_page.append(&include_hidden);
        let terminal_apps = CheckButton::with_label("Include terminal applications");
        terminal_apps.set_active(current.show_terminal_apps);
        search_page.append(&terminal_apps);

        let home_filters_title = Label::new(Some("Home screen filters"));
        home_filters_title.set_halign(Align::Start);
        home_filters_title.add_css_class("settings-section-title");
        search_page.append(&home_filters_title);
        let home_filters_help = Label::new(Some(
            "Choose which compact filters appear beside the search field. All is always kept available; use the arrows to change their order.",
        ));
        home_filters_help.set_wrap(true);
        home_filters_help.set_halign(Align::Start);
        home_filters_help.add_css_class("settings-help");
        search_page.append(&home_filters_help);
        let home_filter_order = Rc::new(RefCell::new(current.home_filter_order.clone()));
        let home_filter_checks = Rc::new(RefCell::new(Vec::<(String, CheckButton)>::new()));
        let home_filter_order_label = Label::new(None);
        home_filter_order_label.set_wrap(true);
        home_filter_order_label.set_halign(Align::Start);
        home_filter_order_label.add_css_class("settings-help");
        refresh_home_filter_order_label(&home_filter_order_label, &home_filter_order.borrow());
        search_page.append(&home_filter_order_label);
        for filter_id in current.home_filter_order.iter() {
            let row = GtkBox::new(Orientation::Horizontal, 6);
            row.set_hexpand(true);
            let check = CheckButton::with_label(home_filter_label(filter_id));
            check.set_active(current.home_filters.iter().any(|id| id == filter_id));
            check.set_sensitive(filter_id != "all");
            row.append(&check);
            let spacer = GtkBox::new(Orientation::Horizontal, 0);
            spacer.set_hexpand(true);
            row.append(&spacer);
            let up = Button::with_label("↑");
            let down = Button::with_label("↓");
            up.set_tooltip_text(Some("Move this filter earlier"));
            down.set_tooltip_text(Some("Move this filter later"));
            let order_for_up = home_filter_order.clone();
            let label_for_up = home_filter_order_label.clone();
            let id_for_up = filter_id.clone();
            up.connect_clicked(move |_| {
                move_home_filter(&order_for_up, &id_for_up, -1, &label_for_up);
            });
            let order_for_down = home_filter_order.clone();
            let label_for_down = home_filter_order_label.clone();
            let id_for_down = filter_id.clone();
            down.connect_clicked(move |_| {
                move_home_filter(&order_for_down, &id_for_down, 1, &label_for_down);
            });
            row.append(&up);
            row.append(&down);
            search_page.append(&row);
            home_filter_checks
                .borrow_mut()
                .push((filter_id.clone(), check));
        }

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

        let agent_label = Label::new(Some("Hermes Agent integration"));
        agent_label.set_halign(Align::Start);
        agent_label.add_css_class("settings-section-title");
        agent_page.append(&agent_label);
        let agent_status = if crate::system::command_available("hermes")
            || crate::system::command_available("hermes-agent")
        {
            "Hermes Agent detected on PATH. Agent results open ProtonSearch's internal prompt window."
        } else {
            "Hermes Agent is not installed. Install it separately, then restart ProtonSearch."
        };
        let agent_status_label = Label::new(Some(agent_status));
        agent_status_label.set_wrap(true);
        agent_status_label.set_halign(Align::Start);
        agent_status_label.add_css_class("settings-help");
        agent_page.append(&agent_status_label);
        let hermes = CheckButton::with_label("Enable Hermes Agent integration");
        hermes.set_active(current.enable_hermes);
        hermes.set_tooltip_text(Some(
            "Uses the installed Hermes Agent command when available",
        ));
        agent_page.append(&hermes);
        let agent_history = CheckButton::with_label("Enable Hermes Agent history");
        agent_history.set_active(current.enable_agent_history);
        agent_history.set_tooltip_text(Some(
            "Lists recent Hermes sessions and opens them in the ProtonSearch Agent window",
        ));
        agent_page.append(&agent_history);

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

        let update_snapshot = Rc::new(RefCell::new(update::load(&paths)));
        let update_current = Label::new(None);
        let update_latest = Label::new(None);
        let update_status = Label::new(None);
        let update_release = Label::new(None);
        let update_package = Label::new(None);
        let update_notes = Label::new(None);
        for label in [
            &update_current,
            &update_latest,
            &update_status,
            &update_release,
            &update_package,
            &update_notes,
        ] {
            label.set_halign(Align::Start);
            label.set_wrap(true);
            label.set_selectable(true);
            label.add_css_class("settings-help");
            updates_page.append(label);
        }
        let update_check = Button::with_label("Check for updates");
        let update_notes_button = Button::with_label("View release notes");
        let update_install = Button::with_label("Download and install");
        let update_buttons = GtkBox::new(Orientation::Horizontal, 8);
        update_buttons.append(&update_check);
        update_buttons.append(&update_notes_button);
        update_buttons.append(&update_install);
        updates_page.append(&update_buttons);
        let automatic_updates = CheckButton::with_label("Automatically check for updates once per day");
        automatic_updates.set_active(current.auto_update_checks);
        automatic_updates.set_tooltip_text(Some(
            "Checks GitHub after startup at most once every 24 hours; installation always requires approval.",
        ));
        updates_page.append(&automatic_updates);
        let update_help = Label::new(Some(
            "Only releases from the official ProtonSearch GitHub repository are considered. Downloads require a matching SHA-256 checksum before installation. Package-managed installs use the system package manager; user-data directories are preserved.",
        ));
        update_help.set_wrap(true);
        update_help.set_halign(Align::Start);
        update_help.add_css_class("settings-help");
        updates_page.append(&update_help);
        render_update_snapshot(
            &update_snapshot.borrow(),
            &update_current,
            &update_latest,
            &update_status,
            &update_release,
            &update_package,
            &update_notes,
            &update_notes_button,
            &update_install,
        );
        if let Some(install_state) = update::load_install_state(&paths) {
            update_status.set_text(&format!(
                "{}: {}",
                match install_state.stage {
                    update::InstallStage::Complete => "Update complete",
                    update::InstallStage::Failed => "Update failed",
                    _ => "Update in progress",
                },
                install_state.message
            ));
        }

        let (update_tx, update_rx) = mpsc::channel::<UpdateUiMessage>();
        let update_receiver = Rc::new(RefCell::new(update_rx));
        let paths_for_update = paths.clone();
        let tx_for_check = update_tx.clone();
        let check_for_updates = Rc::new(move |force: bool| {
            let paths = paths_for_update.clone();
            let tx = tx_for_check.clone();
            let _ = tx.send(UpdateUiMessage::Checking);
            thread::spawn(move || {
                let result = update::check(&paths, force).map_err(|error| error.to_string());
                let _ = tx.send(UpdateUiMessage::Checked(result));
            });
        });
        let check_for_click = check_for_updates.clone();
        update_check.connect_clicked(move |_| check_for_click(true));
        let update_url = update_snapshot.clone();
        update_notes_button.connect_clicked(move |_| {
            if let Some(url) = update_url.borrow().release_url.as_deref() {
                if let Err(error) = crate::system::open_target(url) {
                    eprintln!("ProtonSearch: could not open release notes: {error:#}");
                }
            }
        });
        let paths_for_install = paths.clone();
        let snapshot_for_install = update_snapshot.clone();
        let tx_for_install = update_tx.clone();
        update_install.connect_clicked(move |_| {
            let snapshot = snapshot_for_install.borrow().clone();
            let paths = paths_for_install.clone();
            let tx = tx_for_install.clone();
            thread::spawn(move || {
                let result = update::download_and_verify(&paths, &snapshot, |done, total| {
                    let _ = tx.send(UpdateUiMessage::DownloadProgress(done, total));
                })
                .map_err(|error| error.to_string());
                let _ = tx.send(UpdateUiMessage::Downloaded(result));
            });
        });
        let auto_check = automatic_updates.is_active();
        if auto_check {
            let check_for_startup = check_for_updates.clone();
            glib::timeout_add_local_once(Duration::from_secs(3), move || check_for_startup(false));
        }
        let receiver_for_ui = update_receiver.clone();
        let snapshot_for_ui = update_snapshot.clone();
        let update_current_for_ui = update_current.clone();
        let update_latest_for_ui = update_latest.clone();
        let update_status_for_ui = update_status.clone();
        let update_release_for_ui = update_release.clone();
        let update_package_for_ui = update_package.clone();
        let update_notes_for_ui = update_notes.clone();
        let update_install_for_ui = update_install.clone();
        let update_check_for_ui = update_check.clone();
        let paths_for_update_ui = paths.clone();
        glib::timeout_add_local(Duration::from_millis(200), move || {
            while let Ok(message) = receiver_for_ui.borrow().try_recv() {
                match message {
                    UpdateUiMessage::Checking => {
                        update_status_for_ui.set_text("Checking for updates…");
                        update_check_for_ui.set_sensitive(false);
                        update_install_for_ui.set_sensitive(false);
                    }
                    UpdateUiMessage::Checked(result) => match result {
                        Ok(snapshot) => {
                            *snapshot_for_ui.borrow_mut() = snapshot.clone();
                            render_update_snapshot(
                                &snapshot,
                                &update_current_for_ui,
                                &update_latest_for_ui,
                                &update_status_for_ui,
                                &update_release_for_ui,
                                &update_package_for_ui,
                                &update_notes_for_ui,
                                &update_notes_button,
                                &update_install_for_ui,
                            );
                            update_check_for_ui.set_sensitive(true);
                        }
                        Err(error) => {
                            update_status_for_ui.set_text(&format!("Unable to check for updates: {error}"));
                            update_check_for_ui.set_sensitive(true);
                        }
                    },
                    UpdateUiMessage::DownloadProgress(done, total) => {
                        let status = total
                            .map(|total| format!("Downloading update… {}%", done.saturating_mul(100) / total.max(1)))
                            .unwrap_or_else(|| "Downloading update…".to_string());
                        update_status_for_ui.set_text(&status);
                    }
                    UpdateUiMessage::Downloaded(result) => match result {
                        Ok(downloaded) => {
                            match update::launch_install_helper(&paths_for_update_ui, &downloaded) {
                                Ok(()) => update_status_for_ui.set_text(
                                    "Update verified. ProtonSearch is restarting to install it…",
                                ),
                                Err(error) => update_status_for_ui
                                    .set_text(&format!("Update could not start: {error}")),
                            }
                            update_check_for_ui.set_sensitive(true);
                        }
                        Err(error) => {
                            update_status_for_ui.set_text(&format!("Update verification failed: {error}"));
                            update_install_for_ui.set_sensitive(true);
                            update_check_for_ui.set_sensitive(true);
                        }
                    },
                }
            }
            glib::ControlFlow::Continue
        });

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
        let home_filter_order_for_save = home_filter_order.clone();
        let home_filter_checks_for_save = home_filter_checks.clone();
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
            next.image_preview_mode = image_preview_mode
                .active_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "floating".to_string());
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
            next.auto_update_checks = automatic_updates.is_active();
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
            next.home_filter_order = home_filter_order_for_save.borrow().clone();
            let selected_home_filters = home_filter_checks_for_save
                .borrow()
                .iter()
                .filter_map(|(id, check)| check.is_active().then_some(id.clone()))
                .collect::<std::collections::HashSet<_>>();
            next.home_filters = next
                .home_filter_order
                .iter()
                .filter(|id| selected_home_filters.contains(*id))
                .cloned()
                .collect();
            settings::normalize_home_filters(&mut next);
            let filters_changed = next.home_filters != current.home_filters
                || next.home_filter_order != current.home_filter_order;
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
            if appearance_changed || filters_changed {
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

enum UpdateUiMessage {
    Checking,
    Checked(Result<update::UpdateSnapshot, String>),
    DownloadProgress(u64, Option<u64>),
    Downloaded(Result<update::DownloadedUpdate, String>),
}

fn render_update_snapshot(
    snapshot: &update::UpdateSnapshot,
    current: &Label,
    latest: &Label,
    status: &Label,
    release: &Label,
    package: &Label,
    notes: &Label,
    notes_button: &Button,
    install: &Button,
) {
    current.set_text(&format!("Current version: v{}", snapshot.current_version));
    latest.set_text(&format!(
        "Latest version: {}",
        snapshot
            .latest_version
            .as_deref()
            .map(|version| format!("v{version}"))
            .unwrap_or_else(|| "Not checked".to_string())
    ));
    status.set_text(if snapshot.message.is_empty() {
        "No update check has been completed yet."
    } else {
        &snapshot.message
    });
    release.set_text(&format!(
        "Release date: {}\nRelease: {}",
        snapshot.release_date.as_deref().unwrap_or("Unknown"),
        snapshot.release_name.as_deref().unwrap_or("Unknown")
    ));
    package.set_text(&format!(
        "Package: {}\nAsset: {}\nSHA-256: {}",
        snapshot
            .target
            .as_ref()
            .map(|target| target.installation.label())
            .unwrap_or("Unknown"),
        snapshot.asset_name.as_deref().unwrap_or("None selected"),
        snapshot.sha256.as_deref().unwrap_or("Not provided")
    ));
    notes.set_text(
        snapshot
            .release_notes
            .as_deref()
            .unwrap_or("Release notes will appear after checking for updates."),
    );
    notes_button.set_sensitive(snapshot.release_url.is_some());
    install.set_sensitive(snapshot.installable);
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
    agent_stack: &Stack,
    agent_ui: &AgentUi,
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
                if id == "open-agent" || id == "open-hermes" {
                    agent_new_conversation(agent_ui, args.first().cloned());
                    agent_stack.set_visible_child_name("agent");
                    agent_ui.prompt.grab_focus();
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

#[derive(Clone)]
enum PreviewSource {
    File(PathBuf),
    StoredClipboard { paths: XdgPaths, id: u64 },
}

struct LoadedPreview {
    pixels: Vec<u8>,
    width: i32,
    height: i32,
    stride: usize,
}

#[derive(Clone)]
struct PreviewHandle {
    window: ApplicationWindow,
    image: Image,
    title: Label,
    status: Label,
}

type PreviewState = Rc<RefCell<Option<PreviewHandle>>>;

#[derive(Clone)]
struct SidePreviewHandle {
    panel: GtkBox,
    image: Image,
    title: Label,
    status: Label,
}

type SidePreviewState = Rc<RefCell<Option<SidePreviewHandle>>>;

fn build_side_preview() -> SidePreviewHandle {
    let panel = GtkBox::new(Orientation::Vertical, 10);
    panel.add_css_class("quick-side-preview");
    panel.set_width_request(320);
    panel.set_hexpand(false);
    panel.set_vexpand(true);
    let title = Label::new(Some("Image preview"));
    title.set_halign(Align::Start);
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title.add_css_class("quick-side-preview-title");
    panel.append(&title);
    let surface = GtkBox::new(Orientation::Vertical, 8);
    surface.add_css_class("quick-side-preview-surface");
    surface.set_hexpand(true);
    surface.set_vexpand(true);
    let image = Image::from_icon_name("image-x-generic-symbolic");
    image.set_halign(Align::Center);
    image.set_valign(Align::Center);
    image.set_hexpand(true);
    image.set_vexpand(true);
    surface.append(&image);
    let status = Label::new(Some("Hold Alt to preview"));
    status.set_halign(Align::Center);
    status.add_css_class("preview-loading");
    surface.append(&status);
    panel.append(&surface);
    panel.set_visible(false);
    SidePreviewHandle {
        panel,
        image,
        title,
        status,
    }
}

fn preview_source_for_item(paths: &XdgPaths, item: &Item) -> Option<PreviewSource> {
    match &item.target {
        Target::Path(path) if crate::search::is_image_path(path) => {
            Some(PreviewSource::File(path.clone()))
        }
        Target::Clipboard(id) if item.kind == "IMAGE" => Some(PreviewSource::StoredClipboard {
            paths: paths.clone(),
            id: *id,
        }),
        _ => None,
    }
}

fn preview_bounds(parent: &ApplicationWindow, quick: bool) -> (u32, u32) {
    let fallback = if quick {
        (360_u32, 260_u32)
    } else {
        (900_u32, 680_u32)
    };
    let Some(surface) = parent.surface() else {
        return fallback;
    };
    let Some(monitor) = gtk4::prelude::WidgetExt::display(parent).monitor_at_surface(&surface)
    else {
        return fallback;
    };
    let geometry = monitor.geometry();
    if quick {
        (360, 260)
    } else {
        (
            ((geometry.width() as f64) * 0.72)
                .round()
                .clamp(520.0, 1200.0) as u32,
            ((geometry.height() as f64) * 0.72)
                .round()
                .clamp(400.0, 900.0) as u32,
        )
    }
}

fn side_preview_width(parent: &ApplicationWindow, base_width: u32) -> u32 {
    let fallback = 320_u32;
    let Some(surface) = parent.surface() else {
        return fallback;
    };
    let Some(monitor) = gtk4::prelude::WidgetExt::display(parent).monitor_at_surface(&surface)
    else {
        return fallback;
    };
    let available = monitor.geometry().width().max(1) as u32;
    available
        .saturating_sub(base_width)
        .saturating_sub(24)
        .clamp(220, 320)
}

fn load_preview_bytes(
    source: PreviewSource,
    max_width: u32,
    max_height: u32,
) -> anyhow::Result<LoadedPreview> {
    let pixbuf = match source {
        PreviewSource::File(path) => {
            gdk_pixbuf::Pixbuf::from_file_at_scale(path, max_width as i32, max_height as i32, true)?
        }
        PreviewSource::StoredClipboard { paths, id } => {
            let bytes = crate::clipboard::image_bytes(&paths, id)?;
            let pixbuf = gdk_pixbuf::Pixbuf::from_read(Cursor::new(bytes))?;
            let (width, height) =
                fit_preview_dimensions(pixbuf.width(), pixbuf.height(), max_width, max_height);
            if width != pixbuf.width() || height != pixbuf.height() {
                pixbuf
                    .scale_simple(width, height, gdk_pixbuf::InterpType::Bilinear)
                    .ok_or_else(|| anyhow::anyhow!("could not scale clipboard image"))?
            } else {
                pixbuf
            }
        }
    };
    let width = pixbuf.width();
    let height = pixbuf.height();
    let stride = width as usize * 4;
    let pixels = if pixbuf.has_alpha() && pixbuf.n_channels() == 4 {
        let rowstride = pixbuf.rowstride() as usize;
        let source = unsafe { pixbuf.pixels() };
        let mut pixels = vec![0_u8; stride * height as usize];
        for row in 0..height as usize {
            let source_row = &source[row * rowstride..row * rowstride + stride];
            pixels[row * stride..(row + 1) * stride].copy_from_slice(source_row);
        }
        pixels
    } else {
        let rowstride = pixbuf.rowstride() as usize;
        let channels = pixbuf.n_channels() as usize;
        let source = unsafe { pixbuf.pixels() };
        let mut pixels = vec![0_u8; stride * height as usize];
        for row in 0..height as usize {
            for column in 0..width as usize {
                let source_offset = row * rowstride + column * channels;
                let target_offset = row * stride + column * 4;
                pixels[target_offset..target_offset + 3]
                    .copy_from_slice(&source[source_offset..source_offset + 3]);
                pixels[target_offset + 3] = 255;
            }
        }
        pixels
    };
    Ok(LoadedPreview {
        pixels,
        width,
        height,
        stride,
    })
}

fn fit_preview_dimensions(width: i32, height: i32, max_width: u32, max_height: u32) -> (i32, i32) {
    if width <= 0 || height <= 0 {
        return (1, 1);
    }
    let scale = (max_width as f64 / width as f64)
        .min(max_height as f64 / height as f64)
        .min(1.0);
    (
        (width as f64 * scale).round().max(1.0) as i32,
        (height as f64 * scale).round().max(1.0) as i32,
    )
}

fn set_preview_theme(previews: &PreviewState, theme_mode: &str) {
    if let Some(preview) = previews.borrow().as_ref() {
        preview.window.remove_css_class("dark");
        preview.window.remove_css_class("light");
        preview.window.remove_css_class("system");
        preview.window.add_css_class(theme_class(theme_mode));
    }
}

fn close_image_preview(previews: &PreviewState, load_generation: &Rc<Cell<u64>>) {
    load_generation.set(load_generation.get().saturating_add(1));
    let preview = previews.borrow_mut().take();
    if let Some(preview) = preview {
        preview.window.close();
    }
}

fn close_side_preview(side: &SidePreviewState, load_generation: &Rc<Cell<u64>>) {
    load_generation.set(load_generation.get().saturating_add(1));
    if let Some(preview) = side.borrow().as_ref() {
        preview.panel.set_visible(false);
    }
}

fn open_side_preview(
    paths: &XdgPaths,
    side: &SidePreviewState,
    load_generation: &Rc<Cell<u64>>,
    item: &Item,
) -> bool {
    let Some(source) = preview_source_for_item(paths, item) else {
        return false;
    };
    let Some(preview) = side.borrow().as_ref().cloned() else {
        return false;
    };
    let request_id = load_generation.get().saturating_add(1);
    load_generation.set(request_id);
    preview.panel.set_visible(true);
    preview.title.set_text(&item.title);
    preview.status.set_text("Loading preview…");
    preview.status.remove_css_class("preview-error");
    preview.status.add_css_class("preview-loading");
    preview
        .image
        .set_icon_name(Some("image-x-generic-symbolic"));

    let (sender, receiver) = async_channel::bounded::<Result<LoadedPreview, String>>(1);
    thread::spawn(move || {
        let result = load_preview_bytes(source, 280, 440).map_err(|error| error.to_string());
        let _ = sender.send_blocking(result);
    });
    let weak_side = Rc::downgrade(side);
    let generation_for_result = load_generation.clone();
    glib::MainContext::default().spawn_local(async move {
        let Ok(result) = receiver.recv().await else {
            return;
        };
        if generation_for_result.get() != request_id {
            return;
        }
        let Some(side) = weak_side.upgrade() else {
            return;
        };
        let Some(preview) = side.borrow().as_ref().cloned() else {
            return;
        };
        match result {
            Ok(loaded) => {
                let bytes = glib::Bytes::from(&loaded.pixels);
                let texture = gdk::MemoryTexture::new(
                    loaded.width,
                    loaded.height,
                    gdk::MemoryFormat::R8g8b8a8,
                    &bytes,
                    loaded.stride,
                );
                preview.image.set_paintable(Some(&texture));
                preview
                    .status
                    .set_text(&format!("{} × {}", loaded.width, loaded.height));
                preview.status.remove_css_class("preview-loading");
            }
            Err(_) => {
                preview.status.set_text("Preview unavailable");
                preview.status.remove_css_class("preview-loading");
                preview.status.add_css_class("preview-error");
            }
        }
    });
    true
}

fn open_quick_preview(
    parent: &ApplicationWindow,
    paths: &XdgPaths,
    floating: &PreviewState,
    side: &SidePreviewState,
    load_generation: &Rc<Cell<u64>>,
    alt_active: &Rc<Cell<bool>>,
    item: &Item,
    mode: &str,
) -> bool {
    if mode == "side" {
        open_side_preview(paths, side, load_generation, item)
    } else {
        open_image_preview(
            parent,
            paths,
            floating,
            load_generation,
            item,
            true,
            Some(alt_active.clone()),
        )
    }
}

fn open_image_preview(
    parent: &ApplicationWindow,
    paths: &XdgPaths,
    previews: &PreviewState,
    load_generation: &Rc<Cell<u64>>,
    item: &Item,
    quick: bool,
    alt_active: Option<Rc<Cell<bool>>>,
) -> bool {
    let Some(source) = preview_source_for_item(paths, item) else {
        return false;
    };

    let existing_preview = previews.borrow().as_ref().cloned();
    let preview = if let Some(preview) = existing_preview {
        preview
    } else {
        let Some(application) = parent.application() else {
            return false;
        };
        let window = ApplicationWindow::builder()
            .application(&application)
            .title("ProtonSearch Image Preview")
            .default_width(if quick { 380 } else { 720 })
            .default_height(if quick { 300 } else { 540 })
            .build();
        window.set_decorated(false);
        window.set_resizable(true);
        window.set_modal(false);
        window.set_hide_on_close(true);
        window.set_transient_for(Some(parent));
        window.add_css_class("proton-image-preview");
        window.add_css_class(if parent.has_css_class("light") {
            "light"
        } else {
            "dark"
        });

        let shell = GtkBox::new(Orientation::Vertical, 10);
        shell.add_css_class("preview-shell");
        shell.set_hexpand(true);
        shell.set_vexpand(true);

        let toolbar = GtkBox::new(Orientation::Horizontal, 8);
        toolbar.add_css_class("preview-toolbar");
        let title = Label::new(Some("Image preview"));
        title.set_halign(Align::Start);
        title.set_hexpand(true);
        title.add_css_class("preview-window-title");
        toolbar.append(&title);

        let close_button = Button::from_icon_name("window-close-symbolic");
        close_button.set_tooltip_text(Some("Close preview (Escape)"));
        close_button.add_css_class("preview-close");
        toolbar.append(&close_button);
        shell.append(&toolbar);

        let surface = GtkBox::new(Orientation::Vertical, 8);
        surface.add_css_class("preview-surface");
        surface.set_hexpand(true);
        surface.set_vexpand(true);
        surface.set_halign(Align::Fill);
        surface.set_valign(Align::Fill);

        let image = Image::from_icon_name("image-x-generic-symbolic");
        image.set_halign(Align::Center);
        image.set_valign(Align::Center);
        image.set_hexpand(true);
        image.set_vexpand(true);
        surface.append(&image);

        let status = Label::new(Some("Loading image…"));
        status.set_halign(Align::Center);
        status.add_css_class("preview-loading");
        surface.append(&status);
        shell.append(&surface);
        window.set_child(Some(&shell));

        let preview_state = previews.clone();
        let generation_for_close = load_generation.clone();
        let weak_preview_state = Rc::downgrade(&preview_state);
        window.connect_close_request(move |window| {
            generation_for_close.set(generation_for_close.get().saturating_add(1));
            if let Some(previews) = weak_preview_state.upgrade() {
                previews.borrow_mut().take();
            }
            window.hide();
            glib::Propagation::Stop
        });

        let window_for_close = window.clone();
        close_button.connect_clicked(move |_| window_for_close.close());

        let window_for_keys = window.clone();
        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                window_for_keys.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        if quick {
            let previews_for_alt_release = previews.clone();
            let generation_for_alt_release = load_generation.clone();
            let alt_active_for_release = alt_active.clone();
            key_controller.connect_key_released(move |_, key, _, _| {
                if matches!(key, gdk::Key::Alt_L | gdk::Key::Alt_R) {
                    if let Some(alt_active) = alt_active_for_release.as_ref() {
                        alt_active.set(false);
                    }
                    close_image_preview(&previews_for_alt_release, &generation_for_alt_release);
                }
            });
        }
        window.add_controller(key_controller);

        let preview = PreviewHandle {
            window,
            image,
            title,
            status,
        };
        *previews.borrow_mut() = Some(preview.clone());
        preview
    };

    let request_id = load_generation.get().saturating_add(1);
    load_generation.set(request_id);
    preview.title.set_text(&item.title);
    preview.status.set_text("Loading image…");
    preview.status.remove_css_class("preview-error");
    preview.status.add_css_class("preview-loading");
    preview
        .image
        .set_icon_name(Some("image-x-generic-symbolic"));
    preview.window.present();

    let (sender, receiver) = async_channel::bounded::<Result<LoadedPreview, String>>(1);
    let (max_width, max_height) = preview_bounds(parent, quick);
    thread::spawn(move || {
        let result =
            load_preview_bytes(source, max_width, max_height).map_err(|error| error.to_string());
        let _ = sender.send_blocking(result);
    });

    let weak_previews = Rc::downgrade(previews);
    let load_generation_for_result = load_generation.clone();
    glib::MainContext::default().spawn_local(async move {
        let Ok(result) = receiver.recv().await else {
            return;
        };
        if load_generation_for_result.get() != request_id {
            return;
        }
        let Some(previews) = weak_previews.upgrade() else {
            return;
        };
        let Some(preview) = previews.borrow().as_ref().cloned() else {
            return;
        };
        match result {
            Ok(loaded) => {
                let bytes = glib::Bytes::from(&loaded.pixels);
                let texture = gdk::MemoryTexture::new(
                    loaded.width,
                    loaded.height,
                    gdk::MemoryFormat::R8g8b8a8,
                    &bytes,
                    loaded.stride,
                );
                preview.image.set_paintable(Some(&texture));
                preview.image.set_tooltip_text(Some("Image preview"));
                preview.status.set_text(&format!(
                    "{} × {} · Escape closes",
                    loaded.width, loaded.height
                ));
                preview.status.remove_css_class("preview-error");
                preview.status.add_css_class("preview-loading");
                if quick {
                    preview.window.set_default_size(
                        (loaded.width + 28).clamp(360, 480),
                        (loaded.height + 86).clamp(260, 360),
                    );
                } else {
                    preview.window.set_default_size(
                        (loaded.width + 28).max(520),
                        (loaded.height + 86).max(400),
                    );
                }
            }
            Err(error) => {
                preview
                    .status
                    .set_text(&format!("Could not load image: {error}"));
                preview.status.remove_css_class("preview-loading");
                preview.status.add_css_class("preview-error");
            }
        }
    });
    true
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

    let search_header = GtkBox::new(Orientation::Horizontal, 10);
    search_header.add_css_class("search-header");
    search_header.set_hexpand(true);

    let brand_slot = GtkBox::new(Orientation::Horizontal, 0);
    brand_slot.add_css_class("brand-slot");
    brand_slot.set_halign(Align::Center);
    brand_slot.set_valign(Align::Center);
    let brand_logo = crate::icons::protonsearch(38);
    brand_logo.add_css_class("brand-logo");
    brand_slot.append(&brand_logo);
    search_header.append(&brand_slot);

    let search_shell = GtkBox::new(Orientation::Horizontal, 6);
    search_shell.add_css_class("search-shell");
    search_shell.set_hexpand(true);
    search_shell.set_height_request(linux_settings.search_bar_height.clamp(42, 100) as i32);

    let entry = Entry::builder().hexpand(true).build();
    if linux_settings.show_placeholder {
        entry.set_placeholder_text(Some("Search files, code, PDFs..."));
    }
    entry.add_css_class("search-entry");
    entry.set_tooltip_text(Some(
        "Type to search; Enter opens the selected result; Escape closes",
    ));
    search_shell.append(&entry);

    let shortcut_badge = Label::new(Some("Ctrl+K"));
    shortcut_badge.add_css_class("shortcut-badge");
    shortcut_badge.set_halign(Align::Center);
    shortcut_badge.set_valign(Align::Center);
    shortcut_badge.set_tooltip_text(Some("Focus the search field"));
    search_shell.append(&shortcut_badge);
    search_header.append(&search_shell);
    root.append(&search_header);

    let category_row = GtkBox::new(Orientation::Horizontal, 2);
    category_row.add_css_class("category-row");
    category_row.set_halign(Align::Start);
    category_row.set_vexpand(false);
    let category_scroller = ScrolledWindow::builder()
        .child(&category_row)
        .hexpand(true)
        .vexpand(false)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Never)
        .min_content_height(34)
        .build();
    category_scroller.set_hexpand(true);
    category_scroller.set_vexpand(false);
    let category_bar = GtkBox::new(Orientation::Horizontal, 0);
    category_bar.set_hexpand(true);
    category_bar.set_vexpand(false);
    category_bar.append(&category_scroller);
    let active_category = Rc::new(RefCell::new(None::<Button>));
    let category_buttons = Rc::new(RefCell::new(Vec::<(String, Button)>::new()));
    for filter_id in &linux_settings.home_filters {
        let Some((label, prefix, icon_name)) = home_filter_spec(filter_id) else {
            continue;
        };
        let chip = Button::new();
        chip.set_has_frame(false);
        chip.add_css_class("category-chip");
        let chip_content = GtkBox::new(Orientation::Horizontal, 5);
        chip_content.set_halign(Align::Center);
        chip_content.append(&category_icon(icon_name));
        let chip_label = Label::new(Some(label));
        chip_label.set_single_line_mode(true);
        chip_content.append(&chip_label);
        chip.set_child(Some(&chip_content));
        if *filter_id == "all" {
            chip.add_css_class("active");
            *active_category.borrow_mut() = Some(chip.clone());
        }
        category_row.append(&chip);
        category_buttons
            .borrow_mut()
            .push((prefix.to_string(), chip.clone()));
        let entry_for_chip = entry.clone();
        let active_category_for_chip = active_category.clone();
        let chip_for_callback = chip.clone();
        let scroller_for_chip = category_scroller.clone();
        chip.connect_clicked(move |_| {
            if let Some(previous) = active_category_for_chip
                .borrow_mut()
                .replace(chip_for_callback.clone())
            {
                previous.remove_css_class("active");
            }
            chip_for_callback.add_css_class("active");
            entry_for_chip.set_text(prefix);
            entry_for_chip.grab_focus();
            ensure_category_visible(&scroller_for_chip, &chip_for_callback);
        });
        let focus_controller = EventControllerFocus::new();
        let scroller_for_focus = category_scroller.clone();
        let chip_for_focus = chip.clone();
        focus_controller.connect_enter(move |_| {
            ensure_category_visible(&scroller_for_focus, &chip_for_focus);
        });
        chip.add_controller(focus_controller);
    }
    let filter_scroll_controller = EventControllerScroll::new(
        EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::HORIZONTAL,
    );
    filter_scroll_controller.set_propagation_phase(PropagationPhase::Capture);
    let category_scroller_for_scroll = category_scroller.clone();
    filter_scroll_controller.connect_scroll(move |_, dx, dy| {
        let delta = if dx.abs() > 0.01 { dx } else { dy };
        if delta.abs() <= 0.01 {
            return glib::Propagation::Proceed;
        }
        let adjustment = category_scroller_for_scroll.hadjustment();
        let lower = adjustment.lower();
        let upper = (adjustment.upper() - adjustment.page_size()).max(lower);
        let next = (adjustment.value() + delta * 72.0).clamp(lower, upper);
        if (next - adjustment.value()).abs() > 0.01 {
            adjustment.set_value(next);
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    category_scroller.add_controller(filter_scroll_controller);
    root.append(&category_bar);

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
    let status = Label::new(Some("Quick Search"));
    status.set_halign(Align::Start);
    status.set_hexpand(true);
    status.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    status.set_max_width_chars(34);
    status.add_css_class("status-label");
    let footer_bar = GtkBox::new(Orientation::Horizontal, 8);
    footer_bar.set_hexpand(true);
    footer_bar.append(&status);
    footer_bar.append(&footer);
    root.append(&footer_bar);
    let (agent_root, agent_ui) = build_agent_view(&paths);
    connect_agent_actions(&agent_ui);
    let agent_stack = Stack::new();
    agent_stack.set_hexpand(true);
    agent_stack.set_vexpand(true);
    agent_stack.set_transition_type(StackTransitionType::SlideLeftRight);
    agent_stack.set_transition_duration(160);
    agent_stack.add_named(&root, Some("launcher"));
    agent_stack.add_named(&agent_root, Some("agent"));
    agent_stack.set_visible_child_name("launcher");
    let agent_stack_for_back = agent_stack.clone();
    let entry_for_agent_back = entry.clone();
    agent_ui.back.connect_clicked(move |_| {
        agent_stack_for_back.set_visible_child_name("launcher");
        entry_for_agent_back.grab_focus();
    });
    root.set_width_request(linux_settings.window_width.clamp(480, 1600) as i32);
    let side_handle = build_side_preview();
    let side_panel = side_handle.panel.clone();
    let side_previews: SidePreviewState = Rc::new(RefCell::new(Some(side_handle)));
    let launcher_shell = GtkBox::new(Orientation::Horizontal, 0);
    launcher_shell.set_hexpand(true);
    launcher_shell.set_vexpand(true);
    launcher_shell.append(&agent_stack);
    launcher_shell.append(&side_panel);
    window.set_child(Some(&launcher_shell));

    let items = Rc::new(RefCell::new(Vec::<Item>::new()));
    let animation = Rc::new(RefCell::new(None::<glib::SourceId>));
    let generation = Rc::new(Cell::new(0_u64));
    let previews: PreviewState = Rc::new(RefCell::new(None));
    let preview_load_generation = Rc::new(Cell::new(0_u64));
    let alt_preview_active = Rc::new(Cell::new(false));
    let side_preview_active = Rc::new(Cell::new(false));
    let base_width = Rc::new(Cell::new(linux_settings.window_width.clamp(480, 1600)));
    let base_height = Rc::new(Cell::new(linux_settings.window_height.clamp(420, 1200)));
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
    let cursor_index = Rc::new(Cell::new(0_i32));
    let initial_items = providers::collect(&paths, &linux_settings, "");
    *items.borrow_mut() = initial_items.clone();
    update_results(
        &list,
        &status,
        &initial_items,
        "",
        row_height.get(),
        theme_class(&linux_settings.theme_mode) == "light",
        &paths,
    );

    let generation_for_changed = generation.clone();
    let request_sender_for_changed = request_sender.clone();
    let settings_for_changed = settings_state.clone();
    let active_category_for_changed = active_category.clone();
    let category_buttons_for_changed = category_buttons.clone();
    let debounce_source = Rc::new(RefCell::new(None::<glib::SourceId>));
    entry.connect_changed(move |entry| {
        let next_generation = generation_for_changed.get().saturating_add(1);
        generation_for_changed.set(next_generation);
        let query = entry.text().to_string();
        sync_active_category(
            &active_category_for_changed,
            &category_buttons_for_changed.borrow(),
            &category_scroller,
            &query,
        );
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
    let paths_for_receiver = paths.clone();
    let cursor_index_for_receiver = cursor_index.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Ok((result_generation, query, results)) = receiver.recv().await {
            if result_generation == generation_for_receiver.get() {
                *items_for_receiver.borrow_mut() = results.clone();
                cursor_index_for_receiver.set(0);
                update_results(
                    &list_for_receiver,
                    &status_for_receiver,
                    &results,
                    &query,
                    row_height_for_receiver.get(),
                    theme_class(&settings_for_receiver.borrow().theme_mode) == "light",
                    &paths_for_receiver,
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
    let previews_for_commands = previews.clone();
    let root_for_commands = root.clone();
    let base_width_for_commands = base_width.clone();
    let base_height_for_commands = base_height.clone();
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
                    set_preview_theme(&previews_for_commands, &next_settings.theme_mode);
                    window_for_commands.set_default_size(
                        next_settings.window_width.clamp(480, 1600) as i32,
                        next_settings.window_height.clamp(420, 1200) as i32,
                    );
                    root_for_commands
                        .set_width_request(next_settings.window_width.clamp(480, 1600) as i32);
                    base_width_for_commands.set(next_settings.window_width.clamp(480, 1600));
                    base_height_for_commands.set(next_settings.window_height.clamp(420, 1200));
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
    let agent_stack_for_enter = agent_stack.clone();
    let agent_ui_for_enter = agent_ui.clone();
    let previews_for_enter = previews.clone();
    let preview_generation_for_enter = preview_load_generation.clone();
    let cursor_index_for_enter = cursor_index.clone();
    entry.connect_activate(move |_| {
        let selected_items = list_for_enter
            .selected_rows()
            .into_iter()
            .filter_map(|row| items_for_enter.borrow().get(row.index() as usize).cloned())
            .collect::<Vec<_>>();
        if selected_items.len() > 1
            && selected_items.iter().all(|item| item.source == "Clipboard")
        {
            match providers::activate_clipboard_batch(&paths_for_enter, &selected_items) {
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
            .or_else(|| list_for_enter.row_at_index(cursor_index_for_enter.get()))
            .or_else(|| list_for_enter.row_at_index(0))
        {
            cursor_index_for_enter.set(row.index());
            set_cursor_row(&list_for_enter, row.index());
            let index = row.index();
            let Some(item) = items_for_enter.borrow().get(index as usize).cloned() else {
                return;
            };
            if item.kind == "IMAGE"
                && open_image_preview(
                    &window_for_enter,
                    &paths_for_enter,
                    &previews_for_enter,
                    &preview_generation_for_enter,
                    &item,
                    false,
                    None,
                )
            {
                return;
            }
            activate_item(
                &paths_for_enter,
                &window_for_enter,
                &entry_for_enter,
                &status_for_enter,
                &animation_for_enter,
                &action_sender_for_enter,
                &agent_stack_for_enter,
                &agent_ui_for_enter,
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
    let agent_stack_for_activation = agent_stack.clone();
    let agent_ui_for_activation = agent_ui.clone();
    let previews_for_activation = previews.clone();
    let preview_generation_for_activation = preview_load_generation.clone();
    let cursor_index_for_activation = cursor_index.clone();
    list.connect_row_activated(move |_, row| {
        cursor_index_for_activation.set(row.index());
        set_cursor_row(&list_for_activation, row.index());
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
            match providers::activate_clipboard_batch(&paths_for_activation, &selected_items) {
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
            if open_image_preview(
                &window_for_activation,
                &paths_for_activation,
                &previews_for_activation,
                &preview_generation_for_activation,
                &item,
                false,
                None,
            ) {
                return;
            }
        }
        activate_item(
            &paths_for_activation,
            &window_for_activation,
            &entry_for_activation,
            &status_for_activation,
            &animation_for_activation,
            &action_sender_for_activation,
            &agent_stack_for_activation,
            &agent_ui_for_activation,
            item,
        );
    });

    let key_controller = EventControllerKey::new();
    let entry_for_shortcut = entry.clone();
    let window_for_escape = window.clone();
    let animation_for_escape = animation.clone();
    let agent_stack_for_escape = agent_stack.clone();
    let entry_for_agent_escape = entry.clone();
    let list_for_navigation = list.clone();
    let items_for_preview = items.clone();
    let paths_for_preview = paths.clone();
    let previews_for_key = previews.clone();
    let side_previews_for_key = side_previews.clone();
    let preview_generation_for_key = preview_load_generation.clone();
    let alt_preview_active_for_key = alt_preview_active.clone();
    let side_preview_active_for_key = side_preview_active.clone();
    let base_width_for_key = base_width.clone();
    let base_height_for_key = base_height.clone();
    let root_for_key = root.clone();
    let cursor_index_for_key = cursor_index.clone();
    let settings_state_for_key = settings_state.clone();
    key_controller.connect_key_pressed(move |_, key, _, state| {
        let agent_visible = agent_stack_for_escape.visible_child_name().as_deref() == Some("agent");
        if key == gdk::Key::k && state.contains(gdk::ModifierType::CONTROL_MASK) {
            if agent_visible {
                return glib::Propagation::Proceed;
            }
            entry_for_shortcut.grab_focus();
            entry_for_shortcut.select_region(0, -1);
            return glib::Propagation::Stop;
        }
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
            let mode = settings_state_for_key.borrow().image_preview_mode.clone();
            let is_previewing = selected.as_ref().is_some_and(|item| {
                item.kind == "IMAGE"
                    && open_quick_preview(
                        &window_for_escape,
                        &paths_for_preview,
                        &previews_for_key,
                        &side_previews_for_key,
                        &preview_generation_for_key,
                        &alt_preview_active_for_key,
                        item,
                        &mode,
                    )
            });
            if mode == "side" && is_previewing {
                side_preview_active_for_key.set(true);
                let side_width = side_preview_width(&window_for_escape, base_width_for_key.get());
                if let Some(side) = side_previews_for_key.borrow().as_ref() {
                    side.panel.set_width_request(side_width as i32);
                }
                root_for_key.set_width_request(base_width_for_key.get() as i32);
                window_for_escape.set_default_size(
                    (base_width_for_key.get() + side_width).min(1920) as i32,
                    base_height_for_key.get() as i32,
                );
            }
            alt_preview_active_for_key.set(is_previewing);
            return glib::Propagation::Proceed;
        }
        if key == gdk::Key::Escape {
            if agent_stack_for_escape.visible_child_name().as_deref() == Some("agent") {
                agent_stack_for_escape.set_visible_child_name("launcher");
                entry_for_agent_escape.grab_focus();
            } else {
                animate_hide(&window_for_escape, &animation_for_escape);
            }
            return glib::Propagation::Stop;
        }
        if key == gdk::Key::space && state.contains(gdk::ModifierType::CONTROL_MASK) {
            if agent_visible {
                return glib::Propagation::Proceed;
            }
            if let Some(row) = list_for_navigation.row_at_index(cursor_index_for_key.get()) {
                let selected = list_for_navigation
                    .selected_rows()
                    .iter()
                    .any(|selected| selected.index() == row.index());
                if selected {
                    list_for_navigation.unselect_row(&row);
                } else if !selected {
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
            if agent_visible {
                return glib::Propagation::Proceed;
            }
            let count = list_for_navigation.observe_children().n_items() as i32;
            if count == 0 {
                return glib::Propagation::Stop;
            }
            let current = cursor_index_for_key.get().clamp(0, count - 1);
            let page_step = {
                let current_settings = settings_state_for_key.borrow();
                (current_settings.window_height.saturating_sub(150)
                    / current_settings.item_height.max(1))
                .max(1) as i32
            };
            let next = match key {
                gdk::Key::Down => (current + 1).min(count - 1),
                gdk::Key::Up => current.saturating_sub(1),
                gdk::Key::Page_Down => (current + page_step).min(count - 1),
                gdk::Key::Page_Up => current.saturating_sub(page_step),
                gdk::Key::Home => 0,
                _ => count - 1,
            };
            if let Some(row) = list_for_navigation.row_at_index(next) {
                cursor_index_for_key.set(next);
                // Multiple selection is available for clipboard workflows,
                // but ordinary arrow navigation must behave like a single
                // active cursor. Holding Ctrl intentionally preserves the
                // existing selection set.
                if !state.contains(gdk::ModifierType::CONTROL_MASK) {
                    list_for_navigation.unselect_all();
                }
                list_for_navigation.select_row(Some(&row));
                set_cursor_row(&list_for_navigation, next);
                if alt_preview_active_for_key.get() {
                    if let Some(item) = items_for_preview.borrow().get(next as usize).cloned() {
                        let mode = settings_state_for_key.borrow().image_preview_mode.clone();
                        let previewing = item.kind == "IMAGE"
                            && open_quick_preview(
                                &window_for_escape,
                                &paths_for_preview,
                                &previews_for_key,
                                &side_previews_for_key,
                                &preview_generation_for_key,
                                &alt_preview_active_for_key,
                                &item,
                                &mode,
                            );
                        if mode == "side" && previewing {
                            side_preview_active_for_key.set(true);
                            let side_width =
                                side_preview_width(&window_for_escape, base_width_for_key.get());
                            if let Some(side) = side_previews_for_key.borrow().as_ref() {
                                side.panel.set_width_request(side_width as i32);
                            }
                            window_for_escape.set_default_size(
                                (base_width_for_key.get() + side_width).min(1920) as i32,
                                base_height_for_key.get() as i32,
                            );
                        } else if !previewing {
                            close_image_preview(&previews_for_key, &preview_generation_for_key);
                            close_side_preview(&side_previews_for_key, &preview_generation_for_key);
                            if side_preview_active_for_key.replace(false) {
                                root_for_key.set_width_request(base_width_for_key.get() as i32);
                                window_for_escape.set_default_size(
                                    base_width_for_key.get() as i32,
                                    base_height_for_key.get() as i32,
                                );
                            }
                            alt_preview_active_for_key.set(false);
                        }
                    }
                }
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    let previews_for_release = previews.clone();
    let side_previews_for_release = side_previews.clone();
    let preview_generation_for_release = preview_load_generation.clone();
    let alt_preview_active_for_release = alt_preview_active.clone();
    let side_preview_active_for_release = side_preview_active.clone();
    let window_for_release = window.clone();
    let base_width_for_release = base_width.clone();
    let base_height_for_release = base_height.clone();
    let root_for_release = root.clone();
    key_controller.connect_key_released(move |_, key, _, _| {
        if matches!(key, gdk::Key::Alt_L | gdk::Key::Alt_R)
            && alt_preview_active_for_release.replace(false)
        {
            close_image_preview(&previews_for_release, &preview_generation_for_release);
            close_side_preview(&side_previews_for_release, &preview_generation_for_release);
            if side_preview_active_for_release.replace(false) {
                root_for_release.set_width_request(base_width_for_release.get() as i32);
                window_for_release.set_default_size(
                    base_width_for_release.get() as i32,
                    base_height_for_release.get() as i32,
                );
            }
        }
    });
    key_controller.set_propagation_phase(PropagationPhase::Capture);
    window.add_controller(key_controller);
    let previews_for_focus_loss = previews.clone();
    let side_previews_for_focus_loss = side_previews.clone();
    let generation_for_focus_loss = preview_load_generation.clone();
    let alt_for_focus_loss = alt_preview_active.clone();
    let side_active_for_focus_loss = side_preview_active.clone();
    let root_for_focus_loss = root.clone();
    let base_width_for_focus_loss = base_width.clone();
    let base_height_for_focus_loss = base_height.clone();
    window.connect_is_active_notify(move |window| {
        if window.is_active() {
            return;
        }
        // The floating quick preview is an owned ProtonSearch window. GTK
        // can mark the launcher inactive while that preview is presented;
        // keep the preview alive until Alt is released instead of treating
        // this internal focus transfer as an outside click.
        if alt_for_focus_loss.get() && previews_for_focus_loss.borrow().is_some() {
            return;
        }
        alt_for_focus_loss.set(false);
        close_image_preview(&previews_for_focus_loss, &generation_for_focus_loss);
        close_side_preview(&side_previews_for_focus_loss, &generation_for_focus_loss);
        if side_active_for_focus_loss.replace(false) {
            root_for_focus_loss.set_width_request(base_width_for_focus_loss.get() as i32);
            window.set_default_size(
                base_width_for_focus_loss.get() as i32,
                base_height_for_focus_loss.get() as i32,
            );
        }
    });
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

fn update_results(
    list: &ListBox,
    status: &Label,
    items: &[Item],
    query: &str,
    row_height: u32,
    light_theme: bool,
    paths: &XdgPaths,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for item in items {
        list.append(&result_row(item, row_height, light_theme, paths));
    }
    if items.is_empty() {
        let message = empty_state_message(query);
        list.append(&empty_state_row(&message));
        status.set_text(&message);
    } else {
        status.set_text("Quick Search");
    }
    list.unselect_all();
    if let Some(row) = list.row_at_index(0) {
        list.select_row(Some(&row));
        set_cursor_row(list, 0);
    } else {
        list.select_row(None::<&ListBoxRow>);
    }
}

fn set_cursor_row(list: &ListBox, index: i32) {
    let mut child = list.first_child();
    while let Some(widget) = child {
        let next = widget.next_sibling();
        if let Ok(row) = widget.downcast::<ListBoxRow>() {
            if row.index() == index {
                row.add_css_class("cursor-row");
            } else {
                row.remove_css_class("cursor-row");
            }
        }
        child = next;
    }
}

fn result_row(item: &Item, row_height: u32, light_theme: bool, paths: &XdgPaths) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_height_request(row_height as i32);
    row.set_hexpand(true);
    row.add_css_class("result-row");
    if item.kind.eq_ignore_ascii_case("SOURCE") {
        row.add_css_class("source-row");
    } else {
        row.add_css_class("compact-row");
    }
    let content = GtkBox::new(Orientation::Horizontal, 8);
    content.set_hexpand(true);
    content.set_margin_top(8);
    content.set_margin_bottom(8);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let icon = result_icon(item, light_theme, paths);
    let icon_slot = GtkBox::new(Orientation::Horizontal, 0);
    icon_slot.set_size_request(40, -1);
    icon_slot.set_halign(Align::Center);
    icon_slot.set_valign(Align::Center);
    icon_slot.append(&icon);
    content.append(&icon_slot);

    let text = GtkBox::new(Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.set_halign(Align::Fill);

    let title = Label::new(Some(&item.title));
    title.set_halign(Align::Start);
    title.set_xalign(0.0);
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    title.set_single_line_mode(true);
    title.add_css_class("result-title");
    text.append(&title);

    let subtitle = Label::new(Some(&item.subtitle));
    subtitle.set_halign(Align::Start);
    subtitle.set_xalign(0.0);
    subtitle.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    subtitle.set_single_line_mode(true);
    subtitle.add_css_class("result-subtitle");
    text.append(&subtitle);
    content.append(&text);

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

fn category_icon(name: &str) -> Image {
    let icon_name = match name {
        "all" => "view-grid-symbolic",
        "files" => "document-open-symbolic",
        "apps" => "application-x-executable-symbolic",
        "folders" => "folder-symbolic",
        "content" => "text-x-generic-symbolic",
        "images" => "image-x-generic-symbolic",
        "ocr" => "edit-find-symbolic",
        "source-code" => "text-x-script-symbolic",
        "settings" => "emblem-system-symbolic",
        "commands" => "system-run-symbolic",
        "clipboard" => "edit-paste-symbolic",
        "agents" => "avatar-default-symbolic",
        _ => "view-grid-symbolic",
    };
    let image = Image::from_icon_name(icon_name);
    image.set_pixel_size(16);
    image.add_css_class("category-icon");
    image
}

fn home_filter_spec(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "all" => ("All", "", "all"),
        "files" => ("Files", "file:", "files"),
        "apps" => ("Applications", "app:", "apps"),
        "folders" => ("Folders", "folder:", "folders"),
        "content" => ("Content", "content:", "content"),
        "images" => ("Images", "images:", "images"),
        "ocr" => ("OCR", "ocr:", "ocr"),
        "code" => ("Code", "code:", "source-code"),
        "settings" => ("Settings", "settings:", "settings"),
        "commands" => ("Commands", "commands:", "commands"),
        "clipboard" => ("Clipboard", "clip:", "clipboard"),
        "agents" => ("Agents", "agents:", "agents"),
        _ => return None,
    })
}

fn home_filter_label(id: &str) -> &'static str {
    home_filter_spec(id)
        .map(|(label, _, _)| label)
        .unwrap_or("Filter")
}

fn refresh_home_filter_order_label(label: &Label, order: &[String]) {
    let text = order
        .iter()
        .map(|id| home_filter_label(id))
        .collect::<Vec<_>>()
        .join("  ·  ");
    label.set_text(&format!("Order: {text}"));
}

fn move_home_filter(order: &Rc<RefCell<Vec<String>>>, id: &str, delta: i32, order_label: &Label) {
    if id == "all" {
        return;
    }
    let mut order = order.borrow_mut();
    let Some(index) = order.iter().position(|value| value == id) else {
        return;
    };
    let next = (index as i32 + delta).clamp(1, order.len().saturating_sub(1) as i32) as usize;
    if next != index {
        order.swap(index, next);
        refresh_home_filter_order_label(order_label, &order);
    }
}

fn sync_active_category(
    active_category: &Rc<RefCell<Option<Button>>>,
    categories: &[(String, Button)],
    scroller: &ScrolledWindow,
    query: &str,
) {
    let query = query.trim();
    let Some((_, next)) = categories
        .iter()
        .find(|(prefix, _)| {
            prefix.is_empty() && query.is_empty() || !prefix.is_empty() && query.starts_with(prefix)
        })
        .or_else(|| categories.iter().find(|(prefix, _)| prefix.is_empty()))
    else {
        return;
    };

    if let Some(previous) = active_category.borrow_mut().replace(next.clone()) {
        previous.remove_css_class("active");
    }
    next.add_css_class("active");
    ensure_category_visible(scroller, next);
}

fn ensure_category_visible(scroller: &ScrolledWindow, button: &Button) {
    let allocation = button.allocation();
    let adjustment = scroller.hadjustment();
    let current = adjustment.value();
    let viewport = adjustment.page_size().max(scroller.width() as f64);
    let left = allocation.x() as f64;
    let right = left + allocation.width() as f64;
    let margin = 8.0;
    let target = if left < current + margin {
        (left - margin).max(adjustment.lower())
    } else if right > current + viewport - margin {
        (right - viewport + margin)
            .min((adjustment.upper() - adjustment.page_size()).max(adjustment.lower()))
    } else {
        return;
    };
    adjustment.set_value(target);
}

fn empty_state_message(query: &str) -> String {
    let query = query.trim().to_ascii_lowercase();
    if query.starts_with("images:") || query.starts_with("image:") || query.starts_with("ocr:") {
        return "No image files found".to_string();
    }
    if query.starts_with("clipboard:") || query.starts_with("clip:") {
        return "Clipboard history is empty. Copy something to see it here.".to_string();
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

fn scaled_clipboard_bytes(bytes: &[u8], max_width: i32, max_height: i32) -> Option<Image> {
    let pixbuf = gdk_pixbuf::Pixbuf::from_read(Cursor::new(bytes.to_vec())).ok()?;
    let scale = (max_width as f64 / pixbuf.width() as f64)
        .min(max_height as f64 / pixbuf.height() as f64)
        .min(1.0);
    let width = (pixbuf.width() as f64 * scale).round().max(1.0) as i32;
    let height = (pixbuf.height() as f64 * scale).round().max(1.0) as i32;
    let pixbuf = pixbuf.scale_simple(width, height, gdk_pixbuf::InterpType::Bilinear)?;
    let texture = gdk::Texture::for_pixbuf(&pixbuf);
    Some(Image::from_paintable(Some(&texture)))
}

fn result_icon(item: &Item, light_theme: bool, paths: &XdgPaths) -> Image {
    if let Some(name) = result_asset_name(item) {
        let size = if item.kind.eq_ignore_ascii_case("SOURCE") {
            40
        } else {
            32
        };
        if let Some(image) = crate::icons::source_for_theme(name, size, light_theme) {
            return image;
        }
    }

    if item.kind == "IMAGE" {
        let image = match &item.target {
            Target::Clipboard(id) => crate::clipboard::image_bytes(paths, *id)
                .ok()
                .and_then(|bytes| scaled_clipboard_bytes(&bytes, 64, 64)),
            _ => None,
        };
        if let Some(image) = image {
            image.set_pixel_size(48);
            image.add_css_class("result-thumbnail");
            return image;
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
        Target::Clipboard(_) => Image::from_icon_name("edit-copy-symbolic"),
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

#[cfg(test)]
mod preview_tests {
    use super::*;

    #[test]
    fn fit_preview_preserves_landscape_aspect_ratio() {
        assert_eq!(fit_preview_dimensions(4000, 2000, 1200, 900), (1200, 600));
    }

    #[test]
    fn fit_preview_preserves_portrait_aspect_ratio() {
        assert_eq!(fit_preview_dimensions(1200, 2400, 900, 900), (450, 900));
    }

    #[test]
    fn fit_preview_does_not_upscale_small_images() {
        assert_eq!(fit_preview_dimensions(320, 180, 1200, 900), (320, 180));
    }

    #[test]
    fn fit_preview_handles_unusual_aspect_ratios() {
        assert_eq!(fit_preview_dimensions(10000, 100, 1200, 900), (1200, 12));
    }

    #[test]
    fn image_paths_with_spaces_and_unicode_are_file_sources() {
        let path = PathBuf::from("/tmp/Пример folder/image with spaces.webp");
        let item = Item {
            title: "image with spaces.webp".to_string(),
            subtitle: path.display().to_string(),
            source: "Local".to_string(),
            kind: "IMAGE".to_string(),
            target: Target::Path(path.clone()),
        };
        assert!(crate::search::is_image_path(&path));
        assert!(matches!(
            preview_source_for_item(&XdgPaths {
                home: PathBuf::from("/tmp"),
                config: PathBuf::from("/tmp"),
                data: PathBuf::from("/tmp"),
                state: PathBuf::from("/tmp"),
                cache: PathBuf::from("/tmp"),
                runtime: None,
            }, &item),
            Some(PreviewSource::File(candidate)) if candidate == path
        ));
    }
}
