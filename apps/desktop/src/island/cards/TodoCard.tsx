import { useEffect, useMemo, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import { Check, ListTodo, Pin, Plus, X } from "lucide-react";
import { useTodos, type TodoItem } from "../../lib/todoStore";
import { cn } from "@/lib/utils";
import { ScrollArea } from "@/components/ui/scroll-area";
import { AnimatedTabs } from "../ui/AnimatedTabs";
import { HighlightRow } from "../ui/HighlightList";
import { CardEmpty, CardHeader } from "./CardChrome";
import {
  NEXT_PRIORITY,
  PRIORITY_LABEL,
  PRIORITY_ORDER,
  formatDueBadge,
  nowDateTimeLocal,
  parseQuickTodoInput,
  todayIso,
  type Priority,
} from "./todoRules";

type TodoFilter = "active" | "today" | "high" | "done" | "all";

const FILTER_TABS: ReadonlyArray<{ id: TodoFilter; label: string }> = [
  { id: "active", label: "Active" },
  { id: "today", label: "Today" },
  { id: "high", label: "High" },
  { id: "done", label: "Done" },
  { id: "all", label: "All" },
];

const ICON_BUTTON =
  "grid size-6 shrink-0 place-items-center rounded-[6px] text-fg-3 outline-none hover:bg-hover hover:text-fg focus-visible:ring-2 focus-visible:ring-hi";
const CHIP =
  "h-6 shrink-0 rounded-full px-2 text-[11px] tabular-nums outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-hi";

function visibleTodos(todos: TodoItem[], filter: TodoFilter, today: string): TodoItem[] {
  return todos
    .filter((todo) => {
      if (filter === "active") return !todo.completed;
      if (filter === "done") return todo.completed;
      if (filter === "today") return !todo.completed && (todo.dueDate?.startsWith(today) ?? false);
      if (filter === "high") return !todo.completed && todo.priority === "high";
      return true;
    })
    .sort((a, b) => {
      if (a.completed !== b.completed) return a.completed ? 1 : -1;
      if (Boolean(a.pinned) !== Boolean(b.pinned)) return a.pinned ? -1 : 1;
      const pa = PRIORITY_ORDER[(a.priority ?? "medium") as Priority] ?? 1;
      const pb = PRIORITY_ORDER[(b.priority ?? "medium") as Priority] ?? 1;
      if (pa !== pb) return pa - pb;
      if (a.dueDate && b.dueDate && a.dueDate !== b.dueDate) return a.dueDate.localeCompare(b.dueDate);
      if (a.dueDate && !b.dueDate) return -1;
      if (!a.dueDate && b.dueDate) return 1;
      return b.createdAt - a.createdAt;
    });
}

export function TodoCard({ onOpenMain: _onOpenMain }: { onOpenMain?: () => void }) {
  const { todos, error, addTodo, toggleTodo, updateTodo, deleteTodo } = useTodos();
  const [draft, setDraft] = useState("");
  const [priority, setPriority] = useState<Priority>("medium");
  const [dueDateTime, setDueDateTime] = useState("");
  const [dueOpen, setDueOpen] = useState(false);
  const [filter, setFilter] = useState<TodoFilter>("active");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [editingDueId, setEditingDueId] = useState<string | null>(null);
  const draftRef = useRef<HTMLInputElement>(null);

  // Ready to type a task as soon as the card is shown.
  useEffect(() => draftRef.current?.focus({ preventScroll: true }), []);

  const today = todayIso();
  const openCount = useMemo(() => todos.filter((todo) => !todo.completed).length, [todos]);
  const visible = useMemo(() => visibleTodos(todos, filter, today), [todos, filter, today]);
  const current = Math.min(selectedIndex, Math.max(0, visible.length - 1));

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    const parsed = parseQuickTodoInput(draft, priority, dueDateTime || undefined);
    if (!parsed.title) return;
    if (!(await addTodo(parsed.title, parsed.dueDate, parsed.priority))) return;
    setDraft("");
    setDueDateTime("");
    setDueOpen(false);
    if (filter === "done") setFilter("active");
  };

  const cyclePriority = (todo: TodoItem) => {
    updateTodo(todo.id, { priority: NEXT_PRIORITY[(todo.priority ?? "medium") as Priority] });
  };

  const handleListKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    // The row's due-date input sits inside this list; its keys belong to the input, not the row.
    if (event.target instanceof HTMLInputElement) return;
    const item = visible[current];
    if (!item) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setSelectedIndex((current + step + visible.length) % visible.length);
    } else if (event.key === " " || event.key === "Enter") {
      event.preventDefault();
      toggleTodo(item.id);
    } else if (event.key.toLowerCase() === "p") {
      event.preventDefault();
      cyclePriority(item);
    } else if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      deleteTodo(item.id);
    }
  };

  return (
    <div className="flex max-h-[460px] min-h-0 flex-1 flex-col gap-3 p-4">
      <CardHeader
        icon={ListTodo}
        title="To-do"
        subtitle={`${openCount} open`}
      />
      {error ? <p role="alert" className="line-clamp-2 text-[12px] text-danger">{error}</p> : null}

      <AnimatedTabs
        tabs={FILTER_TABS}
        value={filter}
        onChange={(id) => {
          setFilter(id);
          setSelectedIndex(0);
        }}
        label="Filter tasks"
        className="flex gap-1 rounded-[10px] bg-raised p-1"
        itemClassName="h-7 min-w-0 flex-1 rounded-[7px] px-2 text-[12px] text-fg-3 outline-none hover:text-fg focus-visible:ring-2 focus-visible:ring-hi [&.is-active]:text-fg"
      />

      <form onSubmit={handleSubmit} className="flex flex-wrap items-center gap-2">
        <input
          ref={draftRef}
          aria-label="New task"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Add a task · !high @today"
          className="h-8 min-w-0 flex-1 rounded-[10px] border border-line bg-raised px-3 text-[13px] text-fg outline-none placeholder:text-fg-3 focus-visible:ring-2 focus-visible:ring-hi"
        />
        <button
          type="button"
          title={`Priority: ${priority} (click to cycle)`}
          onClick={() => setPriority((p) => NEXT_PRIORITY[p])}
          className={cn(CHIP, "h-8 border border-line px-2.5 text-fg-2")}
        >
          {PRIORITY_LABEL[priority]}
        </button>
        <button
          type="button"
          title="Set due date & time"
          aria-expanded={dueOpen}
          onClick={() => setDueOpen((open) => !open)}
          className={cn(CHIP, "h-8 border border-line px-2.5 text-fg-2")}
        >
          {dueDateTime ? formatDueBadge(dueDateTime)?.label : "Due"}
        </button>
        {dueDateTime ? (
          <button type="button" aria-label="Clear due date" title="Clear date & time" onClick={() => setDueDateTime("")} className={ICON_BUTTON}>
            <X size={13} />
          </button>
        ) : null}
        <button type="submit" aria-label="Add task" className={cn(ICON_BUTTON, "size-8 border border-line text-fg")}>
          <Plus size={14} />
        </button>
        {dueOpen ? (
          <input
            type="datetime-local"
            aria-label="Due date and time"
            autoFocus
            value={dueDateTime}
            onChange={(event) => setDueDateTime(event.target.value)}
            className="h-8 w-full rounded-[10px] border border-line bg-raised px-3 text-[13px] text-fg outline-none focus-visible:ring-2 focus-visible:ring-hi"
          />
        ) : null}
      </form>

      {visible.length === 0 ? (
        <CardEmpty
          title={todos.length === 0 ? "No tasks yet" : `No ${filter} tasks`}
          hint={
            todos.length === 0
              ? "Type above and press Enter to add your first task."
              : "Switch filter tabs above or add a new task."
          }
        />
      ) : (
        <ScrollArea className="min-h-0 flex-1">
          <div
            role="group"
            aria-label="Task list"
            tabIndex={0}
            onKeyDown={handleListKeyDown}
            className="flex flex-col gap-1 outline-none"
          >
            {visible.map((todo, index) => {
              const due = formatDueBadge(todo.dueDate);
              const prio = (todo.priority ?? "medium") as Priority;
              return (
                <HighlightRow
                  key={todo.id}
                  layoutId="todo-highlight"
                  active={index === current}
                  onHover={() => setSelectedIndex(index)}
                  className="rounded-[10px]"
                >
                  <div className="flex min-h-9 items-center gap-2 px-2.5 py-1">
                    <button
                      type="button"
                      role="checkbox"
                      aria-checked={todo.completed}
                      aria-label={todo.title}
                      onClick={() => toggleTodo(todo.id)}
                      className={cn(
                        "grid size-4 shrink-0 place-items-center rounded-[5px] border outline-none focus-visible:ring-2 focus-visible:ring-hi",
                        todo.completed ? "border-hi bg-hi text-surface" : "border-fg-3",
                      )}
                    >
                      {todo.completed ? <Check size={11} strokeWidth={2.5} /> : null}
                    </button>
                    <div className="min-w-0 flex-1 cursor-pointer" onClick={() => toggleTodo(todo.id)}>
                      <p className={cn("truncate text-[13px] leading-5 text-fg", todo.completed && "text-fg-3 line-through")}>
                        {todo.title}
                      </p>
                      {todo.notes ? <p className="truncate text-[12px] leading-4 text-fg-3">{todo.notes}</p> : null}
                    </div>
                    {editingDueId === todo.id ? (
                      // No onBlur close: the native picker can take focus before its change lands.
                      <input
                        type="datetime-local"
                        aria-label="Due date and time for task"
                        autoFocus
                        defaultValue={todo.dueDate ?? nowDateTimeLocal()}
                        onChange={(event) => updateTodo(todo.id, { dueDate: event.target.value || null })}
                        className="h-6 w-40 rounded-[6px] border border-line bg-raised px-1.5 text-[11px] text-fg outline-none focus-visible:ring-2 focus-visible:ring-hi"
                      />
                    ) : null}
                    <button
                      type="button"
                      title="Change due date & time"
                      aria-expanded={editingDueId === todo.id}
                      onClick={() => setEditingDueId(editingDueId === todo.id ? null : todo.id)}
                      className={cn(CHIP, due?.overdue ? "text-danger" : due?.isToday ? "text-warn" : "text-fg-3")}
                    >
                      {due ? due.label : "+ Due"}
                    </button>
                    <button
                      type="button"
                      title={`Priority: ${prio} (click to cycle)`}
                      onClick={() => cyclePriority(todo)}
                      className={cn(CHIP, "text-fg-3")}
                    >
                      {PRIORITY_LABEL[prio]}
                    </button>
                    <button
                      type="button"
                      aria-label={todo.pinned ? "Unpin task" : "Pin task"}
                      title={todo.pinned ? "Unpin task" : "Pin task to top"}
                      onClick={() => updateTodo(todo.id, { pinned: !todo.pinned })}
                      className={cn(ICON_BUTTON, todo.pinned && "text-hi")}
                    >
                      <Pin size={13} fill={todo.pinned ? "currentColor" : "none"} />
                    </button>
                    <button
                      type="button"
                      aria-label="Delete task"
                      title="Delete task"
                      onClick={() => deleteTodo(todo.id)}
                      className={cn(ICON_BUTTON, "hover:text-danger")}
                    >
                      <X size={13} />
                    </button>
                  </div>
                </HighlightRow>
              );
            })}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
