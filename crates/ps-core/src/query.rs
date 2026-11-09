//! Query parsing: explicit prefixes, quoted phrases, structured filters and
//! intent detection. OWNER: engine-core.
//!
//! Prefixes are case-insensitive and accept an optional space after the colon:
//!   file: folder: content: app: clip: history: bookmarks: git: commits:
//!   todos: settings: calc: ocr: command: recent: quicklink: snippet:
//! Filters may appear anywhere in the query and are removed from the free-text
//! terms:
//!   ext:pdf,docx  size:>1mb  modified:7d  in:"C:\Users\Me\Documents"  kind:file
//! Math detection is delegated to ps_sources::calc (never re-implemented here).

use ps_ipc::{SearchFilters, SearchScope, SourceKind};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedQuery {
    /// Original query, trimmed.
    pub raw: String,
    /// Free-text terms, lower-cased. A quoted phrase is one term.
    pub terms: Vec<String>,
    /// Scope selected by an explicit prefix (All when none).
    pub scope: SearchScope,
    /// Structured filters parsed from the query.
    pub filters: SearchFilters,
    /// Calculator intent, delegated to ps-sources::calc.
    pub is_math: bool,
    /// Text the filename index should search (terms joined by one space).
    pub filename_query: String,
}

impl ParsedQuery {
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        let tokens = tokenize(trimmed);
        let mut terms: Vec<String> = Vec::new();
        let mut scope = SearchScope::All;
        let mut filters = SearchFilters::default();

        for token in tokens {
            if let Some((key, value)) = split_prefix(&token) {
                if apply_prefix(key, &mut scope, &mut filters) {
                    if !value.is_empty() {
                        terms.push(value.to_lowercase());
                    }
                    continue;
                }
                if apply_filter(key, value, &mut filters) {
                    continue;
                }
            }
            terms.push(token.to_lowercase());
        }

        let filename_query = terms.join(" ");
        let is_math = if trimmed.is_empty() {
            false
        } else {
            ps_sources::calc::evaluate(trimmed).is_some()
                || ps_sources::calc::convert(trimmed).is_some()
        };
        Self {
            raw: trimmed.to_string(),
            terms,
            scope,
            filters,
            is_math,
            filename_query,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

/// Split on whitespace but keep quoted sections together (quotes removed).
fn tokenize(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in raw.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                if !quoted {
                    // Closing quote: finish the token, even when empty.
                    if !current.is_empty() {
                        out.push(std::mem::take(&mut current));
                    }
                }
            }
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Split "key:value" (or "key: value" already tokenized as "key:") into parts.
fn split_prefix(token: &str) -> Option<(&str, &str)> {
    let idx = token.find(':')?;
    if idx == 0 || idx + 1 > token.len() {
        return None;
    }
    let key = &token[..idx];
    if !key.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some((key, &token[idx + 1..]))
}

fn apply_prefix(key: &str, scope: &mut SearchScope, filters: &mut SearchFilters) -> bool {
    let mapped = match key.to_ascii_lowercase().as_str() {
        "file" | "files" => SearchScope::Files,
        "folder" | "folders" => SearchScope::Folders,
        "content" => SearchScope::Content,
        "app" | "apps" => SearchScope::Apps,
        "clip" | "clipboard" => SearchScope::Clipboard,
        "history" => SearchScope::Browser,
        "bookmarks" | "bookmark" => {
            push_kind(filters, SourceKind::Bookmark);
            SearchScope::Browser
        }
        "git" | "repo" | "repos" => SearchScope::Git,
        "commits" | "commit" => {
            push_kind(filters, SourceKind::GitCommit);
            SearchScope::Git
        }
        "todos" | "todo" => {
            push_kind(filters, SourceKind::GitTodo);
            SearchScope::Git
        }
        "ocr" => SearchScope::Ocr,
        "settings" | "setting" => SearchScope::Settings,
        "calc" | "calculate" => SearchScope::Actions,
        "command" | "commands" => SearchScope::Commands,
        "recent" => {
            push_kind(filters, SourceKind::Recent);
            SearchScope::All
        }
        "quicklink" | "quicklinks" => {
            push_kind(filters, SourceKind::Quicklink);
            SearchScope::All
        }
        "snippet" | "snippets" => {
            push_kind(filters, SourceKind::Snippet);
            SearchScope::All
        }
        _ => return false,
    };
    *scope = mapped;
    true
}

fn push_kind(filters: &mut SearchFilters, kind: SourceKind) {
    if !filters.kinds.contains(&kind) {
        filters.kinds.push(kind);
    }
}

fn apply_filter(key: &str, value: &str, filters: &mut SearchFilters) -> bool {
    if value.is_empty() {
        return false;
    }
    match key.to_ascii_lowercase().as_str() {
        "ext" | "extension" | "type" => {
            for part in value.split(',') {
                let ext = part.trim().trim_start_matches('.').to_lowercase();
                if !ext.is_empty() {
                    filters.extensions.push(ext);
                }
            }
            true
        }
        "kind" => {
            match value.to_ascii_lowercase().as_str() {
                "file" | "files" => push_kind(filters, SourceKind::File),
                "folder" | "folders" | "dir" | "directory" => push_kind(filters, SourceKind::Folder),
                _ => return false,
            }
            true
        }
        "size" => parse_size_filter(value, filters),
        "modified" | "mtime" => parse_modified_filter(value, filters),
        "in" | "under" | "path" => {
            let path = value.trim().trim_matches('"');
            if path.is_empty() {
                return false;
            }
            filters.under_path = Some(path.to_string());
            true
        }
        _ => false,
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn parse_size_bytes(raw: &str) -> Option<u64> {
    let s = raw.trim().to_lowercase();
    let (number, multiplier) = if let Some(v) = s.strip_suffix("tb") {
        (v, 1024u64.pow(4))
    } else if let Some(v) = s.strip_suffix("gb") {
        (v, 1024u64.pow(3))
    } else if let Some(v) = s.strip_suffix("mb") {
        (v, 1024u64.pow(2))
    } else if let Some(v) = s.strip_suffix("kb") {
        (v, 1024u64)
    } else if let Some(v) = s.strip_suffix('b') {
        (v, 1u64)
    } else {
        (s.as_str(), 1u64)
    };
    let number: f64 = number.trim().parse().ok()?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    Some((number * multiplier as f64) as u64)
}

fn parse_size_filter(value: &str, filters: &mut SearchFilters) -> bool {
    let (op, rest) = split_operator(value);
    let Some(bytes) = parse_size_bytes(rest) else {
        return false;
    };
    match op {
        ">" => filters.min_size = Some(bytes.saturating_add(1)),
        ">=" => filters.min_size = Some(bytes),
        "<" => filters.max_size = Some(bytes.saturating_sub(1)),
        "<=" => filters.max_size = Some(bytes),
        _ => {
            filters.min_size = Some(bytes);
            filters.max_size = Some(bytes);
        }
    }
    true
}

fn split_operator(value: &str) -> (&str, &str) {
    for op in [">=", "<=", ">", "<", "="] {
        if let Some(rest) = value.strip_prefix(op) {
            return (op, rest.trim());
        }
    }
    ("", value.trim())
}

fn parse_duration_ms(raw: &str) -> Option<i64> {
    let s = raw.trim().to_lowercase();
    let (number, multiplier) = if let Some(v) = s.strip_suffix("mo") {
        (v, 30i64 * 86_400_000)
    } else if let Some(v) = s.strip_suffix('d') {
        (v, 86_400_000i64)
    } else if let Some(v) = s.strip_suffix('h') {
        (v, 3_600_000i64)
    } else if let Some(v) = s.strip_suffix('m') {
        (v, 60_000i64)
    } else if let Some(v) = s.strip_suffix('w') {
        (v, 7 * 86_400_000i64)
    } else if let Some(v) = s.strip_suffix('y') {
        (v, 365 * 86_400_000i64)
    } else {
        return None;
    };
    let number: i64 = number.trim().parse().ok()?;
    if number < 0 {
        return None;
    }
    Some(number.saturating_mul(multiplier))
}

fn parse_modified_filter(value: &str, filters: &mut SearchFilters) -> bool {
    let (op, rest) = split_operator(value);
    if let Some(delta) = parse_duration_ms(rest) {
        let cutoff = now_ms().saturating_sub(delta);
        match op {
            ">" => filters.modified_before = Some(cutoff),
            "<" => filters.modified_after = Some(cutoff),
            _ => filters.modified_after = Some(cutoff),
        }
        return true;
    }
    // Absolute date: YYYY-MM-DD (interpreted as UTC midnight).
    if let Some(day) = parse_date_utc_ms(rest) {
        match op {
            ">" => filters.modified_before = Some(day),
            "<" => filters.modified_after = Some(day),
            _ => filters.modified_after = Some(day),
        }
        return true;
    }
    false
}

fn parse_date_utc_ms(raw: &str) -> Option<i64> {
    let s = raw.trim();
    let mut parts = s.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Days since Unix epoch (civil calendar arithmetic, no timezone tables).
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days.saturating_mul(86_400_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_terms_are_lowercased() {
        let p = ParsedQuery::parse("  Quarterly Report  ");
        assert_eq!(p.terms, vec!["quarterly", "report"]);
        assert_eq!(p.filename_query, "quarterly report");
        assert_eq!(p.scope, SearchScope::All);
        assert!(!p.is_empty());
    }

    #[test]
    fn prefixes_are_case_insensitive_with_optional_space() {
        let p = ParsedQuery::parse("FILE: report");
        assert_eq!(p.scope, SearchScope::Files);
        assert_eq!(p.terms, vec!["report"]);
        let p = ParsedQuery::parse("Folder:photos");
        assert_eq!(p.scope, SearchScope::Folders);
        assert_eq!(p.terms, vec!["photos"]);
        let p = ParsedQuery::parse("content: budget 2026");
        assert_eq!(p.scope, SearchScope::Content);
        assert_eq!(p.terms, vec!["budget", "2026"]);
        for (text, scope) in [
            ("app:chrome", SearchScope::Apps),
            ("clip:token", SearchScope::Clipboard),
            ("history:rust", SearchScope::Browser),
            ("git:proton", SearchScope::Git),
            ("commits:fix", SearchScope::Git),
            ("todos:bug", SearchScope::Git),
            ("settings:display", SearchScope::Settings),
            ("calc:2+2", SearchScope::Actions),
            ("ocr:invoice", SearchScope::Ocr),
        ] {
            let p = ParsedQuery::parse(text);
            assert_eq!(p.scope, scope, "{text}");
        }
    }

    #[test]
    fn quoted_phrases_stay_together() {
        let p = ParsedQuery::parse(r#"file:"annual report" final"#);
        assert_eq!(p.scope, SearchScope::Files);
        assert_eq!(p.terms, vec!["annual report", "final"]);
        assert_eq!(p.filename_query, "annual report final");
    }

    #[test]
    fn filters_are_parsed_and_removed_from_terms() {
        let p = ParsedQuery::parse("report ext:pdf,docx size:>1mb");
        assert_eq!(p.terms, vec!["report"]);
        assert_eq!(p.filters.extensions, vec!["pdf", "docx"]);
        assert_eq!(p.filters.min_size, Some(1024 * 1024 + 1));

        let p = ParsedQuery::parse("notes size:<500kb");
        assert_eq!(p.filters.max_size, Some(500 * 1024 - 1));

        let p = ParsedQuery::parse("modified:7d backup");
        assert!(p.filters.modified_after.is_some());
        assert_eq!(p.terms, vec!["backup"]);

        let p = ParsedQuery::parse("kind:folder projects");
        assert_eq!(p.filters.kinds, vec![SourceKind::Folder]);
        assert_eq!(p.terms, vec!["projects"]);

        let p = ParsedQuery::parse(r#"in:"C:\Users\Me\Documents" report"#);
        assert_eq!(
            p.filters.under_path.as_deref(),
            Some(r"C:\Users\Me\Documents")
        );
        assert_eq!(p.terms, vec!["report"]);
    }

    #[test]
    fn unknown_prefixes_stay_as_terms() {
        let p = ParsedQuery::parse("foo:bar baz");
        assert_eq!(p.terms, vec!["foo:bar", "baz"]);
        assert_eq!(p.scope, SearchScope::All);
    }

    #[test]
    fn empty_query_is_empty() {
        assert!(ParsedQuery::parse("   ").is_empty());
        assert!(ParsedQuery::parse("").terms.is_empty());
        assert!(!ParsedQuery::parse("x").is_empty());
    }

    #[test]
    fn modified_absolute_date_is_parsed() {
        let p = ParsedQuery::parse("modified:>2024-01-01 old");
        assert!(p.filters.modified_before.is_some());
        assert_eq!(p.terms, vec!["old"]);
    }
}
