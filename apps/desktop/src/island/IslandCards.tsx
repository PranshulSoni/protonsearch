import type { IslandCardId, SearchResult, SearchScope } from "../bridge/types";
import { useEffect, useMemo, useState } from "react";
import { motion } from "motion/react";
import { Logo } from "../components/Logo";
import { groupResultsInRankOrder } from "../lib/results";
import { ISLAND_VISIBLE_ROWS } from "./islandDims";
import { SharedResultRow } from "../launcher/SharedResultRow";
import { IslandIcon, type IslandIconName } from "./IslandIcon";
import { AnimatedTabs } from "./ui/AnimatedTabs";
import { HighlightRow } from "./ui/HighlightList";

/** Search card and the fallback for a card id with no adapter. The other cards live in ./cards. */

function rowsPerPage(count: number): number {
  return Math.max(1, Math.min(count, 5));
}

const CARD_ICON: Record<IslandCardId, IslandIconName> = {
  search: "search",
  media: "media",
  clipboard: "clipboard",
  agent: "agent",
  controls: "controls",
  browser: "browser",
  git: "git",
  bell: "bell",
};

function CardHead({
  card,
  title,
  subtitle,
}: {
  card: IslandCardId;
  title: string;
  subtitle: string;
}) {
  return (
    <header className="island-card-head">
      <span className="island-icon-tile">
        <IslandIcon name={CARD_ICON[card]} size={23} />
      </span>
      <div className="island-card-heading">
        <h2>{title}</h2>
        <p>{subtitle}</p>
      </div>
    </header>
  );
}

function EmptyState({ title, hint }: { title: string; hint: string }) {
  return (
    <div className="island-empty" role="status">
      <strong>{title}</strong>
      <span>{hint}</span>
    </div>
  );
}

export function SearchCard({
  compact,
  query,
  onQueryChange,
  scope,
  onScope,
  results,
  pending,
  error,
  onOpen,
  onFocus,
  inputRef,
  placeholder,
}: {
  compact: boolean;
  query: string;
  onQueryChange: (value: string) => void;
  scope: SearchScope;
  onScope: (scope: SearchScope) => void;
  results: SearchResult[];
  pending: boolean;
  error: string | null;
  onOpen: (result: SearchResult) => void;
  onFocus: () => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
  placeholder: string;
}) {
  const [activeIndex, setActiveIndex] = useState(0);
  // Every new result set starts on its top hit, even when its length matches
  // the previous one, so Enter never opens a row from the last keystroke.
  useEffect(() => {
    setActiveIndex(0);
  }, [results]);
  const hasQuery = query.trim().length > 0;
  const zeroResults = hasQuery && !pending && !error && results.length === 0;
  // Ranked order is kept; a label starts wherever the result type changes.
  const visibleGroups = useMemo(() => groupResultsInRankOrder(results, ISLAND_VISIBLE_ROWS), [results]);
  const orderedResults = results;
  const rowIndexById = useMemo(
    () => new Map(orderedResults.map((result, index) => [result.id, index] as const)),
    [orderedResults],
  );
  const openActive = () => {
    const result = orderedResults[activeIndex];
    if (result) onOpen(result);
  };
  const filters: Array<{ id: SearchScope; label: string }> = [
    { id: "all", label: "All" },
    { id: "files", label: "Files" },
    { id: "folders", label: "Folders" },
    { id: "apps", label: "Apps" },
    { id: "browser", label: "Web" },
    { id: "actions", label: "Actions" },
    { id: "ocr", label: "OCR" },
  ];
  return (
    <div className={"island-search" + (compact ? " island-search--compact" : " island-search--expanded")}>
      <div className="island-search-row">
        <span className="island-brand" aria-hidden="true">
          <Logo size={34} />
        </span>
        <input
          ref={inputRef}
          className="island-search-input"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          onFocus={onFocus}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setActiveIndex((index) =>
                results.length ? (index + 1) % rowsPerPage(results.length) : 0,
              );
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActiveIndex((index) =>
                results.length
                  ? (index - 1 + rowsPerPage(results.length)) % rowsPerPage(results.length)
                  : 0,
              );
            } else if (event.key === "Enter") {
              event.preventDefault();
              openActive();
            }
          }}
          placeholder={placeholder}
          aria-label="Search"
          role="combobox"
          aria-expanded={!compact}
          aria-controls="island-search-results"
          autoComplete="off"
          spellCheck={false}
        />
        {!compact ? <IslandIcon name="search" size={20} /> : null}
      </div>
      {!compact ? (
        // Results fade in while the surface grows, instead of popping in at full size.
        <motion.div
          className="island-search-body"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.22, delay: 0.08 }}
        >
          <AnimatedTabs
            className="island-filters"
            itemClassName="island-filter"
            label="Search filters"
            tabs={filters}
            value={scope}
            onChange={(id) => {
              onScope(id);
              inputRef.current?.focus();
            }}
          />
          <div className="island-results" id="island-search-results" role="listbox" aria-label="Search results">
            {!hasQuery ? null : pending && results.length === 0 ? (
              <div className="island-skeleton" aria-hidden="true">
                {[0, 1, 2].map((row) => (
                  <span className="island-skeleton-row" key={row} />
                ))}
              </div>
            ) : error ? (
              <EmptyState title="Search is unavailable." hint={error} />
            ) : zeroResults && query.trim() ? (
              <div className="island-zero" role="status">
                {"No matches for “" + query.trim() + "”."}
              </div>
            ) : zeroResults ? null : (
              visibleGroups.map((group, groupIndex) => (
                <div
                  className="island-result-group"
                  role="group"
                  aria-labelledby={"island-group-" + groupIndex}
                  key={group.label + "-" + group.items[0].index}
                >
                  <div
                    className="island-group-label"
                    id={"island-group-" + groupIndex}
                    role="presentation"
                  >
                    {group.label}
                  </div>
                  {group.items.map((item) => {
                    const result = item.result;
                    const index = rowIndexById.get(result.id) ?? 0;
                    return (
                      <HighlightRow
                        key={result.id}
                        layoutId="island-search-highlight"
                        active={index === activeIndex}
                        onHover={() => setActiveIndex(index)}
                      >
                        <SharedResultRow
                          result={result}
                          selected={index === activeIndex}
                          optionId={"island-opt-" + result.id.replace(/[^a-zA-Z0-9_-]/g, "_")}
                          descriptionId={"island-desc-" + result.id.replace(/[^a-zA-Z0-9_-]/g, "_")}
                          metaId={"island-meta-" + result.id.replace(/[^a-zA-Z0-9_-]/g, "_")}
                          testId="island-result-row"
                          className="island-result-row"
                          onClick={() => onOpen(result)}
                        />
                      </HighlightRow>
                    );
                  })}
                </div>
              ))
            )}
          </div>
        </motion.div>
      ) : null}
    </div>
  );
}

export function UnsupportedCard({
  card,
  title,
  subtitle,
  reason,
}: {
  card: IslandCardId;
  title: string;
  subtitle: string;
  reason: string;
}) {
  return (
    <div className="island-card">
      <CardHead card={card} title={title} subtitle={subtitle} />
      <div className="island-divider" />
      <div className="island-unsupported" role="status">
        <span className="island-unsupported-icon">
          <IslandIcon name={CARD_ICON[card]} size={23} />
        </span>
        <strong>Not available yet</strong>
        <span>{reason}</span>
        <span className="island-unsupported-note">No mock data is shown.</span>
      </div>
    </div>
  );
}
