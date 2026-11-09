//! Integration tests for ps-sources against real Windows data and synthetic
//! browser/git/database fixtures created on this machine.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use ps_sources::{
    browser, clipboard, git, quicklinks, BrowserEntry, GitEntry, Sources, WorkflowKind,
};

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, bytes).unwrap();
    path
}

// ---------------------------------------------------------------------------
// Calculator (module has its own unit tests; these exercise the public API)
// ---------------------------------------------------------------------------

#[test]
fn calculator_public_api_works() {
    assert_eq!(ps_sources::calc::evaluate("2+2").unwrap().value, 4.0);
    assert_eq!(ps_sources::calc::evaluate("15% of 340").unwrap().value, 51.0);
    assert!(ps_sources::calc::convert("5 km to mi").is_some());
    assert!(ps_sources::calc::evaluate("just a search phrase").is_none());
}

// ---------------------------------------------------------------------------
// Quicklinks + snippets
// ---------------------------------------------------------------------------

#[test]
fn quicklinks_roundtrip_and_delete() {
    let dir = workdir("quicklinks");
    let db = dir.join("index.db");
    quicklinks::save_quicklink(&db, "Docs", "https://example.com/docs", "docs").unwrap();
    quicklinks::save_snippet(&db, "Signature", "Best regards, Pranshul", "sig").unwrap();

    let entries = quicklinks::load(&db);
    assert_eq!(entries.len(), 2);
    let quicklink = entries
        .iter()
        .find(|entry| entry.kind == WorkflowKind::Quicklink)
        .unwrap();
    assert_eq!(quicklink.name, "Docs");
    assert_eq!(quicklink.id, "quicklink:Docs");
    assert_eq!(quicklink.payload, "https://example.com/docs");
    let snippet = entries
        .iter()
        .find(|entry| entry.kind == WorkflowKind::Snippet)
        .unwrap();
    assert_eq!(snippet.payload, "Best regards, Pranshul");

    assert!(quicklinks::delete(&db, "quicklink:Docs").unwrap());
    assert_eq!(quicklinks::load(&db).len(), 1);
    assert!(!quicklinks::delete(&db, "quicklink:Docs").unwrap());
}

#[test]
fn quicklinks_load_missing_db_returns_empty() {
    let dir = workdir("quicklinks-missing");
    assert!(quicklinks::load(&dir.join("nope.db")).is_empty());
}

// ---------------------------------------------------------------------------
// Clipboard history
// ---------------------------------------------------------------------------

#[test]
fn clipboard_records_dedupes_and_searches() {
    let dir = workdir("clipboard");
    let db = dir.join("index.db");
    assert!(clipboard::record(&db, "first copied text", "Code").unwrap());
    assert!(!clipboard::record(&db, "first copied text", "Code").unwrap(), "consecutive duplicate must be deduped");
    assert!(clipboard::record(&db, "second copied protonsearch text", "chrome").unwrap());
    assert!(!clipboard::record(&db, "   ", "x").unwrap());
    assert!(!clipboard::record(&db, &"x".repeat(2 * 1024 * 1024), "x").unwrap());

    let hits = clipboard::search(&db, "protonsearch", 10);
    assert_eq!(hits.len(), 1, "hits={hits:?}");
    assert_eq!(hits[0].source_app, "chrome");

    let recent = clipboard::list_recent(&db, 10);
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].text, "second copied protonsearch text");
    assert!(clipboard::set_pinned(&db, recent[0].id, true).unwrap());
    let pinned_first = clipboard::list_recent(&db, 10);
    assert!(pinned_first[0].pinned);
}

#[test]
fn clipboard_image_is_stored_as_png() {
    let dir = workdir("clipboard-image");
    let db = dir.join("index.db");
    let images = dir.join("images");
    // 1x1 transparent PNG.
    let png: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    assert!(clipboard::record_image(&db, &png, "snippingtool", &images).unwrap());
    assert!(!clipboard::record_image(&db, &png, "snippingtool", &images).unwrap());
    let files: Vec<_> = std::fs::read_dir(&images).unwrap().flatten().collect();
    assert_eq!(files.len(), 1);
}

// ---------------------------------------------------------------------------
// Browser history / bookmarks (synthetic profiles)
// ---------------------------------------------------------------------------

#[test]
fn chromium_profile_history_and_bookmarks_are_read() {
    let dir = workdir("browser-chromium");
    let profile = dir.join("Default");
    std::fs::create_dir_all(&profile).unwrap();

    // Chromium epoch: 1601-01-01 in microseconds.
    let chrome_time: i64 = 11_644_473_600_000_000 + 1_760_000_000_000 * 1000;
    let conn = rusqlite::Connection::open(profile.join("History")).unwrap();
    conn.execute_batch(
        "CREATE TABLE urls (
            id INTEGER PRIMARY KEY,
            url TEXT,
            title TEXT,
            visit_count INTEGER,
            last_visit_time INTEGER,
            hidden INTEGER DEFAULT 0
        );",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO urls(url, title, visit_count, last_visit_time, hidden) VALUES
            ('https://example.com/rust', 'Rust docs', 7, ?1, 0),
            ('chrome-extension://abc/page.html', 'extension', 1, ?1, 0)",
        rusqlite::params![chrome_time],
    )
    .unwrap();
    drop(conn);

    let bookmarks = serde_json::json!({
        "roots": {
            "bookmark_bar": {
                "type": "folder",
                "children": [
                    {"type": "url", "name": "ProtonSearch", "url": "https://protonsearch.dev"},
                    {"type": "folder", "children": [
                        {"type": "url", "name": "Nested", "url": "https://example.com/nested"}
                    ]}
                ]
            }
        }
    });
    std::fs::write(
        profile.join("Bookmarks"),
        serde_json::to_vec(&bookmarks).unwrap(),
    )
    .unwrap();

    let (entries, warnings) = browser::scan_chromium_profile(&profile, "Chrome");
    assert!(warnings.is_empty(), "warnings={warnings:?}");
    let history = entries
        .iter()
        .find(|entry| entry.url == "https://example.com/rust")
        .expect("history entry");
    assert_eq!(history.title, "Rust docs");
    assert_eq!(history.visit_count, 7);
    assert!(!history.bookmark);
    assert!(history.last_visit_ms > 0, "epoch conversion must produce unix ms");
    assert!(
        entries.iter().all(|entry| !entry.url.starts_with("chrome-extension://")),
        "extension urls must be filtered"
    );
    let bookmarks: Vec<&BrowserEntry> = entries.iter().filter(|entry| entry.bookmark).collect();
    assert_eq!(bookmarks.len(), 2, "nested bookmark must be found");
    assert!(bookmarks.iter().any(|entry| entry.url == "https://example.com/nested"));
}

#[test]
fn firefox_profile_history_and_bookmarks_are_read() {
    let dir = workdir("browser-firefox");
    let profile = dir.join("abc.default");
    std::fs::create_dir_all(&profile).unwrap();
    let conn = rusqlite::Connection::open(profile.join("places.sqlite")).unwrap();
    conn.execute_batch(
        "CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER, last_visit_date INTEGER);
         CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER, title TEXT);",
    )
    .unwrap();
    let firefox_us: i64 = 1_760_000_000_000_000;
    conn.execute(
        "INSERT INTO moz_places VALUES (1, 'https://mozilla.org', 'Mozilla', 3, ?1)",
        rusqlite::params![firefox_us],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO moz_bookmarks VALUES (1, 1, 1, 'Mozilla bookmarked')",
        [],
    )
    .unwrap();
    drop(conn);

    let (entries, warnings) = browser::scan_firefox_profile(&profile);
    assert!(warnings.is_empty(), "warnings={warnings:?}");
    assert!(entries.iter().any(|entry| {
        entry.url == "https://mozilla.org" && entry.title == "Mozilla" && entry.visit_count == 3
    }));
    assert!(entries.iter().any(|entry| entry.bookmark && entry.title == "Mozilla bookmarked"));
}

#[test]
fn browser_privacy_setting_disables_scan() {
    let mut privacy = ps_ipc::PrivacySettings::default();
    privacy.browser_history = false;
    let (entries, warnings) = browser::scan_configured(&privacy);
    assert!(entries.is_empty());
    assert!(warnings.is_empty());
}

#[test]
fn epoch_conversion_helpers_are_correct() {
    // 1970-01-01 in chromium epoch.
    assert_eq!(browser::chromium_time_to_unix_ms(11_644_473_600_000_000), 0);
    assert_eq!(browser::chromium_time_to_unix_ms(0), 0);
    assert_eq!(browser::firefox_time_to_unix_ms(1_760_000_000_000_000), 1_760_000_000_000);
}

// ---------------------------------------------------------------------------
// Git
// ---------------------------------------------------------------------------

fn git_run(dir: &Path, args: &[&str]) -> bool {
    let mut command = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .current_dir(dir)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn git_repo_commits_and_todos_are_scanned() {
    let dir = workdir("git-repo");
    let repo = dir.join("sample-repo");
    std::fs::create_dir_all(&repo).unwrap();
    if !git_run(&repo, &["init", "-q"]) {
        eprintln!("SKIP: git CLI not available");
        return;
    }
    write_file(
        &repo,
        "src/main.rs",
        b"fn main() {}\n// TODO: improve ranking quality\n// FIXME: handle unicode\n",
    );
    git_run(&repo, &["add", "."]);
    let committed = git_run(
        &repo,
        &[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Proton Test",
            "commit",
            "-qm",
            "initial commit",
        ],
    );
    if !committed {
        eprintln!("SKIP: git commit unavailable");
        return;
    }

    let (entries, warnings) = git::scan_repo(&repo);
    assert!(warnings.is_empty(), "warnings={warnings:?}");
    assert!(
        entries.iter().any(|entry| matches!(entry, GitEntry::Repo { name, .. } if name == "sample-repo")),
        "repo entry missing: {entries:?}"
    );
    assert!(entries.iter().any(|entry| matches!(entry, GitEntry::Commit { message, author, .. } if message == "initial commit" && author == "Proton Test")));
    let todos: Vec<&GitEntry> = entries
        .iter()
        .filter(|entry| matches!(entry, GitEntry::Todo { .. }))
        .collect();
    assert_eq!(todos.len(), 2, "todos={todos:?}");
    assert!(matches!(todos[0], GitEntry::Todo { line: 2, .. }));
}

#[test]
fn git_discovery_finds_repo_roots() {
    let dir = workdir("git-discovery");
    let repo = dir.join("projects").join("repo-one");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    // Junk directories must be skipped even if they contain .git.
    let junk = dir.join("projects").join("node_modules").join("fake-repo");
    std::fs::create_dir_all(junk.join(".git")).unwrap();
    let found = git::discover_repos(&dir.join("projects"), 8, 50);
    assert!(found.iter().any(|path| path.ends_with("repo-one")));
    assert!(!found.iter().any(|path| path.to_string_lossy().contains("node_modules")));
}

// ---------------------------------------------------------------------------
// Installed apps (real machine)
// ---------------------------------------------------------------------------

#[test]
fn scans_real_installed_apps() {
    let dir = workdir("apps-scan");
    let (apps, warnings) = ps_sources::apps::scan_with_icons(&dir.join("icons"));
    eprintln!(
        "apps discovered: {} (warnings: {})",
        apps.len(),
        warnings.len()
    );
    assert!(!apps.is_empty(), "expected at least one installed app");
    assert!(
        apps.iter().all(|app| !app.name.trim().is_empty() && !app.target.trim().is_empty()),
        "every app needs a name and target"
    );
    let mut keys = std::collections::HashSet::new();
    for app in &apps {
        let key = format!("{}|{}", app.name.to_lowercase(), app.target.to_lowercase());
        assert!(keys.insert(key), "duplicate app entry: {} {}", app.name, app.target);
    }
    let with_icons = apps.iter().filter(|app| app.icon_path.is_some()).count();
    eprintln!("apps with extracted icons: {with_icons}/{}", apps.len());
    assert!(with_icons > 0, "at least some real icons must extract");
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[test]
fn actions_dispatch_is_structured_and_safe() {
    use ps_ipc::{ActionSpec, NativeCommand};

    // CopyText is a safe real action: verify the clipboard actually changed.
    let payload = format!("ps-sources action test {}", std::process::id());
    let outcome = ps_sources::actions::execute(&ActionSpec::CopyText {
        text: payload.clone(),
    });
    assert!(outcome.ok, "copy action failed: {}", outcome.message);
    assert_eq!(ps_windows::get_clipboard_text().as_deref(), Some(payload.as_str()));

    // Missing paths return structured failures, never panics.
    let outcome = ps_sources::actions::execute(&ActionSpec::OpenPath {
        path: "C:\\definitely\\not\\here.txt".to_string(),
        line: None,
        column: None,
    });
    assert!(!outcome.ok);
    assert!(!outcome.message.is_empty());

    // Non-http url is rejected by the action layer too.
    let outcome = ps_sources::actions::execute(&ActionSpec::OpenUrl {
        url: "file:///C:/Windows/System32/calc.exe".to_string(),
    });
    assert!(!outcome.ok);

    // Indexing pause/resume are engine-owned acknowledgements.
    let outcome = ps_sources::actions::run_native(NativeCommand::PauseIndexing);
    assert!(outcome.ok, "{}", outcome.message);
    let outcome = ps_sources::actions::run_native(NativeCommand::OpenSettings);
    let _ = outcome;
}

// ---------------------------------------------------------------------------
// Aggregate load
// ---------------------------------------------------------------------------

#[test]
fn sources_load_returns_real_apps_and_respects_privacy_defaults() {
    let dir = workdir("sources-load");
    let settings = ps_ipc::Settings::default();
    let (sources, warnings) = Sources::load_with_paths(
        &settings,
        None,
        &dir.join("index.db"),
        &dir.join("icons"),
    );
    eprintln!(
        "sources: apps={} recent={} clipboard={} browser={} git={} workflows={} warnings={}",
        sources.apps.len(),
        sources.recent.len(),
        sources.clipboard.len(),
        sources.browser.len(),
        sources.git.len(),
        sources.workflows.len(),
        warnings.len()
    );
    assert!(!sources.apps.is_empty(), "apps source must load on a real machine");
    assert!(sources.recent.len() <= 400);
    assert!(sources.browser.is_empty(), "browser disabled by default");
    assert!(sources.git.is_empty(), "git disabled by default");
}
