// Settings → Cards & order: the Dynamic Island card list (order, visibility and
// the default card). The parent saves each change as it is made.

import { useRef } from "react";
import type { IslandCardId, IslandSettings } from "../../bridge/types";
import { CARD_META, moveCard, resetIslandToV9, resolveDefaultCard, setCardEnabled } from "../islandLogic";
import { Icon, Switch } from "../ui";

export default function IslandCardsSettings({
  island,
  onChange,
}: {
  island: IslandSettings;
  onChange: (patch: Partial<IslandSettings>) => void;
}) {
  const dragFrom = useRef<number | null>(null);
  const selectedDefault = resolveDefaultCard(island.cards, island.defaultCard);
  const lastIndex = island.cards.length - 1;

  const move = (from: number, to: number, id: IslandCardId) => {
    const cards = moveCard(island.cards, from, to);
    if (cards === island.cards) return;
    onChange({ cards, defaultCard: resolveDefaultCard(cards, island.defaultCard) });
    window.requestAnimationFrame(() => {
      document.querySelector<HTMLElement>('[data-move-card="' + id + '"]')?.focus();
    });
  };

  const toggle = (id: IslandCardId, enabled: boolean) => {
    const cards = setCardEnabled(island.cards, id, enabled);
    onChange({ cards, defaultCard: resolveDefaultCard(cards, island.defaultCard) });
  };

  const reset = () => {
    const base = resetIslandToV9(island);
    onChange({ cards: base.cards, defaultCard: base.defaultCard });
  };

  return (
    <>
      <ul className="prefs-card-list">
        {island.cards.map((card, index) => {
          const meta = CARD_META[card.id];
          const isDefault = card.id === selectedDefault;
          return (
            <li
              key={card.id}
              className={"prefs-card" + (card.enabled ? "" : " is-off")}
              draggable
              onDragStart={(event) => {
                event.dataTransfer.effectAllowed = "move";
                event.dataTransfer.setData("text/plain", card.id);
                dragFrom.current = index;
              }}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                const from = dragFrom.current;
                dragFrom.current = null;
                if (from !== null && from !== index) move(from, index, card.id);
              }}
              onDragEnd={() => {
                dragFrom.current = null;
              }}
            >
              <span className="prefs-card-order">
                <button
                  type="button"
                  data-move-card={card.id}
                  aria-label={"Move " + meta.title + " up"}
                  disabled={index === 0}
                  onClick={() => move(index, index - 1, card.id)}
                >
                  ↑
                </button>
                <button
                  type="button"
                  aria-label={"Move " + meta.title + " down"}
                  disabled={index === lastIndex}
                  onClick={() => move(index, index + 1, card.id)}
                >
                  ↓
                </button>
              </span>
              <span className="prefs-card-icon" aria-hidden="true">
                <Icon name={meta.icon} size={15} />
              </span>
              <span className="prefs-card-main">
                <span className="prefs-card-title">
                  {meta.title}
                  {isDefault ? <span className="prefs-card-tag">Default</span> : null}
                </span>
                <span className="prefs-card-desc">{meta.description}</span>
              </span>
              <button
                type="button"
                className="btn"
                disabled={!card.enabled || isDefault}
                onClick={() => onChange({ defaultCard: resolveDefaultCard(island.cards, card.id) })}
              >
                Set default
              </button>
              <Switch
                checked={card.enabled}
                disabled={card.id === "search"}
                label={"Show " + meta.title}
                onChange={(next) => toggle(card.id, next)}
              />
            </li>
          );
        })}
      </ul>
      <div className="prefs-actions">
        <button type="button" className="btn" onClick={reset}>
          Reset order
        </button>
      </div>
    </>
  );
}
