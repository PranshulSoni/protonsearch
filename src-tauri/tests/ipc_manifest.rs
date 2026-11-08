// Keeps build.rs (app manifest), src/main.rs (generate_handler!) and permissions/commands.toml in step.
use std::collections::BTreeSet;
use std::fs;

fn between<'a>(text: &'a str, open: &str, close: &str) -> &'a str {
    let start = text.find(open).expect(open) + open.len();
    let end = start + text[start..].find(close).expect(close);
    &text[start..end]
}

// Odd-indexed pieces of a split on '"' are the quoted strings.
fn quoted(text: &str) -> Vec<String> {
    text.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

fn handler_names() -> Vec<String> {
    let main_rs = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs")).unwrap();
    between(&main_rs, "tauri::generate_handler![", "]")
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn manifest_names() -> Vec<String> {
    let build_rs = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs")).unwrap();
    quoted(between(&build_rs, ".commands(&[", "]"))
}

// Each commands.allow line belongs to the most recent identifier line.
fn permission_sets() -> Vec<(String, Vec<String>)> {
    let toml = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/permissions/commands.toml"
    ))
    .unwrap();
    let mut id = String::new();
    let mut sets = Vec::new();
    for line in toml.lines() {
        if let Some(rest) = line.strip_prefix("identifier = ") {
            id = quoted(rest).remove(0);
        } else if line.starts_with("commands.allow = [") {
            sets.push((id.clone(), quoted(line)));
        }
    }
    sets
}

#[test]
fn every_handler_is_in_the_manifest() {
    let mut handlers = handler_names();
    let mut manifest = manifest_names();
    handlers.sort();
    manifest.sort();
    assert_eq!(handlers, manifest);
    assert_eq!(
        manifest.iter().collect::<BTreeSet<_>>().len(),
        manifest.len(),
        "duplicate manifest command"
    );
}

#[test]
fn every_manifest_command_is_allowed_by_some_set() {
    let allowed: BTreeSet<String> = permission_sets()
        .into_iter()
        .flat_map(|(_, commands)| commands)
        .collect();
    for command in manifest_names() {
        assert!(allowed.contains(&command), "{command} is in no permission set");
    }
}

#[test]
fn no_set_allows_an_unknown_command() {
    let manifest: BTreeSet<String> = manifest_names().into_iter().collect();
    for (id, commands) in permission_sets() {
        for command in commands {
            assert!(manifest.contains(&command), "{id} allows unknown command {command}");
        }
    }
}
