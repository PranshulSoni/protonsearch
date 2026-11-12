import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SearchResult } from "../../bridge/types";
import { BrowserCard } from "./BrowserCard";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
// jsdom lacks these; Radix ScrollArea calls ResizeObserver on mount.
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

const MINUTE = 60_000;
const PINS_KEY = "ps.island.browser.pins";
const HIDDEN_KEY = "ps.island.browser.hidden";

/** Noon of a day `daysAgo` back, shifted by `hourOffset`, so day groups never depend on the clock. */
function at(daysAgo: number, hourOffset = 0): number {
  const date = new Date();
  date.setDate(date.getDate() - daysAgo);
  date.setHours(12 + hourOffset, 0, 0, 0);
  return date.getTime();
}

const page = (
  id: string,
  title: string,
  url: string,
  source: string,
  modified: number,
  kind: SearchResult["kind"] = "browser_history",
): SearchResult => ({
  id,
  kind,
  title,
  subtitle: url,
  path: url,
  icon: { type: "kind", kind: "bookmark" },
  score: 0,
  meta: { modified, label: `${source} · ${kind === "bookmark" ? "Bookmark" : "3 visits"}` },
  action: { type: "open_url", url },
});

const pages: SearchResult[] = [
  page("a", "GitHub · protonsearch", "https://github.com/pranshul/protonsearch", "Chrome", at(0)),
  page("b", "Docs · Tauri", "https://tauri.app/docs", "Edge", at(0, -1)),
  page("c", "Yesterday news", "https://news.example.com/today", "Chrome", at(1)),
  page("d", "Saved guide", "https://guide.example.org/start", "Chrome", at(3), "bookmark"),
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

beforeEach(() => {
  window.localStorage.clear();
});

afterEach(unmount);

function rowIds(element: HTMLElement): Array<string | null> {
  return Array.from(element.querySelectorAll("[data-row-id]")).map((row) => row.getAttribute("data-row-id"));
}

function row(element: HTMLElement, id: string): HTMLElement {
  const found = element.querySelector<HTMLElement>(`[data-row-id="${id}"]`);
  if (!found) throw new Error(`row ${id} not rendered`);
  return found;
}

function button(scope: HTMLElement, label: string): HTMLButtonElement {
  const found = scope.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
  if (!found) throw new Error(`button ${label} not rendered`);
  return found;
}

function radio(element: HTMLElement, label: string): HTMLButtonElement {
  const found = Array.from(element.querySelectorAll<HTMLButtonElement>("[role='radio']")).find(
    (tab) => tab.textContent === label,
  );
  if (!found) throw new Error(`tab ${label} not rendered`);
  return found;
}

async function typeInto(input: HTMLInputElement, value: string): Promise<void> {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function keyDown(target: HTMLElement, key: string): Promise<void> {
  await act(async () => {
    target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  });
}

describe("BrowserCard", () => {
  it("shows the page count, real browsers and day groups", async () => {
    const element = await mount(<BrowserCard results={pages} pending={false} error={null} onOpen={vi.fn()} />);
    expect(element.querySelector("h2")?.textContent).toBe("Browser History");
    expect(element.textContent).not.toContain("4 pages");
    expect(element.textContent).toContain("2 browsers · Chrome, Edge");
    expect(element.textContent).toContain("Today2");
    expect(element.textContent).toContain("Yesterday1");
    expect(element.textContent).toContain("github.com · Chrome");
    expect(element.textContent).not.toMatch(/demo|select a page/i);
  });

  it("filters the visible pages with the search field", async () => {
    const element = await mount(<BrowserCard results={pages} pending={false} error={null} onOpen={vi.fn()} />);
    const input = element.querySelector("input");
    if (!input) throw new Error("search field not rendered");
    await typeInto(input, "tauri");
    expect(rowIds(element)).toEqual(["b"]);

    await typeInto(input, "nothing matches this");
    expect(element.textContent).toContain("No pages match.");
  });

  it("filters to today and pinned pages with the tabs", async () => {
    const element = await mount(<BrowserCard results={pages} pending={false} error={null} onOpen={vi.fn()} />);
    await act(async () => {
      radio(element, "Today").click();
    });
    expect(rowIds(element)).toEqual(["a", "b"]);

    await act(async () => {
      radio(element, "Pinned").click();
    });
    expect(rowIds(element)).toEqual([]);
    expect(element.textContent).toContain("No pinned pages.");
  });

  it("persists pins in localStorage and restores them after a remount", async () => {
    const first = await mount(<BrowserCard results={pages} pending={false} error={null} onOpen={vi.fn()} />);
    await act(async () => {
      button(row(first, "a"), "Pin page").click();
    });
    expect(JSON.parse(window.localStorage.getItem(PINS_KEY) ?? "[]")).toEqual(["a"]);
    expect(button(row(first, "a"), "Unpin page").getAttribute("aria-pressed")).toBe("true");
    await unmount();

    const again = await mount(<BrowserCard results={pages} pending={false} error={null} onOpen={vi.fn()} />);
    await act(async () => {
      radio(again, "Pinned").click();
    });
    expect(rowIds(again)).toEqual(["a"]);
  });

  it("hides a page from the list without touching the underlying results", async () => {
    const onOpen = vi.fn();
    const element = await mount(<BrowserCard results={pages} pending={false} error={null} onOpen={onOpen} />);
    await act(async () => {
      button(row(element, "b"), "Hide page").click();
    });
    expect(rowIds(element)).toEqual(["a", "c", "d"]);
    expect(JSON.parse(window.localStorage.getItem(HIDDEN_KEY) ?? "[]")).toEqual(["b"]);
    expect(onOpen).not.toHaveBeenCalled();
    expect(pages).toHaveLength(4);
  });

  it("restores hidden pages with Show hidden and offers it only while pages are hidden", async () => {
    const element = await mount(<BrowserCard results={pages} pending={false} error={null} onOpen={vi.fn()} />);
    expect(element.textContent).not.toContain("Show hidden");
    await act(async () => {
      button(row(element, "b"), "Hide page").click();
      button(row(element, "c"), "Hide page").click();
    });
    const restore = Array.from(element.querySelectorAll("button")).find((item) => item.textContent === "Show hidden (2)");
    if (!restore) throw new Error("Show hidden not rendered");
    await act(async () => {
      restore.click();
    });
    expect(rowIds(element)).toEqual(["a", "b", "c", "d"]);
    expect(JSON.parse(window.localStorage.getItem(HIDDEN_KEY) ?? "[]")).toEqual([]);
    expect(element.textContent).not.toContain("Show hidden");
  });

  it("shows the site's favicon and falls back to the globe without one or when it fails to load", async () => {
    const icon = "data:image/png;base64,AAAA";
    const withIcon = pages.map((result) =>
      result.id === "a" ? { ...result, meta: { ...result.meta, faviconDataUrl: icon } } : result,
    );
    const element = await mount(<BrowserCard results={withIcon} pending={false} error={null} onOpen={vi.fn()} />);
    const image = row(element, "a").querySelector("img");
    expect(image?.getAttribute("src")).toBe(icon);
    expect(row(element, "b").querySelector("img")).toBeNull();
    expect(row(element, "b").querySelector("svg")).not.toBeNull();

    await act(async () => {
      image?.dispatchEvent(new Event("error"));
    });
    expect(row(element, "a").querySelector("img")).toBeNull();
    expect(row(element, "a").querySelector("svg")).not.toBeNull();
  });

  it("opens a page on click and on Enter after the arrow keys move the highlight", async () => {
    const onOpen = vi.fn();
    const element = await mount(<BrowserCard results={pages} pending={false} error={null} onOpen={onOpen} />);
    await act(async () => {
      row(element, "c").querySelector<HTMLButtonElement>("button")?.click();
    });
    expect(onOpen.mock.calls[0]?.[0].id).toBe("c");

    const input = element.querySelector("input");
    if (!input) throw new Error("search field not rendered");
    expect(document.activeElement).toBe(input);
    await keyDown(input, "ArrowDown");
    await keyDown(input, "Enter");
    expect(onOpen.mock.calls[1]?.[0].id).toBe("b");
  });

  it("virtualizes lists above 40 page rows", async () => {
    // jsdom has no layout; give elements a 400 px client height so the virtualizer has a window.
    Object.defineProperty(HTMLElement.prototype, "clientHeight", { configurable: true, get: () => 400 });
    try {
      const many = Array.from({ length: 60 }, (_, index) =>
        page(`p${index}`, `Page ${index}`, `https://site${index}.example.com/`, "Chrome", at(0) - index * MINUTE),
      );
      const element = await mount(<BrowserCard results={many} pending={false} error={null} onOpen={vi.fn()} />);
      // One header plus 60 rows of 56 px: the sizer carries the full height, rows are windowed.
      expect(element.querySelector("[style*='3388px']")).not.toBeNull();
      const rendered = rowIds(element).length;
      expect(rendered).toBeGreaterThan(0);
      expect(rendered).toBeLessThan(60);
    } finally {
      Reflect.deleteProperty(HTMLElement.prototype, "clientHeight");
    }
  });

  it("shows honest empty and error states", async () => {
    const empty = await mount(<BrowserCard results={[]} pending={false} error={null} onOpen={vi.fn()} />);
    expect(empty.textContent).toContain("No browser history yet.");
    await unmount();

    const failed = await mount(<BrowserCard results={[]} pending={false} error="Index offline" onOpen={vi.fn()} />);
    expect(failed.textContent).toContain("Browser history is unavailable.");
    expect(failed.textContent).toContain("Index offline");
  });
});
