"""Per-run approval broker.

Tools never touch the OS. They emit an `approval_request` AgentEvent and await
a boolean from the host (`approval_response` control message). The broker owns
those futures; all calls happen on the event loop thread (the stdin reader
thread dispatches with `loop.call_soon_threadsafe`).
"""

from __future__ import annotations

import asyncio
import uuid


class ApprovalBroker:
    def __init__(self, loop: asyncio.AbstractEventLoop) -> None:
        self._loop = loop
        self._pending: dict[str, asyncio.Future[bool]] = {}

    def create(self) -> tuple[str, asyncio.Future[bool]]:
        approval_id = f"apr-{uuid.uuid4().hex[:16]}"
        future: asyncio.Future[bool] = self._loop.create_future()
        self._pending[approval_id] = future
        return approval_id, future

    def resolve(self, approval_id: str, approved: bool) -> bool:
        """Resolve a pending approval. Returns False for unknown/stale ids."""
        future = self._pending.pop(approval_id, None)
        if future is None or future.done():
            return False
        future.set_result(bool(approved))
        return True

    def deny_pending(self, run_id: str | None = None) -> int:
        """Deny everything still waiting (cancel/shutdown path)."""
        count = 0
        for approval_id, future in list(self._pending.items()):
            if future.done():
                self._pending.pop(approval_id, None)
                continue
            future.set_result(False)
            self._pending.pop(approval_id, None)
            count += 1
        return count

    def pending_count(self) -> int:
        return len(self._pending)
