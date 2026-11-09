"""End-to-end tests of the sidecar process in deterministic test-model mode.

These tests spawn the real process with `PROTONSEARCH_AGENT_TEST_MODEL=1`:
no network and no API key are used. They skip (with a clear reason) when the
machine-local virtualenv has not been created.
"""

from __future__ import annotations

import json
import os
import queue
import subprocess
import sys
import threading
import unittest
from pathlib import Path

SIDECAR_DIR = Path(__file__).resolve().parent.parent
PYTHON = SIDECAR_DIR / ".venv" / "Scripts" / "python.exe"
SKIP_REASON = (
    f"sidecar venv not found at {PYTHON}; create it with "
    "C:/Users/Pranshul/AppData/Local/Programs/Python/Python314/python.exe -m venv agent-sidecar/.venv "
    "and install agent-sidecar/requirements.txt"
)


class SidecarProcess:
    def __init__(self) -> None:
        env = dict(os.environ)
        env.update(
            {
                "PROTONSEARCH_AGENT_TEST_MODEL": "1",
                "PYTHONIOENCODING": "utf-8",
                "PYTHONUNBUFFERED": "1",
            }
        )
        self.proc = subprocess.Popen(
            [str(PYTHON), "-m", "protonsearch_agent"],
            cwd=str(SIDECAR_DIR),
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            bufsize=1,
        )
        self.events: queue.Queue = queue.Queue()
        self.stderr_chunks: list[str] = []
        threading.Thread(target=self._pump_stdout, daemon=True).start()
        threading.Thread(target=self._pump_stderr, daemon=True).start()

    def _pump_stdout(self) -> None:
        for line in self.proc.stdout:
            self.events.put(line)
        self.events.put(None)

    def _pump_stderr(self) -> None:
        self.stderr_chunks.append(self.proc.stderr.read())

    def send(self, payload: dict) -> None:
        self.proc.stdin.write(json.dumps(payload) + "\n")
        self.proc.stdin.flush()

    def next_event(self, timeout: float = 30.0) -> dict:
        line = self.events.get(timeout=timeout)
        if line is None:
            raise AssertionError("sidecar closed stdout unexpectedly: " + self.stderr_text())
        return json.loads(line)

    def wait_for(self, event_type: str, timeout: float = 30.0) -> dict:
        deadline = threading.TIMEOUT_MAX if timeout is None else timeout
        del deadline
        while True:
            event = self.next_event(timeout=timeout)
            if event.get("type") == event_type:
                return event

    def stderr_text(self) -> str:
        return "".join(self.stderr_chunks)

    def close(self) -> int:
        try:
            if self.proc.stdin:
                self.proc.stdin.close()
        except OSError:
            pass
        try:
            code = self.proc.wait(timeout=20)
        finally:
            if self.proc.poll() is None:
                self.proc.kill()
                self.proc.wait(timeout=10)
            for stream in (self.proc.stdout, self.proc.stderr):
                try:
                    stream.close()
                except OSError:
                    pass
        return code


@unittest.skipUnless(PYTHON.is_file(), SKIP_REASON)
class ScriptedSidecarTests(unittest.TestCase):
    def setUp(self) -> None:
        self.sidecar = SidecarProcess()

    def tearDown(self) -> None:
        code = self.sidecar.close()
        stderr = self.sidecar.stderr_text()
        self.assertNotIn("Traceback (most recent call last)", stderr, stderr[-2000:])
        self.assertEqual(code, 0, stderr[-2000:])

    def test_approval_flow_streams_events_and_finishes(self) -> None:
        self.sidecar.send(
            {
                "runId": "run-e2e",
                "prompt": "find the quarterly budget file",
                "contextPaths": ["C:/Users/Public/budget.txt"],
            }
        )
        started = self.sidecar.next_event()
        self.assertEqual(started["type"], "started")
        self.assertEqual(started["runId"], "run-e2e")
        self.assertEqual(started["model"], "test:scripted")
        self.assertIn("pydantic-ai", started["runtime"])

        self.assertEqual(self.sidecar.next_event()["type"], "plan")

        first_approval = self.sidecar.wait_for("approval_request")
        self.assertEqual(first_approval["action"], {"type": "none"})
        self.assertFalse(first_approval["destructive"])
        self.assertEqual(first_approval["title"], "Search your files")
        self.assertIn("find the quarterly budget file", first_approval["detail"])
        self.sidecar.send(
            {
                "type": "approval_response",
                "runId": "run-e2e",
                "approvalId": first_approval["approvalId"],
                "approved": True,
            }
        )
        first_result = self.sidecar.wait_for("tool_result")
        self.assertTrue(first_result["ok"])
        self.assertIn("search_files", first_result["preview"])

        second_approval = self.sidecar.wait_for("approval_request")
        self.assertEqual(second_approval["title"], "Preview a file")
        self.assertIn("C:/Users/Public/budget.txt", second_approval["detail"])
        self.sidecar.send(
            {
                "type": "approval_response",
                "runId": "run-e2e",
                "approvalId": second_approval["approvalId"],
                "approved": True,
            }
        )
        self.assertTrue(self.sidecar.wait_for("tool_result")["ok"])

        output = self.sidecar.wait_for("output")
        self.assertTrue(output["finalAnswer"])
        self.assertIn("TEST MODEL", output["text"])
        finished = self.sidecar.wait_for("finished")
        self.assertTrue(finished["ok"])
        self.assertGreaterEqual(finished["durationMs"], 0)

    def test_denied_approval_is_reported_not_executed(self) -> None:
        self.sidecar.send({"runId": "run-denied", "prompt": "find budget"})
        approval = self.sidecar.wait_for("approval_request")
        self.sidecar.send(
            {
                "type": "approval_response",
                "runId": "run-denied",
                "approvalId": approval["approvalId"],
                "approved": False,
            }
        )
        result = self.sidecar.wait_for("tool_result")
        self.assertFalse(result["ok"])
        self.assertIn("denied", result["preview"])
        self.assertTrue(self.sidecar.wait_for("finished")["ok"])

    def test_cancel_while_waiting_for_approval(self) -> None:
        self.sidecar.send({"runId": "run-cancel", "prompt": "find budget"})
        self.sidecar.wait_for("approval_request")
        self.sidecar.send({"type": "cancel", "runId": "run-cancel"})
        cancelled = self.sidecar.wait_for("error")
        self.assertEqual(cancelled["code"], "cancelled")
        finished = self.sidecar.wait_for("finished")
        self.assertFalse(finished["ok"])

    def test_unknown_message_does_not_kill_process(self) -> None:
        self.sidecar.send({"type": "teleport", "runId": "x"})
        error_event = self.sidecar.wait_for("error")
        self.assertEqual(error_event["code"], "protocol_error")
        self.sidecar.send({"runId": "run-after", "prompt": "still alive"})
        self.assertEqual(self.sidecar.wait_for("started")["runId"], "run-after")

    def test_describe_mode_needs_no_protocol(self) -> None:
        env = dict(os.environ)
        env.update({"PROTONSEARCH_AGENT_TEST_MODEL": "1", "PYTHONIOENCODING": "utf-8"})
        result = subprocess.run(
            [str(PYTHON), "-m", "protonsearch_agent", "--describe"],
            cwd=str(SIDECAR_DIR),
            env=env,
            capture_output=True,
            text=True,
            timeout=60,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        described = json.loads(result.stdout)
        self.assertEqual(described["protocol"], 1)
        self.assertIn("search_files", described["tools"])
        self.assertIn("shutdown", described["nativeCommands"])


if __name__ == "__main__":
    unittest.main()
