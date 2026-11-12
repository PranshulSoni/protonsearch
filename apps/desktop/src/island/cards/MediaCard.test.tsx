import { act, type ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../../bridge/api";
import type { ControlCapability, ControlsStatus, MediaSessionInfo } from "../../bridge/types";
import { MediaCard } from "./MediaCard";

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
  album: "Glass Roads",
  artworkDataUrl: null,
  playing: true,
  positionMs: 61_000,
  durationMs: 180_000,
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
    audioOutputs: [
      { id: "spk", name: "Speakers", isDefault: true },
      { id: "hp", name: "Headphones", isDefault: false },
    ],
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

describe("MediaCard", () => {
  it("play, pause and skip call the media bridge and refresh", async () => {
    const mediaCommand = vi.spyOn(api, "mediaCommand").mockResolvedValue(undefined);
    const refresh = vi.fn();
    await render(<MediaCard status={status()} pending={false} error={null} refresh={refresh} />);

    await press(find<HTMLButtonElement>('button[aria-label="Pause"]')!);
    expect(mediaCommand).toHaveBeenLastCalledWith("pause");
    await press(find<HTMLButtonElement>('button[aria-label="Next track"]')!);
    expect(mediaCommand).toHaveBeenLastCalledWith("next");
    expect(refresh).toHaveBeenCalledTimes(2);

    await render(<MediaCard status={status({ media: { ...session, playing: false } })} pending={false} error={null} refresh={refresh} />);
    await press(find<HTMLButtonElement>('button[aria-label="Play"]')!);
    expect(mediaCommand).toHaveBeenLastCalledWith("play");
  });

  it("flips the play/pause icon before the bridge answers, then follows the refreshed status", async () => {
    let answer!: () => void;
    vi.spyOn(api, "mediaCommand").mockReturnValue(new Promise<void>((resolve) => (answer = resolve)));
    const refresh = vi.fn();
    await render(<MediaCard status={status()} pending={false} error={null} refresh={refresh} />);

    await press(find<HTMLButtonElement>('button[aria-label="Pause"]')!);
    expect(find('button[aria-label="Play"]')).not.toBeNull();

    await act(async () => answer());
    expect(refresh).toHaveBeenCalledTimes(1);
    await waitUntil(() => find('button[aria-label="Pause"]') !== null);
  });

  it("seeks to the released position and leaves the bar disabled when the player cannot seek", async () => {
    const mediaCommand = vi.spyOn(api, "mediaCommand").mockResolvedValue(undefined);
    const refresh = vi.fn();
    await render(
      <MediaCard status={status({ media: { ...session, canSeek: true } })} pending={false} error={null} refresh={refresh} />,
    );
    const slider = find<HTMLInputElement>('input[aria-label="Position"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(slider, "120");
      slider.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(mediaCommand).not.toHaveBeenCalled();
    expect(container?.textContent).toContain("2:00");
    await act(async () => {
      slider.dispatchEvent(new Event("pointerup", { bubbles: true }));
    });
    expect(mediaCommand).toHaveBeenCalledWith("seek", 120_000);
    expect(refresh).toHaveBeenCalledTimes(1);

    await render(<MediaCard status={status()} pending={false} error={null} refresh={refresh} />);
    expect(find<HTMLInputElement>('input[aria-label="Position"]')!.disabled).toBe(true);
  });

  it("shows one honest line when no media session is active", async () => {
    await render(<MediaCard status={status({ media: null })} pending={false} error={null} refresh={vi.fn()} />);
    expect(container?.textContent).toContain("Nothing playing");
    expect(find('button[aria-label="Pause"]')).toBeNull();
    expect(find('button[aria-label="Play"]')).toBeNull();
  });

  it("switches the output device from the drill-down and returns to the player", async () => {
    const setAudioOutput = vi.spyOn(api, "setAudioOutput").mockResolvedValue(undefined);
    await render(<MediaCard status={status()} pending={false} error={null} refresh={vi.fn()} />);

    await press(buttonWithText("Speakers")!);
    await waitUntil(() => find('[role="radio"]') !== null);
    const headphones = Array.from(container!.querySelectorAll<HTMLButtonElement>('[role="radio"]')).find((button) =>
      button.textContent?.includes("Headphones"),
    )!;
    await press(headphones);

    expect(setAudioOutput).toHaveBeenCalledWith("hp");
    await waitUntil(() => find('button[aria-label="Pause"]') !== null);
  });

  it("explains an unavailable media adapter with the engine reason", async () => {
    await render(
      <MediaCard
        status={status({ mediaCapability: capability(false, "No media session adapter in this build.") })}
        pending={false}
        error={null}
        refresh={vi.fn()}
      />,
    );
    expect(container?.textContent).toContain("No media session adapter in this build.");
  });
});
