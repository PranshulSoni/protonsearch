//! Spawn the same ps-bench executable as a measurement child and parse its JSON.

use std::time::Instant;

use anyhow::Context;
use serde_json::Value;

pub struct ChildOutcome {
    pub value: Value,
    pub wall_ms: f64,
    pub exit_code: i32,
}

/// Run ps-bench with args (e.g. ["content-cold-child", ...]); the child must print
/// one line starting with PSBENCH_JSON: followed by a JSON object.
pub fn run_child_json(args: &[String]) -> anyhow::Result<ChildOutcome> {
    let exe = std::env::current_exe().context("resolve current exe")?;
    let start = Instant::now();
    let out = std::process::Command::new(exe)
        .args(args)
        .output()
        .context("spawn ps-bench measurement child")?;
    let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
    if !out.status.success() {
        anyhow::bail!(
            "measurement child failed (exit {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let marker = "PSBENCH_JSON:";
    let pos = text.find(marker).with_context(|| {
        format!(
            "child did not print {marker}; stdout head: {}",
            text.chars().take(400).collect::<String>()
        )
    })?;
    let line = text[pos + marker.len()..]
        .lines()
        .next()
        .context("child JSON line missing")?;
    let value: Value = serde_json::from_str(line.trim()).context("parse child JSON")?;
    Ok(ChildOutcome {
        value,
        wall_ms,
        exit_code: out.status.code().unwrap_or(-1),
    })
}
