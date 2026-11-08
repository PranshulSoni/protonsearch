#[path = "../src/layout_store.rs"]
mod layout_store;

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn save_then_load_round_trips() {
    let dir = fresh("save_then_load_round_trips");
    let value = json!({"panels": [1, 2]});
    layout_store::save(&dir, &value).unwrap();
    assert_eq!(layout_store::load(&dir).unwrap(), value);
}

#[test]
fn missing_file_loads_null() {
    let dir = fresh("missing_file_loads_null");
    assert_eq!(layout_store::load(&dir).unwrap(), Value::Null);
}

#[test]
fn object_and_size_limits_are_enforced() {
    let dir = fresh("object_and_size_limits_are_enforced");
    let array = layout_store::save(&dir, &json!([1, 2])).unwrap_err();
    assert_eq!(array, "Invalid layout");
    // Wrapped in an object so the size limit, not the object check, rejects it.
    let big = json!({"v": "a".repeat(1048577)});
    let oversize = layout_store::save(&dir, &big).unwrap_err();
    assert_eq!(oversize, "Invalid layout");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
}

#[test]
fn previous_good_copy_is_kept() {
    let dir = fresh("previous_good_copy_is_kept");
    let (a, b) = (json!({"v": "A"}), json!({"v": "B"}));
    layout_store::save(&dir, &a).unwrap();
    layout_store::save(&dir, &b).unwrap();
    assert_eq!(read_json(&dir.join(layout_store::BACKUP)), a);
    assert_eq!(read_json(&dir.join(layout_store::FILE)), b);
}

#[test]
fn corrupt_file_falls_back_to_backup() {
    let dir = fresh("corrupt_file_falls_back_to_backup");
    let (a, b) = (json!({"v": "A"}), json!({"v": "B"}));
    layout_store::save(&dir, &a).unwrap();
    layout_store::save(&dir, &b).unwrap();
    std::fs::write(dir.join(layout_store::FILE), "{trunc").unwrap();
    assert_eq!(layout_store::load(&dir).unwrap(), a);
}

#[test]
fn corrupt_file_never_replaces_a_good_backup() {
    let dir = fresh("corrupt_file_never_replaces_a_good_backup");
    let (a, c) = (json!({"v": "A"}), json!({"v": "C"}));
    layout_store::save(&dir, &a).unwrap();
    layout_store::save(&dir, &json!({"v": "B"})).unwrap();
    std::fs::write(dir.join(layout_store::FILE), "{trunc").unwrap();
    layout_store::save(&dir, &c).unwrap();
    assert_eq!(read_json(&dir.join(layout_store::BACKUP)), a);
    assert_eq!(read_json(&dir.join(layout_store::FILE)), c);
}

#[test]
fn both_bad_returns_error_and_modifies_nothing() {
    let dir = fresh("both_bad_returns_error_and_modifies_nothing");
    std::fs::write(dir.join(layout_store::FILE), "{trunc").unwrap();
    std::fs::write(dir.join(layout_store::BACKUP), "{bad").unwrap();
    assert!(layout_store::load(&dir).is_err());
    assert_eq!(
        std::fs::read(dir.join(layout_store::FILE)).unwrap(),
        b"{trunc"
    );
    assert_eq!(
        std::fs::read(dir.join(layout_store::BACKUP)).unwrap(),
        b"{bad"
    );
}
