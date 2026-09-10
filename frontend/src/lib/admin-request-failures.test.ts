import { describe, expect, it } from 'vitest';

import {
  accountLabel,
  failureAt,
  failuresQuery,
  statusClass,
  truncationNote,
} from './admin-request-failures';
import type { RequestFailureSummary } from './admin-types';
import { t } from './i18n.svelte';

function failure(overrides: Partial<RequestFailureSummary> = {}): RequestFailureSummary {
  return {
    id: 1,
    occurred_at: '2026-09-10 14:04:10.757061+00',
    route: 'GET /api/search-alerts',
    method: 'GET',
    path: '/api/search-alerts',
    status: 401,
    error_message: 'Unauthenticated.',
    duration_ms: 3,
    user_id: null,
    user_name: null,
    ...overrides,
  };
}

describe('statusClass', () => {
  it('splits the way the API filters, at 500', () => {
    expect(statusClass(401)).toBe('client');
    expect(statusClass(404)).toBe('client');
    expect(statusClass(422)).toBe('client');
    expect(statusClass(500)).toBe('server');
    expect(statusClass(503)).toBe('server');
  });
});

describe('accountLabel', () => {
  it('names the account, or says nobody was signed in', () => {
    expect(accountLabel(failure({ user_id: 7, user_name: 'Wolfgang' }))).toBe('Wolfgang');
    expect(accountLabel(failure())).toBe(t('admin.requestFailures.guest'));
  });
});

describe('failureAt', () => {
  it('reads the API timestamp as unix seconds', () => {
    expect(failureAt(failure())).toBe(Date.parse('2026-09-10T14:04:10.757Z') / 1000);
  });
});

describe('truncationNote', () => {
  it('only speaks up when the body was capped', () => {
    expect(truncationNote('{"message":"nope"}', 18)).toBeNull();
    expect(truncationNote(null, null)).toBeNull();
    expect(truncationNote('x'.repeat(1024), 4096)).toBe(
      t('admin.requestFailures.truncationNote', { shown: '1.0 KB', full: '4.0 KB' }),
    );
  });
});

describe('failuresQuery', () => {
  it('asks for everything when nothing is filtered', () => {
    expect(failuresQuery({})).toBe('/api/admin/request-failures');
    expect(failuresQuery({ route: null, class: null })).toBe('/api/admin/request-failures');
  });

  it('carries each filter the API knows', () => {
    expect(failuresQuery({ class: 'server' })).toBe('/api/admin/request-failures?class=server');
    expect(failuresQuery({ route: 'GET /api/search-alerts', class: 'client' })).toBe(
      '/api/admin/request-failures?route=GET+%2Fapi%2Fsearch-alerts&class=client',
    );
  });
});
