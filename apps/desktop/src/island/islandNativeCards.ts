import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../bridge/api";
import type { ControlsStatus } from "../bridge/types";

/** Native-card data seam for Media / Quick Settings / Notifications. */
export interface IslandControlsState {
  status: ControlsStatus | null;
  pending: boolean;
  error: string | null;
  refresh: () => void | Promise<void>;
}

/**
 * `liveMedia`: while true (the Media card is on screen) the now-playing session is
 * re-read every second, so track changes and the progress bar stay current.
 */
export function useIslandControls(enabled: boolean, liveMedia = false): IslandControlsState {
  const [status, setStatus] = useState<ControlsStatus | null>(null);
  const [pending, setPending] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const refreshRef = useRef<() => void | Promise<void>>(() => undefined);
  const loadedRef = useRef(false);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    let request: Promise<void> | null = null;
    let queued = false;
    let queuedWaiters: Array<{ resolve: () => void; reject: (reason: unknown) => void }> = [];
    const load = (): Promise<void> => {
      if (request) {
        queued = true;
        return new Promise((resolve, reject) => queuedWaiters.push({ resolve, reject }));
      }
      if (!loadedRef.current) setPending(true);
      request = api.getControlsStatus()
        .then((next) => {
          if (cancelled) return;
          loadedRef.current = true;
          setStatus(next);
          setError(null);
        })
        .catch((reason: unknown) => {
          if (!cancelled) setError(reason instanceof Error ? reason.message : "Controls are unavailable right now.");
          throw reason;
        })
        .finally(() => {
          request = null;
          if (!cancelled) setPending(false);
          if (queued && !cancelled) {
            queued = false;
            const waiters = queuedWaiters;
            queuedWaiters = [];
            void load().then(
              () => waiters.forEach(({ resolve }) => resolve()),
              (reason) => waiters.forEach(({ reject }) => reject(reason)),
            );
          }
        });
      return request;
    };
    const refreshOnFocus = () => { void load().catch(() => undefined); };
    refreshRef.current = load;
    void load().catch(() => undefined);
    window.addEventListener("focus", refreshOnFocus);
    return () => {
      cancelled = true;
      window.removeEventListener("focus", refreshOnFocus);
      refreshRef.current = () => undefined;
      queuedWaiters.forEach(({ resolve }) => resolve());
      queuedWaiters = [];
    };
  }, [enabled]);

  useEffect(() => {
    if (!enabled || !liveMedia) return;
    const timer = window.setInterval(() => {
      void api
        .getMediaSession()
        .then((media) => setStatus((current) => (current ? { ...current, media } : current)))
        .catch(() => undefined);
    }, 1000);
    return () => window.clearInterval(timer);
  }, [enabled, liveMedia]);

  const refresh = useCallback(() => refreshRef.current(), []);
  return { status, pending, error, refresh };
}
