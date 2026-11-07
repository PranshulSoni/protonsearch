import type { MouseEvent } from 'react';
import { Icon } from '../../widgets/final/icons';
import { AppGlyph } from '../../widgets/final/primitives';
import type { FinalIconName } from '../../widgets/final/types';

export interface DockGroupItem {
  id: string;
  title: string;
  icon: string | null;
  running: boolean;
}

export interface DockGroupProps {
  label: string;
  placeholder: FinalIconName;
  items: DockGroupItem[];
  disabled?: boolean;
  onOpen: (id: string) => void;
  onCard: (anchor: HTMLElement) => void;
}

export function DockGroup({ label, placeholder, items, disabled = false, onOpen, onCard }: DockGroupProps) {
  const contextCard = (event: MouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    onCard(event.currentTarget);
  };

  if (!items.length) {
    return (
      <button
        type="button"
        className="dock-cell"
        aria-label={`Open ${label} card`}
        disabled={disabled}
        onClick={(event) => onCard(event.currentTarget)}
        onContextMenu={contextCard}
      >
        <span className="dock-tile dock-tile--app is-off">
          <Icon name={placeholder} size={22} />
        </span>
        <span className="dock-tip" aria-hidden="true">{label}</span>
      </button>
    );
  }

  return (
    <div className="dock-group" role="group" aria-label={label}>
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          className="dock-cell dock-app"
          aria-label={item.title}
          disabled={disabled}
          onClick={() => onOpen(item.id)}
          onContextMenu={contextCard}
        >
          <span className="dock-tile dock-tile--app">
            <AppGlyph title={item.title} icon={item.icon} size={32} />
          </span>
          <span className="dock-tip" aria-hidden="true">{item.title}</span>
          {item.running ? <span className="dock-dot" aria-hidden="true" /> : null}
        </button>
      ))}
    </div>
  );
}
