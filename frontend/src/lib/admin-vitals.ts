// Shaping of the recorded metric history into the vital charts' points,
// and the live readouts derived from consecutive /system samples. Pure
// functions; the page only holds the state they run over.
import type { MetricsHistory, SystemStats } from '$lib/admin-types';
import type { VitalPoint, VitalSeries } from '$lib/components/vital-chart.svelte';
import { t } from '$lib/i18n.svelte';

/** The timeframe toggle of the vitals charts. */
export const HISTORY_WINDOWS = ['24h', '3d', '7d'] as const;
export type HistoryWindow = (typeof HISTORY_WINDOWS)[number];

const ACCENT = '#a3e635';
const PARTNER = '#22d3ee';

// Series are built per call so their labels follow the current locale.
/** The machine and, beside it, our own share of it. Both charts carry
 * the pair: the container's readings alone understate the box by about
 * three times, because Postgres, the renderer and the proxy sit next to
 * this process. */
export function hostAndServiceSeries(): VitalSeries[] {
  return [
    { key: 'host', label: t('admin.vitals.series.server'), color: ACCENT },
    { key: 'api', label: t('admin.vitals.series.api'), color: PARTNER },
  ];
}
export function usedSeries(): VitalSeries[] {
  return [{ key: 'value', label: t('admin.vitals.series.used'), color: ACCENT }];
}
export function sizeSeries(): VitalSeries[] {
  return [{ key: 'value', label: t('admin.vitals.series.size'), color: ACCENT }];
}
export function inboundSeries(): VitalSeries[] {
  return [{ key: 'value', label: t('admin.vitals.series.in'), color: ACCENT }];
}
export function outboundSeries(): VitalSeries[] {
  return [{ key: 'value', label: t('admin.vitals.series.out'), color: PARTNER }];
}

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

/** Busy-second deltas as percent of the machine (all cores), for the
 * host and for this service. Samples recorded before the host series
 * existed carry the service line alone. */
export function cpuPoints(history: MetricsHistory | null, cores: number | null): VitalPoint[] {
  const share = (value: number | undefined) => ((value ?? 0) * 100) / (cores ?? 1);
  return ratePoints(history, { host: 'host_cpu_seconds', api: 'cpu_seconds' }).map((point) => ({
    at: point.at,
    values: { host: share(point.values.host), api: share(point.values.api) },
  }));
}

/** Memory as percent of capacity, for the machine and for this service. */
export function memoryPoints(
  history: MetricsHistory | null,
  capacity: number | null,
): VitalPoint[] {
  const host = percentPoints(history, 'host_memory_bytes', capacity);
  const api = new Map(
    percentPoints(history, 'memory_bytes', capacity).map((point) => [point.at, point.values.value]),
  );
  if (host.length === 0) {
    // Before the first host sample, the service line is all there is.
    return [...api.entries()].map(([at, value]) => ({ at, values: { api: value ?? 0 } }));
  }
  return host.map((point) => ({
    at: point.at,
    values: { host: point.values.value ?? 0, api: api.get(point.at) ?? 0 },
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

/** The machine's cpu load between two samples, in percent of all its
 * cores: the number `btop` shows, against `cpuPercent`'s one-process
 * view. */
export function hostCpuPercent(
  previous: SystemSample | null,
  current: SystemSample,
  cores: number | null,
): number | null {
  if (previous === null) return null;
  const { host_cpu_seconds: before } = previous.stats;
  const { host_cpu_seconds: after } = current.stats;
  if (before === null || after === null) return null;
  const wall = current.at - previous.at;
  if (wall <= 0) return null;
  return Math.max((((after - before) / wall) * 100) / (cores ?? 1), 0);
}

/** Whether the machine's own counters reached us, or only this
 * container's: the host needs its sysfs bind-mounted for the former. */
export function hasHostNetwork(system: SystemStats | null): boolean {
  return system?.host_network_rx_bytes != null;
}

/**
 * Bytes per second between two samples, the machine's uplinks when they
 * are readable and this container's veth otherwise. A container sees its
 * traffic with Postgres and ESI, not the traffic the site serves, so the
 * two differ by an order of magnitude on a real box.
 */
export function networkRates(
  previous: SystemSample | null,
  current: SystemSample,
): { rx: number; tx: number } | null {
  if (previous === null) return null;
  const wall = current.at - previous.at;
  if (wall <= 0) return null;
  const host = hasHostNetwork(current.stats) && hasHostNetwork(previous.stats);
  const before = host
    ? [previous.stats.host_network_rx_bytes, previous.stats.host_network_tx_bytes]
    : [previous.stats.network_rx_bytes, previous.stats.network_tx_bytes];
  const after = host
    ? [current.stats.host_network_rx_bytes, current.stats.host_network_tx_bytes]
    : [current.stats.network_rx_bytes, current.stats.network_tx_bytes];
  if (before.some((value) => value === null) || after.some((value) => value === null)) {
    return null;
  }
  return {
    rx: Math.max(((after[0] as number) - (before[0] as number)) / wall, 0),
    tx: Math.max(((after[1] as number) - (before[1] as number)) / wall, 0),
  };
}

/**
 * One direction of the recorded traffic as per-bucket rates, preferring
 * the machine's series over this container's.
 */
export function networkPoints(
  history: MetricsHistory | null,
  direction: 'rx' | 'tx',
): VitalPoint[] {
  const host = ratePoints(history, { value: `host_network_${direction}_bytes` });
  return host.length > 0 ? host : ratePoints(history, { value: `network_${direction}_bytes` });
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
