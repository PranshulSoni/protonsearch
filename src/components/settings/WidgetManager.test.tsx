// @vitest-environment jsdom
import { useState } from 'react';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { WidgetManager } from './WidgetManager';
import { defaultLayout, type DockLayout } from '../dock/layout';

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

  it('installs and removes a store widget in the active profile', () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole('tab', { name: /Widget Store/ }));
    const files = cardFor('Recent Files');
    expect(within(files).getByRole('button', { name: 'Install' })).toBeTruthy();
    fireEvent.click(within(files).getByRole('button', { name: 'Install' }));
    expect(within(files).getByRole('button', { name: 'Remove' })).toBeTruthy();
    fireEvent.click(within(files).getByRole('button', { name: 'Remove' }));
    expect(within(files).getByRole('button', { name: 'Install' })).toBeTruthy();
  });

  it('enables and disables an installed widget immediately', () => {
    render(<Harness />);
    const clock = cardFor('Clock');
    const toggle = within(clock).getByRole('switch', { name: 'Disable Clock' });
    expect(toggle.getAttribute('aria-checked')).toBe('true');
    fireEvent.click(toggle);
    expect(within(clock).getByRole('switch', { name: 'Enable Clock' }).getAttribute('aria-checked')).toBe('false');
  });

  it('persists per-widget settings and resets them to defaults', () => {
    render(<Harness />);
    fireEvent.click(within(cardFor('Clock')).getByRole('button', { name: 'Customize' }));
    const seconds = screen.getByRole('checkbox', { name: 'Seconds' }) as HTMLInputElement;
    expect(seconds.checked).toBe(false);
    fireEvent.click(seconds);
    expect((screen.getByRole('checkbox', { name: 'Seconds' }) as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Reset to defaults' }));
    expect((screen.getByRole('checkbox', { name: 'Seconds' }) as HTMLInputElement).checked).toBe(false);
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
