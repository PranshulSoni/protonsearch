use anyhow::{Context, Result};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct XdgPaths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    pub runtime: Option<PathBuf>,
}

impl XdgPaths {
    pub fn discover() -> Result<Self> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .context("HOME is unset or not an absolute path")?;
        let config = xdg_dir("XDG_CONFIG_HOME", home.join(".config"));
        let data = xdg_dir("XDG_DATA_HOME", home.join(".local/share"));
        let state = xdg_dir("XDG_STATE_HOME", home.join(".local/state"));
        let cache = xdg_dir("XDG_CACHE_HOME", home.join(".cache"));
        let runtime = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute());
        Ok(Self {
            home,
            config,
            data,
            state,
            cache,
            runtime,
        })
    }

    pub fn config_dir(&self) -> PathBuf {
        self.config.join("protonsearch")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.data.join("protonsearch")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.state.join("protonsearch")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.cache.join("protonsearch")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir().join("settings.json")
    }

    pub fn user_dirs(&self) -> Vec<PathBuf> {
        let file = self.config.join("user-dirs.dirs");
        let Ok(contents) = fs::read_to_string(file) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for line in contents.lines() {
            let Some((key, raw)) = line.split_once('=') else {
                continue;
            };
            if !key.starts_with("XDG_") || !key.ends_with("_DIR") {
                continue;
            }
            let value = raw.trim().trim_matches('"').replace("\\\"", "\"");
            let value = value.replace("$HOME", &self.home.to_string_lossy());
            let path = PathBuf::from(value);
            if path.is_absolute() && path.exists() && !result.contains(&path) {
                result.push(path);
            }
        }
        result
    }

    pub fn search_roots(&self, extra: &[PathBuf]) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        let mut seen = HashSet::new();
        for root in std::iter::once(self.home.clone())
            .chain(self.user_dirs())
            .chain(extra.iter().cloned())
        {
            if root.is_absolute() && root.exists() && seen.insert(root.clone()) {
                roots.push(root);
            }
        }
        roots
    }

    pub fn application_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.data.join("applications")];
        let data_dirs = env::var_os("XDG_DATA_DIRS")
            .map(|v| v.to_string_lossy().into_owned())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
        dirs.extend(
            data_dirs
                .split(':')
                .filter(|p| !p.is_empty())
                .map(|p| PathBuf::from(p).join("applications")),
        );
        dedupe_paths(dirs)
    }
}

fn xdg_dir(variable: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(variable)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or(fallback)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

pub const MANDATORY_SYSTEM_ROOTS: &[&str] = &[
    "/proc",
    "/sys",
    "/dev",
    "/run",
    "/tmp",
    "/var/tmp",
    "/lost+found",
];

pub const DEFAULT_IGNORED_COMPONENTS: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".aws",
    ".kube",
    ".config",
    ".local/share/keyrings",
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".cache",
    ".local/share/Trash",
    ".npm",
    ".pnpm-store",
    ".yarn",
    ".cargo/registry",
    ".rustup",
    ".gradle",
    ".m2",
    ".nuget",
    ".steam",
    ".var/app",
    ".wine",
    ".docker",
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    "coverage",
    "__pycache__",
    ".venv",
    "venv",
];

pub fn is_mandatory_system_path(path: &Path) -> bool {
    let Some(path) = path.to_str() else {
        return false;
    };
    MANDATORY_SYSTEM_ROOTS.iter().any(|root| {
        path == *root
            || path
                .strip_prefix(root)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

pub fn is_default_ignored_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    DEFAULT_IGNORED_COMPONENTS.iter().any(|ignored| {
        normalized
            .split('/')
            .collect::<Vec<_>>()
            .windows(ignored.split('/').count())
            .any(|window| window.join("/") == *ignored)
    })
}

pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.starts_with('.') && name != ".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_roots_are_never_searchable() {
        assert!(is_mandatory_system_path(Path::new("/proc/1/status")));
        assert!(!is_mandatory_system_path(Path::new("/home/test/proc/file")));
        assert!(is_default_ignored_path(Path::new(
            "/home/test/.ssh/id_ed25519"
        )));
    }

    #[test]
    fn build_and_cache_paths_use_xdg_shape() {
        let paths = XdgPaths {
            home: PathBuf::from("/home/test"),
            config: PathBuf::from("/home/test/.config"),
            data: PathBuf::from("/home/test/.local/share"),
            state: PathBuf::from("/home/test/.local/state"),
            cache: PathBuf::from("/home/test/.cache"),
            runtime: None,
        };
        assert_eq!(
            paths.settings_file(),
            PathBuf::from("/home/test/.config/protonsearch/settings.json")
        );
        assert_eq!(
            paths.data_dir(),
            PathBuf::from("/home/test/.local/share/protonsearch")
        );
    }
}
