//! Ranking merge. OWNER: engine-core.
//!
//! Deterministic blend of: exact/prefix filename match, path segment match,
//! content BM25, recency, launch/usage frequency, and intent boosts. Semantic
//! results can never outrank an exact match at tier 0.

/// One candidate before ordering. Sources produce these; the coordinator merges.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredHit {
    pub score: f32,
    pub tier: u8,
    /// Higher tie-break priority, lower wins.
    pub tie_break: u32,
}

/// Merge and order hits deterministically (stable regardless of thread timing).
pub fn merge(_hits: Vec<ScoredHit>) -> Vec<ScoredHit> {
    Vec::new()
}
