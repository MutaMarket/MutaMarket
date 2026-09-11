import { describe, expect, it } from 'vitest';

import { load } from './+page.server';

function fetchStub(calls: string[]) {
  return (async (input: string | URL | Request) => {
    calls.push(String(input));
    return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } });
  }) as typeof globalThis.fetch;
}

describe('the console overview load', () => {
  it('never waits for the database counts', async () => {
    const calls: string[] = [];
    // A client-side navigation waits for this load before it swaps the
    // page, and the counts are second-long count scans; they ride the
    // poll instead.
    await load({ fetch: fetchStub(calls) } as never);

    expect(calls.sort()).toEqual([
      '/api/admin/live?sections=system,jobs',
      '/api/admin/service-character',
    ]);
  });
});
