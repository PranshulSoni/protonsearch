import { useEffect, useState } from "react";
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

export function useIslandControls(): IslandControlsState {
  const [status, setStatus] = useState<ControlsStatus | null>(null);
  const [pending, setPending] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setPending(true);
    void api
      .getControlsStatus()
      .then((next) => {
        if (cancelled) return;
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
  }, [nonce]);

  useEffect(() => {
    const refresh = () => setNonce((value) => value + 1);
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, []);

  return {
    status,
    pending,
    error,
    refresh: () => setNonce((value) => value + 1),
  };
}
