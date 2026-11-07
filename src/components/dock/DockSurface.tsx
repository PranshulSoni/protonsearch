import { useCallback, useEffect, useRef, useState } from 'react';
import { isNative, native } from '../../services/native';
import { DockWidget, dockActionFor } from '../../widgets/final/DockWidget';
import { WidgetCard } from '../../widgets/final/WidgetCard';
import { getWidgetMeta, settingsFor } from '../../widgets/final/meta';
import { sampleFor } from '../../widgets/final/sample';
import { useFinalWidgetData } from '../../widgets/final/useFinalWidgetData';
import type { AppsData, FinalWidgetSettings } from '../../widgets/final/types';
import { performWidgetAction } from './actions';
import { DockGroup, type DockGroupItem } from './DockGroup';
import { dockFit } from './fit';
import {
  PROFILE_IDS,
  PROFILE_LABELS,
  effectiveSettings,
  effectiveSize,
  type AppearanceMode,
  type DockItem,
  type DockLayout,
  type ProfileId,
} from './layout';
import { useMagnify } from './magnify';
import { DEFAULT_TILE, largestTile, slotStyle, type TileSize } from './sizing';

export interface DockSurfaceProps {
  layout: DockLayout;
  onChange: (layout: DockLayout) => void;
  onCustomize: () => void;
  disabled?: boolean;
}

function sizeOf(_item: DockItem): TileSize {
  return DEFAULT_TILE;
}

export function DockSurface({ layout, onChange, onCustomize, disabled = false }: DockSurfaceProps) {
  const items = layout.profiles[layout.profile];
  const preview = !isNative() && new URLSearchParams(window.location.search).get('preview') === '1';
  const settings: Partial<Record<DockItem['id'], FinalWidgetSettings>> = {};
  for (const item of items) settings[item.id] = effectiveSettings(item);
  const views = useFinalWidgetData(settings, { preview });
  const railRef = useRef<HTMLDivElement>(null);
  const pillsRef = useRef<HTMLDivElement>(null);
  const [previewCard, setPreviewCard] = useState<DockItem | null>(null);
  const [hot, setHot] = useState(false);
  const cool = useRef(0);
  useMagnify(railRef, 'bottom', 'center', true);
  const heat = () => {
    clearTimeout(cool.current);
    setHot(true);
  };
  const relax = () => {
    clearTimeout(cool.current);
    cool.current = window.setTimeout(() => setHot(false), 240);
  };
  useEffect(() => () => clearTimeout(cool.current), []);

  const selectProfile = (profile: ProfileId) => {
    if (profile !== layout.profile) onChange({ ...layout, profile });
  };

  const openCard = useCallback((item: DockItem, element: HTMLElement) => {
    const rect = element.getBoundingClientRect();
    const settingsForItem = effectiveSettings(item);
    if (isNative()) {
      void native.openWidgetCard({
        id: item.id,
        size: effectiveSize(item),
        settings: settingsForItem,
        anchor: { x: rect.left, y: rect.top, width: rect.width, height: rect.height },
      }).catch(() => {});
      return;
    }
    setPreviewCard(item);
  }, []);

  const activate = (item: DockItem, element: HTMLElement) => {
    const primary = dockActionFor(item.id);
    if (primary) void performWidgetAction(primary).catch(() => {});
    else openCard(item, element);
  };

  useEffect(() => {
    if (!isNative()) return;
    let cancelled = false;
    const fit = () => {
      if (cancelled) return;
      const rail = railRef.current;
      const pills = pillsRef.current;
      if (!rail || !pills) return;
      const railRect = rail.getBoundingClientRect();
      const pillsRect = pills.getBoundingClientRect();
      if (!railRect.width || !pillsRect.width) return;
      const next = dockFit(
        'bottom',
        'center',
        pillsRect,
        {
          left: railRect.left,
          top: railRect.top,
          width: Math.max(railRect.width, rail.scrollWidth),
          height: Math.max(railRect.height, rail.scrollHeight),
        },
        { width: window.innerWidth, height: window.innerHeight },
        hot,
        largestTile(items.map(sizeOf)),
      );
      void native.fitDock(next.width, next.height, next.regions).catch(() => {});
    };
    const frame = requestAnimationFrame(fit);
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(fit);
    if (railRef.current) observer?.observe(railRef.current);
    if (pillsRef.current) observer?.observe(pillsRef.current);
    window.addEventListener('resize', fit);
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
      observer?.disconnect();
      window.removeEventListener('resize', fit);
    };
  }, [layout, hot]);

  const appearance: AppearanceMode = layout.appearances?.[layout.profile] ?? 'capsules';

  return (
    <div className="dock-final" data-appearance={appearance} data-hot={hot ? 'true' : undefined} aria-label="ProtonDock">
      <div className="dock-final-shelf" ref={pillsRef}>
        <div className="dock-final-pills" role="tablist" aria-label="Dock profile">
          {PROFILE_IDS.map((profile) => (
            <button
              key={profile}
              type="button"
              role="tab"
              aria-selected={layout.profile === profile}
              className="dock-final-pill"
              onClick={() => selectProfile(profile)}
            >
              {PROFILE_LABELS[profile]}
            </button>
          ))}
        </div>
        <button type="button" className="dock-final-customize" onClick={onCustomize}>
          Customize Widgets
        </button>
      </div>
      <div
        className="dock-final-rail"
        data-profile={layout.profile}
        ref={railRef}
        onPointerEnter={heat}
        onPointerLeave={relax}
        onFocus={heat}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node | null)) relax();
        }}
      >
        {items.length ? items.map((item) => {
          const meta = getWidgetMeta(item.id);
          const view = views[item.id];
          if (item.id === 'apps') {
            const s = settingsFor('apps', item.settings);
            const apps = ((view.preview ? sampleFor('apps') : view.data) as AppsData | undefined)?.items ?? [];
            const running: DockGroupItem[] = apps
              .filter((app) => !s.activeOnly || app.active)
              .slice(0, s.maxApps)
              .map((app) => ({ id: app.id, title: app.title, icon: app.icon, running: true }));
            return (
              <div
                className="dock-final-slot dock-final-slot--group"
                key={item.id}
                data-widget-id="apps"
                style={slotStyle(sizeOf(item))}
              >
                <DockGroup
                  label={meta.title}
                  placeholder="apps"
                  items={running}
                  disabled={disabled}
                  onOpen={(id) => void performWidgetAction(id).catch(() => {})}
                  onCard={(anchor) => openCard(item, anchor)}
                />
              </div>
            );
          }
          const primary = dockActionFor(item.id);
          return (
            <div
              className="dock-final-slot dock-cell"
              key={item.id}
              data-widget-id={item.id}
              role="button"
              tabIndex={disabled ? -1 : 0}
              aria-label={primary === 'start' ? 'Open Start' : primary === 'search' ? 'Open Spotlight search' : `Open ${meta.title} card`}
              style={slotStyle(sizeOf(item))}
              onClick={(event) => {
                if (disabled) return;
                activate(item, event.currentTarget);
              }}
              onKeyDown={(event) => {
                if (disabled || event.target !== event.currentTarget) return;
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  activate(item, event.currentTarget);
                }
              }}
              onContextMenu={(event) => {
                event.preventDefault();
                openCard(item, event.currentTarget);
              }}
            >
              <DockWidget
                id={item.id}
                settings={settings[item.id]}
                data={view.data}
                preview={view.preview}
              />
              <span className="dock-tip" aria-hidden="true">{meta.title}</span>
            </div>
          );
        }) : <span className="dock-final-empty">No widgets enabled</span>}
      </div>
      {previewCard ? (
        <div
          className="dock-final-overlay"
          onClick={(event) => {
            if (event.target === event.currentTarget) setPreviewCard(null);
          }}
        >
          <button
            type="button"
            className="dock-final-overlay-close"
            aria-label={`Close ${getWidgetMeta(previewCard.id).title} card`}
            onClick={() => setPreviewCard(null)}
          >
            ×
          </button>
          <div
            className="dock-final-overlay-card"
            role="dialog"
            aria-modal="true"
            aria-label={`${getWidgetMeta(previewCard.id).title} card`}
          >
            <WidgetCard
              id={previewCard.id}
              settings={effectiveSettings(previewCard)}
              data={views[previewCard.id].data}
              preview={views[previewCard.id].preview}
              status={views[previewCard.id].status}
              onAction={(action) => { void performWidgetAction(action).catch(() => {}); }}
            />
          </div>
        </div>
      ) : null}
    </div>
  );
}

export default DockSurface;
