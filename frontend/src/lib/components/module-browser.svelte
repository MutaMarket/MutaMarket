<script lang="ts">
  // The module browser, mirroring the legacy browse pages: the filter
  // band above the grid, then the options
  // bar and the masonry card grid.
  import FilterBand from './filter-band.svelte';
  import Logo from './logo.svelte';
  import ModuleDisplay from './module-display.svelte';
  import PageHeader, { type HeaderStat } from './page-header.svelte';
  import type { DisplaySettings } from '$lib/display';
  import { t } from '$lib/i18n.svelte';
  import { parseQueryUi } from '$lib/query';
  import type { BrowserData } from '$lib/server/browser';

  let { data, settings }: { data: BrowserData; settings: DisplaySettings } = $props();

  const search = $derived(parseQueryUi(data.query));
  const historic = $derived(data.prefix === 'historic-sales');
  // Historic sales list the archive too (unlisted stats), so the market
  // cells (for sale, auctions) would be off-topic there.
  const archive = $derived(data.prefix === 'all-modules' || historic);

  const count = (value: number) => value.toLocaleString('en-US');

  const stats = $derived.by((): HeaderStat[] => {
    if (!data.stats) {
      return [];
    }
    if (archive) {
      return [
        {
          label: t(historic ? 'stats.overview.modules' : 'modules.browser.statArchived'),
          value: count(data.stats.total_count),
          accent: 'primary',
        },
        { label: t('modules.browser.statGoldBars'), value: count(data.stats.goldbars_count) },
        { label: t('modules.browser.statDiamondBars'), value: count(data.stats.diamondbars_count) },
        { label: t('modules.browser.statAddedDay'), value: count(data.stats.added_last_day_count) },
      ];
    }
    return [
      {
        label: t('modules.browser.statForSale'),
        value: count(data.stats.listed_count),
        accent: 'primary',
      },
      { label: t('stats.overview.auctions'), value: count(data.stats.auctions_count) },
      { label: t('modules.browser.statExchanges'), value: count(data.stats.item_exchanges_count) },
      { label: t('modules.browser.statAddedDay'), value: count(data.stats.added_last_day_count) },
    ];
  });
</script>

<PageHeader
  title={historic
    ? t('meta.historicSales.title')
    : archive
      ? t('nav.menu.allModules')
      : t('modules.browser.titleMarket')}
  subtitle={historic
    ? t('modules.browser.subtitleHistoric')
    : archive
      ? t('modules.browser.subtitleArchive')
      : t('modules.browser.subtitleMarket')}
  {stats}
>
  {#snippet icon()}
    <Logo class="size-9 {archive || historic ? 'text-muted-foreground' : 'text-primary'}" />
  {/snippet}
</PageHeader>
<FilterBand
  prefix={data.prefix}
  {search}
  panel={data.panel}
  unknownType={data.unknownType}
  variant={data.prefix === 'modules' ? 'market' : historic ? 'historic' : 'archive'}
/>
<div class="my-4 w-full">
  <ModuleDisplay
    entries={data.modules.map((module) => ({ module }))}
    {settings}
    panel={data.panel}
    {search}
    prefix={data.prefix}
    allowSortByPrice={data.prefix === 'modules' || historic}
  />
</div>
