#[allow(dead_code)] // Picker is intentionally never invoked by fixture tests.
#[path = "../src/local_calendar.rs"]
mod local_calendar;
#[path = "../src/layout_store.rs"]
mod layout_store;

use serde_json::json;

#[test]
fn calendar_profile_comes_from_the_backup_when_the_layout_file_is_corrupt() {
    const TOKEN: &str = "0123456789abcdef0123456789abcdef";
    const INACTIVE: &str = "Calendar profile changed; source was not removed";
    const ACTIVE: &str = "Calendar source is not authorized for this profile";
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("calendar_profile_comes_from_the_backup");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // FILE will say daily; the previous good copy says founder.
    layout_store::save(&dir, &json!({"version":1,"profile":"founder"})).unwrap();
    layout_store::save(&dir, &json!({"version":1,"profile":"daily"})).unwrap();
    let layout = || layout_store::load(&dir);
    let forget =
        |profile: &str| local_calendar::forget_source(&dir, profile, TOKEN, &layout).unwrap_err();
    assert_eq!(forget("daily"), ACTIVE);
    assert_eq!(forget("founder"), INACTIVE);
    std::fs::write(dir.join(layout_store::FILE), "{trunc").unwrap();
    assert_eq!(forget("daily"), INACTIVE);
    assert_eq!(forget("founder"), ACTIVE);
}
