//! Shared ProtonSearch engine and native actions, independent of the launcher window.
pub mod ai;
pub mod applog;
pub mod browser_indexer;
pub mod calculator;
pub mod git_indexer;
pub mod indexer;
pub mod launcher;
pub mod quick_actions;
pub mod search;
pub mod settings;
pub mod taskbar_recovery;
pub mod weather;
pub fn clipboard_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}
