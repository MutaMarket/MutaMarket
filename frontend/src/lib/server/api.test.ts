import { describe, expect, it } from 'vitest';
import { isHttpError, isRedirect } from '@sveltejs/kit';

import { apiGet } from './api';

function fetchStub(status: number, body: unknown): typeof globalThis.fetch {
  return async () =>
    new Response(JSON.stringify(body), {
      status,
      headers: { 'content-type': 'application/json' },
    });
}

describe('apiGet', () => {
  it('returns the decoded payload on success', async () => {
    const payload = await apiGet<{ ok: boolean }>(fetchStub(200, { ok: true }), '/api/x');
    expect(payload).toEqual({ ok: true });
  });

  it('sends guests to the login page on 401', async () => {
    const outcome: unknown = await apiGet(
      fetchStub(401, { message: 'Unauthenticated.' }),
      '/api/x',
    ).catch((thrown) => thrown);
    if (!isRedirect(outcome)) throw new Error('expected a redirect');
    expect(outcome.status).toBe(303);
    expect(outcome.location).toBe('/login');
  });

  it('turns API failures into error pages carrying the message', async () => {
    for (const [status, message] of [
      [404, 'Collection not found'],
      [403, 'This collection is private.'],
      [503, 'The documentation is temporarily unavailable.'],
    ] as const) {
      const outcome: unknown = await apiGet(fetchStub(status, { message }), '/api/x').catch(
        (thrown) => thrown,
      );
      if (!isHttpError(outcome)) throw new Error('expected an http error');
      expect(outcome.status).toBe(status);
      expect(outcome.body.message).toBe(message);
    }
  });

  it('falls back to a generic message on undecodable failures', async () => {
    const broken: typeof globalThis.fetch = async () => new Response('boom', { status: 500 });
    const outcome: unknown = await apiGet(broken, '/api/x').catch((thrown) => thrown);
    if (!isHttpError(outcome)) throw new Error('expected an http error');
    expect(outcome.body.message).toBe('The server is unavailable right now.');
  });
});
