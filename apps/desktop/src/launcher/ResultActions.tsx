import { useEffect, useRef, useState } from "react";
import type { ActionSpec, SearchResult } from "../bridge/types";

/**
 * Context menu for a result. Only actions the engine supplies are listed
 * (SearchResult.action + SearchResult.actions). Destructive items are never
 * rendered: the frozen IPC contract has no confirmed-destructive command, so
 * offering Rename/Delete would either be a fake control or an unconfirmed
 * destructive action. Binding: docs/design/SCREENS.md §2.8.
 */

export interface MenuAction {
  id: string;
  label: string;
  action: ActionSpec;
  shortcut?: string | null;
}

export interface MenuModel {
  actions: MenuAction[];
  skippedDestructive: number;
}

export function buildMenuActions(result: SearchResult): MenuModel {
  const actions: MenuAction[] = [];
  if (result.action.type !== "none") {
    actions.push({ id: "open", label: "Open", action: result.action });
  }
  let skippedDestructive = 0;
  for (const item of result.actions ?? []) {
    if (item.destructive) {
      skippedDestructive += 1;
      continue;
    }
    if (actions.some((existing) => existing.id === item.id)) continue;
    actions.push({ id: item.id, label: item.label, action: item.action, shortcut: item.shortcut });
  }
  return { actions, skippedDestructive };
}

export interface ResultActionsProps {
  result: SearchResult;
  model: MenuModel;
  menuRef: React.RefObject<HTMLDivElement | null>;
  onRun: (action: MenuAction) => void;
  onClose: () => void;
}

export function ResultActions({ result, model, menuRef, onRun, onClose }: ResultActionsProps) {
  const [activeIndex, setActiveIndex] = useState(0);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const count = model.actions.length;

  useEffect(() => {
    itemRefs.current[activeIndex]?.focus();
  }, [activeIndex]);

  if (count === 0) return null;

  const move = (delta: number) => {
    setActiveIndex((current) => ((current + delta) % count + count) % count);
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        move(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        move(-1);
        break;
      case "Home":
        event.preventDefault();
        setActiveIndex(0);
        break;
      case "End":
        event.preventDefault();
        setActiveIndex(count - 1);
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        onRun(model.actions[activeIndex]);
        break;
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        onClose();
        break;
      case "Tab":
        event.preventDefault();
        event.stopPropagation();
        onClose();
        break;
      default:
        break;
    }
  };

  return (
    <div
      className="actions-menu"
      role="menu"
      aria-label={"Actions for " + result.title}
      ref={menuRef}
      onKeyDown={onKeyDown}
      data-testid="actions-menu"
    >
      <div className="actions-head" role="presentation">
        <span className="actions-title">{result.title}</span>
        <span className="actions-kind">{result.kind.replace(/_/g, " ")}</span>
      </div>
      {model.actions.map((item, index) => (
        <button
          key={item.id}
          type="button"
          role="menuitem"
          tabIndex={index === activeIndex ? 0 : -1}
          className="menu-item"
          ref={(element) => {
            itemRefs.current[index] = element;
          }}
          onMouseEnter={() => setActiveIndex(index)}
          onClick={() => onRun(item)}
        >
          <span className="menu-label">{item.label}</span>
          {item.shortcut ? <span className="menu-shortcut">{item.shortcut}</span> : null}
        </button>
      ))}
    </div>
  );
}
