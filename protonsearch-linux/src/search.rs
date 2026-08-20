use crate::xdg::{is_default_ignored_path, is_hidden, is_mandatory_system_path, XdgPaths};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use walkdir::{DirEntry, WalkDir};

const MAX_CONTENT_CANDIDATES: usize = 256;
const MAX_PDF_BYTES: u64 = 32 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct FileResult {
    pub path: PathBuf,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentResult {
    pub path: PathBuf,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub include_hidden: bool,
    pub max_results: usize,
    pub max_entries: usize,
    pub max_depth: usize,
    pub extra_roots: Vec<PathBuf>,
    pub ignored_names: Vec<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            include_hidden: false,
            max_results: 100,
            max_entries: 50_000,
            max_depth: 32,
            extra_roots: Vec::new(),
            ignored_names: Vec::new(),
        }
    }
}

pub fn search_files(paths: &XdgPaths, query: &str, options: &SearchOptions) -> Vec<FileResult> {
    search_files_filtered(paths, query, options, None)
}

pub fn search_files_filtered(
    paths: &XdgPaths,
    query: &str,
    options: &SearchOptions,
    kind_filter: Option<&str>,
) -> Vec<FileResult> {
    let query = query.trim().to_lowercase();
    let roots = paths.search_roots(&options.extra_roots);
    let ignored = options
        .ignored_names
        .iter()
        .map(|name| name.to_lowercase())
        .collect::<Vec<_>>();
    let mut results = Vec::new();
    let mut visited_entries = 0;

    for entry in fair_walker(roots, options, &ignored) {
        if results.len() >= options.max_results || visited_entries >= options.max_entries {
            break;
        }
        if entry.depth() == 0 {
            continue;
        }
        visited_entries += 1;
        if entry.path().is_symlink() || is_mandatory_system_path(entry.path()) {
            continue;
        }
        let Some(name) = entry.file_name().to_str() else {
            continue;
        };
        let kind = if entry.file_type().is_dir() {
            "directory"
        } else if entry.file_type().is_file() {
            "file"
        } else {
            // Sockets, devices, and other special entries are neither files
            // nor folders in the launcher and must not be grouped as files.
            continue;
        };
        if (query.is_empty() || name.to_lowercase().contains(&query))
            && kind_filter.is_none_or(|expected| expected == kind)
        {
            results.push(FileResult {
                path: entry.path().to_path_buf(),
                name: name.to_string(),
                kind: kind.to_string(),
            });
        }
    }
    results
}

/// Enumerate image files for the Images category. An empty query intentionally
/// returns the newest traversal results so the category is useful immediately,
/// while a non-empty query matches the filename. Traversal remains bounded by
/// the same limits as regular file search.
pub fn search_images(paths: &XdgPaths, query: &str, options: &SearchOptions) -> Vec<FileResult> {
    let query = query.trim().to_lowercase();
    let roots = paths.search_roots(&options.extra_roots);
    let ignored = options
        .ignored_names
        .iter()
        .map(|name| name.to_lowercase())
        .collect::<Vec<_>>();
    let mut results = Vec::new();
    let mut visited_entries = 0;

    for entry in fair_walker(roots, options, &ignored) {
        if results.len() >= options.max_results || visited_entries >= options.max_entries {
            break;
        }
        if entry.depth() == 0 {
            continue;
        }
        visited_entries += 1;
        if !entry.file_type().is_file() || entry.path().is_symlink() {
            continue;
        }
        let Some(name) = entry.file_name().to_str() else {
            continue;
        };
        if !is_image_path(entry.path())
            || (!query.is_empty() && !name.to_lowercase().contains(&query))
        {
            continue;
        }
        results.push(FileResult {
            path: entry.path().to_path_buf(),
            name: name.to_string(),
            kind: "image".to_string(),
        });
    }
    results
}

/// Search bounded text/document content and optional image OCR. This is an
/// on-demand provider rather than a daemon-backed index, so it remains usable
/// on a fresh Arch install without a database migration.
pub fn search_content(
    paths: &XdgPaths,
    query: &str,
    options: &SearchOptions,
) -> Vec<ContentResult> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let roots = paths.search_roots(&options.extra_roots);
    let ignored = options
        .ignored_names
        .iter()
        .map(|name| name.to_lowercase())
        .collect::<Vec<_>>();
    let mut results = Vec::new();
    let mut visited_entries = 0;
    let mut content_candidates = 0;
    let pdf_available = crate::system::command_available("pdftotext");
    let ocr_available = crate::system::command_available("tesseract");
    for entry in fair_walker(roots, options, &ignored) {
        if results.len() >= options.max_results
            || visited_entries >= options.max_entries
            || content_candidates >= MAX_CONTENT_CANDIDATES
        {
            break;
        }
        if entry.depth() == 0 {
            continue;
        }
        visited_entries += 1;
        if !entry.file_type().is_file() || entry.path().is_symlink() {
            continue;
        }
        let Some(extension) = entry.path().extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        let extension = extension.to_ascii_lowercase();
        let kind = if matches!(
            extension.as_str(),
            "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff" | "avif"
        ) {
            "ocr"
        } else if extension == "pdf" {
            "pdf"
        } else if is_text_extension(&extension) {
            "content"
        } else {
            continue;
        };
        content_candidates += 1;
        let Some(text) = extract_searchable_content(
            entry.path(),
            &extension,
            kind,
            pdf_available,
            ocr_available,
        ) else {
            continue;
        };
        if text.to_lowercase().contains(&query) {
            let Some(name) = entry.file_name().to_str() else {
                continue;
            };
            results.push(ContentResult {
                path: entry.path().to_path_buf(),
                name: name.to_string(),
                kind: kind.to_string(),
            });
        }
    }
    results
}

type EntryWalker = Box<dyn Iterator<Item = DirEntry>>;

struct FairWalker {
    walkers: VecDeque<EntryWalker>,
}

impl Iterator for FairWalker {
    type Item = DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut walker = self.walkers.pop_front()?;
            if let Some(entry) = walker.next() {
                self.walkers.push_back(walker);
                return Some(entry);
            }
        }
    }
}

fn fair_walker(roots: Vec<PathBuf>, options: &SearchOptions, ignored: &[String]) -> FairWalker {
    let seen_directories = Rc::new(RefCell::new(HashSet::new()));
    let include_hidden = options.include_hidden;
    let max_depth = options.max_depth;
    let mut walkers = VecDeque::new();

    for root in roots {
        if root_is_mandatory_system_path(&root) {
            continue;
        }
        let ignored = ignored.to_vec();
        let seen_directories = Rc::clone(&seen_directories);
        let walker = WalkDir::new(root)
            .follow_links(false)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(move |entry| {
                should_descend(entry, include_hidden, &ignored)
                    && (!entry.file_type().is_dir()
                        || seen_directories
                            .borrow_mut()
                            .insert(entry.path().to_path_buf()))
            })
            .filter_map(Result::ok);
        walkers.push_back(Box::new(walker) as EntryWalker);
    }

    FairWalker { walkers }
}

fn root_is_mandatory_system_path(root: &std::path::Path) -> bool {
    is_mandatory_system_path(root)
        || std::fs::canonicalize(root)
            .ok()
            .is_some_and(|path| is_mandatory_system_path(&path))
}

fn is_text_extension(extension: &str) -> bool {
    matches!(
        extension,
        "txt"
            | "md"
            | "rs"
            | "py"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "json"
            | "html"
            | "css"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "cs"
            | "go"
            | "java"
            | "kt"
            | "sh"
            | "yaml"
            | "yml"
            | "toml"
            | "ini"
            | "sql"
            | "xml"
            | "rb"
            | "php"
            | "lua"
            | "swift"
            | "dart"
            | "vue"
            | "svelte"
            | "csv"
            | "tex"
            | "rst"
            | "adoc"
            | "conf"
            | "env"
    )
}

pub fn is_image_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff" | "avif"
            )
        })
}

fn extract_searchable_content(
    path: &std::path::Path,
    extension: &str,
    kind: &str,
    pdf_available: bool,
    ocr_available: bool,
) -> Option<String> {
    if kind == "content" {
        let metadata = std::fs::metadata(path).ok()?;
        if metadata.len() > 2 * 1024 * 1024 {
            return None;
        }
        return std::fs::read_to_string(path).ok();
    }
    if kind == "pdf" && pdf_available {
        if std::fs::metadata(path)
            .ok()
            .is_none_or(|metadata| metadata.len() > MAX_PDF_BYTES)
        {
            return None;
        }
        let path = path.to_string_lossy().into_owned();
        let output =
            crate::system::run("pdftotext", &["-f", "1", "-l", "5", path.as_str(), "-"]).ok()?;
        if output.timed_out || output.status != Some(0) {
            return None;
        }
        return Some(output.stdout);
    }
    if matches!(
        extension,
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff" | "avif"
    ) && ocr_available
    {
        if std::fs::metadata(path)
            .ok()
            .is_none_or(|metadata| metadata.len() > MAX_IMAGE_BYTES)
        {
            return None;
        }
        let path = path.to_string_lossy().into_owned();
        let output =
            crate::system::run("tesseract", &[path.as_str(), "stdout", "--dpi", "150"]).ok()?;
        if output.timed_out || output.status != Some(0) {
            return None;
        }
        return Some(output.stdout);
    }
    None
}

fn should_descend(entry: &DirEntry, include_hidden: bool, ignored: &[String]) -> bool {
    let path = entry.path();
    if is_mandatory_system_path(path) || is_default_ignored_path(path) {
        return false;
    }
    if !include_hidden && is_hidden(path) {
        return false;
    }
    let Some(name) = entry.file_name().to_str() else {
        return true;
    };
    !ignored
        .iter()
        .any(|ignored| ignored == &name.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_paths(home: PathBuf) -> XdgPaths {
        XdgPaths {
            config: home.join("config"),
            data: home.join("data"),
            state: home.join("state"),
            cache: home.join("cache"),
            home,
            runtime: None,
        }
    }

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "protonsearch-search-{label}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn search_is_case_insensitive_and_skips_hidden_by_default() {
        let root = test_root("case-hidden");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("ReadMe.md"), "ok").unwrap();
        fs::write(root.join(".hidden/ReadMe.md"), "hidden").unwrap();
        let paths = test_paths(root.clone());
        let results = search_files(&paths, "README", &SearchOptions::default());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "file");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn entry_limit_does_not_count_the_scope_root() {
        let root = test_root("entry-limit");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("needle.txt"), "ok").unwrap();
        let paths = test_paths(root.clone());
        let options = SearchOptions {
            max_entries: 1,
            ..SearchOptions::default()
        };

        let results = search_files(&paths, "needle", &options);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "needle.txt");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn limited_traversal_gives_each_root_a_turn() {
        let base = test_root("fair-roots");
        let _ = fs::remove_dir_all(&base);
        let home = base.join("home");
        let extra = base.join("extra");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&extra).unwrap();
        fs::write(home.join("noise.txt"), "noise").unwrap();
        fs::write(extra.join("needle.txt"), "ok").unwrap();
        let paths = test_paths(home);
        let options = SearchOptions {
            max_entries: 2,
            extra_roots: vec![extra],
            ..SearchOptions::default()
        };

        let results = search_files(&paths, "needle", &options);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "needle.txt");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn overlapping_roots_do_not_duplicate_results() {
        let root = test_root("overlapping-roots");
        let _ = fs::remove_dir_all(&root);
        let nested = root.join("documents");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("needle.txt"), "ok").unwrap();
        let paths = test_paths(root.clone());
        let options = SearchOptions {
            extra_roots: vec![nested],
            ..SearchOptions::default()
        };

        let results = search_files(&paths, "needle", &options);

        assert_eq!(results.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn file_and_folder_results_stay_distinct_including_empty_scopes() {
        let root = test_root("distinct-kinds");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("needle-folder")).unwrap();
        fs::write(root.join("needle-file.txt"), "ok").unwrap();
        let paths = test_paths(root.clone());
        let options = SearchOptions::default();

        let folders = search_files_filtered(&paths, "", &options, Some("directory"));
        let files = search_files_filtered(&paths, "", &options, Some("file"));
        assert!(folders.iter().all(|result| result.kind == "directory"));
        assert!(files.iter().all(|result| result.kind == "file"));
        assert!(folders.iter().any(|result| result.name == "needle-folder"));
        assert!(files.iter().any(|result| result.name == "needle-file.txt"));

        let matching_folders = search_files_filtered(&paths, "needle", &options, Some("directory"));
        let matching_files = search_files_filtered(&paths, "needle", &options, Some("file"));
        assert_eq!(matching_folders.len(), 1);
        assert_eq!(matching_folders[0].kind, "directory");
        assert_eq!(matching_files.len(), 1);
        assert_eq!(matching_files[0].kind, "file");

        let _ = fs::remove_dir_all(root);
    }
}
