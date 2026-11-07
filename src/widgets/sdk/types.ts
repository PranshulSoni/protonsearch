import type { ComponentType, ReactNode } from 'react';
export type WidgetSize = 'compact' | 'standard' | 'wide';
export type WidgetStatus = 'loading' | 'live' | 'empty' | 'stale' | 'disconnected' | 'permission-denied' | 'error';
export type ProviderId = 'none' | 'system' | 'storage' | 'network' | 'media' | 'clipboard' | 'running-apps' | 'volume' | 'trash' | 'known-folders';
export interface Snapshot<T = unknown> { status: WidgetStatus; data?: T; updatedAt: number; message?: string }
export interface WidgetViewProps<T = unknown, S = Record<string, unknown>> {
  snapshot: Snapshot<T>; settings: S; size: WidgetSize;
  onAction: (action: string, payload?: unknown) => void;
  onSettingsChange?: (settings: Record<string, unknown>) => void;
}
export interface WidgetDefinition<T = unknown, S = Record<string, unknown>> {
  id: string; version: number; title: string; category: string; description: string; icon: string;
  sizes: readonly WidgetSize[]; defaultSize: WidgetSize; defaultWidth: number; minWidth: number; maxWidth: number;
  sizeWidths?: Partial<Record<WidgetSize, number>>;
  profiles: readonly ('daily' | 'founder')[]; permissions: readonly string[]; providerId: ProviderId;
  refresh: { mode: 'poll' | 'manual' | 'event'; intervalMs?: number }; defaults: S;
  validateSettings: (settings: unknown) => settings is S;
  actions: readonly string[]; Component: ComponentType<WidgetViewProps<T, S>>; Preview: ComponentType;
}
export interface WidgetInstance { id: string; size?: WidgetSize; settings?: Record<string, unknown>; width?: number }
export type Children = { children?: ReactNode };
