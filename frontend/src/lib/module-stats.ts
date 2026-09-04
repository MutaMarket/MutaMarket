// The header readout every scoped module page shares (character,
// collection, location, personal, sell): the page's own lead counts,
// then the total value and the roll-bar counts, the legacy
// *ModuleStats components' common cells. A bar cell only appears when
// the set holds such a module, and its number stays plain: the
// animated gold/diamond text effects read wrong on a bare figure.
import type { HeaderStat } from '$lib/components/page-header.svelte';
import { toIskCompact } from '$lib/format-number';
import { t } from '$lib/i18n.svelte';
import type { ScopedModuleStats } from '$lib/types';

export function countStat(label: string, value: number, accent?: HeaderStat['accent']): HeaderStat {
  return { label, value: value.toLocaleString('en-US'), accent };
}

export function scopedModuleStats(lead: HeaderStat[], stats: ScopedModuleStats): HeaderStat[] {
  const bars: [string, number][] = [
    ['stats.overview.goldbars', stats.goldbars_count],
    ['stats.overview.brownbars', stats.brownbars_count],
    ['stats.overview.diamondbars', stats.diamondbars_count],
  ];
  return [
    ...lead,
    { label: t('stats.overview.totalValue'), value: toIskCompact(stats.total_value) },
    ...bars.filter(([, count]) => count > 0).map(([key, count]) => countStat(t(key), count)),
  ];
}
