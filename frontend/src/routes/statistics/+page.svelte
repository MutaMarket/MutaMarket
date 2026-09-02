<script lang="ts">
  // The statistics overview: one HUD telemetry board instead of a
  // grid of cards. Hero numerals up top, then the market, roll-bar
  // and activity clusters separated by hairlines, with the view's
  // refresh stamp as the footer readout.
  import PageHeader from '$lib/components/page-header.svelte';
  import StatisticsTabs from '$lib/components/statistics-tabs.svelte';
  import { toIskCompact } from '$lib/format-number';
  import { syncLabel } from '$lib/statistics';
  import { t } from '$lib/i18n.svelte';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const stats = $derived(data.overview.stats);

  const market = $derived([
    { label: t('stats.overview.modulesForSale'), value: stats.listed_count },
    { label: t('stats.overview.contracts'), value: stats.contracts_count },
    { label: t('stats.overview.itemExchanges'), value: stats.item_exchanges_count },
    { label: t('stats.overview.auctions'), value: stats.auctions_count },
  ]);

  const bars = $derived([
    { label: t('stats.overview.goldbars'), value: stats.goldbars_count, tone: 'text-gold' },
    { label: t('stats.overview.brownbars'), value: stats.brownbars_count, tone: 'text-brown' },
    {
      label: t('stats.overview.diamondbars'),
      value: stats.diamondbars_count,
      tone: 'text-diamond',
    },
  ]);

  const activity = $derived.by(() => {
    const week = Math.max(1, stats.added_last_week_count);
    return [
      { label: t('stats.overview.addedLastHour'), value: stats.added_last_hour_count },
      { label: t('stats.overview.addedLastDay'), value: stats.added_last_day_count },
      { label: t('stats.overview.addedLastWeek'), value: stats.added_last_week_count },
    ].map((row) => ({ ...row, share: Math.max(1, Math.round((row.value / week) * 100)) }));
  });

  const n = (value: number) => value.toLocaleString('en-US');
</script>

<PageMeta title={t('meta.allStatistics.title')} description={t('meta.allStatistics.description')} />

<PageHeader title={t('nav.menu.statistics')} subtitle={t('stats.header.subtitle')} />
<StatisticsTabs />

<div class="hud-frame divide-y divide-border">
  <!-- Hero numerals -->
  <div class="grid divide-y divide-border md:grid-cols-[3fr_2fr_2fr] md:divide-x md:divide-y-0">
    <div class="p-6">
      <h2 class="hud-label">{t('stats.overview.modulesInDatabase')}</h2>
      <div
        class="mt-2 font-mono text-5xl text-primary [text-shadow:0_0_24px_color-mix(in_srgb,var(--color-primary)_35%,transparent)]"
      >
        {n(stats.total_count)}
      </div>
    </div>
    <div class="p-6">
      <h2 class="hud-label">{t('stats.overview.totalValue')}</h2>
      <div class="mt-2 font-mono text-3xl md:mt-4">{toIskCompact(data.overview.total_value)}</div>
    </div>
    <div class="p-6">
      <h2 class="hud-label">{t('stats.overview.averageValue')}</h2>
      <div class="mt-2 font-mono text-3xl md:mt-4">
        {toIskCompact(data.overview.average_value)}
      </div>
    </div>
  </div>

  <!-- Clusters -->
  <div class="grid divide-y divide-border lg:grid-cols-3 lg:divide-x lg:divide-y-0">
    <div class="p-6">
      <h2 class="hud-label mb-4">{t('stats.overview.market')}</h2>
      <dl class="grid grid-cols-2 gap-x-6 gap-y-4">
        {#each market as row (row.label)}
          <div>
            <dt class="text-xs text-muted-foreground">{row.label}</dt>
            <dd class="font-mono text-xl">{n(row.value)}</dd>
          </div>
        {/each}
      </dl>
    </div>
    <div class="p-6">
      <h2 class="hud-label mb-4">{t('stats.overview.rollBars')}</h2>
      <dl class="grid grid-cols-3 gap-x-6 gap-y-4">
        {#each bars as row (row.label)}
          <div>
            <dt class="text-xs text-muted-foreground">{row.label}</dt>
            <dd class="font-mono text-xl {row.tone}">{n(row.value)}</dd>
          </div>
        {/each}
      </dl>
    </div>
    <div class="p-6">
      <h2 class="hud-label mb-4">{t('stats.overview.modulesAdded')}</h2>
      <dl class="grid gap-3">
        {#each activity as row (row.label)}
          <div class="grid grid-cols-[6rem_1fr_auto] items-center gap-3">
            <dt class="text-xs text-muted-foreground">{row.label}</dt>
            <dd class="h-1 overflow-hidden rounded-full bg-card-2">
              <div class="h-full bg-primary/70" style="width: {row.share}%"></div>
            </dd>
            <dd class="font-mono text-sm tabular-nums">{n(row.value)}</dd>
          </div>
        {/each}
      </dl>
    </div>
  </div>

  <!-- Telemetry footer -->
  <div
    class="flex flex-wrap items-center justify-between gap-2 px-6 py-3 font-mono text-2xs tracking-[0.14em] text-muted-foreground uppercase"
  >
    <span>
      {t('stats.overview.footerCounts', {
        creators: n(data.overview.creators_count),
        characters: n(data.overview.characters_count),
      })}
    </span>
    <span>{t('stats.overview.telemetryAsOf', { time: syncLabel(data.overview.refreshed_at) })}</span
    >
  </div>
</div>
