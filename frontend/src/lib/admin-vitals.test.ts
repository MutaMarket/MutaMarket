import { describe, expect, it } from 'vitest';

import {
  capacityOf,
  compact,
  cpuPercent,
  cpuPoints,
  formatBytes,
  formatUptime,
  gaugePoints,
  networkRates,
  percentOf,
  percentPoints,
  ratePoints,
  sameCapacity,
} from './admin-vitals';
import type { MetricsHistory, SystemStats } from './admin-types';

function history(series: Record<string, [number, number][]>, step = 60): MetricsHistory {
  return {
    window: '24h',
    step_seconds: step,
    series: Object.fromEntries(
      Object.entries(series).map(([metric, samples]) => [
        metric,
        samples.map(([taken_at, value]) => ({ taken_at, value })),
      ]),
    ),
  };
}

function system(overrides: Partial<SystemStats> = {}): SystemStats {
  return {
    disk_used_bytes: null,
    disk_total_bytes: null,
    memory_total_bytes: null,
    memory_rss_bytes: null,
    memory_current_bytes: null,
    memory_limit_bytes: null,
    cpu_seconds: null,
    cpu_cores: null,
    network_rx_bytes: null,
    network_tx_bytes: null,
    uptime_seconds: null,
    database_size_bytes: null,
    ...overrides,
  };
}

describe('capacityOf', () => {
  it('prefers the cgroup limit over the machine total', () => {
    expect(
      capacityOf(system({ memory_limit_bytes: 100, memory_total_bytes: 999 })).memoryBytes,
    ).toBe(100);
  });

  it('falls back to the machine total outside a cgroup', () => {
    expect(capacityOf(system({ memory_total_bytes: 999 })).memoryBytes).toBe(999);
  });

  it('compares by value so a fresh poll is not a new capacity', () => {
    const stats = { cpu_cores: 8, memory_total_bytes: 16, disk_total_bytes: 32 };
    expect(sameCapacity(capacityOf(system(stats)), capacityOf(system(stats)))).toBe(true);
    expect(
      sameCapacity(capacityOf(system(stats)), capacityOf(system({ ...stats, cpu_cores: 4 }))),
    ).toBe(false);
  });
});

describe('gaugePoints', () => {
  it('maps recorded samples straight through', () => {
    expect(gaugePoints(history({ disk_used_bytes: [[60, 5]] }), 'disk_used_bytes')).toEqual([
      { at: 60, values: { value: 5 } },
    ]);
  });

  it('is empty without history or for an unrecorded metric', () => {
    expect(gaugePoints(null, 'anything')).toEqual([]);
    expect(gaugePoints(history({}), 'missing')).toEqual([]);
  });
});

describe('ratePoints', () => {
  it('turns counter samples into per-second rates, dropping the first', () => {
    const points = ratePoints(
      history({
        cpu_seconds: [
          [60, 0],
          [120, 60],
          [180, 90],
        ],
      }),
      {
        value: 'cpu_seconds',
      },
    );
    expect(points).toEqual([
      { at: 120, values: { value: 1 } },
      { at: 180, values: { value: 0.5 } },
    ]);
  });

  it('clamps a counter reset to zero instead of drawing a spike down', () => {
    const points = ratePoints(
      history({
        cpu_seconds: [
          [60, 600],
          [120, 0],
          [180, 60],
        ],
      }),
      {
        value: 'cpu_seconds',
      },
    );
    expect(points.map((point) => point.values.value)).toEqual([0, 1]);
  });

  it('merges several counters onto one point per moment', () => {
    const points = ratePoints(
      history({
        network_rx_bytes: [
          [60, 0],
          [120, 600],
        ],
        network_tx_bytes: [
          [60, 0],
          [120, 120],
        ],
      }),
      { rx: 'network_rx_bytes', tx: 'network_tx_bytes' },
    );
    expect(points).toEqual([{ at: 120, values: { rx: 10, tx: 2 } }]);
  });

  it('is empty without history', () => {
    expect(ratePoints(null, { value: 'cpu_seconds' })).toEqual([]);
  });
});

describe('percentPoints', () => {
  it('scales a gauge against its capacity', () => {
    expect(percentPoints(history({ memory_bytes: [[60, 25]] }), 'memory_bytes', 50)).toEqual([
      { at: 60, values: { value: 50 } },
    ]);
  });

  it('draws nothing when the capacity is unknown or zero', () => {
    const recorded = history({ memory_bytes: [[60, 25]] });
    expect(percentPoints(recorded, 'memory_bytes', null)).toEqual([]);
    expect(percentPoints(recorded, 'memory_bytes', 0)).toEqual([]);
  });
});

describe('cpuPoints', () => {
  it('spreads the process rate across the machine cores', () => {
    const recorded = history({
      cpu_seconds: [
        [60, 0],
        [120, 120],
      ],
    });
    expect(cpuPoints(recorded, 4).map((point) => point.values.value)).toEqual([50]);
    expect(cpuPoints(recorded, null).map((point) => point.values.value)).toEqual([200]);
  });
});

describe('cpuPercent', () => {
  it('derives load from two consecutive samples', () => {
    const previous = { at: 100, stats: system({ cpu_seconds: 10 }) };
    const current = { at: 110, stats: system({ cpu_seconds: 15 }) };
    expect(cpuPercent(previous, current)).toBe(50);
  });

  it('has no answer without a previous sample or a positive interval', () => {
    const current = { at: 110, stats: system({ cpu_seconds: 15 }) };
    expect(cpuPercent(null, current)).toBeNull();
    expect(cpuPercent({ at: 110, stats: system({ cpu_seconds: 10 }) }, current)).toBeNull();
  });

  it('clamps a restart to zero', () => {
    const previous = { at: 100, stats: system({ cpu_seconds: 900 }) };
    const current = { at: 110, stats: system({ cpu_seconds: 1 }) };
    expect(cpuPercent(previous, current)).toBe(0);
  });

  it('is null where /proc is unreadable', () => {
    expect(cpuPercent({ at: 100, stats: system() }, { at: 110, stats: system() })).toBeNull();
  });
});

describe('networkRates', () => {
  it('derives bytes per second per direction', () => {
    const previous = { at: 100, stats: system({ network_rx_bytes: 0, network_tx_bytes: 0 }) };
    const current = { at: 110, stats: system({ network_rx_bytes: 100, network_tx_bytes: 50 }) };
    expect(networkRates(previous, current)).toEqual({ rx: 10, tx: 5 });
  });

  it('is null when either counter is unreadable', () => {
    const previous = { at: 100, stats: system({ network_rx_bytes: 0 }) };
    const current = { at: 110, stats: system({ network_rx_bytes: 100 }) };
    expect(networkRates(previous, current)).toBeNull();
  });
});

describe('percentOf', () => {
  it('handles the unknown readings the console has to render', () => {
    expect(percentOf(25, 50)).toBe(50);
    expect(percentOf(null, 50)).toBeNull();
    expect(percentOf(25, null)).toBeNull();
    expect(percentOf(25, 0)).toBeNull();
  });
});

describe('formatting', () => {
  it('scales bytes to the largest unit that fits', () => {
    expect(formatBytes(null)).toBe('—');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(1024 ** 2 * 3)).toBe('3.0 MB');
    expect(formatBytes(1024 ** 3 * 2.5)).toBe('2.5 GB');
  });

  it('reports uptime at the coarsest useful precision', () => {
    expect(formatUptime(null)).toBe('—');
    expect(formatUptime(90)).toBe('1m');
    expect(formatUptime(3600 * 5 + 60 * 7)).toBe('5h 7m');
    expect(formatUptime(86_400 * 2 + 3600 * 3)).toBe('2d 3h');
  });

  it('compacts counts only once they get long', () => {
    expect(compact(999)).toBe('999');
    expect(compact(9_999)).toBe('9,999');
    expect(compact(12_500)).toBe('12.5K');
    expect(compact(2_400_000)).toBe('2.4M');
  });
});
