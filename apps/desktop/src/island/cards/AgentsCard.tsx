import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { ArrowLeft, ArrowUp, Bot, ChevronDown, Pause, Pencil, Play, Trash2 } from "lucide-react";
import { api } from "../../bridge/api";
import type { AgentModel, AgentStatus, AgentTask, AgentTaskRuntime, AgentTaskState } from "../../bridge/types";
import { buildAgentRunRequest, createAgentRunId } from "../../app/agentRun";
import { cn } from "@/lib/utils";
import { ScrollArea } from "@/components/ui/scroll-area";
import { HighlightRow } from "../ui/HighlightList";
import { CardEmpty, CardHeader } from "./CardChrome";
import { Pane, iconButton } from "./controlsShared";

// Red = failed, pulsing orange = working, green = done.
const STATE: Record<AgentTaskState, { label: string; dot: string }> = {
  queued: { label: "Working", dot: "bg-warn animate-pulse" },
  running: { label: "Working", dot: "bg-warn animate-pulse" },
  completed: { label: "Done", dot: "bg-ok" },
  failed: { label: "Failed", dot: "bg-danger" },
  cancelled: { label: "Cancelled", dot: "bg-fg-3" },
  offline: { label: "Offline", dot: "bg-fg-3" },
};

const RUNTIME_TILE: Record<AgentTaskRuntime, string> = {
  pi: "Pi",
  pydantic: "Py",
  unknown: "?",
};

function StatusDot({ state }: { state: AgentTaskState }) {
  const { label, dot } = STATE[state];
  return <span role="img" aria-label={label} title={label} className={cn("size-2 shrink-0 rounded-full", dot)} />;
}

/** A thread is the task that started it plus the follow-ups sent from its chat, oldest first. */
function threadOf(root: AgentTask, tasks: AgentTask[]): AgentTask[] {
  return [root, ...tasks.filter((task) => task.parentId === root.id).sort((a, b) => a.createdAt - b.createdAt)];
}

function sameModel(a: AgentModel | null, b: AgentModel | null): boolean {
  return a?.provider === b?.provider && a?.id === b?.id;
}

function Composer({
  placeholder,
  busy,
  models,
  model,
  onModel,
  onSend,
}: {
  placeholder: string;
  busy: boolean;
  models: AgentModel[];
  model: AgentModel | null;
  onModel: (model: AgentModel | null) => void;
  onSend: (text: string) => Promise<boolean>;
}) {
  const [text, setText] = useState("");
  const [choosing, setChoosing] = useState(false);
  // The model list's filter, and the highlighted row that Enter picks.
  const [filter, setFilter] = useState("");
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const query = filter.trim().toLowerCase();
  const shown = [null, ...models].filter((option) =>
    option ? `${option.provider}/${option.id}`.toLowerCase().includes(query) : "default".includes(query),
  );
  const highlighted = Math.min(active, shown.length - 1);
  useEffect(() => {
    listRef.current?.querySelector('[data-active="true"]')?.scrollIntoView({ block: "nearest" });
  }, [highlighted]);
  const toggle = () => {
    setFilter("");
    setActive(0);
    setChoosing((open) => !open);
  };
  const pickModel = (option: AgentModel | null) => {
    onModel(option);
    setChoosing(false);
  };
  const filterKeys = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setActive((index) => Math.min(Math.max(index + step, 0), shown.length - 1));
    } else if (event.key === "Enter" && shown.length > 0) {
      event.preventDefault();
      pickModel(shown[highlighted]);
    }
  };
  // Escape closes the open list only, not the chat behind it.
  const closeOnEscape = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    setChoosing(false);
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const value = text.trim();
    if (value && !busy && (await onSend(value))) setText("");
  };
  return (
    <div className="flex flex-col gap-1.5">
      <form
        onSubmit={submit}
        className="flex items-center gap-2 rounded-[14px] border border-line bg-raised p-1.5 pl-3 focus-within:border-hi/60"
      >
        <input
          aria-label={placeholder}
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder={placeholder}
          className="h-7 min-w-0 flex-1 bg-transparent text-[13px] text-fg outline-none placeholder:text-fg-3"
        />
        <button
          type="button"
          aria-haspopup="listbox"
          aria-expanded={choosing}
          onClick={toggle}
          className="flex h-6 max-w-[8rem] shrink-0 items-center gap-1 rounded-full px-2 text-[12px] text-fg-2 outline-none hover:text-fg focus-visible:ring-2 focus-visible:ring-hi"
        >
          <span className="truncate">{model?.id ?? "Default"}</span>
          <ChevronDown size={12} strokeWidth={2} aria-hidden="true" />
        </button>
        <button
          type="submit"
          aria-label="Send"
          disabled={busy || !text.trim()}
          className="grid size-7 shrink-0 place-items-center rounded-full bg-hi text-[#0b0d0e] outline-none focus-visible:ring-2 focus-visible:ring-hi disabled:opacity-40"
        >
          <ArrowUp size={15} strokeWidth={2} aria-hidden="true" />
        </button>
      </form>
      {choosing ? (
        <div
          role="listbox"
          aria-label="Models"
          onKeyDown={closeOnEscape}
          className="flex max-h-48 flex-col gap-1 rounded-[12px] border border-line bg-raised p-1"
        >
          <input
            aria-label="Filter models"
            autoFocus
            value={filter}
            onChange={(event) => {
              setFilter(event.target.value);
              setActive(0);
            }}
            onKeyDown={filterKeys}
            placeholder="Filter models…"
            className="h-7 shrink-0 rounded-[8px] bg-transparent px-2.5 text-[12px] text-fg outline-none placeholder:text-fg-3"
          />
          <div ref={listRef} className="flex min-h-0 flex-col gap-0.5 overflow-y-auto">
            {shown.map((option, index) => (
              <button
                key={option ? `${option.provider}/${option.id}` : "none"}
                type="button"
                role="option"
                aria-selected={sameModel(option, model)}
                data-active={index === highlighted}
                onClick={() => pickModel(option)}
                className="flex items-center justify-between gap-3 rounded-[8px] px-2.5 py-1.5 text-left text-[12px] text-fg outline-none hover:bg-hi/10 focus-visible:ring-2 focus-visible:ring-hi aria-selected:text-hi data-[active=true]:bg-hi/10"
              >
                <span className="truncate">{option?.id ?? "Default"}</span>
                {option ? <span className="shrink-0 text-fg-3">{option.provider}</span> : null}
              </button>
            ))}
            {shown.length === 0 ? <p className="px-2.5 py-1.5 text-[12px] text-fg-3">No matching models.</p> : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}

/** One exchange: the message, Pi's steps (collapsed), then its answer or error. */
function Turn({ task }: { task: AgentTask }) {
  const working = task.state === "running" || task.state === "queued";
  const steps = task.log.filter((entry) => entry.level !== "error");
  return (
    <div className="flex flex-col gap-2">
      <p className="ml-10 self-end whitespace-pre-wrap rounded-[14px] bg-raised px-3 py-2 text-[13px] text-fg">
        {task.promptSummary || task.title}
      </p>
      {working || steps.length > 0 ? (
        <details className="text-[12px] text-fg-3">
          <summary className="flex cursor-pointer list-none items-center gap-2 outline-none focus-visible:text-fg">
            {working ? <span aria-hidden="true" className="size-1.5 rounded-full bg-warn animate-pulse" /> : null}
            {working ? "Thinking…" : `${steps.length} ${steps.length === 1 ? "step" : "steps"}`}
          </summary>
          <ol className="mt-1.5 flex flex-col gap-1 border-l border-line pl-3">
            {steps.map((entry, index) => (
              <li key={index} className="truncate">
                {entry.message}
              </li>
            ))}
          </ol>
        </details>
      ) : null}
      {task.answer ? (
        <p className="whitespace-pre-wrap text-[13px] leading-5 text-fg">{task.answer}</p>
      ) : task.error ? (
        <p className="whitespace-pre-wrap text-[12px] text-danger">{task.error.message}</p>
      ) : task.state === "completed" ? (
        <p className="text-[12px] text-fg-3">The answer isn't kept after ProtonSearch restarts.</p>
      ) : null}
    </div>
  );
}

function AgentRow({
  thread,
  active,
  onHover,
  onOpen,
  onDelete,
}: {
  thread: AgentTask[];
  active: boolean;
  onHover: (id: string) => void;
  onOpen: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const root = thread[0];
  const last = thread[thread.length - 1];
  return (
    <HighlightRow layoutId="agent-roster-highlight" active={active} onHover={() => onHover(root.id)} className="rounded-[10px]">
      <div className="group flex items-center gap-1 pr-1">
        <button
          type="button"
          onClick={() => onOpen(root.id)}
          className="flex min-w-0 flex-1 items-center gap-3 rounded-[10px] px-2.5 py-2 text-left outline-none focus-visible:ring-2 focus-visible:ring-hi"
        >
          <span
            aria-hidden="true"
            className="grid size-8 shrink-0 place-items-center rounded-[10px] border border-line bg-raised text-[11px] font-semibold text-fg-2"
          >
            {RUNTIME_TILE[root.runtime]}
          </span>
          <span className="min-w-0 flex-1">
            <span className="flex min-w-0 items-center gap-2">
              <span className="truncate text-[13px] font-medium text-fg">{root.title}</span>
              <StatusDot state={last.state} />
            </span>
            <span className="block truncate text-[12px] text-fg-3">
              {last.answer || last.error?.message || last.promptSummary}
            </span>
          </span>
        </button>
        <button
          type="button"
          aria-label={`Delete ${root.title}`}
          onClick={() => onDelete(root.id)}
          className={cn(iconButton, "size-8 shrink-0 opacity-0 group-hover:opacity-100 focus-visible:opacity-100")}
        >
          <Trash2 size={15} strokeWidth={1.75} aria-hidden="true" />
        </button>
      </div>
    </HighlightRow>
  );
}

export function AgentsCard({
  status,
  tasks,
  onPause,
  onResume,
  onChanged,
}: {
  status: AgentStatus | null;
  tasks: AgentTask[];
  onPause: (taskId: string) => void;
  onResume: (taskId: string) => void;
  /** Reload the task list after a start or delete, so the change shows at once. */
  onChanged?: () => void;
}) {
  const [openId, setOpenId] = useState<string | null>(null);
  const [hoverId, setHoverId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The title being edited in the chat header; null when not renaming.
  const [renameDraft, setRenameDraft] = useState<string | null>(null);
  // Enter or Escape unmounts the input, which can fire one more blur; the first end wins.
  const renameEnded = useRef(true);
  // Models Pi can run, read when the card opens.
  const [models, setModels] = useState<AgentModel[]>([]);
  const [model, setModel] = useState<AgentModel | null>(null);
  useEffect(() => {
    let current = true;
    api.agentModels().then(
      (list) => {
        if (current) setModels(list);
      },
      () => {
        if (current) setModels([]);
      },
    );
    return () => {
      current = false;
    };
  }, []);
  const configured = status != null && status.state !== "not_configured";
  const roots = tasks.filter((task) => !task.parentId);
  const open = roots.find((task) => task.id === openId) ?? null;

  // A new agent has no parent; a chat message continues the open thread.
  const send = async (text: string, parentId: string | null): Promise<boolean> => {
    setBusy(true);
    setError(null);
    try {
      const modelPick = model ? `${model.provider}/${model.id}` : null;
      await api.agentStart(buildAgentRunRequest(createAgentRunId(), text, modelPick), parentId);
      onChanged?.();
      return true;
    } catch (reason) {
      setError(String(reason));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const remove = (id: string) => {
    if (openId === id) setOpenId(null);
    api.agentDelete(id).then(
      () => onChanged?.(),
      (reason) => setError(String(reason)),
    );
  };

  const errorLine = error ? (
    <p role="alert" className="line-clamp-2 text-[12px] text-danger">
      {error}
    </p>
  ) : null;

  if (!configured) {
    return (
      <div className="flex max-h-[460px] min-h-0 flex-1 flex-col gap-3 p-4">
        <CardHeader icon={Bot} title="Agents" subtitle="Not configured" />
        <CardEmpty title="No agent configured." hint="Set up Pi on the Agentic workflows page." />
      </div>
    );
  }

  if (open) {
    const thread = threadOf(open, tasks);
    const last = thread[thread.length - 1];
    const beginRename = () => {
      renameEnded.current = false;
      setRenameDraft(open.title);
    };
    // Saves a changed, non-blank title. Enter and blur save; Escape discards.
    const endRename = (save: boolean) => {
      if (renameEnded.current) return;
      renameEnded.current = true;
      const text = renameDraft?.trim() ?? "";
      setRenameDraft(null);
      if (save && text && text !== open.title) {
        api.agentRename(open.id, text).then(() => onChanged?.(), (reason) => setError(String(reason)));
      }
    };
    return (
      <div className="flex max-h-[460px] min-h-0 flex-1 flex-col gap-3 p-4">
        <Pane onEscape={() => setOpenId(null)}>
          <div className="flex items-center gap-2">
            <button type="button" aria-label="Back" autoFocus onClick={() => setOpenId(null)} className={cn(iconButton, "size-8 shrink-0")}>
              <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
            </button>
            {renameDraft === null ? (
              <>
                <h3 className="min-w-0 truncate text-[15px] font-semibold leading-5">{open.title}</h3>
                <button type="button" aria-label="Rename agent" onClick={beginRename} className={cn(iconButton, "size-8 shrink-0")}>
                  <Pencil size={15} strokeWidth={1.75} aria-hidden="true" />
                </button>
              </>
            ) : (
              <input
                aria-label="Agent title"
                autoFocus
                value={renameDraft}
                onChange={(event) => setRenameDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") endRename(true);
                  if (event.key === "Escape") {
                    // Stops the Pane's Escape from closing the chat.
                    event.stopPropagation();
                    endRename(false);
                  }
                }}
                onBlur={() => endRename(true)}
                className="h-8 min-w-0 flex-1 rounded-[10px] border border-line bg-raised px-2 text-[15px] font-semibold text-fg outline-none focus:border-hi/60"
              />
            )}
            <StatusDot state={last.state} />
            <span className="flex-1" />
            {last.controls.canPause ? (
              <button type="button" aria-label="Pause" onClick={() => onPause(last.id)} className={cn(iconButton, "size-8 shrink-0")}>
                <Pause size={15} strokeWidth={1.75} aria-hidden="true" />
              </button>
            ) : null}
            {last.controls.canResume ? (
              <button type="button" aria-label="Resume" onClick={() => onResume(last.id)} className={cn(iconButton, "size-8 shrink-0")}>
                <Play size={15} strokeWidth={1.75} aria-hidden="true" />
              </button>
            ) : null}
            <button type="button" aria-label="Delete agent" onClick={() => remove(open.id)} className={cn(iconButton, "size-8 shrink-0")}>
              <Trash2 size={15} strokeWidth={1.75} aria-hidden="true" />
            </button>
          </div>
          <ScrollArea className="min-h-0 flex-1">
            <div role="log" aria-label="Conversation" className="flex flex-col gap-4 pr-2">
              {thread.map((task) => (
                <Turn key={task.id} task={task} />
              ))}
            </div>
          </ScrollArea>
          {errorLine}
          <Composer
            placeholder="Message the agent…"
            busy={busy}
            models={models}
            model={model}
            onModel={setModel}
            onSend={(text) => send(text, open.id)}
          />
        </Pane>
      </div>
    );
  }

  return (
    <div className="flex max-h-[460px] min-h-0 flex-1 flex-col gap-3 p-4">
      <CardHeader icon={Bot} title="Agents" subtitle={status?.runtime ?? "Agents"} />
      <Composer
        placeholder="Ask the agent…"
        busy={busy}
        models={models}
        model={model}
        onModel={setModel}
        onSend={(text) => send(text, null)}
      />
      {errorLine}
      {roots.length === 0 ? (
        <CardEmpty title="No agents yet." hint="Ask the agent something above to start one." />
      ) : (
        <ScrollArea className="min-h-0 flex-1">
          <div role="group" aria-label="Agents" className="flex flex-col gap-1">
            {roots.map((root) => (
              <AgentRow
                key={root.id}
                thread={threadOf(root, tasks)}
                active={root.id === (hoverId ?? roots[0]?.id)}
                onHover={setHoverId}
                onOpen={setOpenId}
                onDelete={remove}
              />
            ))}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
