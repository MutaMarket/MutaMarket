<script lang="ts">
  // The collection page's manage-modules dialog, the legacy
  // CollectionLocationSettings.vue plus its two grids: the manual
  // location grid with bulk add/sync/remove per location, and the
  // auto-sync tracking grid with checkboxes, behind the mode toggle.
  import { TriangleAlert } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import EnableAutoSyncConfirmDialog from './enable-auto-sync-confirm-dialog.svelte';
  import GameImage from './game-image.svelte';
  import Trans from './trans.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Label } from '$lib/components/ui/label';
  import { Switch } from '$lib/components/ui/switch';
  import {
    nextSort,
    sortLocations,
    withParents,
    type LocationWithParent,
    type SortDirection,
    type SortField,
  } from '$lib/collection-locations';
  import { parseDbTimestamp, relativeTime } from '$lib/duration';
  import { t } from '$lib/i18n.svelte';
  import { notifySuccess } from '$lib/toast';
  import type { CollectionPageData } from '$lib/types-social';

  let { page }: { page: CollectionPageData } = $props();

  let sortField = $state<SortField>('container');
  let sortDirection = $state<SortDirection>('asc');
  let showEnableDialog = $state(false);
  let busy = $state(false);

  const locations = $derived(
    sortLocations(withParents(page.locations ?? []), sortField, sortDirection),
  );
  const trackedIds = $derived(new Set((page.tracked_locations ?? []).map((l) => l.asset_id)));
  const lastSyncedText = $derived(
    page.last_synced_at
      ? relativeTime(parseDbTimestamp(page.last_synced_at) - Date.now() / 1000)
      : null,
  );

  function handleSort(field: SortField) {
    ({ field: sortField, direction: sortDirection } = nextSort(sortField, sortDirection, field));
  }

  async function mutate(
    path: string,
    method: string,
    body: unknown,
    title: string,
    message: string,
  ) {
    busy = true;
    try {
      const response = await fetch(path, {
        method,
        headers: { 'content-type': 'application/json' },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      if (response.ok || response.redirected) {
        // The legacy back()->notify toasts, verbatim.
        notifySuccess(title, message);
        await invalidateAll();
      }
    } finally {
      busy = false;
    }
  }

  function addModules(assetId: number) {
    void mutate(
      '/collection-locations',
      'POST',
      { location_id: assetId, collection_id: page.collection.id },
      t('collections.notifications.modulesAddedTitle'),
      t('collections.notifications.modulesAddedBody'),
    );
  }

  function syncModules(assetId: number) {
    void mutate(
      '/collection-locations',
      'PUT',
      { location_id: assetId, collection_id: page.collection.id },
      t('collections.notifications.syncedTitle'),
      t('collections.notifications.syncedBody'),
    );
  }

  function removeModules(assetId: number) {
    void mutate(
      '/collection-locations',
      'DELETE',
      { location_id: assetId, collection_id: page.collection.id },
      t('collections.notifications.modulesRemovedTitle'),
      t('collections.notifications.modulesRemovedBody'),
    );
  }

  function removeAllModules() {
    void mutate(
      '/collection-modules/all',
      'DELETE',
      { collection_id: page.collection.id },
      t('collections.notifications.modulesRemovedTitle'),
      t('collections.notifications.modulesRemovedBody'),
    );
  }

  function disableAutoSync() {
    void mutate(
      `/collections/${page.collection.slug}/auto-sync`,
      'DELETE',
      undefined,
      t('collections.notifications.autoSyncDisabledTitle'),
      t('collections.notifications.autoSyncDisabledBody'),
    );
  }

  function toggleTracking(assetId: number, checked: boolean) {
    if (checked) {
      void mutate(
        `/collections/${page.collection.slug}/auto-sync/locations`,
        'POST',
        { asset_id: assetId },
        t('collections.notifications.locationAddedTitle'),
        t('collections.notifications.locationAddedBody'),
      );
    } else {
      void mutate(
        `/collections/${page.collection.slug}/auto-sync/locations/${assetId}`,
        'DELETE',
        undefined,
        t('collections.notifications.locationRemovedTitle'),
        t('collections.notifications.locationRemovedBody'),
      );
    }
  }
</script>

{#snippet gridHeader()}
  <div class="col-span-full grid grid-cols-subgrid border-b border-border pb-2 text-sm font-medium">
    {#if page.auto_sync}<div></div>{/if}
    <div></div>
    <button type="button" class="cursor-pointer px-2 text-left" onclick={() => handleSort('name')}>
      {t('collections.locationGrid.location')}
    </button>
    <button type="button" class="cursor-pointer px-2 text-left" onclick={() => handleSort('type')}>
      {t('common.labels.type')}
    </button>
    <button
      type="button"
      class="cursor-pointer px-2 text-left"
      onclick={() => handleSort('station')}
    >
      {t('collections.locationGrid.station')}
    </button>
    <button
      type="button"
      class="cursor-pointer px-2 text-center"
      onclick={() => handleSort('modules')}
    >
      {t('collections.locationGrid.modules')}
    </button>
    {#if !page.auto_sync}<div></div>{/if}
  </div>
{/snippet}

{#snippet gridRow(location: LocationWithParent)}
  <GameImage
    src="https://images.evetech.net/types/{location.type_id}/icon?size=64"
    alt={location.type_name ?? t('common.labels.unknown')}
    class="mx-auto size-8 rounded-md"
  />
  <a href="/locations/{location.slug}" class="truncate px-2 hover:underline">
    {location.name ?? location.type_name}
  </a>
  <span class="truncate px-2 text-sm">{location.type_name}</span>
  <span class="truncate px-2 text-sm">
    {location.parent?.name ?? location.station?.name ?? t('common.labels.unknown')}
  </span>
  <span class="px-2 text-center text-sm">{location.modules_count}</span>
{/snippet}

{#snippet emptyState()}
  <div
    class="col-span-full grid grid-cols-[auto_1fr] items-center border border-t border-border bg-card"
  >
    <div
      class="flex aspect-square w-24 items-center justify-center border-r border-border bg-card text-primary"
    >
      <TriangleAlert class="w-6" />
    </div>
    <div class="p-4">
      <h1 class="text-lg font-medium">{t('collections.locationEmptyState.title')}</h1>
      <p>
        <Trans key="collections.locationEmptyState.body">
          {#snippet link()}<a href="/personal/modules" class="text-primary hover:underline"
              >{t('collections.locationEmptyState.importLink')}</a
            >{/snippet}
        </Trans>
      </p>
    </div>
  </div>
{/snippet}

<Dialog.Root>
  <Dialog.Trigger>
    {#snippet child({ props })}
      <Button {...props}>{t('collections.locationSettings.manageModules')}</Button>
    {/snippet}
  </Dialog.Trigger>

  <Dialog.Content class="max-h-[85vh] max-w-[1600px] overflow-y-auto sm:max-w-[min(95vw,1600px)]">
    <Dialog.Title>
      {t('collections.locationSettings.manageModulesIn', { name: page.collection.name })}
    </Dialog.Title>

    <!-- Mode toggle -->
    <div class="flex items-center justify-between rounded-lg border border-border bg-card-1 p-4">
      <div class="flex items-center gap-3">
        <Switch
          checked={page.auto_sync}
          disabled={busy}
          onCheckedChange={(checked) => (checked ? (showEnableDialog = true) : disableAutoSync())}
        />
        <Label class="cursor-pointer">
          <span class="font-medium">{t('collections.autoSync.mode')}</span>
          <span class="ml-2 text-sm text-muted-foreground">
            {page.auto_sync ? t('collections.autoSync.modeOn') : t('collections.autoSync.modeOff')}
          </span>
        </Label>
      </div>
      {#if page.auto_sync && lastSyncedText}
        <div class="text-sm text-muted-foreground">
          {t('collections.autoSync.lastSynced', { time: lastSyncedText })}
        </div>
      {/if}
    </div>

    <Dialog.Description class="max-w-lg">
      {#if page.auto_sync}
        {t('collections.autoSync.autoDescription')}
      {:else}
        {t('collections.autoSync.manualDescription')}
      {/if}
    </Dialog.Description>

    <div class="max-w-full overflow-x-auto">
      {#if page.auto_sync}
        <div class="grid grid-cols-[40px_50px_1fr_1fr_1fr_80px] gap-x-2 p-4">
          {@render gridHeader()}
          {#each locations as location (location.item_id)}
            <div
              class="col-span-full grid grid-cols-subgrid items-center border-b border-border py-2 last:border-b-0 hover:bg-muted/50"
            >
              <div class="flex justify-center">
                <Checkbox
                  checked={trackedIds.has(location.asset_id)}
                  disabled={busy}
                  onCheckedChange={(checked) => toggleTracking(location.asset_id, !!checked)}
                />
              </div>
              {@render gridRow(location)}
            </div>
          {/each}
          {#if locations.length === 0}
            {@render emptyState()}
          {/if}
        </div>
      {:else}
        <div class="grid grid-cols-[50px_1fr_1fr_1fr_80px_auto] gap-x-2 p-4">
          {@render gridHeader()}
          {#each locations as location (location.item_id)}
            <div
              class="col-span-full grid grid-cols-subgrid items-center border-b border-border py-2 last:border-b-0 hover:bg-muted/50"
            >
              {@render gridRow(location)}
              <div class="flex justify-end gap-1">
                <Button size="xs" disabled={busy} onclick={() => addModules(location.asset_id)}>
                  {t('common.actions.add')}
                </Button>
                <Button size="xs" disabled={busy} onclick={() => syncModules(location.asset_id)}>
                  {t('collections.locationGrid.sync')}
                </Button>
                <Button size="xs" disabled={busy} onclick={() => removeModules(location.asset_id)}>
                  {t('common.actions.remove')}
                </Button>
              </div>
            </div>
          {/each}
          {#if locations.length === 0}
            {@render emptyState()}
          {/if}
        </div>
      {/if}
    </div>

    <Dialog.Footer>
      {#if !page.auto_sync}
        <Button class="mr-auto" variant="destructive" disabled={busy} onclick={removeAllModules}>
          {t('collections.locationSettings.removeAllModules')}
        </Button>
      {/if}
      <Dialog.Close>
        {#snippet child({ props })}
          <Button {...props} variant="ghost">{t('common.actions.close')}</Button>
        {/snippet}
      </Dialog.Close>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<EnableAutoSyncConfirmDialog bind:open={showEnableDialog} slug={page.collection.slug} />
