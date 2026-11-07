import type { WidgetDefinition, WidgetInstance, WidgetSize } from './types';

export function widgetSize(definition: WidgetDefinition, requested?: WidgetSize): WidgetSize {
  return requested && definition.sizes.includes(requested) ? requested : definition.defaultSize;
}

export function widgetWidth(definition: WidgetDefinition, instance: WidgetInstance): number {
  const size = widgetSize(definition, instance.size);
  return Math.min(definition.maxWidth, Math.max(definition.minWidth,
    instance.width ?? definition.sizeWidths?.[size] ?? definition.defaultWidth));
}
