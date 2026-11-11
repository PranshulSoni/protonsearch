import { useCallback, useEffect, useState } from "react";

export interface TodoItem {
  id: string;
  title: string;
  completed: boolean;
  createdAt: number;
  completedAt?: number;
  dueDate?: string; // e.g. "2026-11-11" or "today"
  priority?: "low" | "medium" | "high";
  notes?: string;
}

const STORAGE_KEY = "protonsearch_todos_v2";
const BROADCAST_EVENT = "protonsearch:todos-changed";

const SEED_TODOS: TodoItem[] = [
  {
    id: "todo-1",
    title: "Press Alt + Space to open the Dynamic Island",
    completed: false,
    createdAt: Date.now() - 3600000,
    priority: "high",
  },
  {
    id: "todo-2",
    title: "Configure your indexed folders in Search & Indexing",
    completed: false,
    createdAt: Date.now() - 1800000,
    priority: "medium",
  },
  {
    id: "todo-3",
    title: "Try searching screenshots with OCR",
    completed: false,
    createdAt: Date.now() - 900000,
    priority: "low",
  },
];

export function getStoredTodos(): TodoItem[] {
  if (typeof window === "undefined") return SEED_TODOS;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(SEED_TODOS));
      return SEED_TODOS;
    }
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : SEED_TODOS;
  } catch {
    return SEED_TODOS;
  }
}

export function saveStoredTodos(todos: TodoItem[]): void {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(todos));
    window.dispatchEvent(new CustomEvent(BROADCAST_EVENT));
  } catch {
    // ignore local storage errors
  }
}

export function useTodos() {
  const [todos, setTodos] = useState<TodoItem[]>(getStoredTodos);

  const reload = useCallback(() => {
    setTodos(getStoredTodos());
  }, []);

  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === STORAGE_KEY) reload();
    };
    const onCustom = () => reload();

    window.addEventListener("storage", onStorage);
    window.addEventListener(BROADCAST_EVENT, onCustom);
    return () => {
      window.removeEventListener("storage", onStorage);
      window.removeEventListener(BROADCAST_EVENT, onCustom);
    };
  }, [reload]);

  const addTodo = useCallback(
    (title: string, dueDate?: string, priority: "low" | "medium" | "high" = "medium") => {
      const trimmed = title.trim();
      if (!trimmed) return;
      const next: TodoItem = {
        id: "todo-" + Date.now() + "-" + Math.random().toString(36).slice(2, 6),
        title: trimmed,
        completed: false,
        createdAt: Date.now(),
        dueDate,
        priority,
      };
      setTodos((prev) => {
        const updated = [next, ...prev];
        saveStoredTodos(updated);
        return updated;
      });
    },
    [],
  );

  const toggleTodo = useCallback((id: string) => {
    setTodos((prev) => {
      const updated = prev.map((t) =>
        t.id === id
          ? {
              ...t,
              completed: !t.completed,
              completedAt: !t.completed ? Date.now() : undefined,
            }
          : t,
      );
      saveStoredTodos(updated);
      return updated;
    });
  }, []);

  const deleteTodo = useCallback((id: string) => {
    setTodos((prev) => {
      const updated = prev.filter((t) => t.id !== id);
      saveStoredTodos(updated);
      return updated;
    });
  }, []);

  const updateTodo = useCallback((id: string, patch: Partial<TodoItem>) => {
    setTodos((prev) => {
      const updated = prev.map((t) => (t.id === id ? { ...t, ...patch } : t));
      saveStoredTodos(updated);
      return updated;
    });
  }, []);

  const clearCompleted = useCallback(() => {
    setTodos((prev) => {
      const updated = prev.filter((t) => !t.completed);
      saveStoredTodos(updated);
      return updated;
    });
  }, []);

  return {
    todos,
    addTodo,
    toggleTodo,
    deleteTodo,
    updateTodo,
    clearCompleted,
    reload,
  };
}
