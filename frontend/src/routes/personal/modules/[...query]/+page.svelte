<script lang="ts">
  // The personal modules page (legacy ShowAllPersonalModulesPage): the
  // filter band with the fitted/asset chips, the asset import panel and
  // the owned-module grid with locations.
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { importRefreshGate, subscribeAssetImport } from '$lib/asset-import-stream';
  import AssetImportStatus from '$lib/components/asset-import-status.svelte';
  import FilterBand from '$lib/components/filter-band.svelte';
  import ModuleDisplay from '$lib/components/module-display.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { toIskCompact } from '$lib/format-number';
  import { parseQueryUi } from '$lib/query';
  import type { AssetImportView, PersonalModuleEntry } from '$lib/types';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  const settings = $state({ ...data.displaySettings });
  const search = $derived(parseQueryUi(data.query));
  const activeCharacter = $derived(
    data.nav?.characters.find((character) => character.active) ?? null,
  );

  // The live import state, shared by the header button and the panel
  // (the legacy AssetImportUpdated event on the user's channel,
  // replacing 2-second polling). While the import runs, freshly found
  // modules stream into the grid via throttled refetches.
  let currentImport = $state<AssetImportView | null>(null);
  let entries = $state<PersonalModuleEntry[]>([]);
  $effect(() => {
    currentImport = data.personal.asset_import;
  });
  $effect(() => {
    entries = data.entries;
  });

  async function refreshEntries() {
    const response = await fetch(`/api/personal/modules?q=${encodeURIComponent(data.query)}`);
    if (response.ok) {
      entries = await response.json();
    }
  }

  // One socket per mount: reading `data` here would tear the socket
  // down and rebuild it on every navigation and every invalidation,
  // and each new socket opens with a fresh snapshot.
  $effect(() => {
    const gate = importRefreshGate();
    return subscribeAssetImport(
      untrack(() => data.personal.user_id),
      (view) => {
        currentImport = view;
        const verdict = gate(view);
        if (verdict === 'stream') {
          void refreshEntries();
        } else if (verdict === 'completed') {
          void refreshEntries();
          void invalidateAll();
        }
      },
    );
  });
</script>

<PageMeta
  title={data.panel ? `Your ${data.panel.type_name}` : 'Your Modules'}
  description="Find the perfect abyssal module for your needs on MutaMarket, the best place to buy and sell abyssal modules!"
  keywords="contracts, public, search, find"
/>

<PageHeader
  title="Your Modules"
  subtitle={activeCharacter ? `Acting as ${activeCharacter.name}` : null}
  stats={[
    {
      label: 'Owned',
      value: data.personal.modules_count.toLocaleString('en-US'),
      accent: 'primary',
    },
    { label: 'Est. value', value: toIskCompact(data.personal.estimated_value_total) },
  ]}
>
  {#snippet icon()}
    {#if activeCharacter}
      <img
        alt=""
        class="size-10 rounded-lg"
        src="https://images.evetech.net/characters/{activeCharacter.id}/portrait?size=64"
      />
    {/if}
  {/snippet}
  {#snippet actions()}
    <AssetImportStatus data={data.personal} current={currentImport} />
  {/snippet}
</PageHeader>
<FilterBand
  prefix="personal/modules"
  {search}
  panel={data.panel}
  unknownType={data.unknownType}
  variant="personal"
/>
<div class="my-4 w-full">
  <ModuleDisplay {entries} {settings} panel={data.panel} {search} prefix="personal/modules" />
</div>
