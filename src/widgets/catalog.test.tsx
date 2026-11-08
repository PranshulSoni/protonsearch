// @vitest-environment jsdom
import { afterEach, expect, test, vi } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/react';
import { catalog } from './catalog';
import type { WidgetStatus } from './sdk/types';

afterEach(() => { cleanup(); vi.useRealTimers(); });
const states: WidgetStatus[] = ['loading', 'live', 'empty', 'stale', 'disconnected', 'permission-denied', 'error'];
for (const definition of catalog) test(`${definition.id} contract and state/size render smoke`, () => {
  expect(definition.id).toMatch(/^[a-z][a-z0-9-]*$/);
  expect(definition.title.trim()).not.toBe('');
  expect(definition.sizes).toContain(definition.defaultSize);
  expect(definition.minWidth).toBeGreaterThan(0);
  expect(definition.defaultWidth).toBeGreaterThanOrEqual(definition.minWidth);
  expect(definition.maxWidth).toBeGreaterThanOrEqual(definition.defaultWidth);
  expect(definition.validateSettings(definition.defaults)).toBe(true);
  expect(definition.validateSettings(null)).toBe(false);
  const Component = definition.Component;
  for (const status of states) for (const size of definition.sizes) {
    const view = render(<Component snapshot={{status, updatedAt: 0}} settings={definition.defaults} size={size} onAction={() => {}} />);
    expect(view.container.firstChild).not.toBeNull();
    expect(view.container.textContent).not.toMatch(/\bDEMO\b/);
    view.unmount();
  }
});

test('clock releases its timer on unmount', () => {
  vi.useFakeTimers();
  const Clock = catalog.find(widget => widget.id === 'time')!.Component;
  const view = render(<Clock snapshot={{status: 'live', updatedAt: 0}} settings={{}} size="standard" onAction={() => {}} />);
  expect(vi.getTimerCount()).toBe(1);
  view.unmount();
  expect(vi.getTimerCount()).toBe(0);
});

test('clipboard widget routes its pointer action to authoritative search', () => {
  const Clipboard = catalog.find(widget => widget.id === 'clipboard')!.Component;
  const onAction = vi.fn();
  const view = render(<Clipboard snapshot={{status: 'live', updatedAt: 0}} settings={{}} size="standard" onAction={onAction} />);
  fireEvent.click(view.getByRole('button', { name: 'Browse saved clipboard history' }));
  expect(onAction).toHaveBeenCalledWith('search', 'clip:');
});
test('system size variants change information density without scaling metrics', () => {
  const Component = catalog.find((widget) => widget.id === 'status')!.Component;
  for (const [size, labels] of [['compact', ['CPU']], ['standard', ['CPU', 'RAM']], ['wide', ['CPU', 'RAM', 'BAT']]] as const) {
    const view = render(<Component snapshot={{ status: 'live', updatedAt: 1, data: { cpu: 12, ram: 34, battery: 56 } }} settings={{}} size={size} onAction={() => {}} />);
    for (const label of ['CPU', 'RAM', 'BAT']) expect(Boolean(view.queryByText(label))).toBe(labels.some((expected) => expected === label));
    view.unmount();
  }
});
test.each([['weather', 'Offline'], ['media', 'Offline'], ['calendar', 'Unlinked']])('compact %s keeps an explicit provider-state marker', (id, label) => {
  const Component = catalog.find((widget) => widget.id === id)!.Component;
  const view = render(<Component snapshot={{ status: 'disconnected', updatedAt: 0 }} settings={{}} size="compact" onAction={() => {}} />);
  expect(view.getByText(label).getAttribute('role')).toBe('status');
  expect(view.getByText(label).className).toBe('compact-provider-state');
});
