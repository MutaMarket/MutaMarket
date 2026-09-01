// Shaping of the recorded metric history into the vital charts' points,
// and the live readouts derived from consecutive /system samples. Pure
// functions; the page only holds the state they run over.
import type { MetricsHistory, SystemStats } from '$lib/admin-types';
import type { VitalPoint, VitalSeries } from '$lib/components/vital-chart.svelte';

/** The timeframe toggle of the vitals charts. */
export const HISTORY_WINDOWS = ['24h', '3d', '7d'] as const;
export type HistoryWindow = (typeof HISTORY_WINDOWS)[number];

const ACCENT = '#a3e635';
const PARTNER = '#22d3ee';

export const LOAD_SERIES: VitalSeries[] = [{ key: 'value', label: 'load', color: ACCENT }];
export const USED_SERIES: VitalSeries[] = [{ key: 'value', label: 'used', color: ACCENT }];
export const SIZE_SERIES: VitalSeries[] = [{ key: 'value', label: 'size', color: ACCENT }];
export const NETWORK_SERIES: VitalSeries[] = [
	{ key: 'rx', label: 'in', color: ACCENT },
	{ key: 'tx', label: 'out', color: PARTNER },
];

/**
 * The capacities the charts divide by. Split out of the polled stats
 * because they never move: keeping them in the five-second `system`
 * object made every history-derived series rebuild on every poll.
 */
export interface Capacity {
	cpuCores: number | null;
	memoryBytes: number | null;
	diskBytes: number | null;
}

export function capacityOf(system: SystemStats): Capacity {
	return {
		cpuCores: system.cpu_cores,
		// The cgroup limit, else the machine's total memory.
		memoryBytes: system.memory_limit_bytes ?? system.memory_total_bytes,
		diskBytes: system.disk_total_bytes,
	};
}

export function sameCapacity(a: Capacity, b: Capacity): boolean {
	return (
		a.cpuCores === b.cpuCores && a.memoryBytes === b.memoryBytes && a.diskBytes === b.diskBytes
	);
}

/** A gauge series as chart points. */
export function gaugePoints(history: MetricsHistory | null, metric: string): VitalPoint[] {
	return (history?.series[metric] ?? []).map((sample) => ({
		at: sample.taken_at,
		values: { value: sample.value },
	}));
}

/**
 * Counter series as per-bucket rates, clamped at zero — which also
 * absorbs a restart resetting the totals.
 */
export function ratePoints(
	history: MetricsHistory | null,
	metrics: Record<string, string>,
): VitalPoint[] {
	if (history === null) return [];
	const step = history.step_seconds;
	const byAt = new Map<number, VitalPoint>();
	for (const [key, metric] of Object.entries(metrics)) {
		const series = history.series[metric] ?? [];
		for (const [index, sample] of series.slice(1).entries()) {
			const value = Math.max((sample.value - series[index].value) / step, 0);
			const point = byAt.get(sample.taken_at) ?? { at: sample.taken_at, values: {} };
			point.values[key] = value;
			byAt.set(sample.taken_at, point);
		}
	}
	return [...byAt.values()].sort((a, b) => a.at - b.at);
}

/** A gauge series as utilization percent of a fixed capacity. */
export function percentPoints(
	history: MetricsHistory | null,
	metric: string,
	capacity: number | null,
): VitalPoint[] {
	if (capacity === null || capacity <= 0) return [];
	return gaugePoints(history, metric).map((point) => ({
		at: point.at,
		values: { value: ((point.values.value ?? 0) * 100) / capacity },
	}));
}

/** cpu_seconds deltas as percent of the machine (all cores). */
export function cpuPoints(history: MetricsHistory | null, cores: number | null): VitalPoint[] {
	return ratePoints(history, { value: 'cpu_seconds' }).map((point) => ({
		at: point.at,
		values: { value: ((point.values.value ?? 0) * 100) / (cores ?? 1) },
	}));
}

/** One /system sample with the wall-clock moment it was taken. */
export interface SystemSample {
	at: number;
	stats: SystemStats;
}

/** CPU load between two samples, in percent of one core. */
export function cpuPercent(previous: SystemSample | null, current: SystemSample): number | null {
	if (previous === null) return null;
	if (current.stats.cpu_seconds === null || previous.stats.cpu_seconds === null) return null;
	const wall = current.at - previous.at;
	if (wall <= 0) return null;
	return Math.max(((current.stats.cpu_seconds - previous.stats.cpu_seconds) / wall) * 100, 0);
}

/** Bytes per second between two samples. */
export function networkRates(
	previous: SystemSample | null,
	current: SystemSample,
): { rx: number; tx: number } | null {
	if (previous === null) return null;
	const { stats } = previous;
	if (
		current.stats.network_rx_bytes === null ||
		stats.network_rx_bytes === null ||
		current.stats.network_tx_bytes === null ||
		stats.network_tx_bytes === null
	) {
		return null;
	}
	const wall = current.at - previous.at;
	if (wall <= 0) return null;
	return {
		rx: Math.max((current.stats.network_rx_bytes - stats.network_rx_bytes) / wall, 0),
		tx: Math.max((current.stats.network_tx_bytes - stats.network_tx_bytes) / wall, 0),
	};
}

export function percentOf(value: number | null, capacity: number | null): number | null {
	if (value === null || capacity === null || capacity <= 0) return null;
	return (value * 100) / capacity;
}

export function formatBytes(value: number | null): string {
	if (value === null) return '—';
	if (value >= 1024 ** 3) return `${(value / 1024 ** 3).toFixed(1)} GB`;
	if (value >= 1024 ** 2) return `${(value / 1024 ** 2).toFixed(1)} MB`;
	if (value >= 1024) return `${(value / 1024).toFixed(1)} KB`;
	return `${value} B`;
}

export function formatUptime(seconds: number | null): string {
	if (seconds === null) return '—';
	if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
	if (seconds < 86_400) {
		return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
	}
	return `${Math.floor(seconds / 86_400)}d ${Math.floor((seconds % 86_400) / 3600)}h`;
}

export function compact(value: number): string {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
	if (value >= 10_000) return `${(value / 1_000).toFixed(1)}K`;
	return value.toLocaleString('en-US');
}
