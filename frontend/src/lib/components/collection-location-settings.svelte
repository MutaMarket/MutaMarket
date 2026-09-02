<script lang="ts">
  // The collection page's manage-modules dialog, the legacy
  // CollectionLocationSettings.vue plus its two grids: the manual
  // location grid with bulk add/sync/remove per location, and the
  // auto-sync tracking grid with checkboxes, behind the mode toggle.
  import { TriangleAlert } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import EnableAutoSyncConfirmDialog from './enable-auto-sync-confirm-dialog.svelte';
  import GameImage from './game-image.svelte';
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
      'Modules added to collection',
      'You have successfully added modules to the collection',
    );
  }

  function syncModules(assetId: number) {
    void mutate(
      '/collection-locations',
      'PUT',
      { location_id: assetId, collection_id: page.collection.id },
      'Synced collection location',
      'You have successfully synced the collection location',
    );
  }

  function removeModules(assetId: number) {
    void mutate(
      '/collection-locations',
      'DELETE',
      { location_id: assetId, collection_id: page.collection.id },
      'Modules removed from collection',
      'You have successfully removed modules from the collection',
    );
  }

  function removeAllModules() {
    void mutate(
      '/collection-modules/all',
      'DELETE',
      { collection_id: page.collection.id },
      'Modules removed from collection',
      'You have successfully removed modules from the collection',
    );
  }

  function disableAutoSync() {
    void mutate(
      `/collections/${page.collection.slug}/auto-sync`,
      'DELETE',
      undefined,
      'Auto-sync disabled',
      'This collection will no longer automatically sync. Current modules have been kept.',
    );
  }

  function toggleTracking(assetId: number, checked: boolean) {
    if (checked) {
      void mutate(
        `/collections/${page.collection.slug}/auto-sync/locations`,
        'POST',
        { asset_id: assetId },
        'Location added',
        'The location has been added to auto-sync tracking.',
      );
    } else {
      void mutate(
        `/collections/${page.collection.slug}/auto-sync/locations/${assetId}`,
        'DELETE',
        undefined,
        'Location removed',
        'The location has been removed from auto-sync tracking.',
      );
    }
  }
</script>

{#snippet gridHeader()}
  <div class="col-span-full grid grid-cols-subgrid border-b border-border pb-2 text-sm font-medium">
    {#if page.auto_sync}<div></div>{/if}
    <div></div>
    <button type="button" class="cursor-pointer px-2 text-left" onclick={() => handleSort('name')}>
      Location
    </button>
    <button type="button" class="cursor-pointer px-2 text-left" onclick={() => handleSort('type')}>
      Type
    </button>
    <button
      type="button"
      class="cursor-pointer px-2 text-left"
      onclick={() => handleSort('station')}
    >
      Station
    </button>
    <button
      type="button"
      class="cursor-pointer px-2 text-center"
      onclick={() => handleSort('modules')}
    >
      Modules
    </button>
    {#if !page.auto_sync}<div></div>{/if}
  </div>
{/snippet}

{#snippet gridRow(location: LocationWithParent)}
  <GameImage
    src="https://images.evetech.net/types/{location.type_id}/icon?size=64"
    alt={location.type_name ?? 'Unknown'}
    class="mx-auto size-8 rounded-md"
  />
  <a href="/locations/{location.slug}" class="truncate px-2 hover:underline">
    {location.name ?? location.type_name}
  </a>
  <span class="truncate px-2 text-sm">{location.type_name}</span>
  <span class="truncate px-2 text-sm">
    {location.parent?.name ?? location.station?.name ?? 'Unknown'}
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
      <h1 class="text-lg font-medium">No locations found</h1>
      <p>
        You don't have any locations yet. Start by
        <a href="/personal/modules" class="text-primary hover:underline">importing your assets</a>.
      </p>
    </div>
  </div>
{/snippet}

<Dialog.Root>
  <Dialog.Trigger>
    {#snippet child({ props })}
      <Button {...props}>Manage modules</Button>
    {/snippet}
  </Dialog.Trigger>

  <Dialog.Content class="max-h-[85vh] max-w-[1600px] overflow-y-auto sm:max-w-[min(95vw,1600px)]">
    <Dialog.Title>Manage modules in {page.collection.name}</Dialog.Title>

    <!-- Mode toggle -->
    <div class="flex items-center justify-between rounded-lg border border-border bg-card-1 p-4">
      <div class="flex items-center gap-3">
        <Switch
          checked={page.auto_sync}
          disabled={busy}
          onCheckedChange={(checked) => (checked ? (showEnableDialog = true) : disableAutoSync())}
        />
        <Label class="cursor-pointer">
          <span class="font-medium">Auto-Sync Mode</span>
          <span class="ml-2 text-sm text-muted-foreground">
            {page.auto_sync
              ? 'Collection syncs automatically with selected locations'
              : 'Manually manage modules in this collection'}
          </span>
        </Label>
      </div>
      {#if page.auto_sync && lastSyncedText}
        <div class="text-sm text-muted-foreground">Last synced {lastSyncedText}</div>
      {/if}
    </div>

    <Dialog.Description class="max-w-lg">
      {#if page.auto_sync}
        Select which locations to track. The collection will automatically sync with these locations
        whenever your assets are imported.
      {:else}
        Due to security reasons you can only synchronize the visibility of modules in containers or
        ships. You can't toggle the visibility of stations. We recommend you to use station
        containers to manage your modules. Only containers that contain abyssal modules are shown
        here.
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
                  Add
                </Button>
                <Button size="xs" disabled={busy} onclick={() => syncModules(location.asset_id)}>
                  Sync
                </Button>
                <Button size="xs" disabled={busy} onclick={() => removeModules(location.asset_id)}>
                  Remove
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
          Remove all modules
        </Button>
      {/if}
      <Dialog.Close>
        {#snippet child({ props })}
          <Button {...props} variant="ghost">Close</Button>
        {/snippet}
      </Dialog.Close>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<EnableAutoSyncConfirmDialog bind:open={showEnableDialog} slug={page.collection.slug} />
