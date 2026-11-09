//! Secret redaction for Pi stderr and tool-result previews.
//!
//! Two layers: exact known values (values of credential environment variables
//! and config-provided secrets) and shape-based redaction (OpenAI-style
//! sk-/pk- keys and Authorization Bearer tokens). Raw stderr is never forwarded
//! to the UI; only the redacted ring buffer and redacted error messages are.

const REDACTED: &str = "[redacted]";

fn redact_shape_based(text: &str, out: &mut String) {
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let rest: String = chars[index..chars.len().min(index + 16)].iter().collect();
        if rest.starts_with("sk-") || rest.starts_with("pk-") {
            let mut end = index + 3;
            while end < chars.len() && (chars[end].is_ascii_alphanumeric() || matches!(chars[end], '-' | '_'))
            {
                end += 1;
            }
            if end - index >= 12 {
                out.push_str(REDACTED);
                index = end;
                continue;
            }
        }
        if rest.to_ascii_lowercase().starts_with("bearer ") {
            let mut end = index + 7;
            while end < chars.len()
                && (chars[end].is_ascii_alphanumeric()
                    || matches!(chars[end], '-' | '_' | '.' | '+' | '/' | '='))
            {
                end += 1;
            }
            if end - (index + 7) >= 16 {
                out.push_str("Bearer ");
                out.push_str(REDACTED);
                index = end;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
}

/// Replace known secret values and credential-shaped tokens.
pub(crate) fn redact_text(text: &str, known_secrets: &[String]) -> String {
    let mut exact = text.to_string();
    for secret in known_secrets {
        if secret.len() >= 8 {
            let current = std::mem::take(&mut exact);
            exact = current.replace(secret.as_str(), REDACTED);
        }
    }
    let mut shaped = String::with_capacity(exact.len());
    redact_shape_based(&exact, &mut shaped);
    shaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_values_are_replaced() {
        let secret = "sk-live-abcdef1234567890".to_string();
        let output = redact_text("token=sk-live-abcdef1234567890 done", &[secret]);
        assert!(!output.contains("sk-live-abcdef"));
        assert!(output.contains(REDACTED));
    }

    #[test]
    fn shaped_keys_and_bearer_tokens_are_replaced() {
        let output = redact_text(
            "key sk-proj-abcdefghijklmnop and Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload",
            &[],
        );
        assert!(output.contains(REDACTED), "{output}");
        assert!(!output.contains("sk-proj-abcdefghijklmnop"), "{output}");
        assert!(!output.contains("eyJhbGciOiJIUzI1NiJ9"), "{output}");
    }

    #[test]
    fn ordinary_text_is_unchanged() {
        let text = "read src/main.rs: 42 lines";
        assert_eq!(redact_text(text, &[]), text);
    }
}
