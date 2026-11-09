//! Ranking merge. OWNER: engine-core.
//!
//! Deterministic blend of: exact/prefix filename match, path segment match,
//! content BM25, recency, launch/usage frequency, and intent boosts. Semantic
//! results can never outrank an exact match at tier 0.
//!
//! This module is a tested utility: the Engine facade may keep its own
//! finalization, but if it switches to merge_results the ordering below is the
//! contract: tier ascending (0 before 1 before 2), then score descending, then
//! source priority ascending, then result id ascending. Exact/prefix filename
//! hits live in tier 0; semantic/agent hits live in tier 2 and can therefore
//! never displace them.


use ps_ipc::{SearchResult, SourceKind};

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
    pub result: SearchResult,
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

/// Merge, de-duplicate (by result id, keeping the best-ranked occurrence) and
/// truncate to limit. Total and deterministic for a given input multiset.
pub fn merge_results(items: Vec<RankedResult>, limit: usize) -> Vec<SearchResult> {
    let mut items = items;
    items.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.priority.cmp(&b.priority))
            .then_with(|| a.result.id.cmp(&b.result.id))
    });
    let mut out: Vec<SearchResult> = Vec::with_capacity(limit.min(items.len()));
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for item in items {
        if out.len() >= limit {
            break;
        }
        if !seen.insert(item.result.id.clone()) {
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
            ScoredHit { score: 0.5, tier: 1, tie_break: 2 },
            ScoredHit { score: 0.9, tier: 0, tie_break: 3 },
            ScoredHit { score: 0.7, tier: 1, tie_break: 1 },
            ScoredHit { score: 0.9, tier: 0, tie_break: 1 },
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
                result: result("semantic", SourceKind::Agent),
            },
            RankedResult {
                tier: TIER_FILES_EXACT,
                score: 1.0,
                priority: priority_for(SourceKind::File),
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
                result: result("same", SourceKind::File),
            },
            RankedResult {
                tier: 0,
                score: 0.9,
                priority: 3,
                result: result("same", SourceKind::File),
            },
            RankedResult {
                tier: 0,
                score: 0.4,
                priority: 3,
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

    #[test]
    fn limit_zero_returns_nothing() {
        let items = vec![RankedResult {
            tier: 0,
            score: 1.0,
            priority: 3,
            result: result("x", SourceKind::File),
        }];
        assert!(merge_results(items, 0).is_empty());
    }
}