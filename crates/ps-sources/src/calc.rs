//! In-process calculator, unit conversion and percentage evaluation.
//! OWNER: sources-content. No eval crates; recursive descent parser.

#[derive(Debug, Clone, PartialEq)]
pub struct CalcResult {
    pub value: f64,
    pub formatted: String,
    pub label: String,
}

/// Evaluate an arithmetic expression like "2+2", "15% of 340", "sqrt(9)*4".
pub fn evaluate(_query: &str) -> Option<CalcResult> {
    None
}

/// Convert units like "5 km to miles".
pub fn convert(_query: &str) -> Option<CalcResult> {
    None
}
