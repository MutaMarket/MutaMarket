import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

const ErrorsPage = (await import('./+page.svelte')).default;

const failures = [
  {
    id: 2,
    occurred_at: '2026-09-10 14:04:10.757061+00',
    route: 'GET /api/search-alerts',
    method: 'GET',
    path: '/api/search-alerts',
    status: 401,
    error_message: 'Unauthenticated.',
    duration_ms: 3,
    user_id: null,
    user_name: null,
  },
  {
    id: 1,
    occurred_at: '2026-09-10 14:03:10.757061+00',
    route: 'GET /api/modules/{module}',
    method: 'GET',
    path: '/api/modules/x-1',
    status: 500,
    error_message: 'Internal server error.',
    duration_ms: 40,
    user_id: 7,
    user_name: 'Wolfgang',
  },
];

const payload = {
  failures,
  routes: [
    { route: 'GET /api/search-alerts', failures: 12, last_at: null, statuses: [401] },
    { route: 'GET /api/modules/{module}', failures: 1, last_at: null, statuses: [500] },
  ],
  keep: 2000,
  retention_days: 7,
  captures_per_minute: 3,
};

function renderPage(seed: { route?: string | null; statusClass?: string | null } = {}) {
  const data = { failures: payload, route: null, statusClass: null, ...seed };
  return render(ErrorsPage, { data } as never);
}

describe('the admin errors page', () => {
  it('lists the failing routes and the captured failures behind them', async () => {
    const screen = await renderPage();

    await expect.element(screen.getByText('/api/modules/x-1')).toBeInTheDocument();
    await expect.element(screen.getByText('Unauthenticated.')).toBeInTheDocument();
    // The account column: the signed-in name, and that nobody was.
    await expect.element(screen.getByText(/Wolfgang/)).toBeInTheDocument();
    await expect.element(screen.getByText(/Guest/)).toBeInTheDocument();
    // What the table is bounded by, so the page never looks complete.
    await expect.element(screen.getByText('newest 2,000 · 7 days')).toBeInTheDocument();
  });

  it('filters by route through the API', async () => {
    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(
        new Response(JSON.stringify({ ...payload, failures: [failures[1]] }), { status: 200 }),
      );
    const screen = await renderPage();

    await screen.getByRole('button', { name: /GET \/api\/modules\/\{module\}/ }).click();
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/admin/request-failures?route=GET+%2Fapi%2Fmodules%2F%7Bmodule%7D',
    );
  });

  it('starts filtered when the activity page linked to one route', async () => {
    const screen = await renderPage({ route: 'GET /api/search-alerts' });

    // The chip that clears it, carrying the route the link asked for.
    await expect
      .element(screen.getByRole('button', { name: 'GET /api/search-alerts ✕' }))
      .toBeInTheDocument();
  });

  it('filters by status class through the API', async () => {
    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(
        new Response(JSON.stringify({ ...payload, failures: [] }), { status: 200 }),
      );
    const screen = await renderPage();

    await screen.getByRole('button', { name: '5xx' }).click();
    expect(fetchMock).toHaveBeenCalledWith('/api/admin/request-failures?class=server');
    // An empty answer says the filter is what emptied it.
    await expect
      .element(screen.getByText('No captured failure matches this filter.'))
      .toBeInTheDocument();
  });
});
