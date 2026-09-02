import { describe, expect, it } from 'vitest';

import {
  CHART_WINDOW_MINUTES,
  ENDPOINT_COLORS,
  ERROR_SERIES,
  OTHER_KEY,
  assignSlots,
  chartMinutes,
  endpointTotals,
  hourTotals,
  requestSeries,
} from './admin-telemetry';
import type { TelemetryBucket, TelemetryCounts, TelemetrySnapshot } from './admin-types';

function counts(overrides: Partial<TelemetryCounts> = {}): TelemetryCounts {
  return {
    requests: 0,
    success: 0,
    client_errors: 0,
    server_errors: 0,
    transport_errors: 0,
    total_ms: 0,
    ...overrides,
  };
}

function bucket(minute_start: number, endpoints: Record<string, TelemetryCounts>): TelemetryBucket {
  return { minute_start, endpoints };
}

function snapshot(buckets: TelemetryBucket[]): TelemetrySnapshot {
  return { window_minutes: CHART_WINDOW_MINUTES, buckets };
}

describe('endpointTotals', () => {
  it('sums requests per endpoint across the window', () => {
    const totals = endpointTotals([
      bucket(60, { '/markets': counts({ requests: 3 }), '/contracts': counts({ requests: 1 }) }),
      bucket(120, { '/markets': counts({ requests: 4 }) }),
    ]);
    expect([...totals.entries()].sort()).toEqual([
      ['/contracts', 1],
      ['/markets', 7],
    ]);
  });

  it('is empty for an empty window', () => {
    expect(endpointTotals([]).size).toBe(0);
  });
});

describe('assignSlots', () => {
  it('fills free slots with the busiest endpoints first', () => {
    const totals = new Map([
      ['/a', 1],
      ['/b', 9],
      ['/c', 5],
    ]);
    expect(assignSlots([], totals)).toEqual(['/b', '/c', '/a']);
  });

  it('keeps held slots in place when volumes reshuffle', () => {
    const totals = new Map([
      ['/a', 100],
      ['/b', 1],
    ]);
    // '/b' led when the slots were handed out; it keeps its color even
    // though '/a' now dwarfs it, so the chart never repaints a series.
    expect(assignSlots(['/b', '/a'], totals)).toEqual(['/b', '/a']);
  });

  it('frees the slot of an endpoint that left the window', () => {
    const totals = new Map([
      ['/a', 5],
      ['/c', 2],
    ]);
    expect(assignSlots(['/gone', '/a'], totals)).toEqual(['/a', '/c']);
  });

  it('never hands out more slots than there are colors', () => {
    const totals = new Map(
      Array.from({ length: ENDPOINT_COLORS.length + 3 }, (_, index) => [`/e${index}`, index]),
    );
    expect(assignSlots([], totals)).toHaveLength(ENDPOINT_COLORS.length);
  });
});

describe('requestSeries', () => {
  it('colors the slots in order and appends the folded tail', () => {
    const series = requestSeries(['/a', '/b'], true);
    expect(series.map((s) => s.key)).toEqual(['/a', '/b', OTHER_KEY]);
    expect(series.map((s) => s.color)).toEqual([ENDPOINT_COLORS[0], ENDPOINT_COLORS[1], '#898781']);
  });

  it('omits the tail when every endpoint holds a slot', () => {
    expect(requestSeries(['/a'], false).map((s) => s.key)).toEqual(['/a']);
  });
});

describe('chartMinutes', () => {
  const minuteNow = 3_600_000;

  it('returns the full window ending at the current minute', () => {
    const { requests, errors } = chartMinutes(snapshot([]), [], minuteNow);
    expect(requests).toHaveLength(CHART_WINDOW_MINUTES);
    expect(errors).toHaveLength(CHART_WINDOW_MINUTES);
    expect(requests[0].minuteStart).toBe(minuteNow - (CHART_WINDOW_MINUTES - 1) * 60);
    expect(requests.at(-1)?.minuteStart).toBe(minuteNow);
  });

  it('fills minutes with no traffic with empty columns', () => {
    const { requests } = chartMinutes(
      snapshot([bucket(minuteNow, { '/markets': counts({ requests: 2, total_ms: 100 }) })]),
      ['/markets'],
      minuteNow,
    );
    expect(requests[0].values).toEqual({});
    expect(requests[0].detail).toBeUndefined();
    expect(requests.at(-1)?.values).toEqual({ '/markets': 2 });
  });

  it('folds endpoints without a slot into the other key', () => {
    const { requests } = chartMinutes(
      snapshot([
        bucket(minuteNow, {
          '/markets': counts({ requests: 2 }),
          '/contracts': counts({ requests: 3 }),
          '/assets': counts({ requests: 4 }),
        }),
      ]),
      ['/markets'],
      minuteNow,
    );
    expect(requests.at(-1)?.values).toEqual({ '/markets': 2, [OTHER_KEY]: 7 });
  });

  it('reports the minute average latency over every endpoint', () => {
    const { requests } = chartMinutes(
      snapshot([
        bucket(minuteNow, {
          '/markets': counts({ requests: 2, total_ms: 100 }),
          '/contracts': counts({ requests: 2, total_ms: 300 }),
        }),
      ]),
      [],
      minuteNow,
    );
    expect(requests.at(-1)?.detail).toBe('avg 100 ms');
  });

  it('stacks error classes regardless of the slot assignment', () => {
    const { errors } = chartMinutes(
      snapshot([
        bucket(minuteNow, {
          '/markets': counts({ requests: 3, client_errors: 1, server_errors: 2 }),
          '/assets': counts({ requests: 1, transport_errors: 4 }),
        }),
      ]),
      ['/markets'],
      minuteNow,
    );
    expect(errors.at(-1)?.values).toEqual({
      client_errors: 1,
      server_errors: 2,
      transport_errors: 4,
    });
  });

  it('drops buckets that fell out of the window', () => {
    const stale = minuteNow - CHART_WINDOW_MINUTES * 60;
    const { requests } = chartMinutes(
      snapshot([bucket(stale, { '/markets': counts({ requests: 9 }) })]),
      ['/markets'],
      minuteNow,
    );
    expect(requests.every((minute) => Object.keys(minute.values).length === 0)).toBe(true);
  });

  it('names every error class the chart stacks', () => {
    expect(ERROR_SERIES.map((s) => s.key)).toEqual([
      'client_errors',
      'server_errors',
      'transport_errors',
    ]);
  });
});

describe('hourTotals', () => {
  it('sums requests, errors and the weighted average latency', () => {
    const buckets = [
      bucket(60, {
        '/markets': counts({ requests: 3, total_ms: 300, client_errors: 1 }),
        '/assets': counts({ requests: 1, total_ms: 500, server_errors: 1 }),
      }),
      bucket(120, { '/markets': counts({ requests: 4, total_ms: 400, transport_errors: 2 }) }),
    ];
    const totals = hourTotals(buckets, endpointTotals(buckets));
    expect(totals.requests).toBe(8);
    expect(totals.errors).toBe(4);
    expect(totals.averageMs).toBe(150);
    expect(totals.busiest).toEqual(['/markets', 7]);
  });

  it('reports a quiet window without dividing by zero', () => {
    const totals = hourTotals([], new Map());
    expect(totals).toEqual({ requests: 0, errors: 0, averageMs: 0, busiest: null });
  });
});
