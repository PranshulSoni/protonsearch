import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ControlsStatus } from "../bridge/types";
import { api } from "../bridge/api";
import { useIslandControls } from "./islandNativeCards";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | null = null;
let container: HTMLDivElement | null = null;

async function mount(element: React.ReactNode): Promise<HTMLDivElement> {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root!.render(element);
    await new Promise((resolve) => setTimeout(resolve, 20));
  });
  return container;
}

afterEach(async () => {
  if (root) {
    await act(async () => {
      root?.unmount();
    });
  }
  container?.remove();
  root = null;
  container = null;
});

describe("island controls data", () => {
  it("refreshes controls on focus and coalesces overlapping probes", async () => {
    const resolvers: Array<(status: ControlsStatus) => void> = [];
    const getStatus = vi.spyOn(api, "getControlsStatus").mockImplementation(
      () => new Promise((done) => { resolvers.push(done); }),
    );
    function Probe() {
      const controls = useIslandControls(true);
      return <><span>{controls.pending ? "pending" : "ready"}</span><button onClick={controls.refresh}>Refresh</button></>;
    }
    const element = await mount(<Probe />);
    expect(getStatus).toHaveBeenCalledTimes(1);
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
      window.dispatchEvent(new Event("focus"));
      element.querySelector("button")!.click();
    });
    expect(getStatus).toHaveBeenCalledTimes(1);
    await act(async () => {
      resolvers[0]({} as ControlsStatus);
      await Promise.resolve();
    });
    expect(getStatus).toHaveBeenCalledTimes(2);
    await act(async () => {
      resolvers[1]({} as ControlsStatus);
      await new Promise((resolve) => setTimeout(resolve, 0));
      window.dispatchEvent(new Event("focus"));
    });
    expect(getStatus).toHaveBeenCalledTimes(3);
    getStatus.mockRestore();
  });
});
