import { describe, expect, it } from 'vitest';

import { loadBrowser } from './browser';

const alerts = [{ id: 7, query: 'type/47408', type: null }];

function eventStub(calls: string[], session: string | undefined) {
  const routes: Record<string, unknown> = {
    '/api/module-cards': [{ id: 1 }],
    '/api/module-cards?unlisted=true': [{ id: 1 }],
    '/api/module-stats?unlisted=false': { total: 1 },
    '/api/module-stats?unlisted=true': { total: 1 },
    '/api/search-alerts': alerts,
  };
  const fetch = (async (input: string | URL | Request) => {
    const path = String(input);
    calls.push(path);
    const body = routes[path];
    if (body === undefined) return new Response('{"message":"Unauthenticated."}', { status: 401 });
    return new Response(JSON.stringify(body), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });
  }) as typeof globalThis.fetch;

  return {
    fetch,
    cookies: { get: (name: string) => (name === 'mm_session' ? session : undefined) },
  };
}

describe('the module browser load', () => {
  it('never asks for the search alerts without a session', async () => {
    const calls: string[] = [];
    const data = await loadBrowser(
      eventStub(calls, undefined) as Parameters<typeof loadBrowser>[0],
      '',
      false,
    );

    expect(data.alerts).toBeNull();
    expect(calls).not.toContain('/api/search-alerts');
    expect(calls.sort()).toEqual(['/api/module-cards', '/api/module-stats?unlisted=false']);
  });

  it('loads the alerts for a signed-in visitor', async () => {
    const calls: string[] = [];
    const data = await loadBrowser(
      eventStub(calls, 'a-session-token') as Parameters<typeof loadBrowser>[0],
      '',
      false,
    );

    expect(data.alerts).toEqual(alerts);
    expect(calls).toContain('/api/search-alerts');
  });

  it('leaves the bell alone on the archive, session or not', async () => {
    const calls: string[] = [];
    const data = await loadBrowser(
      eventStub(calls, 'a-session-token') as Parameters<typeof loadBrowser>[0],
      '',
      true,
    );

    expect(data.alerts).toBeNull();
    expect(calls).not.toContain('/api/search-alerts');
  });
});
