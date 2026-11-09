import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PreviewPayload, SearchResult } from "../bridge/types";
import {
  clearPreviewCache,
  setPreviewLoader,
  useLauncherPreview,
  type LauncherPreviewState,
} from "./previewClient";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function makeResult(id: string): SearchResult {
  return {
    id,
    kind: "file",
    title: id,
    subtitle: "sub",
    icon: { type: "file_type", ext: "txt" },
    score: 1,
    action: { type: "open_path", path: "C:\\x" },
  };
}

function makePayload(id: string, title: string): PreviewPayload {
  return {
    resultId: id,
    title,
    subtitle: "",
    format: "text",
    text: title,
    snippet: null,
    highlightRanges: [],
    imageDataUrl: null,
    ocrText: null,
    fields: [],
    status: "ok",
    truncated: false,
    error: null,
    imageMime: null,
    unsupportedReason: null,
  };
}

function Probe({ result }: { result: SearchResult | null }) {
  const state: LauncherPreviewState = useLauncherPreview(result, result !== null);
  return (
    <span
      data-testid="probe"
      data-status={state.status}
      data-title={state.payload?.title ?? ""}
    />
  );
}

let root: Root | null = null;
let container: HTMLDivElement | null = null;

async function mountRender(result: SearchResult | null): Promise<HTMLDivElement> {
  if (!root) {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
  }
  await act(async () => {
    root!.render(<Probe result={result} />);
  });
  return container!;
}

async function wait(ms: number): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  });
}

beforeEach(() => {
  clearPreviewCache();
});

afterEach(async () => {
  setPreviewLoader(null);
  if (root) {
    await act(async () => {
      root?.unmount();
    });
  }
  container?.remove();
  root = null;
  container = null;
});

describe("preview client", () => {
  it("debounces, caches by result id, and exposes ready/loading states", async () => {
    const loader = vi.fn(async (resultId: string) => makePayload(resultId, "Preview " + resultId));
    setPreviewLoader(loader);
    const element = await mountRender(makeResult("a"));
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-status")).toBe("loading");
    await wait(180);
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-status")).toBe("ready");
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-title")).toBe(
      "Preview a",
    );
    expect(loader).toHaveBeenCalledTimes(1);
    await mountRender(makeResult("a"));
    await wait(20);
    expect(loader).toHaveBeenCalledTimes(1); // cache hit
  });

  it("ignores a late response for a previously selected row", async () => {
    const deferred = new Map<string, (payload: PreviewPayload) => void>();
    setPreviewLoader(
      (resultId) =>
        new Promise<PreviewPayload>((resolve) => {
          deferred.set(resultId, resolve);
        }),
    );
    const element = await mountRender(makeResult("first"));
    await wait(150);
    await mountRender(makeResult("second"));
    await wait(150);
    const resolveFirst = deferred.get("first");
    const resolveSecond = deferred.get("second");
    expect(resolveFirst).toBeTypeOf("function");
    expect(resolveSecond).toBeTypeOf("function");
    await act(async () => {
      resolveFirst!(makePayload("first", "Stale preview"));
      await Promise.resolve();
    });
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-title")).toBe("");
    await act(async () => {
      resolveSecond!(makePayload("second", "Fresh preview"));
      await Promise.resolve();
    });
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-title")).toBe(
      "Fresh preview",
    );
  });
});
