import type { IslandCardId } from "../bridge/types";
import { islandCardLabel } from "./islandDims";

/** V9 page dots: visual dot stays 5/4 px; the button owns a >=24 px hit area. */
export function IslandPageDots({
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
  if (cards.length === 0) return null;
  return (
    <nav
      className={"island-dots" + (compact ? " island-dots--compact" : "")}
      aria-label="Jump to island card"
    >
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
          </button>
        );
      })}
    </nav>
  );
}
