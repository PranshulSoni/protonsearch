import io
p = r'crates/ps-core/src/engine.rs'
s = io.open(p, encoding='utf-8').read()

# 1) Use the engine-core parser for scope/filters/filename_query, and ps-index filters.
old = '''        let settings = self.settings_snapshot();
        let (scope, query_text) = effective_query(&req.query, req.scope);
        let parsed = crate::query::ParsedQuery::parse(&query_text);
        let terms = parsed.terms.clone();'''
new = '''        let settings = self.settings_snapshot();
        let (prefix_scope, query_text) = effective_query(&req.query, req.scope);
        let parsed = crate::query::ParsedQuery::parse(&query_text);
        let scope = if parsed.scope != SearchScope::All {
            parsed.scope
        } else {
            prefix_scope
        };
        let filters = merged_filters(&parsed.filters, &req.filters);
        let filename_query = if parsed.filename_query.trim().is_empty() {
            query_text.clone()
        } else {
            parsed.filename_query.clone()
        };
        let terms = parsed.terms.clone();'''
assert s.count(old) == 1
s = s.replace(old, new)

# 2) Filename tier: filtered search via ps-index EntryFilter.
old = '''        // Tier 1: filename index.
        if scope_allows(scope, SourceKind::File) || scope_allows(scope, SourceKind::Folder) {
            let tier_start = Instant::now();
            let file_hits = {
                let index = self.index.read().unwrap();
                index.search(&query_text, limit.saturating_mul(2))
            };'''
new = '''        // Tier 1: filename index.
        if scope_allows(scope, SourceKind::File) || scope_allows(scope, SourceKind::Folder) {
            let tier_start = Instant::now();
            let extensions: Vec<String> = filters
                .extensions
                .iter()
                .map(|ext| ext.trim_start_matches('.').to_lowercase())
                .collect();
            let kind_filter = match (filters.kinds.contains(&SourceKind::Folder), filters.kinds.contains(&SourceKind::File)) {
                (true, false) => Some(EntryKind::Folder),
                (false, true) => Some(EntryKind::File),
                _ => None,
            };
            let entry_filter = ps_index::EntryFilter {
                kind: kind_filter,
                extensions: &extensions,
                under_path: filters.under_path.as_deref(),
                modified_after: filters.modified_after,
                modified_before: filters.modified_before,
                min_size: filters.min_size,
                max_size: filters.max_size,
            };
            let file_hits = {
                let index = self.index.read().unwrap();
                index.search_filtered(&filename_query, limit.saturating_mul(2), &entry_filter)
            };'''
assert s.count(old) == 1
s = s.replace(old, new)

# 3) finalize via engine-core's deterministic ranker.
old = '''fn finalize(ranked: Vec<Ranked>, limit: usize) -> Vec<SearchResult> {
    let mut items = ranked;
    items.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then(a.priority.cmp(&b.priority))
            .then(b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.result.id.cmp(&b.result.id))
    });
    let mut seen: HashSet<String> = HashSet::new();
    let mut results = Vec::with_capacity(limit.min(items.len()));
    for item in items {
        if !seen.insert(item.result.id.clone()) {
            continue;
        }
        results.push(item.result);
        if results.len() >= limit {
            break;
        }
    }
    results
}'''
new = '''/// Delegate ordering, dedupe and truncation to the shared ranker (engine-core)
/// so every tier obeys the same deterministic contract.
fn finalize(ranked: Vec<Ranked>, limit: usize) -> Vec<SearchResult> {
    let items: Vec<crate::rank::RankedResult> = ranked
        .into_iter()
        .map(|item| crate::rank::RankedResult {
            tier: item.tier,
            score: item.score,
            priority: item.priority,
            result: item.result,
        })
        .collect();
    crate::rank::merge_results(items, limit)
}

/// Merge UI-supplied filters with filters parsed from the query text; both are
/// restrictive, so a field present in either applies.
fn merged_filters(parsed: &SearchFilters, request: &SearchFilters) -> SearchFilters {
    let mut merged = request.clone();
    if !parsed.kinds.is_empty() {
        merged.kinds = parsed.kinds.clone();
    }
    if !parsed.extensions.is_empty() {
        merged.extensions = parsed.extensions.clone();
    }
    if parsed.under_path.is_some() {
        merged.under_path = parsed.under_path.clone();
    }
    merged.modified_after = parsed.modified_after.or(request.modified_after);
    merged.modified_before = parsed.modified_before.or(request.modified_before);
    merged.min_size = parsed.min_size.or(request.min_size);
    merged.max_size = parsed.max_size.or(request.max_size);
    merged
}'''
assert s.count(old) == 1
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('engine wired to parser+ranker+filters')
