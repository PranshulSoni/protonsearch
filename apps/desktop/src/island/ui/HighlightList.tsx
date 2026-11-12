import type { ReactNode } from "react";
import { motion, useReducedMotion, type Transition } from "motion/react";
import { cn } from "@/lib/utils";

const GLIDE: Transition = { type: "spring", stiffness: 520, damping: 42 };

/**
 * One row of a list with a shared highlight. Only the active row mounts the
 * pill and every row uses the same `layoutId`, so the pill glides from the old
 * row to the new one on arrow keys and hover instead of jumping.
 */
export function HighlightRow({
  layoutId,
  active,
  onHover,
  className,
  children,
}: {
  layoutId: string;
  active: boolean;
  onHover?: () => void;
  className?: string;
  children: ReactNode;
}) {
  const reduce = useReducedMotion();
  return (
    <div className={cn("relative", className)} onMouseEnter={onHover}>
      {active ? (
        <motion.span
          layoutId={layoutId}
          aria-hidden="true"
          className="ps-glass-pill absolute inset-0 rounded-[10px] bg-active"
          transition={reduce ? { duration: 0 } : GLIDE}
        />
      ) : null}
      <div className="relative">{children}</div>
    </div>
  );
}
