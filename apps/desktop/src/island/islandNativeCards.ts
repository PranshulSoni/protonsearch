import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../bridge/api";
import type { ControlsStatus } from "../bridge/types";

/**
 * Native-card data seam for Media / Quick Settings / Notifications.
 *
 * The shell controls bridge is now landed (get_controls_status + setters). This
 * hook is the single adapter point for the island cards; capability-disabled
 * sections stay honest and never render fabricated devices, media or counts.
 */

export interface IslandControlsState {
  status: ControlsStatus | null;
  pending: boolean;
  error: string | null;
  refresh: () => void;
}

export function useIslandControls(enabled: boolean): IslandControlsState {
  const [status, setStatus] = useState<ControlsStatus | null>(null);
  const [pending, setPending] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [nonce, setNonce] = useState(0);
  const loadedRef = useRef(false);

  // The Wi-Fi/Bluetooth/brightness probes are heavy: run only while a native card is shown.
  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    // A revisit keeps the loaded status on screen instead of flashing the loading state.
    if (!loadedRef.current) setPending(true);
    void api
      .getControlsStatus()
      .then((next) => {
        if (cancelled) return;
        loadedRef.current = true;
        setStatus(next);
        setError(null);
        setPending(false);
      })
      .catch((reason: unknown) => {
        if (cancelled) return;
        setError(reason instanceof Error ? reason.message : "Controls are unavailable right now.");
        setPending(false);
      });
    return () => {
      cancelled = true;
    };
  }, [enabled, nonce]);

  // No focus-refresh: controls are probed once (prefetched) and refreshed only
  // after explicit setters or card retry, avoiding open-path IPC work.

  const refresh = useCallback(() => setNonce((value) => value + 1), []);

  return { status, pending, error, refresh };
}
