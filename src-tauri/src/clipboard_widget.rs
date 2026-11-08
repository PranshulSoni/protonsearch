use protonsearch::search::{SearchEngine, SearchResult};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn snapshot(engine: &SearchEngine) -> Value {
    from_results(engine.search_clipboard_history(""))
}

fn from_results(results: Vec<SearchResult>) -> Value {
    let count = results.len();
    let items: Vec<Value> = results.into_iter().take(3).map(|result| {
        let preview: String = result.entry.control_name.split_whitespace().collect::<Vec<_>>()
            .join(" ").chars().take(120).collect();
        json!({"preview":preview,
            "source":result.entry.breadcrumb_path.strip_prefix("Clipboard > ").unwrap_or("Clipboard").chars().take(64).collect::<String>(),
            "pinned":result.entry.id.starts_with("clip.pinned.")})
    }).collect();
    json!({"status":if count == 0 {"empty"} else {"live"},
        "data":{"items":items,"availableCount":count,"countIsLowerBound":count >= 50},
        "updatedAt":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
        "message":if count == 0 {"No saved clipboard history"} else {"Saved ProtonSearch history"}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use protonsearch::search::CatalogEntry;

    fn result(index: usize) -> SearchResult {
        SearchResult {
            entry: CatalogEntry {
                id: format!("clip.pinned.{index}"),
                control_name: "Fixture <b> text\n😀".into(),
                breadcrumb_path: "Clipboard > fixture.exe".into(),
                launch_command: "copy:private fixture content never serialized".into(),
                source: "CLIPBOARD".into(),
                description: String::new(),
                synonyms: String::new(),
            },
            score: 3.0,
        }
    }
    #[test]
    fn bounded_previews_keep_engine_order_and_never_expose_actions() {
        let value = from_results((0..50).map(result).collect());
        assert_eq!(value["status"], "live");
        assert_eq!(value["data"]["items"].as_array().unwrap().len(), 3);
        assert_eq!(value["data"]["items"][0]["preview"], "Fixture <b> text 😀");
        assert_eq!(value["data"]["items"][0]["source"], "fixture.exe");
        assert_eq!(value["data"]["items"][0]["pinned"], true);
        assert_eq!(value["data"]["availableCount"], 50);
        assert_eq!(value["data"]["countIsLowerBound"], true);
        assert!(!value.to_string().contains("copy:"));
        assert!(!value.to_string().contains("private fixture"));
    }
    #[test]
    fn empty_history_is_not_presented_as_live() {
        let value = from_results(Vec::new());
        assert_eq!(value["status"], "empty");
        assert_eq!(value["data"]["availableCount"], 0);
        assert_eq!(value["data"]["countIsLowerBound"], false);
    }
}
