// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DockGroup, type DockGroupItem } from './DockGroup';

const apps: DockGroupItem[] = [
  { id: 'chrome', title: 'Chrome', icon: 'data:image/png;base64,iVBORw0KGgo=', running: true },
  { id: 'code', title: 'Code', icon: 'https://x/y.png', running: false },
];

afterEach(cleanup);

describe('DockGroup', () => {
  it('renders safe icons, tooltips and running dots', () => {
    const { container } = render(
      <DockGroup label="Apps" placeholder="apps" items={apps} onOpen={() => {}} onCard={() => {}} />,
    );
    expect(container.querySelectorAll('img.fw-app-img')).toHaveLength(1);
    expect(container.querySelectorAll('.dock-dot')).toHaveLength(1);
    expect(Array.from(container.querySelectorAll('.dock-tip'), (tip) => tip.textContent)).toEqual(['Chrome', 'Code']);
  });

  it('click opens and right-click asks for the card', () => {
    const onOpen = vi.fn();
    const onCard = vi.fn();
    render(<DockGroup label="Apps" placeholder="apps" items={apps} onOpen={onOpen} onCard={onCard} />);
    fireEvent.click(screen.getByRole('button', { name: 'Chrome' }));
    expect(onOpen).toHaveBeenCalledWith('chrome');
    fireEvent.contextMenu(screen.getByRole('button', { name: 'Code' }));
    expect(onCard).toHaveBeenCalledWith(screen.getByRole('button', { name: 'Code' }));
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it('empty group shows a placeholder that opens the card', () => {
    const onCard = vi.fn();
    render(<DockGroup label="Apps" placeholder="apps" items={[]} onOpen={() => {}} onCard={onCard} />);
    fireEvent.click(screen.getByRole('button', { name: 'Open Apps card' }));
    expect(onCard).toHaveBeenCalledTimes(1);
  });
});
