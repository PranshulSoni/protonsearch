use serde_json::Value;

pub fn snapshot(id: &str) -> Result<Value, String> {
    // One arm per line, alphabetical, so parallel packages merge cleanly.
    match id {
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
