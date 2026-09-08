<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  // A collection's modules with the filter band, mirroring the legacy
  // ShowCollectionPage's filter set (general, misc, value, attributes).
  // Owners (the API sends them the locations payload) additionally get
  // the legacy PageActions area: edit, delete and the manage-modules
  // dialog.
  import { goto } from '$app/navigation';
  import CollectionLocationSettings from '$lib/components/collection-location-settings.svelte';
  import EditCollectionDialog from '$lib/components/edit-collection-dialog.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import FilterBand from '$lib/components/filter-band.svelte';
  import ModuleDisplay from '$lib/components/module-display.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { countStat, scopedModuleStats } from '$lib/module-stats';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { collectionOgImage } from '$lib/meta';
  import { Layers } from '@lucide/svelte';
  import { t } from '$lib/i18n.svelte';
  import { openCollection } from '$lib/module-edits';
  import { parseQueryUi } from '$lib/query';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();
  const search = $derived(parseQueryUi(data.query));
  const prefix = $derived(`collections/${data.page.collection.slug}`);
  const isOwner = $derived(data.page.locations !== null);

  let editing = $state(false);
  let confirmingDelete = $state(false);
  let deleting = $state(false);

  async function destroy() {
    deleting = true;
    try {
      await fetch(`/collections/${data.page.collection.slug}`, {
        method: 'DELETE',
        redirect: 'manual',
      });
      confirmingDelete = false;
      await goto('/collections');
    } finally {
      deleting = false;
    }
  }

  // Collection notes only exist inside a collection, so the module
  // menus need to know which one is open (the legacy page.props
  // .collection lookup).
  $effect(() => {
    openCollection.set({
      id: data.page.collection.id,
      characterId: data.page.collection.character_id,
    });
    return () => openCollection.set(null);
  });
</script>

<PageMeta
  title={data.page.collection.name}
  description={data.page.collection.description ||
    t('meta.collection.description', { name: data.page.collection.name })}
  image={collectionOgImage(data.page.collection.id)}
  keywords={[data.page.collection.name, 'collection', 'modules']}
/>

<PageHeader
  banner={false}
  title={data.page.collection.name}
  subtitle={`${t('collections.show.createdBy')} ${data.page.collection.character_name}${
    data.page.collection.description ? ` · ${data.page.collection.description}` : ''
  }`}
  stats={scopedModuleStats(
    [countStat(t('stats.overview.modules'), data.page.stats.total_count, 'primary')],
    data.page.stats,
  )}
>
  {#snippet icon()}
    <div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
      <Layers class="size-5 text-primary" stroke-width={1.5} />
    </div>
  {/snippet}
</PageHeader>
{#if isOwner}
  <div class="mb-4 flex flex-wrap justify-end gap-2">
    <Button onclick={() => (editing = true)}>
      {t('collections.show.editCollection')}
    </Button>
    <Button variant="outline" onclick={() => (confirmingDelete = true)}>
      {t('collections.show.deleteCollection')}
    </Button>
    <CollectionLocationSettings page={data.page} />
  </div>
  <EditCollectionDialog bind:open={editing} collection={data.page.collection} />
  <Dialog.Root bind:open={confirmingDelete}>
    <Dialog.Content>
      <Dialog.Title>{t('collections.dialogs.deleteTitle')}</Dialog.Title>
      <Dialog.Description>{t('collections.dialogs.deleteBody')}</Dialog.Description>
      <Dialog.Footer>
        <Button variant="secondary" onclick={() => (confirmingDelete = false)}>
          {t('common.actions.cancel')}
        </Button>
        <Button variant="destructive" disabled={deleting} onclick={destroy}>
          {t('common.actions.delete')}
        </Button>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>
{/if}
<FilterBand
  {prefix}
  {search}
  panel={data.panel}
  unknownType={data.unknownType}
  variant="collection"
/>
<div class="my-4 w-full">
  <ModuleDisplay
    entries={data.page.modules.map((module) => ({ module }))}
    {settings}
    panel={data.panel}
    {search}
    {prefix}
  />
</div>
