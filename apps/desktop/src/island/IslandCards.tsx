import type {
  AgentStatus,
  AgentTask,
  IslandCardId,
  SearchResult,
  SearchScope,
} from "../bridge/types";
import { useEffect, useMemo, useState, type FormEvent, type KeyboardEvent } from "react";
import { api } from "../bridge/api";
import { buildAgentRunRequest, createAgentRunId } from "../app/agentRun";
import { useTodos, type TodoItem } from "../lib/todoStore";
import { Logo } from "../components/Logo";
import { groupResultsInRankOrder } from "../lib/results";
import { ISLAND_VISIBLE_ROWS } from "./islandDims";
import { SharedResultRow } from "../launcher/SharedResultRow";
import { IslandIcon, type IslandIconName } from "./IslandIcon";
import {
  relativeTime,
  type ClipTab,
  CLIP_TABS,
  clipboardKind,
  filterClipboard,
} from "./islandData";

/** V9 card views. All rows come from real bridge data; unsupported cards say so. */

function clipLabel(kind: string): string {
  switch (kind) {
    case "links":
      return "Link";
    case "code":
      return "Code";
    case "images":
      return "Image";
    case "files":
      return "File";
    default:
      return "Text";
  }
}

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

export function EmptyState({ title, hint }: { title: string; hint: string }) {
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
  placeholder,
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
    { id: "actions", label: "Actions" },
    { id: "ocr", label: "OCR" },
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
        <>
          <div className="island-filters" role="radiogroup" aria-label="Search filters">
            {filters.map((filter) => (
              <button
                key={filter.id}
                type="button"
                role="radio"
                aria-checked={scope === filter.id}
                className={"island-filter" + (scope === filter.id ? " is-active" : "")}
                // Keep typing focus in the search box when a filter is clicked.
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  onScope(filter.id);
                  inputRef.current?.focus();
                }}
              >
                {filter.label}
              </button>
            ))}
          </div>
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
                      <SharedResultRow
                        key={result.id}
                        result={result}
                        selected={index === activeIndex}
                        optionId={"island-opt-" + result.id.replace(/[^a-zA-Z0-9_-]/g, "_")}
                        descriptionId={"island-desc-" + result.id.replace(/[^a-zA-Z0-9_-]/g, "_")}
                        metaId={"island-meta-" + result.id.replace(/[^a-zA-Z0-9_-]/g, "_")}
                        testId="island-result-row"
                        className="island-result-row"
                        onMouseEnter={() => setActiveIndex(index)}
                        onClick={() => onOpen(result)}
                      />
                    );
                  })}
                </div>
              ))
            )}
          </div>
          {hasQuery && !zeroResults ? (
            <footer className="island-result-foot">
              <span>{total} local results</span>
            </footer>
          ) : null}
        </>
      ) : null}
    </div>
  );
}

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
  const visible = filterClipboard(results, tab);
  return (
    <div className="island-card island-clipboard">
      <CardHead
        card="clipboard"
        title="Clipboard"
        subtitle={pending ? "Reading your recent copies…" : "Your recent copies"}
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
            </button>
          );
        })}
      </div>
      <div className="island-clip-list island-scrollable" role="listbox" aria-label="Clipboard history">
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
          visible.map((result, index) => {
            const isCopied = copiedId === result.id || (copiedId === null && index === 0);
            return (
              <button
                type="button"
                role="option"
                aria-selected={index === 0}
                key={result.id}
                className={"island-clip-row" + (index === 0 ? " is-active" : "")}
                onClick={() => {
                  setCopiedId(result.id);
                  setTimeout(() => setCopiedId((cur) => cur === result.id ? null : cur), 1500);
                  onOpen(result);
                }}
              >
                <span className="island-clip-icon">
                  {result.meta?.thumbnailDataUrl ? (
                    <img
                      className="island-clip-thumb"
                      src={result.meta.thumbnailDataUrl}
                      alt=""
                    />
                  ) : (
                    <IslandIcon
                      name={clipboardKind(result) === "links" ? "link" : clipboardKind(result) === "code" ? "code" : clipboardKind(result) === "images" ? "image" : clipboardKind(result) === "files" ? "file" : "text"}
                      size={18}
                    />
                  )}
                </span>
                <span className="island-clip-body">
                  <span className="island-clip-title">{result.title || "Clipboard item"}</span>
                  <span className="island-clip-sub">
                    {clipLabel(clipboardKind(result)) +
                      " · " +
                      (relativeTime(result.meta?.modified) || "Local clip")}
                  </span>
                </span>
                {isCopied ? <IslandIcon name="check" size={16} className="island-check" /> : null}
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}

export function AgentsCard({
  status,
  tasks,
  onPause,
  onResume,
}: {
  status: AgentStatus | null;
  tasks: AgentTask[];
  onPause: (taskId: string) => void;
  onResume: (taskId: string) => void;
}) {
  const [newPrompt, setNewPrompt] = useState("");
  const [launching, setLaunching] = useState(false);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const configured = status != null && status.state !== "not_configured";
  const selected = tasks[0] ?? null;
  const selectedProgress =
    selected && selected.progress.kind === "determinate" ? selected.progress.fraction : null;

  const handleLaunch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newPrompt.trim() || launching) return;
    setLaunching(true);
    setLaunchError(null);
    try {
      await api.agentStart(buildAgentRunRequest(createAgentRunId(), newPrompt.trim()));
      setNewPrompt("");
    } catch (reason) {
      setLaunchError(String(reason));
    } finally {
      setLaunching(false);
    }
  };

  return (
    <div className="island-card island-agent">
      <CardHead
        card="agent"
        title="Agents"
        subtitle={configured ? "Pi tasks" : "Pi is not configured"}
      />
      <div className="island-divider" />
      {!configured ? (
        <EmptyState
          title="No agent configured."
          hint="Enable the agent runtime in ProtonSearch → Settings to see runs here."
        />
      ) : (
        <>
          {launchError ? <p role="alert">{launchError}</p> : null}
          <form className="island-agent-new" onSubmit={handleLaunch} style={{ display: "flex", gap: "6px", margin: "6px 12px 10px" }}>
            <input
              className="island-search-input"
              style={{ flex: 1, height: "30px", fontSize: "12px", padding: "0 10px", background: "rgba(255,255,255,0.06)", borderRadius: "6px", border: "1px solid rgba(255,255,255,0.08)", color: "#fff" }}
              placeholder="Ask Pi to do something…"
              value={newPrompt}
              onChange={(e) => setNewPrompt(e.target.value)}
              disabled={launching}
            />
            <button
              type="submit"
              className="btn btn-sm"
              style={{ height: "30px", padding: "0 10px", fontSize: "11px", fontWeight: 600, background: "var(--ps-brand-red, #dc2626)", color: "#fff", borderRadius: "6px", border: 0, cursor: "pointer" }}
              disabled={launching || !newPrompt.trim()}
            >
              + Run
            </button>
          </form>
          <div className="island-context-caps">
            {"CURRENT WORKSPACE · " +
              tasks.length +
              " " +
              (tasks.length === 1 ? "TASK" : "TASKS")}
          </div>
          <div className="island-agent-list">
            {tasks.length === 0 ? (
              <EmptyState
                title="No agent tasks yet."
                hint="Launch a task from Workflows and it will appear here."
              />
            ) : (
              tasks.slice(0, 3).map((task) => (
                <div className="island-agent-row" key={task.id}>
                  <span className="island-agent-icon">
                    <IslandIcon name="agent" size={18} />
                  </span>
                  <span className="island-agent-body">
                    <span className="island-agent-name">{task.title}</span>
                    <span className="island-agent-task">
                      {task.promptSummary || task.runtime + " · " + task.state}
                    </span>
                  </span>
                  <span className="island-agent-badge">{task.state.toUpperCase()}</span>
                </div>
              ))
            )}
          </div>
          {selected && selected.progress.kind === "determinate" ? (
            <div className="island-progress-row">
              <span>Progress</span>
              <span>{selectedProgress != null ? Math.round(selectedProgress * 100) + "%" : "—"}</span>
              <span className="island-progress-track" aria-hidden="true">
                <span
                  className="island-progress-fill"
                  style={{ width: (selectedProgress ?? 0) * 100 + "%" }}
                />
              </span>
            </div>
          ) : null}
          {selected && selected.progress.kind === "indeterminate" ? (
            <div className="island-progress-row">
              <span>Progress</span>
              <span>Working…</span>
              <span className="island-progress-track island-progress-track--indeterminate" aria-hidden="true">
                <span className="island-progress-fill" />
              </span>
            </div>
          ) : null}
          <div className="island-agent-actions">
            {selected?.controls.canPause ? (
              <button
                type="button"
                className="island-agent-button"
                title="Pause this task"
                onClick={() => selected && onPause(selected.id)}
              >
                <IslandIcon name="pause" size={14} /> Pause
              </button>
            ) : null}
            {selected?.controls.canResume ? (
              <button
                type="button"
                className="island-agent-button"
                title="Resume this task"
                onClick={() => selected && onResume(selected.id)}
              >
                <IslandIcon name="play" size={14} /> Resume
              </button>
            ) : null}

          </div>
          {selected?.permissions ? (
            <p className="island-honest-note">
              {selected.writeAccess ? "Write access" : "Read-only"}
              {" · " +
                selected.permissions.granted.length +
                " " +
                (selected.permissions.granted.length === 1 ? "permission" : "permissions") +
                " granted"}
              {selected.log.length > 0 ? " · " + selected.log[selected.log.length - 1].message : ""}
            </p>
          ) : null}
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
  const chooseRow = (result: SearchResult) => (
    <button
      type="button"
      className="island-choose-row"
      key={result.id}
      onClick={() => props.onOpen(result)}
    >
      <span className="island-choose-icon">
        <IslandIcon name={result.kind === "bookmark" ? "link" : "browser"} size={18} />
      </span>
      <span className="island-choose-body">
        <span className="island-choose-title">{result.title}</span>
        <span className="island-choose-meta">
          {relativeTime(result.meta?.modified) || result.subtitle}
        </span>
      </span>
      <IslandIcon name="chevron" size={16} className="island-choose-tail" />
    </button>
  );
  return (
    <div className="island-card island-result-card">
      <CardHead card="browser" title="Browser" subtitle="Bookmarks & history" />
      <div className="island-divider" />
      {props.error ? (
        <EmptyState title="This source is unavailable." hint={props.error} />
      ) : props.pending && props.results.length === 0 ? (
        <div className="island-skeleton" aria-hidden="true">
          {[0, 1, 2].map((row) => (
            <span className="island-skeleton-row" key={row} />
          ))}
        </div>
      ) : props.results.length === 0 ? (
        <EmptyState
          title="No browser history yet."
          hint="Bookmarks and history appear after the browser source is indexed."
        />
      ) : (
        <>
          <div className="island-context-row">
            <span>BOOKMARKS</span>
            <span>Saved pages</span>
          </div>
          <div className="island-choose-list">{bookmarks.slice(0, 2).map(chooseRow)}</div>
          <div className="island-context-row">
            <span>RECENT HISTORY</span>
            <span>Local</span>
          </div>
          <div className="island-choose-list">{history.slice(0, 1).map(chooseRow)}</div>
        </>
      )}
    </div>
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
      subtitle="Recent commits · ProtonSearch"
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

type TodoFilter = "active" | "today" | "high" | "done" | "all";
type Priority = "low" | "medium" | "high";

const PRIORITY_ORDER: Record<Priority, number> = {
  high: 0,
  medium: 1,
  low: 2,
};

const NEXT_PRIORITY: Record<Priority, Priority> = {
  low: "medium",
  medium: "high",
  high: "low",
};

function todayIso(): string {
  const now = new Date();
  const yyyy = now.getFullYear();
  const mm = String(now.getMonth() + 1).padStart(2, "0");
  const dd = String(now.getDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}`;
}

function tomorrowIso(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}`;
}

function parseQuickTodoInput(
  raw: string,
  fallbackPriority: Priority,
  fallbackDue: string | undefined
): { title: string; priority: Priority; dueDate?: string } {
  let priority: Priority = fallbackPriority;
  let dueDate: string | undefined = fallbackDue;
  let text = raw.trim();

  text = text.replace(/\B!(high|med|medium|low)\b/gi, (_, p: string) => {
    const norm = p.toLowerCase();
    priority = norm === "med" ? "medium" : (norm as Priority);
    return "";
  });

  text = text.replace(/\B@(today|tomorrow|\d{4}-\d{2}-\d{2})\b/gi, (_, d: string) => {
    const norm = d.toLowerCase();
    if (norm === "today") dueDate = todayIso();
    else if (norm === "tomorrow") dueDate = tomorrowIso();
    else dueDate = norm;
    return "";
  });

  return {
    title: text.replace(/\s{2,}/g, " ").trim(),
    priority,
    dueDate,
  };
}

function nowDateTimeLocal(): string {
  const now = new Date();
  const yyyy = now.getFullYear();
  const mm = String(now.getMonth() + 1).padStart(2, "0");
  const dd = String(now.getDate()).padStart(2, "0");
  const hh = String(now.getHours()).padStart(2, "0");
  const min = String(now.getMinutes()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}T${hh}:${min}`;
}

function formatDueBadge(dueDate?: string | null): { label: string; overdue: boolean; isToday: boolean } | null {
  if (!dueDate) return null;
  const today = todayIso();
  const tomorrow = tomorrowIso();
  const datePart = dueDate.slice(0, 10);
  const timePart = dueDate.includes("T") ? dueDate.slice(11, 16) : null;
  const timeSuffix = timePart ? ` ${timePart}` : "";

  if (datePart === today) {
    return { label: `Today${timeSuffix}`, overdue: false, isToday: true };
  }
  if (datePart === tomorrow) {
    return { label: `Tomorrow${timeSuffix}`, overdue: false, isToday: false };
  }
  if (datePart < today) {
    return { label: `Overdue · ${datePart.slice(5)}${timeSuffix}`, overdue: true, isToday: false };
  }
  return { label: `${datePart.slice(5)}${timeSuffix}`, overdue: false, isToday: false };
}

export function TodoCard({ onOpenMain: _onOpenMain }: { onOpenMain?: () => void }) {
  const { todos, error, addTodo, toggleTodo, updateTodo, deleteTodo, clearCompleted } = useTodos();
  const [draft, setDraft] = useState("");
  const [priority, setPriority] = useState<Priority>("medium");
  const [dueDateTime, setDueDateTime] = useState<string>("");
  const [filter, setFilter] = useState<TodoFilter>("active");
  const [selectedIndex, setSelectedIndex] = useState<number>(0);
  const [editingDueId, setEditingDueId] = useState<string | null>(null);

  const today = todayIso();
  const activeTodos = useMemo(() => todos.filter((t) => !t.completed), [todos]);
  const completedCount = todos.length - activeTodos.length;

  const filteredTodos = useMemo(() => {
    const subset = todos.filter((t) => {
      if (filter === "active") return !t.completed;
      if (filter === "done") return t.completed;
      if (filter === "today") return !t.completed && (t.dueDate?.startsWith(today) ?? false);
      if (filter === "high") return !t.completed && t.priority === "high";
      return true;
    });

    return [...subset].sort((a, b) => {
      if (a.completed !== b.completed) return a.completed ? 1 : -1;
      if (Boolean(a.pinned) !== Boolean(b.pinned)) return a.pinned ? -1 : 1;
      const pa = PRIORITY_ORDER[(a.priority ?? "medium") as Priority] ?? 1;
      const pb = PRIORITY_ORDER[(b.priority ?? "medium") as Priority] ?? 1;
      if (pa !== pb) return pa - pb;
      if (a.dueDate && b.dueDate && a.dueDate !== b.dueDate) {
        return a.dueDate.localeCompare(b.dueDate);
      }
      if (a.dueDate && !b.dueDate) return -1;
      if (!a.dueDate && b.dueDate) return 1;
      return b.createdAt - a.createdAt;
    });
  }, [todos, filter, today]);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!draft.trim()) return;
    const parsed = parseQuickTodoInput(draft, priority, dueDateTime || undefined);
    if (!parsed.title) return;
    if (!(await addTodo(parsed.title, parsed.dueDate, parsed.priority))) return;
    setDraft("");
    setDueDateTime("");
    if (filter === "done") setFilter("active");
  };

  const cycleItemPriority = (todo: TodoItem) => {
    const current = (todo.priority ?? "medium") as Priority;
    updateTodo(todo.id, { priority: NEXT_PRIORITY[current] });
  };

  const togglePin = (todo: TodoItem) => {
    updateTodo(todo.id, { pinned: !todo.pinned });
  };

  const handleListKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (filteredTodos.length === 0) return;
    const clampedIdx = Math.min(selectedIndex, filteredTodos.length - 1);
    const activeItem = filteredTodos[clampedIdx];
    if (!activeItem) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => (i + 1) % filteredTodos.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => (i - 1 + filteredTodos.length) % filteredTodos.length);
    } else if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      toggleTodo(activeItem.id);
    } else if (e.key.toLowerCase() === "p") {
      e.preventDefault();
      cycleItemPriority(activeItem);
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      deleteTodo(activeItem.id);
    }
  };

  const filterTabs: { id: TodoFilter; label: string }[] = [
    { id: "active", label: "Active" },
    { id: "today", label: "Today" },
    { id: "high", label: "High" },
    { id: "done", label: "Done" },
    { id: "all", label: "All" },
  ];

  return (
    <div className="island-card island-todo-card">
      <CardHead card="bell" title="To-do" subtitle="Tasks & Reminders" />

      {error ? (
        <p role="alert" className="island-todo-error">
          {error}
        </p>
      ) : null}

      <div className="island-todo-subbar">
        <div className="island-todo-filters" role="tablist" aria-label="Filter tasks">
          {filterTabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={filter === tab.id}
              className={"island-todo-filter-pill" + (filter === tab.id ? " is-active" : "")}
              onClick={() => {
                setFilter(tab.id);
                setSelectedIndex(0);
              }}
            >
              <span>{tab.label}</span>
            </button>
          ))}
        </div>

        {completedCount > 0 ? (
          <button
            type="button"
            className="island-todo-clear-btn"
            onClick={() => clearCompleted()}
            title="Clear completed tasks"
          >
            Clear completed ({completedCount})
          </button>
        ) : null}
      </div>

      <div className="island-divider" />

      <form className="island-todo-add" onSubmit={handleSubmit}>
        <div className="island-todo-input-row">
          <input
            className="island-search-input island-todo-input"
            placeholder="Add task… (try !high @today)"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
          />
          <button
            type="button"
            title={`Priority: ${priority} (click to cycle)`}
            className={`island-todo-prio-toggle prio-${priority}`}
            onClick={() => setPriority((p) => NEXT_PRIORITY[p])}
          >
            {priority === "high" ? "High" : priority === "medium" ? "Med" : "Low"}
          </button>
          <div className="island-todo-due-wrap" title="Set due date & time">
            <input
              type="datetime-local"
              className="island-todo-dt-input"
              value={dueDateTime}
              onChange={(e) => setDueDateTime(e.target.value)}
              aria-label="Due date and time"
            />
            <span className={"island-todo-dt-badge" + (dueDateTime ? " is-set" : "")}>
              {dueDateTime ? dueDateTime.replace("T", " ") : "Due date/time"}
            </span>
          </div>
          {dueDateTime ? (
            <button
              type="button"
              className="island-todo-clear-dt-btn"
              onClick={() => setDueDateTime("")}
              title="Clear date & time"
            >
              ×
            </button>
          ) : null}
          <button type="submit" className="island-todo-submit-btn" aria-label="Add task">
            +
          </button>
        </div>
      </form>

      <div
        className="island-todo-list island-scrollable"
        tabIndex={0}
        onKeyDown={handleListKeyDown}
        aria-label="Task list"
      >
        {filteredTodos.length === 0 ? (
          <div className="island-todo-empty">
            <p className="island-todo-empty-title">
              {todos.length === 0 ? "No tasks yet" : `No ${filter} tasks`}
            </p>
            <p className="island-todo-empty-hint">
              {todos.length === 0
                ? "Type above and press Enter to add your first task."
                : "Switch filter tabs above or add a new task."}
            </p>
          </div>
        ) : (
          filteredTodos.map((todo, idx) => {
            const prio = (todo.priority ?? "medium") as Priority;
            const dueBadge = formatDueBadge(todo.dueDate);
            const isSelected = idx === selectedIndex;
            const isEditingDue = editingDueId === todo.id;

            return (
              <div
                key={todo.id}
                className={
                  "island-todo-row" +
                  ` prio-border-${prio}` +
                  (todo.completed ? " is-completed" : "") +
                  (isSelected ? " is-selected" : "")
                }
                onMouseEnter={() => setSelectedIndex(idx)}
              >
                <button
                  type="button"
                  role="checkbox"
                  aria-checked={todo.completed}
                  className={"island-todo-check" + (todo.completed ? " is-checked" : "")}
                  onClick={() => toggleTodo(todo.id)}
                >
                  {todo.completed ? "✓" : ""}
                </button>

                <div className="island-todo-main" onClick={() => toggleTodo(todo.id)}>
                  <span className="island-todo-title">{todo.title}</span>
                  {todo.notes ? (
                    <span className="island-todo-note-preview" title={todo.notes}>
                      {todo.notes}
                    </span>
                  ) : null}
                </div>

                <div className="island-todo-meta">
                  {isEditingDue ? (
                    <div className="island-todo-row-dt-editor">
                      <input
                        type="datetime-local"
                        className="island-todo-row-dt-input"
                        defaultValue={todo.dueDate ?? nowDateTimeLocal()}
                        autoFocus
                        onChange={(e) => {
                          updateTodo(todo.id, { dueDate: e.target.value || null });
                          setEditingDueId(null);
                        }}
                        onBlur={() => setEditingDueId(null)}
                      />
                    </div>
                  ) : (
                    <button
                      type="button"
                      title="Click to change due date & time"
                      className={
                        "island-todo-due-badge-btn" +
                        (dueBadge ? (dueBadge.overdue ? " is-overdue" : dueBadge.isToday ? " is-today" : "") : " is-empty")
                      }
                      onClick={() => setEditingDueId(todo.id)}
                    >
                      {dueBadge ? dueBadge.label : "+ Due"}
                    </button>
                  )}

                  <button
                    type="button"
                    title={`Priority: ${prio} (click to cycle)`}
                    className={`island-todo-prio-badge prio-${prio}`}
                    onClick={() => cycleItemPriority(todo)}
                  >
                    {prio === "medium" ? "med" : prio}
                  </button>

                  <button
                    type="button"
                    title={todo.pinned ? "Unpin task" : "Pin task to top"}
                    className={"island-todo-pin-btn" + (todo.pinned ? " is-pinned" : "")}
                    onClick={() => togglePin(todo)}
                    aria-label={todo.pinned ? "Unpin task" : "Pin task"}
                  >
                    ★
                  </button>

                  <button
                    type="button"
                    title="Delete task"
                    className="island-todo-del-btn"
                    onClick={() => deleteTodo(todo.id)}
                    aria-label="Delete task"
                  >
                    ×
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
