import type {
  AgentStatus,
  DetectedAgent,
  IslandCardId,
  SearchResult,
  SearchScope,
} from "../bridge/types";
import { useEffect, useState } from "react";
import { Logo } from "../components/Logo";
import { IslandIcon, type IslandIconName } from "./IslandIcon";
import { relativeTime, type ClipTab, CLIP_TABS, clipboardKind } from "./islandData";

/** V9 card views. All rows come from real bridge data; unsupported cards say so. */

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

export function CardHead({
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
  total,
  pending,
  error,
  onOpen,
  onFocus,
  inputRef,
}: {
  compact: boolean;
  query: string;
  onQueryChange: (value: string) => void;
  scope: SearchScope;
  onScope: (scope: SearchScope) => void;
  results: SearchResult[];
  total: number;
  pending: boolean;
  error: string | null;
  onOpen: (result: SearchResult) => void;
  onFocus: () => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
}) {
  const [activeIndex, setActiveIndex] = useState(0);
  useEffect(() => {
    setActiveIndex(0);
  }, [results]);
  const openActive = () => {
    const result = results[activeIndex];
    if (result) onOpen(result);
  };
  const filters: Array<{ id: SearchScope; label: string }> = [
    { id: "all", label: "All" },
    { id: "files", label: "Files" },
    { id: "folders", label: "Folders" },
    { id: "apps", label: "Apps" },
  ];
  return (
    <div className={"island-search" + (compact ? " island-search--compact" : " island-search--expanded")}>
      <div className="island-search-row">
        <span className="island-brand" aria-hidden="true">
          <Logo size={27} />
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
          placeholder="Search anything…"
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
        <>
          <div className="island-filters" role="radiogroup" aria-label="Search filters">
            {filters.map((filter) => (
              <button
                key={filter.id}
                type="button"
                role="radio"
                aria-checked={scope === filter.id}
                className={"island-filter" + (scope === filter.id ? " is-active" : "")}
                onClick={() => onScope(filter.id)}
              >
                {filter.label}
              </button>
            ))}
          </div>
          <div className="island-results" id="island-search-results" role="listbox" aria-label="Search results">
            {pending && results.length === 0 ? (
              <div className="island-skeleton" aria-hidden="true">
                {[0, 1, 2].map((row) => (
                  <span className="island-skeleton-row" key={row} />
                ))}
              </div>
            ) : error ? (
              <EmptyState title="Search is unavailable." hint={error} />
            ) : results.length === 0 ? (
              <EmptyState
                title={query.trim() ? "No matching results." : "Start typing to search."}
                hint={query.trim() ? "Try another word or filter." : "Files, folders, apps and more."}
              />
            ) : (
              results.slice(0, 5).map((result) => (
                <button
                  type="button"
                  role="option"
                  aria-selected={results.indexOf(result) === activeIndex}
                  key={result.id}
                  className={
                    "island-result-row" +
                    (results.indexOf(result) === activeIndex ? " is-active" : "")
                  }
                  onClick={() => onOpen(result)}
                  onMouseEnter={() => setActiveIndex(results.indexOf(result))}
                >
                  <span className="island-result-icon">
                    <IslandIcon
                      name={
                        result.kind === "folder"
                          ? "file"
                          : result.kind === "application"
                            ? "search"
                            : result.kind === "action"
                              ? "bolt"
                              : "file"
                      }
                      size={18}
                    />
                  </span>
                  <span className="island-result-body">
                    <span className="island-result-title">{result.title}</span>
                    <span className="island-result-sub">{result.subtitle}</span>
                  </span>
                  <span className="island-result-meta">{result.meta?.label ?? ""}</span>
                </button>
              ))
            )}
          </div>
          <footer className="island-result-foot">
            <span>{total} local results</span>
            <span className="island-kbd-hints">
              <kbd>↑</kbd>
              <kbd>↓</kbd>
              <span>navigate</span>
              <kbd>Enter</kbd>
              <span>open</span>
            </span>
          </footer>
        </>
      ) : null}
    </div>
  );
}

export function ClipboardCard({
  results,
  total,
  pending,
  error,
  tab,
  onTab,
  onOpen,
}: {
  results: SearchResult[];
  total: number;
  pending: boolean;
  error: string | null;
  tab: ClipTab;
  onTab: (tab: ClipTab) => void;
  onOpen: (result: SearchResult) => void;
}) {
  const visible = tab === "pinned" ? [] : results;
  const selected = visible[0] ?? null;
  return (
    <div className="island-card island-clipboard">
      <CardHead
        card="clipboard"
        title="Clipboard"
        subtitle={pending ? "Reading clipboard history…" : total + " recent clips captured locally"}
      />
      <div className="island-divider" />
      <div className="island-inline-nav" role="tablist" aria-label="Clipboard categories">
        {CLIP_TABS.map((entry) => {
          if (entry.id === "pinned") {
            return (
              <button
                key={entry.id}
                type="button"
                role="tab"
                aria-selected={false}
                className="island-nav-chip"
                disabled
                title="Pinning isn't available yet."
              >
                {entry.label}
              </button>
            );
          }
          const count = entry.id === "all" ? results.length : results.filter((r) => clipboardKind(r) === entry.id).length;
          return (
            <button
              key={entry.id}
              type="button"
              role="tab"
              aria-selected={tab === entry.id}
              className={"island-nav-chip" + (tab === entry.id ? " is-active" : "")}
              onClick={() => onTab(entry.id)}
            >
              {entry.label}
              {count > 0 ? <span className="island-nav-count">{count}</span> : null}
            </button>
          );
        })}
      </div>
      <div className="island-clip-list" role="listbox" aria-label="Clipboard history">
        {error ? (
          <EmptyState title="Clipboard history is unavailable." hint={error} />
        ) : pending && results.length === 0 ? (
          <div className="island-skeleton" aria-hidden="true">
            {[0, 1, 2].map((row) => (
              <span className="island-skeleton-row" key={row} />
            ))}
          </div>
        ) : tab === "pinned" ? (
          <EmptyState title="Pinning isn't available yet." hint="Pinned clips will appear here once pinning ships." />
        ) : visible.length === 0 ? (
          <EmptyState
            title={results.length === 0 ? "No clipboard history yet." : "No " + tab + " clips yet."}
            hint={
              results.length === 0
                ? "Copied text and images will appear here."
                : "Try another category."
            }
          />
        ) : (
          visible.slice(0, 3).map((result, index) => (
            <button
              type="button"
              role="option"
              aria-selected={index === 0}
              key={result.id}
              className={"island-clip-row" + (index === 0 ? " is-active" : "")}
              onClick={() => onOpen(result)}
            >
              <span className="island-clip-icon">
                <IslandIcon
                  name={clipboardKind(result) === "links" ? "link" : clipboardKind(result) === "code" ? "code" : clipboardKind(result) === "images" ? "image" : clipboardKind(result) === "files" ? "file" : "text"}
                  size={18}
                />
              </span>
              <span className="island-clip-body">
                <span className="island-clip-title">{result.title}</span>
                <span className="island-clip-sub">
                  {(result.meta?.label ?? clipboardKind(result)) + " · " + relativeTime(result.meta?.modified)}
                </span>
              </span>
              {index === 0 ? <IslandIcon name="check" size={16} className="island-check" /> : null}
            </button>
          ))
        )}
      </div>
      {selected ? (
        <div className="island-snippet" aria-label="Selected clip text">
          {selected.title}
        </div>
      ) : null}
    </div>
  );
}

export function AgentsCard({
  agents,
  status,
}: {
  agents: DetectedAgent[];
  status: AgentStatus | null;
}) {
  const configured = status != null && status.state !== "not_configured";
  return (
    <div className="island-card island-agent">
      <CardHead
        card="agent"
        title="Agents"
        subtitle={configured ? status?.runtime ?? "Agent runtime" : "Agent runtime isn't configured"}
      />
      <div className="island-divider" />
      {!configured ? (
        <EmptyState
          title="No agent configured."
          hint="Enable the agent runtime in ProtonSearch → Preferences to see runs here."
        />
      ) : (
        <>
          <div className="island-context-caps">
            {"CURRENT WORKSPACE · " + agents.filter((agent) => agent.available).length + " AGENTS"}
          </div>
          <div className="island-agent-list">
            {agents.length === 0 ? (
              <EmptyState title="No agents detected." hint="Install a supported agent runtime to continue." />
            ) : (
              agents.map((agent) => (
                <div className="island-agent-row" key={agent.id}>
                  <span className="island-agent-icon">
                    <IslandIcon name="agent" size={18} />
                  </span>
                  <span className="island-agent-body">
                    <span className="island-agent-name">{agent.name}</span>
                    <span className="island-agent-task">
                      {agent.kind}
                      {agent.version ? " · v" + agent.version : ""}
                    </span>
                  </span>
                  <span className="island-agent-badge">
                    {agent.available ? "READY" : "UNAVAILABLE"}
                  </span>
                </div>
              ))
            )}
          </div>
          <div className="island-agent-actions">
            <button type="button" className="island-agent-button" disabled title="Agent task hierarchy API isn't wired yet.">
              <IslandIcon name="pause" size={14} /> Pause
            </button>
            <button type="button" className="island-agent-button" disabled title="Agent logs API isn't wired yet.">
              Details
            </button>
          </div>
          <p className="island-honest-note">Task hierarchy, progress and logs appear once the agent task API lands.</p>
        </>
      )}
    </div>
  );
}

function ResultListCard({
  card,
  title,
  subtitle,
  caps,
  results,
  pending,
  error,
  onOpen,
  emptyTitle,
  emptyHint,
}: {
  card: IslandCardId;
  title: string;
  subtitle: string;
  caps: string;
  results: SearchResult[];
  pending: boolean;
  error: string | null;
  onOpen: (result: SearchResult) => void;
  emptyTitle: string;
  emptyHint: string;
}) {
  return (
    <div className="island-card island-result-card">
      <CardHead card={card} title={title} subtitle={subtitle} />
      <div className="island-divider" />
      <div className="island-context-caps">{caps}</div>
      <div className="island-choose-list">
        {error ? (
          <EmptyState title="This source is unavailable." hint={error} />
        ) : pending && results.length === 0 ? (
          <div className="island-skeleton" aria-hidden="true">
            {[0, 1, 2].map((row) => (
              <span className="island-skeleton-row" key={row} />
            ))}
          </div>
        ) : results.length === 0 ? (
          <EmptyState title={emptyTitle} hint={emptyHint} />
        ) : (
          results.slice(0, 3).map((result) => (
            <button
              type="button"
              className="island-choose-row"
              key={result.id}
              onClick={() => onOpen(result)}
            >
              <span className="island-choose-icon">
                <IslandIcon
                  name={card === "browser" ? (result.kind === "bookmark" ? "link" : "browser") : "git"}
                  size={18}
                />
              </span>
              <span className="island-choose-body">
                <span className="island-choose-title">{result.title}</span>
                <span className="island-choose-meta">
                  {result.meta?.label ?? relativeTime(result.meta?.modified) ?? result.subtitle}
                </span>
              </span>
              <IslandIcon name="more" size={16} className="island-choose-more" />
            </button>
          ))
        )}
      </div>
    </div>
  );
}

export function BrowserCard(props: {
  results: SearchResult[];
  pending: boolean;
  error: string | null;
  onOpen: (result: SearchResult) => void;
}) {
  const bookmarks = props.results.filter((result) => result.kind === "bookmark");
  const history = props.results.filter((result) => result.kind === "browser_history");
  const rows = [...bookmarks.slice(0, 2), ...history.slice(0, 1)];
  return (
    <ResultListCard
      card="browser"
      title="Browser"
      subtitle="Bookmarks and recent history"
      caps={bookmarks.length > 0 ? "BOOKMARKS · SAVED PAGES" : "RECENT HISTORY · LOCAL"}
      results={rows}
      pending={props.pending}
      error={props.error}
      onOpen={props.onOpen}
      emptyTitle="No browser history yet."
      emptyHint="Bookmarks and history appear after the browser source is indexed."
    />
  );
}

export function GitCard(props: {
  results: SearchResult[];
  pending: boolean;
  error: string | null;
  onOpen: (result: SearchResult) => void;
}) {
  return (
    <ResultListCard
      card="git"
      title="Git"
      subtitle="Recent commits and repositories"
      caps="RECENT COMMITS · LOCAL"
      results={props.results}
      pending={props.pending}
      error={props.error}
      onOpen={props.onOpen}
      emptyTitle="No Git activity yet."
      emptyHint="Enable Git indexing in Preferences to see commits here."
    />
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
