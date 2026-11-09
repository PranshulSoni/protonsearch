//! Fixed query suite for filename benchmarks and golden relevance fixtures.
//! The suite is deterministic and dataset-size independent because the dataset
//! generator always plants the same anchor entries.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDef {
    pub id: String,
    pub category: String,
    pub query: String,
    pub rationale: String,
}

fn q(id: &str, category: &str, query: &str, rationale: &str) -> QueryDef {
    QueryDef {
        id: id.to_string(),
        category: category.to_string(),
        query: query.to_string(),
        rationale: rationale.to_string(),
    }
}

/// Filename query suite. Keep stable: benchmark tables compare across commits.
pub fn filename_suite() -> Vec<QueryDef> {
    vec![
        q("exact_unique", "exact", "Project Phoenix Budget 2026.xlsx", "unique planted anchor; full exact filename"),
        q("exact_dup", "exact_duplicate", "index.js", "planted duplicate in 3 projects; tie handling, not top-1 identity"),
        q("exact_readme", "exact_duplicate", "README.md", "very common duplicate name"),
        q("exact_package", "exact", "package.json", "planted exact name"),
        q("prefix_screenshot", "prefix", "Screenshot 2026-11-09", "prefix of planted screenshot anchor"),
        q("prefix_quarterly", "prefix", "quarterly_rep", "snake-case prefix"),
        q("prefix_img", "prefix", "IMG_2026", "camera-style prefix"),
        q("multi_term", "multi_term", "project phoenix budget", "three-term query"),
        q("multi_term_2", "multi_term", "quarterly report final", "terms present in one anchor"),
        q("typo_quarterly", "typo", "quaterly report final", "one deleted char in first term"),
        q("typo_screenshot", "typo", "screnshot 2026-11-09", "one deleted char in first term"),
        q("fuzzy_subseq", "fuzzy", "ppjbudget", "initialism-ish subsequence, deliberately hard"),
        q("cjk_jp", "cjk", "会議議事録", "planted Japanese filename"),
        q("cjk_zh", "cjk", "日本語_メモ", "planted CJK anchor"),
        q("unicode", "unicode", "café résumé", "planted accented unicode anchor"),
        q("hidden", "hidden", ".env", "hidden file anchor, leading dot"),
        q("long_path", "long_path", r"C:\Users\Pranshul\Documents\Archive\2026\Q3\Backups\Projects\protonsearch\apps\desktop\src\components\internal\generated\very_long_anchor_filename_for_path_length_testing_0001.json", "full long path as query"),
        q("ext_pdf", "extension_like", ".pdf", "dot-extension shaped query"),
        q("ext_xlsx", "extension_like", "xlsx", "bare extension token"),
        q("folder_name", "folder", "ProtonSearch Phoenix", "planted folder anchor"),
        q("zero_match", "zero_match", "zzqxvbnmplkjhgfdsa9917", "must return no results"),
        q("zero_match_cjk", "zero_match", "存在しないファイル名です", "CJK 0-match"),
    ]
}

/// Write the suite as JSONL (one QueryDef per line), creating parent dirs.
pub fn write_suite(path: &Path) -> anyhow::Result<usize> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut out = String::new();
    let suite = filename_suite();
    for item in &suite {
        out.push_str(&serde_json::to_string(item)?);
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(suite.len())
}

/// Load a query suite from JSONL, ignoring blank lines.
pub fn load_suite(path: &Path) -> anyhow::Result<Vec<QueryDef>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let def: QueryDef = serde_json::from_str(line)
            .map_err(|e| anyhow::anyhow!("{}:{}: {e}", path.display(), i + 1))?;
        out.push(def);
    }
    Ok(out)
}
