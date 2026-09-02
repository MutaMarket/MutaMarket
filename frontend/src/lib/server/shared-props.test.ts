import { describe, expect, it } from 'vitest';

import { loadSharedProps } from './shared-props';

function fetchStub(routes: Record<string, unknown | Error>, calls: string[]) {
  return (async (input: string | URL | Request) => {
    const path = String(input);
    calls.push(path);
    const body = routes[path];
    if (body instanceof Error) throw body;
    if (body === undefined) return new Response('{"message":"Unauthenticated."}', { status: 401 });
    return new Response(JSON.stringify(body), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });
  }) as typeof globalThis.fetch;
}

const sidebar = { bookmarks: null, advertisements: [], gear_items: [] };

describe('the shared page props', () => {
  it('loads only the nav and sidebar for a guest', async () => {
    const calls: string[] = [];
    const props = await loadSharedProps(
      fetchStub({ '/api/nav-state': null, '/api/sidebar': sidebar }, calls),
    );
    expect(props).toEqual({ nav: null, sidebar, workbench: null, sentOffers: null });
    expect(calls.sort()).toEqual(['/api/nav-state', '/api/sidebar']);
  });

  it('adds the workbench and sent offers for a signed-in user', async () => {
    const calls: string[] = [];
    const nav = { user: { name: 'Wolfgang' }, characters: [] };
    const props = await loadSharedProps(
      fetchStub(
        {
          '/api/nav-state': nav,
          '/api/sidebar': sidebar,
          '/api/workbench': [{ id: 1, module: { id: 7 } }],
          '/api/offers/sent': [{ id: 3, module_id: 7 }],
        },
        calls,
      ),
    );
    expect(props.nav).toEqual(nav);
    expect(props.workbench).toEqual([{ id: 1, module: { id: 7 } }]);
    expect(props.sentOffers).toEqual([{ id: 3, module_id: 7 }]);
    expect(calls.sort()).toEqual([
      '/api/nav-state',
      '/api/offers/sent',
      '/api/sidebar',
      '/api/workbench',
    ]);
  });

  it('renders as a guest with an empty sidebar when the API is down', async () => {
    const down = new Error('connect ECONNREFUSED');
    const props = await loadSharedProps(
      fetchStub({ '/api/nav-state': down, '/api/sidebar': down }, []),
    );
    expect(props).toEqual({ nav: null, sidebar: null, workbench: null, sentOffers: null });
  });
});
