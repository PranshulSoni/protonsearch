//! Git repositories, commits and TODO/FIXME scanning. OWNER: sources-content.
//! Uses the git CLI with CREATE_NO_WINDOW and structured arguments only.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context};
use walkdir::WalkDir;

use super::GitEntry;

const MAX_REPOS: usize = 100;
const MAX_DEPTH: usize = 8;
const MAX_COMMITS: usize = 100;
const MAX_TODOS_PER_REPO: usize = 200;
const MAX_TODO_FILE_BYTES: u64 = 1024 * 1024;

const JUNK_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "venv",
    ".venv",
    "appdata",
    "obj",
    "out",
    ".next",
    ".nuxt",
    ".cache",
    "cache",
    "__pycache__",
    "vendor",
    "third_party",
];

const CODE_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "jsx", "ts", "tsx", "go", "cpp", "c", "h", "hpp", "java", "kt", "cs", "md", "txt",
    "rb", "php", "swift", "sh", "ps1", "sql", "lua", "scala", "dart", "yaml", "yml", "toml", "css", "html",
];

pub fn scan_roots(roots: &[String], cancelled: &dyn Fn() -> bool) -> (Vec<GitEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_repos: Vec<PathBuf> = Vec::new();
    for root in roots {
        if cancelled() || seen_repos.len() >= MAX_REPOS {
            break;
        }
        let root_path = Path::new(root);
        if !root_path.is_dir() {
            continue;
        }
        for repo in discover_repos(root_path, MAX_DEPTH, MAX_REPOS) {
            if cancelled() || seen_repos.len() >= MAX_REPOS {
                break;
            }
            let normalized = repo.canonicalize().unwrap_or_else(|_| repo.clone());
            if seen_repos.contains(&normalized) {
                continue;
            }
            seen_repos.push(normalized);
            let (mut repo_entries, mut repo_warnings) = scan_repo(&repo);
            entries.append(&mut repo_entries);
            for warning in repo_warnings.drain(..) {
                if warnings.len() < 20 {
                    warnings.push(warning);
                }
            }
        }
    }
    (entries, warnings)
}

/// Recursively find repository roots, pruning at each repository (so submodules
/// and vendored copies are not re-scanned) and skipping junk directories.
pub fn discover_repos(root: &Path, max_depth: usize, max_repos: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if max_repos == 0 || max_depth == 0 {
        return found;
    }
    let walker = WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_dir() {
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                !JUNK_DIRS.contains(&name.as_str())
            } else {
                true
            }
        });
    for entry in walker.filter_map(|entry| entry.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }
        if entry.path().join(".git").exists() {
            found.push(entry.path().to_path_buf());
            if found.len() >= max_repos {
                break;
            }
        }
    }
    found
}

pub fn scan_repo(repo: &Path) -> (Vec<GitEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let name = repo
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| repo.to_string_lossy().to_string());

    let mut branch: Option<String> = None;
    let mut branches = Vec::new();
    match run_git(repo, &["branch", "--no-color", "--sort=-committerdate"]) {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                // "* " marks HEAD, "+ " a branch checked out in another worktree.
                let name = line.trim_start_matches(&['*', '+'][..]).trim();
                if line.starts_with('*') {
                    branch = Some(name.to_string());
                }
                // A detached HEAD prints "(HEAD detached at ...)", which is not a branch.
                if !name.starts_with('(') {
                    branches.push(name.to_string());
                }
            }
        }
        Err(error) => {
            if warnings.len() < 10 {
                warnings.push(format!("{}: {error}", repo.display()));
            }
        }
    }
    entries.push(GitEntry::Repo {
        path: repo.to_string_lossy().to_string(),
        name,
        branch,
        branches,
    });

    match run_git(repo, &["log", "--max-count=100", LOG_FORMAT]) {
        Ok(output) => entries.extend(parse_commits(repo, &output.stdout)),
        Err(error) => {
            if warnings.len() < 10 {
                warnings.push(format!("{}: {error}", repo.display()));
            }
        }
    }

    entries.extend(scan_todos(repo));
    (entries, warnings)
}

const LOG_FORMAT: &str = "--format=%H%x1F%an%x1F%at%x1F%s";

fn parse_commits(repo: &Path, stdout: &[u8]) -> Vec<GitEntry> {
    String::from_utf8_lossy(stdout)
        .lines()
        .take(MAX_COMMITS)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\x1F').collect();
            (parts.len() >= 4).then(|| GitEntry::Commit {
                repo: repo.to_string_lossy().to_string(),
                hash: parts[0].to_string(),
                message: parts[3].to_string(),
                author: parts[1].to_string(),
                date_ms: parts[2].parse::<i64>().unwrap_or(0).saturating_mul(1000),
            })
        })
        .collect()
}

/// Newest commits on one local branch, read on demand by the Git card's branch picker.
pub fn branch_commits(repo: &Path, branch: &str) -> anyhow::Result<Vec<GitEntry>> {
    // The refs/heads/ prefix means a branch name can never be read as a git option.
    let rev = format!("refs/heads/{branch}");
    let output = run_git(repo, &["log", "--max-count=100", LOG_FORMAT, &rev, "--"])?;
    Ok(parse_commits(repo, &output.stdout))
}

fn scan_todos(repo: &Path) -> Vec<GitEntry> {
    let mut todos = Vec::new();
    let walker = WalkDir::new(repo)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if entry.file_type().is_dir() {
                if name == ".git" || JUNK_DIRS.contains(&name.as_str()) {
                    return false;
                }
                // Do not descend into nested repositories/submodules.
                if entry.path().join(".git").exists() {
                    return false;
                }
                true
            } else {
                true
            }
        });
    for entry in walker.filter_map(|entry| entry.ok()) {
        if todos.len() >= MAX_TODOS_PER_REPO {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let extension = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !CODE_EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }
        if entry
            .metadata()
            .map(|meta| meta.len() > MAX_TODO_FILE_BYTES)
            .unwrap_or(true)
        {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        for (index, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if !(trimmed.starts_with("//")
                || trimmed.starts_with('#')
                || trimmed.starts_with("/*")
                || trimmed.starts_with("--")
                || trimmed.starts_with("<!--")
                || trimmed.starts_with('*'))
            {
                continue;
            }
            if trimmed.contains("TODO") || trimmed.contains("FIXME") || trimmed.contains("BUG") {
                todos.push(GitEntry::Todo {
                    repo: repo.to_string_lossy().to_string(),
                    path: path.to_string_lossy().to_string(),
                    line: (index + 1) as u32,
                    text: trimmed.chars().take(300).collect(),
                });
                if todos.len() >= MAX_TODOS_PER_REPO {
                    break;
                }
            }
        }
    }
    todos
}

fn git_command() -> Command {
    let mut command = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

fn run_git(repo: &Path, args: &[&str]) -> anyhow::Result<std::process::Output> {
    let output = git_command()
        .current_dir(repo)
        .args(args)
        .output()
        .with_context(|| format!("running git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        bail!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output)
}
