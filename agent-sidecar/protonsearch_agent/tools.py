"""The sidecar's complete tool allowlist.

Security model: the agent never touches the OS itself. Every tool first emits
an `approval_request` AgentEvent carrying a structured `ActionSpec` from
ps-ipc, waits for the host's `approval_response`, and only then emits
`tool_result`. The Rust host is the only component that executes anything.

There is no shell, no `subprocess`, no file I/O and no network in this module.
Command execution is restricted to the enumerated ps-ipc `NativeCommand`
values, addressed by id (never by command line).

Known protocol-v1 limitation: the frozen `AgentEvent` set has no channel for
the host to return *results* (e.g. search hits) into the agent loop, so tools
report "approved; delegated to the host" and the final answer cannot cite
result contents yet. A v2 capability-result message is proposed in
docs/agent/INTEGRATION.md; it requires an L0-owned ps-ipc change.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable

from pydantic_ai import Agent, RunContext

from . import protocol
from .approvals import ApprovalBroker

#: Enumerated native commands (mirror of ps_ipc::NativeCommand, snake_case ids).
NATIVE_COMMANDS: dict[str, bool] = {
    # id -> destructive (requires a second, explicit confirmation in the UI)
    "lock_workstation": False,
    "sleep": True,
    "shutdown": True,
    "restart": True,
    "empty_recycle_bin": True,
    "open_settings": False,
    "open_task_manager": False,
    "open_terminal": False,
    "toggle_dark_mode": False,
    "pause_indexing": False,
    "resume_indexing": False,
}

TOOL_ALLOWLIST: tuple[str, ...] = (
    "search_files",
    "read_file_preview",
    "open_path",
    "run_native_command",
)

MAX_QUERY_CHARS = 512
MAX_SEARCH_LIMIT = 50
MAX_PREVIEW_BYTES = 64 * 1024


def _clean_single_line(value: str, limit: int) -> str:
    """Collapse control characters so tool arguments cannot forge event lines."""
    cleaned = "".join(ch if ch.isprintable() else " " for ch in value)
    return " ".join(cleaned.split())[:limit]


@dataclass
class RunDeps:
    run_id: str
    emit: Callable[[dict[str, Any]], None]
    broker: ApprovalBroker
    max_steps: int
    steps: int = 0
    approved: int = 0
    denied: int = 0
    rejected: list[str] = field(default_factory=list)

    async def request_host_action(
        self,
        *,
        tool: str,
        title: str,
        detail: str,
        action: dict[str, Any],
        destructive: bool,
        preview: str,
    ) -> str:
        """Emit approval_request, wait for the decision, emit tool_result."""
        if tool not in TOOL_ALLOWLIST:  # defensive: only registered tools reach here
            self.emit(protocol.tool_result(tool, False, "tool is not in the allowlist"))
            return "error: tool not allowed"
        self.steps += 1
        if self.steps > self.max_steps:
            message = f"step limit reached ({self.max_steps}); no further approvals will be requested"
            self.emit(protocol.tool_result(tool, False, message))
            return f"error: {message}"

        self.emit(protocol.progress(f"waiting for approval: {tool}", None))
        approval_id, future = self.broker.create()
        self.emit(protocol.approval_request(approval_id, title, detail, action, destructive))
        approved = await future
        if approved:
            self.approved += 1
            self.emit(protocol.tool_result(tool, True, preview))
            return f"approved: {preview}"
        self.denied += 1
        self.emit(protocol.tool_result(tool, False, "denied by user"))
        return "error: the user denied this action; do not retry it without asking why"

    def reject(self, tool: str, reason: str) -> str:
        self.steps += 1
        self.rejected.append(f"{tool}: {reason}")
        self.emit(protocol.tool_result(tool, False, reason))
        return f"error: {reason}"


async def search_files(ctx: RunContext[RunDeps], query: str, limit: int = 5) -> str:
    """Search the local ProtonSearch file index for a name or content query.

    Read-only capability. The host executes the search after the user approves
    it; this sidecar never reads the index or the filesystem itself.
    """
    deps = ctx.deps
    query = _clean_single_line(query, MAX_QUERY_CHARS)
    if not query:
        return deps.reject("search_files", "query must not be empty")
    if not isinstance(limit, int) or isinstance(limit, bool) or limit < 1 or limit > MAX_SEARCH_LIMIT:
        return deps.reject("search_files", f"limit must be an integer 1..{MAX_SEARCH_LIMIT}")
    detail = f"Search the local index for {query!r} (limit {limit}). Read-only."
    preview = f"search_files(query={query!r}, limit={limit}) approved; delegated to the host"
    return await deps.request_host_action(
        tool="search_files",
        title="Search your files",
        detail=detail,
        action=protocol.action_none(),
        destructive=False,
        preview=preview,
    )


async def read_file_preview(ctx: RunContext[RunDeps], path: str, max_bytes: int = 4096) -> str:
    """Read a short, bounded text preview of one file (host-executed, approved).

    Read-only capability; the host enforces path allowlists. The sidecar never
    opens the file.
    """
    deps = ctx.deps
    path = _clean_single_line(path, protocol.MAX_PATH_CHARS)
    if not path:
        return deps.reject("read_file_preview", "path must not be empty")
    if not isinstance(max_bytes, int) or isinstance(max_bytes, bool) or max_bytes < 1 or max_bytes > MAX_PREVIEW_BYTES:
        return deps.reject("read_file_preview", f"max_bytes must be an integer 1..{MAX_PREVIEW_BYTES}")
    detail = f"Read at most {max_bytes} bytes of {path!r}. Read-only."
    preview = f"read_file_preview(path={path!r}, max_bytes={max_bytes}) approved; delegated to the host"
    return await deps.request_host_action(
        tool="read_file_preview",
        title="Preview a file",
        detail=detail,
        action=protocol.action_none(),
        destructive=False,
        preview=preview,
    )


async def open_path(ctx: RunContext[RunDeps], path: str) -> str:
    """Open a file or folder with the OS default handler (host-executed, approved)."""
    deps = ctx.deps
    path = _clean_single_line(path, protocol.MAX_PATH_CHARS)
    if not path:
        return deps.reject("open_path", "path must not be empty")
    detail = f"Open {path!r} with its default handler."
    preview = f"open_path(path={path!r}) approved; delegated to the host"
    return await deps.request_host_action(
        tool="open_path",
        title="Open a path",
        detail=detail,
        action=protocol.action_open_path(path),
        destructive=False,
        preview=preview,
    )


async def run_native_command(ctx: RunContext[RunDeps], command: str) -> str:
    """Run one enumerated native command (host-executed, approved).

    `command` must be one of the id values of ps_ipc::NativeCommand; free-form
    command lines are impossible by construction.
    """
    deps = ctx.deps
    command = _clean_single_line(command, 64).lower()
    if command not in NATIVE_COMMANDS:
        allowed = ", ".join(sorted(NATIVE_COMMANDS))
        return deps.reject("run_native_command", f"unknown command id {command!r}; allowed: {allowed}")
    destructive = NATIVE_COMMANDS[command]
    detail = f"Run the built-in native command {command!r}."
    preview = f"run_native_command(command={command!r}) approved; delegated to the host"
    return await deps.request_host_action(
        tool="run_native_command",
        title="Run a native command",
        detail=detail,
        action=protocol.action_run_native(command),
        destructive=destructive,
        preview=preview,
    )


def register_tools(agent: Agent[RunDeps, str]) -> None:
    """Register the complete, fixed tool allowlist on the agent."""
    agent.tool(search_files)
    agent.tool(read_file_preview)
    agent.tool(open_path)
    agent.tool(run_native_command)
