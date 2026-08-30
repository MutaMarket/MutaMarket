import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { apply, live, refresh, reset, subscribe, type LivePayload } from './admin-live.svelte';
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
		...overrides
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
		last_runs: []
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
				json: () => Promise.resolve(payload)
			} as Response);
		})
	);
}

beforeEach(() => {
	reset();
	requested = [];
});

afterEach(() => {
	reset();
	vi.unstubAllGlobals();
});

describe('apply', () => {
	it('folds only the sections the payload carries', () => {
		apply({
			header: { enabled: true, in_downtime: false, uptime_seconds: 10 }
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
		await refresh();
		expect(requested.at(-1)).toBe('/api/admin/live?sections=header');

		const stopPage = subscribe(['system', 'jobs']);
		await refresh();
		expect(requested.at(-1)).toBe('/api/admin/live?sections=header%2Csystem%2Cjobs');

		// The page unmounts; the layout's header subscription remains.
		stopPage();
		await refresh();
		expect(requested.at(-1)).toBe('/api/admin/live?sections=header');

		stop();
	});

	it('refcounts a section two pages both draw', async () => {
		respond({});
		const first = subscribe(['jobs']);
		const second = subscribe(['jobs']);
		first();

		await refresh();
		expect(requested.at(-1)).toBe('/api/admin/live?sections=jobs');

		second();
		await refresh();
		expect(requested).toHaveLength(1);
	});

	it('does not poll with nothing mounted', async () => {
		respond({});
		await refresh();
		expect(requested).toEqual([]);
	});
});

describe('refresh', () => {
	it('sends the held jobs revision back so an unchanged section is skipped', async () => {
		respond({ jobs: [job('alliances')], jobs_revision: 'rev-1' });
		const stop = subscribe(['jobs']);

		// The first poll has nothing to send.
		await refresh();
		expect(requested.at(-1)).toBe('/api/admin/live?sections=jobs');

		await refresh();
		expect(requested.at(-1)).toBe('/api/admin/live?sections=jobs&jobs_revision=rev-1');

		stop();
	});

	it('keeps the last state when the API is unreachable', async () => {
		apply({
			header: { enabled: true, in_downtime: false, uptime_seconds: 10 }
		});
		vi.stubGlobal(
			'fetch',
			vi.fn(() => Promise.reject(new Error('offline')))
		);
		const stop = subscribe(['header']);

		await expect(refresh()).resolves.toBeUndefined();
		expect(live.header?.enabled).toBe(true);

		stop();
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
	market_history_days: 0
};
