use serde_json::{json, Value};
use std::sync::Mutex;

static STATUS: Mutex<(Vec<String>, Vec<String>)> = Mutex::new((Vec::new(), Vec::new()));

pub fn hotkeys(configured: &str) -> Vec<String> {
    let mut keys = vec!["Alt+Space".to_string()];
    if configured != "Alt+Space" {
        keys.push(configured.to_string());
    }
    keys
}

pub fn record(shortcut: &str, ok: bool) {
    if let Ok(mut status) = STATUS.lock() {
        if ok {
            status.0.push(shortcut.to_string());
        } else {
            status.1.push(shortcut.to_string());
        }
    }
}

pub fn read() -> Value {
    let (registered, failed) = STATUS.lock().map(|s| s.clone()).unwrap_or_default();
    json!({ "registered": registered, "failed": failed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcut_is_always_first() {
        assert_eq!(hotkeys("Alt+Space"), vec!["Alt+Space"]);
        assert_eq!(hotkeys("Ctrl+Shift+F9"), vec!["Alt+Space", "Ctrl+Shift+F9"]);
    }

    #[test]
    fn records_success_and_failure_separately() {
        record("Test+HotkeyOk", true);
        record("Test+HotkeyFailed", false);
        let snapshot = read();
        let registered = snapshot["registered"].as_array().unwrap();
        let failed = snapshot["failed"].as_array().unwrap();
        assert!(registered.contains(&json!("Test+HotkeyOk")));
        assert!(!registered.contains(&json!("Test+HotkeyFailed")));
        assert!(failed.contains(&json!("Test+HotkeyFailed")));
        assert!(!failed.contains(&json!("Test+HotkeyOk")));
    }

    #[test]
    fn snapshot_has_both_lists() {
        let snapshot = read();
        assert!(snapshot["registered"].is_array());
        assert!(snapshot["failed"].is_array());
    }
}
