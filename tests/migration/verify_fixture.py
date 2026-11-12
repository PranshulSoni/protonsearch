#!/usr/bin/env python3
"""Read-only contract checks for the synthetic legacy fixture.

Proves: schema/rows are as documented, a read-only open does not mutate the source
(byte-identical, no -wal/-shm, no directory changes), and no key material exists in the
fixture's ai_settings table. Does NOT test v2 migration (API absent -> BLOCKED).
"""
from __future__ import annotations

import hashlib
import json
import os
import sqlite3
import sys

EXPECTED_TABLES = {
    "files", "files_fts", "browser_items", "quicklinks", "snippets",
    "ai_settings", "memory_events", "clipboard_history", "timeline_events",
}
EXPECTED_COUNTS = {
    "files": 12, "browser_items": 4, "quicklinks": 2, "snippets": 1,
    "ai_settings": 2, "memory_events": 5, "clipboard_history": 0, "timeline_events": 0,
}


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def fail(msg: str) -> None:
    print("FAIL: " + msg, file=sys.stderr)
    raise SystemExit(1)


def verify(fixture_dir: str) -> dict:
    db_path = os.path.join(fixture_dir, "legacy_fixture.db")
    manifest_path = os.path.join(fixture_dir, "fixture_manifest.json")
    if not os.path.exists(db_path):
        fail("fixture DB missing: " + db_path)
    if not os.path.exists(manifest_path):
        fail("manifest missing: " + manifest_path)
    manifest = json.load(open(manifest_path, encoding="utf-8"))
    before_hash = sha256_file(db_path)
    before_files = sorted(os.listdir(fixture_dir))
    if before_hash != manifest["sha256"]:
        fail("fixture hash mismatch before read (regenerate the fixture)")

    uri = "file:" + db_path.replace("\\", "/") + "?mode=ro&immutable=1"
    con = sqlite3.connect(uri, uri=True)
    try:
        tables = {r[0] for r in con.execute("select name from sqlite_master where type='table'")}
        missing = EXPECTED_TABLES - tables
        if missing:
            fail("missing tables: " + repr(sorted(missing)))
        for table, expected in EXPECTED_COUNTS.items():
            got = con.execute("select count(*) from " + table).fetchone()[0]
            if got != expected:
                fail("row count mismatch for " + table + ": " + str(got) + " != " + str(expected))
        if con.execute("select count(*) from files_fts").fetchone()[0] != 3:
            fail("files_fts doc count != 3")
        keys = sorted(r[0] for r in con.execute("select key from ai_settings"))
        if keys != ["endpoint", "model"]:
            fail("ai_settings keys changed: " + repr(keys))
        if "api_key" in keys:
            fail("fixture must never contain api_key")
        if con.execute("pragma quick_check").fetchone()[0] != "ok":
            fail("pragma quick_check != ok")
        if con.execute("select count(*) from files_fts where files_fts match 'synthetic'").fetchone()[0] < 1:
            fail("FTS search returned no rows for 'synthetic'")
    finally:
        con.close()

    after_hash = sha256_file(db_path)
    after_files = sorted(os.listdir(fixture_dir))
    if after_hash != before_hash:
        fail("read-only access mutated the fixture DB")
    if after_files != before_files:
        fail("read-only access created/removed files: " + repr(set(after_files) ^ set(before_files)))
    for suffix in ("-wal", "-shm", "-journal"):
        if os.path.exists(db_path + suffix):
            fail("unexpected journal file created: " + db_path + suffix)

    import_plan = dict(EXPECTED_COUNTS)
    import_plan["files"] = 12
    return {
        "status": "PASS",
        "db_sha256": before_hash,
        "source_modified": False,
        "import_plan": import_plan,
        "blocked_on": "ps_core migration API (see README.md)",
    }


def main(argv=None) -> int:
    argv = argv if argv is not None else sys.argv[1:]
    fixture_dir = argv[0] if argv else os.environ.get("PS_MIGRATION_FIXTURE_DIR", "")
    if not fixture_dir:
        print("usage: verify_fixture.py <fixture-dir>", file=sys.stderr)
        return 2
    print(json.dumps(verify(fixture_dir), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
