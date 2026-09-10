// Shaping of captured request failures for the console's errors page.
// Pure functions, so the page stays thin and the status split matches
// the API's `class` filter exactly.
import { parseDbTimestamp } from '$lib/duration';
import { t } from '$lib/i18n.svelte';
import type { RequestFailureSummary } from '$lib/admin-types';

/** Bytes of a body the API keeps; mirrors BODY_CAPTURE_BYTES in
 * src/activity/failures.rs, which is the authority. */
export const BODY_CAPTURE_BYTES = 8 * 1024;

/** The API's two `class` values: a refused request versus a broken one. */
export type StatusClass = 'client' | 'server';

export function statusClass(status: number): StatusClass {
  return status >= 500 ? 'server' : 'client';
}

/** Unix seconds the failure happened, from the API's timestamp text. */
export function failureAt(failure: RequestFailureSummary): number {
  return parseDbTimestamp(failure.occurred_at);
}

/** Who made the request: the account's name, or that nobody was signed
 * in. An account deleted since the capture reads as a guest too, because
 * the row's user reference goes with it. */
export function accountLabel(failure: RequestFailureSummary): string {
  return failure.user_name ?? t('admin.requestFailures.guest');
}

function bytes(value: number): string {
  if (value >= 1024 ** 2) return `${(value / 1024 ** 2).toFixed(1)} MB`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${value} B`;
}

/** What the page is not showing, when the body was capped. */
export function truncationNote(stored: string | null, full: number | null): string | null {
  if (stored === null || full === null || full <= stored.length) return null;
  return t('admin.requestFailures.truncationNote', {
    shown: bytes(stored.length),
    full: bytes(full),
  });
}

/** The query string behind the list, so the page and its tests agree on
 * what each filter asks for. */
export function failuresQuery(filter: {
  route?: string | null;
  class?: StatusClass | null;
}): string {
  const params = new URLSearchParams();
  if (filter.route) params.set('route', filter.route);
  if (filter.class) params.set('class', filter.class);
  const query = params.toString();
  return query === '' ? '/api/admin/request-failures' : `/api/admin/request-failures?${query}`;
}
