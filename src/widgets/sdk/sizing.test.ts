import { expect, test } from 'vitest';
import { catalog } from '../catalog';
import { widgetSize, widgetWidth } from './sizing';

test('designed sizes have ordered widths and respect widget bounds', () => {
  for (const definition of catalog) {
    const widths = definition.sizes.map((size) => widgetWidth(definition, { id: definition.id, size }));
    expect(widths).toEqual([...widths].sort((a, b) => a - b));
    expect(widgetWidth(definition, { id: definition.id, width: 1 })).toBe(definition.minWidth);
    expect(widgetWidth(definition, { id: definition.id, width: 9999 })).toBe(definition.maxWidth);
  }
});

test('unsupported legacy sizes fall back without changing the saved instance', () => {
  const definition = catalog.find((widget) => widget.id === 'windows')!;
  const saved = { id: 'windows', size: 'wide' as const, width: 120 };
  expect(widgetSize(definition, saved.size)).toBe('standard');
  expect(widgetWidth(definition, saved)).toBe(120);
  expect(saved).toEqual({ id: 'windows', size: 'wide', width: 120 });
});
