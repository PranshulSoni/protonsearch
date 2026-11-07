import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { isNative, native } from '../../services/native';
import { DockWidget } from '../../widgets/final/DockWidget';
import { WidgetCard } from '../../widgets/final/WidgetCard';
import { getWidgetMeta } from '../../widgets/final/meta';
import { useFinalWidgetData } from '../../widgets/final/useFinalWidgetData';
import type { FinalWidgetId, FinalWidgetSettings } from '../../widgets/final/types';
import type { WidgetSize } from '../../widgets/sdk/types';
import { performWidgetAction } from './actions';
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

export interface DockSurfaceProps {
  layout: DockLayout;
  onChange: (layout: DockLayout) => void;
  onCustomize: () => void;
  disabled?: boolean;
}

function availableWidth(): number {
  if (typeof window === 'undefined') return 1200;
  const screenWidth = isNative() ? window.screen?.availWidth ?? 0 : 0;
  return Math.max(screenWidth || 0, window.innerWidth || 0);
}

export const DOCK_WIDTHS: Record<FinalWidgetId, Record<WidgetSize, number>> = {
  windows: { compact: 76, standard: 116, wide: 140 },
  search: { compact: 100, standard: 154, wide: 190 },
  apps: { compact: 94, standard: 168, wide: 204 },
  spotify: { compact: 104, standard: 180, wide: 220 },
  weather: { compact: 96, standard: 140, wide: 180 },
  system: { compact: 116, standard: 184, wide: 216 },
  claude: { compact: 92, standard: 152, wide: 190 },
  chatgpt: { compact: 92, standard: 152, wide: 190 },
  agent: { compact: 92, standard: 164, wide: 204 },
  files: { compact: 92, standard: 158, wide: 196 },
  folders: { compact: 92, standard: 158, wide: 196 },
  trash: { compact: 78, standard: 104, wide: 134 },
  volume: { compact: 90, standard: 124, wide: 158 },
  clock: { compact: 84, standard: 112, wide: 142 },
};

export const DOCK_DENSE_WIDTHS: Record<FinalWidgetId, number> = {
  windows: 68,
  search: 104,
  apps: 112,
  spotify: 120,
  weather: 88,
  system: 122,
  claude: 104,
  chatgpt: 104,
  agent: 96,
  files: 104,
  folders: 104,
  trash: 68,
  volume: 84,
  clock: 68,
};

function railWidth(items: DockItem[], dense: boolean): number {
  const widths = items.reduce((total, item) => (
    total + (dense ? DOCK_DENSE_WIDTHS[item.id] : DOCK_WIDTHS[item.id][effectiveSize(item)])
  ), 0);
  return widths + Math.max(0, items.length - 1) * 5 + 16;
}

function useDockWidth() {
  const [width, setWidth] = useState(availableWidth);
  useEffect(() => {
    const update = () => setWidth(availableWidth());
    window.addEventListener('resize', update);
    return () => window.removeEventListener('resize', update);
  }, []);
  return width;
}

export function DockSurface({ layout, onChange, onCustomize, disabled = false }: DockSurfaceProps) {
  const items = layout.profiles[layout.profile];
  const width = useDockWidth();
  const dense = width < 1366 || railWidth(items, false) + 8 > width;
  const preview = !isNative() && new URLSearchParams(window.location.search).get('preview') === '1';
  const settings: Partial<Record<DockItem['id'], FinalWidgetSettings>> = {};
  for (const item of items) settings[item.id] = effectiveSettings(item);
  const views = useFinalWidgetData(settings, { preview });
  const railRef = useRef<HTMLDivElement>(null);
  const pillsRef = useRef<HTMLDivElement>(null);
  const [previewCard, setPreviewCard] = useState<DockItem | null>(null);

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
      const left = Math.min(railRect.left, pillsRect.left);
      const contentRight = Math.max(
        pillsRect.right,
        railRect.left + Math.max(railRect.width, rail.scrollWidth),
      );
      const widthRequest = Math.ceil(contentRight - left) + 32;
      const origin = 16;
      const regions = [
        {
          x: origin + (pillsRect.left - left),
          y: Math.max(0, Math.round(pillsRect.top)),
          width: Math.ceil(pillsRect.width),
          height: Math.ceil(pillsRect.height),
          radius: 15,
        },
        {
          x: origin + (railRect.left - left),
          y: Math.max(0, Math.round(railRect.top)),
          width: Math.ceil(Math.max(railRect.width, rail.scrollWidth)),
          height: Math.ceil(railRect.height),
          radius: 18,
        },
      ];
      void native.fitDock(widthRequest, 132, regions).catch(() => {});
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
  }, [layout, dense]);

  const appearance: AppearanceMode = layout.appearances?.[layout.profile] ?? 'capsules';
  const visibleRail = useMemo(() => railWidth(items, dense), [dense, items]);

  return (
    <div className="dock-final" data-appearance={appearance} aria-label="ProtonDock">
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
        className={`dock-final-rail${dense ? ' is-dense' : ''}`}
        style={{ width: Math.min(visibleRail, Math.max(0, width - 8)) }}
        data-profile={layout.profile}
        ref={railRef}
      >
        {items.length ? items.map((item) => {
          const meta = getWidgetMeta(item.id);
          const view = views[item.id];
          return (
            <div
              className="dock-final-slot"
              key={item.id}
              data-widget-id={item.id}
              role="button"
              tabIndex={disabled ? -1 : 0}
              aria-label={`Open ${meta.title} card`}
              onClick={(event) => {
                if (disabled) return;
                if ((event.target as HTMLElement).closest('button')) return;
                openCard(item, event.currentTarget);
              }}
              onKeyDown={(event) => {
                if (disabled || event.target !== event.currentTarget) return;
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  openCard(item, event.currentTarget);
                }
              }}
            >
              <DockWidget
                id={item.id}
                settings={settings[item.id]}
                data={view.data}
                preview={view.preview}
              />
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
