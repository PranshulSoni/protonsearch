import type { KeyboardEvent, ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";
import { ArrowLeft, Check, Speaker, type LucideIcon } from "lucide-react";
import type { AudioDeviceInfo } from "../../bridge/types";
import { cn } from "@/lib/utils";
import { CardHeader } from "./CardChrome";

export const iconButton =
  "grid place-items-center rounded-full text-fg-2 hover:bg-hover hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hi disabled:pointer-events-none";

/** Fills the island slot. Only the body under the title scrolls; drill-downs swap it in place. */
export function CardFrame({
  icon,
  title,
  aside,
  children,
}: {
  icon: LucideIcon;
  title: string;
  aside?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex min-h-0 w-full flex-1 flex-col gap-3 p-4">
      <CardHeader icon={icon} title={title} subtitle={aside} />
      <div className="flex min-h-0 flex-1 flex-col">{children}</div>
    </div>
  );
}

/** Body pane. Mount under AnimatePresence (mode="wait") so panes cross-fade. */
export function Pane({ children, onEscape }: { children: ReactNode; onEscape?: () => void }) {
  const reduced = useReducedMotion();
  return (
    <motion.div
      className="flex min-h-0 flex-1 flex-col gap-3"
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0, transition: { duration: reduced ? 0 : 0.22 } }}
      exit={{ opacity: 0, y: -6, transition: { duration: reduced ? 0 : 0.14 } }}
      onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
        // Stop here so the island's own Escape handling does not also fire.
        if (onEscape && event.key === "Escape") {
          event.stopPropagation();
          onEscape();
        }
      }}
    >
      {children}
    </motion.div>
  );
}

export function DrillHeader({ title, onBack }: { title: string; onBack: () => void }) {
  return (
    <div className="flex items-center gap-2">
      <button type="button" aria-label="Back" autoFocus onClick={onBack} className={cn(iconButton, "size-8 shrink-0")}>
        <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
      </button>
      <h3 className="text-[15px] font-semibold leading-5">{title}</h3>
    </div>
  );
}

export function activeOutput(outputs: AudioDeviceInfo[], defaultId: string | null): AudioDeviceInfo | null {
  return outputs.find((output) => output.id === defaultId || output.isDefault) ?? outputs[0] ?? null;
}

export function AudioOutputList({
  outputs,
  defaultId,
  onSelect,
}: {
  outputs: AudioDeviceInfo[];
  defaultId: string | null;
  onSelect: (deviceId: string) => void;
}) {
  return (
    <div role="radiogroup" aria-label="Audio output" className="flex flex-col gap-1">
      {outputs.map((output) => {
        const active = output.id === defaultId || output.isDefault;
        return (
          <button
            key={output.id}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onSelect(output.id)}
            className={cn(
              "flex items-center gap-3 rounded-row px-3 py-2.5 text-left text-[13px] hover:bg-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-hi",
              active && "bg-active",
            )}
          >
            <Speaker size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-fg-2" />
            <span className="min-w-0 flex-1 truncate">{output.name}</span>
            {active ? <Check size={16} strokeWidth={1.75} aria-hidden="true" className="shrink-0 text-hi" /> : null}
          </button>
        );
      })}
    </div>
  );
}
