<script lang="ts">
  // One row of the table view, the legacy Table/TableModule.vue: sticky
  // type cell (meta accent, abyssal + mutaplasmid icons, full-card
  // hover), one centered decimal cell per attribute column with its
  // score, and the sticky price/options cell. The note, collection-note,
  // asking-price and public-asset (make offer) buttons arrive with
  // their backend features.
  import { ArrowLeftRight, Gavel } from '@lucide/svelte';
  import GameImage from './game-image.svelte';
  import ModuleCard from './module-card.svelte';
  import ModuleMenuItems from './module-menu-items.svelte';
  import {
    attributeScoreClass,
    attributeScoreLabel,
    formatDecimal,
    isVisual,
    metaGroupKey,
  } from '$lib/attributes';
  import { Button } from '$lib/components/ui/button';
  import * as ContextMenu from '$lib/components/ui/context-menu';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import * as HoverCard from '$lib/components/ui/hover-card';
  import * as Table from '$lib/components/ui/table';
  import type { DisplaySettings } from '$lib/display';
  import { toCompact } from '$lib/format-number';
  import type { FilterAttribute, ModuleDetail } from '$lib/types';

  let {
    module,
    columns,
    settings,
  }: {
    module: ModuleDetail;
    columns: FilterAttribute[];
    settings: DisplaySettings;
  } = $props();

  // See module-card.svelte: legacy loads the user's asset on every
  // module, not just on the personal pages.
  const location = $derived(module.asset ?? null);

  const columnCells = $derived(
    columns.map((column) => ({
      column,
      attribute:
        module.mutated_attributes.find(
          (attribute) => attribute.id === column.attribute_id && isVisual(attribute),
        ) ?? null,
    })),
  );

  const metaBar = $derived.by(() => {
    switch (metaGroupKey(module.source_type?.meta_group_id ?? null)) {
      case 't2':
        return 'bg-orange-500';
      case 'storyline':
        return 'bg-green-300';
      case 'faction':
        return 'bg-green-500';
      case 'officer':
        return 'bg-purple-500';
      case 'deadspace':
        return 'bg-blue-500';
      default:
        return 'bg-gray-500';
    }
  });

  // The legacy price chain: sold price, live contract, else nothing
  // (the public-asset price waits on its feature).
  const price = $derived.by(() => {
    if (module.training_module) {
      return toCompact(module.training_module.sold_for ?? 0);
    }
    if (module.contract) {
      return toCompact(module.contract.price ?? 0);
    }
    return null;
  });

  const STICKY_CELL =
    'sticky z-10 bg-background group-hover:bg-card ' +
    'shadow-[inset_1px_0_0_0_var(--border),inset_-1px_0_0_0_var(--border)]';
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger>
    {#snippet child({ props })}
      <Table.Row {...props} class="group">
        <Table.Cell class="{STICKY_CELL} left-0">
          <HoverCard.Root>
            <HoverCard.Trigger>
              {#snippet child({ props: triggerProps })}
                <a
                  {...triggerProps}
                  href="/modules/{module.slug}"
                  class="grid grid-cols-[1px_2rem_2rem] items-center gap-2"
                >
                  <div class="h-6 w-[1px] rounded-full {metaBar}"></div>
                  <GameImage
                    src="/img/icons/{module.type.id}.png"
                    alt={module.source_type?.name ?? module.type.name}
                    class="size-8 rounded-lg"
                  />
                  {#if module.mutaplasmid}
                    <GameImage
                      src="https://images.evetech.net/types/{module.mutaplasmid.id}/icon?size=64"
                      alt={module.mutaplasmid.name}
                      class="size-8 rounded-lg"
                    />
                  {:else}
                    <span></span>
                  {/if}
                </a>
              {/snippet}
            </HoverCard.Trigger>
            <HoverCard.Content class="w-80" side="right">
              <ModuleCard {module} {settings} />
            </HoverCard.Content>
          </HoverCard.Root>
        </Table.Cell>
        {#each columnCells as cell (cell.column.attribute_id)}
          <Table.Cell>
            {#if cell.attribute}
              <div class="flex items-center justify-center gap-2">
                <span>
                  {formatDecimal(
                    cell.attribute.value,
                    cell.attribute.unit?.name ?? null,
                    cell.attribute.unit?.display_name ?? null,
                  )}
                </span>
                <div class="flex w-4 items-center justify-center">
                  <span
                    class="inline-block text-sm font-medium {attributeScoreClass(cell.attribute)}"
                  >
                    {attributeScoreLabel(cell.attribute)}
                  </span>
                </div>
              </div>
            {:else}
              <div class="text-center text-muted-foreground">N/A</div>
            {/if}
          </Table.Cell>
        {/each}
        <Table.Cell class="{STICKY_CELL} right-0">
          <div class="flex items-center justify-end gap-2">
            <Button variant="ghost" class="grid h-auto text-right" href="/modules/{module.slug}">
              {#if price !== null}
                <span class="block">{price}</span>
              {:else}
                <span class="block text-white">-</span>
              {/if}
              <span class="text-muted-foreground">
                {toCompact(module.estimated_value ?? 0)}
              </span>
            </Button>
            <div class="flex items-center">
              {#if module.contract?.type === 'item_exchange'}
                <ArrowLeftRight stroke-width={1} class="w-8 text-amber-500" />
              {:else if module.contract?.type === 'auction'}
                <Gavel stroke-width={1} class="w-8 text-amber-500" />
              {:else if location}
                <HoverCard.Root>
                  <HoverCard.Trigger>
                    {#snippet child({ props: triggerProps })}
                      <a {...triggerProps} href="/locations/{location.parent_slug}">
                        {#if location.parent_type_id !== null}
                          <GameImage
                            src="https://images.evetech.net/types/{location.parent_type_id}/icon?size=64"
                            alt={location.parent_name}
                            class="size-8 rounded-lg"
                          />
                        {/if}
                      </a>
                    {/snippet}
                  </HoverCard.Trigger>
                  <HoverCard.Content class="p-2">
                    <span class="block truncate text-sm font-medium">{location.parent_name}</span>
                    <span class="text-xs text-muted-foreground">
                      Slot {location.location_index + 1}
                    </span>
                  </HoverCard.Content>
                </HoverCard.Root>
              {/if}
            </div>
            <DropdownMenu.Root>
              <DropdownMenu.Trigger>
                {#snippet child({ props: triggerProps })}
                  <Button {...triggerProps} variant="secondary">Options</Button>
                {/snippet}
              </DropdownMenu.Trigger>
              <DropdownMenu.Content align="start" side="left" class="w-60 rounded-lg border">
                <ModuleMenuItems {module} statistics={null} kind="dropdown" />
              </DropdownMenu.Content>
            </DropdownMenu.Root>
          </div>
        </Table.Cell>
      </Table.Row>
    {/snippet}
  </ContextMenu.Trigger>
  <ContextMenu.Content class="w-60 rounded-lg border">
    <ModuleMenuItems {module} statistics={null} kind="context" />
  </ContextMenu.Content>
</ContextMenu.Root>
