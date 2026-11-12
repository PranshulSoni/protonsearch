import { memo, useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Globe, History, Pin, Search, X } from "lucide-react";
import type { SearchResult } from "../../bridge/types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { AnimatedTabs, type AnimatedTab } from "../ui/AnimatedTabs";
import { HighlightRow } from "../ui/HighlightList";
import { relativeTime } from "../islandData";
import { CardEmpty, CardHeader } from "./CardChrome";

type BrowserTab = "all" | "today" | "pinned";

const TABS: ReadonlyArray<AnimatedTab<BrowserTab>> = [
  { id: "all", label: "All" },
  { id: "today", label: "Today" },
  { id: "pinned", label: "Pinned" },
];

const PINS_KEY = "ps.island.browser.pins";
const HIDDEN_KEY = "ps.island.browser.hidden";
/** Above this many page rows the list is virtualized; a plain list is cheaper below it. */
const VIRTUAL_ROWS = 40;
const HEADER_PX = 28;
const ROW_PX = 56;
const DAY_MS = 24 * 60 * 60 * 1000;
const UNKNOWN_BROWSER = "Browser";

type Entry =
  | { key: string; type: "header"; label: string; count: number }
  | { key: string; type: "row"; result: SearchResult };

function isPage(result: SearchResult): boolean {
  return result.kind === "browser_history" || result.kind === "bookmark";
}

function readIds(key: string): Set<string> {
  try {
    const parsed: unknown = JSON.parse(window.localStorage.getItem(key) ?? "[]");
    return new Set(Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === "string") : []);
  } catch {
    return new Set();
  }
}

function saveIds(key: string, ids: Set<string>): void {
  try {
    window.localStorage.setItem(key, JSON.stringify([...ids]));
  } catch {
    // Storage blocked or full: the choice still holds for this session.
  }
}

function withId(set: Set<string>, id: string, on: boolean): Set<string> {
  const next = new Set(set);
  if (on) next.add(id);
  else next.delete(id);
  return next;
}

function dayKey(ms: number): string {
  const date = new Date(ms);
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
}

/** "Chrome · 3 visits" -> "Chrome". Labels without a source use the generic name. */
function browserName(result: SearchResult): string {
  const label = result.meta?.label ?? "";
  return label.includes(" · ") ? (label.split(" · ")[0] ?? UNKNOWN_BROWSER) : UNKNOWN_BROWSER;
}

function hostOf(result: SearchResult): string {
  const raw = result.path ?? result.subtitle;
  try {
    return new URL(raw).hostname.replace(/^www\./, "") || raw;
  } catch {
    return raw;
  }
}

function dayLabel(key: string, ms: number, today: string, yesterday: string): string {
  if (key === today) return "Today";
  if (key === yesterday) return "Yesterday";
  if (key === "earlier") return "Earlier";
  return new Date(ms).toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });
}

function buildEntries(items: SearchResult[], tab: BrowserTab, query: string, pins: Set<string>): Entry[] {
  const now = Date.now();
  const today = dayKey(now);
  const yesterday = dayKey(now - DAY_MS);
  const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const matching = items
    .filter((result) => tab !== "pinned" || pins.has(result.id))
    .filter((result) => {
      const ms = result.meta?.modified;
      return tab !== "today" || (ms != null && dayKey(ms) === today);
    })
    .filter((result) => {
      const haystack = `${result.title} ${result.path ?? result.subtitle}`.toLowerCase();
      return terms.every((term) => haystack.includes(term));
    })
    .sort((a, b) => (b.meta?.modified ?? 0) - (a.meta?.modified ?? 0));

  const groups: Array<{ key: string; ms: number; items: SearchResult[] }> = [];
  for (const result of matching) {
    const ms = result.meta?.modified ?? 0;
    const key = ms ? dayKey(ms) : "earlier";
    const last = groups[groups.length - 1];
    if (last?.key === key) last.items.push(result);
    else groups.push({ key, ms, items: [result] });
  }
  return groups.flatMap((group) => [
    {
      key: `day:${group.key}`,
      type: "header" as const,
      label: dayLabel(group.key, group.ms, today, yesterday),
      count: group.items.length,
    },
    ...group.items.map((result) => ({ key: `row:${result.id}`, type: "row" as const, result })),
  ]);
}

export function BrowserCard({
  results,
  pending,
  error,
  onOpen,
}: {
  results: SearchResult[];
  pending: boolean;
  error: string | null;
  onOpen: (result: SearchResult) => void;
}) {
  const [tab, setTab] = useState<BrowserTab>("all");
  const [query, setQuery] = useState("");
  const [pins, setPins] = useState(() => readIds(PINS_KEY));
  const [hidden, setHidden] = useState(() => readIds(HIDDEN_KEY));
  const [activeId, setActiveId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => saveIds(PINS_KEY, pins), [pins]);
  useEffect(() => saveIds(HIDDEN_KEY, hidden), [hidden]);
  useEffect(() => inputRef.current?.focus({ preventScroll: true }), []);

  // Hiding only filters the view; the browser history itself is never touched.
  const pages = useMemo(
    () => results.filter((result) => isPage(result) && !hidden.has(result.id)),
    [results, hidden],
  );
  const entries = useMemo(() => buildEntries(pages, tab, query, pins), [pages, tab, query, pins]);
  const rowIds = useMemo(
    () => entries.flatMap((entry) => (entry.type === "row" ? [entry.result.id] : [])),
    [entries],
  );
  const current = activeId !== null && rowIds.includes(activeId) ? activeId : (rowIds[0] ?? null);
  const virtualize = rowIds.length > VIRTUAL_ROWS;

  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => listRef.current?.closest<HTMLElement>('[data-slot="scroll-area-viewport"]') ?? null,
    estimateSize: (index) => (entries[index]?.type === "header" ? HEADER_PX : ROW_PX),
    overscan: 6,
  });

  const togglePin = useCallback((id: string) => setPins((cur) => withId(cur, id, !cur.has(id))), []);
  const hide = useCallback((id: string) => setHidden((cur) => withId(cur, id, true)), []);
  const showHidden = useCallback(() => setHidden(new Set()), []);
  const hover = useCallback((id: string) => setActiveId(id), []);

  const scrollRowIntoView = (id: string) => {
    const rows = listRef.current?.querySelectorAll<HTMLElement>("[data-row-id]") ?? [];
    Array.from(rows)
      .find((row) => row.dataset.rowId === id)
      ?.scrollIntoView({ block: "nearest" });
  };

  const move = (step: 1 | -1) => {
    if (rowIds.length === 0) return;
    const index = current === null ? -1 : rowIds.indexOf(current);
    const next = rowIds[Math.min(rowIds.length - 1, Math.max(0, index + step))];
    if (next === undefined) return;
    setActiveId(next);
    scrollRowIntoView(next);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    // Ctrl+Up/Down switches cards at the island level; leave those keys alone.
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      move(event.key === "ArrowDown" ? 1 : -1);
    } else if (event.key === "Enter" && event.target === inputRef.current) {
      const row = pages.find((result) => result.id === current);
      if (row) {
        event.preventDefault();
        onOpen(row);
      }
    }
  };

  const renderEntry = (entry: Entry) =>
    entry.type === "header" ? (
      <div key={entry.key} className="flex h-7 items-end justify-between px-2 pb-1 text-[12px] font-medium text-fg-3">
        <span>{entry.label}</span>
        <span className="tabular-nums">{entry.count}</span>
      </div>
    ) : (
      <BrowserRow
        key={entry.key}
        result={entry.result}
        active={entry.result.id === current}
        pinned={pins.has(entry.result.id)}
        onHover={hover}
        onOpen={onOpen}
        onTogglePin={togglePin}
        onHide={hide}
      />
    );

  const emptyCopy = (): { title: string; hint: string } => {
    if (tab === "pinned") return { title: "No pinned pages.", hint: "Pin a page to keep it here." };
    if (tab === "today") return { title: "Nothing visited today.", hint: "Choose All to see earlier pages." };
    if (query.trim()) return { title: "No pages match.", hint: "Clear the search or switch tabs." };
    return { title: "No pages to show.", hint: "Hidden pages stay in your browser history." };
  };

  const hiddenCount = results.filter((result) => isPage(result) && hidden.has(result.id)).length;
  const browsers = [...new Set(results.filter(isPage).map(browserName))].filter(
    (name) => name !== UNKNOWN_BROWSER,
  );
  let subtitle =
    browsers.length > 0
      ? `${browsers.length} ${browsers.length === 1 ? "browser" : "browsers"} · ${browsers.join(", ")}`
      : "Local history";
  if (error) subtitle = "Unavailable";
  else if (pending && results.length === 0) subtitle = "Reading browser history…";

  const body = error ? (
    <CardEmpty title="Browser history is unavailable." hint={error} />
  ) : pending && results.length === 0 ? (
    <div aria-hidden="true" className="flex flex-col">
      {[0, 1, 2].map((row) => (
        <span key={row} className="my-1 h-12 rounded-[10px] bg-raised" />
      ))}
    </div>
  ) : results.length === 0 ? (
    <CardEmpty
      title="No browser history yet."
      hint="Bookmarks and history appear after the browser source is indexed."
    />
  ) : entries.length === 0 ? (
    <CardEmpty {...emptyCopy()} />
  ) : virtualize ? (
    <div ref={listRef} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
      {virtualizer.getVirtualItems().map((item) => {
        const entry = entries[item.index];
        return entry ? (
          <div
            key={item.key}
            className="absolute left-0 top-0 w-full"
            style={{ height: item.size, transform: `translateY(${item.start}px)` }}
          >
            {renderEntry(entry)}
          </div>
        ) : null;
      })}
    </div>
  ) : (
    <div ref={listRef} className="flex flex-col">
      {entries.map(renderEntry)}
    </div>
  );

  return (
    <div onKeyDown={onKeyDown} className="flex min-h-0 w-full flex-1 flex-col gap-3 p-4">
      <CardHeader
        icon={History}
        title="Browser History"
        subtitle={subtitle}
        end={
          <div className="flex shrink-0 flex-col items-end">
            {hiddenCount > 0 ? (
              <button
                type="button"
                onClick={showHidden}
                className="rounded-[6px] text-[12px] tabular-nums text-fg-3 outline-none hover:text-fg-2 focus-visible:ring-2 focus-visible:ring-hi"
              >
                {`Show hidden (${hiddenCount})`}
              </button>
            ) : null}
          </div>
        }
      />
      <label className="relative block">
        <Search
          size={16}
          strokeWidth={1.75}
          aria-hidden="true"
          className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-fg-3"
        />
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search visited pages..."
          aria-label="Search visited pages"
          className="h-9 w-full rounded-[10px] border border-line bg-raised pl-9 pr-3 text-[13px] text-fg outline-none placeholder:text-fg-3 focus-visible:ring-2 focus-visible:ring-hi"
        />
      </label>
      <AnimatedTabs
        tabs={TABS}
        value={tab}
        onChange={setTab}
        label="Browser history filter"
        className="flex gap-1 rounded-[10px] bg-raised p-1"
        itemClassName="h-7 flex-1 rounded-[6px] text-[12px] font-medium text-fg-3 transition-colors hover:text-fg-2 [&.is-active]:text-fg"
      />
      <ScrollArea className="min-h-0 flex-1">{body}</ScrollArea>
    </div>
  );
}

const BrowserRow = memo(function BrowserRow({
  result,
  active,
  pinned,
  onHover,
  onOpen,
  onTogglePin,
  onHide,
}: {
  result: SearchResult;
  active: boolean;
  pinned: boolean;
  onHover: (id: string) => void;
  onOpen: (result: SearchResult) => void;
  onTogglePin: (id: string) => void;
  onHide: (id: string) => void;
}) {
  const when = relativeTime(result.meta?.modified);
  const [iconFailed, setIconFailed] = useState(false);
  const favicon = iconFailed ? null : result.meta?.faviconDataUrl;
  return (
    <HighlightRow layoutId="browser-highlight" active={active} onHover={() => onHover(result.id)}>
      <div data-row-id={result.id} className="flex h-14 items-center gap-1 pl-2 pr-1">
        <button
          type="button"
          onClick={() => onOpen(result)}
          className="flex h-full min-w-0 flex-1 items-center gap-3 rounded-[10px] text-left outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-hi"
        >
          <span className="grid size-10 shrink-0 place-items-center rounded-[10px] bg-raised text-fg-2">
            {favicon ? (
              <img src={favicon} alt="" onError={() => setIconFailed(true)} className="size-5 object-contain" />
            ) : (
              <Globe size={16} strokeWidth={1.75} aria-hidden="true" />
            )}
          </span>
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-[13px] font-semibold text-fg">{result.title}</span>
            <span className="truncate text-[12px] text-fg-3">{`${hostOf(result)} · ${browserName(result)}`}</span>
          </span>
          {when ? <span className="shrink-0 text-[12px] tabular-nums text-fg-3">{when}</span> : null}
        </button>
        <button
          type="button"
          aria-label={pinned ? "Unpin page" : "Pin page"}
          aria-pressed={pinned}
          onClick={() => onTogglePin(result.id)}
          className={cn(
            "grid size-8 shrink-0 place-items-center rounded-[8px] outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-hi",
            pinned ? "text-hi" : "text-fg-3 hover:text-fg-2",
          )}
        >
          <Pin size={16} strokeWidth={1.75} fill={pinned ? "currentColor" : "none"} aria-hidden="true" />
        </button>
        <button
          type="button"
          aria-label="Hide page"
          onClick={() => onHide(result.id)}
          className="grid size-8 shrink-0 place-items-center rounded-[8px] text-fg-3 outline-none hover:bg-hover hover:text-fg-2 focus-visible:ring-2 focus-visible:ring-hi"
        >
          <X size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
      </div>
    </HighlightRow>
  );
});
