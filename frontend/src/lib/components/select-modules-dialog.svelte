<script lang="ts">
  // The sell page's select-modules dialog, the legacy
  // PublicLocationSettings.vue grown into the whole selling flow: the
  // asset import (button plus last-import status, unconstrained by the
  // header) on top, then the active character's containers with
  // publish switches driving the ported /public-assets endpoints.
  // Every toggle reports how many modules it (un)published.
  import { TriangleAlert } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import AssetImportStatus from './asset-import-status.svelte';
  import GameImage from './game-image.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Switch } from '$lib/components/ui/switch';
  import Trans from './trans.svelte';
  import { t } from '$lib/i18n.svelte';
  import { locationFlagLabel } from '$lib/location-flags';
  import { notifySuccess } from '$lib/toast';
  import type { AssetImportView, PersonalPageData, SellLocation } from '$lib/types';

  let {
    open = $bindable(false),
    personal,
    current,
  }: {
    open?: boolean;
    personal: PersonalPageData;
    current: AssetImportView | null;
  } = $props();

  let locations = $state<SellLocation[] | null>(null);
  let busy = $state<number | null>(null);

  $effect(() => {
    if (open && locations === null) {
      void refresh();
    }
  });

  // A finished import changes the containers; refresh the open list.
  $effect(() => {
    if (current?.status === 'completed' && open) {
      void refresh();
    }
  });

  /** Containers first (the legacy couldBeContainer name check), ships
   * and everything else below, each group alphabetical. */
  function sorted(list: SellLocation[]): SellLocation[] {
    const isContainer = (location: SellLocation) =>
      location.type_name.toLowerCase().includes('container');
    return [...list].sort((a, b) => {
      const containers = Number(isContainer(b)) - Number(isContainer(a));
      if (containers !== 0) return containers;
      return (a.name || a.type_name).localeCompare(b.name || b.type_name);
    });
  }

  async function refresh() {
    const response = await fetch('/api/sell/locations');
    if (response.ok) {
      locations = sorted(await response.json());
    }
  }

  function count(location: SellLocation): string {
    return t('misc.publicLocationSettings.moduleCount', { count: location.abyssal_count });
  }

  function containerName(location: SellLocation): string {
    return location.name || t('misc.publicLocationSettings.theContainer');
  }

  async function toggle(location: SellLocation, publish: boolean) {
    busy = location.asset_id;
    try {
      if (publish) {
        await fetch('/public-assets', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ asset_id: location.asset_id }),
          redirect: 'manual',
        });
        notifySuccess(
          t('misc.publicLocationSettings.publishedTitle'),
          t('misc.publicLocationSettings.publishedBody', {
            modules: count(location),
            name: containerName(location),
          }),
        );
      } else if (location.public_asset_id !== null) {
        await fetch(`/public-assets/${location.public_asset_id}`, {
          method: 'DELETE',
          redirect: 'manual',
        });
        notifySuccess(
          t('misc.publicLocationSettings.unpublishedTitle'),
          t('misc.publicLocationSettings.unpublishedBody', {
            modules: count(location),
            name: containerName(location),
          }),
        );
      }
      await refresh();
      await invalidateAll();
    } finally {
      busy = null;
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="gap-0 overflow-x-hidden p-0 sm:max-w-2xl">
    <div class="border-b border-border p-5">
      <Dialog.Title>{t('misc.publicLocationSettings.title')}</Dialog.Title>
      <Dialog.Description class="mt-1">
        {t('misc.publicLocationSettings.description')}
      </Dialog.Description>
      <div class="mt-4 rounded-lg border border-border bg-card-1 p-3">
        <AssetImportStatus data={personal} {current} class="w-full" />
      </div>
      <div
        class="mt-3 flex items-start gap-2.5 rounded-lg border border-yellow-500/20 bg-yellow-500/5 p-3"
      >
        <TriangleAlert class="mt-0.5 size-4 shrink-0 text-yellow-500" />
        <p class="text-xs text-muted-foreground">
          {t('misc.publicLocationSettings.containerWarning')}
        </p>
      </div>
    </div>
    <div class="p-5">
      {#if locations === null}
        <p class="py-2 text-sm text-muted-foreground">{t('common.actions.loading')}</p>
      {:else if locations.length === 0}
        <p class="py-2 text-sm text-muted-foreground">
          <Trans key="misc.publicLocationSettings.empty.body">
            {#snippet link()}
              <a href="/personal/modules" class="text-primary hover:underline">
                {t('misc.publicLocationSettings.empty.importLink')}
              </a>
            {/snippet}
          </Trans>
        </p>
      {:else}
        <ul class="flex max-h-[50vh] flex-col gap-1 overflow-x-hidden overflow-y-auto pr-1">
          {#each locations as location (location.asset_id)}
            <li class="flex items-center gap-3 rounded-md px-2 py-2 hover:bg-card-2">
              <GameImage
                src="https://images.evetech.net/types/{location.type_id}/icon?size=64"
                alt=""
                class="size-9 rounded"
              />
              <div class="min-w-0 grow">
                <span class="block truncate text-sm">
                  {location.name || t('misc.publicLocationSettings.unnamedContainer')}
                </span>
                <span class="block truncate text-xs text-muted-foreground">
                  {locationFlagLabel(location.location_flag)} · {count(location)}
                  {#if location.station_name}
                    · {location.station_name}
                  {/if}
                </span>
              </div>
              <Switch
                checked={location.public_asset_id !== null}
                disabled={busy === location.asset_id}
                onCheckedChange={(on) => toggle(location, on)}
              />
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    <Dialog.Footer class="border-t border-border p-4">
      <Button variant="secondary" onclick={() => (open = false)}>{t('common.actions.close')}</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
