// Shared, accessible UI primitives for the desktop app and onboarding.
// Real SVG icons (never glyph placeholders), roving-tabindex radiogroups,
// switch semantics, status dots paired with words, and honest empty states.

import { useEffect, useRef, useState, type ReactNode } from "react";
import "./ui.css";

export type IconName =
  | "home"
  | "search"
  | "activity"
  | "workflows"
  | "preferences"
  | "arrowRight"
  | "arrowUpRight"
  | "check"
  | "alert"
  | "plus"
  | "close"
  | "person"
  | "folder"
  | "lock"
  | "moon"
  | "power"
  | "restart"
  | "trash"
  | "settings"
  | "terminal"
  | "grid"
  | "browser"
  | "clipboard"
  | "git"
  | "pause"
  | "play"
  | "shield";

const PATHS: Record<IconName, ReactNode> = {
  home: <path d="M4 10.5 12 4l8 6.5V20a1 1 0 0 1-1 1h-5v-6h-4v6H5a1 1 0 0 1-1-1z" />,
  search: (
    <>
      <circle cx="11" cy="11" r="6.25" />
      <path d="m16 16 4.25 4.25" />
    </>
  ),
  activity: (
    <>
      <circle cx="12" cy="12" r="8.25" />
      <path d="M12 7.75V12l2.75 1.75" />
    </>
  ),
  workflows: (
    <>
      <path d="M5 9a7 7 0 0 1 12.5-3.5L20 8" />
      <path d="M20 4v4h-4" />
      <path d="M19 15a7 7 0 0 1-12.5 3.5L4 16" />
      <path d="M4 20v-4h4" />
    </>
  ),
  preferences: (
    <>
      <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
      <circle cx="15" cy="7" r="2" />
      <circle cx="9" cy="17" r="2" />
    </>
  ),
  arrowRight: <path d="M4 12h15M13 6l6 6-6 6" />,
  arrowUpRight: <path d="M7 17 17 7M8 7h9v9" />,
  check: <path d="m5 13 4 4L19 7" />,
  alert: (
    <>
      <path d="M12 4 21 20H3z" />
      <path d="M12 10v4M12 17.5v.01" />
    </>
  ),
  plus: <path d="M12 5v14M5 12h14" />,
  close: <path d="m6 6 12 12M18 6 6 18" />,
  person: (
    <>
      <circle cx="12" cy="8.5" r="3.5" />
      <path d="M5 20a7 7 0 0 1 14 0" />
    </>
  ),
  folder: <path d="M4 7a1 1 0 0 1 1-1h4l2 2h8a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z" />,
  lock: (
    <>
      <rect x="5" y="10" width="14" height="10" rx="1.5" />
      <path d="M8 10V8a4 4 0 0 1 8 0v2" />
    </>
  ),
  moon: <path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5Z" />,
  power: (
    <>
      <path d="M12 3v9" />
      <path d="M6.5 6.5a7.5 7.5 0 1 0 11 0" />
    </>
  ),
  restart: (
    <>
      <path d="M4 12a8 8 0 0 1 13.6-5.6L20 8" />
      <path d="M20 4v4h-4" />
      <path d="M20 12a8 8 0 0 1-13.6 5.6L4 16" />
      <path d="M4 20v-4h4" />
    </>
  ),
  trash: (
    <>
      <path d="M4 7h16M9 7V5h6v2M6 7l1 13h10l1-13" />
      <path d="M10 11v6M14 11v6" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M18.4 5.6 17 7M7 17l-1.4 1.4" />
    </>
  ),
  terminal: (
    <>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
      <path d="m7.5 9 3 3-3 3M13 15h4" />
    </>
  ),
  grid: (
    <>
      <rect x="4" y="4" width="7" height="7" rx="1" />
      <rect x="13" y="4" width="7" height="7" rx="1" />
      <rect x="4" y="13" width="7" height="7" rx="1" />
      <rect x="13" y="13" width="7" height="7" rx="1" />
    </>
  ),
  browser: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17M12 3.5c2.5 2.4 3.8 5.2 3.8 8.5s-1.3 6.1-3.8 8.5c-2.5-2.4-3.8-5.2-3.8-8.5S9.5 5.9 12 3.5Z" />
    </>
  ),
  clipboard: (
    <>
      <rect x="5" y="5" width="14" height="15" rx="2" />
      <path d="M9 5V3.5h6V5M8.5 10h7M8.5 14h7" />
    </>
  ),
  git: (
    <>
      <circle cx="6" cy="6" r="2" />
      <circle cx="6" cy="18" r="2" />
      <circle cx="17" cy="9" r="2" />
      <path d="M6 8v8M8 6h4a5 5 0 0 1 5 5" />
    </>
  ),
  pause: <path d="M9 5v14M15 5v14" />,
  play: <path d="M8 5.5v13l10-6.5z" />,
  shield: <path d="M12 3.5 19 6v5.5c0 4.2-2.9 7.4-7 9-4.1-1.6-7-4.8-7-9V6z" />,
};

export function Icon({
  name,
  size = 16,
  className,
}: {
  name: IconName;
  size?: number;
  className?: string;
}) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {PATHS[name]}
    </svg>
  );
}

export function Dot({ tone }: { tone: "ok" | "busy" | "warn" | "error" | "off" }) {
  return <span className={"dot tone-" + tone} aria-hidden="true" />;
}

export function Panel({
  id,
  title,
  hint,
  actions,
  children,
  className,
}: {
  id: string;
  title: string;
  hint?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={"panel" + (className ? " " + className : "")} aria-labelledby={id}>
      <div className="panel-head">
        <h2 id={id}>{title}</h2>
        {actions}
      </div>
      {hint ? <p className="panel-hint">{hint}</p> : null}
      {children}
    </section>
  );
}

export function Switch({
  checked,
  onChange,
  label,
  disabled,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={"switch" + (checked ? " is-on" : "")}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span className="switch-thumb" aria-hidden="true" />
    </button>
  );
}

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
}

/**
 * Segmented control with radiogroup semantics: arrow keys change the value
 * immediately (and move focus), Home/End jump to the ends.
 */
export function SegmentedControl<T extends string>({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string;
  value: T;
  options: SegmentedOption<T>[];
  onChange: (next: T) => void;
  disabled?: boolean;
}) {
  const refs = useRef<Array<HTMLButtonElement | null>>([]);
  const index = Math.max(
    0,
    options.findIndex((option) => option.value === value),
  );

  const select = (nextIndex: number) => {
    const next = options[(nextIndex + options.length) % options.length];
    onChange(next.value);
    refs.current[(nextIndex + options.length) % options.length]?.focus();
  };

  return (
    <div
      className="segmented"
      role="radiogroup"
      aria-label={label}
      onKeyDown={(event) => {
        if (disabled) return;
        if (event.key === "ArrowRight" || event.key === "ArrowDown") {
          event.preventDefault();
          select(index + 1);
        } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
          event.preventDefault();
          select(index - 1);
        } else if (event.key === "Home") {
          event.preventDefault();
          select(0);
        } else if (event.key === "End") {
          event.preventDefault();
          select(options.length - 1);
        }
      }}
    >
      {options.map((option, optionIndex) => {
        const checked = option.value === value;
        return (
          <button
            key={option.value}
            ref={(element) => {
              refs.current[optionIndex] = element;
            }}
            type="button"
            role="radio"
            aria-checked={checked}
            tabIndex={checked ? 0 : -1}
            disabled={disabled}
            className="seg"
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

/** Filter-style segmented buttons (role=group + aria-pressed, never mixed with radios). */
export function FilterBar<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: SegmentedOption<T>[];
  onChange: (next: T) => void;
}) {
  return (
    <div className="filter-bar" role="group" aria-label={label}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          className="filter-btn"
          aria-pressed={option.value === value}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

export function Callout({
  tone = "note",
  children,
  role,
}: {
  tone?: "note" | "warn" | "danger" | "ok";
  children: ReactNode;
  role?: "alert" | "status";
}) {
  return (
    <div className={"callout callout-" + tone} role={role ?? (tone === "danger" ? "alert" : undefined)}>
      {children}
    </div>
  );
}

export function EmptyState({
  title,
  body,
  action,
}: {
  title: string;
  body?: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty-block">
      <p className="empty-title">{title}</p>
      {body ? <p className="empty-body">{body}</p> : null}
      {action ? <div className="empty-action">{action}</div> : null}
    </div>
  );
}

/** Route heading that receives focus on mount (SCREENS §3.1 focus contract). */
export function RouteHeading({
  eyebrow,
  children,
}: {
  eyebrow: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, []);
  return (
    <header className="page-head">
      <p className="eyebrow">{eyebrow}</p>
      <h1 ref={ref} tabIndex={-1}>
        {children}
      </h1>
    </header>
  );
}

export function SavedStamp({ show, label = "Saved" }: { show: boolean; label?: string }) {
  return (
    <span className="saved-stamp" role="status" aria-live="polite">
      {show ? label : ""}
    </span>
  );
}

/** Small state hook for the 2 s "Saved" stamp next to a section title. */
export function useSavedStamp(resetMs = 2000): {
  savedKey: string | null;
  markSaved: (key: string) => void;
} {
  const [savedKey, setSavedKey] = useState<string | null>(null);
  const timer = useRef<number | null>(null);
  useEffect(() => {
    return () => {
      if (timer.current !== null) window.clearTimeout(timer.current);
    };
  }, []);
  const markSaved = (key: string) => {
    setSavedKey(key);
    if (timer.current !== null) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setSavedKey(null), resetMs);
  };
  return { savedKey, markSaved };
}
