import { useEffect, useState } from "react";
import { api } from "../bridge/api";
import type {
  AudioDeviceInfo,
  ControlCapability,
  ControlsStatus,
  MediaSessionInfo,
} from "../bridge/types";
import { CardHead, EmptyState, UnsupportedCard } from "./IslandCards";
import { IslandIcon } from "./IslandIcon";
import { relativeTime } from "./islandData";

/**
 * Native cards wired to the landed shell controls bridge. Capabilities are
 * honoured literally: unsupported media/Wi-Fi/Bluetooth/notifications sections
 * stay disabled with the engine's reason; supported volume/brightness/output
 * controls call the real setters and refresh the status after each change.
 */

function formatTime(ms: number | null): string {
  if (ms == null || !Number.isFinite(ms)) return "—";
  const total = Math.max(0, Math.floor(ms / 1000));
  return Math.floor(total / 60) + ":" + String(total % 60).padStart(2, "0");
}

function capabilityReason(capability: ControlCapability, fallback: string): string {
  return capability.supported ? fallback : capability.reason || fallback;
}

function CardLoading({ card, title, subtitle }: { card: "media" | "controls" | "bell"; title: string; subtitle: string }) {
  return (
    <div className="island-card">
      <CardHead card={card} title={title} subtitle={subtitle} />
      <div className="island-divider" />
      <div className="island-skeleton" aria-hidden="true">
        {[0, 1, 2].map((row) => (
          <span className="island-skeleton-row" key={row} />
        ))}
      </div>
    </div>
  );
}

function CardError({ card, title, subtitle, error, refresh }: { card: "media" | "controls" | "bell"; title: string; subtitle: string; error: string; refresh: () => void }) {
  return (
    <div className="island-card">
      <CardHead card={card} title={title} subtitle={subtitle} />
      <div className="island-divider" />
      <div className="island-empty" role="alert">
        <strong>Controls are unavailable right now.</strong>
        <span>{error}</span>
        <button type="button" className="island-agent-button" onClick={refresh}>
          Retry
        </button>
      </div>
    </div>
  );
}

export function MediaCard({
  status,
  pending,
  error,
  refresh,
}: {
  status: ControlsStatus | null;
  pending: boolean;
  error: string | null;
  refresh: () => void;
}) {
  if (pending && !status) return <CardLoading card="media" title="Media" subtitle="Now playing" />;
  if (error && !status) {
    return <CardError card="media" title="Media" subtitle="Now playing" error={error} refresh={refresh} />;
  }
  if (!status) return <UnsupportedCard card="media" title="Media" subtitle="Now playing" reason="Controls are unavailable right now." />;
  const capability = status.mediaCapability;
  if (!capability.supported) {
    return <UnsupportedCard card="media" title="Media" subtitle="Now playing" reason={capabilityReason(capability, "Media session adapter isn't wired in this build.")} />;
  }
  const media: MediaSessionInfo | null = status.media;
  if (!media) {
    return (
      <div className="island-card island-media-card">
        <CardHead card="media" title="Media" subtitle="Now playing" />
        <div className="island-divider" />
        <EmptyState title="Nothing is playing right now." hint="Start playback and the current session appears here." />
      </div>
    );
  }
  return (
    <div className="island-card island-media-card">
      <CardHead card="media" title="Media" subtitle="Now playing" />
      <div className="island-divider" />
      <div className="island-media-body">
        <div className="island-media-label">CURRENT SESSION</div>
        <div className="island-media-title">{media.title}</div>
        <div className="island-media-artist">
          {media.artist}
          {media.album ? " · " + media.album : ""}
        </div>
        <div className="island-media-progress" aria-hidden="true">
          <span
            className="island-media-progress-fill"
            style={{
              width:
                media.positionMs != null && media.durationMs
                  ? Math.min(100, (media.positionMs / media.durationMs) * 100) + "%"
                  : "0%",
            }}
          />
        </div>
        <div className="island-media-time">
          <span>{formatTime(media.positionMs)}</span>
          <span>{formatTime(media.durationMs)}</span>
        </div>
        <div className="island-media-actions">
          {(["prev", "play", "next"] as const).map((action) => (
            <button
              key={action}
              type="button"
              className="island-media-button"
              title="Control the current media session"
              onClick={() => {
                void api
                  .mediaCommand(action === "play" ? (media.playing ? "pause" : "play") : action)
                  .then(refresh)
                  .catch(() => undefined);
              }}
            >
              <IslandIcon
                name={action === "prev" ? "prev" : action === "next" ? "next" : media.playing ? "pause" : "play"}
                size={action === "play" ? 20 : 18}
              />
            </button>
          ))}
        </div>
        {status.audioOutputs.length > 0 ? (
          <AudioOutputs
            outputs={status.audioOutputs}
            defaultId={status.defaultAudioId}
            onSelect={(deviceId) => {
              void api.setAudioOutput(deviceId).then(refresh).catch(() => undefined);
            }}
          />
        ) : null}
      </div>
    </div>
  );
}

function AudioOutputs({
  outputs,
  defaultId,
  onSelect,
}: {
  outputs: AudioDeviceInfo[];
  defaultId: string | null;
  onSelect: (deviceId: string) => void;
}) {
  return (
    <div className="island-media-outputs">
      <span className="island-context-caps">OUTPUT</span>
      {outputs.map((output) => {
        const active = output.isDefault || output.id === defaultId;
        return (
          <button
            type="button"
            className={"island-media-output" + (active ? " is-active" : "")}
            key={output.id}
            onClick={() => onSelect(output.id)}
          >
            <IslandIcon name="output" size={16} />
            <span>{output.name}</span>
            {active ? <IslandIcon name="check" size={14} /> : null}
          </button>
        );
      })}
    </div>
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
  refresh: () => void;
}) {
  const [volume, setVolume] = useState(status?.volumePercent ?? 0);
  const [brightness, setBrightness] = useState(status?.brightnessPercent ?? 0);
  const [audioOpen, setAudioOpen] = useState(false);
  const [drilldown, setDrilldown] = useState<"wifi" | "bluetooth" | null>(null);
  const [controlError, setControlError] = useState<string | null>(null);
  const [controlErrorPage, setControlErrorPage] = useState<string>("display");
  useEffect(() => {
    setVolume(status?.volumePercent ?? 0);
    setBrightness(status?.brightnessPercent ?? 0);
  }, [status?.volumePercent, status?.brightnessPercent]);

  const runControl = (label: string, action: () => Promise<unknown>, fallbackPage = "display") => {
    setControlError(null);
    void action()
      .then(refresh)
      .catch((reason: unknown) => {
        const message = reason instanceof Error ? reason.message : "";
        setControlError("Couldn't change " + label + ".");
        setControlErrorPage(
          /location|permission|consent/i.test(message) ? "privacy_location" : fallbackPage,
        );
      });
  };
  const openSettings = (page: string) => {
    void api.openSettings(page).catch(() => undefined);
  };

  if (pending && !status) return <CardLoading card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" />;
  if (error && !status) {
    return <CardError card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" error={error} refresh={refresh} />;
  }
  if (!status) {
    return <UnsupportedCard card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" reason="Controls are unavailable right now." />;
  }
  const output = status.audioOutputs.find((device) => device.id === status.defaultAudioId) ?? status.audioOutputs[0] ?? null;
  const wifiSupported = status.wifi.supported;
  const bluetoothSupported = status.bluetooth.supported;

  if (drilldown) {
    const isWifi = drilldown === "wifi";
    return (
      <div className="island-card island-controls-card">
        <CardHead
          card="controls"
          title={isWifi ? "Wi-Fi" : "Bluetooth"}
          subtitle={isWifi ? "Networks and connectivity" : "Paired devices"}
        />
        <div className="island-divider" />
        <button
          type="button"
          className="island-drilldown-back"
          onClick={() => setDrilldown(null)}
        >
          <IslandIcon name="back" size={16} /> Back to Quick Settings
        </button>
        {isWifi ? (
          status.wifiError ? (
            <div className="island-control-error">
              <span>{status.wifiError}</span>
              <button type="button" className="island-control-link" onClick={() => openSettings("privacy_location")}>
                Open location settings
              </button>
            </div>
          ) : status.wifiNetworks.length === 0 ? (
            <div className="island-control-error">
              <span>No Wi-Fi networks are available here.</span>
              <button type="button" className="island-control-link" onClick={() => openSettings("wifi")}>
                Open network settings
              </button>
            </div>
          ) : (
            <div className="island-audio-list" role="list" aria-label="Wi-Fi networks">
              {status.wifiNetworks.map((network) => (
                <div className="island-drilldown-row" key={network.ssid}>
                  <IslandIcon name="wifi" size={18} />
                  <span>
                    {network.ssid + " · " + network.signal + "% · " + network.security}
                    {network.connected ? " · connected" : ""}
                  </span>
                  {network.hasProfile && network.connectable ? (
                    <button
                      type="button"
                      className="island-drilldown-action"
                      onClick={() => runControl("Wi-Fi", () => api.connectWifi(network.ssid), "wifi")}
                    >
                      Connect
                    </button>
                  ) : (
                    <button type="button" className="island-drilldown-action" onClick={() => openSettings("wifi")}>
                      Open network settings
                    </button>
                  )}
                </div>
              ))}
            </div>
          )
        ) : status.bluetoothError ? (
          <div className="island-control-error">
            <span>{status.bluetoothError}</span>
            <button type="button" className="island-control-link" onClick={() => openSettings("bluetooth")}>
              Open Bluetooth settings
            </button>
          </div>
        ) : status.bluetoothDevices.length === 0 ? (
          <div className="island-control-error">
            <span>No paired Bluetooth devices.</span>
            <button type="button" className="island-control-link" onClick={() => openSettings("bluetooth")}>
              Open Bluetooth settings
            </button>
          </div>
        ) : (
          <div className="island-audio-list" role="list" aria-label="Bluetooth devices">
            {status.bluetoothDevices.map((device) => (
              <div className="island-drilldown-row" key={device.id}>
                <IslandIcon name="bluetooth" size={18} />
                <span>{device.name + (device.connected ? " · connected" : " · paired")}</span>
                <button type="button" className="island-drilldown-action" onClick={() => openSettings("bluetooth")}>
                  Manage
                </button>
              </div>
            ))}
          </div>
        )}
        {controlError ? <p className="island-control-error">{controlError}</p> : null}
      </div>
    );
  }

  return (
    <div className="island-card island-controls-card">
      <CardHead card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" />
      <div className="island-divider" />

      <div className="island-setting-row">
        <IslandIcon name="volume" size={19} />
        <span className="island-setting-label">Volume</span>
        <input
          className="island-setting-slider"
          type="range"
          min={0}
          max={100}
          value={volume}
          disabled={status.volumePercent == null}
          aria-label="Volume"
          onChange={(event) => setVolume(Number(event.target.value))}
          onPointerUp={() => runControl("volume", () => api.setVolume(volume))}
          onKeyUp={() => runControl("volume", () => api.setVolume(volume))}
        />
        <span className="island-setting-value">
          {status.volumePercent == null ? "—" : volume + "%"}
        </span>
        <button
          type="button"
          className="island-setting-toggle"
          aria-pressed={status.muted === true}
          disabled={status.muted == null}
          title="Mute or unmute"
          onClick={() => runControl("mute", () => api.setMuted(!(status.muted === true)))}
        >
          {status.muted ? "Unmute" : "Mute"}
        </button>
      </div>

      {status.brightnessCapability.supported ? (
        <div className="island-setting-row">
          <IslandIcon name="brightness" size={19} />
          <span className="island-setting-label">Brightness</span>
          <input
            className="island-setting-slider"
            type="range"
            min={0}
            max={100}
            value={brightness}
            disabled={status.brightnessPercent == null}
            aria-label="Brightness"
            onChange={(event) => setBrightness(Number(event.target.value))}
            onPointerUp={() => runControl("brightness", () => api.setBrightness(brightness))}
            onKeyUp={() => runControl("brightness", () => api.setBrightness(brightness))}
          />
          <span className="island-setting-value">
            {status.brightnessPercent == null ? "—" : brightness + "%"}
          </span>
        </div>
      ) : (
        <div className="island-control-fallback-row">
          <span>Brightness can't be changed here.</span>
          <button type="button" className="island-control-link" onClick={() => openSettings("display")}>
            Open display settings
          </button>
        </div>
      )}

      {status.audioOutputs.length > 0 ? (
        <>
          <button
            type="button"
            className="island-audio-row"
            aria-expanded={audioOpen}
            onClick={() => setAudioOpen((open) => !open)}
          >
            <IslandIcon name="output" size={16} />
            <span className="island-audio-label">Audio output</span>
            <span className="island-audio-device">{output?.name ?? "Choose output"}</span>
            <IslandIcon name="chevron" size={16} />
          </button>
          {audioOpen ? (
            <div className="island-audio-list" role="radiogroup" aria-label="Audio output">
              {status.audioOutputs.map((device) => (
                <button
                  key={device.id}
                  type="button"
                  role="radio"
                  aria-checked={device.id === status.defaultAudioId || device.isDefault}
                  className={
                    "island-audio-option" +
                    (device.id === status.defaultAudioId || device.isDefault ? " is-active" : "")
                  }
                  onClick={() =>
                    runControl("audio output", () => api.setAudioOutput(device.id))
                  }
                >
                  <IslandIcon name="volume" size={16} />
                  <span>{device.name}</span>
                  {device.id === status.defaultAudioId || device.isDefault ? (
                    <IslandIcon name="check" size={14} />
                  ) : null}
                </button>
              ))}
            </div>
          ) : null}
        </>
      ) : null}

      <div className="island-control-grid">
        <div className="island-control-tile">
          <IslandIcon name="wifi" size={19} />
          <span className="island-control-tile-body">
            <span className="island-control-tile-label">Wi-Fi</span>
            <span className="island-control-tile-value">
              {wifiSupported ? (status.wifiEnabled == null ? "—" : status.wifiEnabled ? "On" : "Off") : "Can't change here"}
            </span>
          </span>
          <button
            type="button"
            role="switch"
            aria-checked={status.wifiEnabled === true}
            aria-label="Wi-Fi"
            className="island-control-tile-toggle"
            disabled={!wifiSupported || status.wifiEnabled == null}
            title={wifiSupported ? "Toggle Wi-Fi" : status.wifi.reason || "Wi-Fi can't be changed here."}
            onClick={() => runControl("Wi-Fi", () => api.setWifi(!(status.wifiEnabled === true)), "wifi")}
          >
            {status.wifiEnabled ? "On" : "Off"}
          </button>
          <button
            type="button"
            className="island-control-tile-chevron"
            aria-label="Show Wi-Fi networks"
            onClick={() => setDrilldown((current) => (current === "wifi" ? null : "wifi"))}
          >
            <IslandIcon name="chevron" size={16} />
          </button>
        </div>
        <div className="island-control-tile">
          <IslandIcon name="bluetooth" size={19} />
          <span className="island-control-tile-body">
            <span className="island-control-tile-label">Bluetooth</span>
            <span className="island-control-tile-value">
              {bluetoothSupported
                ? status.bluetoothEnabled == null
                  ? "—"
                  : status.bluetoothEnabled
                    ? "On"
                    : "Off"
                : "Can't change here"}
            </span>
          </span>
          <button
            type="button"
            role="switch"
            aria-checked={status.bluetoothEnabled === true}
            aria-label="Bluetooth"
            className="island-control-tile-toggle"
            disabled={!bluetoothSupported || status.bluetoothEnabled == null}
            title={
              bluetoothSupported
                ? "Toggle Bluetooth"
                : status.bluetooth.reason || "Bluetooth can't be changed here."
            }
            onClick={() =>
              runControl(
                "Bluetooth",
                () => api.setBluetooth(!(status.bluetoothEnabled === true)),
                "bluetooth",
              )
            }
          >
            {status.bluetoothEnabled ? "On" : "Off"}
          </button>
          <button
            type="button"
            className="island-control-tile-chevron"
            aria-label="Show Bluetooth devices"
            onClick={() =>
              setDrilldown((current) => (current === "bluetooth" ? null : "bluetooth"))
            }
          >
            <IslandIcon name="chevron" size={16} />
          </button>
        </div>
      </div>

      {controlError ? (
        <div className="island-control-error">
          <span>{controlError}</span>
          <button
            type="button"
            className="island-control-link"
            onClick={() => openSettings(controlErrorPage)}
          >
            Open Windows settings
          </button>
        </div>
      ) : null}

      <div className="island-settings-actions" aria-label="Open Windows settings shortcuts">
        <button
          type="button"
          title="Open Windows settings for Airplane mode"
          aria-label="Open Windows settings for Airplane mode"
          onClick={() => openSettings("airplane_mode")}
        >
          <IslandIcon name="airplane" size={14} /> Airplane
          <IslandIcon name="chevron" size={11} />
        </button>
        <button
          type="button"
          title="Open Windows settings for Night light"
          aria-label="Open Windows settings for Night light"
          onClick={() => openSettings("night_light")}
        >
          <IslandIcon name="brightness" size={14} /> Night light
          <IslandIcon name="chevron" size={11} />
        </button>
        <button
          type="button"
          title="Open Windows settings for Energy saver"
          aria-label="Open Windows settings for Energy saver"
          onClick={() => openSettings("energy_saver")}
        >
          <IslandIcon name="bolt" size={14} /> Energy saver
          <IslandIcon name="chevron" size={11} />
        </button>
      </div>
      <button type="button" className="island-control-settings" onClick={() => openSettings("display")}>
        <IslandIcon name="controls" size={16} />
        <span>Open Windows settings</span>
        <IslandIcon name="chevron" size={16} />
      </button>
    </div>
  );
}

export function NotificationsCard({
  status,
  pending,
  error,
  refresh,
}: {
  status: ControlsStatus | null;
  pending: boolean;
  error: string | null;
  refresh: () => void;
}) {
  if (pending && !status) return <CardLoading card="bell" title="Notifications" subtitle="Recent activity" />;
  if (error && !status) {
    return <CardError card="bell" title="Notifications" subtitle="Recent activity" error={error} refresh={refresh} />;
  }
  if (!status) {
    return <UnsupportedCard card="bell" title="Notifications" subtitle="Recent activity" reason="Controls are unavailable right now." />;
  }
  const supported = status.notifications.supported;
  return (
    <div className="island-card island-notifications-card">
      <CardHead
        card="bell"
        title="Notifications"
        subtitle={
          status.notificationUnread > 0
            ? "Recent activity · " + status.notificationUnread + " unread"
            : "Recent activity"
        }
      />
      <div className="island-divider" />
      {!supported ? (
        <div className="island-unsupported" role="status">
          <span className="island-unsupported-icon">
            <IslandIcon name="bell" size={23} />
          </span>
          <strong>Notifications aren't available yet</strong>
          <span>{status.notifications.reason || "Notification history isn't wired in this build."}</span>
        </div>
      ) : status.notificationItems.length === 0 ? (
        <EmptyState title="All caught up." hint="New notifications will appear here." />
      ) : (
        <div className="island-notification-list">
          {status.notificationItems.map((item) => (
            <div className="island-notification-row" key={item.id}>
              <IslandIcon name="bell" size={18} />
              <span className="island-notification-body">
                <span className="island-notification-title">{item.title}</span>
                <span className="island-notification-text">{item.body}</span>
                <span className="island-notification-source">
                  {item.source}
                  {item.createdAt ? " · " + relativeTime(item.createdAt) : ""}
                </span>
              </span>
              <button
                type="button"
                className="island-dismiss-button"
                disabled={!supported}
                title="Dismiss this notification"
                aria-label={"Dismiss " + item.title}
                onClick={() => {
                  void api
                    .notificationsDismiss(item.id)
                    .then(refresh)
                    .catch(() => undefined);
                }}
              >
                <IslandIcon name="dismiss" size={15} />
              </button>
            </div>
          ))}
        </div>
      )}
      <div className="island-notification-footer">
        <button
          type="button"
          className="island-agent-button"
          disabled={!supported || status.notificationItems.length === 0}
          title={
            status.notificationItems.length === 0
              ? "No notifications to clear."
              : "Dismiss all stored notifications"
          }
          onClick={() => {
            void api
              .notificationsClear()
              .then(refresh)
              .catch(() => undefined);
          }}
        >
          Clear all
        </button>
      </div>
    </div>
  );
}