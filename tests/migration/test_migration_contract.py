#!/usr/bin/env python3
"""Contract tests for the legacy migration API.

Run: agent-sidecar\.venv\Scripts\python.exe -m unittest discover -s tests/migration -p "test_*.py" -v

The probe test builds a dev-only Rust harness (tests/migration/harness) that calls
ps_core::migration::migrate_legacy_db directly; it does not reimplement migration.
Set PS_REAL_LEGACY_DB to also run a READ-ONLY dry run against a real legacy DB.
"""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
PY = sys.executable
PROBE_MANIFEST = os.path.join(HERE, "harness", "Cargo.toml")


def run(script: str, *args: str, check: bool = True, env: dict | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(
        [PY, os.path.join(HERE, script), *args],
        capture_output=True, text=True, check=check, env=env,
    )


def run_probe(legacy_db: str, v2_db: str, dry_run: bool = True) -> dict:
    cargo_exe = shutil.which("cargo")
    if cargo_exe is None:
        raise unittest.SkipTest("cargo not available; migration probe cannot build")
    env = dict(os.environ)
    env.setdefault("CARGO_TARGET_DIR", os.path.join(REPO_ROOT, "target"))
    cmd = [
        cargo_exe, "run", "--quiet", "--manifest-path", PROBE_MANIFEST, "--",
        "--legacy", legacy_db, "--v2", v2_db,
    ]
    if dry_run:
        cmd.append("--dry-run")
    proc = subprocess.run(cmd, cwd=REPO_ROOT, capture_output=True, text=True, env=env)
    if proc.returncode != 0:
        raise AssertionError(
            "migration probe failed (exit %s)\nSTDOUT:\n%s\nSTDERR:\n%s"
            % (proc.returncode, proc.stdout[-4000:], proc.stderr[-8000:])
        )
    return json.loads(proc.stdout)


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


class MigrationFixtureContract(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="ps-migration-test-")
        self.addCleanup(self.tmp.cleanup)
        self.fixture_dir = os.path.join(self.tmp.name, "fixture")
        run("make_legacy_fixture.py", "--out-dir", self.fixture_dir)
        self.legacy_db = os.path.join(self.fixture_dir, "legacy_fixture.db")
        self.v2_db = os.path.join(self.tmp.name, "v2.db")

    def test_generated_fixture_matches_manifest(self) -> None:
        with open(os.path.join(self.fixture_dir, "fixture_manifest.json"), encoding="utf-8") as handle:
            manifest = json.load(handle)
        self.assertEqual(manifest["tables"]["files"], 12)
        self.assertEqual(manifest["ai_settings_keys"], ["endpoint", "model"])
        self.assertNotIn("api_key", manifest["ai_settings_keys"])

    def test_read_only_access_never_mutates_source(self) -> None:
        result = json.loads(run("verify_fixture.py", self.fixture_dir).stdout)
        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["source_modified"])

    def test_deterministic_fixture(self) -> None:
        other = os.path.join(self.tmp.name, "fixture2")
        run("make_legacy_fixture.py", "--out-dir", other)
        with open(os.path.join(self.fixture_dir, "fixture_manifest.json"), encoding="utf-8") as handle:
            a = json.load(handle)
        with open(os.path.join(other, "fixture_manifest.json"), encoding="utf-8") as handle:
            b = json.load(handle)
        self.assertEqual(a["sha256"], b["sha256"], "fixture generation must be deterministic")

    def test_expected_import_plan(self) -> None:
        plan = json.loads(run("verify_fixture.py", self.fixture_dir).stdout)["import_plan"]
        self.assertEqual(plan["browser_items"], 4)
        self.assertEqual(plan["memory_events"], 5)
        self.assertEqual(plan["quicklinks"], 2)
        self.assertEqual(plan["snippets"], 1)

    def test_migration_api_dry_run_contract(self) -> None:
        before = sha256_file(self.legacy_db)
        result = run_probe(self.legacy_db, self.v2_db, dry_run=True)
        self.assertTrue(result["ok"], result.get("error"))
        report = result["report"]
        self.assertEqual(report["files"], 10)
        self.assertEqual(report["dirs"], 2)
        self.assertEqual(report["content_docs"], 3)
        self.assertEqual(report["browser_items"], 4)
        self.assertEqual(report["quicklinks"], 2)
        self.assertEqual(report["snippets"], 1)
        self.assertEqual(report["memory_events"], 5)
        self.assertIn("ai_settings", report["skipped_tables"])
        self.assertIn("clipboard_history", report["skipped_tables"])
        self.assertIn("timeline_events", report["skipped_tables"])
        self.assertTrue(report["source_unchanged"])
        self.assertTrue(result["legacy_files"]["db_unchanged"], result["legacy_files"])
        self.assertEqual(before, sha256_file(self.legacy_db))
        self.assertFalse(result["v2_exists_after"], "dry run must not create a v2 database")
        self.assertFalse(os.path.exists(self.v2_db))

    @unittest.skipUnless(os.environ.get("PS_REAL_LEGACY_DB"), "set PS_REAL_LEGACY_DB for a real-DB read-only dry run")
    def test_real_legacy_db_dry_run_is_read_only(self) -> None:
        legacy = os.environ["PS_REAL_LEGACY_DB"]
        if not os.path.isfile(legacy):
            self.skipTest("real legacy database not present: " + legacy)
        before = sha256_file(legacy)
        result = run_probe(legacy, self.v2_db, dry_run=True)
        self.assertTrue(result["ok"], result.get("error"))
        report = result["report"]
        self.assertTrue(report["source_unchanged"])
        self.assertTrue(result["legacy_files"]["db_unchanged"], result["legacy_files"])
        self.assertEqual(before, sha256_file(legacy))
        self.assertFalse(result["v2_exists_after"], "dry run must not create a v2 database")
        print("REAL_DB_DRY_RUN " + json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    unittest.main(verbosity=2)
