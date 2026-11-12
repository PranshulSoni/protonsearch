import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

/** Head and empty state shared by the agent, to-do and git cards. */
export function CardHeader({
  icon: Icon,
  title,
  subtitle,
  end,
}: {
  icon: LucideIcon;
  title: string;
  subtitle?: string;
  end?: ReactNode;
}) {
  return (
    <header className="flex min-w-0 items-center gap-3">
      <span className="grid size-9 shrink-0 place-items-center rounded-[10px] border border-line bg-raised text-fg-2">
        <Icon size={16} strokeWidth={1.75} />
      </span>
      <div className="min-w-0 flex-1">
        <h2 className="truncate text-[15px] font-semibold leading-5 text-fg">{title}</h2>
        {subtitle ? <p className="truncate text-[12px] leading-4 text-fg-3">{subtitle}</p> : null}
      </div>
      {end}
    </header>
  );
}

export function CardEmpty({ title, hint }: { title: string; hint: string }) {
  return (
    <div role="status" className="flex flex-1 flex-col items-center justify-center gap-1 px-4 py-6 text-center">
      <strong className="text-[13px] font-semibold text-fg">{title}</strong>
      <span className="line-clamp-3 text-[12px] text-fg-3">{hint}</span>
    </div>
  );
}
