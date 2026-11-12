"""Deterministic scripted model used by `PROTONSEARCH_AGENT_TEST_MODEL=1`.

This is a *supported* CI/E2E path, not the product default: it exercises the
real pydantic-ai agent loop, the real typed tools and the real approval
protocol, but it makes no network call and needs no API key.

Script (per run):
  1. call `search_files` (approval required)
  2. if the request supplied contextPaths, call `read_file_preview` on the first
     one (approval required); otherwise skip straight to the answer
  3. produce a short final answer naming the run id

The script is driven by the message history the framework passes in, so tool
arguments and approvals still flow through the normal agent loop.
"""

from __future__ import annotations

from pydantic_ai.messages import ModelRequest, ModelResponse, TextPart, ToolCallPart, ToolReturnPart
from pydantic_ai.models.function import FunctionModel


def _tool_returns(messages) -> int:
    count = 0
    for message in messages:
        if isinstance(message, ModelRequest):
            for part in message.parts:
                if isinstance(part, ToolReturnPart):
                    count += 1
    return count


def build_scripted_test_model(run_id: str, prompt: str, context_paths: tuple[str, ...]) -> FunctionModel:
    query = " ".join(prompt.split())[:120] or "protonsearch sidecar smoke test"
    preview_path = context_paths[0] if context_paths else None

    def script(messages, info) -> ModelResponse:  # noqa: ANN001 - framework signature
        returns = _tool_returns(messages)
        if returns == 0:
            return ModelResponse(parts=[ToolCallPart("search_files", {"query": query, "limit": 5})])
        if returns == 1 and preview_path is not None:
            return ModelResponse(
                parts=[ToolCallPart("read_file_preview", {"path": preview_path, "max_bytes": 2048})]
            )
        text = (
            f"TEST MODEL: scripted plan completed for run {run_id} with {returns} approved "
            "host call(s). No model provider and no network were used."
        )
        return ModelResponse(parts=[TextPart(text)])

    return FunctionModel(script, model_name="protonsearch-scripted-test")
