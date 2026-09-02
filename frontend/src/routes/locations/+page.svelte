<script lang="ts">
  // The asset-locations tree, the legacy ShowLocationsPage: stations
  // and structures rooting the ships and containers that hold abyssal
  // modules, with rolled-up counts and a name/type search.
  import { ChevronDown, ChevronUp, MapPin } from '@lucide/svelte';
  import GameImage from '$lib/components/game-image.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { Input } from '$lib/components/ui/input';
  import Trans from '$lib/components/trans.svelte';
  import { t } from '$lib/i18n.svelte';
  import { buildTree, filterTree, type TreeNode } from '$lib/location-tree';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  let query = $state('');
  const tree = $derived(filterTree(buildTree(data.tree), query));

  // Collapsed node ids; everything starts open like the legacy tree.
  let collapsed = $state<Set<number>>(new Set());
  function toggle(id: number) {
    const next = new Set(collapsed);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    collapsed = next;
  }
</script>

<PageMeta
  title={t('meta.locations.title')}
  description={t('meta.locations.description')}
  keywords="contracts, public, search, find"
/>

<PageHeader title={t('misc.locations.title')} subtitle={t('misc.locations.subtitle')} />

<div class="mb-4">
  <Input
    type="search"
    placeholder={t('misc.locations.searchPlaceholder')}
    class="h-10 w-full bg-card-2 dark:bg-card-2"
    bind:value={query}
  />
</div>

{#snippet node(location: TreeNode, depth: number)}
  <div class="col-span-full grid grid-cols-subgrid border-b border-border last:border-b-0">
    <button
      type="button"
      class="col-span-5 grid w-full grid-cols-subgrid items-center gap-4 p-2 text-left transition-colors hover:bg-card"
      onclick={() => toggle(location.id)}
    >
      <div style="margin-left: {depth * 1.5}rem">
        <GameImage
          src="https://images.evetech.net/types/{location.type_id ?? 0}/icon?size=64"
          alt={location.name}
          class="size-8 rounded-md"
        />
      </div>
      <a
        href="/locations/{location.slug}"
        class="justify-self-start hover:underline"
        onclick={(event) => event.stopPropagation()}
      >
        {location.name}
      </a>
      <span class="text-muted-foreground">{location.type}</span>
      <span class="pr-4 text-right font-mono tabular-nums">{location.count}</span>
      <span class="justify-self-center text-muted-foreground">
        {#if location.children.length > 0}
          {#if collapsed.has(location.id)}
            <ChevronDown class="size-4" />
          {:else}
            <ChevronUp class="size-4" />
          {/if}
        {/if}
      </span>
    </button>
    {#if location.children.length > 0 && !collapsed.has(location.id)}
      {#each location.children as child (child.id)}
        {@render node(child, depth + 1)}
      {/each}
    {/if}
  </div>
{/snippet}

{#if tree.length > 0}
  <div class="hud-frame mb-4 grid grid-cols-[auto_1fr_1fr_auto_auto]">
    {#each tree as station (station.id)}
      {@render node(station, 0)}
    {/each}
  </div>
{:else}
  <div class="hud-frame mb-4 flex items-center justify-center gap-4 p-10">
    <MapPin class="size-6 text-muted-foreground" />
    <span class="text-muted-foreground">
      {#if query.trim() === ''}
        <Trans key="misc.locations.noneFound.body">
          {#snippet link()}
            <a href="/personal/modules" class="text-primary hover:underline">
              {t('misc.locations.noneFound.assetsPage')}
            </a>
          {/snippet}
        </Trans>
      {:else}
        {t('misc.locations.noneMatch')}
      {/if}
    </span>
  </div>
{/if}
