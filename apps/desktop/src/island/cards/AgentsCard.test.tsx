import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { AgentStatus, AgentTask } from "../../bridge/types";
import { api } from "../../bridge/api";
import { AgentsCard } from "./AgentsCard";

vi.mock("../../bridge/api", () => ({
  api: {
    agentStart: vi.fn(async () => undefined),
    agentDelete: vi.fn(async () => undefined),
    agentRename: vi.fn(async () => undefined),
    agentModels: vi.fn(async () => []),
  },
}));
vi.mock("../../app/agentRun", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../app/agentRun")>()),
  buildAgentRunRequest: (runId: string, prompt: string, model?: string | null) => ({ runId, prompt, model: model ?? null }),
  createAgentRunId: () => "run-1",
}));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const RUNNING: AgentStatus = { state: "running", runtime: "pi", model: null, detail: null };

function task(overrides: Partial<AgentTask> = {}): AgentTask {
  return {
    id: "t1",
    runtime: "pi",
    runtimeVersion: null,
    parentId: null,
    title: "Summarize inbox",
    promptSummary: "Reads 12 threads",
    state: "running",
    progress: { kind: "determinate", fraction: 0.72 },
    createdAt: 0,
    startedAt: 0,
    endedAt: null,
    updatedAt: 0,
    log: [],
    error: null,
    permissions: { requested: [], granted: ["read"], denied: [], pending: null, readOnly: true },
    writeAccess: false,
    controls: { canCancel: true, canPause: true, canResume: false, canRetry: false },
    restarts: [],
    model: null,
    ...overrides,
  };
}

const FAILED = task({
  id: "t2",
  title: "Index docs",
  state: "failed",
  progress: { kind: "indeterminate", fraction: null },
  error: { code: "rate", message: "Rate limited" },
  controls: { canCancel: false, canPause: false, canResume: true, canRetry: true },
});

let root: Root | null = null;
let host: HTMLDivElement | null = null;

beforeAll(() => {
  vi.stubGlobal("ResizeObserver", class {
    observe() {}
    unobserve() {}
    disconnect() {}
  });
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({ matches: false, media: query, addEventListener() {}, removeEventListener() {} }),
  });
});

afterEach(async () => {
  await act(async () => root?.unmount());
  host?.remove();
  root = null;
  host = null;
  vi.clearAllMocks();
});

async function mount(ui: ReactNode): Promise<HTMLDivElement> {
  const el = document.createElement("div");
  document.body.appendChild(el);
  host = el;
  root = createRoot(el);
  const r = root;
  await act(async () => r.render(ui));
  return el;
}

function buttonWith(el: HTMLElement, text: string): HTMLButtonElement | undefined {
  return [...el.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.includes(text));
}

function setInput(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("AgentsCard", () => {
  it("says so when no agent is configured and hides the composer", async () => {
    const el = await mount(<AgentsCard status={null} tasks={[]} onPause={vi.fn()} onResume={vi.fn()} />);
    expect(el.textContent).toContain("No agent configured.");
    expect(el.querySelector("form")).toBeNull();
  });

  it("marks each agent with a status dot and shows no task count", async () => {
    const el = await mount(<AgentsCard status={RUNNING} tasks={[task(), FAILED]} onPause={vi.fn()} onResume={vi.fn()} />);
    expect(el.querySelector('[role="img"][aria-label="Working"]')).not.toBeNull();
    expect(el.querySelector('[role="img"][aria-label="Failed"]')).not.toBeNull();
    expect(el.textContent).not.toContain("tasks");
  });

  it("creates a new agent from the composer and reloads the list", async () => {
    const onChanged = vi.fn();
    const el = await mount(
      <AgentsCard status={RUNNING} tasks={[]} onPause={vi.fn()} onResume={vi.fn()} onChanged={onChanged} />,
    );
    await act(async () => setInput(el.querySelector<HTMLInputElement>('input[aria-label="Ask the agent…"]')!, "Summarize notes"));
    await act(async () => el.querySelector<HTMLButtonElement>('button[aria-label="Send"]')!.click());
    expect(api.agentStart).toHaveBeenCalledWith({ runId: "run-1", prompt: "Summarize notes", model: null }, null);
    expect(onChanged).toHaveBeenCalled();
  });

  it("opens an agent to chat, continues its thread and deletes it", async () => {
    const followUp = task({ id: "t3", parentId: "t2", promptSummary: "And now?", state: "completed", answer: "All done.", createdAt: 5 });
    const el = await mount(<AgentsCard status={RUNNING} tasks={[FAILED, followUp]} onPause={vi.fn()} onResume={vi.fn()} />);
    await act(async () => buttonWith(el, "Index docs")!.click());
    expect(el.textContent).toContain("Rate limited");
    expect(el.textContent).toContain("All done.");
    await act(async () => setInput(el.querySelector<HTMLInputElement>('input[aria-label="Message the agent…"]')!, "Try again"));
    await act(async () => el.querySelector<HTMLButtonElement>('button[aria-label="Send"]')!.click());
    expect(api.agentStart).toHaveBeenCalledWith({ runId: "run-1", prompt: "Try again", model: null }, "t2");
    await act(async () => el.querySelector<HTMLButtonElement>('button[aria-label="Delete agent"]')!.click());
    expect(api.agentDelete).toHaveBeenCalledWith("t2");
  });

  it("sends the model picked in the chip, and the default model otherwise", async () => {
    vi.mocked(api.agentModels).mockResolvedValueOnce([{ provider: "opencode", id: "claude-sonnet-4-6" }]);
    const el = await mount(<AgentsCard status={RUNNING} tasks={[]} onPause={vi.fn()} onResume={vi.fn()} />);
    const chip = () => el.querySelector<HTMLButtonElement>('button[aria-haspopup="listbox"]')!;
    expect(chip().textContent).toContain("Default");
    await act(async () => chip().click());
    const option = [...el.querySelectorAll<HTMLButtonElement>('[role="option"]')].find((item) => item.textContent?.includes("claude-sonnet-4-6"));
    await act(async () => option!.click());
    expect(chip().textContent).toContain("claude-sonnet-4-6");
    await act(async () => setInput(el.querySelector<HTMLInputElement>('input[aria-label="Ask the agent…"]')!, "Summarize notes"));
    await act(async () => el.querySelector<HTMLButtonElement>('button[aria-label="Send"]')!.click());
    expect(api.agentStart).toHaveBeenCalledWith({ runId: "run-1", prompt: "Summarize notes", model: "opencode/claude-sonnet-4-6" }, null);
  });

  it("filters the model list, picks with the keyboard and closes on Escape", async () => {
    vi.mocked(api.agentModels).mockResolvedValueOnce([
      { provider: "opencode", id: "claude-sonnet-4-6" },
      { provider: "opencode", id: "gpt-5.4" },
    ]);
    const el = await mount(<AgentsCard status={RUNNING} tasks={[]} onPause={vi.fn()} onResume={vi.fn()} />);
    const chip = () => el.querySelector<HTMLButtonElement>('button[aria-haspopup="listbox"]')!;
    const key = (input: HTMLInputElement, name: string) => input.dispatchEvent(new KeyboardEvent("keydown", { key: name, bubbles: true }));
    await act(async () => chip().click());
    const filter = el.querySelector<HTMLInputElement>('input[aria-label="Filter models"]')!;
    await act(async () => setInput(filter, "gpt"));
    expect([...el.querySelectorAll('[role="option"]')].map((item) => item.textContent)).toEqual(["gpt-5.4opencode"]);
    await act(async () => key(filter, "Enter"));
    expect(chip().textContent).toContain("gpt-5.4");
    await act(async () => chip().click());
    await act(async () => key(el.querySelector<HTMLInputElement>('input[aria-label="Filter models"]')!, "Escape"));
    expect(el.querySelector('[role="listbox"][aria-label="Models"]')).toBeNull();
  });

  it("renames an agent from its chat header", async () => {
    const el = await mount(<AgentsCard status={RUNNING} tasks={[FAILED]} onPause={vi.fn()} onResume={vi.fn()} />);
    await act(async () => buttonWith(el, "Index docs")!.click());
    await act(async () => el.querySelector<HTMLButtonElement>('button[aria-label="Rename agent"]')!.click());
    const input = el.querySelector<HTMLInputElement>('input[aria-label="Agent title"]')!;
    expect(input.value).toBe("Index docs");
    await act(async () => setInput(input, "Reindex docs"));
    await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    expect(api.agentRename).toHaveBeenCalledWith("t2", "Reindex docs");
  });
});
