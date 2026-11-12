import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

/** Shared pieces for the cards that own their look (ClipboardCard, BrowserCard). */

export function CardHeader({
  icon: Icon,
  title,
  subtitle,
  trailing,
}: {
  icon: LucideIcon;
  title: string;
  subtitle: string;
  trailing?: ReactNode;
}) {
  return (
    <header className="flex items-center gap-3">
      <span className="grid h-10 w-10 shrink-0 place-items-center rounded-[14px] bg-raised text-fg-2">
        <Icon size={18} strokeWidth={1.75} aria-hidden="true" />
      </span>
      <div className="flex min-w-0 flex-1 flex-col">
        <h2 className="text-[15px] font-semibold leading-tight text-fg">{title}</h2>
        <p className="truncate text-[12px] text-fg-3">{subtitle}</p>
      </div>
      {trailing}
    </header>
  );
}

export function EmptyState({ title, hint }: { title: string; hint: string }) {
  return (
    <div role="status" className="flex flex-col items-center gap-1 px-4 py-10 text-center">
      <strong className="text-[13px] font-semibold text-fg">{title}</strong>
      <span className="text-[12px] text-fg-3">{hint}</span>
    </div>
  );
}

/** Static placeholder rows while a source loads. No animation, so the island stays calm. */
export function CardSkeleton() {
  return (
    <div aria-hidden="true" className="flex flex-col">
      {[0, 1, 2].map((row) => (
        <span key={row} className="my-1 h-12 rounded-[10px] bg-raised" />
      ))}
    </div>
  );
}
