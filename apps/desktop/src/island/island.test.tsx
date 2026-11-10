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
import { api } from "../bridge/api";

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
  it("searches using the selected filter", async () => {
    const search = vi.spyOn(api, "search");
    const element = await renderIsland();
    const input = element.querySelector(".island-search-input") as HTMLInputElement;
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    await act(async () => {
      setter.call(input, "report");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    for (const [label, scope] of [["Apps", "apps"], ["Files", "files"], ["Folders", "folders"], ["Actions", "actions"], ["All", "all"]]) {
      await act(async () => {
        const button = Array.from(element.querySelectorAll<HTMLButtonElement>(".island-filter")).find((item) => item.textContent === label)!;
        button.click();
      });
      await waitFor(() => search.mock.calls.some(([request]) => request.query === "report" && request.scope === scope));
    }
    search.mockRestore();
  });
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

  it("keeps blank Search compact on focus and clicks, expands for a query, and collapses when cleared", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelector(".island-search-input") !== null);
    const input = element.querySelector(".island-search-input") as HTMLInputElement;
    expect(input.placeholder).toBe("Search files, folders, apps…");
    await act(async () => {
      input.focus();
      await new Promise((resolve) => setTimeout(resolve, 150));
    });
    expect(element.querySelector(".island-surface--compact")).not.toBeNull();
    await act(async () => {
      (element.querySelector(".island-surface") as HTMLElement).click();
    });
    expect(element.querySelector(".island-surface--compact")).not.toBeNull();
    expect(document.activeElement).toBe(input);
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    await act(async () => {
      setter.call(input, "architecture");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 250));
    });
    await waitFor(() => element.querySelector(".island-surface--expanded") !== null);
    expect(element.querySelectorAll(".island-filter").length).toBe(5);
    await waitFor(() => element.querySelectorAll(".island-result-row").length > 0);
    expect(element.querySelectorAll(".island-group-label").length).toBeGreaterThan(0);
    expect(element.querySelector(".island-result-foot")?.textContent).toContain("local results");
    await act(async () => {
      setter.call(input, "");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 30));
    });
    expect(element.querySelector(".island-surface--compact")).not.toBeNull();
  });

  it("removes the empty-search hint and uses the query-driven expanded height", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelector(".island-search-input") !== null);
    const input = element.querySelector(".island-search-input") as HTMLInputElement;
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    await act(async () => {
      setter.call(input, "zzzz no match");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      await new Promise((resolve) => setTimeout(resolve, 300));
    });
    const surface = element.querySelector(".island-surface") as HTMLElement;
    expect(surface.style.height).toBe("185px");
    expect(element.querySelector(".island-result-row")).toBeNull();
    expect(element.querySelector(".island-empty--compact")).toBeNull();
    expect(surface.textContent).not.toContain("Start typing");
    expect(element.querySelector(".island-result-foot")).toBeNull();
  });

  it("switches card with the dots and exposes real clipboard tabs", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await act(async () => {
      (element.querySelectorAll(".island-dot-button")[2] as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 450));
    });
    expect(
      element.querySelector('.island-card-slot[aria-hidden="false"] .island-card-heading h2')
        ?.textContent,
    ).toBe("Clipboard");
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
    expect(
      element.querySelector('.island-card-slot[aria-hidden="false"] .island-card-heading h2')
        ?.textContent,
    ).toBe("Media");
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
    expect(
      element.querySelector('.island-card-slot[aria-hidden="false"] .island-card-heading h2')
        ?.textContent,
    ).toBe("Media");
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

  it("moves one card per scroll gesture, however many wheel events it produces", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    const surface = element.querySelector(".island-surface")!;
    const active = () =>
      element.querySelector(".island-dot-button.is-active")?.getAttribute("aria-label");
    const stream = async (deltas: number[]) => {
      for (const deltaY of deltas) {
        surface.dispatchEvent(new WheelEvent("wheel", { deltaY, bubbles: true, cancelable: true }));
        await new Promise((resolve) => setTimeout(resolve, 12));
      }
    };
    const inertia = [30, 24, 19, 15, 12, 9, 7, 5, 4, 3, 2, 2, 1, 1];

    // One touchpad swipe: a ramp, then a long decaying tail (298 px in total).
    await act(async () => {
      await stream([4, 12, 26, 40, 46, 38, ...inertia]);
      await new Promise((resolve) => setTimeout(resolve, 700));
    });
    expect(active()).toBe("Go to Media card");

    // Another swipe whose inertia is still streaming...
    await act(async () => {
      await stream([30, 50, 40, ...inertia, ...Array(14).fill(1)]);
    });
    expect(active()).toBe("Go to Clipboard card");
    // ...when the next swipe starts on top of it: the deltas jump back up.
    await act(async () => {
      await stream([10, 30, 60, 40, ...inertia]);
      await new Promise((resolve) => setTimeout(resolve, 900));
    });
    expect(active()).toBe("Go to Agents card");
  });

  it("keeps now-playing live only while the Media card is on screen", async () => {
    const session = {
      title: "First track",
      artist: "Artist",
      album: null,
      artworkDataUrl: null,
      playing: true,
      positionMs: 1000,
      durationMs: 60000,
      canSeek: false,
      canShuffle: false,
      shuffle: false,
      canRepeat: false,
      repeat: false,
      appId: "Player",
    };
    const base = await api.getControlsStatus();
    const status = vi.spyOn(api, "getControlsStatus").mockResolvedValue({
      ...base,
      mediaCapability: { supported: true, reason: null },
      media: session,
    });
    const poll = vi.spyOn(api, "getMediaSession").mockResolvedValue({ ...session, title: "Second track" });
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await settle(1200);
    expect(poll).not.toHaveBeenCalled();

    await act(async () => {
      (element.querySelector('.island-dot-button[aria-label="Go to Media card"]') as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 300));
    });
    expect(element.querySelector(".island-media-title")?.textContent).toBe("First track");
    await settle(1300);
    expect(element.querySelector(".island-media-title")?.textContent).toBe("Second track");

    await act(async () => {
      (element.querySelector('.island-dot-button[aria-label="Go to Search card"]') as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 400));
    });
    const polled = poll.mock.calls.length;
    await settle(1300);
    expect(poll.mock.calls.length).toBe(polled);
    status.mockRestore();
    poll.mockRestore();
  });

  it("prefetches native card data before a card switch and keeps it cached", async () => {
    const getControls = vi.spyOn(api, "getControlsStatus");
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await waitFor(() => getControls.mock.calls.length > 0, 2500);
    const before = getControls.mock.calls.length;
    expect(before).toBeGreaterThan(0);
    await act(async () => {
      (element.querySelector('.island-dot-button[aria-label="Go to Quick Settings card"]') as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 450));
    });
    // No refetch caused by the first switch (the probe runs once at bootstrap).
    expect(getControls.mock.calls.length).toBe(before);
    getControls.mockRestore();
  });

  it("sizes the native window once per card mode with no per-frame resize storm", async () => {
    const calls: Array<{ w: number; h: number }> = [];
    const geometry = vi.spyOn(api, "setIslandGeometry").mockImplementation(async (width, height) => {
      calls.push({ w: width, h: height });
    });
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await settle(120);
    const initial = calls.length;
    await act(async () => {
      (element.querySelector('.island-dot-button[aria-label="Go to Clipboard card"]') as HTMLButtonElement).click();
await new Promise((resolve) => setTimeout(resolve, 800));
    });
    // Grows once before the morph; the settle afterwards is the same size.
    expect(calls.length).toBe(initial + 1);
    expect(calls[initial]).toEqual({ w: 550, h: 391 });
    const afterSwitch = calls.length;
    await act(async () => {
      (element.querySelector('.island-dot-button[aria-label="Go to Search card"]') as HTMLButtonElement).click();
await new Promise((resolve) => setTimeout(resolve, 800));
    });
    // Stays reserved during the morph and shrinks once after it.
    expect(calls.length).toBe(afterSwitch + 1);
    expect(calls[calls.length - 1]).toEqual({ w: 550, h: 66 });
    // One width for every card: a switch can never move the window sideways.
    expect(calls.every(({ w }) => w === 550)).toBe(true);
    geometry.mockRestore();
  });
it("retargets an in-flight size morph to the latest card and settles its geometry", async () => {
const calls: Array<{ w: number; h: number }> = [];
const geometry = vi.spyOn(api, "setIslandGeometry").mockImplementation(async (width, height) => {
calls.push({ w: width, h: height });
});
const element = await renderIsland();
await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
const dots = element.querySelectorAll(".island-dot-button");
await act(async () => {
(dots[1] as HTMLButtonElement).click();
await new Promise((resolve) => setTimeout(resolve, 145));
(dots[2] as HTMLButtonElement).click();
await new Promise((resolve) => setTimeout(resolve, 1000));
});
expect(element.querySelector(".island-dot-button.is-active")?.getAttribute("aria-label")).toBe(
"Go to Clipboard card",
);
expect(calls.at(-1)).toEqual({ w: 550, h: 391 });
expect(calls.some(({ h }) => h < 391)).toBe(true);
geometry.mockRestore();
});
it("keeps reserved native bounds when the active card cancels a morph", async () => {
const calls: Array<{ w: number; h: number }> = [];
const geometry = vi.spyOn(api, "setIslandGeometry").mockImplementation(async (width, height) => {
calls.push({ w: width, h: height });
});
const element = await renderIsland();
await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
await act(async () => {
(element.querySelector('.island-dot-button[aria-label="Go to Media card"]') as HTMLButtonElement).click();
await new Promise((resolve) => setTimeout(resolve, 50));
(element.querySelector('.island-dot-button[aria-label="Go to Search card"]') as HTMLButtonElement).click();
await new Promise((resolve) => setTimeout(resolve, 350));
});
expect(calls.at(-1)).toEqual({ w: 550, h: 286 });
await act(async () => {
await new Promise((resolve) => setTimeout(resolve, 250));
});
expect(calls.at(-1)).toEqual({ w: 550, h: 66 });
geometry.mockRestore();
});

  it("applies the latest card selected during the switch throttle", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    const dots = element.querySelectorAll(".island-dot-button");
    await act(async () => {
      (dots[1] as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 40));
      (dots[2] as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 500));
    });
    expect(element.querySelector(".island-dot-button.is-active")?.getAttribute("aria-label")).toBe(
      "Go to Clipboard card",
    );
  });

  it("cancels a delayed card change on Escape", async () => {
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await act(async () => {
      (element.querySelector('.island-dot-button[aria-label="Go to Media card"]') as HTMLButtonElement).click();
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
      await new Promise((resolve) => setTimeout(resolve, 300));
    });
    expect(element.querySelector(".island-dot-button.is-active")?.getAttribute("aria-label")).toBe(
      "Go to Search card",
    );
  });

  it("cancels queued card changes when the active Search dot is selected", async () => {
    const geometry = vi.spyOn(api, "setIslandGeometry").mockResolvedValue(undefined);
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    const dots = element.querySelectorAll(".island-dot-button");
    await act(async () => {
      (dots[1] as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 30));
      (dots[0] as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 700));
    });
    expect(element.querySelector(".island-dot-button.is-active")?.getAttribute("aria-label")).toBe(
      "Go to Search card",
    );
    expect(element.querySelector(".island-surface--compact")).not.toBeNull();
    expect(geometry.mock.calls.at(-1)).toEqual([550, 66]);
    expect(element.querySelector(".island-surface--out")).toBeNull();
    geometry.mockRestore();
  });

  it("returns from an expanded card with Escape and restores visible compact geometry", async () => {
    const geometry = vi.spyOn(api, "setIslandGeometry").mockResolvedValue(undefined);
    const element = await renderIsland();
    await waitFor(() => element.querySelectorAll(".island-dot-button").length === 8);
    await act(async () => {
      (element.querySelector('.island-dot-button[aria-label="Go to Media card"]') as HTMLButtonElement).click();
      await new Promise((resolve) => setTimeout(resolve, 450));
    });
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
await new Promise((resolve) => setTimeout(resolve, 700));
    });
    expect(element.querySelector(".island-surface--compact")).not.toBeNull();
    expect(element.querySelector(".island-surface--in, .island-surface--out")).toBeNull();
    expect(geometry.mock.calls.at(-1)).toEqual([550, 66]);
    geometry.mockRestore();
  });
});
