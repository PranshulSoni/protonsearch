import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLDivElement;
let root: Root;

function stubMatchMedia(query: string) {
  return {
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(() => false),
  };
}

beforeEach(() => {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn((query: string) => stubMatchMedia(query)),
  });
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => {
    root.unmount();
  });
  container.remove();
});

async function flush(times = 6) {
  for (let index = 0; index < times; index += 1) {
    await act(async () => {
      await Promise.resolve();
    });
  }
}

describe("desktop app shell smoke", () => {
  it("renders all five routes from real mock-bridge data without crashing", async () => {
    await act(async () => {
      root.render(<App view="main" />);
    });
    await flush(8);

    expect(container.querySelector(".app-shell")).not.toBeNull();
    const navButtons = [...container.querySelectorAll<HTMLButtonElement>(".navbtn")];
    expect(navButtons.map((button) => button.textContent?.trim())).toEqual([
      "Home",
      "Search & Indexing",
      "Agentic workflows",
      "To-do",
      "Settings",
    ]);
    expect(container.querySelector("#main h1")?.textContent).toMatch(
      /^Good (morning|afternoon|evening)/,
    );

    const routes: Array<{ navIndex: number; heading: string }> = [
      { navIndex: 1, heading: "Search & Indexing" },
      { navIndex: 2, heading: "Agentic workflows" },
      { navIndex: 3, heading: "To-do" },
      { navIndex: 4, heading: "Settings" },
    ];
    for (const route of routes) {
      await act(async () => {
        navButtons[route.navIndex].click();
      });
      await flush(3);
      expect(container.querySelector("#main h1")?.textContent).toBe(route.heading);
    }
  });

  it("plays the window-open animation once and not on route changes", async () => {
    await act(async () => {
      root.render(<App view="main" />);
    });
    await flush(8);
    const shell = container.querySelector<HTMLElement>(".app-shell");
    expect(shell?.classList.contains("is-opening")).toBe(true);

    // animationend normally clears the class; wait past the 240 ms fallback
    // timer so this also covers environments where the event is not fired.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 280));
    });
    await flush(2);
    expect(container.querySelector(".app-shell")?.classList.contains("is-opening")).toBe(false);

    await act(async () => {
      container.querySelectorAll<HTMLButtonElement>(".navbtn")[2].click();
    });
    await flush(3);
    expect(container.querySelector(".app-shell")?.classList.contains("is-opening")).toBe(false);
  });

  it("renders the engine filename count rather than a transient zero", async () => {
    await act(async () => {
      root.render(<App view="main" />);
    });
    await flush(8);
    const indexStat = container.querySelector(".index-stat");
    expect(indexStat?.textContent).not.toBe("0");
    expect(indexStat?.textContent).toMatch(/\d/);
  });

  it("exposes web-search engine settings in Preferences", async () => {
    await act(async () => {
      root.render(<App view="main" />);
    });
    await flush(8);
    await act(async () => {
      container.querySelectorAll<HTMLButtonElement>(".navbtn")[4].click();
    });
    await flush(4);
    const text = container.textContent ?? "";
    expect(text).toContain("Offer web search results");
    expect(text).toContain("DuckDuckGo");
    expect(text).toContain("Used by the web search result.");
    const radios = [...container.querySelectorAll('[role="radio"]')].map((node) =>
      node.textContent?.trim(),
    );
    expect(radios).toEqual(expect.arrayContaining(["Google", "Bing", "DuckDuckGo", "YouTube"]));
  });

  it("shows the working OCR engine and marks unavailable alternatives", async () => {
    await act(async () => {
      root.render(<App view="main" />);
    });
    await flush(8);
    await act(async () => container.querySelectorAll<HTMLButtonElement>(".navbtn")[1].click());
    await flush(4);
    const selector = container.querySelector<HTMLSelectElement>("#ocr-engine");
    expect(selector?.value).toBe("windows");
    expect([...((selector?.options ?? []) as HTMLOptionsCollection)].map((option) => [option.value, option.disabled])).toEqual([
      ["windows", false],
      ["pp_ocr_v6", true],
      ["indra", true],
      ["tesseract", true],
    ]);
    // Each engine that is not installed offers an Install button.
    expect([...container.querySelectorAll("button")].filter((button) => button.textContent === "Install")).toHaveLength(3);
    expect(container.textContent).not.toContain("acceptance");
  });

  it("shows Pi and its providers, and no other agent", async () => {
    window.localStorage.clear();
    await act(async () => {
      root.render(<App view="main" />);
    });
    await flush(8);
    await act(async () => {
      container.querySelectorAll<HTMLButtonElement>(".navbtn")[2].click();
    });
    await flush(8);
    const text = container.textContent ?? "";
    expect(text).toContain("Pi agent");
    expect(text).toContain("OpenAI-compatible endpoint");
    expect(text).toContain("Key set in the environment");
    expect(text).not.toContain("Claude Code");
    expect(text).not.toContain("Gemini CLI");
    expect(container.querySelector("#workflows-agents")).not.toBeNull();
    expect(container.querySelector("#workflows-providers")).not.toBeNull();
  });

  it("keeps the first focusable element a skip link and the route h1 focusable", async () => {
    await act(async () => {
      root.render(<App view="main" />);
    });
    await flush(8);
    const first = container.querySelector<HTMLElement>(".skip-link");
    expect(first).not.toBeNull();
    expect(first?.textContent).toContain("Skip to main content");
    const heading = container.querySelector<HTMLHeadingElement>("#main h1");
    expect(heading?.getAttribute("tabindex")).toBe("-1");
  });
});
