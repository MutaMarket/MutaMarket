<script lang="ts">
  // The table view, the legacy Table/TableModules.vue: a real table with
  // a sticky type column, one sortable centered column per attribute
  // with a real roll range (best !== worst), and the sticky
  // price/options column (price sorting on the market page only, like
  // the legacy home-route check). Without a category there are no
  // columns to show, so the view asks for one.
  import { ArrowUpDown, TriangleAlert } from '@lucide/svelte';
  import GameImage from './game-image.svelte';
  import ModuleTableRow from './module-table-row.svelte';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import * as Table from '$lib/components/ui/table';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import type { DisplaySettings } from '$lib/display';
  import { buildQueryPath, type UiSearch } from '$lib/query';
  import type { DisplayEntry, FilterPanelData } from '$lib/types';

  let {
    entries,
    settings,
    panel,
    search,
    prefix,
    allowSortByPrice = false,
  }: {
    entries: DisplayEntry[];
    settings: DisplaySettings;
    panel: FilterPanelData | null;
    search: UiSearch;
    prefix: string;
    allowSortByPrice?: boolean;
  } = $props();

  const columns = $derived(
    (panel?.attributes ?? []).filter((attribute) => attribute.best !== attribute.worst),
  );

  // The legacy getSortDirection: 'asc' unless currently ascending.
  function sortBy(field: string) {
    const next: UiSearch = { ...search, sort: [field, search.sort?.[1] === false], page: 1 };
    goto(buildQueryPath(prefix, next), { keepFocus: true, noScroll: true });
  }

  const STICKY_HEAD =
    'sticky top-0 z-30 bg-background ' +
    'shadow-[inset_1px_0_0_0_var(--border),inset_-1px_0_0_0_var(--border)]';
</script>

{#if panel !== null}
  <Tooltip.Provider delayDuration={300}>
    <div class="my-4 flex overflow-x-auto">
      <div class="hud-frame grow">
        <Table.Root>
          <Table.Header>
            <Table.Row class="sticky top-0 z-20 bg-background">
              <Table.Head class="{STICKY_HEAD} left-0">Type</Table.Head>
              {#each columns as column (column.attribute_id)}
                <Table.Head>
                  <Tooltip.Root>
                    <Tooltip.Trigger>
                      {#snippet child({ props })}
                        <Button
                          {...props}
                          variant="ghost"
                          class="flex w-full items-center justify-center gap-4"
                          onclick={() => sortBy(column.name)}
                        >
                          <GameImage
                            src="/img/icons/{column.attribute_id}.png"
                            alt={column.display_name}
                            class="size-4"
                          />
                          <ArrowUpDown stroke-width={1} class="h-[1em] w-[1em]" />
                        </Button>
                      {/snippet}
                    </Tooltip.Trigger>
                    <Tooltip.Content>{column.display_name}</Tooltip.Content>
                  </Tooltip.Root>
                </Table.Head>
              {/each}
              <Table.Head class="{STICKY_HEAD} right-0 text-center">
                {#if allowSortByPrice}
                  <Button
                    variant="ghost"
                    class="flex w-full items-center justify-center gap-4"
                    onclick={() => sortBy('price')}
                  >
                    <GameImage src="/img/icons/wallet.png" alt="Price" class="size-4" />
                    Price
                    <ArrowUpDown stroke-width={1} class="h-[1em] w-[1em]" />
                  </Button>
                {/if}
              </Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {#each entries as entry (entry.module.id)}
              <ModuleTableRow
                module={entry.module}
                location={entry.location ?? null}
                {columns}
                {settings}
              />
            {/each}
            {#if entries.length === 0}
              <tr>
                <td class="p-4" colspan={columns.length + 2}>
                  <TriangleAlert class="mr-2 inline-block size-4 text-orange-500" />
                  <span>No modules found</span>
                </td>
              </tr>
            {/if}
          </Table.Body>
        </Table.Root>
      </div>
    </div>
  </Tooltip.Provider>
{:else}
  <div class="hud-frame my-4 flex items-center gap-4 p-4">
    <TriangleAlert class="size-8 shrink-0 text-orange-500" />
    <span class="block text-lg font-medium">Please select a category</span>
  </div>
{/if}
