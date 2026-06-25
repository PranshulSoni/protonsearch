//! Minimal AI client for OpenSearch OS — talks to any OpenAI-compatible
//! chat-completions endpoint (DeepSeek by default). Blocking (ureq), runs on a
//! worker thread so the UI never stalls.

use anyhow::{anyhow, Result};
use std::sync::atomic::AtomicBool;
use std::os::windows::process::CommandExt;

pub static HERMES_GATEWAY_RUNNING: AtomicBool = AtomicBool::new(false);

// DeepSeek V4 Flash (OpenAI-compatible). Override endpoint/model via env if desired.
const DEFAULT_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
const DEFAULT_MODEL: &str = "deepseek-chat";

// ── API key resolution ────────────────────────────────────────────────────────
// Order: env var → %APPDATA%/opensearch-os/ai_key.txt → hardcoded constant below.
// Leave the constant empty in source (never commit a real key); the user pastes
// their DeepSeek key into the file or env var.
const HARDCODED_KEY: &str = "";

pub struct AiConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
}

fn get_db_conn() -> Option<rusqlite::Connection> {
    let appdata = std::env::var("APPDATA").ok()?;
    let path = std::path::PathBuf::from(appdata).join("opensearch-os").join("file_index.db");
    let conn = rusqlite::Connection::open(&path).ok()?;
    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
    Some(conn)
}

pub fn get_config() -> Result<AiConfig> {
    // 1. Resolve API key
    let mut api_key = None;
    let mut is_opencode = false;

    // Check SQLite settings table
    if let Some(conn) = get_db_conn() {
        if let Ok(val) = conn.query_row("SELECT value FROM ai_settings WHERE key = 'api_key'", [], |row| row.get::<_, String>(0)) {
            let val_trimmed = val.trim().to_string();
            if !val_trimmed.is_empty() {
                if val_trimmed.starts_with("sk-oc-") || val_trimmed.contains("opencode") {
                    is_opencode = true;
                }
                api_key = Some(val_trimmed);
            }
        }
    }

    // Check Environment Variables
    if api_key.is_none() {
        if let Ok(k) = std::env::var("OPENCODE_API_KEY") {
            if !k.trim().is_empty() {
                api_key = Some(k.trim().to_string());
                is_opencode = true;
            }
        }
    }
    if api_key.is_none() {
        if let Ok(k) = std::env::var("DEEPSEEK_API_KEY") {
            if !k.trim().is_empty() {
                api_key = Some(k.trim().to_string());
            }
        }
    }
    if api_key.is_none() {
        if let Ok(k) = std::env::var("OPENSEARCH_AI_KEY") {
            if !k.trim().is_empty() {
                api_key = Some(k.trim().to_string());
            }
        }
    }

    // Check AppData files
    if api_key.is_none() {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = std::path::Path::new(&appdata).join("opensearch-os").join("opencode_key.txt");
            if let Ok(s) = std::fs::read_to_string(&p) {
                let k = s.trim().to_string();
                if !k.is_empty() {
                    api_key = Some(k);
                    is_opencode = true;
                }
            }
        }
    }
    if api_key.is_none() {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = std::path::Path::new(&appdata).join("opensearch-os").join("ai_key.txt");
            if let Ok(s) = std::fs::read_to_string(&p) {
                let k = s.trim().to_string();
                if !k.is_empty() {
                    api_key = Some(k);
                }
            }
        }
    }

    // Check Hardcoded Key
    if api_key.is_none() && !HARDCODED_KEY.is_empty() {
        api_key = Some(HARDCODED_KEY.to_string());
    }

    let key = api_key.ok_or_else(|| anyhow!(
        "No AI API key found. Type 'ai config key <your_key>' in search or set OPENCODE_API_KEY/DEEPSEEK_API_KEY environment variable."
    ))?;

    // If key contains cues about OpenCode Zen
    if key.starts_with("sk-oc-") || key.contains("opencode") {
        is_opencode = true;
    }

    // 2. Resolve Endpoint
    let mut endpoint = None;

    // Check SQLite
    if let Some(conn) = get_db_conn() {
        if let Ok(val) = conn.query_row("SELECT value FROM ai_settings WHERE key = 'endpoint'", [], |row| row.get::<_, String>(0)) {
            let val_trimmed = val.trim().to_string();
            if !val_trimmed.is_empty() {
                endpoint = Some(val_trimmed);
            }
        }
    }

    // Check Environment Variable
    if endpoint.is_none() {
        if let Ok(ep) = std::env::var("OPENSEARCH_AI_ENDPOINT") {
            if !ep.trim().is_empty() {
                endpoint = Some(ep.trim().to_string());
            }
        }
    }

    // Check AppData files
    if endpoint.is_none() {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = std::path::Path::new(&appdata).join("opensearch-os").join("ai_endpoint.txt");
            if let Ok(s) = std::fs::read_to_string(&p) {
                let ep = s.trim().to_string();
                if !ep.is_empty() {
                    endpoint = Some(ep);
                }
            }
        }
    }

    // Fallback Default
    let endpoint = endpoint.unwrap_or_else(|| {
        if is_opencode {
            "https://opencode.ai/zen/v1/chat/completions".to_string()
        } else {
            DEFAULT_ENDPOINT.to_string()
        }
    });

    // 3. Resolve Model
    let mut model = None;

    // Check SQLite
    if let Some(conn) = get_db_conn() {
        if let Ok(val) = conn.query_row("SELECT value FROM ai_settings WHERE key = 'model'", [], |row| row.get::<_, String>(0)) {
            let val_trimmed = val.trim().to_string();
            if !val_trimmed.is_empty() {
                model = Some(val_trimmed);
            }
        }
    }

    // Check Environment Variable
    if model.is_none() {
        if let Ok(m) = std::env::var("OPENSEARCH_AI_MODEL") {
            if !m.trim().is_empty() {
                model = Some(m.trim().to_string());
            }
        }
    }

    // Check AppData files
    if model.is_none() {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = std::path::Path::new(&appdata).join("opensearch-os").join("ai_model.txt");
            if let Ok(s) = std::fs::read_to_string(&p) {
                let m = s.trim().to_string();
                if !m.is_empty() {
                    model = Some(m);
                }
            }
        }
    }

    // Fallback Default
    let model = model.unwrap_or_else(|| {
        if is_opencode {
            "deepseek-v4-flash-free".to_string()
        } else {
            DEFAULT_MODEL.to_string()
        }
    });

    Ok(AiConfig {
        endpoint,
        model,
        api_key: key,
    })
}

/// One-shot chat completion (non-streaming). Returns the assistant's text.
pub fn complete(system: &str, user: &str) -> Result<String> {
    let cfg = get_config()?;

    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "stream": false,
        "temperature": 0.3,
    });

    let timeout_secs = if cfg.model == "hermes-agent" { 300 } else { 30 };
    let resp = ureq::post(&cfg.endpoint)
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send_json(body);

    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let msg = r.into_string().unwrap_or_default();
            return Err(anyhow!("AI error {code}: {}", msg.chars().take(300).collect::<String>()));
        }
        Err(e) => return Err(anyhow!("AI request failed: {e}")),
    };

    let v: serde_json::Value = resp.into_json().map_err(|e| anyhow!("bad AI response: {e}"))?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("AI response had no content"))?;
    Ok(text.trim().to_string())
}

/// Multi-turn chat completion. Passes conversation history to the API.
pub fn complete_chat(system: &str, prev_user: &str, prev_assistant: &str, user: &str) -> Result<String> {
    let cfg = get_config()?;

    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": prev_user },
            { "role": "assistant", "content": prev_assistant },
            { "role": "user", "content": user }
        ],
        "stream": false,
        "temperature": 0.3,
    });

    let timeout_secs = if cfg.model == "hermes-agent" { 300 } else { 30 };
    let resp = ureq::post(&cfg.endpoint)
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send_json(body);

    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let msg = r.into_string().unwrap_or_default();
            return Err(anyhow!("AI error {code}: {}", msg.chars().take(300).collect::<String>()));
        }
        Err(e) => return Err(anyhow!("AI request failed: {e}")),
    };

    let v: serde_json::Value = resp.into_json().map_err(|e| anyhow!("bad AI response: {e}"))?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("AI response had no content"))?;
    Ok(text.trim().to_string())
}

fn get_hermes_config() -> AiConfig {
    let mut api_key = "hermes".to_string();
    if let Ok(k) = std::env::var("API_SERVER_KEY") {
        if !k.trim().is_empty() {
            api_key = k.trim().to_string();
        }
    }
    if let Ok(k) = std::env::var("HERMES_API_KEY") {
        if !k.trim().is_empty() {
            api_key = k.trim().to_string();
        }
    }
    if let Some(conn) = get_db_conn() {
        if let Ok(val) = conn.query_row("SELECT value FROM ai_settings WHERE key = 'hermes_api_key'", [], |row| row.get::<_, String>(0)) {
            let val_trimmed = val.trim().to_string();
            if !val_trimmed.is_empty() {
                api_key = val_trimmed;
            }
        }
    }
    AiConfig {
        endpoint: "http://127.0.0.1:8642/v1/chat/completions".to_string(),
        model: "hermes-agent".to_string(),
        api_key,
    }
}



pub fn get_hermes_executable() -> String {
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let venv_path = std::path::Path::new(&localappdata)
            .join("hermes")
            .join("hermes-agent")
            .join("venv")
            .join("Scripts")
            .join("hermes.exe");
        if venv_path.exists() {
            return venv_path.to_string_lossy().to_string();
        }

        let cmd_path = std::path::Path::new(&localappdata)
            .join("hermes")
            .join("bin")
            .join("hermes.cmd");
        if cmd_path.exists() {
            return cmd_path.to_string_lossy().to_string();
        }
    }
    "hermes".to_string()
}

pub fn start_hermes_gateway_daemon() {
    let hermes_cmd = get_hermes_executable();

    if let Ok(appdata) = std::env::var("APPDATA") {
        let log_dir = std::path::Path::new(&appdata).join("opensearch-os");
        let _ = std::fs::create_dir_all(&log_dir);
        let log_file = log_dir.join("hermes_gateway.log");
        if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(log_file) {
            let _ = std::process::Command::new(&hermes_cmd)
                .arg("gateway")
                .stdout(file.try_clone().unwrap())
                .stderr(file)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn();
        }
    } else {
        let _ = std::process::Command::new(&hermes_cmd)
            .arg("gateway")
            .creation_flags(0x08000000)
            .spawn();
    }
}

fn ensure_hermes_gateway_running() -> Result<()> {
    if !HERMES_GATEWAY_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        // Double check status
        let running = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8642".parse().unwrap(),
            std::time::Duration::from_millis(300)
        ).is_ok();
        if running {
            HERMES_GATEWAY_RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);
            return Ok(());
        }

        start_hermes_gateway_daemon();

        let mut started = false;
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let running = std::net::TcpStream::connect_timeout(
                &"127.0.0.1:8642".parse().unwrap(),
                std::time::Duration::from_millis(300)
            ).is_ok();
            if running {
                HERMES_GATEWAY_RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);
                started = true;
                break;
            }
        }

        if !started {
            return Err(anyhow!("Hermes gateway is not running and failed to start automatically. Please check your installation."));
        }
    }
    Ok(())
}

fn get_agent_config() -> AiConfig {
    get_hermes_config()
}

/// Human-readable label for errors, based on which backend the request hit.
fn agent_label(cfg: &AiConfig) -> &'static str {
    if cfg.model == "hermes-agent" { "Hermes" } else { "AI" }
}

pub fn complete_agent(system: &str, user: &str) -> Result<String> {
    let cfg = get_agent_config();
    ensure_hermes_gateway_running()?;
    let timeout_secs = 300;
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "stream": false,
        "temperature": 0.3,
    });
    let resp = ureq::post(&cfg.endpoint)
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send_json(body);
    let label = agent_label(&cfg);
    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let msg = r.into_string().unwrap_or_default();
            return Err(anyhow!("{label} error {code}: {}", msg.chars().take(300).collect::<String>()));
        }
        Err(e) => return Err(anyhow!("{label} request failed: {e}")),
    };
    let v: serde_json::Value = resp.into_json().map_err(|e| anyhow!("bad {label} response: {e}"))?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("{label} response had no content"))?;
    Ok(text.trim().to_string())
}

pub fn complete_chat_agent(system: &str, prev_user: &str, prev_assistant: &str, user: &str) -> Result<String> {
    let cfg = get_agent_config();
    ensure_hermes_gateway_running()?;
    let timeout_secs = 300;
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": prev_user },
            { "role": "assistant", "content": prev_assistant },
            { "role": "user", "content": user }
        ],
        "stream": false,
        "temperature": 0.3,
    });
    let resp = ureq::post(&cfg.endpoint)
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send_json(body);
    let label = agent_label(&cfg);
    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let msg = r.into_string().unwrap_or_default();
            return Err(anyhow!("{label} error {code}: {}", msg.chars().take(300).collect::<String>()));
        }
        Err(e) => return Err(anyhow!("{label} request failed: {e}")),
    };
    let v: serde_json::Value = resp.into_json().map_err(|e| anyhow!("bad {label} response: {e}"))?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("{label} response had no content"))?;
    Ok(text.trim().to_string())
}

/// Map a command + input to a (system prompt, user content) and run it.
/// Commands: ask, explain, grammar, translate, summarize.
pub fn run(cmd: &str, input: &str) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        return Err(anyhow!("Nothing to send — type text or copy something first."));
    }
    let (system, user): (&str, String) = match cmd {
        "ask" | "chat" => (
            "You are a concise, helpful assistant. Answer directly in at most a few short paragraphs.",
            input.to_string(),
        ),
        "explain" => (
            "Explain the following clearly and simply for a general audience. Be concise.",
            input.to_string(),
        ),
        "grammar" => (
            "Fix the spelling and grammar of the text. Output ONLY the corrected text, with no preamble or quotes.",
            input.to_string(),
        ),
        "translate" => (
            "You are a translator. If the input names a target language (e.g. 'X to Spanish'), translate X into it; otherwise translate the text to English. Output ONLY the translation.",
            input.to_string(),
        ),
        "summarize" => (
            "Summarize the following text concisely as a few short bullet points.",
            input.to_string(),
        ),
        "bugs" => (
            "You are a code reviewer. List likely bugs and issues in the following code as short bullet points. Be specific.",
            input.to_string(),
        ),
        _ => (
            "You are a concise, helpful assistant.",
            input.to_string(),
        ),
    };
    complete(system, &user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_resolution() {
        // Isolate APPDATA to a temporary path to avoid reading host DB/configs
        let old_appdata = std::env::var("APPDATA").ok();
        let temp_dir = std::env::temp_dir().join("opensearch-os-test-appdata");
        let _ = std::fs::create_dir_all(&temp_dir);
        std::env::set_var("APPDATA", &temp_dir);

        // Clear environment variables that might interfere
        std::env::remove_var("OPENCODE_API_KEY");
        std::env::remove_var("DEEPSEEK_API_KEY");
        std::env::remove_var("OPENSEARCH_AI_KEY");
        std::env::remove_var("OPENSEARCH_AI_ENDPOINT");
        std::env::remove_var("OPENSEARCH_AI_MODEL");

        // Temporarily set OPENCODE_API_KEY env to test fallback
        std::env::set_var("OPENCODE_API_KEY", "sk-oc-test-key-12345");
        let cfg = get_config().unwrap();
        assert_eq!(cfg.api_key, "sk-oc-test-key-12345");
        assert_eq!(cfg.endpoint, "https://opencode.ai/zen/v1/chat/completions");
        assert_eq!(cfg.model, "deepseek-v4-flash-free");

        // Cleanup
        std::env::remove_var("OPENCODE_API_KEY");

        // Now set DEEPSEEK_API_KEY
        std::env::set_var("DEEPSEEK_API_KEY", "sk-ds-test-key-12345");
        let cfg2 = get_config().unwrap();
        assert_eq!(cfg2.api_key, "sk-ds-test-key-12345");
        assert_eq!(cfg2.endpoint, "https://api.deepseek.com/chat/completions");
        assert_eq!(cfg2.model, "deepseek-chat");

        // Cleanup
        std::env::remove_var("DEEPSEEK_API_KEY");

        // Restore APPDATA
        if let Some(val) = old_appdata {
            std::env::set_var("APPDATA", val);
        } else {
            std::env::remove_var("APPDATA");
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn hermes_falls_back_to_opencode_key_when_gateway_is_down() {
        let old_appdata = std::env::var("APPDATA").ok();
        let temp_dir = std::env::temp_dir().join("opensearch-os-test-hermes-fallback");
        let app_dir = temp_dir.join("opensearch-os");
        let _ = std::fs::create_dir_all(&app_dir);
        std::env::set_var("APPDATA", &temp_dir);
        std::env::remove_var("OPENCODE_API_KEY");
        std::env::remove_var("DEEPSEEK_API_KEY");
        std::env::remove_var("OPENSEARCH_AI_KEY");
        HERMES_GATEWAY_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);

        let conn = rusqlite::Connection::open(app_dir.join("file_index.db")).unwrap();
        conn.execute("CREATE TABLE ai_settings (key TEXT PRIMARY KEY, value TEXT);", []).unwrap();
        conn.execute("INSERT INTO ai_settings (key, value) VALUES ('model', 'hermes-agent');", []).unwrap();
        conn.execute("INSERT INTO ai_settings (key, value) VALUES ('api_key', 'sk-H-test-key');", []).unwrap();
        drop(conn);

        let cfg = get_agent_config();

        assert_eq!(cfg.endpoint, "https://opencode.ai/zen/v1/chat/completions");
        assert_eq!(cfg.model, "deepseek-v4-flash-free");
        assert_eq!(cfg.api_key, "sk-H-test-key");

        if let Some(val) = old_appdata {
            std::env::set_var("APPDATA", val);
        } else {
            std::env::remove_var("APPDATA");
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
