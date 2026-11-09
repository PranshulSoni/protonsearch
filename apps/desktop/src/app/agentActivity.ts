// Live "Pi is running" signal shared by the Pi panel and the Agents list
// (SCREENS §3.5 availability badge). In-memory only; no bridge/IPC state.

export interface AgentActivity {
  active: boolean;
  runtime: string;
}

type Listener = () => void;
const listeners = new Set<Listener>();
let current: AgentActivity = { active: false, runtime: "Pi" };

export function getAgentActivity(): AgentActivity {
  return current;
}

export function setAgentActivity(active: boolean, runtime = "Pi"): void {
  if (current.active === active && current.runtime === runtime) return;
  current = { active, runtime };
  for (const listener of listeners) listener();
}

export function subscribeAgentActivity(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
