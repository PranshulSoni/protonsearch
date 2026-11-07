import type { ReactNode } from 'react';

const RING_CIRCUMFERENCE = 2 * Math.PI * 15;

export function Ring({
  value,
  warnAt,
  children,
}: {
  value: number | null | undefined;
  warnAt?: number;
  children?: ReactNode;
}) {
  const safe = value == null || !Number.isFinite(value) ? null : Math.min(100, Math.max(0, value));
  const warn = safe != null && warnAt != null && safe >= warnAt;
  return (
    <span className={`dock-ring${warn ? ' dock-ring--warn' : ''}`}>
      <svg width="34" height="34" viewBox="0 0 34 34" aria-hidden="true" focusable="false">
        <circle className="dock-ring-track" cx="17" cy="17" r="15" />
        {safe == null ? null : (
          <circle
            className="dock-ring-fill"
            cx="17"
            cy="17"
            r="15"
            strokeDasharray={`${(RING_CIRCUMFERENCE * safe) / 100} ${RING_CIRCUMFERENCE}`}
            transform="rotate(-90 17 17)"
          />
        )}
      </svg>
      <span className="dock-ring-center">{children}</span>
    </span>
  );
}

export function ClockFace({ now, seconds = false }: { now: Date | number; seconds?: boolean }) {
  const d = new Date(now);
  const s = d.getSeconds();
  const m = d.getMinutes() + s / 60;
  const h = d.getHours() % 12 + m / 60;
  return (
    <svg className="dock-clock" width="36" height="36" viewBox="0 0 36 36">
      <circle className="dock-clock-face" cx="18" cy="18" r="16.5" />
      {[0, 90, 180, 270].map((angle) => (
        <line
          key={angle}
          className="dock-clock-tick"
          x1="18"
          y1="3.5"
          x2="18"
          y2="6"
          transform={`rotate(${angle} 18 18)`}
        />
      ))}
      <line className="dock-clock-hour" x1="18" y1="18" x2="18" y2="10" transform={`rotate(${h * 30} 18 18)`} />
      <line className="dock-clock-minute" x1="18" y1="18" x2="18" y2="6.5" transform={`rotate(${m * 6} 18 18)`} />
      {seconds ? (
        <line className="dock-clock-second" x1="18" y1="20" x2="18" y2="5.5" transform={`rotate(${s * 6} 18 18)`} />
      ) : null}
      <circle className="dock-clock-pin" cx="18" cy="18" r="1.4" />
    </svg>
  );
}
