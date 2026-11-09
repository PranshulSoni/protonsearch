#!/usr/bin/env python3
"""Contract tests for the legacy migration fixture.

Run: agent-sidecar\.venv\Scripts\python.exe -m unittest discover -s tests/migration -p "test_*.py" -v

These tests pass today. The v2 IMPORT step is BLOCKED on the ps_core migration API and is
reported as skipped, never faked (see README.md for the exact required API).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
PY = sys.executable


def run(script: str, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run([PY, os.path.join(HERE, script), *args],
                          capture_output=True, text=True, check=check)


class MigrationFixtureContract(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="ps-migration-test-")
        self.addCleanup(self.tmp.cleanup)
        self.fixture_dir = os.path.join(self.tmp.name, "fixture")
        run("make_legacy_fixture.py", "--out-dir", self.fixture_dir)

    def test_generated_fixture_matches_manifest(self) -> None:
        manifest = json.load(open(os.path.join(self.fixture_dir, "fixture_manifest.json"), encoding="utf-8"))
        self.assertEqual(manifest["tables"]["files"], 12)
        self.assertEqual(manifest["ai_settings_keys"], ["endpoint", "model"])
        self.assertNotIn("api_key", manifest["ai_settings_keys"])

    def test_read_only_access_never_mutates_source(self) -> None:
        result = json.loads(run("verify_fixture.py", self.fixture_dir).stdout)
        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["source_modified"])
        self.assertIn("ps_core migration API", result["blocked_on"])

    def test_deterministic_fixture(self) -> None:
        other = os.path.join(self.tmp.name, "fixture2")
        run("make_legacy_fixture.py", "--out-dir", other)
        a = json.load(open(os.path.join(self.fixture_dir, "fixture_manifest.json"), encoding="utf-8"))
        b = json.load(open(os.path.join(other, "fixture_manifest.json"), encoding="utf-8"))
        self.assertEqual(a["sha256"], b["sha256"], "fixture generation must be deterministic")

    def test_expected_import_plan(self) -> None:
        plan = json.loads(run("verify_fixture.py", self.fixture_dir).stdout)["import_plan"]
        self.assertEqual(plan["browser_items"], 4)
        self.assertEqual(plan["memory_events"], 5)
        self.assertEqual(plan["quicklinks"], 2)
        self.assertEqual(plan["snippets"], 1)

    @unittest.skip("BLOCKED: ps_core migration API does not exist yet (see README.md)")
    def test_v2_migration_imports_without_mutating_source(self) -> None:
        self.fail("Do not implement until ps_core::migration::migrate_legacy_db exists")


if __name__ == "__main__":
    unittest.main(verbosity=2)
