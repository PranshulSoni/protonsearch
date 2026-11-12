import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PreviewPayload, SearchResult } from "../bridge/types";
import {
  clearPreviewCache,
  setPreviewLoader,
  useLauncherPreview,
  type LauncherPreviewState,
  type PreviewLoader,
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
    <>
      <span
        data-testid="probe"
        data-status={state.status}
        data-title={state.payload?.title ?? ""}
        data-error={state.error ?? ""}
      />
      <button data-testid="reload" onClick={state.reload} />
    </>
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
vi.useRealTimers();
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
  it("preserves a string rejection message", async () => {
    setPreviewLoader(vi.fn<PreviewLoader>().mockRejectedValue("Preview capacity is full."));
    const element = await mountRender(makeResult("capacity"));
    await wait(150);
    const probe = element.querySelector('[data-testid="probe"]');
    expect(probe?.getAttribute("data-status")).toBe("error");
    expect(probe?.getAttribute("data-error")).toBe("Preview capacity is full.");
  });

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

  it("refreshes when the preview is reopened or retried", async () => {
    const loader = vi
      .fn<PreviewLoader>()
      .mockResolvedValueOnce(makePayload("same", "Old preview"))
      .mockResolvedValueOnce(makePayload("same", "Reopened preview"))
      .mockResolvedValueOnce(makePayload("same", "Retried preview"));
    setPreviewLoader(loader);
    const element = await mountRender(makeResult("same"));
    await wait(150);
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-title")).toBe(
      "Old preview",
    );

    await mountRender(null);
    await mountRender(makeResult("same"));
    await wait(150);
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-title")).toBe(
      "Reopened preview",
    );

    await act(async () => {
      element.querySelector<HTMLButtonElement>('[data-testid="reload"]')?.click();
    });
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-status")).toBe(
      "loading",
    );
    await wait(150);
    expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-title")).toBe(
      "Retried preview",
    );
    expect(loader).toHaveBeenCalledTimes(3);
  });
it("does not launch additional requests when retrying a stalled preview", async () => {
vi.useFakeTimers();
const loader = vi.fn<PreviewLoader>(() => new Promise<PreviewPayload>(() => {}));
setPreviewLoader(loader);
const element = await mountRender(makeResult("stuck"));
await act(async () => {
vi.advanceTimersByTime(8_120);
});
expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-status")).toBe("error");
for (let retry = 0; retry < 3; retry += 1) {
await act(async () => {
element.querySelector<HTMLButtonElement>('[data-testid="reload"]')?.click();
});
await act(async () => {
vi.advanceTimersByTime(120);
});
expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-status")).toBe("loading");
await act(async () => {
vi.advanceTimersByTime(8_000);
});
expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-status")).toBe("error");
}
expect(loader).toHaveBeenCalledTimes(1);
});
it("allows a new retry after the timed-out request eventually settles", async () => {
vi.useFakeTimers();
let resolveStalled!: (payload: PreviewPayload) => void;
const loader = vi
.fn<PreviewLoader>()
.mockImplementationOnce(
() => new Promise<PreviewPayload>((resolve) => {
resolveStalled = resolve;
}),
)
.mockResolvedValueOnce(makePayload("eventual", "Fresh retry"));
setPreviewLoader(loader);
const element = await mountRender(makeResult("eventual"));
await act(async () => {
vi.advanceTimersByTime(8_120);
});
expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-status")).toBe("error");
await act(async () => {
resolveStalled(makePayload("eventual", "Late result"));
await Promise.resolve();
await Promise.resolve();
});
expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-title")).toBe("");
await act(async () => {
element.querySelector<HTMLButtonElement>('[data-testid="reload"]')?.click();
});
await act(async () => {
vi.advanceTimersByTime(120);
await Promise.resolve();
});
expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-status")).toBe("ready");
expect(element.querySelector('[data-testid="probe"]')?.getAttribute("data-title")).toBe(
"Fresh retry",
);
expect(loader).toHaveBeenCalledTimes(2);
});
});
