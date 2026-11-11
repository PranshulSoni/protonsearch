import { useEffect, useRef, useState } from "react";
import { api } from "../bridge/api";
import type { PreviewPayload, SearchResult } from "../bridge/types";

/**
 * Preview client seam. The panel only consumes the engine's PreviewPayload;
 * requests are debounced, cached by result id, and guarded by a request token so
 * a late response can never replace the currently selected row's preview.
 */

const DEFAULT_MAX_CHARS = 12_000;
const DEBOUNCE_MS = 120;
const REQUEST_TIMEOUT_MS = 8_000;
const MAX_CACHE_ENTRIES = 32;

export type PreviewLoader = (resultId: string, maxChars: number) => Promise<PreviewPayload>;

let loader: PreviewLoader = (resultId, maxChars) => api.getPreview(resultId, maxChars);
const cache = new Map<string, PreviewPayload>();
const inFlight = new Map<string, Promise<PreviewPayload>>();

function cacheKey(resultId: string, maxChars: number): string {
  return resultId + "::" + maxChars;
}

function loadPreview(key: string, resultId: string, maxChars: number): Promise<PreviewPayload> {
  const pending = inFlight.get(key);
  if (pending) return pending;
  const request = loader(resultId, maxChars);
  inFlight.set(key, request);
  void request.then(
    () => {
      if (inFlight.get(key) === request) inFlight.delete(key);
    },
    () => {
      if (inFlight.get(key) === request) inFlight.delete(key);
    },
  );
  return request;
}

/** Test / Gate-wiring seam. Pass null to restore the bridge loader. */
export function setPreviewLoader(next: PreviewLoader | null): void {
  loader = next ?? ((resultId, maxChars) => api.getPreview(resultId, maxChars));
}

export function primePreviewCache(resultId: string, payload: PreviewPayload, maxChars = DEFAULT_MAX_CHARS): void {
  storePreview(cacheKey(resultId, maxChars), payload);
}

function storePreview(key: string, payload: PreviewPayload): void {
  cache.delete(key);
  cache.set(key, payload);
  if (cache.size > MAX_CACHE_ENTRIES) {
    const oldest = cache.keys().next().value;
    if (oldest !== undefined) cache.delete(oldest);
  }
}

export function clearPreviewCache(): void {
  cache.clear();
}

export interface LauncherPreviewState {
  status: "idle" | "loading" | "ready" | "unsupported" | "error";
  payload: PreviewPayload | null;
  error: string | null;
  reload: () => void;
}

function statusFor(payload: PreviewPayload): LauncherPreviewState["status"] {
  if (payload.status === "unsupported") return "unsupported";
  if (payload.status === "error") return "error";
  return "ready";
}

export function useLauncherPreview(
  result: SearchResult | null,
  enabled: boolean,
  maxChars = DEFAULT_MAX_CHARS,
): LauncherPreviewState {
  const [nonce, setNonce] = useState(0);
  const [state, setState] = useState<Omit<LauncherPreviewState, "reload">>({
    status: enabled && result ? "loading" : "idle",
    payload: null,
    error: null,
  });
  const tokenRef = useRef(0);

  useEffect(() => {
    if (!enabled || !result) {
      setState({ status: "idle", payload: null, error: null });
      return;
    }
    const key = cacheKey(result.id, maxChars);
    const cached = cache.get(key);
    if (cached) {
    cache.delete(key);
    storePreview(key, cached);
      setState({
        status: statusFor(cached),
        payload: cached,
        error: cached.error,
      });
      return;
    }
    const token = ++tokenRef.current;
    setState({ status: "loading", payload: null, error: null });
    let requestTimeout: number | undefined;
    const timer = window.setTimeout(() => {
      requestTimeout = window.setTimeout(() => {
        if (token !== tokenRef.current) return;
        tokenRef.current += 1;
        setState({ status: "error", payload: null, error: "Preview request timed out." });
      }, REQUEST_TIMEOUT_MS);
      loadPreview(key, result.id, maxChars)
        .then((payload) => {
          if (token !== tokenRef.current) return;
        storePreview(key, payload);
          setState({
            status: statusFor(payload),
            payload,
            error: payload.error,
          });
        })
        .catch((error: unknown) => {
          if (token !== tokenRef.current) return;
          setState({
            status: "error",
            payload: null,
            error:
              error instanceof Error
                ? error.message
                : typeof error === "string" && error.trim()
                  ? error
                  : "Preview is unavailable right now.",
          });
        })
        .finally(() => {
          if (requestTimeout !== undefined) window.clearTimeout(requestTimeout);
        });
    }, DEBOUNCE_MS);
    return () => {
      window.clearTimeout(timer);
      if (requestTimeout !== undefined) window.clearTimeout(requestTimeout);
      tokenRef.current += 1;
      cache.delete(key);
    };
  }, [enabled, result, maxChars, nonce]);

  const key = enabled && result ? cacheKey(result.id, maxChars) : null;
  const cached = key ? cache.get(key) : undefined;
  const visible = cached
    ? { status: statusFor(cached), payload: cached, error: cached.error }
    : state;

  return {
    ...visible,
    reload: () => {
      if (key) cache.delete(key);
      setState({ status: "loading", payload: null, error: null });
      setNonce((value) => value + 1);
    },
  };
}
