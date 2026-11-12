import { useEffect, useRef, useState } from "react";
import { Command } from "cmdk";
import {
  Check,
  Clipboard,
  Code as CodeIcon,
  FileText as FileIcon,
  Image as ImageIcon,
  Link as LinkIcon,
  Pin,
  Type as TextIcon,
  type LucideIcon,
} from "lucide-react";
import { api } from "../../bridge/api";
import type { SearchResult } from "../../bridge/types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { AnimatedTabs } from "../ui/AnimatedTabs";
import { HighlightRow } from "../ui/HighlightList";
import { CLIP_TABS, clipboardKind, filterClipboard, relativeTime, type ClipTab } from "../islandData";
import { CardEmpty, CardHeader } from "./CardChrome";

const COPY_FEEDBACK_MS = 1500;

const KIND: Record<ReturnType<typeof clipboardKind>, { label: string; icon: LucideIcon }> = {
  text: { label: "Text", icon: TextIcon },
  links: { label: "Link", icon: LinkIcon },
  code: { label: "Code", icon: CodeIcon },
  images: { label: "Image", icon: ImageIcon },
  files: { label: "File", icon: FileIcon },
};

export function ClipboardCard({
  results,
  pending,
  error,
  tab,
  onTab,
  onOpen,
}: {
  results: SearchResult[];
  pending: boolean;
  error: string | null;
  tab: ClipTab;
  onTab: (tab: ClipTab) => void;
  onOpen: (result: SearchResult) => void;
}) {
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [activeId, setActiveId] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const visible = filterClipboard(results, tab);
  const active = visible.some((result) => result.id === activeId) ? (activeId ?? "") : (visible[0]?.id ?? "");
  const latest = results.reduce((max, result) => Math.max(max, result.meta?.modified ?? 0), 0);
  const hasRows = !error && visible.length > 0;

  useEffect(() => {
    if (!copiedId) return;
    const timer = window.setTimeout(() => setCopiedId(null), COPY_FEEDBACK_MS);
    return () => window.clearTimeout(timer);
  }, [copiedId]);

  // Focus the list so the arrow keys work as soon as the card is shown.
  useEffect(() => {
    if (hasRows) listRef.current?.focus({ preventScroll: true });
  }, [hasRows]);

  const copy = (result: SearchResult) => {
    setCopiedId(result.id);
    onOpen(result);
  };

  // The engine raises clipboard:change after a pin, which refreshes the list.
  const togglePin = (result: SearchResult) => {
    void api.setClipboardPinned(result.id, !result.meta?.pinned).catch(() => undefined);
  };

  let subtitle = "Nothing copied yet";
  if (error) subtitle = "Unavailable";
  else if (pending && results.length === 0) subtitle = "Reading your recent copies…";
  else if (latest) subtitle = `Copied ${relativeTime(latest)}`;

  return (
    <div className="flex min-h-0 w-full flex-1 flex-col gap-3 p-4">
      <CardHeader icon={Clipboard} title="Clipboard" subtitle={subtitle} />
      <AnimatedTabs
        tabs={CLIP_TABS}
        value={tab}
        onChange={onTab}
        label="Clipboard categories"
        className="flex gap-1 rounded-[10px] bg-raised p-1"
        itemClassName="h-7 flex-1 rounded-[6px] px-2 text-[12px] font-medium text-fg-3 transition-colors hover:text-fg-2 [&.is-active]:text-fg"
      />
      <ScrollArea className="min-h-0 flex-1">
        {error ? (
          <CardEmpty title="Clipboard history is unavailable." hint={error} />
        ) : pending && results.length === 0 ? (
          <div aria-hidden="true" className="flex flex-col">
            {[0, 1, 2].map((row) => (
              <span key={row} className="my-1 h-12 rounded-[10px] bg-raised" />
            ))}
          </div>
        ) : visible.length === 0 ? (
          <CardEmpty
            title={results.length === 0 ? "No clipboard history yet." : `No ${tab} clips yet.`}
            hint={
              results.length === 0
                ? "Copied text and images will appear here."
                : tab === "pinned"
                  ? "Pin a clip to keep it here."
                  : "Try another category."
            }
          />
        ) : (
          <Command
            ref={listRef}
            tabIndex={0}
            label="Clipboard history"
            shouldFilter={false}
            value={active}
            onValueChange={setActiveId}
            onKeyDown={(event) => {
              const row = visible.find((result) => result.id === active);
              if (row && event.key.toLowerCase() === "p" && !event.ctrlKey && !event.metaKey && !event.altKey) {
                event.preventDefault();
                togglePin(row);
              }
            }}
            className="outline-none"
          >
            <Command.List label="Clipboard history">
              {visible.map((result) => {
                const kind = KIND[clipboardKind(result)];
                const Icon = kind.icon;
                const thumb = result.meta?.thumbnailDataUrl;
                const pinned = result.meta?.pinned === true;
                return (
                  <Command.Item key={result.id} value={result.id} onSelect={() => copy(result)}>
                    <HighlightRow layoutId="clipboard-highlight" active={result.id === active}>
                      <div className="flex h-14 items-center gap-3 px-2">
                        <span className="grid h-10 w-10 shrink-0 place-items-center overflow-hidden rounded-[10px] bg-raised text-fg-2">
                          {thumb ? (
                            <img src={thumb} alt="" className="h-full w-full object-cover" />
                          ) : (
                            <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
                          )}
                        </span>
                        <span className="flex min-w-0 flex-1 flex-col">
                          <span className="truncate text-[13px] font-semibold text-fg">
                            {result.title || "Clipboard item"}
                          </span>
                          <span className="truncate text-[12px] tabular-nums text-fg-3">
                            {`${kind.label} · ${relativeTime(result.meta?.modified) || "Local clip"}`}
                          </span>
                        </span>
                        {copiedId === result.id ? (
                          <Check size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-ok" />
                        ) : null}
                        <button
                          type="button"
                          tabIndex={-1}
                          aria-label={pinned ? "Unpin clip" : "Pin clip"}
                          aria-pressed={pinned}
                          title={pinned ? "Unpin (P)" : "Pin (P)"}
                          onClick={(event) => {
                            event.stopPropagation();
                            togglePin(result);
                          }}
                          className={cn(
                            "grid size-8 shrink-0 place-items-center rounded-[8px] outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-hi",
                            pinned ? "text-hi" : "text-fg-3 hover:text-fg-2",
                          )}
                        >
                          <Pin size={16} strokeWidth={1.75} fill={pinned ? "currentColor" : "none"} aria-hidden="true" />
                        </button>
                      </div>
                    </HighlightRow>
                  </Command.Item>
                );
              })}
            </Command.List>
          </Command>
        )}
      </ScrollArea>
    </div>
  );
}
