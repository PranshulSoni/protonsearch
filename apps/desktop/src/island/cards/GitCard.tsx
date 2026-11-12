import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Check, ChevronDown, FolderGit2, GitBranch, GitCommitHorizontal, type LucideIcon } from "lucide-react";
import { api } from "../../bridge/api";
import type { SearchResult } from "../../bridge/types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { relativeTime } from "../islandData";
import { HighlightRow } from "../ui/HighlightList";
import { CardEmpty, CardHeader } from "./CardChrome";
import { DrillHeader, Pane } from "./controlsShared";

/** Commit subtitles end with the repo path, which the project picker already names. */
function detail(result: SearchResult): string {
  const text = result.meta?.label || result.subtitle;
  const path = result.path;
  const trimmed = path && text.endsWith(` - ${path}`) ? text.slice(0, -(path.length + 3)) : text;
  return trimmed.replace(/ - /g, " · ");
}

/** The checked-out branch, from the repository row's subtitle ("Repository - main"). */
function headBranch(repo: SearchResult | undefined): string | null {
  return repo?.subtitle.match(/^Repository - (.+)$/)?.[1] ?? null;
}

const rowClass =
  "flex w-full min-w-0 items-center gap-3 rounded-[10px] px-2.5 py-2 text-left outline-none focus-visible:ring-2 focus-visible:ring-hi";

function Picker({
  icon: Icon,
  label,
  value,
  onClick,
  className,
}: {
  icon: LucideIcon;
  label: string;
  value: string;
  onClick: () => void;
  className?: string;
}) {
  return (
    <button
      type="button"
      aria-label={`${label}: ${value}`}
      onClick={onClick}
      className={cn(
        "flex h-9 min-w-0 items-center gap-2 rounded-[10px] border border-line bg-raised px-2.5 text-[13px] text-fg outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-hi",
        className,
      )}
    >
      <Icon size={15} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-2" />
      <span className="min-w-0 flex-1 truncate text-left">{value}</span>
      <ChevronDown size={14} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-3" />
    </button>
  );
}

function Choices({
  title,
  items,
  onPick,
  onBack,
}: {
  title: string;
  items: Array<{ id: string; label: string; hint?: string; current: boolean }>;
  onPick: (id: string) => void;
  onBack: () => void;
}) {
  return (
    <Pane onEscape={onBack}>
      <DrillHeader title={title} onBack={onBack} />
      <ScrollArea className="min-h-0 flex-1">
        <div role="group" aria-label={title} className="flex flex-col gap-1">
          {items.map((item) => (
            <button
              key={item.id}
              type="button"
              aria-current={item.current || undefined}
              onClick={() => onPick(item.id)}
              className={cn(rowClass, "hover:bg-hover")}
            >
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[13px] font-medium text-fg">{item.label}</span>
                {item.hint ? <span className="block truncate text-[12px] text-fg-3">{item.hint}</span> : null}
              </span>
              {item.current ? <Check size={15} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-hi" /> : null}
            </button>
          ))}
        </div>
      </ScrollArea>
    </Pane>
  );
}

const skeleton = (
  <div aria-hidden="true" className="flex flex-col gap-2">
    {[0, 1, 2].map((row) => (
      <span key={row} className="h-10 rounded-[10px] bg-raised" />
    ))}
  </div>
);

export function GitCard({
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
  const repos = useMemo(() => results.filter((result) => result.kind === "git_repository"), [results]);
  const [repoPath, setRepoPath] = useState<string | null>(null);
  // null = the checked-out branch, whose commits are already indexed.
  const [branch, setBranch] = useState<string | null>(null);
  const [view, setView] = useState<"commits" | "projects" | "branches">("commits");
  const [loaded, setLoaded] = useState<{ key: string; rows: SearchResult[] } | null>(null);
  const [picked, setPicked] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  const repo = repos.find((result) => result.path === repoPath) ?? repos[0];
  const head = headBranch(repo);
  const branches = repo?.meta?.branches?.length ? repo.meta.branches : head ? [head] : [];
  const logKey = `${repo?.path ?? ""}\n${branch ?? ""}`;

  // Any other branch is read on demand.
  useEffect(() => {
    if (!repo?.path || !branch) return;
    let live = true;
    const done = (rows: SearchResult[]) => {
      if (live) setLoaded({ key: logKey, rows });
    };
    api.gitBranchCommits(repo.path, branch).then(done, () => done([]));
    return () => {
      live = false;
    };
  }, [logKey]);

  const commits = useMemo(
    () =>
      !branch
        ? results.filter((result) => result.kind === "git_commit" && result.path === repo?.path)
        : loaded?.key === logKey
          ? loaded.rows
          : null,
    [branch, loaded, logKey, repo?.path, results],
  );
  const rows = commits ?? [];
  const current = Math.min(picked, Math.max(0, rows.length - 1));
  const hasRows = view === "commits" && rows.length > 0;

  // Focus the list so the arrow keys work as soon as the card is shown.
  useEffect(() => {
    if (hasRows) listRef.current?.focus({ preventScroll: true });
  }, [hasRows]);

  const choose = (nextRepo: string | null, nextBranch: string | null) => {
    setRepoPath(nextRepo);
    setBranch(nextBranch);
    setPicked(0);
    setView("commits");
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const result = rows[current];
    if (!result) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setPicked((current + step + rows.length) % rows.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      onOpen(result);
    }
  };

  let body;
  if (error) {
    body = <CardEmpty title="This source is unavailable." hint={error} />;
  } else if (pending && results.length === 0) {
    body = skeleton;
  } else if (!repo) {
    body = <CardEmpty title="No Git activity yet." hint="Enable Git indexing in Preferences to see commits here." />;
  } else if (view === "projects") {
    body = (
      <Choices
        title="Projects"
        items={repos.map((result) => {
          const count = result.meta?.branches?.length ?? 0;
          return {
            id: result.path ?? result.id,
            label: result.title,
            hint: [headBranch(result), count > 1 ? `${count} branches` : null].filter(Boolean).join(" · "),
            current: result === repo,
          };
        })}
        onPick={(path) => choose(path, null)}
        onBack={() => setView("commits")}
      />
    );
  } else if (view === "branches") {
    body = (
      <Choices
        title="Branches"
        items={branches.map((name) => ({
          id: name,
          label: name,
          hint: name === head ? "Checked out" : undefined,
          current: name === (branch ?? head),
        }))}
        onPick={(name) => choose(repo.path ?? null, name === head ? null : name)}
        onBack={() => setView("commits")}
      />
    );
  } else {
    body = (
      <>
        <div className="flex gap-2">
          <Picker icon={FolderGit2} label="Project" value={repo.title} onClick={() => setView("projects")} className="flex-1" />
          <Picker
            icon={GitBranch}
            label="Branch"
            value={branch ?? head ?? "Detached"}
            onClick={() => setView("branches")}
            className="max-w-[45%]"
          />
        </div>
        {commits === null ? (
          skeleton
        ) : rows.length === 0 ? (
          <p className="px-1 text-[12px] text-fg-3">No commits on this branch.</p>
        ) : (
          <ScrollArea className="min-h-0 flex-1">
            <div
              ref={listRef}
              role="group"
              aria-label="Git activity"
              tabIndex={0}
              onKeyDown={handleKeyDown}
              className="flex flex-col gap-1 outline-none"
            >
              {rows.map((result, index) => {
                const time = relativeTime(result.meta?.modified);
                return (
                  <HighlightRow
                    key={result.id}
                    layoutId="git-highlight"
                    active={index === current}
                    onHover={() => setPicked(index)}
                    className="rounded-[10px]"
                  >
                    <button
                      type="button"
                      onClick={() => {
                        setPicked(index);
                        onOpen(result);
                      }}
                      className={rowClass}
                    >
                      <span className="grid size-8 shrink-0 place-items-center rounded-[10px] border border-line bg-raised text-fg-2">
                        <GitCommitHorizontal size={15} strokeWidth={1.75} />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-[13px] font-medium text-fg">{result.title}</span>
                        <span className="block truncate text-[12px] text-fg-3">{detail(result)}</span>
                      </span>
                      {time ? <span className="shrink-0 text-[12px] tabular-nums text-fg-3">{time}</span> : null}
                    </button>
                  </HighlightRow>
                );
              })}
            </div>
          </ScrollArea>
        )}
      </>
    );
  }

  return (
    <div className="flex max-h-[460px] min-h-0 flex-1 flex-col gap-3 p-4">
      {view === "commits" || !repo ? (
        <CardHeader
          icon={GitBranch}
          title="Git"
          subtitle={repos.length ? `${repos.length} ${repos.length === 1 ? "project" : "projects"}` : "Recent commits"}
        />
      ) : null}
      {body}
    </div>
  );
}
