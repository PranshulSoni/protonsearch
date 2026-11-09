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
  useEffect(() => {
    setVolume(status?.volumePercent ?? 0);
    setBrightness(status?.brightnessPercent ?? 0);
  }, [status?.volumePercent, status?.brightnessPercent]);

  if (pending && !status) return <CardLoading card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" />;
  if (error && !status) {
    return <CardError card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" error={error} refresh={refresh} />;
  }
  if (!status) {
    return <UnsupportedCard card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" reason="Controls are unavailable right now." />;
  }
  const wifiEnabled = status.wifiEnabled;
  const bluetoothEnabled = status.bluetoothEnabled;
  return (
    <div className="island-card island-controls-card">
      <CardHead card="controls" title="Quick Settings" subtitle="Your PC, one gesture away" />
      <div className="island-divider" />
      <div className="island-setting-list">
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
            onPointerUp={() => {
              void api.setVolume(volume).then(refresh).catch(() => undefined);
            }}
            onKeyUp={() => {
              void api.setVolume(volume).then(refresh).catch(() => undefined);
            }}
          />
          <span className="island-setting-value">{status.volumePercent == null ? "—" : volume + "%"}</span>
          <button
            type="button"
            className="island-setting-toggle"
            aria-pressed={status.muted === true}
            disabled={status.muted == null}
            title="Mute or unmute"
            onClick={() => {
              void api.setMuted(!(status.muted === true)).then(refresh).catch(() => undefined);
            }}
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
              onPointerUp={() => {
                void api.setBrightness(brightness).then(refresh).catch(() => undefined);
              }}
              onKeyUp={() => {
                void api.setBrightness(brightness).then(refresh).catch(() => undefined);
              }}
            />
            <span className="island-setting-value">
              {status.brightnessPercent == null ? "—" : brightness + "%"}
            </span>
          </div>
        ) : (
          <CapabilityRow
            icon="brightness"
            label="Brightness"
            capability={status.brightnessCapability}
          />
        )}
      </div>
      <div className="island-control-grid">
        <CapabilityTile
          icon="wifi"
          label="Wi-Fi"
          capability={status.wifi}
          value={wifiEnabled == null ? null : wifiEnabled ? "On" : "Off"}
        />
        <CapabilityTile
          icon="bluetooth"
          label="Bluetooth"
          capability={status.bluetooth}
          value={bluetoothEnabled == null ? null : bluetoothEnabled ? "On" : "Off"}
        />
        <CapabilityTile icon="airplane" label="Airplane mode" capability={status.airplaneMode} />
        <CapabilityTile icon="bolt" label="Energy saver" capability={status.energySaver} />
      </div>
    </div>
  );
}

function CapabilityRow({
  icon,
  label,
  capability,
  value,
}: {
  icon: "wifi" | "bluetooth" | "airplane" | "brightness" | "bolt";
  label: string;
  capability: ControlCapability;
  value?: string | null;
}) {
  return (
    <div className="island-control-row">
      <IslandIcon name={icon} size={19} />
      <span className="island-control-label">{label}</span>
      <span className="island-control-value">
        {capability.supported ? value ?? "Available" : "Unsupported"}
      </span>
      <button
        type="button"
        className="island-agent-button"
        disabled
        title={
          capability.supported
            ? "This toggle isn't exposed by the controls API yet."
            : capability.reason || "Not supported by Windows in this build."
        }
      >
        {capability.supported ? "Toggle" : "Unavailable"}
      </button>
    </div>
  );
}

export function CapabilityTile({
  icon,
  label,
  capability,
  value,
}: {
  icon: "wifi" | "bluetooth" | "airplane" | "bolt";
  label: string;
  capability: ControlCapability;
  value?: string | null;
}) {
  return (
    <div
      className="island-control-tile"
      aria-disabled="true"
      title={
        capability.supported
          ? "This toggle isn't exposed by the controls API yet."
          : capability.reason || "Not supported by Windows in this build."
      }
    >
      <IslandIcon name={icon} size={19} />
      <span className="island-control-tile-body">
        <span className="island-control-tile-label">{label}</span>
        <span className="island-control-tile-value">
          {capability.supported ? value ?? "Available" : "Unsupported"}
        </span>
      </span>
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
          {import.meta.env.DEV && status.toastCapability.supported ? (
            <button
              type="button"
              className="island-agent-button"
              onClick={() => {
                void api
                  .showLocalToast("ProtonSearch island", "Local notification test")
                  .then(refresh)
                  .catch(() => undefined);
              }}
            >
              Test local toast
            </button>
          ) : null}
        </div>
      ) : status.notificationItems.length === 0 ? (
        <EmptyState title="All caught up" hint="No recent notifications." />
      ) : (
        <div className="island-notification-list">
          {status.notificationItems.map((item) => (
            <div className="island-notification-row" key={item.id}>
              <IslandIcon name="bell" size={18} />
              <span className="island-notification-body">
                <span className="island-notification-title">{item.title}</span>
                <span className="island-notification-text">{item.body}</span>
                <span className="island-notification-source">{item.source}</span>
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
        {import.meta.env.DEV && status.toastCapability.supported ? (
          <button
            type="button"
            className="island-agent-button"
            title="Store a local notification and attempt a Windows toast"
            onClick={() => {
              void api
                .showLocalToast("ProtonSearch island", "Local notification test")
                .then(refresh)
                .catch(() => undefined);
            }}
          >
            Test local toast
          </button>
        ) : null}
      </div>
    </div>
  );
}
