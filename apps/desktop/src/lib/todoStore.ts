import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isTauriRuntime } from "../bridge/api";

export interface TodoItem {
  id: string;
  title: string;
  completed: boolean;
  createdAt: number;
  updatedAt?: number;
  completedAt?: number | null;
  dueDate?: string | null;
  priority?: "low" | "medium" | "high" | null;
  notes?: string | null;
  timezone?: string | null;
  pinned?: boolean;
  position?: number;
  association?: string | null;
}
const STORAGE_KEY = "protonsearch_todos_v2";
const BROADCAST_EVENT = "protonsearch:todos-changed";

export function getStoredTodos(): TodoItem[] {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return [];
  const items: unknown = JSON.parse(raw);
  if (!Array.isArray(items)) throw new Error("Saved tasks could not be read");
  return items;
}
export function saveStoredTodos(items: TodoItem[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
  window.dispatchEvent(new Event(BROADCAST_EVENT));
}
function payload(item: TodoItem) {
  return {
    ...item, updatedAt: item.updatedAt ?? item.createdAt,
    completedAt: item.completedAt ?? null, notes: item.notes ?? null,
    dueDate: item.dueDate ?? null, timezone: item.timezone ?? null,
    priority: item.priority ?? null, pinned: item.pinned ?? false,
    position: item.position ?? -item.createdAt, association: item.association ?? null,
  };
}
export function useTodos() {
  const [todos, setTodos] = useState<TodoItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const reload = useCallback(async () => {
    try {
      setTodos(isTauriRuntime() ? await invoke<TodoItem[]>("todos_list") : getStoredTodos());
      setError(null);
    } catch (reason) { setError(String(reason)); }
  }, []);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const refresh = () => { void reload(); };
    const start = async () => {
      if (isTauriRuntime()) {
        unlisten = await listen("todos:changed", refresh);
        if (disposed) { unlisten(); return; }
        // The original browser copy remains available after an idempotent import.
        const items = getStoredTodos().map(payload);
        await invoke("todos_import", { items, source: "browser-v2" });
      }
      if (!disposed) await reload();
    };
    void start().catch((reason) => { if (!disposed) setError(String(reason)); });
    window.addEventListener(BROADCAST_EVENT, refresh);
    window.addEventListener("storage", refresh);
    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener(BROADCAST_EVENT, refresh);
      window.removeEventListener("storage", refresh);
    };
  }, [reload]);
  const mutate = useCallback(async (saved: TodoItem[], deleted: string[] = []) => {
    try {
      if (isTauriRuntime()) {
        for (const item of saved) await invoke("todos_save", { item: payload(item) });
        for (const id of deleted) await invoke("todos_delete", { id });
      } else {
        const items = new Map(getStoredTodos().map((item) => [item.id, item]));
        for (const item of saved) items.set(item.id, item);
        for (const id of deleted) items.delete(id);
        saveStoredTodos([...items.values()]);
      }
      await reload();
      return true;
    } catch (reason) { setError(String(reason)); return false; }
  }, [reload]);
  const addTodo = useCallback(async (title: string, dueDate?: string, priority: "low" | "medium" | "high" = "medium") => {
    if (!title.trim()) return false;
    return mutate([{ id: crypto.randomUUID(), title: title.trim(), completed: false,
      createdAt: Date.now(), dueDate, priority }]);
  }, [mutate]);
  const updateTodo = useCallback((id: string, patch: Partial<TodoItem>) => {
    const item = todos.find((task) => task.id === id);
    if (item) void mutate([{ ...item, ...patch, id, createdAt: item.createdAt, updatedAt: Date.now() }]);
  }, [todos, mutate]);
  const toggleTodo = useCallback((id: string) => {
    const item = todos.find((task) => task.id === id);
    if (item) updateTodo(id, { completed: !item.completed, completedAt: item.completed ? null : Date.now() });
  }, [todos, updateTodo]);
  const deleteTodo = useCallback((id: string) => {
    if (window.confirm("Delete this task?")) void mutate([], [id]);
  }, [mutate]);
  const clearCompleted = useCallback(() => {
    const ids = todos.filter((item) => item.completed).map((item) => item.id);
    if (ids.length && window.confirm("Delete completed tasks?")) void mutate([], ids);
  }, [todos, mutate]);
  return { todos, error, addTodo, toggleTodo, deleteTodo, updateTodo, clearCompleted, reload };
}
