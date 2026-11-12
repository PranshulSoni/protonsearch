"""Unit tests for protocol v1 wire shapes (stdlib unittest, no network)."""

from __future__ import annotations

import io
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from protonsearch_agent import protocol  # noqa: E402


class ParseInboundTests(unittest.TestCase):
    def test_agent_run_request_is_camel_case(self) -> None:
        msg = protocol.parse_inbound(
            json.dumps(
                {
                    "runId": "run-1",
                    "prompt": "find budget",
                    "contextPaths": ["C:/Users/x/budget.xlsx"],
                    "model": "openai:gpt-x",
                    "maxSteps": 3,
                }
            )
        )
        assert isinstance(msg, protocol.RunRequest)
        self.assertEqual(msg.run_id, "run-1")
        self.assertEqual(msg.prompt, "find budget")
        self.assertEqual(msg.context_paths, ("C:/Users/x/budget.xlsx",))
        self.assertEqual(msg.model, "openai:gpt-x")
        self.assertEqual(msg.max_steps, 3)

    def test_run_request_defaults_and_clamping(self) -> None:
        msg = protocol.parse_inbound(json.dumps({"runId": "r", "prompt": "x"}))
        assert isinstance(msg, protocol.RunRequest)
        self.assertEqual(msg.max_steps, protocol.DEFAULT_MAX_STEPS)
        big = protocol.parse_inbound(json.dumps({"runId": "r", "prompt": "x", "maxSteps": 999}))
        assert isinstance(big, protocol.RunRequest)
        self.assertEqual(big.max_steps, protocol.MAX_STEPS_CEILING)

    def test_cancel_message(self) -> None:
        msg = protocol.parse_inbound('{"type":"cancel","runId":"run-1"}')
        self.assertEqual(msg, protocol.CancelRequest(run_id="run-1"))
        self.assertEqual(protocol.parse_inbound('{"type":"cancel"}'), protocol.CancelRequest())

    def test_approval_response(self) -> None:
        msg = protocol.parse_inbound(
            '{"type":"approval_response","runId":"r","approvalId":"apr-1","approved":true}'
        )
        self.assertEqual(msg, protocol.ApprovalResponse(approval_id="apr-1", approved=True, run_id="r"))
        # The bare frozen AgentApprovalResponse shape (no type tag) is accepted.
        bare = protocol.parse_inbound('{"runId":"r","approvalId":"apr-1","approved":false}')
        self.assertEqual(bare, protocol.ApprovalResponse(approval_id="apr-1", approved=False, run_id="r"))

    def test_rejects_malformed_messages(self) -> None:
        for bad in (
            "",
            "not json",
            "[]",
            '{"runId":"r"}',
            '{"prompt":"x"}',
            '{"runId":"r","prompt":"x","contextPaths":[1]}',
            '{"runId":"r","prompt":"' + "x" * (protocol.MAX_PROMPT_CHARS + 1) + '"}',
            '{"type":"teleport","runId":"r"}',
        ):
            with self.subTest(bad=bad[:40]):
                if not bad.strip():
                    self.assertIsNone(protocol.parse_inbound(bad))
                else:
                    with self.assertRaises(protocol.ProtocolError):
                        protocol.parse_inbound(bad)

    def test_unknown_extra_fields_are_ignored(self) -> None:
        msg = protocol.parse_inbound('{"runId":"r","prompt":"x","futureField":1}')
        assert isinstance(msg, protocol.RunRequest)


class OutboundEventShapeTests(unittest.TestCase):
    """The frozen Rust serde definition is camelCase inside the tagged enum."""

    def test_started(self) -> None:
        self.assertEqual(
            protocol.started("r1", "pydantic-ai 2.66.0", "test:scripted"),
            {"type": "started", "runId": "r1", "runtime": "pydantic-ai 2.66.0", "model": "test:scripted"},
        )

    def test_plan_and_progress(self) -> None:
        self.assertEqual(protocol.plan(["a", "b"]), {"type": "plan", "steps": ["a", "b"]})
        self.assertEqual(
            protocol.progress("m", 0.5), {"type": "progress", "message": "m", "fraction": 0.5}
        )

    def test_approval_request_open_path(self) -> None:
        event = protocol.approval_request(
            "apr-1", "Open", "detail", protocol.action_open_path("C:/x y.txt"), False
        )
        self.assertEqual(
            event,
            {
                "type": "approval_request",
                "approvalId": "apr-1",
                "title": "Open",
                "detail": "detail",
                "action": {"type": "open_path", "path": "C:/x y.txt", "line": None, "column": None},
                "destructive": False,
            },
        )

    def test_native_command_action_is_enumerated(self) -> None:
        self.assertEqual(
            protocol.action_run_native("shutdown"),
            {"type": "run_native", "command": "shutdown"},
        )

    def test_tool_result_output_error_finished(self) -> None:
        self.assertEqual(
            protocol.tool_result("search_files", True, "p"),
            {"type": "tool_result", "tool": "search_files", "ok": True, "preview": "p"},
        )
        self.assertEqual(
            protocol.output("hello", True),
            {"type": "output", "text": "hello", "finalAnswer": True},
        )
        self.assertEqual(
            protocol.error("cancelled", "m"),
            {"type": "error", "code": "cancelled", "message": "m"},
        )
        self.assertEqual(
            protocol.finished(False, 12),
            {"type": "finished", "ok": False, "durationMs": 12},
        )


class EventWriterTests(unittest.TestCase):
    def test_writes_one_json_object_per_line(self) -> None:
        stream = io.StringIO()
        writer = protocol.EventWriter(stream)
        writer.emit(protocol.progress("one"))
        writer.emit(protocol.progress("two"))
        lines = stream.getvalue().splitlines()
        self.assertEqual(len(lines), 2)
        self.assertEqual(json.loads(lines[1])["message"], "two")

    def test_broken_pipe_is_silent(self) -> None:
        class Broken(io.StringIO):
            def write(self, _value):  # noqa: ANN001
                raise BrokenPipeError()

        writer = protocol.EventWriter(Broken())
        writer.emit(protocol.progress("x"))
        self.assertTrue(writer.broken)


if __name__ == "__main__":
    unittest.main()
