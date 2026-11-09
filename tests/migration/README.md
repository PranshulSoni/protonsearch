# Migration fixture + contract test (Gate 0/G4)

Status: **API delivered; the contract test now exercises ps_core::migration::migrate_legacy_db
through a dev-only Rust probe.** The real-DB read-only dry run is recorded in
docs/release/MIGRATION.md. Import correctness beyond these contract checks remains gated on
G4 acceptance (UI flow, consent, rollback UX).

## Why a synthetic fixture
The real baseline DB lives at %APPDATA%\protonsearch\file_index.db (149 MB, user data).
Tests must never depend on, copy, or mutate it. The fixture reproduces the legacy schema and
representative synthetic rows only:

| Table | Rows in fixture | Notes |
|---|---:|---|
| files | 12 | mixed files/dirs (10 files + 2 dirs), extensions, last_seen_scan |
| files_fts (FTS5) | 3 | path UNINDEXED, content; same schema as legacy |
| browser_items | 4 | chrome/brave/firefox history+bookmark sources |
| quicklinks | 2 | name/url/keyword |
| snippets | 1 | name/content/keyword |
| ai_settings | 2 | keys endpoint, model only - NO api_key |
| memory_events | 5 | Browser/Visited Page + Git/Commit rows |
| clipboard_history | 0 | empty table; empty-source handling |
| timeline_events | 0 | empty table; empty-source handling |

All content is synthetic. The generator is deterministic and writes fixture_manifest.json
with row counts + SHA-256.

## Commands
```powershell
# generate into a temp dir (never into the repo)
python tests/migration/make_legacy_fixture.py --out-dir <temp-dir>

# contract tests (builds the probe with cargo; cargo must be installed)
agent-sidecar\.venv\Scripts\python.exe -m unittest discover -s tests/migration -p "test_*.py" -v

# optional real-DB READ-ONLY dry run (never imports into real data)
$env:PS_REAL_LEGACY_DB = "$env:APPDATA\protonsearch\file_index.db"
agent-sidecar\.venv\Scripts\python.exe -m unittest discover -s tests/migration -p "test_*.py" -v
```

Environment limitation (recorded, not product behaviour): this sandbox's child processes
cannot write %APPDATA%. E2E/migration tests must set PROTONSEARCH_DATA_DIR to a temp directory
(implemented in crates/ps-core/src/paths.rs:6-19; v2 data then lives under <override>\v2).

## What the contract test proves (via the real API)
1. migrate_legacy_db opens the fixture read-only and reports counts: files=10, dirs=2,
   content_docs=3, browser_items=4, quicklinks=2, snippets=1, memory_events=5;
   skipped_tables contains ai_settings, clipboard_history, timeline_events;
   source_unchanged=true.
2. The legacy DB SHA-256 is identical before/after; the dry run creates no v2 database.
3. The probe also reports WAL/SHM hashes and any new files in the legacy directory - so a
   read-only open that silently touches WAL/SHM is visible in the evidence (see the v2
   security review).
4. Deterministic fixture: two generations produce the same SHA-256.

## Probe design (tests/migration/harness)
harness/Cargo.toml is a standalone workspace (dev-only, not shipped) depending on ps-core;
src/main.rs calls the API and prints a JSON report (ok, report.*, legacy_files.*,
v2_exists_after, elapsed_ms). This keeps the test honest: it exercises the same code the
engine calls, without reimplementing migration in Python.

## What it does NOT prove (honest)
- UI/onboarding consent flow, rollback snapshot UX, or AI-key consent (G4).
- Full 149 MB import performance; the real-DB dry run is read-only and reported separately.
- WAL-replay correctness on a real un-checkpointed legacy DB (observed and recorded, not a
  regression test).
