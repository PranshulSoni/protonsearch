#[path = "calendar_ics.rs"]
mod ics;
use serde_json::{json, Value};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
    sync::Mutex,
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HANDLE, HWND},
        Storage::FileSystem::{
            GetDriveTypeW, GetFinalPathNameByHandleW, MoveFileExW, FILE_NAME_NORMALIZED,
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        },
        UI::Shell::{
            Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileOpenDialog, FOS_DONTADDTORECENT,
            FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_PATHMUSTEXIST,
            SIGDN_FILESYSPATH,
        },
    },
};

const MAX_ICS: u64 = 1024 * 1024;
static GRANTS: Mutex<()> = Mutex::new(());
fn profile_id(profile: &str) -> bool {
    let edge = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    (1..=24).contains(&profile.len())
        && profile.starts_with(edge)
        && profile.chars().all(|c| edge(c) || c == '-')
}
fn profile_name(profile: &str) -> Result<(), String> {
    if profile_id(profile) {
        Ok(())
    } else {
        Err("Invalid calendar profile".into())
    }
}
fn valid_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit())
}
fn bounded_json(path: &Path, maximum: u64) -> Result<Option<Value>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot read native calendar configuration".into()),
    };
    let mut bytes = Vec::new();
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read native calendar configuration")?;
    if bytes.len() as u64 > maximum {
        return Err("Native calendar configuration is too large".into());
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "Native calendar configuration is invalid".into())
}
fn profile_is_active(layout: Option<&Value>, profile: &str) -> Result<bool, String> {
    match layout {
        None => Ok(profile == "daily"),
        Some(layout)
            if layout["version"] == 1
                && layout["profile"]
                    .as_str()
                    .is_some_and(|name| ["daily", "founder"].contains(&name)) =>
        {
            Ok(layout["profile"] == profile)
        }
        Some(layout)
            if (layout["version"] == 2 || layout["version"] == 3)
                && layout["profile"].as_str().is_some_and(profile_id)
                && layout["profiles"].is_object() =>
        {
            Ok(layout["profile"] == profile && layout["profiles"].get(profile).is_some())
        }
        _ => Err("Native dock profile could not be validated".into()),
    }
}
fn active_profile(directory: &Path, profile: &str) -> Result<bool, String> {
    profile_name(profile)?;
    let layout = bounded_json(&directory.join("protondock-layout.json"), 1024 * 1024)?;
    profile_is_active(layout.as_ref(), profile)
}
fn grants(directory: &Path) -> Result<Value, String> {
    let value = bounded_json(&directory.join("calendar-sources.json"), 65536)?
        .unwrap_or_else(|| json!({"version":1,"profiles":{}}));
    check_grants(value)
}
fn check_grants(value: Value) -> Result<Value, String> {
    if value["version"] != 1 || !value["profiles"].is_object() {
        return Err("Native calendar grants are invalid; existing grants were preserved".into());
    }
    for profile in value["profiles"].as_object().into_iter().flat_map(|profiles| profiles.keys()) {
        if let Some(grant) = value["profiles"].get(profile) {
            let legacy = grant.get("token").is_some() || grant.get("path").is_some();
            if !profile_id(profile)
                || !grant.is_object()
                || (legacy
                    && (!grant["token"].as_str().is_some_and(valid_token)
                        || !grant["path"].as_str().is_some_and(|path| !path.is_empty())))
                || (!legacy && grant.get("sources").is_none())
                || grant.get("sources").is_some_and(|sources| {
                    sources.as_object().is_none_or(|sources| {
                        sources.iter().any(|(token, path)| {
                            !valid_token(token)
                                || grant["token"] == token.as_str()
                                || !path.as_str().is_some_and(|path| !path.is_empty())
                        })
                    })
                })
            {
                return Err(
                    "Native calendar grants are invalid; existing grants were preserved".into(),
                );
            }
        }
    }
    Ok(value)
}
fn save_grants(directory: &Path, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|_| "Cannot encode calendar grant")?;
    if bytes.len() > 65536 {
        return Err("Native calendar grants are too large".into());
    }
    let temporary = directory.join(format!(
        "calendar-sources-{:032x}.tmp",
        windows::core::GUID::new()
            .map_err(|_| "Cannot create calendar grant ID")?
            .to_u128()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "Cannot save calendar source")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Cannot save calendar source")?;
        drop(file);
        let destination = directory.join("calendar-sources.json");
        let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let target: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // SAFETY: both terminated UTF-16 buffers remain alive throughout the call.
        unsafe {
            MoveFileExW(
                PCWSTR(source.as_ptr()),
                PCWSTR(target.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(|_| "Cannot commit calendar source")
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(String::from)
}
fn local_ics_path(path: &Path) -> Result<(), String> {
    let value = path
        .to_str()
        .ok_or("Calendar path cannot be represented safely")?;
    let value = value.strip_prefix(r"\\?\").unwrap_or(value);
    let bytes = value.as_bytes();
    if bytes.len() < 4
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || bytes[2] != b'\\'
        || value[3..].contains(':')
        || value[3..].contains('/')
    {
        return Err(
            "Select a calendar on a local drive; network and device paths are not supported".into(),
        );
    }
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ics"))
    {
        return Err("Select a local .ics calendar file".into());
    }
    for segment in value[3..].split('\\') {
        let stem = segment
            .split('.')
            .next()
            .unwrap_or("")
            .trim_end_matches(' ')
            .to_ascii_uppercase();
        if segment.is_empty()
            || segment.ends_with([' ', '.'])
            || ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
            || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(
                    &stem[3..],
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                ))
        {
            return Err("Calendar path contains an unsupported Windows filename".into());
        }
    }
    let root: Vec<u16> = value[..3].encode_utf16().chain(Some(0)).collect();
    // SAFETY: root is an owned, terminated UTF-16 drive-root buffer.
    let kind = unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) };
    if ![2, 3, 5, 6].contains(&kind) {
        return Err("Network or unavailable calendar drives are not supported".into());
    }
    Ok(())
}
fn read_calendar(path: &Path) -> Result<(PathBuf, String), String> {
    local_ics_path(path)?;
    // Extended paths disable legacy DOS-device name interpretation before any open.
    let value = path.to_str().ok_or("Calendar filename is unsupported")?;
    let absolute = PathBuf::from(format!(
        r"\\?\{}",
        value.strip_prefix(r"\\?\").unwrap_or(value)
    ));
    let path = absolute.as_path();
    // Hold every local ancestor without write/delete sharing before descending;
    // OPEN_REPARSE_POINT prevents even an accidental remote junction traversal.
    let mut ancestors: Vec<&Path> = path.ancestors().skip(1).collect();
    ancestors.reverse();
    let mut directory_guards = Vec::new();
    for ancestor in ancestors {
        let directory = no_reparse_open(ancestor, true)?;
        directory_guards.push(directory);
    }
    let file = no_reparse_open(path, false)?;
    let metadata = file
        .metadata()
        .map_err(|_| "Cannot inspect the selected calendar")?;
    if !metadata.is_file() || metadata.len() > MAX_ICS {
        return Err("Calendar must be a regular .ics file no larger than 1 MiB".into());
    }
    let mut name = vec![0u16; 32768];
    // Validate the opened handle's resolved DOS path so reparse/rename races cannot authorize a network target.
    // SAFETY: file owns the handle and name is writable for its full declared length.
    let length = unsafe {
        GetFinalPathNameByHandleW(
            HANDLE(file.as_raw_handle()),
            &mut name,
            FILE_NAME_NORMALIZED,
        )
    } as usize;
    if length == 0 || length >= name.len() {
        return Err("Cannot validate the selected calendar location".into());
    }
    let resolved = PathBuf::from(
        String::from_utf16(&name[..length]).map_err(|_| "Calendar filename is unsupported")?,
    );
    local_ics_path(&resolved)?;
    let mut bytes = Vec::new();
    file.take(MAX_ICS + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read the selected calendar")?;
    if bytes.len() as u64 > MAX_ICS {
        return Err("Calendar grew beyond the 1 MiB limit".into());
    }
    let content = String::from_utf8(bytes).map_err(|_| "Calendar must use UTF-8 text")?;
    Ok((resolved, content))
}
fn no_reparse_open(path: &Path, directory: bool) -> Result<File, String> {
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_GENERIC_READ, FILE_SHARE_READ,
    };
    let file = OpenOptions::new()
        .access_mode(FILE_GENERIC_READ.0)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT.0
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS.0
                } else {
                    0
                },
        )
        .open(path)
        .map_err(|_| "Cannot safely open the local calendar location")?;
    let metadata = file
        .metadata()
        .map_err(|_| "Cannot inspect the calendar location")?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        || directory != metadata.is_dir()
    {
        return Err("Calendar sources must not use links or reparse points".into());
    }
    Ok(file)
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: Apartment exists only after successful initialization on this thread.
        unsafe {
            CoUninitialize();
        }
    }
}
fn pick(owner: isize) -> Result<Option<PathBuf>, String> {
    // SAFETY: this is a fresh dedicated thread; Apartment balances successful initialization.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok() }
        .map_err(|_| "Cannot initialize calendar picker")?;
    let _apartment = Apartment;
    let picked = (|| -> windows::core::Result<Option<PathBuf>> {
        // SAFETY: COM objects remain in this initialized STA and drop before Apartment.
        unsafe {
            let dialog: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            dialog.SetOptions(
                FOS_FORCEFILESYSTEM
                    | FOS_FILEMUSTEXIST
                    | FOS_PATHMUSTEXIST
                    | FOS_NOCHANGEDIR
                    | FOS_DONTADDTORECENT,
            )?;
            dialog.SetFileTypes(&[COMDLG_FILTERSPEC {
                pszName: w!("iCalendar (*.ics)"),
                pszSpec: w!("*.ics"),
            }])?;
            dialog.SetTitle(w!("Choose a local one-off-event calendar"))?;
            match dialog.Show(HWND(owner as *mut std::ffi::c_void)) {
                Err(error) if error.code().0 as u32 == 0x800704C7 => return Ok(None),
                result => result?,
            }
            let value = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
            let path = value.to_string();
            CoTaskMemFree(Some(value.0.cast()));
            Ok(Some(PathBuf::from(path?)))
        }
    })();
    picked.map_err(|_| "Windows could not select the calendar file".into())
}
pub fn select_source(
    directory: PathBuf,
    profile: String,
    owner: isize,
) -> Result<Option<Value>, String> {
    if !active_profile(&directory, &profile)? {
        return Err("Calendar profile changed; select the source again".into());
    }
    // A dedicated thread guarantees an STA rather than reusing media's MTA worker.
    let path = std::thread::spawn(move || pick(owner))
        .join()
        .map_err(|_| "Calendar picker stopped unexpectedly")??;
    let Some(path) = path else {
        return Ok(None);
    };
    let (path, content) = read_calendar(&path)?;
    ics::parse(&content, chrono::Utc::now().timestamp_millis())?;
    let _lock = GRANTS.lock().map_err(|_| "Calendar source is busy")?;
    if !active_profile(&directory, &profile)? {
        return Err("Calendar profile changed; selected source was not saved".into());
    }
    let token = format!(
        "{:032x}",
        windows::core::GUID::new()
            .map_err(|_| "Cannot create calendar source token")?
            .to_u128()
    );
    let name: String = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Calendar.ics")
        .chars()
        .take(80)
        .collect();
    retain_source(&directory, &profile, &token, &path)?;
    Ok(Some(json!({"token":token,"name":name,"profile":profile})))
}
fn retain_source(directory: &Path, profile: &str, token: &str, path: &Path) -> Result<(), String> {
    let mut config = grants(directory)?;
    if config["profiles"].get(profile).is_none() {
        config["profiles"][profile] = json!({"sources":{}});
    }
    if config["profiles"][profile].get("sources").is_none() {
        config["profiles"][profile]["sources"] = json!({});
    }
    // Keep earlier grants valid if the picker result is discarded or layout saving fails.
    config["profiles"][profile]["sources"][token] =
        json!(path.to_str().ok_or("Calendar path is unsupported")?);
    save_grants(directory, &config)
}
pub fn snapshot(directory: &Path, profile: &str, token: &str) -> Result<Value, String> {
    profile_name(profile)?;
    if !valid_token(token) {
        return Err("Invalid calendar source token".into());
    }
    let _lock = GRANTS.lock().map_err(|_| "Calendar source is busy")?;
    if !active_profile(directory, profile)? {
        return Ok(denied());
    }
    let config = grants(directory)?;
    let grant = &config["profiles"][profile];
    let path = if grant["token"] == token {
        grant["path"].as_str()
    } else {
        grant["sources"][token].as_str()
    };
    let Some(path) = path else {
        return Ok(denied());
    };
    let (path, content) = read_calendar(Path::new(path))?;
    let now = chrono::Utc::now().timestamp_millis();
    let events = ics::parse(&content, now)?;
    if !active_profile(directory, profile)? {
        return Ok(denied());
    }
    let name: String = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Calendar.ics")
        .chars()
        .take(80)
        .collect();
    Ok(
        json!({"status":if events.is_empty() {"empty"} else {"live"},"data":{"sourceName":name,"events":events},"updatedAt":now,
        "message":if events.is_empty() {"No upcoming one-off events"} else {""}}),
    )
}
fn denied() -> Value {
    json!({"status":"permission-denied","data":null,"updatedAt":chrono::Utc::now().timestamp_millis(),"message":"Select a calendar source for the active profile"})
}
pub fn forget_source(directory: &Path, profile: &str, token: &str) -> Result<(), String> {
    profile_name(profile)?;
    if !valid_token(token) {
        return Err("Invalid calendar source token".into());
    }
    let _lock = GRANTS.lock().map_err(|_| "Calendar source is busy")?;
    if !active_profile(directory, profile)? {
        return Err("Calendar profile changed; source was not removed".into());
    }
    let mut config = grants(directory)?;
    let grant = &mut config["profiles"][profile];
    if grant["token"] == token {
        let object = grant.as_object_mut().ok_or("Calendar grants are invalid")?;
        object.remove("token");
        object.remove("path");
    } else if let Some(sources) = grant["sources"].as_object_mut() {
        if sources.remove(token).is_none() {
            return Err("Calendar source is not authorized for this profile".into());
        }
    } else {
        return Err("Calendar source is not authorized for this profile".into());
    }
    if grant.get("token").is_none()
        && grant.get("sources").is_none_or(|sources| {
            sources
                .as_object()
                .is_some_and(|sources| sources.is_empty())
        })
    {
        config["profiles"]
            .as_object_mut()
            .ok_or("Calendar grants are invalid")?
            .remove(profile);
    }
    save_grants(directory, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "protondock-calendar-fixture-{:032x}",
                windows::core::GUID::new().unwrap().to_u128()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn calendar(&self) -> PathBuf {
            let path = self.0.join("fixture.ics");
            std::fs::write(&path, "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nDTSTART:20991105T120000Z\nSUMMARY:Fixture event\nEND:VEVENT\nEND:VCALENDAR").unwrap();
            path
        }
        fn grant(&self, path: &Path) -> String {
            let token = "0123456789abcdef0123456789abcdef".to_string();
            save_grants(&self.0, &json!({"version":1,"profiles":{"daily":{"token":token,"path":path}} ,"future":{"preserve":true}})).unwrap();
            token
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn local_fixture_is_bounded_and_profile_token_scoped() {
        let fixture = Fixture::new();
        let path = fixture.calendar();
        let token = fixture.grant(&path);
        let value = snapshot(&fixture.0, "daily", &token).unwrap();
        assert_eq!(value["status"], "live");
        assert_eq!(value["data"]["events"][0]["title"], "Fixture event");
        assert!(!value.to_string().contains(fixture.0.to_str().unwrap()));
        assert_eq!(
            snapshot(&fixture.0, "founder", &token).unwrap()["status"],
            "permission-denied"
        );
        assert_eq!(
            snapshot(&fixture.0, "daily", "ffffffffffffffffffffffffffffffff").unwrap()["status"],
            "permission-denied"
        );
        assert!(forget_source(&fixture.0, "founder", &token).is_err());
        forget_source(&fixture.0, "daily", &token).unwrap();
        let saved = grants(&fixture.0).unwrap();
        assert_eq!(saved["future"]["preserve"], true);
        assert!(saved["profiles"].get("daily").is_none());
        std::fs::write(&path, vec![b'x'; MAX_ICS as usize + 1]).unwrap();
        assert!(read_calendar(&path).unwrap_err().contains("1 MiB"));
        std::fs::write(&path, [0xff, 0xfe]).unwrap();
        assert!(read_calendar(&path).unwrap_err().contains("UTF-8"));
    }
    #[test]
    fn corrupt_grants_and_other_profiles_are_preserved() {
        let fixture = Fixture::new();
        let path = fixture.0.join("calendar-sources.json");
        for content in [
            "not json",
            r#"{"version":1,"profiles":{"daily":{"token":"bad","path":42}}}"#,
            r#"{"version":1,"profiles":{"daily":{"sources":{"bad":"file.ics"}}}}"#,
            r#"{"version":1,"profiles":{"daily":{"sources":[]}}}"#,
            r#"{"version":1,"profiles":{"daily":{"sources":{"ffffffffffffffffffffffffffffffff":42}}}}"#,
            r#"{"version":1,"profiles":{"daily":{"token":"ffffffffffffffffffffffffffffffff","path":"A.ics","sources":{"ffffffffffffffffffffffffffffffff":"B.ics"}}}}"#,
        ] {
            std::fs::write(&path, content).unwrap();
            assert!(grants(&fixture.0).is_err());
            assert!(retain_source(
                &fixture.0,
                "daily",
                "0123456789abcdef0123456789abcdef",
                Path::new(r"C:\fixture.ics")
            )
            .is_err());
            assert!(
                forget_source(&fixture.0, "daily", "0123456789abcdef0123456789abcdef").is_err()
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        }
    }
    #[test]
    fn discarded_selection_preserves_existing_token_and_forget_is_exact() {
        let fixture = Fixture::new();
        let path = fixture.calendar();
        let old = fixture.grant(&path);
        let selected = "ffffffffffffffffffffffffffffffff";
        retain_source(&fixture.0, "daily", selected, &path).unwrap();
        assert_eq!(
            snapshot(&fixture.0, "daily", &old).unwrap()["status"],
            "live"
        );
        assert_eq!(
            snapshot(&fixture.0, "daily", selected).unwrap()["status"],
            "live"
        );
        forget_source(&fixture.0, "daily", selected).unwrap();
        assert_eq!(
            snapshot(&fixture.0, "daily", &old).unwrap()["status"],
            "live"
        );
        assert_eq!(
            snapshot(&fixture.0, "daily", selected).unwrap()["status"],
            "permission-denied"
        );
    }
    #[test]
    fn rejects_network_devices_and_ambiguous_names_without_opening_them() {
        for path in [
            r"\\server\share\calendar.ics",
            r"\\?\UNC\server\share\calendar.ics",
            r"\\.\C:\calendar.ics",
            r"C:calendar.ics",
            r"C:\safe\NUL.ics",
            r"C:\safe\COM1.ics",
            r"C:\safe\..\calendar.ics",
            r"C:\safe.\calendar.ics",
            r"C:\safe\calendar.ics:stream",
        ] {
            assert!(local_ics_path(Path::new(path)).is_err(), "{path}");
        }
    }
    #[test]
    fn ancestor_handles_block_replacement_until_read_finishes() {
        let fixture = Fixture::new();
        let directory = fixture.0.join("parent");
        std::fs::create_dir(&directory).unwrap();
        let guard = no_reparse_open(&directory, true).unwrap();
        assert!(std::fs::rename(&directory, fixture.0.join("moved")).is_err());
        drop(guard);
        std::fs::rename(&directory, fixture.0.join("moved")).unwrap();
    }
    #[test]
    fn local_junction_ancestor_is_rejected_without_following_it() {
        let fixture = Fixture::new();
        let target = fixture.0.join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("fixture.ics"), "must not read through junction").unwrap();
        let link = fixture.0.join("link");
        let status = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "local fixture junction creation failed"
        );
        let result = read_calendar(&link.join("fixture.ics"));
        std::fs::remove_dir(&link).unwrap();
        assert!(result.unwrap_err().contains("reparse"));
    }
}
#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn version_two_layout_activates_its_profile() {
        let layout = json!({"version":2,"profile":"work","profiles":{"work":{}}});
        assert_eq!(profile_is_active(Some(&layout), "work"), Ok(true));
    }
    #[test]
    fn version_three_custom_profile_is_valid() {
        let layout = json!({"version":3,"profile":"deep-work","profiles":{"deep-work":{}}});
        assert_eq!(profile_is_active(Some(&layout), "deep-work"), Ok(true));
    }
    #[test]
    fn inactive_profile_is_denied_not_an_error() {
        let layout = json!({"version":3,"profile":"deep-work","profiles":{"deep-work":{},"daily":{}}});
        assert_eq!(profile_is_active(Some(&layout), "daily"), Ok(false));
    }
    #[test]
    fn unknown_layout_version_is_rejected() {
        let version_four = json!({"version":4,"profile":"daily","profiles":{"daily":{}}});
        assert!(profile_is_active(Some(&version_four), "daily").is_err());
        let bad_name = json!({"version":2,"profile":"Bad Name","profiles":{"Bad Name":{}}});
        assert!(profile_is_active(Some(&bad_name), "daily").is_err());
    }
    #[test]
    fn missing_layout_keeps_daily_active() {
        assert_eq!(profile_is_active(None, "daily"), Ok(true));
        assert_eq!(profile_is_active(None, "founder"), Ok(false));
    }
    #[test]
    fn grants_for_custom_profiles_are_validated() {
        let sources = json!({"0123456789abcdef0123456789abcdef": "a.ics"});
        assert!(check_grants(json!({"version":1,"profiles":{"deep-work":{"sources":sources}}})).is_ok());
        assert!(check_grants(json!({"version":1,"profiles":{"Bad Key":{"sources":sources}}})).is_err());
        assert!(check_grants(json!({"version":1,"profiles":{"deep-work":{"token":"bad","path":42}}})).is_err());
    }
}
