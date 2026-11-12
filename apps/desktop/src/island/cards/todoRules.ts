export type Priority = "low" | "medium" | "high";

export const PRIORITY_ORDER: Record<Priority, number> = { high: 0, medium: 1, low: 2 };
export const NEXT_PRIORITY: Record<Priority, Priority> = { low: "medium", medium: "high", high: "low" };
export const PRIORITY_LABEL: Record<Priority, string> = { high: "High", medium: "Med", low: "Low" };

const pad = (n: number) => String(n).padStart(2, "0");
const isoDay = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

export function todayIso(): string {
  return isoDay(new Date());
}

export function tomorrowIso(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  return isoDay(d);
}

export function nowDateTimeLocal(): string {
  const d = new Date();
  return `${isoDay(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Reads `!high`, `!med`, `@today`, `@tomorrow` and `@YYYY-MM-DD` out of the add row. */
export function parseQuickTodoInput(
  raw: string,
  fallbackPriority: Priority,
  fallbackDue: string | undefined,
): { title: string; priority: Priority; dueDate?: string } {
  let priority = fallbackPriority;
  let dueDate = fallbackDue;
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

  return { title: text.replace(/\s{2,}/g, " ").trim(), priority, dueDate };
}

export function formatDueBadge(
  dueDate?: string | null,
): { label: string; overdue: boolean; isToday: boolean } | null {
  if (!dueDate) return null;
  const today = todayIso();
  const tomorrow = tomorrowIso();
  const datePart = dueDate.slice(0, 10);
  const timePart = dueDate.includes("T") ? dueDate.slice(11, 16) : null;
  const timeSuffix = timePart ? ` ${timePart}` : "";

  if (datePart === today) return { label: `Today${timeSuffix}`, overdue: false, isToday: true };
  if (datePart === tomorrow) return { label: `Tomorrow${timeSuffix}`, overdue: false, isToday: false };
  if (datePart < today) {
    return { label: `Overdue · ${datePart.slice(5)}${timeSuffix}`, overdue: true, isToday: false };
  }
  return { label: `${datePart.slice(5)}${timeSuffix}`, overdue: false, isToday: false };
}
