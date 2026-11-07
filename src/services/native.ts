import { invoke } from '@tauri-apps/api/core';
import type { FinalWidgetId } from '../widgets/final/types';
import type { ProviderId, Snapshot, WidgetSize } from '../widgets/sdk/types';

export interface SearchResult { id: string; title: string; detail: string; type: string; actions: string[] }
export interface SearchResponse { requestId: number; results: SearchResult[]; elapsedMs: number }

export type WidgetActionId =
  | 'media:toggle'
  | 'media:next'
  | 'media:previous'
  | 'volume:up'
  | 'volume:down'
  | 'volume:mute'
  | 'trash:empty'
  | 'settings:sound'
  | 'settings:network'
  | 'settings:bluetooth'
  | 'settings:notifications'
  | `folder:open:${string}`
  | `volume:set:${number}`;

export interface WidgetCardAnchor {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WidgetCardRequest {
  id: FinalWidgetId;
  size: WidgetSize;
  settings?: unknown;
  anchor?: WidgetCardAnchor;
}

export interface WidgetCardState {
  id: FinalWidgetId;
  size: WidgetSize;
  settings?: unknown;
}

export interface DockRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  radius?: number;
}

export const isNative = () => '__TAURI_INTERNALS__' in window;

export const native = {
  search: (query: string, limit: number, requestId: number) => invoke<SearchResponse>('search', { query, limit, requestId }),
  execute: (resultId: string, actionId?: string) => invoke<void>('execute_search_result', { resultId, actionId }),
  openSearch: (query = '') => invoke<void>('open_search', { query }),
  hideSearch: () => invoke<void>('hide_search'),
  openStart: () => invoke<void>('open_start'),
  openSettings: () => invoke<void>('open_settings'),
  hideSettings: () => invoke<void>('hide_settings'),
  loadSettings: () => invoke<unknown>('load_dock_settings'),
  saveSettings: (value: unknown) => invoke<void>('save_dock_settings', { value }),
  snapshot: (providerId: ProviderId) => providerId === 'clipboard'
    ? invoke<Snapshot>('clipboard_snapshot')
    : providerId === 'running-apps' ? invoke<Snapshot>('running_apps_snapshot')
    : providerId === 'known-folders' ? invoke<Snapshot>('known_folders_snapshot')
    : invoke<Snapshot>('provider_snapshot', { providerId }),
  activateRunningApp: (id: string) => invoke<void>('running_apps_activate', { id }),
  widgetAction: (actionId: WidgetActionId) => invoke<void>('widget_action', { actionId }),
  fitDock: (width: number, height: number, regions: DockRegion[]) =>
    invoke<void>('fit_dock', { width, height, regions }),
  openWidgetCard: (request: WidgetCardRequest) => invoke<void>('open_widget_card', {
    id: request.id,
    size: request.size,
    settings: request.settings,
    anchor: request.anchor,
  }),
  hideWidgetCard: () => invoke<void>('hide_widget_card'),
  widgetCardState: () => invoke<WidgetCardState | null>('widget_card_state'),
};
