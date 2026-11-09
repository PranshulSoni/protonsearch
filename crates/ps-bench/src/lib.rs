//! Reproducible benchmark + relevance harness for ProtonSearch v2. OWNER: perf-bench.
//!
//! Design rules (see docs/performance/METHODOLOGY.md):
//! - Every number is produced by a command in this crate and stored with host,
//!   corpus hash, cache state and the exact command line.
//! - Engine-only time is reported separately from process startup / dataset load.
//! - When an engine API is still a stub the harness records "engine_not_ready"
//!   instead of inventing numbers.

pub mod args;
pub mod childproc;
pub mod contentbench;
pub mod dataset;
pub mod env;
pub mod filenamebench;
pub mod ocrbench;
pub mod queries;
pub mod realfiles;
pub mod relevance;
pub mod stats;

use std::path::Path;

use serde::Serialize;

/// Write pretty JSON, creating parent directories.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Result schema token, bumped when the on-disk result format changes.
pub const RESULT_SCHEMA: &str = "ps-bench-result/1";

/// FNV-1a 64-bit hash; cheap deterministic corpus fingerprint.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// FNV-1a hash of a file, streamed. Returns (bytes, hash).
pub fn fnv1a64_file(path: &Path) -> anyhow::Result<(u64, u64)> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut total: u64 = 0;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        for &b in &buf[..n] {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Ok((total, h))
}

/// SHA-256 via the OS certutil tool (no extra crate). Returns None when the
/// tool is unavailable so callers can record that honestly instead of guessing.
pub fn sha256_via_certutil(path: &Path) -> Option<String> {
    let out = std::process::Command::new("certutil")
        .args(["-hashfile"])
        .arg(path)
        .arg("SHA256")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let t = line.trim().replace(' ', "");
        if t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(t.to_ascii_lowercase());
        }
    }
    None
}
