<script lang="ts">
  // The module card mirroring the legacy Grid/Module.vue tree
  //: meta-group accent header with the local
  // abyssal icon, per-attribute rows, and exactly one location row —
  // Contract when for sale, the owner's Asset, the seller's
  // PublicAsset (make-offer entry) — else the EstimatedValue fallback,
  // then the note / collection-note / asking-price rows.
  import { ArrowLeftRight, Cpu, EllipsisVertical, Gavel, Sparkles } from '@lucide/svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { openMakeOffer, sentOfferId, sentOffers } from '$lib/make-offer';
  import AttributeRow from './attribute-row.svelte';
  import GameImage from './game-image.svelte';
  import FindAssetCard from './find-asset-card.svelte';
  import ModuleEditRow from './module-edit-row.svelte';
  import ModuleMenuItems from './module-menu-items.svelte';
  import { isVisual, metaGroupKey } from '$lib/attributes';
  import { Button } from '$lib/components/ui/button';
  import * as ContextMenu from '$lib/components/ui/context-menu';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import * as HoverCard from '$lib/components/ui/hover-card';
  import type { DisplaySettings } from '$lib/display';
  import { parseDbTimestamp, relativeTime } from '$lib/duration';
  import { toIskCompact } from '$lib/format-number';
  import { locationFlagLabel } from '$lib/location-flags';
  import {
    canEditCollectionNote,
    canSetPrice,
    navCharacterIds,
    editSession,
    openCollection,
    showsEditRow,
  } from '$lib/module-edits';
  import type { AbyssalTypeStatistic, ModuleDetail } from '$lib/types';

  let {
    module,
    settings,
    statistics = null,
  }: {
    module: ModuleDetail;
    settings: DisplaySettings;
    /** Roll extremes for the search menus; fetched lazily when null. */
    statistics?: AbyssalTypeStatistic[] | null;
  } = $props();

  // "2 d ago" for the training row's sale timestamp.
  const soldAgo = $derived.by(() => {
    const soldAt = module.training_module?.sold_at;
    if (!soldAt) {
      return '';
    }
    return relativeTime(parseDbTimestamp(soldAt) - Date.now() / 1000);
  });

  const headerBorder = $derived.by(() => {
    switch (metaGroupKey(module.source_type?.meta_group_id ?? null)) {
      case 't2':
        return 'border-b-orange-500';
      case 'storyline':
        return 'border-b-green-300';
      case 'faction':
        return 'border-b-green-500';
      case 'officer':
        return 'border-b-purple-500';
      case 'deadspace':
        return 'border-b-blue-500';
      default:
        return 'border-b-gray-500';
    }
  });

  // Legacy loads the signed-in user's asset on every module, so "where
  // is mine" answers on the market and every other list, not just the
  // personal pages.
  const asset = $derived(module.asset ?? null);

  const visualAttributes = $derived(module.mutated_attributes.filter(isVisual));

  const characterIds = $derived(navCharacterIds(page.data.nav));
  const canNote = $derived(page.data.nav?.user != null);
  const canCollectionNote = $derived(canEditCollectionNote($openCollection, characterIds));
  const canPrice = $derived(canSetPrice(module, characterIds));

  // Masonry alignment like the legacy getRowSpan: header + exactly one
  // location row + one per visual attribute + whichever of the note,
  // collection-note and asking-price rows are showing.
  const rowSpan = $derived(
    2 +
      visualAttributes.length +
      Number(showsEditRow('note', module, $editSession, canNote)) +
      Number(showsEditRow('collection-note', module, $editSession, canCollectionNote)) +
      Number(showsEditRow('price', module, $editSession, canPrice)),
  );

  // "est. 142 million ISK" / "No estimate available" (legacy card copy).
  const estimateLine = $derived(
    module.estimated_value !== null
      ? `est. ${toIskCompact(module.estimated_value)}`
      : 'No estimate available',
  );
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger>
    {#snippet child({ props })}
      <div
        {...props}
        class="grid overflow-hidden rounded-lg border border-border *:first:rounded-t-lg *:last:rounded-b-lg"
        style="grid-row: span {rowSpan}"
      >
        <div
          class="relative grid h-[50px] grid-cols-[36px_1fr_auto] content-center items-center gap-x-2 border-b-2 bg-card-1 p-2 {headerBorder}"
        >
          <GameImage
            src="/img/icons/{module.type.id}.png"
            alt={module.type.name}
            class="row-span-2 size-8 rounded-lg"
          />
          <a
            class="col-start-2 row-start-1 truncate text-sm text-foreground"
            href="/modules/{module.slug}"
          >
            {module.source_type?.name ?? module.type.name}
            <span aria-hidden="true" class="absolute inset-0"></span>
          </a>
          <span class="col-start-2 row-start-2 mt-1 truncate text-xs text-muted-foreground">
            {module.mutaplasmid?.name ?? ''}
          </span>
          <DropdownMenu.Root>
            <DropdownMenu.Trigger>
              {#snippet child({ props: triggerProps })}
                <span {...triggerProps} class="relative col-start-3 row-span-2 row-start-1">
                  <Button variant="ghost" size="icon" class="cursor-pointer">
                    <EllipsisVertical class="size-4" />
                  </Button>
                </span>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content align="start" side="right" class="w-60 rounded-lg border">
              <ModuleMenuItems {module} {statistics} kind="dropdown" />
            </DropdownMenu.Content>
          </DropdownMenu.Root>
        </div>

        {#each visualAttributes as attribute (attribute.id)}
          <AttributeRow {attribute} {settings} />
        {/each}

        {#if module.training_module}
          <!-- The legacy Grid/Training.vue: what the roll actually sold for. -->
          <a
            href="/modules/{module.slug}"
            class="grid h-[50px] grid-cols-[36px_1fr] items-center bg-card px-2"
          >
            <div class="grid place-items-center text-green-500">
              <Sparkles stroke-width={1} class="h-[1em] w-[1em]" />
            </div>
            <div class="grid text-right">
              <span>{toIskCompact(module.training_module.sold_for)}</span>
              <span class="text-sm leading-4 text-muted-foreground">
                {estimateLine} | {soldAgo}
              </span>
            </div>
          </a>
        {:else if module.contract}
          <!-- The legacy Grid/Contract.vue: sale type icon and price. -->
          <a
            href="/modules/{module.slug}"
            class="grid h-[50px] grid-cols-[36px_1fr] items-center bg-card px-2"
          >
            <div class="relative grid place-items-center text-amber-500">
              {#if module.contract.type === 'item_exchange'}
                <ArrowLeftRight stroke-width={1} class="h-[1em] w-[1em]" />
              {:else}
                <Gavel stroke-width={1} class="h-[1em] w-[1em]" />
              {/if}
              {#if module.contract.abyssal_modules_count > 1}
                <span class="absolute top-1/2 left-full -translate-y-1/2 text-xs">
                  +{module.contract.abyssal_modules_count - 1}
                </span>
              {/if}
            </div>
            <div class="grid text-right">
              <span>{toIskCompact(module.contract.price)}</span>
              <span class="text-sm leading-4 text-muted-foreground">{estimateLine}</span>
            </div>
          </a>
        {:else if asset}
          <!-- The legacy Grid/Asset.vue: where the owner's module sits,
               with the find-asset card on hover. -->
          <HoverCard.Root openDelay={300}>
            <HoverCard.Trigger>
              {#snippet child({ props: triggerProps })}
                <a
                  {...triggerProps}
                  class="grid grid-cols-[36px_1fr_auto] items-center gap-2 bg-card p-2"
                  href="/locations/{asset.parent_slug}"
                >
                  {#if asset.parent_type_id !== null}
                    <GameImage
                      src="https://images.evetech.net/types/{asset.parent_type_id}/icon?size=64"
                      alt={asset.parent_name}
                      class="size-9 rounded-lg"
                    />
                  {:else}
                    <span></span>
                  {/if}
                  <div class="overflow-hidden py-[3px] text-xs">
                    <span class="block truncate font-medium">{asset.parent_name}</span>
                    <span class="block truncate text-muted-foreground">
                      {locationFlagLabel(asset.location_flag)} | Est. {toIskCompact(
                        module.estimated_value,
                      )}
                    </span>
                  </div>
                  <div class="pr-2 pl-4 font-medium">{asset.location_index + 1}</div>
                </a>
              {/snippet}
            </HoverCard.Trigger>
            <FindAssetCard {module} {asset} />
          </HoverCard.Root>
        {:else if module.public_asset}
          <!-- The legacy Grid/PublicAsset.vue: the seller, with the price
		     cell doubling as the make-offer button (or the jump into an
		     already-running thread). -->
          {@const myOffer = sentOfferId($sentOffers, page.data.sentOffers, module.id)}
          <div class="relative grid h-[50px] grid-cols-[36px_1fr] items-center bg-card px-2">
            <img
              alt={module.public_asset.owner.name}
              class="size-8 rounded-lg"
              src="https://images.evetech.net/characters/{module.public_asset.owner
                .id}/portrait?size=64"
            />
            {#if myOffer !== undefined}
              <a class="grid text-right" href="/offers/{myOffer}">
                <span>Go to offer</span>
                <span class="absolute inset-0"></span>
                <span class="text-sm leading-4 text-muted-foreground">{estimateLine}</span>
              </a>
            {:else}
              <button
                type="button"
                class="grid cursor-pointer text-right"
                onclick={() => (page.data.nav?.user ? openMakeOffer(module) : goto('/login'))}
              >
                <span>
                  {module.public_asset.price
                    ? toIskCompact(module.public_asset.price)
                    : 'Make offer'}
                </span>
                <span class="absolute inset-0"></span>
                <span class="text-sm leading-4 text-muted-foreground">{estimateLine}</span>
              </button>
            {/if}
          </div>
        {:else}
          <!-- The legacy Grid/EstimatedValue.vue fallback row. -->
          <a
            href="/modules/{module.slug}"
            class="grid h-[50px] grid-cols-[36px_1fr] items-center bg-card px-2"
          >
            <div class="grid place-items-center text-green-500">
              <Cpu stroke-width={1} class="h-[1em] w-[1em]" />
            </div>
            <div class="grid text-right">
              <span>{estimateLine}</span>
              <span class="text-sm leading-4 text-muted-foreground">
                {module.creator ? `Created by ${module.creator.name}` : ''}
              </span>
            </div>
          </a>
        {/if}

        <ModuleEditRow {module} mode="note" allowed={canNote} />
        <ModuleEditRow {module} mode="collection-note" allowed={canCollectionNote} />
        <ModuleEditRow {module} mode="price" allowed={canPrice} />
      </div>
    {/snippet}
  </ContextMenu.Trigger>
  <ContextMenu.Content class="w-60 rounded-lg border">
    <ModuleMenuItems {module} {statistics} kind="context" />
  </ContextMenu.Content>
</ContextMenu.Root>
