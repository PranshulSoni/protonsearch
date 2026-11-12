// To-do route. Storage and sync live in lib/todoStore; this file only lays them out.

import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { useState } from "react";
import { useTodos, type TodoItem } from "../../lib/todoStore";
import { SECTION_EYEBROWS } from "../sections";
import { Icon, RouteHeading } from "../ui";

type Filter = "all" | "active" | "today" | "done";
type Priority = "low" | "medium" | "high";

const FILTERS: { value: Filter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "active", label: "Active" },
  { value: "today", label: "Today" },
  { value: "done", label: "Done" },
];

const EMPTY_COPY: Record<Filter, { title: string; body: string }> = {
  all: { title: "No tasks yet.", body: "Add a task above. It stays on this PC." },
  active: { title: "No active tasks.", body: "Completed tasks are still listed under All." },
  today: { title: "Nothing due today.", body: "Tasks without a due date show here too." },
  done: { title: "Nothing done yet.", body: "Tick a task off and it moves here." },
};

const PRIORITY_LABEL: Record<Priority, string> = { low: "Low", medium: "Medium", high: "High" };

/** Local calendar day as YYYY-MM-DD. toISOString() is UTC and shifts the day after midnight. */
function dayKey(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return date.getFullYear() + "-" + month + "-" + day;
}

function dueLabel(dueDate: string, today: string, tomorrow: string): string {
  if (dueDate === today) return "Today";
  if (dueDate === tomorrow) return "Tomorrow";
  if (!/^\d{4}-\d{2}-\d{2}$/.test(dueDate)) return dueDate;
  const [year, month, day] = dueDate.split("-").map(Number);
  return new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" }).format(
    new Date(year, month - 1, day),
  );
}

function PencilIcon() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
    </svg>
  );
}

export default function TodoView() {
  const { todos, error, addTodo, toggleTodo, deleteTodo, updateTodo, clearCompleted } = useTodos();
  const reduceMotion = useReducedMotion();
  const [filter, setFilter] = useState<Filter>("all");
  const [newTitle, setNewTitle] = useState("");
  const [newPriority, setNewPriority] = useState<Priority>("medium");
  const [newDueDate, setNewDueDate] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [editNotes, setEditNotes] = useState("");

  const now = new Date();
  const today = dayKey(now);
  const tomorrow = dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1));
  // Reduced motion turns these into instant changes; the CSS durations are zeroed in tokens.css.
  const rowMotion = reduceMotion ? { duration: 0 } : { duration: 0.2, ease: "easeOut" as const };
  const pillMotion = reduceMotion ? { duration: 0 } : { type: "spring" as const, stiffness: 520, damping: 40 };

  const visible = todos.filter((todo) => {
    if (filter === "active") return !todo.completed;
    if (filter === "today") return todo.dueDate === today || (!todo.dueDate && !todo.completed);
    if (filter === "done") return todo.completed;
    return true;
  });
  const activeCount = todos.filter((todo) => !todo.completed).length;
  const completedCount = todos.length - activeCount;

  const handleAdd = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!newTitle.trim()) return;
    if (!(await addTodo(newTitle, newDueDate || undefined, newPriority))) return;
    setNewTitle("");
    setNewDueDate("");
    setNewPriority("medium");
  };

  const startEdit = (todo: TodoItem) => {
    setEditingId(todo.id);
    setEditText(todo.title);
    setEditNotes(todo.notes || "");
  };

  const saveEdit = (id: string) => {
    if (!editText.trim()) {
      deleteTodo(id);
    } else {
      updateTodo(id, { title: editText.trim(), notes: editNotes.trim() });
    }
    setEditingId(null);
  };

  const editRow = (todo: TodoItem) => (
    <motion.li
      key={todo.id}
      layout
      initial={reduceMotion ? false : { opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={rowMotion}
      className="todo-row"
    >
      <div className="todo-edit">
        <input
          className="input"
          value={editText}
          onChange={(event) => setEditText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") saveEdit(todo.id);
            if (event.key === "Escape") setEditingId(null);
          }}
          aria-label="Task"
          autoFocus
        />
        <input
          className="input"
          value={editNotes}
          onChange={(event) => setEditNotes(event.target.value)}
          placeholder="Notes"
          aria-label="Notes"
        />
        <div className="todo-edit-actions">
          <button type="button" className="btn btn-primary" onClick={() => saveEdit(todo.id)}>
            Save
          </button>
          <button type="button" className="btn" onClick={() => setEditingId(null)}>
            Cancel
          </button>
        </div>
      </div>
    </motion.li>
  );

  const taskRow = (todo: TodoItem) => {
    const overdue = !todo.completed && !!todo.dueDate && todo.dueDate < today;
    return (
      <motion.li
        key={todo.id}
        layout
        initial={reduceMotion ? false : { opacity: 0, y: -6 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, x: 16 }}
        transition={rowMotion}
        className={"todo-row" + (todo.completed ? " is-done" : "")}
      >
        <label className="todo-check">
          <input
            type="checkbox"
            className="todo-check-input"
            checked={todo.completed}
            onChange={() => toggleTodo(todo.id)}
            aria-label={todo.title}
          />
          <span className="todo-check-box" aria-hidden="true">
            <AnimatePresence initial={false}>
              {todo.completed ? (
                <motion.span
                  key="tick"
                  className="todo-tick"
                  initial={reduceMotion ? false : { scale: 0.4, opacity: 0 }}
                  animate={{ scale: 1, opacity: 1 }}
                  exit={{ scale: 0.4, opacity: 0 }}
                  transition={rowMotion}
                >
                  <Icon name="check" size={12} />
                </motion.span>
              ) : null}
            </AnimatePresence>
          </span>
        </label>
        <div className="todo-main">
          <button type="button" className="todo-title" onClick={() => startEdit(todo)}>
            {todo.title}
          </button>
          {todo.notes ? <p className="todo-notes">{todo.notes}</p> : null}
          {todo.priority || todo.dueDate ? (
            <div className="todo-chips">
              {todo.priority ? (
                <span className={"todo-chip is-" + todo.priority}>{PRIORITY_LABEL[todo.priority]}</span>
              ) : null}
              {todo.dueDate ? (
                <span className={"todo-chip" + (overdue ? " is-overdue" : "")}>
                  {overdue ? "Overdue · " : ""}
                  {dueLabel(todo.dueDate, today, tomorrow)}
                </span>
              ) : null}
            </div>
          ) : null}
        </div>
        <div className="todo-actions">
          <button
            type="button"
            className="icon-btn todo-action"
            title="Edit task"
            aria-label="Edit task"
            onClick={() => startEdit(todo)}
          >
            <PencilIcon />
          </button>
          <button
            type="button"
            className="icon-btn todo-action is-danger"
            title="Delete task"
            aria-label="Delete task"
            onClick={() => deleteTodo(todo.id)}
          >
            <Icon name="trash" size={15} />
          </button>
        </div>
      </motion.li>
    );
  };

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.todo}>To-do</RouteHeading>
      <p className="todo-summary">
        <span className="todo-count">{activeCount}</span> active,{" "}
        <span className="todo-count">{completedCount}</span> done. Stored on this PC and shown in the Dynamic Island.
      </p>
      {error ? (
        <p className="todo-error" role="alert">
          {error}
        </p>
      ) : null}

      <form className="todo-add" onSubmit={handleAdd}>
        <input
          className="todo-field todo-add-title"
          value={newTitle}
          onChange={(event) => setNewTitle(event.target.value)}
          placeholder="Add a task"
          aria-label="Task"
        />
        <select
          className="todo-field todo-priority"
          value={newPriority}
          onChange={(event) => setNewPriority(event.target.value as Priority)}
          aria-label="Priority"
        >
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
        <input
          type="date"
          className="todo-field todo-date"
          value={newDueDate}
          onChange={(event) => setNewDueDate(event.target.value)}
          aria-label="Due date"
        />
        <button type="submit" className="btn btn-primary" disabled={!newTitle.trim()}>
          Add task
        </button>
      </form>

      <div className="todo-toolbar">
        <div className="todo-segmented" role="group" aria-label="Show tasks">
          {FILTERS.map((item) => {
            const active = filter === item.value;
            return (
              <button
                key={item.value}
                type="button"
                className="todo-segment"
                aria-pressed={active}
                onClick={() => setFilter(item.value)}
              >
                {active ? (
                  <motion.span layoutId="todo-segment-pill" className="todo-segment-pill" transition={pillMotion} />
                ) : null}
                <span className="todo-segment-label">{item.label}</span>
              </button>
            );
          })}
        </div>
        {completedCount > 0 ? (
          <button type="button" className="linkbtn" onClick={clearCompleted}>
            Clear completed
          </button>
        ) : null}
      </div>

      <div className="todo-card">
        {visible.length === 0 ? (
          <div className="todo-empty">
            <span className="todo-empty-mark" aria-hidden="true">
              <Icon name={filter === "done" ? "check" : "todo"} size={18} />
            </span>
            <p className="todo-empty-title">{EMPTY_COPY[filter].title}</p>
            <p className="todo-empty-body">{EMPTY_COPY[filter].body}</p>
          </div>
        ) : (
          <ul className="todo-list">
            <AnimatePresence initial={false}>
              {visible.map((todo) => (editingId === todo.id ? editRow(todo) : taskRow(todo)))}
            </AnimatePresence>
          </ul>
        )}
      </div>
    </div>
  );
}
