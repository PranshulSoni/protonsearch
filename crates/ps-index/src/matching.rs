//! Query planning, prefiltering and match scoring for the filename index.
//! OWNER: engine-core.
//!
//! Everything here is allocation-free on the hot path except the few
//! candidate strings that are matched through the Unicode case-folding
//! fallback (non-ASCII only). ASCII comparisons are folded in place.

use std::cmp::Ordering;

/// Fixed-point scale used so ranking keys are exactly comparable.
pub(crate) const SCORE_SCALE: f32 = 1_000_000.0;

// ---------------------------------------------------------------------------
// Case-insensitive primitives
// ---------------------------------------------------------------------------

#[inline]
fn fold_byte(b: u8) -> u8 {
    b.to_ascii_lowercase()
}

#[inline]
fn char_eq_ci(a: char, b: char) -> bool {
    if a == b {
        return true;
    }
    if a.is_ascii() && b.is_ascii() {
        return a.eq_ignore_ascii_case(&b);
    }
    // Single-character lowercase fold (covers Latin/Greek/Cyrillic). Multi-char
    // expansions (e.g. German sharp s) are intentionally not supported: they
    // would change byte offsets used for highlight ranges.
    let mut al = a.to_lowercase();
    let mut bl = b.to_lowercase();
    al.next() == bl.next() && al.next().is_none() && bl.next().is_none()
}

#[inline]
fn char_fold(c: char) -> char {
    if c.is_ascii() {
        c.to_ascii_lowercase()
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

/// Case-insensitive equality of two UTF-8 strings (ASCII folded, non-ASCII
/// folded char-by-char).
pub(crate) fn eq_ci(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if a.len() != b.len() {
        // Byte lengths can differ only through non-ASCII folds; fall through to
        // the char comparison in that (rare) case.
        if a.is_ascii() || b.is_ascii() {
            return false;
        }
    }
    let mut ai = a.chars();
    let mut bi = b.chars();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) if char_eq_ci(x, y) => {}
            _ => return false,
        }
    }
}

/// ASCII-insensitive byte-slice equality (non-ASCII bytes compared exactly).
#[inline]
pub(crate) fn eq_ci_bytes(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| fold_byte(*x) == fold_byte(*y))
}

/// Does hay start with needle case-insensitively?
pub(crate) fn starts_with_ci(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if hay.len() < needle.len() {
        return false;
    }
    let hb = &hay.as_bytes()[..needle.len()];
    let nb = needle.as_bytes();
    if eq_ci_bytes(hb, nb) {
        return true;
    }
    // Non-ASCII case fold can change byte lengths; fall back to char walk.
    if !hay.is_ascii() || !needle.is_ascii() {
        let mut hc = hay.chars();
        for nc in needle.chars() {
            match hc.next() {
                Some(h) if char_eq_ci(h, nc) => {}
                _ => return false,
            }
        }
        return true;
    }
    false
}

/// Find needle (expected lower-case) inside hay, case-insensitively.
/// Returns the byte range in hay. ASCII fast path uses memchr on the first
/// byte; a char-fold fallback handles non-ASCII case differences.
pub(crate) fn find_ci(hay: &str, needle: &str) -> Option<(usize, usize)> {
    find_ci_from(hay, needle, 0)
}

pub(crate) fn find_ci_from(hay: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    if needle.is_empty() {
        let at = from.min(hay.len());
        return Some((at, at));
    }
    let h = hay.as_bytes();
    let n = needle.as_bytes();
    if from >= h.len() || n.len() > h.len() - from {
        return None;
    }
    let first = n[0];
    let first_upper = first.to_ascii_uppercase();
    let mut start = from;
    while start + n.len() <= h.len() {
        let rel = if first_upper != first {
            memchr::memchr2(first, first_upper, &h[start..])
        } else {
            memchr::memchr(first, &h[start..])
        };
        let Some(rel) = rel else {
            break;
        };
        let pos = start + rel;
        if pos + n.len() > h.len() {
            return None;
        }
        if eq_ci_bytes(&h[pos..pos + n.len()], n) {
            return Some((pos, pos + n.len()));
        }
        start = pos + 1;
    }
    // Only non-ASCII case differences can still match here.
    if !needle.is_ascii() && !hay.is_ascii() {
        return unicode_ci_find(hay, needle, from);
    }
    None
}

fn unicode_ci_find(hay: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    let needle_chars: Vec<char> = needle.chars().collect();
    if needle_chars.is_empty() {
        return Some((from, from));
    }
    let starts: Vec<(usize, char)> = hay
        .char_indices()
        .filter(|(i, _)| *i >= from)
        .collect();
    let mut idx = 0;
    while idx < starts.len() {
        if char_eq_ci(starts[idx].1, needle_chars[0]) {
            let mut hi = idx;
            let mut ni = 0;
            let mut end = starts[idx].0;
            while hi < starts.len() && ni < needle_chars.len() {
                if char_eq_ci(starts[hi].1, needle_chars[ni]) {
                    end = starts[hi].0 + starts[hi].1.len_utf8();
                    ni += 1;
                }
                hi += 1;
            }
            if ni == needle_chars.len() {
                return Some((starts[idx].0, end));
            }
        }
        idx += 1;
    }
    None
}

/// Compare two strings case-insensitively (ASCII folding) for the sorted-name
/// index. Ordering is total and deterministic; non-ASCII bytes compare exactly.
pub(crate) fn cmp_ci(a: &str, b: &str) -> Ordering {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let n = ab.len().min(bb.len());
    for i in 0..n {
        let x = fold_byte(ab[i]);
        let y = fold_byte(bb[i]);
        if x != y {
            return x.cmp(&y);
        }
    }
    ab.len().cmp(&bb.len())
}

// ---------------------------------------------------------------------------
// Bloom prefilter
// ---------------------------------------------------------------------------

/// Add one character to an entry bloom word.
///
/// ASCII letters/digits get a dedicated bit (a-z: 0..26, 0-9: 26..36) so a
/// membership test is exact. Every other character (space, dot, CJK, ...) is
/// hashed into the upper 28 bits with two bits per character, which keeps the
/// filter correct (never a false negative) for arbitrary Unicode names.
#[inline]
pub(crate) fn bloom_add(bloom: &mut u64, c: char) {
    if c.is_ascii_alphanumeric() {
        let lc = c.to_ascii_lowercase() as u8;
        let bit = if lc.is_ascii_lowercase() {
            (lc - b'a') as u32
        } else {
            26 + (lc - b'0') as u32
        };
        *bloom |= 1u64 << bit;
    } else {
        let h = char_hash(c);
        *bloom |= 1u64 << (36 + (h % 28) as u32);
        *bloom |= 1u64 << (36 + ((h >> 8) % 28) as u32);
    }
}

#[inline]
fn char_hash(c: char) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ (c as u32 as u64);
    h = h.wrapping_mul(0x0000_0100_0000_01b3);
    h ^= h >> 29;
    h
}

/// A parsed, precomputed query used by the scan path.
#[derive(Debug, Clone)]
pub(crate) struct QueryPlan {
    /// Lower-cased query text (used for every comparison).
    pub lower: String,
    /// Lower-cased query characters (fuzzy matching).
    pub chars: Vec<char>,
    /// Bits required from ASCII letters/digits (one bit per distinct char).
    pub bloom_exact: u64,
    /// Bits required from hashed non-alphanumeric characters.
    pub bloom_hash: u64,
    /// How many exact bits may be missing (allows single-char typos in fuzzy
    /// queries without losing recall for normal queries).
    pub allowed_missing: u32,
    /// Query contains a path separator -> spanning-path mode.
    pub has_sep: bool,
}

impl QueryPlan {
    pub fn new(raw: &str) -> Self {
        let trimmed = raw.trim();
        let lower = trimmed.to_lowercase();
        let mut bloom_exact = 0u64;
        let mut bloom_hash = 0u64;
        let chars: Vec<char> = lower.chars().collect();
        let mut seen = std::collections::HashSet::new();
        for c in lower.chars() {
            if !seen.insert(c) {
                continue;
            }
            if c.is_ascii_alphanumeric() {
                let lc = c.to_ascii_lowercase() as u8;
                let bit = if lc.is_ascii_lowercase() {
                    (lc - b'a') as u32
                } else {
                    26 + (lc - b'0') as u32
                };
                bloom_exact |= 1u64 << bit;
            } else {
                let h = char_hash(c);
                bloom_hash |= 1u64 << (36 + (h % 28) as u32);
                bloom_hash |= 1u64 << (36 + ((h >> 8) % 28) as u32);
            }
        }
        let n = chars.len();
        let allowed_missing = if n >= 5 { 1 } else { 0 };
        let has_sep = lower.contains('/') || lower.contains('\\');
        Self {
            lower,
            chars,
            bloom_exact,
            bloom_hash,
            allowed_missing,
            has_sep,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lower.is_empty()
    }

    /// Cheap candidate prefilter. Never produces false negatives for
    /// substring/subsequence matches (ASCII chars are exact bits; other chars
    /// use a 2-bit hash bloom); allowed_missing relaxes it by at most one
    /// character to keep single-typo fuzzy recall.
    #[inline]
    pub fn bloom_pass(&self, bloom: u64) -> bool {
        if bloom & self.bloom_hash != self.bloom_hash {
            return false;
        }
        let missing = (self.bloom_exact & !bloom).count_ones();
        missing <= self.allowed_missing
    }
}

// ---------------------------------------------------------------------------
// Match scoring
// ---------------------------------------------------------------------------

/// Score for each match class. Ordering is part of the product contract:
/// exact name > exact stem > prefix > word-boundary prefix > substring >
/// path segment > fuzzy subsequence > typo.
pub(crate) const SCORE_EXACT_NAME: f32 = 1.00;
pub(crate) const SCORE_EXACT_STEM: f32 = 0.97;
pub(crate) const SCORE_PREFIX: f32 = 0.94;
pub(crate) const SCORE_WORD_PREFIX: f32 = 0.90;
pub(crate) const SCORE_SUBSTRING: f32 = 0.85;
pub(crate) const SCORE_PATH_SEGMENT: f32 = 0.80;
pub(crate) const SCORE_PATH_PREFIX: f32 = 0.78;
pub(crate) const SCORE_PATH_SUBSTRING: f32 = 0.76;
pub(crate) const SCORE_SPANNING: f32 = 0.83;
pub(crate) const SCORE_FUZZY_BASE: f32 = 0.60;
pub(crate) const SCORE_FUZZY_SPAN: f32 = 0.14;
pub(crate) const SCORE_TYPO: f32 = 0.55;
/// Highest score the path phase can produce; used to skip it when the name
/// phase already filled the top-k above this value.
pub(crate) const PATH_MAX_SCORE: f32 = SCORE_PATH_SEGMENT;

#[derive(Debug, Clone)]
pub(crate) struct NameMatch {
    pub score: f32,
    /// Byte ranges inside the name that matched.
    pub byte_ranges: Vec<(usize, usize)>,
}

#[inline]
fn is_word_boundary(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '-' | '_' | '.' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | '+' | '&' | '#' | '@' | '~'
    )
}

fn boundary_before(s: &str, pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    match s[..pos].chars().next_back() {
        Some(c) => is_word_boundary(c) || c == '/' || c == '\\',
        None => true,
    }
}

/// Path-segment classification for a directory path: 3 = whole segment,
/// 2 = segment prefix, 1 = substring somewhere, 0 = no match.
pub(crate) fn segment_level(path: &str, needle: &str) -> u8 {
    if needle.is_empty() || path.is_empty() {
        return 0;
    }
    let mut best = 0u8;
    let mut from = 0usize;
    let mut attempts = 0;
    while attempts < 12 {
        attempts += 1;
        let Some((start, end)) = find_ci_from(path, needle, from) else {
            break;
        };
        let before = boundary_before(path, start);
        let after = end == path.len()
            || path[end..]
                .chars()
                .next()
                .map(|c| c == '/' || c == '\\')
                .unwrap_or(true);
        let level = if before && after {
            3
        } else if before {
            2
        } else {
            1
        };
        if level > best {
            best = level;
        }
        if best == 3 {
            break;
        }
        from = start + 1;
    }
    best
}

pub(crate) fn score_for_segment(level: u8) -> f32 {
    match level {
        3 => SCORE_PATH_SEGMENT,
        2 => SCORE_PATH_PREFIX,
        1 => SCORE_PATH_SUBSTRING,
        _ => 0.0,
    }
}

/// Full name match evaluation. Returns the best match class and its byte
/// ranges (inside name). stem is the name without its extension.
pub(crate) fn match_name(name: &str, stem: &str, plan: &QueryPlan) -> Option<NameMatch> {
    let q = &plan.lower;
    if q.is_empty() || name.is_empty() {
        return None;
    }
    // 1. exact name
    if eq_ci(name, q) {
        return Some(NameMatch {
            score: SCORE_EXACT_NAME,
            byte_ranges: vec![(0, name.len())],
        });
    }
    // 2. exact stem (only when the entry actually has an extension)
    if stem.len() != name.len() && eq_ci(stem, q) {
        return Some(NameMatch {
            score: SCORE_EXACT_STEM,
            byte_ranges: vec![(0, stem.len())],
        });
    }
    // 3. prefix
    if starts_with_ci(name, q) {
        let end = prefix_match_len(name, q);
        return Some(NameMatch {
            score: SCORE_PREFIX,
            byte_ranges: vec![(0, end)],
        });
    }
    // 4/5. substring; prefer a word-boundary occurrence when one exists.
    let mut from = 0usize;
    let mut found: Option<(usize, usize, bool)> = None;
    let mut attempts = 0;
    while attempts < 8 {
        attempts += 1;
        let Some((start, end)) = find_ci_from(name, q, from) else {
            break;
        };
        let boundary = boundary_before(name, start);
        if boundary {
            found = Some((start, end, true));
            break;
        }
        if found.is_none() {
            found = Some((start, end, false));
        }
        from = start + 1;
    }
    if let Some((start, end, boundary)) = found {
        return Some(NameMatch {
            score: if boundary { SCORE_WORD_PREFIX } else { SCORE_SUBSTRING },
            byte_ranges: vec![(start, end)],
        });
    }
    // 6. fuzzy subsequence
    if let Some((score, ranges)) = fuzzy_match(name, &plan.chars) {
        return Some(NameMatch {
            score,
            byte_ranges: ranges,
        });
    }
    // 7. single-typo (Damerau-Levenshtein distance <= 1) against stem/name
    if typo_match(name, stem, plan) {
        return Some(NameMatch {
            score: SCORE_TYPO,
            byte_ranges: Vec::new(),
        });
    }
    None
}

fn prefix_match_len(name: &str, q: &str) -> usize {
    // For ASCII-folded matches the byte length is the query byte length. For
    // non-ASCII folds walk chars to get the true end offset.
    if eq_ci_bytes(&name.as_bytes()[..q.len().min(name.len())], q.as_bytes()) {
        return q.len();
    }
    let mut consumed = 0usize;
    let mut qi = q.chars();
    for (bi, c) in name.char_indices() {
        match qi.next() {
            Some(qc) if char_eq_ci(c, qc) => consumed = bi + c.len_utf8(),
            _ => break,
        }
    }
    consumed
}

/// Greedy subsequence match with a quality score based on compactness and
/// consecutive runs. q is lower-cased query chars.
pub(crate) fn fuzzy_match(name: &str, q: &[char]) -> Option<(f32, Vec<(usize, usize)>)> {
    if q.is_empty() {
        return None;
    }
    let mut qi = 0usize;
    let mut first: Option<usize> = None;
    let mut last_end = 0usize;
    let mut prev_end: Option<usize> = None;
    let mut consecutive = 0usize;
    let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(q.len());
    for (bi, c) in name.char_indices() {
        if qi < q.len() && char_eq_ci(c, q[qi]) {
            if first.is_none() {
                first = Some(bi);
            }
            if prev_end == Some(bi) {
                consecutive += 1;
            }
            prev_end = Some(bi + c.len_utf8());
            last_end = bi + c.len_utf8();
            ranges.push((bi, bi + c.len_utf8()));
            qi += 1;
            if qi == q.len() {
                break;
            }
        }
    }
    if qi < q.len() {
        return None;
    }
    let first = first?;
    let span_chars = name[first..last_end].chars().count().max(1);
    let span_quality = (q.len() as f32 / span_chars as f32).clamp(0.0, 1.0);
    let run_quality = (consecutive as f32 / q.len() as f32).clamp(0.0, 1.0);
    let quality = 0.5 * span_quality + 0.5 * run_quality;
    Some((SCORE_FUZZY_BASE + SCORE_FUZZY_SPAN * quality, ranges))
}

/// Bounded Damerau-Levenshtein <= 1 check on folded characters.
pub(crate) fn damerau_le1(a: &[char], b: &[char]) -> bool {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    match long.len().checked_sub(short.len()) {
        Some(0) => {
            // equal length: one substitution or one adjacent transposition
            let mut diff: Vec<usize> = Vec::new();
            for i in 0..long.len() {
                if long[i] != short[i] {
                    diff.push(i);
                    if diff.len() > 2 {
                        return false;
                    }
                }
            }
            match diff.len() {
                0 | 1 => true,
                2 => {
                    let (i, j) = (diff[0], diff[1]);
                    j == i + 1 && long[i] == short[j] && long[j] == short[i]
                }
                _ => false,
            }
        }
        Some(1) => {
            // one insertion/deletion
            let mut i = 0usize;
            let mut j = 0usize;
            let mut skipped = false;
            while i < long.len() && j < short.len() {
                if long[i] == short[j] {
                    i += 1;
                    j += 1;
                } else if skipped {
                    return false;
                } else {
                    skipped = true;
                    i += 1;
                }
            }
            true
        }
        _ => false,
    }
}

fn typo_match(name: &str, stem: &str, plan: &QueryPlan) -> bool {
    let q = &plan.chars;
    if q.len() < 4 {
        return false;
    }
    let first = q[0];
    let candidates: [&str; 2] = if stem.len() == name.len() {
        [stem, ""]
    } else {
        [stem, name]
    };
    for cand in candidates {
        if cand.is_empty() {
            continue;
        }
        let cand_chars: Vec<char> = cand.chars().map(char_fold).collect();
        if cand_chars.len() + 2 < q.len() || cand_chars.len() > q.len() + 2 {
            continue;
        }
        let Some(&c0) = cand_chars.first() else {
            continue;
        };
        if c0 != first {
            continue;
        }
        if damerau_le1(&cand_chars, q) {
            return true;
        }
    }
    false
}

/// Convert byte ranges inside name into UTF-16 code-unit ranges (the UI's
/// JavaScript string model), merging adjacent ranges.
pub(crate) fn to_utf16_ranges(name: &str, byte_ranges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if byte_ranges.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<(usize, usize)> = Vec::with_capacity(byte_ranges.len());
    for &(start, end) in byte_ranges {
        let (s, e) = if name.is_ascii() {
            (start, end)
        } else {
            (
                name[..start.min(name.len())].encode_utf16().count(),
                name[..end.min(name.len())].encode_utf16().count(),
            )
        };
        if let Some(last) = out.last_mut() {
            if last.1 == s {
                last.1 = e;
                continue;
            }
        }
        out.push((s, e));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmp_ci_is_case_insensitive_and_total() {
        assert_eq!(cmp_ci("Report", "report"), Ordering::Equal);
        assert_eq!(cmp_ci("report", "reports"), Ordering::Less);
        assert_eq!(cmp_ci("Report2", "report1"), Ordering::Greater);
        assert_eq!(cmp_ci("报告", "报告"), Ordering::Equal);
    }

    #[test]
    fn bloom_never_rejects_contained_chars() {
        let mut b = 0u64;
        for c in "Report-2024_最终版.pdf".chars() {
            bloom_add(&mut b, c);
        }
        let plan = QueryPlan::new("report");
        assert!(plan.bloom_pass(b));
        let plan = QueryPlan::new("最终");
        assert!(plan.bloom_pass(b));
        let plan = QueryPlan::new("xyzq");
        assert!(!plan.bloom_pass(b));
    }

    #[test]
    fn find_ci_handles_ascii_unicode_and_long_paths() {
        assert_eq!(find_ci("Annual-Report.pdf", "report"), Some((7, 13)));
        assert_eq!(find_ci("Ünïcode", "ünï"), Some((0, 5)));
        assert_eq!(find_ci("報告書.pdf", "報告"), Some((0, 6)));
        assert_eq!(find_ci("abc", "abcd"), None);
    }

    #[test]
    fn match_classes_are_ordered() {
        let p = QueryPlan::new("report");
        let exact = match_name("report", "report", &p).unwrap();
        let stem = match_name("report.pdf", "report", &p).unwrap();
        let prefix = match_name("reports.txt", "reports", &p).unwrap();
        let word = match_name("annual-report.txt", "annual-report", &p).unwrap();
        let sub = match_name("myreport.txt", "myreport", &p).unwrap();
        let fuzzy = match_name("r_e_p_o_r_t.txt", "r_e_p_o_r_t", &p).unwrap();
        assert!(exact.score > stem.score);
        assert!(stem.score > prefix.score);
        assert!(prefix.score > word.score);
        assert!(word.score > sub.score);
        assert!(sub.score > fuzzy.score);
        assert_eq!(exact.byte_ranges, vec![(0, 6)]);
    }

    #[test]
    fn fuzzy_and_typo_matching() {
        let p = QueryPlan::new("rprt");
        assert!(fuzzy_match("report", &p.chars).is_some());
        // Transposition typo is caught by the bounded edit-distance fallback.
        let p = QueryPlan::new("reprot");
        let m = match_name("report.txt", "report", &p).unwrap();
        assert_eq!(m.score, SCORE_TYPO);
        let p = QueryPlan::new("recieve");
        let m = match_name("receive.txt", "receive", &p).unwrap();
        assert_eq!(m.score, SCORE_TYPO);
    }

    #[test]
    fn utf16_ranges_account_for_surrogates_and_cjk() {
        // A 4-byte emoji is 2 UTF-16 code units; a 3-byte CJK char is 1.
        let name = "😀report.txt";
        let ranges = to_utf16_ranges(name, &[(4, 10)]);
        assert_eq!(ranges, vec![(2, 8)]);
        let name = "报告final.txt";
        let ranges = to_utf16_ranges(name, &[(6, 11)]);
        assert_eq!(ranges, vec![(2, 7)]);
    }
}
