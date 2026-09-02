<script lang="ts">
  // The console overview: who the background work acts through, the
  // container's vitals over the toggled window, what the ingestion has
  // landed, and a one-line-per-job roll-up that links into the jobs
  // board. The heavy per-job cards and the ESI charts live on their own
  // sections, so this page only ever mounts the five vital charts.
  import VitalChart from '$lib/components/vital-chart.svelte';
  import { apply, live, subscribe } from '$lib/admin-live.svelte';
  import {
    HISTORY_WINDOWS,
    cpuPercent,
    cpuPoints,
    formatBytes,
    gaugePoints,
    loadSeries,
    networkRates,
    networkSeries,
    percentOf,
    percentPoints,
    ratePoints,
    sizeSeries,
    usedSeries,
    type HistoryWindow,
  } from '$lib/admin-vitals';
  import { JOB_CARDS, jobCard } from '$lib/job-cards';
  import { parseDbTimestamp, relativeTime } from '$lib/duration';
  import { t } from '$lib/i18n.svelte';
  import type { MetricsHistory } from '$lib/admin-types';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  $effect(() => {
    apply(data.live);
  });
  $effect(() => subscribe(['system', 'database', 'jobs']));

  const system = $derived(live.system ?? data.live.system ?? null);
  const database = $derived(live.database ?? data.live.database ?? null);

  // The capacities the charts divide by, as their own derived numbers.
  // They never move, so a poll that reassigns `system` recomputes them
  // to the same value and stops there — the history-derived point
  // arrays below are not rebuilt. Reading system.cpu_cores inside those
  // deriveds instead is what used to redraw the CPU chart every five
  // seconds.
  const cores = $derived(system?.cpu_cores ?? null);
  /** The cgroup limit, else the machine's total memory. */
  const memoryCapacity = $derived(
    system === null ? null : (system.memory_limit_bytes ?? system.memory_total_bytes),
  );
  const diskCapacity = $derived(system?.disk_total_bytes ?? null);

  // --- Vitals history ----------------------------------------------------

  let historyWindow = $state<HistoryWindow>('24h');
  let history = $state<MetricsHistory | null>(null);

  $effect(() => {
    const window = historyWindow;
    void (async () => {
      const response = await fetch(`/api/admin/metrics?window=${window}`);
      if (response.ok) {
        history = await response.json();
      }
    })();
  });

  // Only `history` and the (stable) capacities feed these, so a poll
  // that leaves them alone never hands the charts new data.
  const cpu = $derived(cpuPoints(history, cores));
  const memory = $derived(percentPoints(history, 'memory_bytes', memoryCapacity));
  const disk = $derived(percentPoints(history, 'disk_used_bytes', diskCapacity));
  const network = $derived(ratePoints(history, { rx: 'network_rx_bytes', tx: 'network_tx_bytes' }));
  const databaseSize = $derived(gaugePoints(history, 'database_size_bytes'));

  const sample = $derived(live.currentSample);
  const load = $derived(sample === null ? null : cpuPercent(live.previousSample, sample));
  const rates = $derived(sample === null ? null : networkRates(live.previousSample, sample));
  const cpuUtilization = $derived(load === null ? null : load / (cores ?? 1));
  const memoryUsed = $derived(
    system === null ? null : (system.memory_current_bytes ?? system.memory_rss_bytes),
  );
  const memoryPercent = $derived(percentOf(memoryUsed, memoryCapacity));
  const diskPercent = $derived(percentOf(system?.disk_used_bytes ?? null, diskCapacity));

  const databaseTiles = $derived(
    database === null
      ? []
      : ([
          [t('admin.overview.tiles.modules'), database.modules],
          [t('admin.overview.tiles.noEstimate'), database.modules_without_estimate],
          [t('admin.overview.tiles.contracts'), database.contracts],
          [t('admin.overview.tiles.contractItems'), database.contract_items],
          [t('admin.overview.tiles.characters'), database.characters],
          [t('admin.overview.tiles.users'), database.users],
          [t('admin.overview.tiles.assets'), database.assets],
          [t('admin.overview.tiles.publicOwnerships'), database.public_ownerships],
          [t('admin.overview.tiles.marketDays'), database.market_history_days],
        ] as const),
  );

  // --- Job roll-up -------------------------------------------------------

  /** Jobs needing attention first: failing, then paused, then the rest. */
  const jobSummary = $derived(
    live.jobs
      .map((job) => {
        const last = job.last_runs.find((run) => run.finished_at !== null) ?? null;
        const failed = last?.outcome === 'error';
        return {
          name: job.name,
          title: job.name in JOB_CARDS ? jobCard(job.name).title : job.name,
          running: job.running,
          paused: job.paused,
          failed,
          last,
          rank: failed ? 0 : job.paused ? 1 : 2,
        };
      })
      .sort((a, b) => a.rank - b.rank || a.title.localeCompare(b.title)),
  );
  const attention = $derived(jobSummary.filter((job) => job.rank < 2));
  const running = $derived(jobSummary.filter((job) => job.running).length);
</script>

<svelte:head><title>{t('meta.admin.title')} - MutaMarket</title></svelte:head>

<!-- Service character: who the background features act through
     (structure resolution, donation processing when it lands). -->
<section class="mb-8">
  <h2 class="hud-label mb-3">{t('admin.overview.serviceHeading')}</h2>
  <div class="hud-frame flex flex-wrap items-center gap-4 p-4">
    {#if data.service.character}
      <img
        src="https://images.evetech.net/characters/{data.service.character.id}/portrait?size=64"
        alt=""
        class="size-12 rounded-lg"
      />
      <div>
        <div class="font-medium">
          {data.service.character.name ??
            t('admin.overview.characterFallback', { id: data.service.character.id })}
        </div>
        <div class="text-xs text-muted-foreground">
          {data.service.source === 'env'
            ? t('admin.overview.serviceFromEnv')
            : t('admin.overview.scopesAuthorized', {
                count: data.service.character.scopes.length,
              })}
          · {t('admin.overview.serviceRoles')}
        </div>
      </div>
    {:else}
      <div class="text-sm text-muted-foreground">
        {t('admin.overview.noServiceCharacter')}
      </div>
    {/if}
    <a
      href="/eve/admin"
      rel="external"
      class="ml-auto rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground transition hover:brightness-110"
    >
      {data.service.character ? t('admin.overview.reauthorize') : t('admin.overview.authorize')}
    </a>
  </div>
</section>

<!-- System: live vitals with their recorded history, one card each. -->
<section class="mb-8">
  <div class="mb-3 flex items-center gap-4">
    <h2 class="hud-label">{t('admin.overview.systemHeading')}</h2>
    <div class="flex rounded-[7px] border border-border bg-card-2 p-0.5">
      {#each HISTORY_WINDOWS as window (window)}
        <button
          type="button"
          class="flex h-6 items-center rounded-[5px] px-2.5 text-xs transition-colors {historyWindow ===
          window
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => (historyWindow = window)}
        >
          {window}
        </button>
      {/each}
    </div>
  </div>
  <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-5">
    <VitalChart
      title={t('admin.vitals.cpu')}
      headline={cpuUtilization === null ? '—' : `${cpuUtilization.toFixed(0)}%`}
      sub={cores !== null ? t('admin.vitals.ofCores', { count: cores }) : undefined}
      series={loadSeries()}
      points={cpu}
      yDomain={[0, 100]}
      format={(value) => `${value.toFixed(0)}%`}
    />
    <!-- Utilization needs a capacity: the cgroup limit, else the
		     machine's total memory. Without either (non-Linux) the chart
		     falls back to plain bytes. -->
    {#if memoryCapacity !== null}
      <VitalChart
        title={t('admin.vitals.memory')}
        headline={memoryPercent === null ? '—' : `${memoryPercent.toFixed(0)}%`}
        sub={t('admin.vitals.usedOf', {
          used: formatBytes(memoryUsed),
          capacity: formatBytes(memoryCapacity),
        })}
        series={usedSeries()}
        points={memory}
        yDomain={[0, 100]}
        format={(value) => `${value.toFixed(0)}%`}
      />
    {:else}
      <VitalChart
        title={t('admin.vitals.memory')}
        headline={formatBytes(memoryUsed)}
        series={usedSeries()}
        points={gaugePoints(history, 'memory_bytes')}
        format={(value) => formatBytes(Math.round(value))}
      />
    {/if}
    <VitalChart
      title={t('admin.vitals.storage')}
      headline={diskPercent === null ? '—' : `${diskPercent.toFixed(0)}%`}
      sub={system?.disk_used_bytes != null && diskCapacity !== null
        ? t('admin.vitals.freeOf', {
            free: formatBytes(diskCapacity - system.disk_used_bytes),
            capacity: formatBytes(diskCapacity),
          })
        : undefined}
      series={usedSeries()}
      points={disk}
      yDomain={[0, 100]}
      format={(value) => `${value.toFixed(0)}%`}
    />
    <VitalChart
      title={t('admin.vitals.network')}
      headline={rates === null
        ? '—'
        : `${formatBytes(Math.round(rates.rx))}/s · ${formatBytes(Math.round(rates.tx))}/s`}
      sub={t('admin.vitals.inOut')}
      series={networkSeries()}
      points={network}
      format={(value) => formatBytes(Math.round(value))}
    />
    <VitalChart
      title={t('admin.vitals.database')}
      headline={formatBytes(system?.database_size_bytes ?? null)}
      series={sizeSeries()}
      points={databaseSize}
      format={(value) => formatBytes(Math.round(value))}
    />
  </div>
</section>

<!-- Database: what the background work is landing. -->
<section class="mb-8">
  <h2 class="hud-label mb-3">{t('admin.overview.databaseHeading')}</h2>
  <div class="grid grid-cols-3 gap-2 sm:grid-cols-5 lg:grid-cols-9">
    {#each databaseTiles as [label, value] (label)}
      <div class="hud-panel px-3 py-2.5">
        <div class="text-sm font-semibold text-foreground tabular-nums">
          {value.toLocaleString('en-US')}
        </div>
        <div class="truncate text-xs text-muted-foreground">{label}</div>
      </div>
    {/each}
  </div>
</section>

<!-- Jobs roll-up: the state of the board without its charts. Anything
     failing or paused surfaces here; the rest is a count. -->
<section>
  <div class="mb-3 flex items-center gap-4">
    <h2 class="hud-label">{t('admin.overview.jobsHeading')}</h2>
    <a class="text-xs text-muted-foreground hover:text-foreground" href="/admin/jobs">
      {t('admin.overview.openBoard')}
    </a>
    <span class="ml-auto text-xs text-muted-foreground tabular-nums">
      {t('admin.overview.jobsSummary', { count: jobSummary.length, running })}
    </span>
  </div>
  <div class="hud-frame divide-y divide-border">
    {#each attention as job (job.name)}
      <a
        href="/admin/jobs"
        class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 transition hover:bg-white/[0.03]"
      >
        <span class="size-2 shrink-0 rounded-full {job.failed ? 'bg-negative' : 'bg-[#fab219]'}"
        ></span>
        <span class="text-sm font-medium">{job.title}</span>
        <span class="text-xs text-muted-foreground">
          {job.failed ? t('admin.jobs.lastRunFailed') : t('admin.jobs.paused')}
        </span>
        {#if job.failed && job.last?.error}
          <span class="min-w-0 truncate text-xs text-negative">{job.last.error}</span>
        {/if}
        {#if job.last}
          <span class="ml-auto text-xs text-muted-foreground">
            {relativeTime(parseDbTimestamp(job.last.finished_at ?? job.last.started_at) - live.now)}
          </span>
        {/if}
      </a>
    {:else}
      <p class="px-4 py-3 text-sm text-muted-foreground">
        {t('admin.overview.allJobsHealthy')}
      </p>
    {/each}
  </div>
</section>
