//! Latency distribution helpers. Nearest-rank percentiles over raw samples.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub count: usize,
    pub min: f64,
    pub mean: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

/// Nearest-rank percentile over an already sorted slice.
pub fn percentile_sorted(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let pct = pct.clamp(0.0, 100.0);
    let n = sorted.len() as f64;
    let rank = (pct / 100.0 * n).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

pub fn summarize(samples: &[f64]) -> Summary {
    if samples.is_empty() {
        return Summary {
            count: 0,
            min: 0.0,
            mean: 0.0,
            p50: 0.0,
            p90: 0.0,
            p95: 0.0,
            p99: 0.0,
            max: 0.0,
        };
    }
    let mut v = samples.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sum: f64 = v.iter().sum();
    Summary {
        count: v.len(),
        min: v[0],
        mean: sum / v.len() as f64,
        p50: percentile_sorted(&v, 50.0),
        p90: percentile_sorted(&v, 90.0),
        p95: percentile_sorted(&v, 95.0),
        p99: percentile_sorted(&v, 99.0),
        max: v[v.len() - 1],
    }
}

/// Aggregate several sample vectors into one distribution.
pub fn summarize_slices(slices: &[Vec<f64>]) -> Summary {
    let all: Vec<f64> = slices.iter().flat_map(|s| s.iter().copied()).collect();
    summarize(&all)
}
