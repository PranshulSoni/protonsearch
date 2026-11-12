// Agent event subscription. Tauri emits "agent:event" payloads (AgentEvent);
// the browser preview has no sidecar, so a window CustomEvent test seam is
// provided for visual-QA state screenshots. No invoke calls live here.

import { isTauriRuntime } from "../bridge/api";
import type { AgentEvent, AgentStatus } from "../bridge/types";

export const MOCK_AGENT_EVENT = "ps:agent-event";

export function subscribeAgentEvents(onEvent: (event: AgentEvent) => void): () => void {
  if (isTauriRuntime()) {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    void import("@tauri-apps/api/event")
      .then(({ listen }) =>
        listen<AgentEvent>("agent:event", (event) => onEvent(event.payload)).then((fn) => {
          if (disposed) fn();
          else unlisten = fn;
        }),
      )
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }

  const listener = (event: Event) => {
    const detail = (event as CustomEvent<AgentEvent>).detail;
    if (detail) onEvent(detail);
  };
  window.addEventListener(MOCK_AGENT_EVENT, listener);
  return () => window.removeEventListener(MOCK_AGENT_EVENT, listener);
}

/** Browser-preview-only status override so visual QA can capture idle/running states. */
export function readMockAgentStatus(): AgentStatus | null {
  if (isTauriRuntime()) return null;
  const value = (window as unknown as Record<string, unknown>).__psMockAgentStatus;
  if (!value || typeof value !== "object") return null;
  return value as AgentStatus;
}
