import { useId } from "react";
import { motion, useReducedMotion, type Transition } from "motion/react";
import type { IslandCardId } from "../../bridge/types";
import { cn } from "@/lib/utils";
import { islandCardLabel } from "../islandDims";

const GLIDE: Transition = { type: "spring", stiffness: 420, damping: 34 };

/**
 * Card dots with a gooey active blob. The blob shares one layoutId, so it glides
 * from dot to dot, and the SVG filter merges it with the dots it passes.
 */
export function GooeyNav({
  cards,
  active,
  compact,
  onSelect,
}: {
  cards: IslandCardId[];
  active: IslandCardId;
  compact: boolean;
  onSelect: (card: IslandCardId) => void;
}) {
  // React's useId output contains characters that are not safe in url(#...).
  const id = "island-goo-" + useId().replace(/[^\w-]/g, "");
  const reduce = useReducedMotion();
  if (cards.length === 0) return null;
  return (
    <nav
      className={"island-dots" + (compact ? " island-dots--compact" : "")}
      style={{ filter: "url(#" + id + ")" }}
      aria-label="Jump to island card"
    >
      <svg className="absolute" width="0" height="0" aria-hidden="true">
        <defs>
          <filter id={id}>
            <feGaussianBlur in="SourceGraphic" stdDeviation={compact ? 1 : 2} result="blur" />
            <feColorMatrix
              in="blur"
              mode="matrix"
              values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -7"
              result="goo"
            />
            <feComposite in="SourceGraphic" in2="goo" operator="atop" />
          </filter>
        </defs>
      </svg>
      {cards.map((card) => {
        const isActive = card === active;
        return (
          <button
            key={card}
            type="button"
            className={"island-dot-button" + (isActive ? " is-active" : "")}
            aria-current={isActive ? "page" : undefined}
            aria-label={"Go to " + islandCardLabel(card) + " card"}
            onClick={() => onSelect(card)}
          >
            <span className="island-dot" aria-hidden="true" />
            {isActive ? (
              <motion.span
                layoutId={id}
                aria-hidden="true"
                className={cn(
                  "pointer-events-none absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-hi",
                  compact ? "h-1 w-1.5" : "h-[5px] w-4",
                )}
                transition={reduce ? { duration: 0 } : GLIDE}
              />
            ) : null}
          </button>
        );
      })}
    </nav>
  );
}
