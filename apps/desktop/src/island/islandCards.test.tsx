import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import type { AgentTask, SearchResult } from "../bridge/types";
import { AgentsCard, ClipboardCard } from "./IslandCards";
import { ControlsCard, NotificationsCard } from "./NativeCards";
import type { ControlsStatus } from "../bridge/types";
import { api } from "../bridge/api";
import { useIslandControls } from "./islandNativeCards";
import { vi } from "vitest";

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

const clip = (id: string, label: string, title: string, thumb?: string): SearchResult => ({
  id,
  kind: "clipboard",
  title,
  subtitle: "Copied",
  icon: { type: "monogram" },
  score: 1,
  meta: { label, modified: Date.now() - 60_000, thumbnailDataUrl: thumb ?? null },
  action: { type: "copy_text", text: title },
});

describe("island cards real data", () => {
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
  it("filters clipboard tabs and renders real image thumbnails", async () => {
    const thumb =
      "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2n0kAAAAASUVORK5CYII=";
    const element = await mount(
      <ClipboardCard
        results={[
          clip("text", "TEXT", "remember the budget"),
          clip("image", "IMAGE", "diagram.png", thumb),
        ]}
        pending={false}
        error={null}
        tab="all"
        onTab={() => undefined}
        onOpen={() => undefined}
      />,
    );
    expect(element.querySelectorAll(".island-clip-row").length).toBe(2);

    // Drive the Images tab through the same filter used by the card.
    const imageOnly = [
      clip("text", "TEXT", "remember the budget"),
      clip("image", "IMAGE", "diagram.png", thumb),
    ].filter((row) => row.meta?.label === "IMAGE");
    const reRendered = await mount(
      <ClipboardCard
        results={imageOnly}
        pending={false}
        error={null}
        tab="images"
        onTab={() => undefined}
        onOpen={() => undefined}
      />,
    );
    expect(reRendered.querySelectorAll(".island-clip-row").length).toBe(1);
    expect(reRendered.querySelector(".island-clip-thumb")?.getAttribute("src")).toBe(thumb);
  });

  it("renders real volume/brightness/output controls and capability-honest toggles", async () => {
const setMuted = vi.spyOn(api, "setMuted").mockResolvedValue();
let finishRefresh!: () => void;
let failRefresh = false;
const refresh = () => failRefresh
? Promise.reject(new Error("Controls probe failed"))
: new Promise<void>((resolve) => { finishRefresh = resolve; });
    const status: ControlsStatus = {
      mediaCapability: { supported: false, reason: "Media session adapter isn't wired." },
      media: null,
      audioOutputs: [
        { id: "default", name: "Laptop speakers", isDefault: true },
        { id: "usb", name: "External display", isDefault: false },
      ],
      defaultAudioId: "default",
      volumePercent: 62,
      muted: false,
      brightnessCapability: { supported: true, reason: null },
      brightnessPercent: 80,
      brightnessMonitors: [
        { index: 0, name: "Built-in display", current: 80, minimum: 0, maximum: 100, method: "dxva2" },
      ],
      wifi: { supported: true, reason: null },
      wifiEnabled: true,
      wifiNetworks: [],
      wifiError: null,
      bluetooth: { supported: false, reason: "Bluetooth adapter isn't wired." },
      bluetoothEnabled: null,
      bluetoothDevices: [],
      bluetoothError: null,
      airplaneMode: { supported: false, reason: "Not exposed." },
      nightLight: { supported: false, reason: "Not exposed." },
      energySaver: { supported: true, reason: null },
      notifications: { supported: false, reason: "Local notification history isn't available." },
      toastCapability: { supported: false, reason: "Windows toast notifications need an app identity." },
      notificationUnread: 0,
      notificationItems: [],
    };
    const element = await mount(
<ControlsCard status={status} pending={false} error={null} refresh={refresh} />,
    );
    expect(element.textContent).toContain("62%");
    expect(element.textContent).toContain("80%");
    expect(element.querySelectorAll(".island-setting-slider").length).toBe(2);
    expect(element.querySelectorAll(".island-control-tile").length).toBe(2);
    const unsupportedTile = [...element.querySelectorAll(".island-control-tile")].find((tile) =>
      tile.textContent?.includes("Bluetooth"),
    );
    expect(unsupportedTile?.textContent).toContain("Can't change");
    const bluetoothToggle = unsupportedTile?.querySelector<HTMLButtonElement>(".island-tile-main");
    expect(bluetoothToggle?.disabled).toBe(true);
    expect(bluetoothToggle?.getAttribute("title")).toContain("Bluetooth adapter isn't wired.");
const muteButton = element.querySelector<HTMLButtonElement>(".island-mute-button")!;
await act(async () => { muteButton.click(); await Promise.resolve(); });
expect(muteButton.disabled).toBe(true);
await act(async () => { finishRefresh(); await Promise.resolve(); });
expect(muteButton.disabled).toBe(false);
failRefresh = true;
await act(async () => { muteButton.click(); await new Promise((resolve) => setTimeout(resolve, 0)); });
expect(element.querySelector('[role="alert"]')?.textContent).toContain("Controls probe failed");
expect(element.textContent).not.toContain("Changed mute.");
setMuted.mockRestore();
  });

  it("wires real Wi-Fi/Bluetooth toggles and fallbacks", async () => {
    const setWifi = vi.spyOn(api, "setWifi").mockResolvedValue();
    const setBluetooth = vi.spyOn(api, "setBluetooth").mockResolvedValue();
    const status: ControlsStatus = {
      mediaCapability: { supported: false, reason: "n/a" },
      media: null,
      audioOutputs: [],
      defaultAudioId: null,
      volumePercent: 62,
      muted: false,
      brightnessCapability: { supported: true, reason: null },
      brightnessPercent: 80,
      brightnessMonitors: [],
      wifi: { supported: true, reason: null },
      wifiEnabled: true,
      wifiNetworks: [],
      wifiError: null,
      bluetooth: { supported: true, reason: null },
      bluetoothEnabled: false,
      bluetoothDevices: [],
      bluetoothError: null,
      airplaneMode: { supported: false, reason: "n/a" },
      nightLight: { supported: false, reason: "n/a" },
      energySaver: { supported: false, reason: "n/a" },
      notifications: { supported: true, reason: null },
      toastCapability: { supported: false, reason: "n/a" },
      notificationUnread: 0,
      notificationItems: [],
    };
    const element = await mount(
      <ControlsCard status={status} pending={false} error={null} refresh={() => undefined} />,
    );
    const tiles = element.querySelectorAll(".island-control-tile");
    const wifiToggle = tiles[0].querySelector<HTMLButtonElement>(".island-tile-main")!;
    const btToggle = tiles[1].querySelector<HTMLButtonElement>(".island-tile-main")!;
    expect(wifiToggle.disabled).toBe(false);
    expect(btToggle.disabled).toBe(false);
    expect(wifiToggle.getAttribute("aria-checked")).toBe("true");
    expect(btToggle.getAttribute("aria-checked")).toBe("false");
    await act(async () => {
      wifiToggle.click();
      await Promise.resolve();
    });
    await act(async () => {
      btToggle.click();
      await Promise.resolve();
    });
    expect(setWifi).toHaveBeenCalledWith(false);
    expect(setBluetooth).toHaveBeenCalledWith(true);
    // Silent success - no noisy status banner per user requirement
    expect(element.querySelector(".island-control-feedback")).toBeNull();
    setWifi.mockRestore();
    setBluetooth.mockRestore();
  });

  it("shows rejection reason on failure without noisy update text", async () => {
    let rejectSetWifi!: (reason: string) => void;
    const setWifi = vi.spyOn(api, "setWifi").mockImplementation(
      () => new Promise<void>((_resolve, reject) => { rejectSetWifi = reject; }),
    );
    const status: ControlsStatus = {
      mediaCapability: { supported: false, reason: "n/a" }, media: null, audioOutputs: [], defaultAudioId: null,
      volumePercent: null, muted: null, brightnessCapability: { supported: false, reason: "n/a" },
      brightnessPercent: null, brightnessMonitors: [], wifi: { supported: true, reason: null }, wifiEnabled: true,
      wifiNetworks: [], wifiError: null, bluetooth: { supported: false, reason: "n/a" }, bluetoothEnabled: null,
      bluetoothDevices: [], bluetoothError: null, airplaneMode: { supported: false, reason: "n/a" },
      nightLight: { supported: false, reason: "n/a" }, energySaver: { supported: false, reason: "n/a" },
      notifications: { supported: false, reason: "n/a" }, toastCapability: { supported: false, reason: "n/a" },
      notificationUnread: 0, notificationItems: [],
    };
    const element = await mount(<ControlsCard status={status} pending={false} error={null} refresh={() => undefined} />);
    const wifiToggle = element.querySelector<HTMLButtonElement>(".island-tile-main")!;
    await act(async () => { wifiToggle.click(); });
    await act(async () => { rejectSetWifi("Access denied by system"); await Promise.resolve(); });
    expect(element.textContent).toContain("Access denied by system");
    expect(element.querySelector('[role="alert"]')).not.toBeNull();
    setWifi.mockRestore();
  });

  it("shows the notification capability reason and a local toast test", async () => {
    const status: ControlsStatus = {
      mediaCapability: { supported: false, reason: "n/a" },
      media: null,
      audioOutputs: [],
      defaultAudioId: null,
      volumePercent: null,
      muted: null,
      brightnessCapability: { supported: false, reason: "No brightness path." },
      brightnessPercent: null,
      brightnessMonitors: [],
      wifi: { supported: false, reason: "n/a" },
      wifiEnabled: null,
      wifiNetworks: [],
      wifiError: null,
      bluetooth: { supported: false, reason: "n/a" },
      bluetoothEnabled: null,
      bluetoothDevices: [],
      bluetoothError: null,
      airplaneMode: { supported: false, reason: "n/a" },
      nightLight: { supported: false, reason: "n/a" },
      energySaver: { supported: false, reason: "n/a" },
      notifications: { supported: false, reason: "Local notification history isn't available yet." },
      toastCapability: { supported: false, reason: "Windows toast notifications need an app identity." },
      notificationUnread: 0,
      notificationItems: [],
    };
    const element = await mount(
      <NotificationsCard status={status} pending={false} error={null} refresh={() => undefined} />,
    );
    expect(element.textContent).toContain("Local notification history isn't available yet.");
    // Test-only controls are gated out of normal states (toastCapability false here).
    expect(element.textContent).not.toContain("Test local toast");
  });

  it("dismisses one notification and clears all through the bridge", async () => {
    const dismiss = vi.spyOn(api, "notificationsDismiss").mockResolvedValue();
    const clear = vi.spyOn(api, "notificationsClear").mockResolvedValue();
    const status: ControlsStatus = {
      mediaCapability: { supported: false, reason: "n/a" },
      media: null,
      audioOutputs: [],
      defaultAudioId: null,
      volumePercent: 62,
      muted: false,
      brightnessCapability: { supported: true, reason: null },
      brightnessPercent: 80,
      brightnessMonitors: [
        { index: 0, name: "Built-in display", current: 80, minimum: 0, maximum: 100, method: "dxva2" },
      ],
      wifi: { supported: false, reason: "n/a" },
      wifiEnabled: null,
      wifiNetworks: [],
      wifiError: null,
      bluetooth: { supported: false, reason: "n/a" },
      bluetoothEnabled: null,
      bluetoothDevices: [],
      bluetoothError: null,
      airplaneMode: { supported: false, reason: "n/a" },
      nightLight: { supported: false, reason: "n/a" },
      energySaver: { supported: false, reason: "n/a" },
      notifications: { supported: true, reason: null },
      toastCapability: { supported: false, reason: "App identity needed." },
      notificationUnread: 1,
      notificationItems: [
        {
          id: "n1",
          title: "ProtonSearch is ready",
          body: "Your index is warm.",
          source: "ProtonSearch",
          createdAt: Date.now(),
          read: false,
        },
      ],
    };
    const element = await mount(
      <NotificationsCard status={status} pending={false} error={null} refresh={() => undefined} />,
    );
    const dismissButton = element.querySelector<HTMLButtonElement>(".island-dismiss-button");
    expect(dismissButton?.disabled).toBe(false);
    await act(async () => {
      dismissButton?.click();
      await Promise.resolve();
    });
    expect(dismiss).toHaveBeenCalledWith("n1");
    const clearButton = element.querySelector<HTMLButtonElement>(".island-clear-button");
    expect(clearButton?.disabled).toBe(false);
    await act(async () => {
      clearButton?.click();
      await Promise.resolve();
    });
    expect(clear).toHaveBeenCalled();
    dismiss.mockRestore();
    clear.mockRestore();
  });

  it("shows an empty agent state without task-specific controls", async () => {
    const element = await mount(<AgentsCard status={{ state: "running", runtime: "pi", model: "pi-1.5" }} tasks={[]} onPause={() => undefined} onResume={() => undefined} />);
    expect(element.textContent).toContain("No agent tasks yet.");
    expect(element.querySelector(".island-agent-note")).toBeNull();
  });

  it("renders flat real agent tasks and disables controls the task cannot use", async () => {
    const task: AgentTask = {
      id: "task:1",
      runtime: "pi",
      runtimeVersion: "1.5.0",
      parentId: null,
      title: "Index the docs workspace",
      promptSummary: "Index docs",
      state: "running",
      progress: { kind: "determinate", fraction: 0.42 },
      createdAt: Date.now() - 1000,
      startedAt: Date.now() - 900,
      endedAt: null,
      updatedAt: Date.now(),
      log: [{ at: Date.now(), level: "info", message: "Indexed 42 files" }],
      error: null,
      permissions: { requested: [], granted: ["fs.read"], denied: [], pending: null, readOnly: true },
      writeAccess: false,
      controls: { canCancel: true, canPause: false, canResume: false, canRetry: false },
      restarts: [],
      model: "pi-1.5",
    };
    const element = await mount(
      <AgentsCard
        status={{ state: "running", runtime: "pi", model: "pi-1.5" }}
        tasks={[task]}
        onPause={() => undefined}
        onResume={() => undefined}
      />,
    );
    expect(element.textContent).toContain("Index the docs workspace");
    expect(element.textContent).toContain("CURRENT WORKSPACE · 1 TASK");
    expect(element.textContent).toContain("1 permission granted");
    expect(element.querySelector(".island-agent-row")?.textContent).toContain("RUNNING");
    // Pi is flat: no fabricated child rows; controls follow task.controls.
    expect(element.querySelectorAll(".island-agent-row").length).toBe(1);
    // Neither control is available for this task: no ambiguous disabled pair.
    expect(element.querySelectorAll(".island-agent-button").length).toBe(0);
    expect(element.querySelector(".island-agent-note")).toBeNull();
    expect(element.querySelector(".island-progress-fill")).not.toBeNull();
  });
});
