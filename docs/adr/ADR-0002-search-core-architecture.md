# ADR-0002: Search core — compact in-memory filename index + tiered coordinator

- Status: **Accepted** (Gate 1); performance targets provisional until Gate 2 benchmarks
- Owners: L1-A (engine), L1-C (perf), L0 (decision)

## Context

Requirements: exact/prefix/fuzzy filename search over 10k–1M entries with p95 in single-digit milliseconds; content and OCR retrieval through an index (never per-query inference); incremental maintenance; must work without admin rights and on non-NTFS volumes; must stay correct under renames/deletes and while indexing.

Legacy v1 stored filenames in SQLite and queried with LIKE/prefix heuristics. That is simple and crash-safe but adds per-query SQL overhead and cannot provide a bounded top-k fuzzy scan at 1M entries without full table scans.

## Decision

1. **Filename index is in-memory and compact**: one byte arena for strings (path + name, case-folded copy) and fixed-size entry records; a per-entry character-class/trigram bitmask for cheap candidate prefiltering; parallel chunked scan with rayon; bounded top-k via binary heap; deterministic tie-breaking. Persisted as a versioned binary file for warm startup, rebuilt atomically from a filesystem walk when missing/corrupt/roots-changed.
2. **Content/full-text uses SQLite FTS5** in the same v2 database (external-content table). Rationale and comparison with Tantivy in ADR-0003.
3. **OCR text is indexed text**, stored in FTS5 after a separate bounded extraction pipeline; search never triggers OCR.
4. **Tiered coordinator**: Tier 0 (exact/prefix names, apps, recents, actions, calculator, quicklinks) -> Tier 1 (fuzzy names + content FTS) -> Tier 2 (OCR text, optional semantic). Results are merged by a deterministic ranker; late tiers may add rows but never displace exact matches; stale request IDs are dropped.
5. **Windows-native acceleration is an optimization, not a requirement**: the first implementation uses walkdir-style traversal with reparse-point protection and exclusion rules that work on every filesystem. NTFS MFT enumeration and USN journal tracking are evaluated as later fast paths behind a capability check (admin-free when possible), with the same index semantics. This avoids an admin requirement and FAT/exFAT/network-drive failures.

## Consequences

- Memory grows with entry count (documented: ~100–150 MB RSS ceiling investigated for 1M entries; measured numbers published, not assumed).
- Warm startup requires loading the persisted index; cold startup falls back to progressive scan with partial results available early.
- Rename/move handling uses stable identity where the platform provides it; otherwise delete+add, which is correct but causes re-extraction unless content hashes match.
- The binary format is versioned and validated (magic/version/checksum); corrupt files trigger a rebuild instead of a panic.
