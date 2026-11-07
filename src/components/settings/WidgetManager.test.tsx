// @vitest-environment jsdom
import { useState } from 'react';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { WidgetManager } from './WidgetManager';
import { defaultLayout, type DockLayout } from '../dock/layout';
import { FINAL_WIDGET_IDS, getWidgetMeta } from '../../widgets/final';

function Harness({ initial = defaultLayout() }: { initial?: DockLayout }) {
  const [layout, setLayout] = useState(initial);
  return <WidgetManager layout={layout} onChange={setLayout} onClose={() => {}} />;
}

function cardFor(title: string) {
  const heading = Array.from(document.querySelectorAll('.wm-card-copy h3')).find((node) => node.textContent === title);
  return heading?.closest('article') as HTMLElement;
}

afterEach(cleanup);

describe('WidgetManager', () => {
  it('shows all fourteen widgets with full preview cards', () => {
    const { container } = render(<Harness />);
    expect(container.querySelectorAll('.wm-grid [data-widget-card]')).toHaveLength(14);
    expect(screen.getByText('Installed widgets')).toBeTruthy();
    expect(screen.getByText(/9 enabled/)).toBeTruthy();
  });

  it('orders the installed grid by the active profile and appends disabled widgets', () => {
    const layout = defaultLayout();
    const active = layout.profiles.daily.map((item) => getWidgetMeta(item.id).title);
    const disabled = FINAL_WIDGET_IDS
      .filter((id) => !layout.profiles.daily.some((item) => item.id === id))
      .map((id) => getWidgetMeta(id).title);
    render(<Harness />);
    const headings = Array.from(document.querySelectorAll('.wm-grid .wm-card-copy h3'))
      .map((node) => node.textContent);
    expect(headings).toEqual([...active, ...disabled]);
  });

  it('installs and removes a store widget in the active profile', () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole('tab', { name: /Widget Store/ }));
    const files = cardFor('Recent Files');
    const customize = () => within(files).getByRole('button', { name: 'Customize' }) as HTMLButtonElement;
    expect(within(files).getByRole('button', { name: 'Install' })).toBeTruthy();
    expect(customize().disabled).toBe(true);
    fireEvent.click(within(files).getByRole('button', { name: 'Install' }));
    expect(within(files).getByRole('button', { name: 'Remove' })).toBeTruthy();
    expect(customize().disabled).toBe(false);
    fireEvent.click(within(files).getByRole('button', { name: 'Remove' }));
    expect(within(files).getByRole('button', { name: 'Install' })).toBeTruthy();
    expect(customize().disabled).toBe(true);
  });

  it('enables and disables an installed widget immediately', () => {
    render(<Harness />);
    const clock = cardFor('Clock');
    const toggle = within(clock).getByRole('switch', { name: 'Disable Clock' });
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    fireEvent.click(toggle);
    expect(within(clock).getByRole('switch', { name: 'Enable Clock' }).getAttribute('aria-checked')).toBe('false');
  });

  it('persists per-widget settings and resets settings and size to defaults', () => {
    render(<Harness />);
    fireEvent.click(within(cardFor('Clock')).getByRole('button', { name: 'Customize' }));
    const seconds = screen.getByRole('checkbox', { name: 'Seconds' }) as HTMLInputElement;
    expect(seconds.checked).toBe(false);
    fireEvent.click(seconds);
    expect((screen.getByRole('checkbox', { name: 'Seconds' }) as HTMLInputElement).checked).toBe(true);
    fireEvent.click(within(screen.getByRole('group', { name: 'Size for Clock' })).getByRole('button', { name: 'Card' }));
    expect(within(screen.getByRole('group', { name: 'Size for Clock' })).getByRole('button', { name: 'Card' }).getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: 'Reset to defaults' }));
    expect((screen.getByRole('checkbox', { name: 'Seconds' }) as HTMLInputElement).checked).toBe(false);
    expect(within(screen.getByRole('group', { name: 'Size for Clock' })).getByRole('button', { name: 'Pill' }).getAttribute('aria-pressed')).toBe('true');
  });

  it('changes size from the widget detail view and keeps install state separate', () => {
    render(<Harness />);
    fireEvent.click(within(cardFor('Weather')).getByRole('button', { name: 'Customize' }));
    const detail = screen.getByRole('group', { name: 'Size for Weather' });
    expect(within(detail).getByRole('button', { name: 'Tile' }).getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(within(detail).getByRole('button', { name: 'Card' }));
    expect(within(screen.getByRole('group', { name: 'Size for Weather' })).getByRole('button', { name: 'Card' }).getAttribute('aria-pressed')).toBe('true');
    const summary = detail.closest('.wm-detail-summary') as HTMLElement;
    expect(within(summary).getByText('Installed')).toBeTruthy();
  });

  it('changes size and dock appearance for the active profile', () => {
    render(<Harness />);
    const weather = cardFor('Weather');
    fireEvent.click(within(weather).getByRole('button', { name: 'Card' }));
    expect(within(weather).getByRole('button', { name: 'Card' }).getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: 'Lines' }));
    expect(screen.getByRole('button', { name: 'Lines' }).getAttribute('aria-pressed')).toBe('true');
  });

  it('switches the manager to another profile', () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole('button', { name: 'Focus' }));
    expect(screen.getByText(/Focus profile/)).toBeTruthy();
    expect(screen.getByText('5 enabled')).toBeTruthy();
  });
});
