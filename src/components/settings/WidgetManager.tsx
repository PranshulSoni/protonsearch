import { useMemo, useState } from 'react';
import { DockWidget } from '../../widgets/final/DockWidget';
import { WidgetCard } from '../../widgets/final/WidgetCard';
import { WidgetSettings } from '../../widgets/final/WidgetSettings';
import {
  FINAL_WIDGET_IDS,
  SIZE_LABELS,
  STORE_WIDGET_IDS,
  getWidgetMeta,
  settingsFor,
} from '../../widgets/final/meta';
import type { FinalWidgetId, FinalWidgetSettings } from '../../widgets/final/types';
import type { WidgetSize } from '../../widgets/sdk/types';
import {
  PROFILE_IDS,
  PROFILE_LABELS,
  disableWidget,
  effectiveSettings,
  effectiveSize,
  enableWidget,
  moveWidget,
  patchItem,
  type AppearanceMode,
  type DockItem,
  type DockLayout,
} from '../dock/layout';

export interface WidgetManagerProps {
  layout: DockLayout;
  onChange: (layout: DockLayout) => void;
  onClose: () => void;
  error?: string | null;
}

type ManagerTab = 'installed' | 'store';

const SIZE_ORDER: readonly WidgetSize[] = ['compact', 'standard', 'wide'];

function itemFor(layout: DockLayout, id: FinalWidgetId): DockItem | undefined {
  return layout.profiles[layout.profile].find((item) => item.id === id);
}

function PreviewStrip({ items }: { items: DockItem[] }) {
  return (
    <div className="wm-dock-strip" role="img" aria-label="Dock preview">
      <div className="wm-dock-rail">
        {items.length ? items.map((item) => (
          <DockWidget
            key={item.id}
            id={item.id}
            size="compact"
            settings={effectiveSettings(item)}
            preview
          />
        )) : <span className="wm-dock-empty">No widgets enabled</span>}
      </div>
    </div>
  );
}

function SizeControl({ id, value, onChange, disabled = false }: {
  id: FinalWidgetId;
  value: WidgetSize;
  onChange: (size: WidgetSize) => void;
  disabled?: boolean;
}) {
  const title = getWidgetMeta(id).title;
  return (
    <div className="wm-size" role="group" aria-label={`Size for ${title}`}>
      {SIZE_ORDER.map((size) => (
        <button
          key={size}
          type="button"
          className="wm-size-btn"
          aria-pressed={value === size}
          disabled={disabled}
          onClick={() => onChange(size)}
        >
          {SIZE_LABELS[size]}
        </button>
      ))}
    </div>
  );
}

function WidgetPreview({ id }: { id: FinalWidgetId }) {
  return (
    <div className="wm-preview">
      <WidgetCard id={id} preview />
    </div>
  );
}

function WidgetCardItem({
  layout,
  item,
  index,
  onToggle,
  onSize,
  onCustomize,
  onMove,
}: {
  layout: DockLayout;
  item: DockItem;
  index: number;
  onToggle: () => void;
  onSize: (size: WidgetSize) => void;
  onCustomize: () => void;
  onMove: (from: number, to: number) => void;
}) {
  const meta = getWidgetMeta(item.id);
  const [dragging, setDragging] = useState(false);
  const enabled = layout.profiles[layout.profile].some((entry) => entry.id === item.id);
  return (
    <article
      className={`wm-card${dragging ? ' is-dragging' : ''}`}
      onDragOver={(event) => { event.preventDefault(); event.dataTransfer.dropEffect = 'move'; }}
      onDrop={(event) => {
        event.preventDefault();
        const from = Number(event.dataTransfer.getData('text/plain'));
        if (Number.isInteger(from)) onMove(from, index);
        setDragging(false);
      }}
    >
      <div className="wm-card-preview">
        <WidgetPreview id={item.id} />
      </div>
      <div className="wm-card-copy">
        <div className="wm-card-title-row">
          <div>
            <h3>{meta.title}</h3>
            <p>{meta.description}</p>
          </div>
          <button
            type="button"
            role="switch"
            className="wm-enable"
            aria-checked={enabled}
            aria-label={`${enabled ? 'Disable' : 'Enable'} ${meta.title}`}
            onClick={onToggle}
          >
            <span className="wm-enable-track" aria-hidden="true"><span className="wm-enable-thumb" /></span>
          </button>
        </div>
        <div className="wm-card-controls">
          <SizeControl id={item.id} value={effectiveSize(item)} onChange={onSize} disabled={!enabled} />
          <button type="button" className="wm-customize" onClick={onCustomize} disabled={!enabled}>
            Customize
          </button>
          <button
            type="button"
            className="wm-grip"
            aria-label={`Reorder ${meta.title}`}
            draggable={enabled}
            disabled={!enabled}
            onDragStart={(event) => {
              event.dataTransfer.setData('text/plain', String(index));
              event.dataTransfer.effectAllowed = 'move';
              setDragging(true);
            }}
            onDragEnd={() => setDragging(false)}
          >
            <span aria-hidden="true">⠿</span>
          </button>
        </div>
      </div>
    </article>
  );
}

export function WidgetManager({ layout, onChange, onClose, error }: WidgetManagerProps) {
  const [tab, setTab] = useState<ManagerTab>('installed');
  const [detail, setDetail] = useState<FinalWidgetId | null>(null);
  const profile = layout.profile;
  const items = layout.profiles[profile];
  const appearance: AppearanceMode = layout.appearances?.[profile] ?? 'capsules';
  const installed = useMemo(() => new Set(items.map((item) => item.id)), [items]);
  const detailItem = detail ? itemFor(layout, detail) : undefined;

  const toggle = (id: FinalWidgetId) => {
    onChange(installed.has(id) ? disableWidget(layout, id) : enableWidget(layout, id));
  };
  const setSize = (id: FinalWidgetId, size: WidgetSize) => {
    onChange(patchItem(layout, id, { size }));
  };
  const setSettings = (id: FinalWidgetId, value: FinalWidgetSettings) => {
    onChange(patchItem(layout, id, { settings: value as unknown as Record<string, unknown> }));
  };
  const resetWidget = (id: FinalWidgetId) => {
    onChange(patchItem(layout, id, { settings: undefined, size: undefined }));
  };

  return (
    <main className="wm-shell" aria-label="Dock and widgets settings">
      <header className="wm-head" data-tauri-drag-region>
        <div className="wm-head-title" data-tauri-drag-region>
          <span className="wm-mark" aria-hidden="true"><span /></span>
          <div>
            <h1>Dock &amp; Widgets</h1>
            <p>{PROFILE_LABELS[profile]} profile · {items.length} of {FINAL_WIDGET_IDS.length} widgets</p>
          </div>
        </div>
        <div className="wm-profiles" role="group" aria-label="Active profile">
          {PROFILE_IDS.map((id) => (
            <button
              key={id}
              type="button"
              className="wm-profile"
              aria-pressed={profile === id}
              onClick={() => onChange({ ...layout, profile: id })}
            >
              {PROFILE_LABELS[id]}
            </button>
          ))}
        </div>
        <button type="button" className="wm-close" aria-label="Close Dock and Widgets" onClick={onClose}>
          <span aria-hidden="true">×</span>
        </button>
      </header>

      <div className="wm-tabs" role="tablist" aria-label="Widget manager">
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'installed'}
          className="wm-tab"
          onClick={() => { setTab('installed'); setDetail(null); }}
        >
          Installed <span>{FINAL_WIDGET_IDS.length}</span>
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === 'store'}
          className="wm-tab"
          onClick={() => { setTab('store'); setDetail(null); }}
        >
          Widget Store <span>{STORE_WIDGET_IDS.length}</span>
        </button>
      </div>

      {error ? <div className="wm-alert" role="alert">{error}</div> : null}

      {detail ? (
        <section className="wm-detail" aria-label={`${getWidgetMeta(detail).title} settings`}>
          <div className="wm-detail-bar">
            <button type="button" className="wm-back" onClick={() => setDetail(null)}>Back</button>
            <div>
              <h2>{getWidgetMeta(detail).title}</h2>
              <p>{getWidgetMeta(detail).description}</p>
            </div>
            <button type="button" className="wm-done" onClick={() => setDetail(null)}>Done</button>
          </div>
          <div className="wm-detail-body">
            <div className="wm-detail-preview">
              <WidgetPreview id={detail} />
              <div className="wm-detail-summary">
                <span>{SIZE_LABELS[detailItem ? effectiveSize(detailItem) : getWidgetMeta(detail).defaultSize]} size</span>
                <span>{installed.has(detail) ? 'Installed' : 'Not installed'}</span>
              </div>
            </div>
            <div className="wm-detail-settings">
              <WidgetSettings
                id={detail}
                value={settingsFor(detail, detailItem?.settings)}
                onChange={(value) => setSettings(detail, value)}
                disabled={!installed.has(detail)}
              />
              <div className="wm-detail-actions">
                <button type="button" className="wm-reset" onClick={() => resetWidget(detail)} disabled={!installed.has(detail)}>
                  Reset to defaults
                </button>
                <button type="button" className="wm-done" onClick={() => setDetail(null)}>Done</button>
              </div>
              {!installed.has(detail) ? (
                <button type="button" className="wm-install" onClick={() => toggle(detail)}>Install {getWidgetMeta(detail).title}</button>
              ) : null}
            </div>
          </div>
        </section>
      ) : (
        <section className="wm-body">
          <div className="wm-toolbar">
            <div className="wm-toolbar-title">
              <strong>{tab === 'installed' ? 'Installed widgets' : 'Widget Store'}</strong>
              <span>{tab === 'installed' ? `${installed.size} enabled` : 'Add optional widgets to the dock'}</span>
            </div>
            {tab === 'installed' ? (
              <div className="wm-appearance" role="group" aria-label="Dock tile style">
                <button
                  type="button"
                  className="wm-style-pill"
                  aria-pressed={appearance === 'capsules'}
                  onClick={() => onChange({ ...layout, appearances: { ...layout.appearances, [profile]: 'capsules' } })}
                >
                  Capsules
                </button>
                <button
                  type="button"
                  className="wm-style-pill"
                  aria-pressed={appearance === 'lines'}
                  onClick={() => onChange({ ...layout, appearances: { ...layout.appearances, [profile]: 'lines' } })}
                >
                  Lines
                </button>
              </div>
            ) : null}
          </div>

          <PreviewStrip items={items} />

          <div className="wm-grid">
            {tab === 'installed' ? FINAL_WIDGET_IDS.map((id) => {
              const item = itemFor(layout, id) ?? { id };
              const index = items.findIndex((entry) => entry.id === id);
              return (
                <WidgetCardItem
                  key={id}
                  layout={layout}
                  item={item}
                  index={index}
                  onToggle={() => toggle(id)}
                  onSize={(size) => setSize(id, size)}
                  onCustomize={() => setDetail(id)}
                  onMove={(from, to) => onChange(moveWidget(layout, from, to))}
                />
              );
            }) : STORE_WIDGET_IDS.map((id) => {
              const meta = getWidgetMeta(id);
              const isInstalled = installed.has(id);
              return (
                <article className="wm-card wm-card--store" key={id}>
                  <div className="wm-card-preview"><WidgetPreview id={id} /></div>
                  <div className="wm-card-copy">
                    <div className="wm-card-title-row">
                      <div>
                        <h3>{meta.title}</h3>
                        <p>{meta.description}</p>
                      </div>
                    </div>
                    <div className="wm-card-controls">
                      <button type="button" className="wm-install" onClick={() => toggle(id)}>
                        {isInstalled ? 'Remove' : 'Install'}
                      </button>
                      <button type="button" className="wm-customize" onClick={() => setDetail(id)} disabled={!isInstalled}>
                        Customize
                      </button>
                    </div>
                  </div>
                </article>
              );
            })}
          </div>
          {tab === 'store' ? (
            <p className="wm-store-note">Store widgets are installed into the {PROFILE_LABELS[profile]} profile. Changes apply immediately.</p>
          ) : null}
        </section>
      )}
    </main>
  );
}

export default WidgetManager;
