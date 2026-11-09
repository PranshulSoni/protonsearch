import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import type { AgentTask, SearchResult } from "../bridge/types";
import { AgentsCard, ClipboardCard } from "./IslandCards";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | null = null;
let container: HTMLDivElement | null = null;

async function mount(element: React.ReactNode): Promise<HTMLDivElement> {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root!.render(element);
    await new Promise((resolve) => setTimeout(resolve, 20));
  });
  return container;
}

afterEach(async () => {
  if (root) {
    await act(async () => {
      root?.unmount();
    });
  }
  container?.remove();
  root = null;
  container = null;
});

const clip = (id: string, label: string, title: string, thumb?: string): SearchResult => ({
  id,
  kind: "clipboard",
  title,
  subtitle: "Copied",
  icon: { type: "monogram" },
  score: 1,
  meta: { label, modified: Date.now() - 60_000, thumbnailDataUrl: thumb ?? null },
  action: { type: "copy_text", text: title },
});

describe("island cards real data", () => {
  it("filters clipboard tabs and renders real image thumbnails", async () => {
    const thumb =
      "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2n0kAAAAASUVORK5CYII=";
    const element = await mount(
      <ClipboardCard
        results={[
          clip("text", "TEXT", "remember the budget"),
          clip("image", "IMAGE", "diagram.png", thumb),
        ]}
        pending={false}
        error={null}
        tab="all"
        onTab={() => undefined}
        onOpen={() => undefined}
      />,
    );
    expect(element.querySelectorAll(".island-clip-row").length).toBe(2);

    // Drive the Images tab through the same filter used by the card.
    const imageOnly = [
      clip("text", "TEXT", "remember the budget"),
      clip("image", "IMAGE", "diagram.png", thumb),
    ].filter((row) => row.meta?.label === "IMAGE");
    const reRendered = await mount(
      <ClipboardCard
        results={imageOnly}
        pending={false}
        error={null}
        tab="images"
        onTab={() => undefined}
        onOpen={() => undefined}
      />,
    );
    expect(reRendered.querySelectorAll(".island-clip-row").length).toBe(1);
    expect(reRendered.querySelector(".island-clip-thumb")?.getAttribute("src")).toBe(thumb);
  });

  it("renders flat real agent tasks and disables controls the task cannot use", async () => {
    const task: AgentTask = {
      id: "task:1",
      runtime: "pi",
      runtimeVersion: "1.5.0",
      parentId: null,
      title: "Index the docs workspace",
      promptSummary: "Index docs",
      state: "running",
      progress: { kind: "determinate", fraction: 0.42 },
      createdAt: Date.now() - 1000,
      startedAt: Date.now() - 900,
      endedAt: null,
      updatedAt: Date.now(),
      log: [{ at: Date.now(), level: "info", message: "Indexed 42 files" }],
      error: null,
      permissions: { requested: [], granted: ["fs.read"], denied: [], pending: null, readOnly: true },
      writeAccess: false,
      controls: { canCancel: true, canPause: false, canResume: false, canRetry: false },
      restarts: [],
      model: "pi-1.5",
    };
    const element = await mount(
      <AgentsCard
        status={{ state: "running", runtime: "pi", model: "pi-1.5" }}
        tasks={[task]}
        onPause={() => undefined}
        onResume={() => undefined}
      />,
    );
    expect(element.textContent).toContain("Index the docs workspace");
    expect(element.textContent).toContain("CURRENT WORKSPACE · 1 TASKS");
    expect(element.querySelector(".island-agent-row")?.textContent).toContain("RUNNING");
    // Pi is flat: no fabricated child rows; controls follow task.controls.
    expect(element.querySelectorAll(".island-agent-row").length).toBe(1);
    const buttons = [...element.querySelectorAll<HTMLButtonElement>(".island-agent-button")];
    expect(buttons.find((button) => button.textContent?.includes("Pause"))?.disabled).toBe(true);
    expect(buttons.find((button) => button.textContent?.includes("Resume"))?.disabled).toBe(true);
    expect(element.querySelector(".island-progress-fill")).not.toBeNull();
  });
});
