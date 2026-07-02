//! Minimal AI client for OpenSearch OS — talks to any OpenAI-compatible
//! chat-completions endpoint (DeepSeek by default). Blocking (ureq), runs on a
//! worker thread so the UI never stalls.

use anyhow::{anyhow, Result};
use std::os::windows::process::CommandExt;
use std::sync::atomic::AtomicBool;

pub static HERMES_GATEWAY_RUNNING: AtomicBool = AtomicBool::new(false);
pub static ALWAYS_APPROVE: AtomicBool = AtomicBool::new(false);

// DeepSeek V4 Flash (OpenAI-compatible). Override endpoint/model via env if desired.
const DEFAULT_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
const DEFAULT_MODEL: &str = "deepseek-chat";

// ── API key resolution ────────────────────────────────────────────────────────
// Order: env var → %APPDATA%/omnisearch/ai_key.txt → embedded rotated keys → hardcoded constant below.
const HARDCODED_KEY: &str = "";

const EMBEDDED_KEYS_RAW: Option<&str> = option_env!("OMNISEARCH_KEYS");

fn get_embedded_keys_count() -> usize {
    if let Some(raw) = EMBEDDED_KEYS_RAW {
        if raw.trim().is_empty() {
            0
        } else {
            raw.split(',').count()
        }
    } else {
        0
    }
}

fn get_rotated_key(index: usize) -> Option<String> {
    let raw = EMBEDDED_KEYS_RAW?;
    let parts: Vec<&str> = raw.split(',').collect();
    let hex_str = parts.get(index)?;
    
    let mut bytes = Vec::new();
    let chars: Vec<char> = hex_str.chars().collect();
    for i in (0..chars.len()).step_by(2) {
        if i + 1 < chars.len() {
            let hex_pair: String = chars[i..=i+1].iter().collect();
            if let Ok(b) = u8::from_str_radix(&hex_pair, 16) {
                bytes.push(b ^ 0x5F);
            }
        }
    }
    String::from_utf8(bytes).ok()
}

fn get_active_key_index(count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    if let Some(conn) = get_db_conn() {
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'active_key_index'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            if let Ok(idx) = val.trim().parse::<usize>() {
                return idx % count;
            }
        }
    }
    0
}

fn set_active_key_index(index: usize) {
    if let Some(conn) = get_db_conn() {
        let _ = conn.execute(
            "INSERT INTO ai_settings (key, value) VALUES ('active_key_index', ?1) \
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            [index.to_string()],
        );
    }
}

pub struct AiConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
}

fn get_db_conn() -> Option<std::sync::MutexGuard<'static, rusqlite::Connection>> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<rusqlite::Connection>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let appdata = std::env::var("APPDATA").ok();
        let path = appdata.map(|a| {
            std::path::PathBuf::from(a)
                .join("omnisearch")
                .join("file_index.db")
        });
        let conn = path
            .and_then(|p| rusqlite::Connection::open(&p).ok())
            .unwrap_or_else(|| {
                rusqlite::Connection::open_in_memory().unwrap()
            });
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        Mutex::new(conn)
    })
    .lock()
    .ok()
}

pub fn get_config() -> Result<AiConfig> {
    // 1. Resolve API key
    let mut api_key = None;
    let mut is_opencode = false;

    // Check SQLite settings table
    if let Some(conn) = get_db_conn() {
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'api_key'",
            [],
            |row| row.get::<_, String>(0),
        ) {
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
            let p = std::path::Path::new(&appdata)
                .join("omnisearch")
                .join("opencode_key.txt");
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
            let p = std::path::Path::new(&appdata)
                .join("omnisearch")
                .join("ai_key.txt");
            if let Ok(s) = std::fs::read_to_string(&p) {
                let k = s.trim().to_string();
                if !k.is_empty() {
                    api_key = Some(k);
                }
            }
        }
    }

    let mut is_embedded = false;

    // Check Embedded Rotated Keys
    if api_key.is_none() {
        let count = get_embedded_keys_count();
        if count > 0 {
            let idx = get_active_key_index(count);
            if let Some(k) = get_rotated_key(idx) {
                api_key = Some(k);
                is_embedded = true;
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
    if is_embedded || key.starts_with("sk-oc-") || key.contains("opencode") || key.starts_with("sk-HrvSzHIY") {
        is_opencode = true;
    }

    // 2. Resolve Endpoint
    let mut endpoint = None;

    // Check SQLite
    if let Some(conn) = get_db_conn() {
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'endpoint'",
            [],
            |row| row.get::<_, String>(0),
        ) {
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
            let p = std::path::Path::new(&appdata)
                .join("omnisearch")
                .join("ai_endpoint.txt");
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
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'model'",
            [],
            |row| row.get::<_, String>(0),
        ) {
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
            let p = std::path::Path::new(&appdata)
                .join("omnisearch")
                .join("ai_model.txt");
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
    let count = get_embedded_keys_count();
    let max_attempts = if count > 0 { count } else { 1 };

    for attempt in 0..max_attempts {
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

        match resp {
            Ok(r) => {
                let v: serde_json::Value = r.into_json()
                    .map_err(|e| anyhow!("bad AI response: {e}"))?;
                let text = v["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| anyhow!("AI response had no content"))?;
                return Ok(text.trim().to_string());
            }
            Err(ureq::Error::Status(code, r)) => {
                let msg = r.into_string().unwrap_or_default();
                let is_auth_or_quota = code == 401 || code == 429 || code == 402 ||
                                       msg.contains("insufficient_quota") ||
                                       msg.contains("quota") ||
                                       msg.contains("balance");

                let is_using_embedded = count > 0 && {
                    let idx = get_active_key_index(count);
                    get_rotated_key(idx).map(|k| k == cfg.api_key).unwrap_or(false)
                };

                if is_auth_or_quota && is_using_embedded && attempt + 1 < max_attempts {
                    let next_idx = (get_active_key_index(count) + 1) % count;
                    set_active_key_index(next_idx);
                    continue;
                }

                return Err(anyhow!(
                    "AI error {code}: {}",
                    msg.chars().take(300).collect::<String>()
                ));
            }
            Err(e) => return Err(anyhow!("AI request failed: {e}")),
        }
    }

    Err(anyhow!("All embedded API keys failed or were exhausted."))
}

/// Multi-turn chat completion. Passes conversation history to the API.
pub fn complete_chat(
    system: &str,
    prev_user: &str,
    prev_assistant: &str,
    user: &str,
) -> Result<String> {
    let count = get_embedded_keys_count();
    let max_attempts = if count > 0 { count } else { 1 };

    for attempt in 0..max_attempts {
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

        match resp {
            Ok(r) => {
                let v: serde_json::Value = r.into_json()
                    .map_err(|e| anyhow!("bad AI response: {e}"))?;
                let text = v["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| anyhow!("AI response had no content"))?;
                return Ok(text.trim().to_string());
            }
            Err(ureq::Error::Status(code, r)) => {
                let msg = r.into_string().unwrap_or_default();
                let is_auth_or_quota = code == 401 || code == 429 || code == 402 ||
                                       msg.contains("insufficient_quota") ||
                                       msg.contains("quota") ||
                                       msg.contains("balance");

                let is_using_embedded = count > 0 && {
                    let idx = get_active_key_index(count);
                    get_rotated_key(idx).map(|k| k == cfg.api_key).unwrap_or(false)
                };

                if is_auth_or_quota && is_using_embedded && attempt + 1 < max_attempts {
                    let next_idx = (get_active_key_index(count) + 1) % count;
                    set_active_key_index(next_idx);
                    continue;
                }

                return Err(anyhow!(
                    "AI error {code}: {}",
                    msg.chars().take(300).collect::<String>()
                ));
            }
            Err(e) => return Err(anyhow!("AI request failed: {e}")),
        }
    }

    Err(anyhow!("All embedded API keys failed or were exhausted."))
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
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'hermes_api_key'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            let val_trimmed = val.trim().to_string();
            if !val_trimmed.is_empty() {
                api_key = val_trimmed;
            }
        }
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'always_approve'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            ALWAYS_APPROVE.store(val.trim() == "1", std::sync::atomic::Ordering::Release);
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

    // Auto-configure the API server settings
    if hermes_cmd != "hermes" {
        let _ = std::process::Command::new(&hermes_cmd)
            .args(["config", "set", "platforms.api_server.enabled", "true"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .status();
        let _ = std::process::Command::new(&hermes_cmd)
            .args([
                "config",
                "set",
                "platforms.api_server.extra.host",
                "127.0.0.1",
            ])
            .creation_flags(0x08000000)
            .status();
        let _ = std::process::Command::new(&hermes_cmd)
            .args(["config", "set", "platforms.api_server.extra.port", "8642"])
            .creation_flags(0x08000000)
            .status();
        let _ = std::process::Command::new(&hermes_cmd)
            .args(["config", "set", "platforms.api_server.extra.key", "hermes"])
            .creation_flags(0x08000000)
            .status();
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        let log_dir = std::path::Path::new(&appdata).join("omnisearch");
        let _ = std::fs::create_dir_all(&log_dir);
        let log_file = log_dir.join("hermes_gateway.log");
        if let Ok(meta) = std::fs::metadata(&log_file) {
            if meta.len() > 1024 * 1024 {
                let _ = std::fs::remove_file(&log_file);
            }
        }
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
        {
            let _ = std::process::Command::new(&hermes_cmd)
                .args(["gateway", "run", "--replace", "--accept-hooks"])
                .stdout(file.try_clone().unwrap())
                .stderr(file)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn();
        }
    } else {
        let _ = std::process::Command::new(&hermes_cmd)
            .args(["gateway", "run", "--replace", "--accept-hooks"])
            .creation_flags(0x08000000)
            .spawn();
    }
}

fn ensure_hermes_gateway_running() -> Result<()> {
    if !HERMES_GATEWAY_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
        let hermes_cmd = get_hermes_executable();
        if hermes_cmd == "hermes" {
            let _ = std::process::Command::new("powershell")
                .args([
                    "-NoExit",
                    "-Command",
                    "Write-Host 'Hermes Agent not found. Starting automatic installation...'; iex (irm https://hermes-agent.nousresearch.com/install.ps1); Read-Host 'Installation completed. Press Enter to close'"
                ])
                .spawn();
            return Err(anyhow!("Hermes Agent is not installed. An installation window has been opened. Please wait for the setup to complete and try again."));
        }

        // Double check status
        let running = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8642".parse().unwrap(),
            std::time::Duration::from_millis(300),
        )
        .is_ok();
        if running {
            HERMES_GATEWAY_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);
            return Ok(());
        }

        start_hermes_gateway_daemon();

        let mut started = false;
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let running = std::net::TcpStream::connect_timeout(
                &"127.0.0.1:8642".parse().unwrap(),
                std::time::Duration::from_millis(300),
            )
            .is_ok();
            if running {
                HERMES_GATEWAY_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);
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

fn fallback_config_from_key(key: &str) -> Option<AiConfig> {
    if key.is_empty() || key == "hermes" {
        return None;
    }
    Some(AiConfig {
        endpoint: "https://opencode.ai/zen/v1/chat/completions".to_string(),
        model: "deepseek-v4-flash-free".to_string(),
        api_key: key.to_string(),
    })
}

fn get_agent_config() -> AiConfig {
    if HERMES_GATEWAY_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
        get_hermes_config()
    } else if let Ok(cfg) = get_config() {
        fallback_config_from_key(&cfg.api_key).unwrap_or_else(get_hermes_config)
    } else {
        get_hermes_config()
    }
}

/// Human-readable label for errors, based on which backend the request hit.
fn agent_label(cfg: &AiConfig) -> &'static str {
    if cfg.model == "hermes-agent" {
        "Hermes"
    } else {
        "AI"
    }
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
            return Err(anyhow!(
                "{label} error {code}: {}",
                msg.chars().take(300).collect::<String>()
            ));
        }
        Err(e) => return Err(anyhow!("{label} request failed: {e}")),
    };
    let v: serde_json::Value = resp
        .into_json()
        .map_err(|e| anyhow!("bad {label} response: {e}"))?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("{label} response had no content"))?;
    Ok(text.trim().to_string())
}

pub fn complete_chat_agent(
    system: &str,
    prev_user: &str,
    prev_assistant: &str,
    user: &str,
) -> Result<String> {
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
            return Err(anyhow!(
                "{label} error {code}: {}",
                msg.chars().take(300).collect::<String>()
            ));
        }
        Err(e) => return Err(anyhow!("{label} request failed: {e}")),
    };
    let v: serde_json::Value = resp
        .into_json()
        .map_err(|e| anyhow!("bad {label} response: {e}"))?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("{label} response had no content"))?;
    Ok(text.trim().to_string())
}

// ── Hermes Runs API (streaming + approval) ───────────────────────────────────
//
// The legacy `/v1/chat/completions` endpoint is blocking and stateless, so it
// cannot surface tool-approval prompts — when Hermes wants to run a tool that
// needs approval, the request just hangs until the timeout. The Runs API is the
// intended path for UIs that need to show an approve/deny button:
//
//   POST   /v1/runs                 — start a run, returns { run_id }
//   GET    /v1/runs/{id}/events     — SSE stream of run events
//   POST   /v1/runs/{id}/approval   — resolve a pending approval
//   GET    /v1/capabilities         — feature flags (run_submission, ...)
//
// The exact JSON shapes for the approval *request* event and the approval
// *decision* body are not fully published, so we parse defensively and send a
// body covering the likely shapes. If the live gateway doesn't match, callers
// fall back to the blocking `complete_agent` path.

const HERMES_BASE: &str = "http://127.0.0.1:8642";

/// An approval prompt surfaced from a running agent.
#[derive(Clone)]
pub struct HermesApproval {
    pub run_id: String,
    pub approval_id: String,
    pub tool: String,
    pub summary: String,
}

/// Callbacks the streaming run uses to talk back to the UI thread. Each is a
/// best-effort fire-and-forget (the UI owns its own message box / state).
pub trait RunCallbacks: Send {
    /// Callback when run_id is known, allowing database updates.
    fn on_run_id(&self, _run_id: &str) {}
    /// A tool needs approval. The UI shows an Approve/Deny button.
    fn on_approval(&self, approval: HermesApproval);
    /// Incremental progress text (assistant thinking/deltas). Replaces the
    /// "Executing…" line.
    fn on_progress(&self, text: &str);
    /// Terminal result. `ok=false` means `text` is an error message.
    fn on_done(&self, ok: bool, text: &str);
}

/// Returns true if the gateway advertises the runs + approval + SSE features.
pub fn supports_runs_api() -> bool {
    let cfg = get_agent_config();
    if cfg.model == "hermes-agent" {
        return true;
    }
    let cfg_hermes = get_hermes_config();
    let resp = ureq::get(&format!("{HERMES_BASE}/v1/capabilities"))
        .set("Authorization", &format!("Bearer {}", cfg_hermes.api_key))
        .timeout(std::time::Duration::from_secs(4))
        .call();
    let v = match resp {
        Ok(r) => r.into_json::<serde_json::Value>(),
        Err(_) => return false,
    };
    let v = match v {
        Ok(v) => v,
        Err(_) => return false,
    };
    // Tolerant: accept either a flat feature list or a nested { "features": {...} }.
    let has = |key: &str| -> bool {
        let k = [key, &format!("run_{key}"), &format!("features.{key}")];
        k.iter().any(|path| {
            let mut cur = &v;
            for seg in path.split('.') {
                match cur.get(seg) {
                    Some(n) => cur = n,
                    None => return false,
                }
            }
            cur.as_bool()
                .unwrap_or(cur.as_str().map_or(false, |s| s == "enabled"))
                || cur.as_array().map_or(false, |_| true)
        })
    };
    // If we can't find explicit flags but the endpoint exists, treat it as supported
    // only when the approval-specific flag is present.
    has("approval") || has("run_approval")
}

/// Resolve a pending approval for a run: `approved=true` continues, `false` denies.
pub fn resolve_run_approval(approval: &HermesApproval, approved: bool) -> Result<()> {
    let cfg = get_hermes_config();
    let url = format!("{HERMES_BASE}/v1/runs/{}/approval", approval.run_id);
    let choice = if approved {
        if ALWAYS_APPROVE.load(std::sync::atomic::Ordering::Acquire) {
            "always"
        } else {
            "once"
        }
    } else {
        "deny"
    };
    // Cover the likely decision shapes the gateway might expect.
    let body = serde_json::json!({
        "decision": if approved { "approved" } else { "denied" },
        "approved": approved,
        "approval_id": approval.approval_id,
        "status": if approved { "approved" } else { "denied" },
        "choice": choice,
    });
    let resp = ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send_json(body);
    match resp {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(code, r)) => {
            // 409/404 can mean "already resolved" — treat as success so the UI clears.
            if code == 409 || code == 404 {
                Ok(())
            } else {
                let msg = r.into_string().unwrap_or_default();
                Err(anyhow!(
                    "approval error {code}: {}",
                    msg.chars().take(200).collect::<String>()
                ))
            }
        }
        Err(e) => Err(anyhow!("approval request failed: {e}")),
    }
}

/// Start a run and stream its events. Blocks until the run completes (or fails),
/// invoking the callbacks along the way. Should run on a worker thread.
pub fn run_agent_streaming(system: &str, user: &str, cb: &dyn RunCallbacks) -> Result<()> {
    let cfg = get_hermes_config();
    ensure_hermes_gateway_running()?;

    // 1. Start the run.
    let start_body = serde_json::json!({
        "input": user,
        "instructions": system,
        "stream": true,
    });
    let start = ureq::post(&format!("{HERMES_BASE}/v1/runs"))
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .send_json(start_body);
    let start_v: serde_json::Value = match start {
        Ok(r) => r
            .into_json()
            .map_err(|e| anyhow!("bad run start response: {e}"))?,
        Err(ureq::Error::Status(code, r)) => {
            let msg = r.into_string().unwrap_or_default();
            return Err(anyhow!(
                "run start {code}: {}",
                msg.chars().take(300).collect::<String>()
            ));
        }
        Err(e) => return Err(anyhow!("run start failed: {e}")),
    };
    let run_id = start_v
        .get("run_id")
        .or_else(|| start_v.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("run start response had no run_id"))?
        .to_string();

    cb.on_run_id(&run_id);

    // 2. Open the SSE event stream.
    let events_url = format!("{HERMES_BASE}/v1/runs/{}/events", run_id);
    let resp = ureq::get(&events_url)
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .set("Accept", "text/event-stream")
        .timeout(std::time::Duration::from_secs(300))
        .call();
    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let msg = r.into_string().unwrap_or_default();
            return Err(anyhow!(
                "events {code}: {}",
                msg.chars().take(300).collect::<String>()
            ));
        }
        Err(e) => return Err(anyhow!("events request failed: {e}")),
    };

    // 3. Read the stream line by line, parsing SSE `data:` payloads.
    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(resp.into_reader());
    let mut final_text = String::new();
    let mut done_ok: Option<bool> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let payload = if let Some(d) = line.strip_prefix("data:") {
            d.trim().to_string()
        } else if line.starts_with("event:") || line.is_empty() {
            continue;
        } else {
            continue;
        };
        if payload == "[DONE]" {
            break;
        }
        let v: serde_json::Value = match serde_json::from_str(&payload) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(approval) = extract_approval(&v, &run_id) {
            cb.on_approval(approval);
            continue;
        }

        // Progress / delta text: accumulate whatever content we can find.
        let kind = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match kind {
            "completed" | "run.completed" | "response.completed" => {
                if let Some(t) = v.get("output").and_then(|o| o.as_str()) {
                    final_text = t.to_string();
                } else if let Some(t) = v.get("result").and_then(|o| o.as_str()) {
                    final_text = t.to_string();
                } else if let Some(t) = v.get("response").and_then(|o| o.as_str()) {
                    final_text = t.to_string();
                }
                done_ok = Some(true);
            }
            "failed" | "error" | "run.failed" => {
                let msg = v
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .or_else(|| v.get("message").and_then(|m| m.as_str()))
                    .unwrap_or("agent run failed");
                cb.on_done(false, msg);
                return Ok(());
            }
            _ => {
                // Streaming delta: append any text fragment we can find.
                if let Some(delta) = v.get("delta").and_then(|d| d.as_str()) {
                    final_text.push_str(delta);
                    cb.on_progress(&final_text);
                } else if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                    final_text.push_str(text);
                    cb.on_progress(&final_text);
                } else if let Some(content) = v.pointer("/delta/content").and_then(|c| c.as_str()) {
                    final_text.push_str(content);
                    cb.on_progress(&final_text);
                }
            }
        }
    }

    match done_ok {
        Some(true) => {
            cb.on_done(true, final_text.trim());
            Ok(())
        }
        _ => {
            // Stream ended without an explicit completion event. If we collected
            // any text, treat that as success; otherwise it's an error.
            if !final_text.trim().is_empty() {
                cb.on_done(true, final_text.trim());
                Ok(())
            } else {
                Err(anyhow!("agent run ended without a result"))
            }
        }
    }
}

/// Heuristically detect an approval-request event in an SSE payload and pull out
/// the fields the UI needs. Defensive: returns `None` if the payload doesn't
/// look like an approval request.
fn extract_approval(v: &serde_json::Value, run_id: &str) -> Option<HermesApproval> {
    // Type-based detection first.
    let kind = v
        .get("type")
        .or_else(|| v.get("event"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    let approval_types = [
        "approval_required",
        "approval.requested",
        "approval.request",
        "tool_call_pending",
        "approval_pending",
        "requires_approval",
        "permission_request",
    ];
    let mut looks_like_approval = approval_types.contains(&kind);

    // Key-based detection: presence of any of these keys signals approval.
    let approval_keys = [
        "approval_id",
        "approval",
        "permission_id",
        "needs_approval",
        "requires_approval",
    ];
    for k in approval_keys {
        if v.get(k).is_some() {
            looks_like_approval = true;
            break;
        }
    }
    if !looks_like_approval {
        return None;
    }

    // opt(key) returns the first non-empty string value found at `key`.
    let opt = |key: &str| -> Option<String> {
        v.get(key)
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    let approval_id = opt("approval_id")
        .or_else(|| opt("approval"))
        .or_else(|| opt("permission_id"))
        .or_else(|| opt("id"))
        .unwrap_or_default();

    let tool = opt("tool")
        .or_else(|| opt("tool_name"))
        .or_else(|| opt("command"))
        .or_else(|| opt("name"))
        .unwrap_or_default();
    let summary = opt("summary")
        .or_else(|| opt("description"))
        .or_else(|| opt("message"))
        .or_else(|| opt("reason"))
        .or_else(|| {
            // Some payloads nest the tool call under "data" / "payload".
            let d = v.get("data").or_else(|| v.get("payload"))?;
            d.get("command")
                .and_then(|c| c.as_str())
                .map(|c| c.to_string())
        })
        .unwrap_or_default();

    Some(HermesApproval {
        run_id: run_id.to_string(),
        approval_id,
        tool,
        summary,
    })
}

/// Map a command + input to a (system prompt, user content) and run it.
/// Fetch a URL and return a plain-text approximation of its readable content.
fn fetch_url_text(url: &str) -> Result<String> {
    let resp = ureq::get(url)
        .set("User-Agent", "Mozilla/5.0 (OpenSearch-OS)")
        .timeout(std::time::Duration::from_secs(20))
        .call()
        .map_err(|e| anyhow!("Couldn't fetch the page: {e}"))?;
    let html = resp
        .into_string()
        .map_err(|e| anyhow!("Couldn't read the page: {e}"))?;
    Ok(html_to_text(&html))
}

/// Crude HTML→text: drop script/style, strip tags, decode a few entities, collapse
/// whitespace. ponytail: good enough to summarize; not a real parser. ASCII-only tag
/// matching keeps byte indexing UTF-8-safe.
fn html_to_text(html: &str) -> String {
    let b = html.as_bytes();
    let n = b.len();
    let mut out = String::with_capacity(n / 2);
    let starts_ci =
        |i: usize, pat: &[u8]| i + pat.len() <= n && b[i..i + pat.len()].eq_ignore_ascii_case(pat);
    let find_ci = |from: usize, pat: &[u8]| -> Option<usize> {
        if pat.is_empty() || from >= n {
            return None;
        }
        (from..=n.saturating_sub(pat.len()))
            .find(|&j| b[j..j + pat.len()].eq_ignore_ascii_case(pat))
    };
    let mut i = 0;
    while i < n {
        if starts_ci(i, b"<script") || starts_ci(i, b"<style") {
            let close: &[u8] = if starts_ci(i, b"<script") {
                b"</script>"
            } else {
                b"</style>"
            };
            match find_ci(i, close) {
                Some(end) => {
                    i = end + close.len();
                    continue;
                }
                None => break,
            }
        }
        if b[i] == b'<' {
            match find_ci(i, b">") {
                Some(end) => {
                    i = end + 1;
                    out.push(' ');
                    continue;
                }
                None => break,
            }
        }
        let ch_len = match b[i] {
            x if x < 0x80 => 1,
            x if x < 0xE0 => 2,
            x if x < 0xF0 => 3,
            _ => 4,
        };
        let end = (i + ch_len).min(n);
        if let Ok(seg) = std::str::from_utf8(&b[i..end]) {
            out.push_str(seg);
        }
        i = end;
    }
    let out = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Commands: ask, explain, grammar, translate, summarize.
pub fn run(cmd: &str, input: &str) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        return Err(anyhow!(
            "Nothing to send — type text or copy something first."
        ));
    }
    // Summarize Webpage: if the input is a URL, fetch it and strip to text first.
    let owned_input: String;
    let input: &str =
        if cmd == "summarize" && (input.starts_with("http://") || input.starts_with("https://")) {
            let mut text = fetch_url_text(input)?;
            text.truncate(12000); // keep the prompt small
            if text.trim().is_empty() {
                return Err(anyhow!("Couldn't extract readable text from that page."));
            }
            owned_input = text;
            &owned_input
        } else {
            input
        };
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

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_html_to_text() {
        let html = "<html><head><style>p{color:red}</style></head><body><p>Hello &amp; <b>world</b></p><script>alert(1)</script></body></html>";
        assert_eq!(super::html_to_text(html), "Hello & world");
    }

    #[test]
    fn test_config_resolution() {
        let _guard = TEST_LOCK.lock().unwrap();
        // Isolate APPDATA to a temporary path to avoid reading host DB/configs
        let old_appdata = std::env::var("APPDATA").ok();
        let temp_dir = std::env::temp_dir().join("omnisearch-test-appdata");
        let _ = std::fs::remove_dir_all(&temp_dir); // Clean any stale database/directory
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
        let _guard = TEST_LOCK.lock().unwrap();
        let old_appdata = std::env::var("APPDATA").ok();
        let temp_dir = std::env::temp_dir().join("omnisearch-test-hermes-fallback");
        let _ = std::fs::remove_dir_all(&temp_dir); // Clean any stale database/directory
        let app_dir = temp_dir.join("omnisearch");
        let _ = std::fs::create_dir_all(&app_dir);
        std::env::set_var("APPDATA", &temp_dir);
        std::env::remove_var("OPENCODE_API_KEY");
        std::env::remove_var("DEEPSEEK_API_KEY");
        std::env::remove_var("OPENSEARCH_AI_KEY");
        HERMES_GATEWAY_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);

        let conn = rusqlite::Connection::open(app_dir.join("file_index.db")).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_settings (key TEXT PRIMARY KEY, value TEXT);",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('model', 'hermes-agent');",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('api_key', 'sk-H-test-key');",
            [],
        )
        .unwrap();
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

#[derive(serde::Deserialize, Debug)]
pub struct RunStatusResponse {
    pub status: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

pub fn get_run_status(run_id: &str) -> Result<RunStatusResponse> {
    let cfg = get_hermes_config();
    let url = format!("{HERMES_BASE}/v1/runs/{run_id}");
    let resp = ureq::get(&url)
        .set("Authorization", &format!("Bearer {}", cfg.api_key))
        .timeout(std::time::Duration::from_secs(5))
        .call()?;
    let status_resp: RunStatusResponse = resp.into_json()?;
    Ok(status_resp)
}

pub fn poll_and_stream_existing_run(run_id: &str, cb: &dyn RunCallbacks) -> Result<()> {
    let mut seen_waiting = false;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let status_resp = match get_run_status(run_id) {
            Ok(r) => r,
            Err(_) => continue, // ignore transient network errors
        };

        match status_resp.status.as_str() {
            "queued" | "running" => {
                // Still running
            }
            "waiting_for_approval" => {
                if !seen_waiting {
                    seen_waiting = true;
                    cb.on_approval(HermesApproval {
                        run_id: run_id.to_string(),
                        approval_id: "".to_string(),
                        tool: "System command".to_string(),
                        summary: "Hermes is waiting for your approval to run a command."
                            .to_string(),
                    });
                }
            }
            "completed" => {
                let out = status_resp.output.unwrap_or_default();
                cb.on_done(true, &out);
                break;
            }
            "failed" => {
                let err = status_resp
                    .error
                    .unwrap_or_else(|| "Run failed".to_string());
                cb.on_done(false, &err);
                break;
            }
            _ => {
                cb.on_done(false, "Run terminated");
                break;
            }
        }
    }
    Ok(())
}
