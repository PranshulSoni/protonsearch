import { api, isTauriRuntime } from "../bridge/api";

let enabledPromise: Promise<boolean> | null = null;
export function perfEnabled(): Promise<boolean> {
  enabledPromise ??= isTauriRuntime()
    ? api.appInfo().then((info) => info.perfHooks === true).catch(() => false)
    : Promise.resolve(false);
  return enabledPromise;
}

export interface FrameStats {
  n: number;
  p50: number;
  p95: number;
  p99: number;
  max: number;
  drop: number;
  long: number;
}

export function frameStats(intervals: number[]): FrameStats {
  const sorted = [...intervals].sort((a, b) => a - b);
  const at = (p: number) =>
    sorted.length ? sorted[Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1)] : 0;
  const p50 = at(50);
  return {
    n: intervals.length,
    p50,
    p95: at(95),
    p99: at(99),
    max: sorted.length ? sorted[sorted.length - 1] : 0,
    drop: intervals.filter((v) => v > p50 * 1.5).length, // > 1.5x the median frame time
    long: intervals.filter((v) => v > 50).length,
  };
}

// Labels are written raw into a JSON line by the shell: never emit quotes or backslashes.
export function formatFrameMark(label: string, s: FrameStats): string {
  const f = (v: number) => v.toFixed(1);
  return `${label}|n=${s.n}|p50=${f(s.p50)}|p95=${f(s.p95)}|p99=${f(s.p99)}|max=${f(s.max)}|drop=${s.drop}|long=${s.long}`;
}

/** Sample requestAnimationFrame intervals for `durationMs`, then write one perf mark. */
export function recordFrames(label: string, durationMs: number): void {
  void perfEnabled().then((on) => {
    if (!on) return;
    const intervals: number[] = [];
    const t0 = performance.now();
    let last = t0;
    const tick = (now: number) => {
      intervals.push(now - last);
      last = now;
      if (now - t0 < durationMs) requestAnimationFrame(tick);
      else void api.perfMark(formatFrameMark(label, frameStats(intervals.slice(1))), Date.now()).catch(() => undefined);
    };
    requestAnimationFrame(tick);
  });
}

export function markTyping(ms: number): void {
  void perfEnabled().then((on) => {
    if (on) requestAnimationFrame(() => void api.perfMark(`island-typing|ms=${ms.toFixed(1)}`, Date.now()).catch(() => undefined));
  });
}
