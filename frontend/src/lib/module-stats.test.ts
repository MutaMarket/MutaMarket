import { describe, expect, it } from 'vitest';

import { toIskCompact } from './format-number';
import { t } from './i18n.svelte';
import { countStat, scopedModuleStats } from './module-stats';

describe('scopedModuleStats', () => {
  it('appends the value and the present bar counts after the lead counts', () => {
    const lead = countStat('Owned', 1234, 'primary');
    expect(
      scopedModuleStats([lead], {
        total_count: 1234,
        total_value: 2_500_000_000,
        average_value: 2_025_931.9,
        goldbars_count: 12,
        brownbars_count: 0,
        diamondbars_count: 1,
      }),
    ).toEqual([
      { label: 'Owned', value: '1,234', accent: 'primary' },
      { label: t('stats.overview.totalValue'), value: toIskCompact(2_500_000_000) },
      { label: t('stats.overview.goldbars'), value: '12', accent: undefined },
      { label: t('stats.overview.diamondbars'), value: '1', accent: undefined },
    ]);
  });

  it('shows no bar cells for a set without bars', () => {
    expect(
      scopedModuleStats([], {
        total_count: 0,
        total_value: 0,
        average_value: 0,
        goldbars_count: 0,
        brownbars_count: 0,
        diamondbars_count: 0,
      }),
    ).toEqual([{ label: t('stats.overview.totalValue'), value: toIskCompact(0) }]);
  });
});
