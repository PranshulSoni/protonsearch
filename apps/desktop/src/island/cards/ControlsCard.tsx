import { useEffect, useRef, useState } from "react";
import { AnimatePresence } from "motion/react";
import {
  Bluetooth,
  BluetoothOff,
  ChevronRight,
  Leaf,
  Moon,
  Plane,
  Speaker,
  Sun,
  Volume2,
  VolumeX,
  Wifi,
  WifiOff,
  type LucideIcon,
  SlidersHorizontal,
} from "lucide-react";
import { api } from "../../bridge/api";
import type { ControlCapability, ControlsStatus } from "../../bridge/types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { Slider } from "../ui/Slider";
import { AudioOutputList, CardFrame, DrillHeader, Pane, activeOutput, iconButton } from "./controlsShared";

type View = "main" | "wifi" | "bluetooth" | "output";
type ControlKey = "wifi" | "bluetooth" | "mute" | "volume" | "brightness" | "output";
/** Values shown ahead of the engine until the status refresh after the action lands. */
type Optimistic = { wifi?: boolean; bluetooth?: boolean; muted?: boolean; output?: string };

function without(current: Optimistic, shown: Optimistic): Optimistic {
  const next: Optimistic = { ...current };
  for (const name of Object.keys(shown) as (keyof Optimistic)[]) delete next[name];
  return next;
}

const openSettings = (page: string) => {
  void api.openSettings(page).catch(() => undefined);
};

function SettingsLink({ label, page }: { label: string; page: string }) {
  return (
    <button
      type="button"
      onClick={() => openSettings(page)}
      className="self-start rounded text-xs text-fg-2 underline-offset-4 hover:text-fg hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hi"
    >
      {label}
    </button>
  );
}

function ToggleTile({
  label,
  drillLabel,
  on,
  icon: Icon,
  detail,
  title,
  disabled,
  onToggle,
  onDrill,
}: {
  label: string;
  drillLabel: string;
  on: boolean;
  icon: LucideIcon;
  detail: string;
  title: string;
  disabled: boolean;
  onToggle: () => void;
  onDrill: () => void;
}) {
  return (
    <div className="flex min-w-0 items-center gap-1 rounded-tile bg-raised p-1.5">
      <button
        type="button"
        role="switch"
        aria-checked={on}
        aria-label={label}
        title={title}
        disabled={disabled}
        onClick={onToggle}
        className="flex min-w-0 flex-1 items-center gap-2 rounded-row p-1 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hi disabled:opacity-50"
      >
        <span
          className={cn(
            "grid size-8 shrink-0 place-items-center rounded-full",
            on ? "bg-hi text-surface" : "bg-active text-fg-2",
          )}
        >
          <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
        </span>
        <span className="flex min-w-0 flex-col">
          <span className="truncate text-[13px] font-medium leading-4">{label}</span>
          <span className="truncate text-xs leading-4 text-fg-3">{detail}</span>
        </span>
      </button>
      <button type="button" aria-label={drillLabel} onClick={onDrill} className={cn(iconButton, "size-7 shrink-0")}>
        <ChevronRight size={16} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </div>
  );
}

function LinkTile({
  label,
  icon: Icon,
  capability,
  page,
}: {
  label: string;
  icon: LucideIcon;
  capability: ControlCapability;
  page: string;
}) {
  return (
    <button
      type="button"
      title={capability.reason || `Open ${label.toLowerCase()} settings`}
      onClick={() => openSettings(page)}
      className="flex flex-col items-center gap-1.5 rounded-tile bg-raised px-2 py-2.5 text-xs text-fg-2 hover:bg-hover hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hi"
    >
      <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
      <span className="truncate">{label}</span>
    </button>
  );
}

export function ControlsCard({
  status,
  pending,
  error,
  refresh,
}: {
  status: ControlsStatus | null;
  pending: boolean;
  error: string | null;
  refresh: () => void | Promise<void>;
}) {
  const [view, setView] = useState<View>("main");
  const [volume, setVolume] = useState(status?.volumePercent ?? 0);
  const [brightness, setBrightness] = useState(status?.brightnessPercent ?? 0);
  const [busy, setBusy] = useState<ReadonlySet<ControlKey>>(() => new Set<ControlKey>());
  const [optimistic, setOptimistic] = useState<Optimistic>({});
  const [failure, setFailure] = useState<{ message: string; page: string } | null>(null);
  const inFlight = useRef(new Set<ControlKey>());
  const latestRun = useRef<Partial<Record<ControlKey, number>>>({});

  // Slider values follow the engine between commits.
  useEffect(() => {
    setVolume(status?.volumePercent ?? 0);
  }, [status?.volumePercent]);
  useEffect(() => {
    setBrightness(status?.brightnessPercent ?? 0);
  }, [status?.brightnessPercent]);

  if (pending && !status) {
    return (
      <CardFrame icon={SlidersHorizontal} title="Quick Settings">
        <p className="text-[13px] text-fg-3">Loading settings.</p>
      </CardFrame>
    );
  }
  if (!status) {
    return (
      <CardFrame icon={SlidersHorizontal} title="Quick Settings">
        <p className="text-[13px] text-fg-2">Controls are unavailable right now.</p>
        {error ? (
          <p role="alert" className="text-xs text-danger">
            {error}
          </p>
        ) : null}
        <button
          type="button"
          onClick={() => refresh()}
          className="self-start rounded-full bg-raised px-3 py-1.5 text-xs text-fg hover:bg-hover"
        >
          Retry
        </button>
      </CardFrame>
    );
  }

  const wifiOn = (optimistic.wifi ?? status.wifiEnabled) === true;
  const btOn = (optimistic.bluetooth ?? status.bluetoothEnabled) === true;
  const muted = (optimistic.muted ?? status.muted) === true;
  const outputId = optimistic.output ?? status.defaultAudioId;
  const outputs =
    optimistic.output === undefined
      ? status.audioOutputs
      : status.audioOutputs.map((output) => ({ ...output, isDefault: output.id === optimistic.output }));
  const current = activeOutput(outputs, outputId);
  const connectedNetwork = status.wifiNetworks.find((network) => network.connected)?.ssid;
  const connectedDevices = status.bluetoothDevices.filter((device) => device.connected).length;

  const setInFlight = (key: ControlKey, on: boolean) => {
    if (on) inFlight.current.add(key);
    else inFlight.current.delete(key);
    setBusy(new Set(inFlight.current));
  };
  const run = (
    key: ControlKey,
    label: string,
    action: () => Promise<unknown>,
    page = "display",
    shown: Optimistic = {},
  ) => {
    if (inFlight.current.has(key)) return;
    const runId = (latestRun.current[key] ?? 0) + 1;
    latestRun.current[key] = runId;
    const settleShown = () => {
      // A newer action on this control owns the shown value; leave it alone.
      if (latestRun.current[key] === runId) setOptimistic((previous) => without(previous, shown));
    };
    setInFlight(key, true);
    setFailure(null);
    setOptimistic((previous) => ({ ...previous, ...shown }));
    action().then(
      () => {
        // Busy ends with the action; the status refresh is slow and runs behind it.
        setInFlight(key, false);
        void Promise.resolve(refresh()).catch(() => undefined).finally(settleShown);
      },
      (reason: unknown) => {
        setInFlight(key, false);
        settleShown();
        const message = reason instanceof Error ? reason.message : typeof reason === "string" ? reason : "";
        // Location-consent failures from the Wi-Fi toggle point at the privacy page.
        const consent = page === "wifi" && /location|permission|consent|denied/i.test(message);
        setVolume(status.volumePercent ?? 0);
        setBrightness(status.brightnessPercent ?? 0);
        setFailure({ message: message || `Couldn't change ${label}.`, page: consent ? "privacy_location" : page });
      },
    );
  };
  const commitVolume = () => {
    if (volume !== status.volumePercent) run("volume", "volume", () => api.setVolume(volume));
  };
  const commitBrightness = () => {
    if (brightness !== status.brightnessPercent) run("brightness", "brightness", () => api.setBrightness(brightness));
  };
  const back = () => setView("main");

  const networksPane = (
    <Pane key="wifi" onEscape={back}>
      <DrillHeader title="Wi-Fi" onBack={back} />
      <ScrollArea className="min-h-0 flex-1">
        <div className="flex flex-col gap-2">
          {!status.wifi.supported ? (
            <p className="text-[13px] text-fg-2">{status.wifi.reason || "Wi-Fi can't be changed here."}</p>
          ) : null}
          {status.wifiError ? (
            <p role="alert" className="text-xs text-danger">
              {status.wifiError}
            </p>
          ) : null}
          {status.wifiNetworks.length === 0 ? (
            <p className="text-[13px] text-fg-3">
              {status.wifiEnabled === false ? "Turn Wi-Fi on to see networks." : "No networks found."}
            </p>
          ) : (
            <ul className="flex flex-col gap-1">
              {status.wifiNetworks.map((network, index) => (
                <li
                  key={`${network.ssid}-${index}`}
                  className={cn("flex items-center gap-3 rounded-row px-3 py-2.5", network.connected && "bg-active")}
                >
                  <Wifi size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-2" />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[13px]">{network.ssid}</span>
                    <span className="block truncate text-xs text-fg-3">{network.security}</span>
                  </span>
                  <span className={cn("shrink-0 text-xs tabular-nums", network.connected ? "text-ok" : "text-fg-3")}>
                    {network.connected ? "Connected" : `${network.signal}%`}
                  </span>
                </li>
              ))}
            </ul>
          )}
          <SettingsLink label="Open Wi-Fi settings" page="wifi" />
        </div>
      </ScrollArea>
    </Pane>
  );

  const devicesPane = (
    <Pane key="bluetooth" onEscape={back}>
      <DrillHeader title="Bluetooth" onBack={back} />
      <ScrollArea className="min-h-0 flex-1">
        <div className="flex flex-col gap-2">
          {!status.bluetooth.supported ? (
            <p className="text-[13px] text-fg-2">{status.bluetooth.reason || "Bluetooth can't be changed here."}</p>
          ) : null}
          {status.bluetoothError ? (
            <p role="alert" className="text-xs text-danger">
              {status.bluetoothError}
            </p>
          ) : null}
          {status.bluetoothDevices.length === 0 ? (
            <p className="text-[13px] text-fg-3">
              {status.bluetoothEnabled === false ? "Turn Bluetooth on to see devices." : "No devices found."}
            </p>
          ) : (
            <ul className="flex flex-col gap-1">
              {status.bluetoothDevices.map((device, index) => (
                <li key={`${device.id}-${index}`} className="flex items-center gap-3 rounded-row px-3 py-2.5">
                  <Bluetooth size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-2" />
                  <span className="min-w-0 flex-1 truncate text-[13px]">{device.name}</span>
                  <span
                    className={cn(
                      "shrink-0 text-xs",
                      device.connected ? "text-ok" : device.paired ? "text-fg-2" : "text-fg-3",
                    )}
                  >
                    {device.connected ? "Connected" : device.paired ? "Paired" : "Not paired"}
                  </span>
                </li>
              ))}
            </ul>
          )}
          <SettingsLink label="Open Bluetooth settings" page="bluetooth" />
        </div>
      </ScrollArea>
    </Pane>
  );

  const outputPane = (
    <Pane key="output" onEscape={back}>
      <DrillHeader title="Audio output" onBack={back} />
      <ScrollArea className="min-h-0 flex-1">
        <AudioOutputList
          outputs={outputs}
          defaultId={outputId}
          onSelect={(deviceId) => {
            setView("main");
            run("output", "audio output", () => api.setAudioOutput(deviceId), "display", { output: deviceId });
          }}
        />
      </ScrollArea>
    </Pane>
  );

  const mainPane = (
    <Pane key="main">
      <ScrollArea className="min-h-0 flex-1">
        <div className="flex flex-col gap-3">
          <div className="grid grid-cols-2 gap-2">
            <ToggleTile
              label="Wi-Fi"
              drillLabel="Wi-Fi networks"
              on={wifiOn}
              icon={wifiOn ? Wifi : WifiOff}
              detail={
                !status.wifi.supported
                  ? "Can't change"
                  : status.wifiEnabled == null
                    ? "Unavailable"
                    : !wifiOn
                      ? "Off"
                      : (connectedNetwork ?? "On")
              }
              title={status.wifi.supported ? "Turn Wi-Fi on or off" : status.wifi.reason || "Wi-Fi can't be changed here."}
              disabled={!status.wifi.supported || status.wifiEnabled == null || busy.has("wifi")}
              onToggle={() => run("wifi", "Wi-Fi", () => api.setWifi(!wifiOn), "wifi", { wifi: !wifiOn })}
              onDrill={() => setView("wifi")}
            />
            <ToggleTile
              label="Bluetooth"
              drillLabel="Bluetooth devices"
              on={btOn}
              icon={btOn ? Bluetooth : BluetoothOff}
              detail={
                !status.bluetooth.supported
                  ? "Can't change"
                  : status.bluetoothEnabled == null
                    ? "Unavailable"
                    : !btOn
                      ? "Off"
                      : connectedDevices > 0
                        ? `${connectedDevices} connected`
                        : "On"
              }
              title={
                status.bluetooth.supported ? "Turn Bluetooth on or off" : status.bluetooth.reason || "Bluetooth can't be changed here."
              }
              disabled={!status.bluetooth.supported || status.bluetoothEnabled == null || busy.has("bluetooth")}
              onToggle={() => run("bluetooth", "Bluetooth", () => api.setBluetooth(!btOn), "bluetooth", { bluetooth: !btOn })}
              onDrill={() => setView("bluetooth")}
            />
          </div>

          <div className="grid grid-cols-3 gap-2">
            <LinkTile label="Airplane" icon={Plane} capability={status.airplaneMode} page="airplane_mode" />
            <LinkTile label="Night light" icon={Moon} capability={status.nightLight} page="night_light" />
            <LinkTile label="Energy saver" icon={Leaf} capability={status.energySaver} page="energy_saver" />
          </div>

          <div className="flex flex-col gap-3 rounded-tile bg-raised p-3">
            <div className="flex items-center gap-3">
              <button
                type="button"
                aria-label={muted ? "Unmute" : "Mute"}
                title={muted ? "Unmute" : "Mute"}
                disabled={status.muted == null || busy.has("mute")}
                onClick={() => run("mute", "mute", () => api.setMuted(!muted), "display", { muted: !muted })}
                className={cn(iconButton, "size-8 shrink-0 disabled:opacity-50")}
              >
                {muted ? <VolumeX size={16} strokeWidth={1.75} aria-hidden="true" /> : <Volume2 size={16} strokeWidth={1.75} aria-hidden="true" />}
              </button>
              <div className="min-w-0 flex-1" onPointerUp={commitVolume} onKeyUp={commitVolume}>
                <Slider
                  label="Volume"
                  value={volume}
                  onChange={setVolume}
                  disabled={status.volumePercent == null || busy.has("volume")}
                />
              </div>
              <span className="w-9 shrink-0 text-right text-xs tabular-nums text-fg-2">
                {status.volumePercent == null ? "—" : `${volume}%`}
              </span>
            </div>

            {status.brightnessCapability.supported ? (
              <div className="flex items-center gap-3">
                <span className="grid size-8 shrink-0 place-items-center text-fg-2">
                  <Sun size={16} strokeWidth={1.75} aria-hidden="true" />
                </span>
                <div className="min-w-0 flex-1" onPointerUp={commitBrightness} onKeyUp={commitBrightness}>
                  <Slider
                    label="Brightness"
                    value={brightness}
                    onChange={setBrightness}
                    disabled={status.brightnessPercent == null || busy.has("brightness")}
                  />
                </div>
                <span className="w-9 shrink-0 text-right text-xs tabular-nums text-fg-2">
                  {status.brightnessPercent == null ? "—" : `${brightness}%`}
                </span>
              </div>
            ) : null}
          </div>

          <button
            type="button"
            disabled={outputs.length === 0}
            onClick={() => setView("output")}
            className="flex items-center gap-3 rounded-tile bg-raised px-3 py-2.5 text-left text-[13px] hover:bg-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hi disabled:opacity-50"
          >
            <Speaker size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-2" />
            <span className="text-fg-2">Output</span>
            <span className="min-w-0 flex-1 truncate text-right">{current?.name ?? "No audio output"}</span>
            <ChevronRight size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-3" />
          </button>

          {failure ? (
            <div role="alert" className="flex items-center justify-between gap-3 rounded-tile bg-raised px-3 py-2">
              <span className="min-w-0 text-xs text-danger">{failure.message}</span>
              <SettingsLink label="Open Windows settings" page={failure.page} />
            </div>
          ) : null}

        </div>
      </ScrollArea>
    </Pane>
  );

  return (
    <CardFrame icon={SlidersHorizontal} title="Quick Settings">
      <AnimatePresence mode="wait" initial={false}>
        {view === "wifi" ? networksPane : view === "bluetooth" ? devicesPane : view === "output" ? outputPane : mainPane}
      </AnimatePresence>
    </CardFrame>
  );
}
