import type { QuickDestination } from "./launcherQuickDestinations";
import { quickDestinationDomId } from "./launcherQuickDestinations";

/**
 * Empty-state quick destinations (v10 Normal Launcher §5). Rendered as listbox
 * options so ↑/↓ from the search input traverse them; Enter activates.
 * Only real data (source counts / agent state) appears in the subtitle.
 */

export interface QuickDestinationOptionsProps {
  destinations: readonly QuickDestination[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onActivate: (index: number) => void;
}

function Option({
  destination,
  index,
  selected,
  onSelect,
  onActivate,
}: {
  destination: QuickDestination;
  index: number;
  selected: boolean;
  onSelect: (index: number) => void;
  onActivate: (index: number) => void;
}) {
  return (
    <button
      type="button"
      role="option"
      id={quickDestinationDomId(destination.id)}
      aria-selected={selected}
      tabIndex={-1}
      className={
        "quick-destination" +
        (destination.secondary ? " quick-destination--secondary" : "") +
        (destination.off ? " quick-destination--off" : "")
      }
      data-testid="quick-destination"
      data-destination-id={destination.id}
      onMouseDown={(event) => event.preventDefault()}
      onMouseEnter={() => onSelect(index)}
      onClick={() => onActivate(index)}
    >
      <span className="quick-destination-label">{destination.label}</span>
      <span className="quick-destination-subtitle">{destination.subtitle}</span>
    </button>
  );
}

export function QuickDestinationOptions({
  destinations,
  selectedIndex,
  onSelect,
  onActivate,
}: QuickDestinationOptionsProps) {
  const primary = destinations
    .map((destination, index) => ({ destination, index }))
    .filter((entry) => !entry.destination.secondary);
  const secondary = destinations
    .map((destination, index) => ({ destination, index }))
    .filter((entry) => entry.destination.secondary);
  return (
    <>
      <div role="group" aria-label="Quick destinations" className="quick-grid">
        {primary.map(({ destination, index }) => (
          <Option
            key={destination.id}
            destination={destination}
            index={index}
            selected={selectedIndex === index}
            onSelect={onSelect}
            onActivate={onActivate}
          />
        ))}
      </div>
      <div role="group" aria-label="More destinations" className="quick-secondary">
        {secondary.map(({ destination, index }) => (
          <Option
            key={destination.id}
            destination={destination}
            index={index}
            selected={selectedIndex === index}
            onSelect={onSelect}
            onActivate={onActivate}
          />
        ))}
      </div>
    </>
  );
}
