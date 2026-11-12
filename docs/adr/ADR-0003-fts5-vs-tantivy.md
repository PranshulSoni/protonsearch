# ADR-0003: Content index — SQLite FTS5 for v2, Tantivy re-evaluated at scale

- Status: **Provisional** — decided for v2 alpha; must be re-confirmed by benchmark data (Gates 2/5)
- Owners: L1-C (benchmarks), L1-A (implementation), L0

## Context

Content search must cover mixed documents (PDF/DOCX/PPTX/XLSX/Markdown/plain text/code) at the scale of a real user profile. The legacy index contains 14,609 extracted documents in a 149 MB SQLite file. v2 needs: incremental updates, snippet/highlight with line numbers for open-at-match, crash-safe writes, small on-disk footprint, and p95 query latency under 30 ms.

Candidates:
- **SQLite FTS5**: single-file, transactional, external-content tables, mature snippet function, already proven in the legacy app, zero extra process. Writes are serialized; large bulk imports need batching.
- **Tantivy**: excellent query throughput and BM25, but a separate index directory, more complex incremental deletes/updates, and an additional dependency; best fit at very large corpora or when advanced phrase/field queries and facets dominate.

## Decision

Use **SQLite FTS5** (external content table) for v2 alpha. Keep the content layer behind a ContentIndex API so a Tantivy backend can be swapped if the measured corpus justifies it.

Decision criteria for revisiting (falsifiable):
- FTS5 p95 retrieval on the fixed mixed corpus exceeds 30 ms after query tuning; or
- Full rebuild/update throughput becomes the dominant indexing bottleneck at >100k documents; or
- Snippet/line-offset quality is insufficient for open-at-match.

## Consequences

- One database for filename metadata, content index, OCR text and activity -> simple backup/migration, lower RSS than a second engine.
- Must sanitize user input into FTS queries (quoting terms) to avoid FTS syntax errors/injection-like behavior.
- Must set WAL, busy_timeout, and batch transactions to avoid lock contention with the UI.
- Benchmark report will show FTS5 numbers on the real legacy corpus and the decision will be marked Confirmed or Superseded with evidence.
