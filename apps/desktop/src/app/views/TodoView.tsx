import { useMemo, useState } from "react";
import { useTodos, type TodoItem } from "../../lib/todoStore";
import { SECTION_EYEBROWS } from "../sections";
import { Callout, EmptyState, Icon, Panel, RouteHeading } from "../ui";

type FilterTab = "all" | "active" | "completed" | "high" | "today";

export default function TodoView() {
  const { todos, addTodo, toggleTodo, deleteTodo, updateTodo, clearCompleted } = useTodos();
  const [filter, setFilter] = useState<FilterTab>("all");
  const [newTitle, setNewTitle] = useState("");
  const [newPriority, setNewPriority] = useState<"low" | "medium" | "high">("medium");
  const [newDueDate, setNewDueDate] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [editNotes, setEditNotes] = useState("");

  const todayStr = useMemo(() => new Date().toISOString().slice(0, 10), []);

  const filteredTodos = useMemo(() => {
    return todos.filter((todo) => {
      if (filter === "active") return !todo.completed;
      if (filter === "completed") return todo.completed;
      if (filter === "high") return todo.priority === "high" && !todo.completed;
      if (filter === "today") return todo.dueDate === todayStr || (!todo.dueDate && !todo.completed);
      return true;
    });
  }, [todos, filter, todayStr]);

  const activeCount = useMemo(() => todos.filter((t) => !t.completed).length, [todos]);
  const completedCount = useMemo(() => todos.filter((t) => t.completed).length, [todos]);

  const handleAdd = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!newTitle.trim()) return;
    addTodo(newTitle, newDueDate || undefined, newPriority);
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

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.todo}>To-do</RouteHeading>
      <p className="page-lead">
        Keep track of your active tasks, reminders and notes with local-only storage.
      </p>

      <form className="config-box" onSubmit={handleAdd} style={{ marginBottom: "var(--space-4)" }}>
        <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center", width: "100%" }}>
          <input
            className="input"
            style={{ flex: 1, minHeight: 40, fontSize: "14px" }}
            placeholder="Add a new task… (Press Enter to save)"
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
          />
          <select
            className="input"
            style={{ width: "110px", minHeight: 40, fontSize: "13px" }}
            value={newPriority}
            onChange={(e) => setNewPriority(e.target.value as "low" | "medium" | "high")}
          >
            <option value="low">Low Priority</option>
            <option value="medium">Medium</option>
            <option value="high">High Priority</option>
          </select>
          <input
            type="date"
            className="input"
            style={{ width: "140px", minHeight: 40, fontSize: "13px" }}
            value={newDueDate}
            onChange={(e) => setNewDueDate(e.target.value)}
            title="Optional due date"
          />
          <button type="submit" className="btn" disabled={!newTitle.trim()}>
            <Icon name="plus" size={16} /> Add Task
          </button>
        </div>
      </form>

      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-3)" }}>
        <div className="segmented-control" role="tablist" aria-label="Task filters">
          <button
            type="button"
            className={"seg-btn" + (filter === "all" ? " is-active" : "")}
            onClick={() => setFilter("all")}
          >
            All ({todos.length})
          </button>
          <button
            type="button"
            className={"seg-btn" + (filter === "active" ? " is-active" : "")}
            onClick={() => setFilter("active")}
          >
            Active ({activeCount})
          </button>
          <button
            type="button"
            className={"seg-btn" + (filter === "today" ? " is-active" : "")}
            onClick={() => setFilter("today")}
          >
            Today
          </button>
          <button
            type="button"
            className={"seg-btn" + (filter === "high" ? " is-active" : "")}
            onClick={() => setFilter("high")}
          >
            High Priority
          </button>
          <button
            type="button"
            className={"seg-btn" + (filter === "completed" ? " is-active" : "")}
            onClick={() => setFilter("completed")}
          >
            Completed ({completedCount})
          </button>
        </div>

        {completedCount > 0 ? (
          <button type="button" className="linkbtn" onClick={clearCompleted} style={{ fontSize: "13px" }}>
            Clear completed
          </button>
        ) : null}
      </div>

      <Panel
        id="todo-list"
        title={filter === "all" ? "All Tasks" : filter === "active" ? "Active Tasks" : filter === "completed" ? "Completed Tasks" : filter === "high" ? "High Priority Tasks" : "Today's Tasks"}
        hint="Synced automatically with the Dynamic Island To-do card in real-time."
      >
        {filteredTodos.length === 0 ? (
          <EmptyState
            title="No tasks in this view."
            body={filter === "completed" ? "No tasks completed yet." : "You're all caught up! Add a new task above."}
          />
        ) : (
          <div className="config-list">
            {filteredTodos.map((todo) => {
              const isEditing = editingId === todo.id;
              return (
                <div
                  key={todo.id}
                  className="config-row"
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "var(--space-3)",
                    padding: "var(--space-3) var(--space-4)",
                    opacity: todo.completed ? 0.65 : 1,
                  }}
                >
                  <button
                    type="button"
                    className={"check-toggle" + (todo.completed ? " is-checked" : "")}
                    aria-label={todo.completed ? "Mark task active" : "Mark task complete"}
                    onClick={() => toggleTodo(todo.id)}
                    style={{
                      width: 22,
                      height: 22,
                      borderRadius: "6px",
                      border: "1.5px solid var(--ps-line-strong)",
                      background: todo.completed ? "var(--ps-accent)" : "transparent",
                      color: "#fff",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      cursor: "pointer",
                      flexShrink: 0,
                    }}
                  >
                    {todo.completed ? <Icon name="check" size={14} /> : null}
                  </button>

                  <div style={{ flex: 1, minWidth: 0 }}>
                    {isEditing ? (
                      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
                        <input
                          className="input"
                          value={editText}
                          onChange={(e) => setEditText(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") saveEdit(todo.id);
                            if (e.key === "Escape") setEditingId(null);
                          }}
                          autoFocus
                        />
                        <input
                          className="input"
                          placeholder="Optional notes or details…"
                          value={editNotes}
                          onChange={(e) => setEditNotes(e.target.value)}
                          style={{ fontSize: "12px" }}
                        />
                        <div style={{ display: "flex", gap: "var(--space-2)" }}>
                          <button type="button" className="btn" onClick={() => saveEdit(todo.id)}>
                            Save
                          </button>
                          <button type="button" className="btn" onClick={() => setEditingId(null)}>
                            Cancel
                          </button>
                        </div>
                      </div>
                    ) : (
                      <>
                        <div
                          style={{
                            fontSize: "14px",
                            fontWeight: 500,
                            textDecoration: todo.completed ? "line-through" : "none",
                            color: todo.completed ? "var(--ps-ink-2)" : "var(--ps-ink)",
                            cursor: "pointer",
                          }}
                          onClick={() => startEdit(todo)}
                        >
                          {todo.title}
                        </div>
                        {todo.notes ? (
                          <div style={{ fontSize: "12px", color: "var(--ps-ink-3)", marginTop: 2 }}>
                            {todo.notes}
                          </div>
                        ) : null}
                        <div style={{ display: "flex", gap: "var(--space-2)", marginTop: 4, alignItems: "center" }}>
                          {todo.priority && todo.priority !== "medium" ? (
                            <span
                              className="badge"
                              style={{
                                fontSize: "11px",
                                textTransform: "uppercase",
                                padding: "1px 6px",
                                borderRadius: "4px",
                                background: todo.priority === "high" ? "var(--ps-negative-pill-bg)" : "var(--ps-surface-soft)",
                                color: todo.priority === "high" ? "var(--ps-negative-pill-ink)" : "var(--ps-ink-2)",
                              }}
                            >
                              {todo.priority}
                            </span>
                          ) : null}
                          {todo.dueDate ? (
                            <span style={{ fontSize: "12px", color: "var(--ps-ink-3)" }}>
                              Due: {todo.dueDate}
                            </span>
                          ) : null}
                        </div>
                      </>
                    )}
                  </div>

                  {!isEditing ? (
                    <div style={{ display: "flex", gap: "var(--space-2)" }}>
                      <button
                        type="button"
                        className="icon-btn"
                        title="Edit task"
                        aria-label="Edit task"
                        onClick={() => startEdit(todo)}
                      >
                        <Icon name="preferences" size={15} />
                      </button>
                      <button
                        type="button"
                        className="icon-btn"
                        title="Delete task"
                        aria-label="Delete task"
                        onClick={() => deleteTodo(todo.id)}
                      >
                        <Icon name="close" size={15} />
                      </button>
                    </div>
                  ) : null}
                </div>
              );
            })}
          </div>
        )}
      </Panel>
    </div>
  );
}
