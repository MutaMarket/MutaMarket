import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { apply, live, refresh, reset, subscribe, type LivePayload } from './admin-live.svelte';
import { cpuPercent, networkRates } from './admin-vitals';
import type { SchedulerJob, SystemStats } from './admin-types';

function system(overrides: Partial<SystemStats> = {}): SystemStats {
  return {
    disk_used_bytes: 10,
    disk_total_bytes: 100,
    memory_total_bytes: 800,
    memory_rss_bytes: 40,
    memory_current_bytes: 50,
    memory_limit_bytes: 400,
    cpu_seconds: 12,
    cpu_cores: 8,
    network_rx_bytes: 1000,
    network_tx_bytes: 500,
    uptime_seconds: 60,
    database_size_bytes: 900,
    ...overrides,
  };
}

function job(name: string): SchedulerJob {
  return {
    name,
    interval_seconds: 60,
    downtime_guarded: false,
    paused: false,
    running: false,
    next_run_at: 100,
    progress: null,
    last_runs: [],
  };
}

/** The URLs the store asked for, in order. */
let requested: string[] = [];

function respond(payload: LivePayload) {
  vi.stubGlobal(
    'fetch',
    vi.fn((url: string) => {
      requested.push(url);
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve(payload),
      } as Response);
    }),
  );
}

beforeEach(() => {
  reset();
  requested = [];
  vi.useFakeTimers({ toFake: ['Date'] });
});

afterEach(() => {
  reset();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('apply', () => {
  it('folds only the sections the payload carries', () => {
    apply({
      header: { enabled: true, in_downtime: false, uptime_seconds: 10 },
    });
    expect(live.header?.enabled).toBe(true);
    expect(live.system).toBeNull();
    expect(live.telemetry).toBeNull();

    apply({ database: { ...ZERO_COUNTS, modules: 7 } });
    expect(live.database?.modules).toBe(7);
    // The header survives a payload that did not carry it.
    expect(live.header?.enabled).toBe(true);
  });

  it('keeps the previous system sample so the rates have two points', () => {
    apply({ system: system({ cpu_seconds: 10 }) });
    expect(live.previousSample).toBeNull();

    apply({ system: system({ cpu_seconds: 20 }) });
    expect(live.previousSample?.stats.cpu_seconds).toBe(10);
    expect(live.currentSample?.stats.cpu_seconds).toBe(20);
  });

  it('holds the jobs it has when the revision matched', () => {
    const jobs = [job('alliances')];
    apply({ jobs, jobs_revision: 'r1' });
    expect(live.jobs).toBe(jobs);

    // A gated poll: same revision, no section. The identity must
    // survive, or every job card rebuilds for nothing.
    apply({ jobs: null, jobs_revision: 'r1' });
    expect(live.jobs).toBe(jobs);

    const next = [job('alliances'), job('og-cache')];
    apply({ jobs: next, jobs_revision: 'r2' });
    expect(live.jobs).toBe(next);
  });
});

describe('subscribe', () => {
  it('asks only for the sections its subscribers draw', async () => {
    respond({});
    const stop = subscribe(['header']);
    await refresh(true);
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header');

    const stopPage = subscribe(['system', 'jobs']);
    await refresh(true);
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header%2Csystem%2Cjobs');

    // The page unmounts; the layout's header subscription remains.
    stopPage();
    await refresh(true);
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header');

    stop();
  });

  it('refcounts a section two pages both draw', async () => {
    respond({});
    const first = subscribe(['jobs']);
    const second = subscribe(['jobs']);
    first();

    await refresh(true);
    expect(requested.at(-1)).toBe('/api/admin/live?sections=jobs');

    second();
    await refresh(true);
    expect(requested).toHaveLength(1);
  });

  it('does not poll with nothing mounted', async () => {
    respond({});
    await refresh(true);
    expect(requested).toEqual([]);
  });
});

describe('per-section cadence', () => {
  it('leaves the slow sections out of a tick that is not due for them', async () => {
    respond({});
    const stop = subscribe(['header', 'telemetry', 'database']);

    // The first tick is due for everything.
    await refresh();
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header%2Ctelemetry%2Cdatabase');

    // The next tick, five seconds later, is due only for the header:
    // redrawing 60 telemetry columns to show the same minute is what
    // made the page block for most of a second on every poll.
    vi.setSystemTime(Date.now() + 5_000);
    await refresh();
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header');

    // Thirty seconds in, telemetry comes due again; the counts do not.
    vi.setSystemTime(Date.now() + 26_000);
    await refresh();
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header%2Ctelemetry');

    stop();
  });

  it('puts activity on the slow cadence with telemetry', async () => {
    respond({});
    const stop = subscribe(['header', 'activity']);
    await refresh();
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header%2Cactivity');

    // A 60-column chart is not worth redrawing every five seconds.
    vi.setSystemTime(Date.now() + 5_000);
    await refresh();
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header');

    vi.setSystemTime(Date.now() + 26_000);
    await refresh();
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header%2Cactivity');

    stop();
  });

  it('forces every mounted section after an action changed the state', async () => {
    respond({});
    const stop = subscribe(['header', 'telemetry']);
    await refresh();

    vi.setSystemTime(Date.now() + 5_000);
    await refresh(true);
    expect(requested.at(-1)).toBe('/api/admin/live?sections=header%2Ctelemetry');

    stop();
  });

  it('does not reissue a section whose fetch failed', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.resolve({ ok: false, status: 503 } as Response)),
    );
    const stop = subscribe(['telemetry']);
    await refresh();
    // A failed poll records nothing, so the next tick retries it.
    vi.setSystemTime(Date.now() + 5_000);
    await expect(refresh()).resolves.toBeUndefined();

    stop();
  });
});

describe('refresh', () => {
  it('sends the held jobs revision back so an unchanged section is skipped', async () => {
    respond({ jobs: [job('alliances')], jobs_revision: 'rev-1' });
    const stop = subscribe(['jobs']);

    // The first poll has nothing to send.
    await refresh();
    expect(requested.at(-1)).toBe('/api/admin/live?sections=jobs');

    vi.setSystemTime(Date.now() + 5_000);
    await refresh();
    expect(requested.at(-1)).toBe('/api/admin/live?sections=jobs&jobs_revision=rev-1');

    stop();
  });

  it('keeps the last state when the API is unreachable', async () => {
    apply({
      header: { enabled: true, in_downtime: false, uptime_seconds: 10 },
    });
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('offline'))),
    );
    const stop = subscribe(['header']);

    await expect(refresh()).resolves.toBeUndefined();
    expect(live.header?.enabled).toBe(true);

    stop();
  });
});

describe('seeding from an effect', () => {
  it('does not re-run the effect that applied a payload', () => {
    // Pages seed the store from their loaded data inside an $effect.
    // If apply() tracked the state it writes, every poll would re-run
    // that effect and re-apply the page's original payload.
    let seeded = 0;
    const seed: LivePayload = { system: system({ cpu_seconds: 10 }) };
    const cleanup = $effect.root(() => {
      $effect(() => {
        seeded += 1;
        apply(seed);
      });
    });
    flushSync();
    expect(seeded).toBe(1);

    apply({ system: system({ cpu_seconds: 20 }) });
    flushSync();
    expect(seeded).toBe(1);

    cleanup();
  });

  it('keeps the two samples the rates need apart', () => {
    // The re-run above re-stamped the sample clock, collapsing both
    // samples onto one instant, so every rate divided by a zero
    // interval and the CPU and network cards showed a dash forever.
    vi.setSystemTime(1_000_000_000_000);
    const seed: LivePayload = { system: system({ cpu_seconds: 10, network_rx_bytes: 0 }) };
    const cleanup = $effect.root(() => {
      $effect(() => {
        apply(seed);
      });
    });
    flushSync();

    vi.setSystemTime(Date.now() + 5_000);
    apply({ system: system({ cpu_seconds: 25, network_rx_bytes: 5_000 }) });
    flushSync();

    const current = live.currentSample;
    expect(current).not.toBeNull();
    expect(live.previousSample).not.toBeNull();
    expect(current!.at - live.previousSample!.at).toBe(5);
    expect(cpuPercent(live.previousSample, current!)).toBe(300);
    expect(networkRates(live.previousSample, current!)).toEqual({ rx: 1000, tx: 0 });

    cleanup();
  });
});

describe('reactivity', () => {
  it('does not invalidate a capacity reader when a poll leaves it alone', () => {
    let runs = 0;
    const cleanup = $effect.root(() => {
      const cores = $derived(live.system?.cpu_cores ?? null);
      const points = $derived.by(() => {
        runs += 1;
        return cores;
      });
      $effect(() => {
        void points;
      });
    });

    apply({ system: system({ cpu_seconds: 10 }) });
    flushSync();
    const afterFirst = runs;

    // A poll that moves the live readings but not the core count:
    // the chart data derived from the capacity must not rebuild.
    apply({ system: system({ cpu_seconds: 99, memory_rss_bytes: 41 }) });
    flushSync();
    expect(runs).toBe(afterFirst);

    // A capacity that genuinely changed does rebuild.
    apply({ system: system({ cpu_cores: 4 }) });
    flushSync();
    expect(runs).toBe(afterFirst + 1);

    cleanup();
  });
});

const ZERO_COUNTS = {
  modules: 0,
  modules_without_estimate: 0,
  contracts: 0,
  contract_items: 0,
  characters: 0,
  users: 0,
  assets: 0,
  public_ownerships: 0,
  market_history_days: 0,
};
