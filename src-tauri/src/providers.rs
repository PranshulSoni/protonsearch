use serde_json::{json, Value};

#[path = "hotkey_status.rs"]
pub mod hotkey_status;

fn envelope(status: &str, data: Value, message: &str) -> Value {
    let updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    json!({ "status": status, "data": data, "updatedAt": updated_at, "message": message })
}

pub fn snapshot(id: &str) -> Result<Value, String> {
    // One arm per line, alphabetical, so parallel packages merge cleanly.
    match id {
        "hotkey" => Ok(envelope("live", hotkey_status::read(), "")),
        _ => Err("Unknown provider".into()),
    }
}

pub fn action(id: &str) -> Result<(), String> {
    // One arm per line, alphabetical, so parallel packages merge cleanly.
    match id {
        _ => Err("Unsupported widget action".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_provider_is_rejected() {
        assert_eq!(snapshot("nope").unwrap_err(), "Unknown provider");
    }

    #[test]
    fn unknown_action_is_rejected() {
        assert_eq!(action("nope:x").unwrap_err(), "Unsupported widget action");
    }
}
