import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const invalidateAll = vi.fn();
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll }));

const SearchAlertsControl = (await import('./search-alerts-control.svelte')).default;

import { defaultUiSearch } from '$lib/query';

const alerts = [
  {
    id: 7,
    query: 'type/47408/goldbar',
    type: { id: 47408, name: '50MN Abyssal Microwarpdrive' },
    created_at: '2026-09-01T10:00:00Z',
    last_notified_at: '2026-09-05T10:00:00Z',
    notified_count: 3,
  },
  {
    id: 8,
    query: 'type/47702',
    type: { id: 47702, name: 'Abyssal Stasis Webifier' },
    created_at: '2026-09-01T10:00:00Z',
    last_notified_at: null,
    notified_count: 0,
  },
];

afterEach(() => {
  invalidateAll.mockReset();
  vi.restoreAllMocks();
});

describe('search-alerts-control.svelte', () => {
  it('needs a type before the bell can save', async () => {
    const screen = await render(SearchAlertsControl, {
      search: defaultUiSearch(),
      alerts: [],
      hasPremium: true,
    });
    const bell = screen.getByRole('button', { name: 'Alert me' });
    await expect.element(bell).toHaveAttribute('aria-disabled', 'true');
    await bell.hover();
    await expect
      .element(screen.getByText('Pick a module type to save an alert', { exact: true }).first())
      .toBeInTheDocument();
  });

  it('saves the normalized query from the bell and refreshes', async () => {
    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response('{}', { status: 201 }));
    const screen = await render(SearchAlertsControl, {
      search: { ...defaultUiSearch(), typeSlug: '47408', goldbar: true, page: 2 },
      alerts: [],
      hasPremium: true,
    });
    const bell = screen.getByRole('button', { name: 'Alert me' });
    await expect.element(bell).toHaveAttribute('aria-pressed', 'false');
    await bell.click();
    expect(fetchMock).toHaveBeenCalledWith('/search-alerts', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ query: 'type/47408/goldbar' }),
      redirect: 'manual',
    });
    await vi.waitFor(() => expect(invalidateAll).toHaveBeenCalled());
  });

  it('removes the alert holding the current query from the bell', async () => {
    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response(null, { status: 204 }));
    const screen = await render(SearchAlertsControl, {
      search: { ...defaultUiSearch(), typeSlug: '47408', goldbar: true, sort: ['price', true] },
      alerts,
      hasPremium: true,
    });
    const bell = screen.getByRole('button', { name: 'Alert on' });
    await expect.element(bell).toHaveAttribute('aria-pressed', 'true');
    await bell.click();
    expect(fetchMock).toHaveBeenCalledWith('/search-alerts/7', {
      method: 'DELETE',
      redirect: 'manual',
    });
    await vi.waitFor(() => expect(invalidateAll).toHaveBeenCalled());
  });

  it('does not refresh when the server rejects the save', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(
        JSON.stringify({
          message: 'The given data was invalid.',
          errors: { query: ['You can save at most 10 search alerts.'] },
        }),
        { status: 422 },
      ),
    );
    const screen = await render(SearchAlertsControl, {
      search: { ...defaultUiSearch(), typeSlug: '47408' },
      alerts: [],
      hasPremium: true,
    });
    await screen.getByRole('button', { name: 'Alert me' }).click();
    await vi.waitFor(() => expect(fetch).toHaveBeenCalled());
    expect(invalidateAll).not.toHaveBeenCalled();
  });

  it('pitches premium instead of saving when the account has none', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch');
    const screen = await render(SearchAlertsControl, {
      search: { ...defaultUiSearch(), typeSlug: '47408' },
      alerts: [],
      hasPremium: false,
    });
    await screen.getByRole('button', { name: 'Alert me' }).click();
    await expect
      .element(screen.getByText('Search alerts are a premium feature', { exact: true }))
      .toBeInTheDocument();
    expect(document.querySelector('a[href="/premium"]')?.textContent).toContain('Discover premium');
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('counts the alerts on the chevron and lists them with their search links', async () => {
    const screen = await render(SearchAlertsControl, {
      search: defaultUiSearch(),
      alerts,
      hasPremium: true,
    });
    const list = screen.getByRole('button', { name: 'Search alerts' });
    expect(list.element().textContent).toContain('2');
    await list.click();
    await expect.element(screen.getByText('50MN Abyssal Microwarpdrive')).toBeInTheDocument();
    const links = Array.from(document.querySelectorAll('a')).map((link) =>
      link.getAttribute('href'),
    );
    expect(links).toEqual([
      '/modules/type/47408/sort/date-added/desc/goldbar',
      '/modules/type/47702/sort/date-added/desc',
    ]);
    expect(document.body.textContent).toContain('goldbar');
    expect(document.body.textContent).toContain('3 modules reported, last on 5 Sept 2026');
    expect(document.body.textContent).toContain('No matches reported yet');
  });

  it('removes an alert from the list and refreshes', async () => {
    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response(null, { status: 204 }));
    const screen = await render(SearchAlertsControl, {
      search: defaultUiSearch(),
      alerts,
      hasPremium: true,
    });
    await screen.getByRole('button', { name: 'Search alerts' }).click();
    await screen.getByRole('button', { name: 'Remove' }).first().click();
    expect(fetchMock).toHaveBeenCalledWith('/search-alerts/7', {
      method: 'DELETE',
      redirect: 'manual',
    });
    await vi.waitFor(() => expect(invalidateAll).toHaveBeenCalled());
  });

  it('explains the bell when there are no alerts', async () => {
    const screen = await render(SearchAlertsControl, {
      search: defaultUiSearch(),
      alerts: [],
      hasPremium: true,
    });
    const list = screen.getByRole('button', { name: 'Search alerts' });
    expect(list.element().textContent?.trim()).toBe('');
    await list.click();
    await vi.waitFor(() => expect(document.body.textContent).toContain('press the bell'));
  });
});
