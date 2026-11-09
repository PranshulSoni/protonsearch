//! Git repositories, commits and TODO/FIXME scanning. OWNER: sources-content.
//! Uses git CLI with CREATE_NO_WINDOW and structured arguments only.

use super::GitEntry;
use std::path::Path;

pub fn scan_roots(_roots: &[String], _cancelled: &dyn Fn() -> bool) -> (Vec<GitEntry>, Vec<String>) {
    (Vec::new(), Vec::new())
}

pub fn scan_repo(_repo: &Path) -> (Vec<GitEntry>, Vec<String>) {
    (Vec::new(), Vec::new())
}
