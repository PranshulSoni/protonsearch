# Migration fixture + contract test (Gate 0/G4 groundwork)

Status: **BLOCKED on the v2 migration API** (not invented). This directory contains a
deterministic synthetic legacy database, a read-only contract test, and a Rust test
skeleton documenting the exact API the engine must expose.

## Why a synthetic fixture
The real baseline DB lives at `%APPDATA%\protonsearch\file_index.db` (149 MB, user data).
Tests must never depend on, copy, or mutate it. The fixture reproduces the legacy schema
and representative synthetic rows only:

| Table | Rows in fixture | Notes |
|---|---:|---|
| `files` | 12 | mixed files/dirs, extensions, `last_seen_scan` |
| `files_fts` (FTS5) | 3 | `path UNINDEXED, content`, same schema as legacy |
| `browser_items` | 4 | chrome/brave/firefox history+bookmark sources |
| `quicklinks` | 2 | name/url/keyword |
| `snippets` | 1 | name/content/keyword |
| `ai_settings` | 2 | keys `endpoint`, `model` only — **no `api_key`** |
| `memory_events` | 5 | Browser/Visited Page + Git/Commit rows |
| `clipboard_history` | 0 | empty table; proves empty-source handling |
| `timeline_events` | 0 | empty table; proves empty-source handling |

All content is synthetic (no real paths, titles, usernames, or secrets). The generator is
deterministic and writes `fixture_manifest.json` with row counts + SHA-256.

## Commands
```powershell
# generate into a temp dir (never into the repo)
python tests/migration/make_legacy_fixture.py --out-dir <temp-dir>

# contract test: schema/rows + source never mutated + no WAL/journal created
agent-sidecar\.venv\Scripts\python.exe -m unittest discover -s tests/migration -p "test_*.py" -v
```

Environment limitation (recorded, not product behaviour): this sandbox's child processes
cannot write `%APPDATA%`. E2E/migration tests must set `PROTONSEARCH_DATA_DIR` to a temp
directory (implemented in `crates/ps-core/src/paths.rs:6-19`; v2 data then lives under
`<override>\v2`). The same override is used by portable installs.

## BLOCKED — exact API needed from the engine
The v2 workspace currently has **no migration API** (verified: grep for `migrate` in
`crates/` finds only bench/doc references; `crates/ps-core/src/` contains `engine.rs,
lib.rs, paths.rs, query.rs, rank.rs, settings.rs`). To complete this test, `ps-core` must
expose something equivalent to:

```rust
// crates/ps-core/src/migration.rs (proposed)
pub struct MigrationOptions {
    pub import_clipboard: bool,      // default false (privacy)
    pub import_ai_keys: bool,        // default false; never without explicit consent
    pub dry_run: bool,               // default true for first run
}
pub struct MigrationReport {
    pub files: u64, pub dirs: u64, pub content_docs: u64, pub browser_items: u64,
    pub quicklinks: u64, pub snippets: u64, pub memory_events: u64,
    pub skipped_tables: Vec<&'static str>,
    pub source_unchanged: bool,
}
/// Opens the legacy DB read-only (WAL-aware), imports into the v2 DB, and guarantees
/// the source file is byte-identical afterwards (hash before == hash after).
pub fn migrate_legacy_db(
    legacy_db: &std::path::Path,
    v2_db: &std::path::Path,
    opts: &MigrationOptions,
) -> anyhow::Result<MigrationReport>;
```

Until that exists, `v2_migration_test.rs.skeleton` is **not compiled** and the import
claims are untested. Do not claim migration works in release notes before this test passes.

## What the current test DOES prove
1. The fixture matches the documented legacy schema and row counts.
2. A read-only (immutable) open does not change the DB hash, does not create `-wal`/`-shm`,
   and leaves directory contents unchanged.
3. `ai_settings` contains no key material (only `endpoint`/`model`) — a migration cannot
   "leak" a key from this fixture.
4. The expected import plan (counts per table) is deterministic; v2 must match it.

## What it does NOT prove (honest)
- That v2 imports data correctly (API missing → BLOCKED).
- WAL-replay correctness on a real un-checkpointed legacy DB (fixture is checkpointed; the
  real baseline has a 23.8 MB WAL — see `docs/audit/GATE0_BASELINE.md` §3).
- Rollback/export UX, consent for AI keys, or performance on 149 MB.
