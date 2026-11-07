import { useSyncExternalStore } from 'react';
import { isNative, native } from '../../services/native';
import type { ProviderId, Snapshot } from './types';
const initial: Snapshot = { status: 'loading', updatedAt: 0 };
type Entry = { snapshot: Snapshot; listeners: Set<() => void>; interval: number; next: number; pending: boolean };
const entries = new Map<ProviderId, Entry>();
let timer: ReturnType<typeof setInterval> | undefined;
function entry(id: ProviderId, interval: number) {
  let value = entries.get(id);
  if (!value) { value = { snapshot: initial, listeners: new Set(), interval: Math.max(1000, interval), next: 0, pending: false }; entries.set(id, value); }
  value.interval = Math.min(value.interval, Math.max(1000, interval));
  return value;
}
async function refresh(id: ProviderId, value: Entry) {
  if (value.pending || !value.listeners.size || document.hidden || Date.now() < value.next) return;
  value.pending = true;
  try { value.snapshot = isNative() ? await native.snapshot(id) : { status: 'disconnected', updatedAt: Date.now(), message: 'Open the Windows app' }; }
  catch (error) { value.snapshot = { ...value.snapshot, status: value.snapshot.data ? 'stale' : 'error', message: String(error) }; }
  finally { value.pending = false; value.next = Date.now() + value.interval; value.listeners.forEach(listener => listener()); }
}
function tick() { entries.forEach((value, id) => { void refresh(id, value); }); }
export function subscribeProvider(id: ProviderId, interval: number, listener: () => void) {
  const value = entry(id, interval); value.listeners.add(listener);
  if (!timer) { timer = setInterval(tick, 1000); document.addEventListener('visibilitychange', tick); }
  void refresh(id, value);
  return () => {
    value.listeners.delete(listener);
    if (![...entries.values()].some(item => item.listeners.size)) { clearInterval(timer); timer = undefined; document.removeEventListener('visibilitychange', tick); }
  };
}
export function providerSnapshot(id: ProviderId) { return entries.get(id)?.snapshot ?? initial; }
export function refreshProvider(id: ProviderId) {
  const value = entries.get(id);
  if (value) { value.next = 0; void refresh(id, value); }
}
export function useProvider(id: ProviderId, interval = 10000): Snapshot {
  return useSyncExternalStore(listener => id === 'none' ? () => {} : subscribeProvider(id, interval, listener), () => id === 'none' ? initial : providerSnapshot(id));
}
