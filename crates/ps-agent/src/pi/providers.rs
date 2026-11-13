//! Providers Pi can use, kept in Pi's own config folder: API keys in `auth.json`,
//! OpenAI-compatible endpoints in `models.json`. Pi reads both, so
//! `pi --list-models` lists their models. Keys are written, never read back.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Map, Value};

const AUTH: &str = "auth.json";
const MODELS: &str = "models.json";

/// Built-in Pi providers that take an API key: (Pi id, name, Pi's env variable).
pub const BUILTIN_PROVIDERS: &[(&str, &str, &str)] = &[
    ("openai", "OpenAI", "OPENAI_API_KEY"),
    ("anthropic", "Anthropic", "ANTHROPIC_API_KEY"),
    ("google", "Google Gemini", "GEMINI_API_KEY"),
    ("openrouter", "OpenRouter", "OPENROUTER_API_KEY"),
    ("groq", "Groq", "GROQ_API_KEY"),
    ("xai", "xAI", "XAI_API_KEY"),
    ("deepseek", "DeepSeek", "DEEPSEEK_API_KEY"),
    ("mistral", "Mistral", "MISTRAL_API_KEY"),
    ("opencode", "OpenCode Zen", "OPENCODE_API_KEY"),
    ("opencode-go", "OpenCode Go", "OPENCODE_API_KEY"),
];

/// One provider for the Workflows page. Never carries a key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiProvider {
    pub id: String,
    pub name: String,
    /// An OpenAI-compatible endpoint from `models.json`.
    pub custom: bool,
    /// Pi's config holds a credential for it.
    pub saved: bool,
    /// Its key is set in the environment.
    pub from_env: bool,
    pub base_url: Option<String>,
    pub models: Vec<String>,
}

/// Pi's config folder: `PI_CODING_AGENT_DIR`, else `~/.pi/agent`.
pub fn agent_dir() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("PI_CODING_AGENT_DIR").filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    home.map(|home| PathBuf::from(home).join(".pi").join("agent"))
        .ok_or_else(|| "Pi's config folder could not be found.".to_string())
}

pub fn list(dir: &Path) -> Result<Vec<PiProvider>, String> {
    let auth = read(&dir.join(AUTH))?;
    let mut providers: Vec<PiProvider> = BUILTIN_PROVIDERS
        .iter()
        .map(|(id, name, env)| PiProvider {
            id: id.to_string(),
            name: name.to_string(),
            custom: false,
            saved: auth.contains_key(*id),
            from_env: std::env::var_os(env).is_some_and(|value| !value.is_empty()),
            base_url: None,
            models: Vec::new(),
        })
        .collect();
    let models = read(&dir.join(MODELS))?;
    for (id, entry) in models.get("providers").and_then(Value::as_object).into_iter().flatten() {
        // A built-in id there only adjusts that provider (a proxy URL, say).
        if is_builtin(id) {
            continue;
        }
        let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_string);
        providers.push(PiProvider {
            id: id.clone(),
            name: text("name").unwrap_or_else(|| id.clone()),
            custom: true,
            saved: true,
            from_env: false,
            base_url: text("baseUrl"),
            models: entry
                .get("models")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|model| Some(model.get("id")?.as_str()?.to_string()))
                .collect(),
        });
    }
    Ok(providers)
}

/// Stores the API key of a built-in provider in `auth.json`.
pub fn save_key(dir: &Path, id: &str, key: &str) -> Result<Vec<PiProvider>, String> {
    if !is_builtin(id) {
        return Err("Choose a provider from the list.".into());
    }
    let key = key.trim();
    if key.is_empty() {
        return Err("Enter the API key.".into());
    }
    let path = dir.join(AUTH);
    let mut auth = read(&path)?;
    auth.insert(id.to_string(), json!({ "type": "api_key", "key": literal(key) }));
    write(&path, auth)?;
    list(dir)
}

/// Adds or replaces an OpenAI-compatible endpoint in `models.json`.
pub fn save_endpoint(dir: &Path, name: &str, base_url: &str, key: &str, models: &[String]) -> Result<Vec<PiProvider>, String> {
    let name = name.trim();
    let id = slug(name);
    if id.is_empty() {
        return Err("Enter a name.".into());
    }
    if is_builtin(&id) {
        return Err("That name belongs to a built-in provider. Choose another.".into());
    }
    let base_url = base_url.trim();
    if !base_url.starts_with("https://") && !base_url.starts_with("http://") {
        return Err("The base URL must start with http:// or https://.".into());
    }
    let models: Vec<Value> = models
        .iter()
        .map(|model| model.trim())
        .filter(|model| !model.is_empty())
        .map(|model| json!({ "id": model }))
        .collect();
    if models.is_empty() {
        return Err("Enter at least one model id.".into());
    }
    let key = key.trim();
    let path = dir.join(MODELS);
    let mut file = read(&path)?;
    let providers = file.entry("providers").or_insert_with(|| json!({}));
    let providers = providers.as_object_mut().ok_or("Pi's models.json has an unexpected layout.")?;
    providers.insert(
        id,
        json!({
            "name": name,
            "baseUrl": base_url,
            "api": "openai-completions",
            // Pi offers a provider's models only when it has a key; local servers ignore this one.
            "apiKey": if key.is_empty() { "none".to_string() } else { literal(key) },
            "models": models,
        }),
    );
    write(&path, file)?;
    list(dir)
}

/// Removes a provider's stored key, and an endpoint's entry.
pub fn remove(dir: &Path, id: &str) -> Result<Vec<PiProvider>, String> {
    let path = dir.join(AUTH);
    let mut auth = read(&path)?;
    if auth.remove(id).is_some() {
        write(&path, auth)?;
    }
    if !is_builtin(id) {
        let path = dir.join(MODELS);
        let mut file = read(&path)?;
        let removed = file.get_mut("providers").and_then(Value::as_object_mut).and_then(|providers| providers.remove(id));
        if removed.is_some() {
            write(&path, file)?;
        }
    }
    list(dir)
}

fn is_builtin(id: &str) -> bool {
    BUILTIN_PROVIDERS.iter().any(|(builtin, ..)| *builtin == id)
}

/// Pi reads `$NAME` and a leading `!` in these values as a variable or a command;
/// escaping keeps a pasted key literal.
fn literal(key: &str) -> String {
    let escaped = key.replace('$', "$$");
    if escaped.starts_with('!') { format!("${escaped}") } else { escaped }
}

/// Provider id for an endpoint name: "My LM Studio" -> "my-lm-studio".
fn slug(name: &str) -> String {
    name.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// A JSON object file; missing or blank reads as empty. Anything else that is not
/// an object is refused, so a save never overwrites a file it could not read.
fn read(path: &Path) -> Result<Map<String, Value>, String> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    match fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(Map::new()),
        Ok(text) => match serde_json::from_str(&text) {
            Ok(Value::Object(map)) => Ok(map),
            _ => Err(format!("Pi's {name} could not be read. Fix or remove it, then try again.")),
        },
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Map::new()),
        Err(error) => Err(format!("Pi's {name} could not be read: {error}")),
    }
}

/// Writes through a temporary file, so an interrupted save leaves the old file whole.
fn write(path: &Path, map: Map<String, Value>) -> Result<(), String> {
    let fail = |error: std::io::Error| format!("Pi's config could not be saved: {error}");
    let text = serde_json::to_string_pretty(&Value::Object(map)).map_err(|error| error.to_string())?;
    fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).map_err(fail)?;
    let temp = path.with_extension("json.tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(&temp).and_then(|mut file| file.write_all(text.as_bytes())).map_err(fail)?;
    fs::rename(&temp, path).map_err(fail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_endpoints_round_trip_through_pi_config() {
        // target/ rather than %TEMP%: the host sandbox may deny writes there.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("ps-agent-providers-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(AUTH), r#"{"github-copilot":{"type":"oauth","refresh":"r"}}"#).unwrap();

        save_key(&dir, "openai", " sk-a$b ").unwrap();
        let listed = save_endpoint(&dir, "My LM Studio", "http://localhost:1234/v1", "", &["qwen".into(), " ".into()]).unwrap();
        let auth: Value = serde_json::from_str(&fs::read_to_string(dir.join(AUTH)).unwrap()).unwrap();
        assert_eq!(auth["openai"], json!({ "type": "api_key", "key": "sk-a$$b" }));
        assert_eq!(auth["github-copilot"]["refresh"], "r", "other credentials stay");
        assert!(listed.iter().any(|provider| provider.id == "openai" && provider.saved));
        let endpoint = listed.iter().find(|provider| provider.custom).unwrap();
        assert_eq!((endpoint.id.as_str(), endpoint.models.as_slice()), ("my-lm-studio", ["qwen".to_string()].as_slice()));
        assert_eq!(literal("!cmd"), "$!cmd");

        remove(&dir, "my-lm-studio").unwrap();
        let listed = remove(&dir, "openai").unwrap();
        assert!(listed.iter().all(|provider| !provider.custom && !(provider.id == "openai" && provider.saved)));
        assert!(save_endpoint(&dir, "OpenAI", "https://example.com", "k", &["m".into()]).is_err());
        assert!(save_key(&dir, "openai", " ").is_err());

        fs::write(dir.join(AUTH), "not json").unwrap();
        assert!(save_key(&dir, "groq", "k").is_err());
        assert_eq!(fs::read_to_string(dir.join(AUTH)).unwrap(), "not json", "an unreadable file is never overwritten");
        let _ = fs::remove_dir_all(dir);
    }
}
