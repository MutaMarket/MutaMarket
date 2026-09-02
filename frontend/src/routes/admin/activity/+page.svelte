<script lang="ts">
  // Request activity: the live minute, traffic over the window split by
  // whether the request carried a session, who is making the most of
  // it, and how many of them come back month over month.
  //
  // Every number here excludes the console's own polls, which would
  // otherwise dwarf real traffic and put whoever has this page open at
  // the top of the leaderboard.
  import TelemetryChart from '$lib/components/telemetry-chart.svelte';
  import { apply, live, subscribe } from '$lib/admin-live.svelte';
  import {
    ACTIVITY_WINDOWS,
    bucketLabel,
    cohortBuckets,
    cohortSeries,
    requestsPerUser,
    signedInShare,
    tickPredicate,
    trafficBuckets,
    trafficSeries,
    userBuckets,
    usersSeries,
    type ActivityWindow,
  } from '$lib/admin-activity';
  import { compact } from '$lib/admin-vitals';
  import { t } from '$lib/i18n.svelte';
  import type { ActivityHistory } from '$lib/admin-types';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  $effect(() => {
    apply(data.live);
  });
  $effect(() => subscribe(['activity']));

  const snapshot = $derived(live.activity ?? data.live.activity ?? null);

  // --- The window toggle -------------------------------------------------

  let activityWindow = $state<ActivityWindow>('24h');
  let history = $state<ActivityHistory | null>(null);

  $effect(() => {
    const wanted = activityWindow;
    void (async () => {
      const response = await fetch(`/api/admin/activity?window=${wanted}`);
      if (response.ok) {
        history = await response.json();
      }
    })();
  });

  const report = $derived(history ?? data.history);
  const now = $derived(live.now);

  /** Buckets the window holds, so a quiet one still takes its place. */
  const bucketCount = $derived(
    report.step_seconds >= 86_400 ? 30 : activityWindow === '24h' ? 24 : 24 * 7,
  );
  const traffic = $derived(trafficBuckets(report.traffic, report.step_seconds, now, bucketCount));
  const users = $derived(userBuckets(report.daily_users, bucketCount === 24 ? 7 : 30, now));
  const cohorts = $derived(cohortBuckets(report.months));

  const share = $derived(signedInShare(report.totals));
  const perUser = $derived(requestsPerUser(report.totals));

  const tiles = $derived([
    [t('admin.activity.tiles.requests'), compact(report.totals.requests)],
    [t('admin.activity.tiles.signedIn'), share === null ? '—' : `${share.toFixed(0)}%`],
    [t('admin.activity.tiles.pageViews'), compact(report.totals.page_views)],
    [t('admin.activity.tiles.activeUsers'), compact(report.totals.active_users)],
    [t('admin.activity.tiles.newThisMonth'), compact(report.totals.new_users)],
    [t('admin.activity.tiles.requestsPerUser'), perUser === null ? '—' : perUser.toFixed(0)],
  ] as const);

  const liveSeries = $derived(trafficSeries());
  const liveBuckets = $derived(
    (snapshot?.buckets ?? []).map((bucket) => ({
      minuteStart: bucket.minute_start,
      values: { signed_in: bucket.signed_in, anonymous: bucket.anonymous },
    })),
  );
</script>

<svelte:head>
  <title>{t('meta.adminActivity.title')} - {t('meta.admin.title')} - MutaMarket</title>
</svelte:head>

<!-- The window's headline numbers. -->
<section class="mb-8">
  <div class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
    {#each tiles as [label, value] (label)}
      <div class="hud-panel px-3 py-2.5">
        <div class="text-sm font-semibold text-foreground tabular-nums">{value}</div>
        <div class="truncate text-xs text-muted-foreground">{label}</div>
      </div>
    {/each}
  </div>
</section>

<!-- Live: the last hour, from the in-memory recorder. -->
<section class="mb-8">
  <h2 class="hud-label mb-3">{t('admin.activity.liveHeading')}</h2>
  <TelemetryChart
    title={t('admin.telemetry.requestsPerMinute')}
    headline={compact(snapshot?.hour.requests ?? 0)}
    headlineClass="text-primary"
    sub={t('admin.activity.liveSub', { count: snapshot?.hour.users ?? 0 })}
    series={liveSeries}
    minutes={liveBuckets}
    emptyText={t('admin.activity.liveEmpty')}
  />
</section>

<!-- The windowed report. -->
<section class="mb-8">
  <div class="mb-3 flex items-center gap-4">
    <h2 class="hud-label">{t('admin.activity.trafficHeading', { window: report.window })}</h2>
    <div class="flex rounded-[7px] border border-border bg-card-2 p-0.5">
      {#each ACTIVITY_WINDOWS as option (option)}
        <button
          type="button"
          class="flex h-6 items-center rounded-[5px] px-2.5 text-xs transition-colors {activityWindow ===
          option
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:text-foreground'}"
          onclick={() => (activityWindow = option)}
        >
          {option}
        </button>
      {/each}
    </div>
  </div>
  <div class="grid gap-3 xl:grid-cols-2">
    <TelemetryChart
      title={t('admin.activity.tiles.requests')}
      headline={compact(report.totals.requests)}
      sub={t('admin.activity.trafficSub')}
      series={trafficSeries()}
      minutes={traffic}
      emptyText={t('admin.activity.trafficEmpty')}
      formatLabel={bucketLabel(report.step_seconds)}
      tickAt={tickPredicate(traffic.length)}
    />
    <TelemetryChart
      title={t('admin.activity.activeUsersPerDay')}
      headline={compact(report.totals.active_users)}
      sub={t('admin.activity.activeUsersSub')}
      series={usersSeries()}
      minutes={users}
      emptyText={t('admin.activity.activeUsersEmpty')}
      formatLabel={bucketLabel(86_400)}
      tickAt={tickPredicate(users.length)}
    />
  </div>
</section>

<!-- New versus returning, always over the same 24 months. -->
<section class="mb-8">
  <h2 class="hud-label mb-3">{t('admin.activity.cohortsHeading')}</h2>
  <TelemetryChart
    title={t('admin.activity.activeUsersPerMonth')}
    headline={compact(report.months.at(-1)?.active_users ?? 0)}
    sub={t('admin.activity.cohortsSub')}
    series={cohortSeries()}
    minutes={cohorts}
    emptyText={t('admin.activity.cohortsEmpty')}
    formatLabel={bucketLabel(2_592_000)}
    tickAt={tickPredicate(cohorts.length, 12)}
  />
</section>

<div class="grid gap-8 xl:grid-cols-2">
  <!-- Who is making the requests. -->
  <section>
    <h2 class="hud-label mb-3">{t('admin.activity.topUsersHeading')}</h2>
    <div class="hud-frame divide-y divide-border">
      {#each report.top_users as user, index (user.user_id)}
        <div class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5">
          <span class="w-5 shrink-0 text-xs text-muted-foreground tabular-nums">
            {index + 1}
          </span>
          <span class="min-w-0 truncate text-sm font-medium">{user.name}</span>
          <span class="text-xs text-muted-foreground">
            {t('admin.activity.userActivity', { count: user.active_days, since: user.created_at })}
          </span>
          <span class="ml-auto text-sm tabular-nums">
            {user.requests.toLocaleString('en-US')}
          </span>
        </div>
      {:else}
        <p class="px-4 py-3 text-sm text-muted-foreground">{t('admin.activity.topUsersEmpty')}</p>
      {/each}
    </div>
  </section>

  <!-- What they asked for. -->
  <section>
    <h2 class="hud-label mb-3">{t('admin.activity.routesHeading')}</h2>
    <div class="hud-frame divide-y divide-border">
      {#each report.routes as route (route.route)}
        <div class="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5">
          <span class="min-w-0 truncate font-mono text-xs">{route.route}</span>
          {#if route.errors > 0}
            <span class="shrink-0 text-xs text-negative tabular-nums">
              {t('admin.activity.routeErrors', { count: route.errors.toLocaleString('en-US') })}
            </span>
          {/if}
          <span class="ml-auto shrink-0 text-xs text-muted-foreground tabular-nums">
            {t('admin.activity.routeAverageMs', { ms: route.average_ms.toFixed(0) })}
          </span>
          <span class="w-20 shrink-0 text-right text-sm tabular-nums">
            {route.requests.toLocaleString('en-US')}
          </span>
        </div>
      {:else}
        <p class="px-4 py-3 text-sm text-muted-foreground">{t('admin.activity.routesEmpty')}</p>
      {/each}
    </div>
  </section>
</div>
