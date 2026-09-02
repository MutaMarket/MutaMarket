// Shaping of captured ESI failures for the console. Pure functions, so
// the panel stays thin and the classification matches the error chart's
// series exactly.
import { parseDbTimestamp } from '$lib/duration';
import type { EsiFailureSummary } from '$lib/admin-types';

/** Bytes of a body the API keeps; mirrors BODY_CAPTURE_BYTES in
 * src/esi/failures.rs, which is the authority. */
export const BODY_CAPTURE_BYTES = 8 * 1024;

/** The chart's series keys, so a row's dot matches its column. */
export type FailureClass = 'client_errors' | 'server_errors' | 'transport_errors';

export function failureClass(failure: EsiFailureSummary): FailureClass {
  if (failure.status === null) return 'transport_errors';
  if (failure.status >= 500) return 'server_errors';
  return 'client_errors';
}

/** The status as the row shows it: a code, or why nothing came back. */
export function failureLabel(failure: EsiFailureSummary): string {
  if (failure.status !== null) return String(failure.status);
  return failure.error_kind ? `no response · ${failure.error_kind}` : 'no response';
}

/** The job or route that raised it, without the machine prefix. */
export function callerLabel(failure: EsiFailureSummary): string | null {
  if (failure.caller === null) return null;
  const [kind, ...rest] = failure.caller.split(':');
  const label = rest.join(':');
  return kind === 'job' ? `job ${label}` : label || failure.caller;
}

/** A job failure names the run it belongs to, which the jobs board can
 * open; a handler failure has none. */
export function jobName(failure: EsiFailureSummary): string | null {
  return failure.caller?.startsWith('job:') ? failure.caller.slice('job:'.length) : null;
}

/** Unix seconds the failure happened, from the API's timestamp text. */
export function failureAt(failure: EsiFailureSummary): number {
  return parseDbTimestamp(failure.occurred_at);
}

/** Pretty-printed when the body is JSON, raw otherwise. */
export function formatBody(body: string | null): string | null {
  if (body === null || body.trim() === '') return null;
  try {
    return JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    return body;
  }
}

function bytes(value: number): string {
  if (value >= 1024 ** 2) return `${(value / 1024 ** 2).toFixed(1)} MB`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${value} B`;
}

/** What the page is not showing, when the body was capped. */
export function truncationNote(stored: string | null, full: number | null): string | null {
  if (stored === null || full === null || full <= stored.length) return null;
  return `showing the first ${bytes(stored.length)} of ${bytes(full)}`;
}

/** Client-side narrowing of the live set, before falling back to a
 * fetch for a minute the live set does not reach. */
export function filterFailures(
  failures: EsiFailureSummary[],
  filter: { minute?: number | null; endpoint?: string | null; class?: FailureClass | null },
): EsiFailureSummary[] {
  return failures.filter((failure) => {
    if (filter.endpoint && failure.endpoint !== filter.endpoint) return false;
    if (filter.class && failureClass(failure) !== filter.class) return false;
    if (filter.minute != null) {
      const at = parseDbTimestamp(failure.occurred_at);
      if (at < filter.minute || at >= filter.minute + 60) return false;
    }
    return true;
  });
}
