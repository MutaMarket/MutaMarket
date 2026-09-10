<script lang="ts">
  // The failures behind the activity roll-up's error counts: which routes
  // are answering with an error at all, and the captured requests
  // themselves, newest first. The counts on the activity page are exact;
  // these are a sample of at most a few per route and status per minute,
  // so a burst shows fewer rows than it cost.
  import RequestFailureDialog from '$lib/components/request-failure-dialog.svelte';
  import { live } from '$lib/admin-live.svelte';
  import {
    accountLabel,
    failureAt,
    failuresQuery,
    statusClass,
    type StatusClass,
  } from '$lib/admin-request-failures';
  import { relativeTime } from '$lib/duration';
  import { t } from '$lib/i18n.svelte';
  import type { RequestFailureSummary, RequestFailuresPayload } from '$lib/admin-types';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  /** The filtered list, once a filter or the refresh has asked for one;
   * the SSR payload until then. */
  let fetched = $state<RequestFailuresPayload | null>(null);
  const payload = $derived(fetched ?? data.failures);
  // svelte-ignore state_referenced_locally -- the URL seeds the filters once
  let route = $state<string | null>(data.route);
  // svelte-ignore state_referenced_locally -- the URL seeds the filters once
  let statuses = $state<StatusClass | null>(data.statusClass);
  let loading = $state(false);
  let inspecting = $state<RequestFailureSummary | null>(null);

  const CLASSES: { value: StatusClass | null; label: () => string }[] = [
    { value: null, label: () => t('admin.requestFailures.allStatuses') },
    { value: 'client', label: () => '4xx' },
    { value: 'server', label: () => '5xx' },
  ];

  async function reload() {
    loading = true;
    const response = await fetch(failuresQuery({ route, class: statuses }));
    if (response.ok) {
      fetched = await response.json();
    }
    loading = false;
  }

  function pickRoute(next: string | null) {
    route = route === next ? null : next;
    void reload();
  }

  function pickClass(next: StatusClass | null) {
    statuses = next;
    void reload();
  }

  const filtered = $derived(route !== null || statuses !== null);
</script>

<svelte:head>
  <title>{t('meta.adminErrors.title')} - {t('meta.admin.title')} - MutaMarket</title>
</svelte:head>

<div class="mb-5 flex flex-wrap items-center gap-3">
  <div class="flex rounded-[9px] border border-border bg-card-2 p-0.5">
    {#each CLASSES as option (option.value ?? 'all')}
      <button
        class="flex h-7 items-center rounded-[6px] px-3 font-mono text-2xs tracking-[0.12em] uppercase transition-colors
          {statuses === option.value
          ? 'bg-primary text-primary-foreground'
          : 'text-muted-foreground hover:bg-white/[0.04] hover:text-foreground'}"
        onclick={() => pickClass(option.value)}
      >
        {option.label()}
      </button>
    {/each}
  </div>

  {#if route !== null}
    <button
      class="flex items-center gap-2 rounded-full border border-border px-2.5 py-0.5 text-xs hover:bg-white/[0.04]"
      onclick={() => pickRoute(null)}
    >
      <span class="font-mono">{route}</span>
      <span class="text-muted-foreground">✕</span>
    </button>
  {/if}

  <a class="text-xs text-primary hover:underline" href="/admin/activity">
    {t('admin.requestFailures.openActivity')}
  </a>

  <span class="ml-auto text-xs text-muted-foreground">
    {t('admin.requestFailures.retention', {
      keep: payload.keep.toLocaleString('en-US'),
      days: payload.retention_days,
    })}
  </span>
</div>

<div class="grid gap-8 xl:grid-cols-[22rem_1fr]">
  <!-- What is failing, over the whole retained window: the roll-up is
       how a route gets picked, so it ignores the filters. -->
  <section>
    <h2 class="hud-label mb-3">{t('admin.requestFailures.routesHeading')}</h2>
    <div class="hud-frame divide-y divide-border">
      {#each payload.routes as entry (entry.route)}
        <button
          class="flex w-full flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 text-left transition hover:bg-white/[0.03]
            {route === entry.route ? 'bg-white/[0.04]' : ''}"
          onclick={() => pickRoute(entry.route)}
        >
          <span class="min-w-0 flex-1 truncate font-mono text-xs">{entry.route}</span>
          <span class="shrink-0 font-mono text-2xs text-muted-foreground tabular-nums">
            {entry.statuses.join(' ')}
          </span>
          <span class="w-14 shrink-0 text-right text-sm tabular-nums">
            {entry.failures.toLocaleString('en-US')}
          </span>
        </button>
      {:else}
        <p class="px-4 py-3 text-sm text-muted-foreground">
          {t('admin.requestFailures.routesEmpty')}
        </p>
      {/each}
    </div>
    <p class="mt-2 text-xs text-muted-foreground">
      {t('admin.requestFailures.sampling', { count: payload.captures_per_minute })}
    </p>
  </section>

  <!-- The captured failures themselves. -->
  <section>
    <div class="mb-3 flex items-center gap-3">
      <h2 class="hud-label">{t('admin.requestFailures.listHeading')}</h2>
      <button
        class="ml-auto rounded-full border border-border px-2.5 py-0.5 text-xs hover:bg-white/[0.04]"
        onclick={() => void reload()}
        disabled={loading}
      >
        {loading ? t('common.actions.loading') : t('common.actions.refresh')}
      </button>
    </div>

    <div class="hud-frame divide-y divide-border">
      {#each payload.failures as failure (failure.id)}
        <button
          class="flex w-full flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 text-left transition hover:bg-white/[0.03]"
          onclick={() => (inspecting = failure)}
        >
          <span
            class="w-9 shrink-0 font-mono text-xs tabular-nums {statusClass(failure.status) ===
            'server'
              ? 'text-negative'
              : 'text-[#ec835a]'}"
          >
            {failure.status}
          </span>
          <span class="min-w-0 flex-1 truncate text-sm">
            <span class="text-muted-foreground">{failure.method}</span>
            {failure.path}
          </span>
          {#if failure.error_message}
            <span class="min-w-0 max-w-64 truncate text-xs text-muted-foreground">
              {failure.error_message}
            </span>
          {/if}
          <span class="shrink-0 text-xs text-muted-foreground">
            {accountLabel(failure)} · {relativeTime(failureAt(failure) - live.now)}
          </span>
        </button>
      {:else}
        <p class="px-4 py-3 text-sm text-muted-foreground">
          {filtered
            ? t('admin.requestFailures.listEmptyFiltered')
            : t('admin.requestFailures.listEmpty')}
        </p>
      {/each}
    </div>
  </section>
</div>

<RequestFailureDialog bind:failure={inspecting} now={live.now} />
