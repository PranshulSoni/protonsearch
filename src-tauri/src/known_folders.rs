use crate::widget_data::envelope;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    ffi::c_void,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use windows::{
    core::GUID,
    Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT,
        System::Com::CoTaskMemFree,
        UI::Shell::{
            FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Pictures,
            FOLDERID_Profile, FOLDERID_Screenshots, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
        },
    },
};

const MAX_RECENT: usize = 12;
const RECENT_SCAN_DEPTH: usize = 2;
const MAX_ENTRIES_PER_ROOT: usize = 2_000;
const STANDARD_FOLDERS: [(&str, &str, &str, GUID); 5] = [
    ("desktop", "Desktop", "desktop", FOLDERID_Desktop),
    ("documents", "Documents", "documents", FOLDERID_Documents),
    ("downloads", "Downloads", "downloads", FOLDERID_Downloads),
    ("pictures", "Pictures", "pictures", FOLDERID_Pictures),
    (
        "screenshots",
        "Screenshots",
        "screenshots",
        FOLDERID_Screenshots,
    ),
];

enum KnownFolderError {
    Denied,
    Other,
}
struct FolderRecord {
    id: &'static str,
    name: &'static str,
    icon: &'static str,
    path: PathBuf,
}
struct RecentFile {
    name: String,
    path: String,
    kind: &'static str,
    modified: SystemTime,
}

pub fn snapshot() -> Value {
    let mut folders = Vec::new();
    let mut seen_folders = HashSet::new();
    let mut denied = false;
    for (id, name, icon, guid) in STANDARD_FOLDERS {
        match known_folder_path(&guid) {
            Ok(path) => push_folder(&mut folders, &mut seen_folders, id, name, icon, path),
            Err(KnownFolderError::Denied) => denied = true,
            Err(KnownFolderError::Other) => {}
        }
    }
    if let Some(documents) = folders
        .iter()
        .find(|folder| folder.id == "documents")
        .map(|folder| folder.path.join("Projects"))
    {
        push_folder(
            &mut folders,
            &mut seen_folders,
            "projects",
            "Projects",
            "folder",
            documents,
        );
    }
    if !folders.iter().any(|folder| folder.id == "projects") {
        if let Ok(profile) = known_folder_path(&FOLDERID_Profile) {
            push_folder(
                &mut folders,
                &mut seen_folders,
                "projects",
                "Projects",
                "folder",
                profile.join("Projects"),
            );
        }
    }
    let recent = collect_recent(&folders);
    let status = if !folders.is_empty() {
        "live"
    } else if denied {
        "permission-denied"
    } else {
        "empty"
    };
    let message = match status {
        "live" => "",
        "permission-denied" => "Windows denied access to known folders",
        _ => "No known folders are available",
    };
    let folders: Vec<Value> = folders
        .into_iter()
        .map(|folder| {
            json!({
                "id":folder.id,
                "name":folder.name,
                "path":folder.path.to_string_lossy(),
                "icon":folder.icon
            })
        })
        .collect();
    envelope(status, json!({"folders":folders,"recent":recent}), message)
}

fn push_folder(
    folders: &mut Vec<FolderRecord>,
    seen: &mut HashSet<String>,
    id: &'static str,
    name: &'static str,
    icon: &'static str,
    path: PathBuf,
) {
    if !is_local_absolute_path(&path) || !path.is_dir() || path.to_str().is_none() {
        return;
    }
    let key = path.to_string_lossy().to_ascii_lowercase();
    if seen.insert(key) {
        folders.push(FolderRecord {
            id,
            name,
            icon,
            path,
        });
    }
}
fn known_folder_path(guid: &GUID) -> Result<PathBuf, KnownFolderError> {
    let path = unsafe { SHGetKnownFolderPath(guid, KF_FLAG_DEFAULT, HANDLE::default()) }.map_err(
        |error| {
            if error.code().0 as u32 == 0x80070005 {
                KnownFolderError::Denied
            } else {
                KnownFolderError::Other
            }
        },
    )?;
    // SHGetKnownFolderPath allocates with CoTaskMemAlloc; copy then free exactly once.
    let text = unsafe { path.to_string() };
    unsafe { CoTaskMemFree(Some(path.0 as *const c_void)) };
    text.map(PathBuf::from).map_err(|_| KnownFolderError::Other)
}
pub(crate) fn is_local_absolute_path(path: &Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    let bytes = text.as_bytes();
    bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}
pub(crate) fn folder_path(id: &str) -> Option<PathBuf> {
    if id == "projects" {
        if let Ok(documents) = known_folder_path(&FOLDERID_Documents) {
            let path = documents.join("Projects");
            if valid_folder(&path) {
                return Some(path);
            }
        }
        let path = known_folder_path(&FOLDERID_Profile).ok()?.join("Projects");
        return valid_folder(&path).then_some(path);
    }
    STANDARD_FOLDERS
        .iter()
        .find(|folder| folder.0 == id)
        .and_then(|folder| known_folder_path(&folder.3).ok())
        .filter(|path| valid_folder(path))
}
fn valid_folder(path: &Path) -> bool {
    is_local_absolute_path(path) && path.is_dir() && path.to_str().is_some()
}
fn collect_recent(folders: &[FolderRecord]) -> Vec<Value> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for folder in folders {
        let Ok(root) = std::fs::symlink_metadata(&folder.path) else {
            continue;
        };
        if root.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            continue;
        }
        let mut budget = MAX_ENTRIES_PER_ROOT;
        scan_folder(
            &folder.path,
            RECENT_SCAN_DEPTH,
            &mut budget,
            &mut seen,
            &mut files,
        );
    }
    files.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    files.truncate(MAX_RECENT);
    files
        .into_iter()
        .map(|file| {
            json!({
                "name":file.name,
                "path":file.path,
                "kind":file.kind,
                "modifiedAt":epoch_ms(file.modified)
            })
        })
        .collect()
}
fn scan_folder(
    directory: &Path,
    depth: usize,
    budget: &mut usize,
    seen: &mut HashSet<String>,
    files: &mut Vec<RecentFile>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let path = entry.path();
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            continue;
        }
        if metadata.is_dir() {
            if depth > 1 {
                scan_folder(&path, depth - 1, budget, seen, files);
            }
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(path_text) = path.to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if seen.insert(path_text.to_ascii_lowercase()) {
            files.push(RecentFile {
                name,
                path: path_text,
                kind: file_kind(&path),
                modified,
            });
        }
    }
}
fn file_kind(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "heic" | "tif" | "tiff" => {
            "image"
        }
        "pdf" => "pdf",
        "doc" | "docx" | "txt" | "md" | "rtf" | "odt" | "pages" => "document",
        "xls" | "xlsx" | "csv" | "ods" | "numbers" => "spreadsheet",
        "ppt" | "pptx" | "odp" | "key" => "presentation",
        "mp4" | "mov" | "mkv" | "avi" | "webm" => "video",
        "mp3" | "wav" | "flac" | "m4a" | "ogg" => "audio",
        "zip" | "7z" | "rar" | "tar" | "gz" => "archive",
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "c" | "cpp" | "h" | "cs"
        | "json" | "toml" | "yaml" | "yml" | "html" | "css" | "sh" | "ps1" => "code",
        _ => "other",
    }
}
fn epoch_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_local_absolute_drive_paths_are_accepted() {
        assert!(is_local_absolute_path(Path::new(r"C:\Users\Public")));
        assert!(is_local_absolute_path(Path::new("D:/Projects")));
        assert!(!is_local_absolute_path(Path::new(r"\\server\share")));
        assert!(!is_local_absolute_path(Path::new("//server/share")));
        assert!(!is_local_absolute_path(Path::new(r"C:relative")));
        assert!(!is_local_absolute_path(Path::new(r"relative\path")));
        assert!(!is_local_absolute_path(Path::new(r"C:\")));
    }
    #[test]
    fn folder_open_allowlist_rejects_paths_and_unknown_ids() {
        for id in [
            "",
            "..",
            "../Windows",
            r"C:\Windows",
            "desktop/../..",
            r"documents\..",
            "shell:startup",
        ] {
            assert!(folder_path(id).is_none(), "unexpected folder for {id}");
        }
    }
    #[test]
    fn known_folder_ids_resolve_only_to_existing_local_directories() {
        for id in [
            "desktop",
            "documents",
            "downloads",
            "pictures",
            "screenshots",
            "projects",
        ] {
            if let Some(path) = folder_path(id) {
                assert!(is_local_absolute_path(&path), "{id} is not local");
                assert!(path.is_dir(), "{id} is not a directory");
                assert!(path.to_str().is_some(), "{id} is not Unicode");
            }
        }
    }
    #[test]
    fn recent_file_kinds_cover_the_widget_groups() {
        assert_eq!(file_kind(Path::new("photo.JPEG")), "image");
        assert_eq!(file_kind(Path::new("report.pdf")), "pdf");
        assert_eq!(file_kind(Path::new("notes.md")), "document");
        assert_eq!(file_kind(Path::new("budget.xlsx")), "spreadsheet");
        assert_eq!(file_kind(Path::new("deck.pptx")), "presentation");
        assert_eq!(file_kind(Path::new("clip.mp4")), "video");
        assert_eq!(file_kind(Path::new("track.flac")), "audio");
        assert_eq!(file_kind(Path::new("bundle.zip")), "archive");
        assert_eq!(file_kind(Path::new("main.rs")), "code");
        assert_eq!(file_kind(Path::new("README")), "other");
    }
    #[test]
    fn epoch_timestamps_are_epoch_milliseconds() {
        assert_eq!(
            epoch_ms(UNIX_EPOCH + std::time::Duration::from_millis(1_700_000_000_123)),
            1_700_000_000_123
        );
        assert_eq!(epoch_ms(UNIX_EPOCH), 0);
    }
}
