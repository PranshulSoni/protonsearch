import { useEffect, useRef, useState } from "react";
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
 refresh: () => void | Promise<void>;
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
                  .mediaCommand(
                  action === "play"
                    ? media.playing
                      ? "pause"
                      : "play"
                    : action === "prev"
                      ? "previous"
                      : action,
                )
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
  const [controlError, setControlError] = useState<string | null>(null);
  const [controlSuccess, setControlSuccess] = useState<string | null>(null);
  const [controlErrorPage, setControlErrorPage] = useState<string>("display");
  const [controlPending, setControlPending] = useState<string | null>(null);
  const controlPendingRef = useRef(false);
  useEffect(() => {
    setVolume(status?.volumePercent ?? 0);
    setBrightness(status?.brightnessPercent ?? 0);
  }, [status?.volumePercent, status?.brightnessPercent]);

  const runControl = (label: string, action: () => Promise<unknown>, fallbackPage = "display") => {
    if (controlPendingRef.current) return;
    controlPendingRef.current = true;
    setControlPending(label);
    setControlError(null);
    setControlSuccess(null);
    void action()
      .then(() => refresh())
      .then(() => setControlSuccess("Changed " + label + "."))
      .catch((reason: unknown) => {
        const message = reason instanceof Error ? reason.message : typeof reason === "string" ? reason : "";
        setVolume(status?.volumePercent ?? 0);
        setBrightness(status?.brightnessPercent ?? 0);
        setControlError(message || "Couldn't change " + label + ".");
        setControlErrorPage(
          fallbackPage === "wifi" && /location|permission|consent|denied/i.test(message)
            ? "privacy_location"
            : fallbackPage,
        );
      })
      .finally(() => {
        controlPendingRef.current = false;
        setControlPending(null);
      });
  };
  const openSettings = (page: string) => {
    void api.openSettings(page).catch(() => undefined);
  };
  const feedback = controlPending || controlError || controlSuccess ? (
    <div className={"island-control-feedback" + (controlError ? " is-error" : controlPending ? " is-pending" : " is-success")} role={controlError ? "alert" : "status"}>
      <span>{controlPending ? "Updating " + controlPending + "…" : controlError ?? controlSuccess}</span>
      {controlError ? <button type="button" className="island-control-link" onClick={() => openSettings(controlErrorPage)}>
        Open Windows settings
      </button> : null}
    </div>
  ) : null;

  if (pending && !status) return <CardLoading card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" />;
  if (error && !status) {
    return <CardError card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" error={error} refresh={refresh} />;
  }
  if (!status) {
    return <UnsupportedCard card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" reason="Controls are unavailable right now." />;
  }

  const output = status.audioOutputs.find((device) => device.id === status.defaultAudioId) ?? status.audioOutputs[0] ?? null;
  return (
    <div className="island-card island-controls-card">
      <CardHead card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" />

      <div className="island-setting-row">
        <IslandIcon name="volume" size={18} />
        <span className="island-setting-label">Volume</span>
        <input
          className="island-setting-slider"
          type="range"
          min={0}
          max={100}
          value={volume}
          disabled={status.volumePercent == null || controlPending != null}
          aria-label="Volume"
          onChange={(event) => setVolume(Number(event.target.value))}
          onPointerUp={() => runControl("volume", () => api.setVolume(volume))}
          onKeyUp={() => runControl("volume", () => api.setVolume(volume))}
        />
        <span className="island-setting-value">{status.volumePercent == null ? "—" : volume + "%"}</span>
        <button
          type="button"
          className="island-mute-button"
          aria-pressed={status.muted === true}
          aria-label={status.muted ? "Unmute" : "Mute"}
          title={status.muted ? "Unmute" : "Mute"}
          disabled={status.muted == null || controlPending != null}
          onClick={() => runControl("mute", () => api.setMuted(!(status.muted === true)))}
        >
          <IslandIcon name="volume" size={16} />
        </button>
      </div>

      {status.brightnessCapability.supported ? (
        <div className="island-setting-row">
          <IslandIcon name="brightness" size={18} />
          <span className="island-setting-label">Brightness</span>
          <input
            className="island-setting-slider"
            type="range"
            min={0}
            max={100}
            value={brightness}
            disabled={status.brightnessPercent == null || controlPending != null}
            aria-label="Brightness"
            onChange={(event) => setBrightness(Number(event.target.value))}
            onPointerUp={() => runControl("brightness", () => api.setBrightness(brightness))}
            onKeyUp={() => runControl("brightness", () => api.setBrightness(brightness))}
          />
          <span className="island-setting-value">{status.brightnessPercent == null ? "—" : brightness + "%"}</span>
        </div>
      ) : null}

      {status.audioOutputs.length > 0 ? (
        <>
          <button type="button" className="island-audio-row" aria-expanded={audioOpen} onClick={() => setAudioOpen((open) => !open)}>
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
                  className={"island-audio-option" + (device.id === status.defaultAudioId || device.isDefault ? " is-active" : "")}
                  disabled={controlPending != null}
                  onClick={() => runControl("audio output", () => api.setAudioOutput(device.id))}
                >
                  <IslandIcon name="volume" size={16} />
                  <span>{device.name}</span>
                  {device.id === status.defaultAudioId || device.isDefault ? <IslandIcon name="check" size={14} /> : null}
                </button>
              ))}
            </div>
          ) : null}
        </>
      ) : null}

      <div className="island-control-grid">
        <div className="island-control-tile">
          <button
            type="button"
            role="switch"
            aria-checked={status.wifiEnabled === true}
            aria-label="Wi-Fi"
            className="island-tile-main"
            disabled={!status.wifi.supported || status.wifiEnabled == null || controlPending != null}
            title={status.wifi.supported ? "Toggle Wi-Fi" : status.wifi.reason || "Wi-Fi can't be changed here."}
            onClick={() => runControl("Wi-Fi", () => api.setWifi(!(status.wifiEnabled === true)), "wifi")}
          >
            <IslandIcon name="wifi" size={19} />
            <span className="island-tile-label">Wi-Fi</span>
            <span className="island-tile-state">
              {status.wifi.supported ? (status.wifiEnabled == null ? "—" : status.wifiEnabled ? "On" : "Off") : "Can't change"}
            </span>
          </button>
        </div>
        <div className="island-control-tile">
          <button
            type="button"
            role="switch"
            aria-checked={status.bluetoothEnabled === true}
            aria-label="Bluetooth"
            className="island-tile-main"
            disabled={!status.bluetooth.supported || status.bluetoothEnabled == null || controlPending != null}
            title={status.bluetooth.supported ? "Toggle Bluetooth" : status.bluetooth.reason || "Bluetooth can't be changed here."}
            onClick={() => runControl("Bluetooth", () => api.setBluetooth(!(status.bluetoothEnabled === true)), "bluetooth")}
          >
            <IslandIcon name="bluetooth" size={19} />
            <span className="island-tile-label">Bluetooth</span>
            <span className="island-tile-state">
              {status.bluetooth.supported ? (status.bluetoothEnabled == null ? "—" : status.bluetoothEnabled ? "On" : "Off") : "Can't change"}
            </span>
          </button>
        </div>
      </div>

      {feedback}

      <button type="button" className="island-settings-link" onClick={() => openSettings("display")}>
        <IslandIcon name="controls" size={16} />
        Open Windows settings
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
  const unread = status.notificationUnread;
  return (
    <div className="island-card island-notifications-card">
      <header className="island-notif-head">
        <span className="island-icon-tile">
          <IslandIcon name="bell" size={23} />
        </span>
        <span className="island-notif-heading">
          <h2>Notifications</h2>
          <p>{unread > 0 ? unread + " unread" : "Recent activity"}</p>
        </span>
        {supported && status.notificationItems.length > 0 ? (
          <button
            type="button"
            className="island-clear-button"
            aria-label="Clear all notifications"
            title="Clear all notifications"
            onClick={() => {
              void api.notificationsClear().then(refresh).catch(() => undefined);
            }}
          >
            <IslandIcon name="dismiss" size={15} />
          </button>
        ) : null}
      </header>
      {!supported ? (
        <div className="island-unsupported" role="status">
          <span className="island-unsupported-icon">
            <IslandIcon name="bell" size={23} />
          </span>
          <strong>Notifications aren't available yet</strong>
          <span>{status.notifications.reason || "Notification history isn't wired in this build."}</span>
        </div>
      ) : status.notificationItems.length === 0 ? (
        <div className="island-empty island-notif-empty" role="status">
          <strong>All caught up.</strong>
        </div>
      ) : (
        <div className="island-notification-list island-scrollable" role="list" aria-label="Notifications">
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
                  void api.notificationsDismiss(item.id).then(refresh).catch(() => undefined);
                }}
              >
                <IslandIcon name="dismiss" size={15} />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
