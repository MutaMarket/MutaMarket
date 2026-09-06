import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import SearchAlertsCard from './search-alerts-card.svelte';
import { toIsk } from '$lib/format-number';

describe('the search alerts card', () => {
  it('breaks each alert down into its criteria and history', async () => {
    const onRemove = vi.fn();
    const screen = await render(SearchAlertsCard, {
      alerts: [
        {
          id: 7,
          query: 'type/47408/attributes/cpu/10-20/contract-price/1000000.00/goldbar/in-jita',
          type: { id: 47408, name: '50MN Abyssal Microwarpdrive' },
          created_at: '2026-09-01T10:00:00Z',
          last_notified_at: '2026-09-05T10:00:00Z',
          notified_count: 3,
        },
        {
          id: 8,
          query: 'type/47702',
          type: { id: 47702, name: 'Abyssal Stasis Webifier' },
          created_at: '2026-09-02T10:00:00Z',
          last_notified_at: null,
          notified_count: 0,
        },
      ],
      hasPremium: true,
      onRemove,
    });
    await expect.element(screen.getByText('50MN Abyssal Microwarpdrive')).toBeInTheDocument();
    const text = screen.baseElement.textContent ?? '';
    expect(text).toContain('cpu between 10 and 20');
    expect(text).toContain(`Contract price up to ${toIsk(1_000_000)}`);
    expect(text).toContain('Goldbar');
    expect(text).toContain('Located in Jita 4-4');
    expect(text).toContain('Created on 1 Sept 2026');
    expect(text).toContain('3 modules reported, last on 5 Sept 2026');
    expect(text).toContain('No matches reported yet');
    expect(text).toContain('Any listing of this type');
    const links = Array.from(screen.baseElement.querySelectorAll('a')).map((link) =>
      link.getAttribute('href'),
    );
    expect(links).toEqual([
      '/modules/type/47408/attributes/cpu/10-20/sort/date-added/desc/contract-price/1000000.00/goldbar/in-jita',
      '/modules/type/47702/sort/date-added/desc',
    ]);
    await screen.getByRole('button', { name: 'Remove' }).first().click();
    expect(onRemove).toHaveBeenCalledWith(7);
  });

  it('explains the bell when there are no alerts', async () => {
    const screen = await render(SearchAlertsCard, {
      alerts: [],
      hasPremium: true,
      onRemove: vi.fn(),
    });
    await expect.element(screen.getByText('Search alerts', { exact: true })).toBeInTheDocument();
    expect(screen.baseElement.textContent).toContain('press the bell');
    expect(screen.baseElement.textContent).toContain('Every five minutes');
    expect(screen.baseElement.querySelector('button')).toBeNull();
    expect(screen.baseElement.querySelector('a[href="/premium"]')).toBeNull();
  });

  it('points an account without premium at the premium page', async () => {
    const screen = await render(SearchAlertsCard, {
      alerts: [],
      hasPremium: false,
      onRemove: vi.fn(),
    });
    await expect.element(screen.getByText('Search alerts', { exact: true })).toBeInTheDocument();
    expect(screen.baseElement.textContent).toContain('Get a message whenever');
    expect(screen.baseElement.querySelector('a[href="/premium"]')?.textContent).toContain(
      'Discover premium',
    );
  });
});
