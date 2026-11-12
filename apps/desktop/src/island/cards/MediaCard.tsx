import { useEffect, useRef, useState } from "react";
import { AnimatePresence } from "motion/react";
import { ChevronRight, Disc3, Pause, Play, SkipBack, SkipForward, Speaker } from "lucide-react";
import { api } from "../../bridge/api";
import type { ControlCapability, ControlsStatus } from "../../bridge/types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { Slider } from "../ui/Slider";
import { CardEmpty } from "./CardChrome";
import { AudioOutputList, CardFrame, DrillHeader, Pane, activeOutput, iconButton } from "./controlsShared";

function formatTime(ms: number | null): string {
  if (ms == null || !Number.isFinite(ms)) return "—";
  const total = Math.max(0, Math.floor(ms / 1000));
  return Math.floor(total / 60) + ":" + String(total % 60).padStart(2, "0");
}

function Artwork({ src }: { src: string | null }) {
  return (
    <div className="size-20 shrink-0 overflow-hidden rounded-tile bg-raised">
      {src ? (
        <img src={src} alt="" className="size-full object-cover" />
      ) : (
        <div className="grid size-full place-items-center text-fg-3">
          <Disc3 size={28} strokeWidth={1.75} aria-hidden="true" />
        </div>
      )}
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
  const media = status?.media ?? null;
  const duration = media?.durationMs ?? null;
  const [outputsOpen, setOutputsOpen] = useState(false);
  // Play/pause shows the requested state at once; the next status read decides it again.
  const [intent, setIntent] = useState<boolean | null>(null);
  const intentId = useRef(0);
  const playing = intent ?? media?.playing === true;

  // The engine position is re-anchored only when the reported snapshot changes. A player that
  // publishes no timestamp can report the same position for a while, and the bar must keep running.
  const snapshot = media ? `${media.title}|${media.artist}|${media.playing}|${media.positionMs ?? ""}` : "";
  const [anchor, setAnchor] = useState(() => ({ key: snapshot, position: media?.positionMs ?? 0, readAt: Date.now() }));
  if (anchor.key !== snapshot) {
    setAnchor({ key: snapshot, position: media?.positionMs ?? 0, readAt: Date.now() });
  }
  const rebase = (at: number) => setAnchor({ key: snapshot, position: at, readAt: Date.now() });
  const [now, setNow] = useState(() => Date.now());
  // Seconds while the thumb is held; the seek is sent on release.
  const [scrub, setScrub] = useState<number | null>(null);
  const running = playing && duration != null;
  const position = Math.min(
    anchor.position + (running ? Math.max(0, now - anchor.readAt) : 0),
    duration ?? Infinity,
  );

  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(id);
  }, [running]);

  if (pending && !status) {
    return (
      <CardFrame icon={Disc3} title="Media">
        <p className="text-[13px] text-fg-3">Checking the current session.</p>
      </CardFrame>
    );
  }
  if (!status) {
    return (
      <CardFrame icon={Disc3} title="Media">
        <p className="text-[13px] text-fg-2">Controls are unavailable right now.</p>
        {error ? (
          <p role="alert" className="text-xs text-danger">
            {error}
          </p>
        ) : null}
        <button
          type="button"
          onClick={() => void refresh()}
          className="self-start rounded-full bg-raised px-3 py-1.5 text-xs text-fg hover:bg-hover"
        >
          Retry
        </button>
      </CardFrame>
    );
  }

  const capability: ControlCapability = status.mediaCapability;
  if (!capability.supported) {
    return (
      <CardFrame icon={Disc3} title="Media" aside="Unavailable">
        <p className="text-[13px] text-fg-2">{capability.reason || "Media session adapter isn't wired in this build."}</p>
      </CardFrame>
    );
  }
  if (!media) {
    return (
      <CardFrame icon={Disc3} title="Media">
        <CardEmpty title="Nothing playing" hint="Start playback in any app to see it here." />
      </CardFrame>
    );
  }

  const outputs = status.audioOutputs;
  const current = activeOutput(outputs, status.defaultAudioId);
  // Re-reads status after every action, whatever its outcome, so the card shows what the player or
  // Windows actually did. A newer play/pause supersedes the optimistic state of an older one.
  const send = (action: Promise<void>, playingAfter?: boolean) => {
    const id = playingAfter === undefined ? -1 : ++intentId.current;
    if (playingAfter !== undefined) setIntent(playingAfter);
    void action
      .then(() => refresh(), () => refresh())
      .catch(() => undefined)
      .finally(() => {
        if (intentId.current === id) setIntent(null);
      });
  };
  const command = (action: string) => send(api.mediaCommand(action));
  const togglePlay = () => {
    rebase(position);
    send(api.mediaCommand(playing ? "pause" : "play"), !playing);
  };
  const shown = scrub == null ? position : scrub * 1000;
  const commitSeek = () => {
    if (scrub == null) return;
    const target = Math.round(scrub * 1000);
    setScrub(null);
    rebase(target);
    send(api.mediaCommand("seek", target));
  };

  return (
    <CardFrame icon={Disc3} title="Media" aside={playing ? "Playing" : "Paused"}>
      <AnimatePresence mode="wait" initial={false}>
        {outputsOpen ? (
          <Pane key="outputs" onEscape={() => setOutputsOpen(false)}>
            <DrillHeader title="Audio output" onBack={() => setOutputsOpen(false)} />
            <ScrollArea className="min-h-0 flex-1">
              <AudioOutputList
                outputs={outputs}
                defaultId={status.defaultAudioId}
                onSelect={(deviceId) => {
                  setOutputsOpen(false);
                  send(api.setAudioOutput(deviceId));
                }}
              />
            </ScrollArea>
          </Pane>
        ) : (
          <Pane key="player">
            <ScrollArea className="min-h-0 flex-1">
              <div className="flex flex-col gap-3">
                <div className="flex gap-3">
                  <Artwork src={media.artworkDataUrl} />
                  <div className="flex min-w-0 flex-1 flex-col justify-center gap-0.5">
                    <p className="truncate text-[15px] font-semibold leading-5">{media.title}</p>
                    <p className="truncate text-[13px] text-fg-2">{media.artist}</p>
                    {media.album ? <p className="truncate text-xs text-fg-3">{media.album}</p> : null}
                  </div>
                </div>
                <div className="flex flex-col gap-1.5">
                  <div onPointerUp={commitSeek} onKeyUp={commitSeek}>
                    <Slider
                      label="Position"
                      value={duration == null ? 0 : Math.min(shown, duration) / 1000}
                      min={0}
                      max={(duration ?? 0) / 1000}
                      onChange={setScrub}
                      disabled={duration == null || !media.canSeek}
                    />
                  </div>
                  <div className="flex justify-between text-xs tabular-nums text-fg-3">
                    <span>{formatTime(shown)}</span>
                    <span>{duration == null ? "—" : "-" + formatTime(Math.max(0, duration - shown))}</span>
                  </div>
                </div>
                <div className="flex items-center justify-center gap-4">
                  <button
                    type="button"
                    aria-label="Previous track"
                    title="Previous track"
                    onClick={() => command("previous")}
                    className={cn(iconButton, "size-10")}
                  >
                    <SkipBack size={18} strokeWidth={1.75} aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    aria-label={playing ? "Pause" : "Play"}
                    title={playing ? "Pause" : "Play"}
                    onClick={togglePlay}
                    className="grid size-12 place-items-center rounded-full bg-fg text-surface hover:opacity-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hi"
                  >
                    {playing ? (
                      <Pause size={20} strokeWidth={1.75} aria-hidden="true" />
                    ) : (
                      <Play size={20} strokeWidth={1.75} aria-hidden="true" />
                    )}
                  </button>
                  <button
                    type="button"
                    aria-label="Next track"
                    title="Next track"
                    onClick={() => command("next")}
                    className={cn(iconButton, "size-10")}
                  >
                    <SkipForward size={18} strokeWidth={1.75} aria-hidden="true" />
                  </button>
                </div>
                <button
                  type="button"
                  disabled={outputs.length === 0}
                  onClick={() => setOutputsOpen(true)}
                  className="flex items-center gap-3 rounded-tile bg-raised px-3 py-2.5 text-left text-[13px] hover:bg-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hi disabled:opacity-50"
                >
                  <Speaker size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-2" />
                  <span className="min-w-0 flex-1 truncate">{current?.name ?? "No audio output"}</span>
                  <ChevronRight size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-3" />
                </button>
              </div>
            </ScrollArea>
          </Pane>
        )}
      </AnimatePresence>
    </CardFrame>
  );
}
