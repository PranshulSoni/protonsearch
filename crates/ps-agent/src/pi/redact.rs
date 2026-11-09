//! Secret redaction for Pi stderr, tool-result previews, errors and the final
//! assistant Output.
//!
//! Two layers (PI-SEC-04):
//! 1. exact known values: credential environment values, config-provided
//!    secrets and anything else the caller registers;
//! 2. shape-based patterns for credential formats that appear in model output
//!    or file contents: OpenAI sk-/pk-, Google AIza, Groq gsk_, HuggingFace
//!    hf_, GitHub ghp_/gho_/ghu_/ghs_/ghr_, AWS AKIA, Slack xox?- tokens,
//!    Authorization Bearer tokens and key=/token:/secret: assignments.
//!
//! Raw stderr is never forwarded to the UI; only redacted text is stored or
//! emitted in events.

const REDACTED: &str = "[redacted]";

pub(crate) fn redact_text(text: &str, known_secrets: &[String]) -> String {
    let mut exact = text.to_string();
    for secret in known_secrets {
        if secret.chars().count() >= 8 {
            exact = exact.replace(secret.as_str(), REDACTED);
        }
    }
    redact_shapes(&exact)
}

fn redact_shapes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        if let Some((consumed, replacement)) = match_shape(&chars, index) {
            out.push_str(&replacement);
            index += consumed;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

#[derive(Clone, Copy)]
enum TokenClass {
    KeyLike,
    UpperAlnum,
    Xox,
}

struct Prefix {
    text: &'static str,
    /// Minimum number of characters required after the prefix.
    min_rest: usize,
    class: TokenClass,
}

const PREFIXES: &[Prefix] = &[
    Prefix {
        text: "sk-",
        min_rest: 9,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "sk_",
        min_rest: 9,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "pk-",
        min_rest: 9,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "AIza",
        min_rest: 30,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "gsk_",
        min_rest: 16,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "hf_",
        min_rest: 16,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "ghp_",
        min_rest: 20,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "gho_",
        min_rest: 20,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "ghu_",
        min_rest: 20,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "ghs_",
        min_rest: 20,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "ghr_",
        min_rest: 20,
        class: TokenClass::KeyLike,
    },
    Prefix {
        text: "AKIA",
        min_rest: 16,
        class: TokenClass::UpperAlnum,
    },
    Prefix {
        text: "xoxb-",
        min_rest: 12,
        class: TokenClass::Xox,
    },
    Prefix {
        text: "xoxp-",
        min_rest: 12,
        class: TokenClass::Xox,
    },
    Prefix {
        text: "xoxa-",
        min_rest: 12,
        class: TokenClass::Xox,
    },
    Prefix {
        text: "xoxs-",
        min_rest: 12,
        class: TokenClass::Xox,
    },
];

/// Keys whose values are redacted when written as `key=...` / `key: ...`.
const ASSIGNMENT_KEYS: &[&str] = &[
    "api_key",
    "api-key",
    "apikey",
    "x-api-key",
    "access_token",
    "access-token",
    "auth_token",
    "auth-token",
    "refresh_token",
    "refresh-token",
    "client_secret",
    "client-secret",
    "password",
    "passwd",
    "secret",
    "token",
];

fn is_key_like(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')
}

fn is_upper_alnum(ch: char) -> bool {
    ch.is_ascii_uppercase() || ch.is_ascii_digit()
}

fn is_xox(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-'
}

fn is_value_char(ch: char) -> bool {
    ch.is_ascii_graphic() && !matches!(ch, '"' | '\'' | ',' | ';')
}

fn starts_with_ci(chars: &[char], index: usize, needle: &str) -> bool {
    let mut offset = 0;
    for expected in needle.chars() {
        match chars.get(index + offset) {
            Some(actual) if actual.eq_ignore_ascii_case(&expected) => offset += 1,
            _ => return false,
        }
    }
    true
}

fn boundary_before(chars: &[char], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    let previous = chars[index - 1];
    !previous.is_ascii_alphanumeric() && !matches!(previous, '_' | '-')
}

fn match_shape(chars: &[char], index: usize) -> Option<(usize, String)> {
    // Authorization: Bearer <token> / Basic <base64>
    for (scheme, label) in [("bearer", "Bearer"), ("basic", "Basic")] {
        if boundary_before(chars, index) && starts_with_ci(chars, index, scheme) {
            let mut cursor = index + scheme.chars().count();
            if chars.get(cursor).is_some_and(|ch| ch.is_whitespace()) {
                while chars.get(cursor).is_some_and(|ch| ch.is_whitespace()) {
                    cursor += 1;
                }
                let start = cursor;
                while chars
                    .get(cursor)
                    .is_some_and(|ch| is_key_like(*ch) || matches!(ch, '.' | '~' | '+' | '/' | '='))
                {
                    cursor += 1;
                }
                if cursor - start >= 16 || (scheme == "basic" && cursor - start >= 12) {
                    return Some((cursor - index, format!("{label} {REDACTED}")));
                }
            }
        }
    }

    // Well-known provider key prefixes.
    for prefix in PREFIXES {
        if !starts_with_ci(chars, index, prefix.text) {
            continue;
        }
        let mut cursor = index + prefix.text.chars().count();
        let start = cursor;
        while let Some(ch) = chars.get(cursor) {
            let accepted = match prefix.class {
                TokenClass::KeyLike => is_key_like(*ch),
                TokenClass::UpperAlnum => is_upper_alnum(*ch),
                TokenClass::Xox => is_xox(*ch),
            };
            if !accepted {
                break;
            }
            cursor += 1;
        }
        if cursor - start >= prefix.min_rest {
            return Some((cursor - index, REDACTED.to_string()));
        }
    }

    // key=value / key: value forms (non-Bearer).
    if boundary_before(chars, index) {
        for key in ASSIGNMENT_KEYS {
            if !starts_with_ci(chars, index, key) {
                continue;
            }
            let mut cursor = index + key.chars().count();
            while chars.get(cursor).is_some_and(|ch| *ch == ' ') {
                cursor += 1;
            }
            if !chars.get(cursor).is_some_and(|ch| matches!(ch, '=' | ':')) {
                continue;
            }
            cursor += 1;
            while chars.get(cursor).is_some_and(|ch| *ch == ' ') {
                cursor += 1;
            }
            let quote = chars.get(cursor).copied().filter(|ch| matches!(ch, '"' | '\''));
            if quote.is_some() {
                cursor += 1;
            }
            let start = cursor;
            while chars.get(cursor).is_some_and(|ch| is_value_char(*ch)) {
                cursor += 1;
            }
            let value_len = cursor - start;
            if quote.is_some() && chars.get(cursor).is_some_and(|ch| Some(*ch) == quote) {
                cursor += 1;
            }
            if value_len >= 8 {
                return Some((cursor - index, REDACTED.to_string()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redacted(text: &str) -> String {
        redact_text(text, &[])
    }

    #[test]
    fn exact_values_are_replaced() {
        let secret = "sk-live-abcdef1234567890".to_string();
        let output = redact_text("token=sk-live-abcdef1234567890 done", &[secret]);
        assert!(!output.contains("sk-live-abcdef"));
        assert!(output.contains(REDACTED));
    }

    #[test]
    fn openai_and_bearer_shapes_are_replaced() {
        let output =
            redacted("key sk-proj-abcdefghijklmnop and Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload");
        assert!(!output.contains("sk-proj-abcdefghijklmnop"), "{output}");
        assert!(!output.contains("eyJhbGciOiJIUzI1NiJ9"), "{output}");
        assert!(output.contains(REDACTED), "{output}");
    }

    #[test]
    fn broadened_provider_shapes_are_replaced() {
        let cases = [
            "AIzaSyD-EXAMPLEFAKEKEY0123456789abcdefg",
            "gsk_0123456789abcdef0123456789abcdef",
            "hf_0123456789abcdef0123456789abcdef",
            "ghp_0123456789abcdef0123456789abcdef",
            "AKIAIOSFODNN7EXAMPLE",
            "xoxb-123456789012-abcdefghijkl",
            "api_key=abcdefgh12345678",
            "password: hunter2hunter2",
            "Authorization: Basic dXNlcjpwYXNzd29yZA==",
        ];
        for case in cases {
            let output = redacted(case);
            assert!(output.contains(REDACTED), "not redacted: {case} -> {output}");
            if !case.starts_with("Authorization: Basic") {
                assert!(!output.contains(case), "leaked: {case} -> {output}");
            }
        }
    }

    #[test]
    fn ordinary_text_is_unchanged() {
        let text = "read src/main.rs: 42 lines and find key settings";
        assert_eq!(redact_text(text, &[]), text);
    }

    #[test]
    fn short_values_are_not_falsely_redacted() {
        assert_eq!(redacted("token=abc"), "token=abc");
        assert_eq!(redacted("sk-123"), "sk-123");
    }
}
