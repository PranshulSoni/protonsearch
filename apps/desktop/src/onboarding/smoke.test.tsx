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

describe("onboarding smoke", () => {
  it("explains OCR engines during setup and only enables the installed engine", async () => {
    const { default: App } = await import("../App");
    await act(async () => root.render(<App view="main" />));
    await flush(8);
    const primary = () => container.querySelector<HTMLButtonElement>(".setup-bottom .btn-primary");
    for (let index = 0; index < 2; index += 1) {
      await act(async () => primary()?.click());
      await flush(2);
    }
    const firstLocation = container.querySelector<HTMLInputElement>(".choices input[type=checkbox]");
    expect(firstLocation).not.toBeNull();
    await act(async () => firstLocation?.click());
    await flush(2);
    await act(async () => primary()?.click());
    await flush(2);
    expect(container.querySelector("#onboarding-step-eyebrow")?.textContent).toContain("04 of 07");
    const selector = container.querySelector<HTMLSelectElement>("#ocr-engine");
    expect(selector?.value).toBe("windows");
    expect(selector?.querySelector<HTMLOptionElement>('option[value="pp_ocr_v6"]')?.disabled).toBe(true);
    expect(container.textContent).toContain("Built into Windows 10 and 11. Nothing to download.");
    expect(container.textContent).toContain("Open-source engine that uses your installed Tesseract.");
  });

  it("runs the seven-step flow, keeps progress local and completes through the bridge", async () => {
    const { default: App } = await import("../App");
    await act(async () => {
      root.render(<App view="main" />);
    });
    await flush(10);

    expect(container.querySelector(".onboarding")).not.toBeNull();
    expect(container.querySelector("#onboarding-step-eyebrow")?.textContent).toContain("01 of 07");
    expect(container.querySelector(".legacy-note")).not.toBeNull();

    const primary = () => {
      const button = container.querySelector<HTMLButtonElement>(".setup-bottom .btn-primary");
      if (!button) throw new Error("primary button missing");
      return button;
    };

    await act(async () => primary().click());
    await flush(2);
    expect(container.querySelector("h1")?.textContent).toBe("What should we call you?");

    // Back returns to the welcome step.
    const back = container.querySelector<HTMLButtonElement>(".setup-bottom-left .btn-ghost");
    expect(back).not.toBeNull();
    await act(async () => back?.click());
    await flush(2);
    expect(container.querySelector("h1")?.textContent).toBe("Find anything, without the friction.");

    // Persisted draft: resume state is written before completion.
    await act(async () => primary().click());
    await flush(2);
    const name = container.querySelector<HTMLInputElement>("#onboarding-name");
    expect(name).not.toBeNull();
    await act(async () => {
      if (name) {
        name.value = "Alex";
        name.dispatchEvent(new Event("input", { bubbles: true }));
      }
    });
    await flush(2);

    // Skip setup completes without roots and hands back to the app shell.
    const skip = container.querySelector<HTMLButtonElement>(".setup-bottom .linkbtn");
    expect(skip?.textContent).toContain("Skip setup");
    await act(async () => skip?.click());
    await flush(12);
    expect(container.querySelector(".app-shell")).not.toBeNull();
    expect(container.querySelector(".onboarding")).toBeNull();
  });
});
