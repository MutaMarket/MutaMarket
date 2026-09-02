<script lang="ts">
  // The list view, the legacy List/ListModules.vue: a bordered subgrid
  // with one row per module. With a category selected the attributes
  // align into sortable columns (only those whose roll range is real,
  // best !== worst); without one each row flows its own attributes.
  // Header clicks re-sort like legacy: the direction flips on every
  // click, whichever column it lands on.
  import { ArrowUpDown } from '@lucide/svelte';
  import GameImage from './game-image.svelte';
  import ModuleListRow from './module-list-row.svelte';
  import NoModulesFound from './no-modules-found.svelte';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import type { DisplaySettings } from '$lib/display';
  import { t } from '$lib/i18n.svelte';
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
</script>

<Tooltip.Provider delayDuration={300}>
  <div class="my-4 flex overflow-x-auto">
    <div class="grow">
      <div class="hud-frame grid grid-cols-[3rem_minmax(0,1fr)_auto_auto]">
        {#if columns.length > 0}
          <div class="sticky top-0 col-span-4 grid grid-cols-subgrid self-start border-b p-2">
            <div></div>
            <div
              class="grid min-w-0"
              style="grid-template-columns: repeat({columns.length}, minmax(0, 1fr));"
            >
              {#each columns as column (column.attribute_id)}
                <Tooltip.Root>
                  <Tooltip.Trigger>
                    {#snippet child({ props })}
                      <Button
                        {...props}
                        variant="ghost"
                        class="flex w-full items-center gap-2"
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
              {/each}
            </div>
            <div></div>
            {#if allowSortByPrice}
              <Button
                variant="ghost"
                class="flex items-center gap-2"
                onclick={() => sortBy('price')}
              >
                <GameImage
                  src="/img/icons/wallet.png"
                  alt={t('common.labels.price')}
                  class="size-4"
                />
                {t('common.labels.price')}
                <ArrowUpDown stroke-width={1} class="h-[1em] w-[1em]" />
              </Button>
            {:else}
              <div></div>
            {/if}
          </div>
        {/if}
        {#each entries as entry (entry.module.id)}
          <ModuleListRow
            module={entry.module}
            columns={columns.length > 0 ? columns : null}
            {settings}
          />
        {/each}
        {#if entries.length === 0}
          <NoModulesFound />
        {/if}
      </div>
    </div>
  </div>
</Tooltip.Provider>
