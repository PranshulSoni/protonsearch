//! Well-known v2 data paths. OWNER: engine-core.
//! v2 owns %APPDATA%/protonsearch/v2 exactly so legacy data is never mutated.

use std::path::PathBuf;

pub fn app_data_dir() -> PathBuf {
    // Portable/testing override: all v2 data lives under this directory when
    // PROTONSEARCH_DATA_DIR is set (also used by the E2E harness and portable
    // installs). Default is the per-user Roaming profile.
    if let Some(dir) = std::env::var_os("PROTONSEARCH_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    match std::env::var_os("APPDATA") {
        Some(base) => PathBuf::from(base).join("protonsearch"),
        None => PathBuf::from("."),
    }
}

pub fn legacy_data_dir() -> PathBuf {
    app_data_dir()
}

pub fn v2_data_dir() -> PathBuf {
    app_data_dir().join("v2")
}

pub fn index_db_path() -> PathBuf {
    v2_data_dir().join("index.db")
}

pub fn settings_path() -> PathBuf {
    v2_data_dir().join("settings.json")
}

pub fn cache_dir() -> PathBuf {
    v2_data_dir().join("cache")
}

pub fn icon_cache_dir() -> PathBuf {
    cache_dir().join("icons")
}

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(v2_data_dir())?;
    std::fs::create_dir_all(icon_cache_dir())
}
