import { useId } from "react";
import { motion, useReducedMotion, type Transition } from "motion/react";
import { cn } from "@/lib/utils";

export interface AnimatedTab<T extends string> {
  id: T;
  label: string;
}

const GLIDE: Transition = { type: "spring", stiffness: 520, damping: 42 };

/**
 * Tab strip with one shared pill under the active tab. The pill glides between
 * tabs instead of jumping. Look comes from the caller (`className`, `itemClassName`).
 */
export function AnimatedTabs<T extends string>({
  tabs,
  value,
  onChange,
  label,
  className,
  itemClassName,
}: {
  tabs: ReadonlyArray<AnimatedTab<T>>;
  value: T;
  onChange: (id: T) => void;
  label: string;
  className?: string;
  itemClassName?: string;
}) {
  const pillId = useId();
  const reduce = useReducedMotion();
  return (
    <div role="radiogroup" aria-label={label} className={className}>
      {tabs.map((tab) => {
        const active = tab.id === value;
        return (
          <button
            key={tab.id}
            type="button"
            role="radio"
            aria-checked={active}
            className={cn("relative", itemClassName, active && "is-active")}
            // Pointer clicks keep focus where it was, e.g. in the search box.
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => onChange(tab.id)}
          >
            {active ? (
              <motion.span
                layoutId={pillId}
                aria-hidden="true"
                className="ps-glass-pill absolute inset-0 rounded-[6px] bg-active"
                transition={reduce ? { duration: 0 } : GLIDE}
              />
            ) : null}
            <span className="relative">{tab.label}</span>
          </button>
        );
      })}
    </div>
  );
}
