import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { isNative, native } from '../../services/native';
import { DockWidget } from '../../widgets/final/DockWidget';
import { WidgetCard } from '../../widgets/final/WidgetCard';
import { getWidgetMeta } from '../../widgets/final/meta';
import { useFinalWidgetData } from '../../widgets/final/useFinalWidgetData';
import type { FinalWidgetSettings } from '../../widgets/final/types';
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

function railWidth(items: DockItem[], dense: boolean): number {
  const widths = items.reduce((total, item) => {
    const size: WidgetSize = dense ? 'compact' : effectiveSize(item);
    return total + getWidgetMeta(item.id).widths[size];
  }, 0);
  const gap = dense ? 6 : 8;
  return widths + Math.max(0, items.length - 1) * gap + (dense ? 18 : 22);
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
  const dense = width < 1460 || railWidth(items, false) + 32 > width - 32;
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
          radius: 14,
        },
        {
          x: origin + (railRect.left - left),
          y: Math.max(0, Math.round(railRect.top)),
          width: Math.ceil(Math.max(railRect.width, rail.scrollWidth)),
          height: Math.ceil(railRect.height),
          radius: 20,
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
      <div className="dock-final-pills" role="tablist" aria-label="Dock profile" ref={pillsRef}>
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
        <span className="dock-final-pill-divider" aria-hidden="true" />
        <button type="button" className="dock-final-customize" onClick={onCustomize}>
          Customize Widgets
        </button>
      </div>
      <div
        className={`dock-final-rail${dense ? ' is-dense' : ''}`}
        style={{ width: Math.min(visibleRail, Math.max(0, width - 24)) }}
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
                size={dense ? 'compact' : effectiveSize(item)}
                settings={settings[item.id]}
                data={view.data}
                preview={view.preview}
                status={view.status}
                onAction={(action) => { void performWidgetAction(action).catch(() => {}); }}
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
