import { act, type ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../bridge/api";
import type { ControlCapability, ControlsStatus, MediaSessionInfo } from "../../bridge/types";
import { ControlsCard } from "./ControlsCard";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let root: Root | null = null;
let container: HTMLDivElement | null = null;

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", class {
    observe() {}
    unobserve() {}
    disconnect() {}
  });
});

afterEach(async () => {
  await act(async () => {
    root?.unmount();
  });
  container?.remove();
  root = null;
  container = null;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function render(element: ReactElement): Promise<void> {
  if (!container) {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
  }
  await act(async () => {
    root!.render(element);
  });
}

function find<T extends Element = HTMLElement>(selector: string): T | null {
  return container?.querySelector<T>(selector) ?? null;
}

function buttonWithText(text: string): HTMLButtonElement | undefined {
  return Array.from(container?.querySelectorAll("button") ?? []).find((button) => button.textContent?.includes(text));
}

async function press(element: HTMLElement): Promise<void> {
  await act(async () => {
    element.click();
  });
}

/** Sets a range input the way a user drag does, so React sees the change. */
async function dragRange(input: HTMLInputElement, value: number): Promise<void> {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, String(value));
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function release(element: HTMLElement): Promise<void> {
  await act(async () => {
    element.dispatchEvent(new Event("pointerup", { bubbles: true }));
  });
}

async function waitUntil(check: () => boolean): Promise<void> {
  const started = Date.now();
  while (!check()) {
    if (Date.now() - started > 1500) throw new Error("condition not met in time");
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
  }
}

const capability = (supported = true, reason: string | null = null): ControlCapability => ({ supported, reason });

const session: MediaSessionInfo = {
  title: "Night Drive",
  artist: "Kavi",
  album: null,
  artworkDataUrl: null,
  playing: true,
  positionMs: 0,
  durationMs: null,
  canSeek: false,
  canShuffle: false,
  shuffle: false,
  canRepeat: false,
  repeat: false,
  appId: null,
};

function status(overrides: Partial<ControlsStatus> = {}): ControlsStatus {
  return {
    mediaCapability: capability(),
    media: session,
    audioOutputs: [{ id: "spk", name: "Speakers", isDefault: true }],
    defaultAudioId: "spk",
    volumePercent: 40,
    muted: false,
    brightnessCapability: capability(),
    brightnessPercent: 70,
    brightnessMonitors: [],
    wifi: capability(),
    wifiEnabled: true,
    wifiNetworks: [],
    wifiError: null,
    bluetooth: capability(),
    bluetoothEnabled: false,
    bluetoothDevices: [],
    bluetoothError: null,
    airplaneMode: capability(),
    nightLight: capability(),
    energySaver: capability(),
    notifications: capability(),
    toastCapability: capability(),
    notificationUnread: 0,
    notificationItems: [],
    ...overrides,
  };
}

describe("ControlsCard", () => {
  it("commits the volume on release, not while dragging", async () => {
    const setVolume = vi.spyOn(api, "setVolume").mockResolvedValue(undefined);
    await render(<ControlsCard status={status()} pending={false} error={null} refresh={vi.fn()} />);

    const slider = find<HTMLInputElement>('input[aria-label="Volume"]')!;
    await dragRange(slider, 55);
    expect(setVolume).not.toHaveBeenCalled();

    await release(slider);
    expect(setVolume).toHaveBeenCalledWith(55);
  });

  it("toggles Wi-Fi, drills into networks and returns with Back", async () => {
    const setWifi = vi.spyOn(api, "setWifi").mockResolvedValue(undefined);
    await render(
      <ControlsCard
        status={status({
          wifiNetworks: [{ ssid: "Home 5G", signal: 82, security: "WPA2-Personal", connected: true, hasProfile: true, connectable: true }],
        })}
        pending={false}
        error={null}
        refresh={vi.fn()}
      />,
    );

    await press(find<HTMLButtonElement>('[role="switch"][aria-label="Wi-Fi"]')!);
    expect(setWifi).toHaveBeenCalledWith(false);

    await press(find<HTMLButtonElement>('button[aria-label="Wi-Fi networks"]')!);
    // "Back" exists only in the drill-down, so this also waits out the main pane's exit.
    await waitUntil(() => find('button[aria-label="Back"]') !== null);
    expect(container?.textContent).toContain("Home 5G");
    expect(find('[role="switch"]')).toBeNull();

    await press(find<HTMLButtonElement>('button[aria-label="Back"]')!);
    await waitUntil(() => find('[role="switch"][aria-label="Wi-Fi"]') !== null);
  });

  it("hides brightness when the display can't change it", async () => {
    await render(
      <ControlsCard
        status={status({
          brightnessCapability: capability(false, "This display doesn't expose brightness control."),
          brightnessPercent: null,
        })}
        pending={false}
        error={null}
        refresh={vi.fn()}
      />,
    );

    expect(container?.textContent).not.toContain("Brightness can't be changed");
    expect(buttonWithText("Open display settings")).toBeFalsy();
  });

  it("keeps the Airplane, Night light and Energy saver links equally emphasised and enabled", async () => {
    const openSettings = vi.spyOn(api, "openSettings").mockResolvedValue(undefined);
    await render(
      <ControlsCard
        status={status({
          airplaneMode: capability(false, "Airplane mode has no stable API."),
          nightLight: capability(false, "Night light has no stable API."),
          energySaver: capability(true),
        })}
        pending={false}
        error={null}
        refresh={vi.fn()}
      />,
    );
    const tiles = ["Airplane", "Night light", "Energy saver"].map((label) => buttonWithText(label)!);
    expect(tiles.every((tile) => tile && !tile.disabled)).toBe(true);
    expect(new Set(tiles.map((tile) => tile.className)).size).toBe(1);
    expect(tiles[0]?.title).toBe("Airplane mode has no stable API.");
    expect(tiles[2]?.title).toBe("Open energy saver settings");
    await press(tiles[0]!);
    expect(openSettings).toHaveBeenCalledWith("airplane_mode");
  });

  it("links a location-consent failure to the privacy settings page", async () => {
    vi.spyOn(api, "setWifi").mockRejectedValue(new Error("Location access is off for desktop apps."));
    const openSettings = vi.spyOn(api, "openSettings").mockResolvedValue(undefined);
    await render(<ControlsCard status={status()} pending={false} error={null} refresh={vi.fn()} />);

    await press(find<HTMLButtonElement>('[role="switch"][aria-label="Wi-Fi"]')!);
    await waitUntil(() => find('[role="alert"]') !== null);

    const alert = find<HTMLElement>('[role="alert"]')!;
    expect(alert.textContent).toContain("Location access is off");
    await press(alert.querySelector("button")!);
    expect(openSettings).toHaveBeenCalledWith("privacy_location");
  });

  it("re-enables a control as soon as its action resolves, even while the status refresh is pending", async () => {
    vi.spyOn(api, "setWifi").mockResolvedValue(undefined);
    const refresh = vi.fn(() => new Promise<void>(() => undefined));
    await render(<ControlsCard status={status()} pending={false} error={null} refresh={refresh} />);

    await press(find<HTMLButtonElement>('[role="switch"][aria-label="Wi-Fi"]')!);
    await waitUntil(() => !find<HTMLButtonElement>('[role="switch"][aria-label="Wi-Fi"]')!.disabled);
    expect(refresh).toHaveBeenCalledTimes(1);
    // The engine still reports Wi-Fi on; the card keeps showing the value the action set.
    expect(find('[role="switch"][aria-label="Wi-Fi"]')!.getAttribute("aria-checked")).toBe("false");
  });
});
