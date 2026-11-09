# Migration Guide — legacy v1.1.1 → v2 (Gate 0 baseline)

Status: **design + fixture only. BLOCKED on the v2 migration API** (no ps_core migration
module exists yet — verified by searching crates/; only ps-bench has read-only legacy access
for benchmarking). Do not claim migration works until the blocked test in tests/migration/
passes.

## 1. Inputs (measured at baseline, 2026-11-09)

Source: %APPDATA%\protonsearch\file_index.db — 149,372,928 B, SHA-256
58249F011A045E74F02582F50722C80699E28BD6EDBE99073F567D386F361809; WAL 23,788,912 B
(un-checkpointed) + SHM 32,768 B; PRAGMA quick_check = ok.

| Legacy data | Count | Migration decision (proposed) |
|---|---:|---|
| files | 89,813 (30,287 dirs / 59,526 files) | Import paths + metadata; v2 should re-key on volume+file_id, not path |
| files_fts | 14,609 docs | Import content text into ps-content FTS5; do not trust legacy offsets |
| browser_items | 4,523 | Import into ps-sources browser store; v2 also re-indexes live browser DBs |
| memory_events | 5,286 | Import into v2 activity store (shape TBD; G4) |
| git_repos/commits/branches/todos | 15 / 516 / 23 / 132 | Re-index from repos (cheaper/safer) or import; counts are acceptance fixture |
| quicklinks | 4 | Import (GitHub, Google, Rust Docs, YouTube) |
| snippets | 1 | Import ("Example Snippet") |
| settings_catalog_fts | 238 (173 SETTINGS + 65 CONTROL) | **Do not import** — regenerated from embedded catalog |
| ai_settings | 2 (endpoint, model; **no api_key**) | Consent to import non-secret endpoint/model; never import keys |
| clipboard_history | 0 | Empty; nothing to import. v2 capture is opt-in/off |
| timeline_events | 0 | Empty; legacy tracker wrote nothing. Do not migrate |
| focus_categories | 1 ("Deep Work") | Import if focus survives G5 review |
| ignored_folders | 2 (node_modules, .angular) | Import as exclusions |
| notes/ (files) | empty | Copy only with opt-in; v2 owns a different data dir |
| protondock-layout.json | 1,602 B | Optional UI-layout import at G4 |
| logs (indexer.log, git_indexer.log, panic.log) | present | **Never migrate** (paths/user data) |
| index.db (0 B) | stale | Ignore |

## 2. Invariants (non-negotiable)

1. **Never mutate the legacy DB.** Open file:...?mode=ro (WAL-aware) or copy DB+WAL+SHM to a
   migration cache and open read-only. Legacy DB stays byte-identical (hash before/after).
2. **Never run the legacy uninstaller or its install scripts.** installer.iss:32-33 deletes
   {userappdata}\protonsearch (the index/history) — v2 must never call it.
3. **Never import secret material silently.** If ai_settings.api_key/hermes_api_key exist,
   skip by default; require explicit consent to move into OS secret storage. Never log values.
4. **Backup before import:** export a rollback snapshot (SQLite backup API or file copy with
   WAL) and record its SHA-256; keep until the user confirms v2 works.
5. **Dry-run first:** report counts and skipped tables before writing v2 data.
6. **Idempotent or explicit:** second run must be a no-op or fail clearly; never duplicate.
7. **Respect PROTONSEARCH_DATA_DIR** for tests/portable installs (paths.rs:6-19).

## 3. Proposed v2 flow (blocked on API)

```text
detect legacy_db (legacy_data_dir()/file_index.db)
  -> if absent: skip
  -> snapshot copy (db + -wal + -shm) to v2 cache/migration/<timestamp>/
  -> open read-only; quick_check; record counts + sha256
  -> MigrationOptions { dry_run: true, import_clipboard: false, import_ai_keys: false }
  -> report preview -> user consent screen
  -> migrate (import tables above) -> verify counts -> write migration_report.json
  -> keep rollback snapshot; never touch legacy files
```

## 4. Current test coverage (honest)

tests/migration/ contains:
- make_legacy_fixture.py — deterministic synthetic legacy DB (9 tables, 12 files, 3 FTS docs,
  4 browser items, 5 memory events, no secrets) + manifest with SHA-256.
- verify_fixture.py — proves schema/rows and that a read-only open does **not** mutate the
  source (hash unchanged, no -wal/-shm, no file-set change).
- test_migration_contract.py — 5 tests: **4 pass, 1 skipped/BLOCKED** (v2 import test).
- v2_migration_test.rs.skeleton — exact API needed: ps_core::migration::migrate_legacy_db
  with MigrationOptions and MigrationReport { source_unchanged, ... }.

Verified run (2026-11-09):

```text
Ran 5 tests in 1.512s
OK (skipped=1)
fixture sha256=f713bae3...4d2d; verify_fixture status=PASS; source_modified=false
```

**Not proven:** WAL-replay on the real 23.8 MB WAL, 149 MB performance, rollback UX, AI-key
consent, import correctness. Those require the engine API and a real-DB read-only dry run.

## 5. Test-environment limitation

Child processes in this sandbox cannot write %APPDATA%. Tests must use
PROTONSEARCH_DATA_DIR=<temp>/ so v2 data lands in <temp>/v2. This is a harness constraint,
not product behaviour; portable installs use the same override by design.
