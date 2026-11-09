"""Wire protocol v1 for the ProtonSearch agent sidecar.

Contract (frozen Rust side: `crates/ps-ipc/src/lib.rs`):

* Transport: UTF-8 JSON Lines over stdio. One JSON object per line, no
  trailing content, no partial lines.
* Host -> sidecar messages:
  - `AgentRunRequest` with **camelCase** fields: runId, prompt, contextPaths,
    model, maxSteps (serde `rename_all = "camelCase"`).
  - cancel control message: `{"type":"cancel","runId":"..."}`.
  - `AgentApprovalResponse` with camelCase fields plus a `type` tag:
    `{"type":"approval_response","runId":"...","approvalId":"...","approved":true}`.
    The bare frozen struct (no `type`) is accepted as well.
* Sidecar -> host messages: `AgentEvent`, externally tagged with `type`
  ("started", "plan", "progress", "approval_request", "tool_result", "output",
  "error", "finished"). Field names inside the variants are **snake_case**:
  the frozen Rust enum has `rename_all = "snake_case"` on the variant names but
  no `rename_all_fields`, so serde expects e.g. `run_id`, `approval_id`,
  `duration_ms`, `final_answer`. Do not "helpfully" camelCase them.

Anything the host does not understand is reported back as an `error` event and
does not kill the process.
"""

from __future__ import annotations

import json
import threading
from dataclasses import dataclass
from typing import Any, Callable, TextIO

PROTOCOL_VERSION = 1

#: Hard bound on one JSONL frame in either direction (Rust client enforces the
#: same bound). Keeps a corrupt/hostile peer from growing our line buffer.
MAX_LINE_BYTES = 4 * 1024 * 1024
MAX_PROMPT_CHARS = 64 * 1024
MAX_CONTEXT_PATHS = 64
MAX_PATH_CHARS = 4096
MAX_RUN_ID_CHARS = 128
MAX_APPROVAL_ID_CHARS = 128
MAX_MODEL_CHARS = 200
MAX_STEPS_CEILING = 32
DEFAULT_MAX_STEPS = 8


class ProtocolError(ValueError):
    """A line could not be interpreted as a protocol v1 message."""


@dataclass(frozen=True)
class RunRequest:
    run_id: str
    prompt: str
    context_paths: tuple[str, ...] = ()
    model: str | None = None
    max_steps: int = DEFAULT_MAX_STEPS


@dataclass(frozen=True)
class CancelRequest:
    run_id: str | None = None


@dataclass(frozen=True)
class ApprovalResponse:
    approval_id: str
    approved: bool
    run_id: str | None = None


@dataclass(frozen=True)
class ProtocolErrorMessage:
    message: str


Inbound = RunRequest | CancelRequest | ApprovalResponse | ProtocolErrorMessage


def _text(obj: dict[str, Any], *names: str) -> str | None:
    for name in names:
        value = obj.get(name)
        if isinstance(value, str):
            return value
    return None


def _int(obj: dict[str, Any], *names: str) -> int | None:
    for name in names:
        value = obj.get(name)
        if isinstance(value, bool):
            continue
        if isinstance(value, int):
            return value
        if isinstance(value, float) and value.is_integer():
            return int(value)
    return None


def parse_inbound(line: str) -> Inbound | None:
    """Parse one line. Returns None for blank lines, raises ProtocolError otherwise."""
    line = line.strip()
    if not line:
        return None
    try:
        obj = json.loads(line)
    except json.JSONDecodeError as exc:
        raise ProtocolError(f"invalid JSON: {exc.msg}") from None
    if not isinstance(obj, dict):
        raise ProtocolError("message must be a JSON object")

    msg_type = obj.get("type")
    if isinstance(msg_type, str) and msg_type == "cancel":
        return CancelRequest(run_id=_validate_run_id(_text(obj, "runId", "run_id"), required=False))

    approval_id = _text(obj, "approvalId", "approval_id")
    approved = obj.get("approved")
    if approval_id is not None and isinstance(approved, bool):
        if msg_type is not None and msg_type not in ("approval_response", "approval"):
            raise ProtocolError(f"unexpected type {msg_type!r} for an approval response")
        if len(approval_id) > MAX_APPROVAL_ID_CHARS:
            raise ProtocolError("approvalId is too long")
        return ApprovalResponse(
            approval_id=approval_id,
            approved=approved,
            run_id=_validate_run_id(_text(obj, "runId", "run_id"), required=False),
        )
    if msg_type is not None and msg_type != "run_request":
        raise ProtocolError(f"unknown message type {msg_type!r}")

    prompt = obj.get("prompt")
    if not isinstance(prompt, str):
        raise ProtocolError("run request requires a string 'prompt'")
    if len(prompt) > MAX_PROMPT_CHARS:
        raise ProtocolError(f"prompt exceeds {MAX_PROMPT_CHARS} characters")

    paths_raw = obj.get("contextPaths", obj.get("context_paths", ()))
    if paths_raw is None:
        paths_raw = ()
    if not isinstance(paths_raw, (list, tuple)):
        raise ProtocolError("contextPaths must be an array of strings")
    if len(paths_raw) > MAX_CONTEXT_PATHS:
        raise ProtocolError(f"contextPaths exceeds {MAX_CONTEXT_PATHS} entries")
    context_paths: list[str] = []
    for index, raw_path in enumerate(paths_raw):
        if not isinstance(raw_path, str):
            raise ProtocolError(f"contextPaths[{index}] must be a string")
        if len(raw_path) > MAX_PATH_CHARS:
            raise ProtocolError(f"contextPaths[{index}] is too long")
        if "\x00" in raw_path:
            raise ProtocolError(f"contextPaths[{index}] contains a NUL byte")
        context_paths.append(raw_path)

    model = _text(obj, "model")
    if model is not None and len(model) > MAX_MODEL_CHARS:
        raise ProtocolError("model name is too long")

    max_steps = _int(obj, "maxSteps", "max_steps")
    if max_steps is None:
        max_steps = DEFAULT_MAX_STEPS
    max_steps = max(1, min(max_steps, MAX_STEPS_CEILING))

    return RunRequest(
        run_id=_validate_run_id(_text(obj, "runId", "run_id"), required=True),
        prompt=prompt,
        context_paths=tuple(context_paths),
        model=model,
        max_steps=max_steps,
    )


def _validate_run_id(run_id: str | None, *, required: bool) -> str | None:
    if run_id is None:
        if required:
            raise ProtocolError("run request requires a string 'runId'")
        return None
    if not run_id or len(run_id) > MAX_RUN_ID_CHARS:
        raise ProtocolError(f"runId must be 1..{MAX_RUN_ID_CHARS} characters")
    return run_id


# ---------------------------------------------------------------------------
# Outbound AgentEvent constructors (snake_case fields, frozen Rust shape)
# ---------------------------------------------------------------------------


def started(run_id: str, runtime: str, model: str) -> dict[str, Any]:
    return {"type": "started", "run_id": run_id, "runtime": runtime, "model": model}


def plan(steps: list[str]) -> dict[str, Any]:
    return {"type": "plan", "steps": list(steps)}


def progress(message: str, fraction: float | None = None) -> dict[str, Any]:
    return {"type": "progress", "message": message, "fraction": fraction}


def approval_request(
    approval_id: str,
    title: str,
    detail: str,
    action: dict[str, Any],
    destructive: bool,
) -> dict[str, Any]:
    return {
        "type": "approval_request",
        "approval_id": approval_id,
        "title": title,
        "detail": detail,
        "action": action,
        "destructive": destructive,
    }


def tool_result(tool: str, ok: bool, preview: str) -> dict[str, Any]:
    return {"type": "tool_result", "tool": tool, "ok": ok, "preview": preview}


def output(text: str, final_answer: bool = True) -> dict[str, Any]:
    return {"type": "output", "text": text, "final_answer": final_answer}


def error(code: str, message: str) -> dict[str, Any]:
    return {"type": "error", "code": code, "message": message}


def finished(ok: bool, duration_ms: int) -> dict[str, Any]:
    return {"type": "finished", "ok": ok, "duration_ms": duration_ms}


# ---------------------------------------------------------------------------
# ActionSpec constructors (frozen Rust shape: tag "type", snake_case variant)
# ---------------------------------------------------------------------------


def action_none() -> dict[str, Any]:
    return {"type": "none"}


def action_open_path(path: str) -> dict[str, Any]:
    return {"type": "open_path", "path": path, "line": None, "column": None}


def action_run_native(command: str) -> dict[str, Any]:
    return {"type": "run_native", "command": command}


# ---------------------------------------------------------------------------
# Writer
# ---------------------------------------------------------------------------


class EventWriter:
    """Thread-safe JSONL writer for stdout. Never writes non-protocol text."""

    def __init__(self, stream: TextIO | None = None) -> None:
        self._stream = stream
        self._lock = threading.Lock()
        self._broken = False

    @property
    def broken(self) -> bool:
        return self._broken

    def emit(self, event: dict[str, Any]) -> None:
        line = json.dumps(event, ensure_ascii=False, separators=(",", ":"))
        encoded_len = len(line.encode("utf-8"))
        if encoded_len > MAX_LINE_BYTES:
            raise ProtocolError(f"outbound event exceeds {MAX_LINE_BYTES} bytes")
        with self._lock:
            if self._broken:
                return
            stream = self._stream
            if stream is None:
                return
            try:
                stream.write(line + "\n")
                stream.flush()
            except (BrokenPipeError, OSError, ValueError):
                # The host is gone; stop writing but let the main loop unwind.
                self._broken = True


def emit_jsonl(writer: Callable[[dict[str, Any]], None], event: dict[str, Any]) -> None:
    writer(event)
