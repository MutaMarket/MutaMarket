<script lang="ts">
  // The asset import readout of the personal page header, replacing the
  // legacy AssetImportStatus.vue sidebar panel: a fixed-footprint
  // status block plus the action button, so the header never shifts
  // when an import starts — the button just swaps to its busy state.
  import { LoaderCircle } from '@lucide/svelte';
  import { elapsedAge } from '$lib/asset-import-stream';
  import type { AssetImportView, PersonalPageData } from '$lib/types';

  let {
    data,
    current,
    buttonVariant = 'primary',
    compact = false,
    class: columnClass = 'w-56 border-l border-border pl-6',
  }: {
    data: PersonalPageData;
    current: AssetImportView | null;
    /** The sell page demotes the button so Select modules leads. */
    buttonVariant?: 'primary' | 'secondary';
    /** Button only: no separator, caption or status column. */
    compact?: boolean;
    /** The status column's frame; the default is the page-header
     * slot (fixed width behind a hairline separator). */
    class?: string;
  } = $props();

  let starting = $state(false);

  // The completed caption's age ticks between pushes (see elapsedAge).
  const AGE_TICK_MS = 1000;
  let receivedAt = $state(Date.now());
  let now = $state(Date.now());
  $effect(() => {
    void current;
    receivedAt = Date.now();
  });
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), AGE_TICK_MS);
    return () => clearInterval(timer);
  });

  async function startImport() {
    starting = true;
    try {
      await fetch('/personal/modules', { method: 'POST', redirect: 'manual' });
    } finally {
      starting = false;
    }
  }

  const BUTTON_VARIANTS = {
    primary: 'bg-primary text-primary-foreground hover:bg-primary/90 disabled:hover:bg-primary',
    secondary:
      'border border-border bg-card-2 text-foreground hover:brightness-125 disabled:hover:brightness-100',
  };

  const active = $derived(
    current !== null && current.status !== 'completed' && current.status !== 'failed',
  );

  const stepTexts: Record<string, string> = {
    fetching_assets: 'Fetching assets from ESI',
    fetching_asset_names: 'Fetching asset names from ESI',
    fetching_corporation_assets: 'Fetching corporation assets from ESI',
    fetching_corporation_asset_names: 'Fetching corporation asset names from ESI',
    searching_abyssal_modules: 'Searching for abyssal modules',
  };

  // The failed-step wording of the legacy FailedAssetImport.vue.
  const failedActions: Record<string, string> = {
    fetching_assets: 'fetch your assets from ESI',
    fetching_asset_names: 'fetch your asset names from ESI',
    fetching_corporation_assets: 'fetch your corporation assets from ESI',
    fetching_corporation_asset_names: 'fetch your corporation asset names from ESI',
    searching_abyssal_modules: 'search for abyssal modules',
    importing_abyssal_modules: 'import abyssal modules',
  };

  /** Compact age for the one-line caption: 45s, 12m, 3h, 185d. */
  function distanceCompact(seconds: number): string {
    const clamped = Math.max(seconds, 0);
    if (clamped < 60) return `${clamped}s`;
    if (clamped < 3600) return `${Math.floor(clamped / 60)}m`;
    if (clamped < 86_400) return `${Math.floor(clamped / 3600)}h`;
    return `${Math.floor(clamped / 86_400)}d`;
  }

  const progressPercent = $derived(
    current !== null && current.abyssal_modules_count > 0
      ? (current.abyssal_modules_imported_count / current.abyssal_modules_count) * 100
      : 0,
  );

  const statusLine = $derived.by(() => {
    if (!data.has_assets_scope) {
      return 'Grant the "Read Assets" ESI scope to import your modules';
    }
    if (current === null) {
      return 'No assets imported yet';
    }
    switch (current.status) {
      case 'pending':
        return 'Import queued · this may take a few minutes';
      case 'processing':
        return (
          stepTexts[current.step] ??
          `Importing modules ${current.abyssal_modules_imported_count}/${current.abyssal_modules_count}`
        );
      case 'completed':
        return `Imported ${current.abyssal_modules_imported_count} modules · ${distanceCompact(
          elapsedAge(current.updated_seconds_ago, receivedAt, now),
        )}`;
      default:
        return `Import failed while trying to ${failedActions[current.step] ?? 'import your assets'}`;
    }
  });

  const showBar = $derived(
    current !== null && current.status === 'processing' && !stepTexts[current.step],
  );
</script>

{#snippet importButton()}
  {#if !data.has_assets_scope}
    <a
      href={data.grant_scope_url}
      rel="external"
      class="inline-flex h-8 w-full items-center justify-center rounded-md text-sm font-medium transition-colors {BUTTON_VARIANTS[
        buttonVariant
      ]}"
    >
      Grant ESI scope
    </a>
  {:else}
    <!-- fetch, not a form post: a navigation would close hosting
		     dialogs; the /ws stream delivers the started import's state. -->
    <button
      type="button"
      onclick={startImport}
      disabled={active || starting}
      class="relative inline-flex h-8 w-full items-center justify-center gap-2 overflow-hidden rounded-md text-sm font-medium transition-colors {active
        ? 'cursor-default bg-primary/50 text-primary-foreground'
        : BUTTON_VARIANTS[buttonVariant]}"
    >
      {#if showBar}
        <span
          class="absolute inset-y-0 left-0 bg-primary transition-[width] duration-1000"
          style="width: {progressPercent}%"
        ></span>
      {/if}
      <span class="relative inline-flex items-center gap-2">
        {#if active}
          <LoaderCircle class="size-4 animate-spin" />
          Importing…
        {:else}
          Start Import
        {/if}
      </span>
    </button>
  {/if}
{/snippet}

{#if compact}
  <div class="w-36">
    {@render importButton()}
  </div>
{:else}
  <!-- A fixed-slot column separated from the stats by the same
	     hairline: the button with the module progress painted as its own
	     fill, and the caption below — every state fills the same geometry,
	     so the header never resizes. -->
  <div class="flex flex-col gap-1 {columnClass}">
    {@render importButton()}
    <p
      class="hud-label h-3.5 max-w-full truncate normal-case {current?.status === 'pending'
        ? 'animate-pulse'
        : ''}"
      title={statusLine}
    >
      {statusLine}
    </p>
  </div>
{/if}
