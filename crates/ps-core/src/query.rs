//! Query parsing: explicit prefixes, intent detection, filters. OWNER: engine-core.

use ps_ipc::{SearchFilters, SearchScope};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedQuery {
    pub raw: String,
    pub terms: Vec<String>,
    pub scope: SearchScope,
    pub filters: SearchFilters,
    /// Calculator intent, if the query looks like math.
    pub is_math: bool,
}

impl ParsedQuery {
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        Self {
            raw: trimmed.to_string(),
            terms: trimmed
                .split_whitespace()
                .map(|t| t.to_lowercase())
                .collect(),
            scope: SearchScope::All,
            filters: SearchFilters::default(),
            is_math: false,
        }
    }
}
