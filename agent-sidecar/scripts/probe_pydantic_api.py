
import asyncio, inspect, traceback
from pydantic_ai import Agent, RunContext
from pydantic_ai.models.function import FunctionModel, StreamFunctionDef, FunctionDef
from pydantic_ai.messages import ModelResponse, ModelRequest, ToolCallPart, TextPart, UserPromptPart

print("StreamFunctionDef", StreamFunctionDef)
print("ToolCallPart sig", inspect.signature(ToolCallPart))
print("TextPart sig", inspect.signature(TextPart))
print("ModelResponse sig", inspect.signature(ModelResponse))

class Deps:
    def __init__(self):
        self.calls = []

async def search_files(ctx: RunContext[Deps], query: str, limit: int = 5) -> str:
    """Search indexed files by name."""
    ctx.deps.calls.append((query, limit))
    return f"delegated:search_files(query={query}, limit={limit})"

def model_fn(messages, info):
    # count tool returns to decide
    tool_returns = [m for m in messages if isinstance(m, ModelRequest) for p in m.parts if type(p).__name__ == "ToolReturnPart"]
    if not tool_returns:
        return ModelResponse(parts=[ToolCallPart("search_files", {"query": "budget", "limit": 3})])
    return ModelResponse(parts=[TextPart("Done. Found the file.")])

agent = Agent(FunctionModel(model_fn), deps_type=Deps, output_type=str)
agent.tool(search_files)

async def main():
    deps = Deps()
    res = await agent.run("find budget", deps=deps)
    print("OUTPUT", repr(res.output))
    print("DEPS_CALLS", deps.calls)

asyncio.run(main())
