import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

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
  // Fresh bridge mock per test: its settings live in module state.
  vi.resetModules();
  window.history.replaceState({}, "", "/?view=main&onboard=1");
  window.localStorage.clear();
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

async function flush(times = 8) {
  for (let index = 0; index < times; index += 1) {
    await act(async () => {
      await Promise.resolve();
    });
  }
}

async function renderApp() {
  const { default: App } = await import("../App");
  await act(async () => root.render(<App view="main" />));
  await flush(10);
}

const title = () => container.querySelector("h1")?.textContent;

function primary(): HTMLButtonElement {
  const button = container.querySelector<HTMLButtonElement>(".onboard-nav .btn-primary");
  if (!button) throw new Error("primary button missing");
  return button;
}

function button(text: string): HTMLButtonElement | undefined {
  return [...container.querySelectorAll<HTMLButtonElement>("button")].find(
    (element) => element.textContent?.trim() === text,
  );
}

async function click(element: HTMLElement | null | undefined) {
  if (!element) throw new Error("element missing");
  await act(async () => element.click());
  await flush(6);
}

async function press(init: KeyboardEventInit) {
  const target = document.activeElement ?? document.body;
  await act(async () => {
    target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));
  });
  await flush(6);
}

async function type(input: HTMLInputElement, value: string) {
  const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  await act(async () => {
    setValue?.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

const caps = (scope: string) =>
  [...container.querySelectorAll(scope + " .kbd")].map((element) => element.textContent);

describe("onboarding smoke", () => {
  it("walks the six steps, saving each choice, and opens the launcher", async () => {
    await renderApp();
    expect(container.querySelector('.onboard-progress [aria-current="step"]')?.textContent).toBe("Welcome");
    expect(title()).toBe("Welcome to ProtonSearch");
    expect(document.activeElement).toBe(primary());

    // Name: focused on arrival, Back returns to the welcome step.
    await click(primary());
    expect(title()).toBe("What should we call you?");
    await click(button("Back"));
    expect(title()).toBe("Welcome to ProtonSearch");
    await click(primary());
    const name = container.querySelector<HTMLInputElement>(".onboard-input");
    expect(document.activeElement).toBe(name);
    await type(name!, "Alex");
    await click(primary());

    // OCR: the shared engine selector, or no OCR at all.
    expect(title()).toBe("Read text in images?");
    expect(container.querySelector<HTMLSelectElement>("#ocr-engine")?.value).toBe("windows");
    await click(button("No OCR"));
    expect(container.querySelector("[data-testid='ocr-engine-options']")).toBeNull();
    await click(button("Use OCR"));
    expect(container.querySelector("[data-testid='ocr-engine-options']")).not.toBeNull();
    await click(primary());

    // Appearance: the preview pill follows the choices before they are saved.
    expect(title()).toBe("Choose a look");
    const pill = () => container.querySelector<HTMLElement>(".onboard-pill");
    expect(pill()?.classList.contains("is-glass")).toBe(false);
    await click(container.querySelector<HTMLElement>('[role="switch"][aria-label="Liquid glass"]'));
    await click(button("Dark"));
    expect(pill()?.classList.contains("is-glass")).toBe(true);
    expect(pill()?.style.getPropertyValue("--glass")).toBe("0.6");
    expect(container.querySelector(".onboard-preview")?.getAttribute("data-look")).toBe("dark");
    await click(primary());
    expect(JSON.parse(window.localStorage.getItem("protonsearch.islandLook") ?? "{}")).toMatchObject({
      liquidGlass: true,
      theme: "dark",
    });

    // Shortcut: Esc cancels recording; Alt+Shift+Space is recorded by physical key.
    expect(title()).toBe("Set your shortcut");
    expect(container.textContent).not.toMatch(/at least one/i);
    expect(document.activeElement).toBe(button("Change"));
    expect(caps(".onboard-shortcut")).toEqual(["Alt", "Space"]);
    await click(button("Change"));
    expect(button("Listening…")).toBeDefined();
    await press({ key: "Escape", code: "Escape" });
    expect(button("Change")).toBeDefined();
    expect(caps(".onboard-shortcut")).toEqual(["Alt", "Space"]);
    await click(button("Change"));
    await press({ key: " ", code: "Space", altKey: true, shiftKey: true });
    expect(caps(".onboard-shortcut")).toEqual(["Alt", "Shift", "Space"]);
    await click(primary());

    // Done: the saved combo as caps, then the launcher.
    expect(title()).toBe("You're all set");
    expect(caps(".onboard-lead")).toEqual(["Alt", "Shift", "Space"]);
    expect(primary().textContent).toBe("Open launcher");
    await click(primary());
    await flush(12);
    expect(container.querySelector(".onboard")).toBeNull();
    expect(container.querySelector(".app-shell")).not.toBeNull();
    // Home greets with the saved name instead of the Windows user name.
    expect(container.querySelector(".page-head")?.textContent).toContain("Alex");
  }, 15_000);

  it("skip setup hands back to the app shell", async () => {
    await renderApp();
    await click(button("Skip setup"));
    await flush(12);
    expect(container.querySelector(".app-shell")).not.toBeNull();
    expect(container.querySelector(".onboard")).toBeNull();
  });
});
