//! Ranking merge. OWNER: engine-core.
//!
//! Deterministic blend of: exact/prefix filename match, path segment match,
//! content BM25, recency, launch/usage frequency, and intent boosts. Semantic
//! results can never outrank an exact match at tier 0.
//!
//! The Engine finalizes through merge_results, whose ordering is the
//! contract: tier ascending, then source priority ascending, then score
//! descending, then `order` ascending, then result id ascending. For the
//! All / Files / Folders / Apps filters the tier is the relevance class of the
//! name match (see `relevance_class`), the priority is the category (Apps,
//! Folders, Files, documents/OCR, other sources) and the score carries the
//! history signal, so relevance always comes first and category only orders
//! comparable matches.

use ps_ipc::{SearchResult, SearchScope, SourceKind};

/// One candidate before ordering. Sources produce these; the coordinator merges.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredHit {
    pub score: f32,
    pub tier: u8,
    /// Higher tie-break priority, lower wins.
    pub tie_break: u32,
}

/// Merge and order hits deterministically (stable regardless of thread timing).
pub fn merge(hits: Vec<ScoredHit>) -> Vec<ScoredHit> {
    let mut hits = hits;
    hits.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.tie_break.cmp(&b.tie_break))
    });
    hits
}

/// Tier constants used by the Engine coordinator.
pub const TIER_FILES_EXACT: u8 = 0;
pub const TIER_FILES_SCAN: u8 = 1;
pub const TIER_CONTENT: u8 = 1;
pub const TIER_SEMANTIC: u8 = 2;

/// A fully-built result plus its ranking metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedResult {
    pub tier: u8,
    pub score: f32,
    pub priority: u8,
    /// Final per-source tie-break (lower first) before the id.
    pub order: u32,
    pub result: SearchResult,
}

/// How a row's name matched the query, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchKind {
    ExactName,
    ExactStem,
    ExactAlias,
    ExactToken,
    Prefix,
    WordPrefix,
    Substring,
    StrongFuzzy,
    WeakFuzzy,
    /// Matched only inside the document text or OCR text, not the name.
    ContentOnly,
}

impl From<ps_index::NameMatch> for MatchKind {
    fn from(kind: ps_index::NameMatch) -> Self {
        use ps_index::NameMatch as N;
        match kind {
            N::ExactName => Self::ExactName,
            N::ExactStem => Self::ExactStem,
            N::ExactToken => Self::ExactToken,
            N::Prefix => Self::Prefix,
            N::WordPrefix => Self::WordPrefix,
            N::Substring => Self::Substring,
            N::StrongFuzzy => Self::StrongFuzzy,
            N::WeakFuzzy => Self::WeakFuzzy,
        }
    }
}

impl From<crate::appmatch::AppMatch> for MatchKind {
    fn from(kind: crate::appmatch::AppMatch) -> Self {
        use crate::appmatch::AppMatch as A;
        match kind {
            A::ExactName => Self::ExactName,
            A::ExactAlias => Self::ExactAlias,
            A::Prefix => Self::Prefix,
            A::ExactToken => Self::ExactToken,
            A::WordPrefix => Self::WordPrefix,
            A::Substring => Self::Substring,
            A::StrongFuzzy => Self::StrongFuzzy,
            A::WeakFuzzy => Self::WeakFuzzy,
        }
    }
}

/// Relevance class (lower first) of a match in a filter, following each
/// filter's own ladder:
/// - All: exact name; exact token; prefix; ordered word prefix; substring;
///   strong fuzzy; weak fuzzy; content/OCR-only. An app's exact alias (the
///   program it launches, e.g. "chrome", or a well-known short name such as
///   "vscode") is its name here, so an installed app is not buried under files
///   that merely share that name.
/// - Apps: exact name; exact alias; prefix; exact token; ordered word prefix;
///   substring; strong fuzzy; weak fuzzy.
/// - Files / Folders: exact full name; exact stem; exact token; prefix;
///   ordered word prefix; substring; strong fuzzy; weak fuzzy.
pub fn relevance_class(kind: MatchKind, scope: SearchScope) -> u8 {
    use MatchKind::*;
    match scope {
        SearchScope::All => match kind {
            ExactName | ExactStem | ExactAlias => 0,
            ExactToken => 1,
            Prefix => 2,
            WordPrefix => 3,
            Substring => 4,
            StrongFuzzy => 5,
            WeakFuzzy => 6,
            ContentOnly => 7,
        },
        SearchScope::Apps => match kind {
            ExactName => 0,
            ExactAlias | ExactStem => 1,
            Prefix => 2,
            ExactToken => 3,
            WordPrefix => 4,
            Substring => 5,
            StrongFuzzy => 6,
            WeakFuzzy => 7,
            ContentOnly => 8,
        },
        _ => match kind {
            ExactName => 0,
            ExactStem | ExactAlias => 1,
            ExactToken => 2,
            Prefix => 3,
            WordPrefix => 4,
            Substring => 5,
            StrongFuzzy => 6,
            WeakFuzzy => 7,
            ContentOnly => 8,
        },
    }
}

/// After every real match: the "search the web" suggestion.
pub const CLASS_FALLBACK: u8 = 9;

/// Category order inside one relevance class.
pub const CATEGORY_CALCULATOR: u8 = 0;
pub const CATEGORY_APP: u8 = 1;
pub const CATEGORY_FOLDER: u8 = 2;
pub const CATEGORY_FILE: u8 = 3;
pub const CATEGORY_CONTENT: u8 = 4;
pub const CATEGORY_ACTION: u8 = 5;
pub const CATEGORY_BROWSER: u8 = 6;
pub const CATEGORY_GIT: u8 = 7;
pub const CATEGORY_CLIPBOARD: u8 = 8;
pub const CATEGORY_WEB: u8 = 9;

/// History signal for something opened or launched `count` times, most
/// recently at `last_ms`: frequency on a log scale plus a recency step, in
/// whole steps so near-equal items keep their stable order. 0 = never used.
pub fn usage_signal(count: u32, last_ms: i64, now_ms: i64) -> f32 {
    if count == 0 {
        return 0.0;
    }
    let frequency = (count.min(10_000) as f32 + 1.0).log2();
    let age_days = (now_ms - last_ms).max(0) as f32 / 86_400_000.0;
    let recency = if age_days < 1.0 {
        4.0
    } else if age_days < 7.0 {
        3.0
    } else if age_days < 30.0 {
        2.0
    } else if age_days < 180.0 {
        1.0
    } else {
        0.0
    };
    (frequency + recency).floor()
}

/// Rows describing the same file or folder (a filename hit, a document-text
/// hit, an OCR hit, a recent entry) collapse into one.
fn dedupe_key(result: &SearchResult) -> String {
    match (result.kind, result.path.as_deref()) {
        (
            SourceKind::File
            | SourceKind::Folder
            | SourceKind::Recent
            | SourceKind::Document
            | SourceKind::ContentMatch
            | SourceKind::OcrText,
            Some(path),
        ) => format!("path:{}", path.to_lowercase()),
        _ => result.id.clone(),
    }
}

/// Stable source priority for ties (lower wins). Mirrors the launcher's visual
/// grouping: calculator/actions at the top, machine-generated sources last.
pub fn priority_for(kind: SourceKind) -> u8 {
    match kind {
        SourceKind::Calculator => 0,
        SourceKind::Quicklink | SourceKind::Snippet => 1,
        SourceKind::Application => 2,
        SourceKind::File | SourceKind::Folder => 3,
        SourceKind::Recent => 4,
        SourceKind::Action | SourceKind::Command => 5,
        SourceKind::BrowserHistory | SourceKind::Bookmark => 6,
        SourceKind::GitCommit | SourceKind::GitRepository | SourceKind::GitTodo => 7,
        SourceKind::Clipboard => 8,
        SourceKind::Setting => 9,
        SourceKind::Document | SourceKind::ContentMatch | SourceKind::OcrText => 10,
        SourceKind::Window => 11,
        SourceKind::Agent => 12,
        SourceKind::System => 13,
    }
}

/// Merge, de-duplicate (by result id, and by path for file-like rows, keeping
/// the best-ranked occurrence) and truncate to limit. Total and deterministic
/// for a given input multiset.
pub fn merge_results(items: Vec<RankedResult>, limit: usize) -> Vec<SearchResult> {
    let mut items = items;
    items.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| a.priority.cmp(&b.priority))
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| a.result.id.cmp(&b.result.id))
    });
    let mut out: Vec<SearchResult> = Vec::with_capacity(limit.min(items.len()));
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for item in items {
        if out.len() >= limit {
            break;
        }
        if !seen.insert(dedupe_key(&item.result)) {
            continue;
        }
        out.push(item.result);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_ipc::{ActionSpec, IconRef};

    fn result(id: &str, kind: SourceKind) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            kind,
            title: id.to_string(),
            subtitle: String::new(),
            path: None,
            icon: IconRef::Monogram,
            score: 0.0,
            match_ranges: Vec::new(),
            meta: Default::default(),
            action: ActionSpec::None,
            actions: Vec::new(),
        }
    }

    #[test]
    fn merge_is_deterministic_and_orders_by_tier_then_score() {
        let hits = vec![
            ScoredHit {
                score: 0.5,
                tier: 1,
                tie_break: 2,
            },
            ScoredHit {
                score: 0.9,
                tier: 0,
                tie_break: 3,
            },
            ScoredHit {
                score: 0.7,
                tier: 1,
                tie_break: 1,
            },
            ScoredHit {
                score: 0.9,
                tier: 0,
                tie_break: 1,
            },
        ];
        let a = merge(hits.clone());
        let b = merge(hits);
        assert_eq!(a, b);
        assert_eq!(a[0].tier, 0);
        assert_eq!(a[0].tie_break, 1, "equal scores use tie_break");
        assert_eq!(a[1].tie_break, 3);
        assert_eq!(a[2].tie_break, 1);
        assert_eq!(a[3].tie_break, 2);
    }

    #[test]
    fn merge_results_never_lets_semantic_outrank_exact() {
        let items = vec![
            RankedResult {
                tier: TIER_SEMANTIC,
                score: 1000.0,
                priority: priority_for(SourceKind::Agent),
                order: 0,
                result: result("semantic", SourceKind::Agent),
            },
            RankedResult {
                tier: TIER_FILES_EXACT,
                score: 1.0,
                priority: priority_for(SourceKind::File),
                order: 0,
                result: result("exact", SourceKind::File),
            },
        ];
        let out = merge_results(items, 10);
        assert_eq!(out[0].id, "exact");
        assert_eq!(out[1].id, "semantic");
    }

    #[test]
    fn merge_results_dedupes_by_id_keeping_best() {
        let items = vec![
            RankedResult {
                tier: 0,
                score: 0.5,
                priority: 3,
                order: 0,
                result: result("same", SourceKind::File),
            },
            RankedResult {
                tier: 0,
                score: 0.9,
                priority: 3,
                order: 0,
                result: result("same", SourceKind::File),
            },
            RankedResult {
                tier: 0,
                score: 0.4,
                priority: 3,
                order: 0,
                result: result("other", SourceKind::File),
            },
        ];
        let out = merge_results(items, 10);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "same");
        assert_eq!(out[1].id, "other");
    }

    #[test]
    fn merge_results_is_deterministic_for_identical_inputs() {
        let items: Vec<RankedResult> = (0..20)
            .map(|i| RankedResult {
                tier: (i % 3) as u8,
                score: ((i * 37) % 11) as f32 / 10.0,
                priority: (i % 5) as u8,
                order: 0,
                result: result(&format!("id-{i}"), SourceKind::File),
            })
            .collect();
        let a = merge_results(items.clone(), 12);
        let b = merge_results(items, 12);
        assert_eq!(a, b);
        assert_eq!(a.len(), 12);
        // Scores must be non-increasing inside a tier.
        for w in a.windows(2) {
            assert!(w[0].score >= w[1].score - 1e-9);
        }
    }

    fn row(id: &str, kind: SourceKind, path: Option<&str>, tier: u8, priority: u8, score: f32) -> RankedResult {
        let mut result = result(id, kind);
        result.path = path.map(str::to_string);
        RankedResult {
            tier,
            score,
            priority,
            order: 0,
            result,
        }
    }

    #[test]
    fn relevance_class_comes_before_category_and_history() {
        // An exact filename beats a heavily used app that only matched by
        // substring; inside one class Apps come before Folders before Files.
        let out = merge_results(
            vec![
                row("app:weak", SourceKind::Application, None, 4, CATEGORY_APP, 9000.0),
                row("file:exact", SourceKind::File, Some(r"C:\a\budget.xlsx"), 0, CATEGORY_FILE, 0.0),
                row("folder:exact", SourceKind::Folder, Some(r"C:\a\budget"), 0, CATEGORY_FOLDER, 0.0),
                row("app:exact", SourceKind::Application, None, 0, CATEGORY_APP, 0.0),
                row("file:used", SourceKind::File, Some(r"C:\b\budget.xlsx"), 0, CATEGORY_FILE, 5000.0),
            ],
            10,
        );
        let ids: Vec<&str> = out.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["app:exact", "folder:exact", "file:used", "file:exact", "app:weak"]);
    }

    #[test]
    fn same_path_collapses_to_the_strongest_row() {
        let out = merge_results(
            vec![
                row("content:c:\\a\\notes.txt", SourceKind::ContentMatch, Some(r"C:\a\notes.txt"), 7, CATEGORY_CONTENT, 0.0),
                row("file:c:\\a\\notes.txt", SourceKind::File, Some(r"C:\a\notes.txt"), 0, CATEGORY_FILE, 0.0),
                row("ocr:c:\\a\\notes.txt", SourceKind::OcrText, Some(r"c:\A\Notes.txt"), 7, CATEGORY_CONTENT, 0.0),
                row("recent:c:\\a\\notes.txt", SourceKind::Recent, Some(r"C:\a\notes.txt"), 0, CATEGORY_FILE, 0.0),
            ],
            10,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, SourceKind::File);
    }

    #[test]
    fn filters_follow_their_own_ladders() {
        use MatchKind::*;
        // All: an app alias is as strong as an exact name; tokens beat prefixes.
        assert_eq!(relevance_class(ExactAlias, SearchScope::All), relevance_class(ExactName, SearchScope::All));
        assert!(relevance_class(ExactToken, SearchScope::All) < relevance_class(Prefix, SearchScope::All));
        // Apps: prefix ranks ahead of an exact token.
        assert!(relevance_class(Prefix, SearchScope::Apps) < relevance_class(ExactToken, SearchScope::Apps));
        // Files: full name, then stem, then token, then prefix.
        let files: Vec<u8> = [ExactName, ExactStem, ExactToken, Prefix, WordPrefix, Substring, StrongFuzzy, WeakFuzzy]
            .into_iter()
            .map(|kind| relevance_class(kind, SearchScope::Files))
            .collect();
        assert!(files.windows(2).all(|w| w[0] < w[1]), "{files:?}");
        // Content-only matches always trail name matches.
        assert!(relevance_class(WeakFuzzy, SearchScope::All) < relevance_class(ContentOnly, SearchScope::All));
        assert!(relevance_class(ContentOnly, SearchScope::All) < CLASS_FALLBACK);
    }

    #[test]
    fn usage_signal_rewards_frequency_and_recency() {
        let day = 86_400_000;
        let now = 1_000 * day;
        assert_eq!(usage_signal(0, now, now), 0.0);
        assert!(usage_signal(20, now - day / 2, now) > usage_signal(1, now - day / 2, now));
        assert!(usage_signal(3, now - day / 2, now) > usage_signal(3, now - 60 * day, now));
        assert_eq!(usage_signal(5, now, now), usage_signal(5, now, now));
    }

    #[test]
    fn limit_zero_returns_nothing() {
        let items = vec![RankedResult {
            tier: 0,
            score: 1.0,
            priority: 3,
            order: 0,
            result: result("x", SourceKind::File),
        }];
        assert!(merge_results(items, 0).is_empty());
    }
}
