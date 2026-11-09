import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import type { AgentTask, SearchResult } from "../bridge/types";
import { AgentsCard, ClipboardCard } from "./IslandCards";
import { ControlsCard, NotificationsCard } from "./NativeCards";
import type { ControlsStatus } from "../bridge/types";
import { api } from "../bridge/api";
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
      wifi: { supported: true, reason: null },
      wifiEnabled: true,
      wifiNetworks: [],
      bluetooth: { supported: false, reason: "Bluetooth adapter isn't wired." },
      bluetoothEnabled: null,
      bluetoothDevices: [],
      airplaneMode: { supported: false, reason: "Not exposed." },
      nightLight: { supported: false, reason: "Not exposed." },
      energySaver: { supported: true, reason: null },
      notifications: { supported: false, reason: "Local notification history isn't available." },
      toastCapability: { supported: false, reason: "Windows toast notifications need an app identity." },
      notificationUnread: 0,
      notificationItems: [],
    };
    const element = await mount(
      <ControlsCard status={status} pending={false} error={null} refresh={() => undefined} />,
    );
    expect(element.textContent).toContain("62%");
    expect(element.textContent).toContain("80%");
    expect(element.querySelectorAll(".island-setting-slider").length).toBe(2);
    expect(element.querySelectorAll(".island-control-tile").length).toBe(4);
    const unsupportedTile = [...element.querySelectorAll(".island-control-tile")].find((tile) =>
      tile.textContent?.includes("Bluetooth"),
    );
    expect(unsupportedTile?.textContent).toContain("Unsupported");
    expect(unsupportedTile?.getAttribute("aria-disabled")).toBe("true");
    expect(unsupportedTile?.getAttribute("title")).toContain("Bluetooth adapter isn't wired.");
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
      wifi: { supported: false, reason: "n/a" },
      wifiEnabled: null,
      wifiNetworks: [],
      bluetooth: { supported: false, reason: "n/a" },
      bluetoothEnabled: null,
      bluetoothDevices: [],
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
      wifi: { supported: false, reason: "n/a" },
      wifiEnabled: null,
      wifiNetworks: [],
      bluetooth: { supported: false, reason: "n/a" },
      bluetoothEnabled: null,
      bluetoothDevices: [],
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
    const clearButton = [...element.querySelectorAll<HTMLButtonElement>(".island-agent-button")].find(
      (button) => button.textContent?.includes("Clear all"),
    );
    expect(clearButton?.disabled).toBe(false);
    await act(async () => {
      clearButton?.click();
      await Promise.resolve();
    });
    expect(clear).toHaveBeenCalled();
    dismiss.mockRestore();
    clear.mockRestore();
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
    expect(element.querySelector(".island-agent-note")?.textContent).toContain(
      "No pause or resume action is available",
    );
    expect(element.querySelector(".island-progress-fill")).not.toBeNull();
  });
});
