import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import IslandView from "./IslandView";
import {
  clampIslandWidth,
  expandedSearchHeight,
  islandCardDims,
  visibleIslandCards,
} from "./islandDims";

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | null = null;
let container: HTMLDivElement | null = null;

async function renderIsland(): Promise<HTMLDivElement> {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root!.render(<IslandView />);
    await new Promise((resolve) => setTimeout(resolve, 120));
  });
  return container;
}

async function waitFor(condition: () => boolean, timeout = 2500): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    if (condition()) return;
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 25));
    });
  }
  if (!condition()) throw new Error("condition not met in time");
}

async function settle(ms = 500): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  });
}

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", ResizeObserverStub);
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

afterEach(async () => {
  if (root) {
    await act(async () => {
      root?.unmount();
    });
  }
  container?.remove();
  root = null;
  container = null;
  vi.unstubAllGlobals();
});

describe("V9 island geometry", () => {
  it("matches the authoritative card dimension table", () => {
    const cases: Array<[Parameters<typeof islandCardDims>[0], { w: number; h: number }]> = [
      [{ card: "search", stageWidth: 1150, compact: true }, { w: 520, h: 66 }],
      [{ card: "search", stageWidth: 520, compact: true }, { w: 500, h: 66 }],
      [{ card: "search", stageWidth: 430, compact: true }, { w: 410, h: 66 }],
      [{ card: "search", stageWidth: 335, compact: true }, { w: 315, h: 66 }],
      [{ card: "clipboard", stageWidth: 1150, compact: false }, { w: 510, h: 391 }],
      [{ card: "clipboard", stageWidth: 520, compact: false }, { w: 500, h: 416 }],
      [{ card: "clipboard", stageWidth: 430, compact: false }, { w: 410, h: 416 }],
      [{ card: "clipboard", stageWidth: 335, compact: false }, { w: 315, h: 446 }],
      [{ card: "agent", stageWidth: 1150, compact: false }, { w: 520, h: 411 }],
      [{ card: "agent", stageWidth: 335, compact: false }, { w: 315, h: 445 }],
      [{ card: "controls", stageWidth: 1150, compact: false }, { w: 515, h: 376 }],
      [{ card: "controls", stageWidth: 335, compact: false }, { w: 315, h: 483 }],
      [{ card: "media", stageWidth: 1150, compact: false }, { w: 490, h: 286 }],
      [{ card: "git", stageWidth: 520, compact: false }, { w: 500, h: 355 }],
      [{ card: "browser", stageWidth: 335, compact: false }, { w: 315, h: 408 }],
      [{ card: "bell", stageWidth: 1150, compact: false }, { w: 500, h: 345 }],
    ];
    for (const [input, expected] of cases) {
      const dims = islandCardDims(input);
      expect({ w: dims.width, h: dims.height }).toEqual(expected);
    }
  });

  it("derives expanded search height and clamps work-area widths", () => {
    expect(expandedSearchHeight(0)).toBe(185);
    expect(expandedSearchHeight(3)).toBe(344);
    expect(expandedSearchHeight(5)).toBe(450);
    expect(expandedSearchHeight(9)).toBe(450);
    expect(clampIslandWidth(550, 335)).toBe(315);
    expect(islandCardDims({ card: "media", stageWidth: 1150, compact: false }).radius).toBe(41);
    expect(islandCardDims({ card: "search", stageWidth: 1150, compact: true }).radius).toBe(999);
  });

  it("renders enabled cards in saved order for the dots", () => {
    expect(
      visibleIslandCards({
        enabled: true,
        placement: {
          monitor: null,
          anchor: "top_center",
          offsetX: 0,
          offsetY: 22,
          keepInWorkArea: true,
          autoFlipExpanded: true,
        },
        cards: [
          { id: "git", enabled: true },
          { id: "search", enabled: true },
          { id: "media", enabled: false },
        ],
        defaultCard: "search",
        showDots: true,
        hideOnFocusLoss: true,
        skipDisabled: true,
        returnToSearch: true,
      }),
    ).toEqual(["git", "search"]);
  });
});

describe("V9 island surface", () => {
  it("renders the compact capsule with eight synchronised dots", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelector(".island-surface") !== null);
    const surface = element.querySelector(".island-surface") as HTMLElement;
    expect(surface.style.width).toBe("520px");
    expect(surface.style.height).toBe("66px");
    expect(surface.style.borderRadius).toBe("999px");
    expect(element.querySelectorAll(".island-dot-button").length).toBe(8);
    expect(
      element.querySelector(".island-dot-button.is-active")?.getAttribute("aria-label"),
    ).toBe("Go to Search card");
  });

  it("expands search on focus and shows real results with filters", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelector(".island-search-input") !== null);
    const input = element.querySelector(".island-search-input") as HTMLInputElement;
    await act(async () => {
      input.focus();
      await new Promise((resolve) => setTimeout(resolve, 120));
    });
    await waitFor(() => element.querySelector(".island-surface--expanded") !== null);
    expect(element.querySelectorAll(".island-filter").length).toBe(4);
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    await act(async () => {
      setter.call(input, "architecture");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    await waitFor(() => element.querySelectorAll(".island-result-row").length > 0);
    expect(element.querySelector(".island-result-foot")?.textContent).toContain("local results");
  });

  it("switches card with the dots and exposes real clipboard tabs", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await act(async () => {
      (element.querySelectorAll(".island-dot-button")[2] as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 450));
    });
    expect(element.querySelector(".island-card-heading h2")?.textContent).toBe("Clipboard");
    const chips = [...element.querySelectorAll(".island-nav-chip")];
    expect(chips.some((chip) => chip.textContent?.includes("Pinned"))).toBe(true);
    expect(chips.find((chip) => chip.textContent?.includes("Pinned"))?.hasAttribute("disabled")).toBe(true);
  });

  it("shows a capability-honest state for cards without native adapters", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await act(async () => {
      (element.querySelectorAll(".island-dot-button")[1] as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 450));
    });
    expect(element.querySelector(".island-card-heading h2")?.textContent).toBe("Media");
    expect(element.querySelector(".island-unsupported")?.textContent).toContain("Not available yet");
    expect(element.querySelector(".island-unsupported")?.textContent).toContain("No mock data is shown.");
  });

  it("honours reduced motion: card swaps are instant and never play the out phase", async () => {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: query.includes("prefers-reduced-motion"),
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await act(async () => {
      (element.querySelectorAll(".island-dot-button")[1] as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(element.querySelector(".island-card-heading h2")?.textContent).toBe("Media");
    expect(element.querySelector(".island-surface--out")).toBeNull();
  });

  it("switches cards with Ctrl+ArrowDown, wheel threshold and Esc", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", ctrlKey: true }));
      await new Promise((resolve) => setTimeout(resolve, 450));
    });
    expect(element.querySelector(".island-dot-button.is-active")?.getAttribute("aria-label")).toBe(
      "Go to Media card",
    );
    await act(async () => {
      element.querySelector(".island-surface")!.dispatchEvent(
        new WheelEvent("wheel", { deltaY: 60, bubbles: true, cancelable: true }),
      );
      await new Promise((resolve) => setTimeout(resolve, 600));
    });
    expect(element.querySelector(".island-dot-button.is-active")?.getAttribute("aria-label")).toBe(
      "Go to Clipboard card",
    );
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
      await new Promise((resolve) => setTimeout(resolve, 120));
    });
    await waitFor(() => element.querySelector(".island-surface--compact") !== null);
    await settle(100);
  });
});
