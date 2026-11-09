//! Shared test helpers. The sandboxed test process cannot write %TEMP% or
//! %APPDATA% on this host, so tests use a writable directory under the
//! workspace target dir (or PROTONSEARCH_TEST_DIR when set).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

pub fn base_dir() -> PathBuf {
    static BASE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        if let Some(dir) = std::env::var_os("PROTONSEARCH_TEST_DIR") {
            let dir = PathBuf::from(dir);
            let _ = std::fs::create_dir_all(&dir);
            return dir;
        }
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target = manifest
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("target"))
            .unwrap_or_else(std::env::temp_dir);
        let dir = target.join("psidx-tests");
        let _ = std::fs::create_dir_all(&dir);
        let probe = dir.join(format!(".probe-{}", std::process::id()));
        if std::fs::write(&probe, b"x").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return dir;
        }
        std::env::temp_dir()
    })
    .clone()
}
pub fn unique(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    base_dir().join(format!("{}-{}-{}", std::process::id(), n, tag))
}

pub struct TempTree {
    pub root: PathBuf,
}

impl TempTree {
    pub fn new(tag: &str) -> Self {
        let root = unique(tag);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn file(&self, rel: &str, contents: &str) -> PathBuf {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&path).unwrap();
        std::io::Write::write_all(&mut f, contents.as_bytes()).unwrap();
        path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}