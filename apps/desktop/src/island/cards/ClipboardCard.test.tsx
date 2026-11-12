import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "../../bridge/api";
import type { SearchResult } from "../../bridge/types";
import { ClipboardCard } from "./ClipboardCard";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
// jsdom lacks these; cmdk and Radix ScrollArea call them unguarded.
Object.assign(Element.prototype, { scrollIntoView: () => {} });
if (typeof ResizeObserver === "undefined") {
  Object.assign(globalThis, {
    ResizeObserver: class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  });
}

const MIN = 60_000;
const NOW = Date.now();

const clip = (id: string, title: string, minutesAgo: number): SearchResult => ({
  id,
  kind: "clipboard",
  title,
  subtitle: "Copied",
  icon: { type: "monogram" },
  score: 1,
  meta: { modified: NOW - minutesAgo * MIN },
  action: { type: "copy_text", text: title },
});

const results: SearchResult[] = [
  clip("link", "https://example.com/pricing", 5),
  clip("code", "const total = 4;", 9),
  clip("text", "Order 4417 ships on Friday.", 0),
  {
    ...clip("image", "Screenshot", 20),
    meta: { label: "IMAGE", modified: NOW - 20 * MIN, thumbnailDataUrl: "data:image/png;base64,AAAA" },
  },
  clip("file", "C:\\Users\\me\\notes.txt", 60),
];

let root: Root | null = null;
let container: HTMLDivElement | null = null;

async function mount(element: ReactNode): Promise<HTMLDivElement> {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(element);
  });
  return container;
}

async function unmount(): Promise<void> {
  if (root) {
    await act(async () => root?.unmount());
  }
  container?.remove();
  root = null;
  container = null;
}

afterEach(unmount);
afterEach(() => vi.restoreAllMocks());

function rowValues(element: HTMLElement): Array<string | null> {
  return Array.from(element.querySelectorAll("[cmdk-item]")).map((row) => row.getAttribute("data-value"));
}

function selected(element: HTMLElement): string | null | undefined {
  return element.querySelector("[cmdk-item][data-selected='true']")?.getAttribute("data-value");
}

function radio(element: HTMLElement, label: string): HTMLButtonElement {
  const found = Array.from(element.querySelectorAll<HTMLButtonElement>("[role='radio']")).find(
    (button) => button.textContent === label,
  );
  if (!found) throw new Error(`tab ${label} not rendered`);
  return found;
}

async function keyDown(target: HTMLElement, key: string): Promise<void> {
  await act(async () => {
    target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  });
}

describe("ClipboardCard", () => {
  it("lists real clips newest first with kind labels and no paste hint", async () => {
    const element = await mount(
      <ClipboardCard results={results} pending={false} error={null} tab="all" onTab={vi.fn()} onOpen={vi.fn()} />,
    );
    expect(rowValues(element)).toEqual(["text", "link", "code", "image", "file"]);
    expect(element.textContent).toContain("Text · just now");
    expect(element.textContent).toContain("Link · 5 min ago");
    expect(element.textContent).toContain("Copied just now");
    expect(element.textContent).not.toMatch(/paste/i);
  });

  it("filters by kind tab and reports tab changes", async () => {
    const onTab = vi.fn();
    const element = await mount(
      <ClipboardCard results={results} pending={false} error={null} tab="links" onTab={onTab} onOpen={vi.fn()} />,
    );
    expect(rowValues(element)).toEqual(["link"]);
    await act(async () => {
      radio(element, "Code").click();
    });
    expect(onTab).toHaveBeenCalledWith("code");
  });

  it("lists pinned clips first and filters the Pinned tab", async () => {
    const withPin = results.map((row) => (row.id === "file" ? { ...row, meta: { ...row.meta, pinned: true } } : row));
    const all = await mount(
      <ClipboardCard results={withPin} pending={false} error={null} tab="all" onTab={vi.fn()} onOpen={vi.fn()} />,
    );
    expect(rowValues(all)).toEqual(["file", "text", "link", "code", "image"]);
    await unmount();

    const pinned = await mount(
      <ClipboardCard results={withPin} pending={false} error={null} tab="pinned" onTab={vi.fn()} onOpen={vi.fn()} />,
    );
    expect(rowValues(pinned)).toEqual(["file"]);
    await unmount();

    const none = await mount(
      <ClipboardCard results={results} pending={false} error={null} tab="pinned" onTab={vi.fn()} onOpen={vi.fn()} />,
    );
    expect(none.textContent).toContain("No pinned clips yet.");
    expect(none.textContent).toContain("Pin a clip to keep it here.");
  });

  it("pins and unpins through the bridge from the row button and the P key without copying", async () => {
    const setPinned = vi.spyOn(api, "setClipboardPinned").mockResolvedValue(undefined);
    const onOpen = vi.fn();
    const withPin = results.map((row) => (row.id === "file" ? { ...row, meta: { ...row.meta, pinned: true } } : row));
    const element = await mount(
      <ClipboardCard results={withPin} pending={false} error={null} tab="all" onTab={vi.fn()} onOpen={onOpen} />,
    );
    await act(async () => {
      element.querySelector<HTMLButtonElement>("[cmdk-item][data-value='link'] button[aria-label='Pin clip']")?.click();
    });
    expect(setPinned).toHaveBeenLastCalledWith("link", true);
    await act(async () => {
      element.querySelector<HTMLButtonElement>("[cmdk-item][data-value='file'] button[aria-label='Unpin clip']")?.click();
    });
    expect(setPinned).toHaveBeenLastCalledWith("file", false);
    expect(onOpen).not.toHaveBeenCalled();

    const list = element.querySelector<HTMLElement>("[cmdk-root]");
    if (!list) throw new Error("command list not rendered");
    await keyDown(list, "ArrowDown");
    await keyDown(list, "p");
    expect(setPinned).toHaveBeenLastCalledWith("text", true);
  });

  it("names the error when the history is unavailable", async () => {

    const failed = await mount(
      <ClipboardCard results={[]} pending={false} error="Service offline" tab="all" onTab={vi.fn()} onOpen={vi.fn()} />,
    );
    expect(failed.textContent).toContain("Clipboard history is unavailable.");
    expect(failed.textContent).toContain("Service offline");
  });

  it("moves the highlight with the arrow keys and Enter copies that clip back", async () => {
    const onOpen = vi.fn();
    const element = await mount(
      <ClipboardCard results={results} pending={false} error={null} tab="all" onTab={vi.fn()} onOpen={onOpen} />,
    );
    const list = element.querySelector<HTMLElement>("[cmdk-root]");
    if (!list) throw new Error("command list not rendered");
    expect(document.activeElement).toBe(list);
    expect(selected(element)).toBe("text");

    await keyDown(list, "ArrowDown");
    expect(selected(element)).toBe("link");

    await keyDown(list, "Enter");
    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(onOpen.mock.calls[0]?.[0].id).toBe("link");
  });

  it("copies on click and clears the copied mark after 1.5 s", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    try {
      const onOpen = vi.fn();
      const element = await mount(
        <ClipboardCard results={results} pending={false} error={null} tab="all" onTab={vi.fn()} onOpen={onOpen} />,
      );
      const row = element.querySelector<HTMLElement>("[cmdk-item][data-value='code']");
      if (!row) throw new Error("code row not rendered");
      await act(async () => {
        row.click();
      });
      expect(onOpen.mock.calls[0]?.[0].id).toBe("code");
      expect(element.querySelector("svg.text-ok")).not.toBeNull();

      await act(async () => {
        vi.advanceTimersByTime(1500);
      });
      expect(element.querySelector("svg.text-ok")).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });
});
