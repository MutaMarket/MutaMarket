<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  // The sell page, the legacy ShowSellModulesPage: your published
  // modules under the full filter grammar, shaped like My Modules —
  // with the import status in the header plus the select-modules
  // dialog for publishing containers.
  import { Coins, PackagePlus } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { importRefreshGate, subscribeAssetImport } from '$lib/asset-import-stream';
  import FilterBand from '$lib/components/filter-band.svelte';
  import ModuleDisplay from '$lib/components/module-display.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { countStat, scopedModuleStats } from '$lib/module-stats';
  import SelectModulesDialog from '$lib/components/select-modules-dialog.svelte';
  import { Button } from '$lib/components/ui/button';
  import { t } from '$lib/i18n.svelte';
  import { editSession, startEdit } from '$lib/module-edits';
  import { parseQueryUi } from '$lib/query';
  import type { AssetImportView, ModuleDetail } from '$lib/types';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();
  const search = $derived(parseQueryUi(data.query));

  let selecting = $state(false);

  // The live import state, shared with the personal page pattern (the
  // AssetImportUpdated event on the user's channel). Modules found for
  // already-published containers stream in while the import runs.
  let currentImport = $state<AssetImportView | null>(null);
  let modules = $state<ModuleDetail[]>([]);
  $effect(() => {
    currentImport = data.personal.asset_import;
  });
  $effect(() => {
    modules = data.modules;
  });

  async function refreshModules() {
    const response = await fetch(`/api/sell/modules?q=${encodeURIComponent(data.query)}`);
    if (response.ok) {
      modules = await response.json();
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
          void refreshModules();
        } else if (verdict === 'completed') {
          void refreshModules();
          void invalidateAll();
        }
      },
    );
  });
</script>

<PageMeta
  title={t('meta.sell.title')}
  description={t('meta.sell.description')}
  keywords="contracts, public, search, find"
/>

<PageHeader
  title={t('modules.sellPage.title')}
  subtitle={t('modules.sellPage.subtitle')}
  stats={scopedModuleStats(
    [countStat(t('modules.sellPage.published'), data.sell.stats.total_count, 'primary')],
    data.sell.stats,
  )}
>
  {#snippet icon()}
    <img
      alt=""
      class="size-10 rounded-lg"
      src="https://images.evetech.net/characters/{data.sell.character_id}/portrait?size=64"
    />
  {/snippet}
  {#snippet actions()}
    <Button
      class="h-8 gap-2"
      variant="secondary"
      disabled={$editSession !== null}
      onclick={() => startEdit('price')}
    >
      <Coins class="size-4" />
      {t('forms.sellFilters.editAskingPrices')}
    </Button>
    <Button class="h-8 gap-2" onclick={() => (selecting = true)}>
      <PackagePlus class="size-4" />
      {t('forms.sellFilters.selectModules')}
    </Button>
  {/snippet}
</PageHeader>
<FilterBand
  prefix="sell/modules"
  {search}
  panel={data.panel}
  unknownType={data.unknownType}
  variant="sell"
/>
<div class="my-4 w-full">
  <ModuleDisplay
    entries={modules.map((module) => ({ module }))}
    {settings}
    panel={data.panel}
    {search}
    prefix="sell/modules"
  />
</div>

<SelectModulesDialog bind:open={selecting} personal={data.personal} current={currentImport} />
