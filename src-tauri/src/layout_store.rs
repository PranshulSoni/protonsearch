use serde_json::Value;
use std::path::Path;

pub const FILE: &str = "protondock-layout.json";
pub const BACKUP: &str = "protondock-layout.json.bak";

pub fn load(dir: &Path) -> Result<Value, String> {
    let path = dir.join(FILE);
    if !path.exists() {
        return Ok(Value::Null);
    }
    std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string()))
        .or_else(|error| read_object(&dir.join(BACKUP)).ok_or(error))
}

pub fn save(dir: &Path, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if !value.is_object() || bytes.len() > 1024 * 1024 {
        return Err("Invalid layout".into());
    }
    let path = dir.join(FILE);
    let tmp = path.with_extension("json.tmp");
    if read_object(&path).is_some() {
        std::fs::copy(&path, dir.join(BACKUP))
            .map_err(|_| "Could not back up the dock layout".to_string())?;
    }
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    // SAFETY: both NUL-terminated UTF-16 buffers stay alive for the MoveFileExW call.
    unsafe {
        use windows::{
            core::PCWSTR,
            Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            },
        };
        let from: Vec<u16> = tmp
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let to: Vec<u16> = path
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_object(path: &Path) -> Option<Value> {
    let value = serde_json::from_slice::<Value>(&std::fs::read(path).ok()?).ok()?;
    value.is_object().then_some(value)
}
