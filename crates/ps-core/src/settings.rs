//! Settings persistence and validation. OWNER: engine-core.

use ps_ipc::{Settings, SettingsPatch};
use std::path::Path;

pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn apply_patch(settings: &mut Settings, patch: &SettingsPatch) -> bool {
    settings.apply(patch)
}
