import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import IslandView from "./IslandView";
import { api } from "../bridge/api";

// Native window events, captured so the test can play the shell's part.
const { handlers } = vi.hoisted(() => ({ handlers: new Map<string, () => void>() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: () => void) => {
    handlers.set(name, handler);
    return () => handlers.delete(name);
  }),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ isVisible: async () => false }),
}));

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | null = null;
let container: HTMLDivElement | null = null;

const wait = (ms: number) =>
  act(async () => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  });

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      addEventListener() {},
      removeEventListener() {},
    }),
  });
});

afterEach(async () => {
  await act(async () => root?.unmount());
  container?.remove();
  vi.unstubAllGlobals();
});

it("waits tucked while hidden, pops in on summon and tucks back on dismiss", async () => {
  const hide = vi.spyOn(api, "hideIsland").mockResolvedValue(undefined);
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => root!.render(<IslandView />));
  await wait(150);
  const state = () =>
    Array.from(container!.querySelector(".island-surface")!.classList)
      .filter((name) => /--(hidden|opening|closing)$/.test(name))
      .join(" ");

  // Prewarmed, never shown: nothing drawn, so the first summon cannot flash.
  expect(state()).toBe("island-surface--hidden");

  await act(async () => handlers.get("island:shown")!());
  expect(state()).toBe("island-surface--opening");
  await wait(470);
  expect(state()).toBe("");

  await act(async () => handlers.get("island:dismiss")!());
  expect(state()).toBe("island-surface--closing");
  expect(hide).not.toHaveBeenCalled();
  await wait(200);
  expect(state()).toBe("island-surface--hidden");
  expect(hide).toHaveBeenCalledTimes(1);

  // Re-summoned mid-dismiss: the pending hide is cancelled and it pops back in.
  await act(async () => handlers.get("island:shown")!());
  await act(async () => handlers.get("island:dismiss")!());
  await act(async () => handlers.get("island:shown")!());
  await wait(470);
  expect(state()).toBe("");
  expect(hide).toHaveBeenCalledTimes(1);

  // Hidden by the shell itself (safety net): parked again for the next summon.
  await act(async () => handlers.get("island:hidden")!());
  expect(state()).toBe("island-surface--hidden");
  hide.mockRestore();
});
