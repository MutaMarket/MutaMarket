<script lang="ts">
  // The source-type comparison table, mirroring the legacy
  // Tables/SourceTypes/TypesTable.vue: one row per published input type
  // of the mutaplasmid, one column per mutated attribute, sortable
  // headers, default order from the server (meta-group rank, meta
  // level, name).
  import { ArrowUpDown } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button';
  import * as HoverCard from '$lib/components/ui/hover-card';
  import * as Table from '$lib/components/ui/table';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { toMillionsCompact } from '$lib/format-number';
  import { t } from '$lib/i18n.svelte';
  import { comparisonCells, compareTypes } from '$lib/source-types';
  import type { ModuleDetail, SourceTypeComparison } from '$lib/types';

  let {
    module,
    comparisons,
  }: {
    module: ModuleDetail;
    comparisons: SourceTypeComparison[];
  } = $props();

  const rows = $derived(
    comparisons.map((comparison) => ({
      comparison,
      cells: comparisonCells(module.mutated_attributes, comparison),
    })),
  );
  type Row = (typeof rows)[number];

  const compact = $derived(module.mutated_attributes.length > 5);

  let sortKey: string | null = $state(null);
  let sortDesc = $state(false);

  function toggleSort(key: string) {
    sortDesc = sortKey === key && !sortDesc;
    sortKey = key;
  }

  function compareRows(a: Row, b: Row): number {
    if (sortKey === 'type') {
      return compareTypes(a.comparison, b.comparison);
    }
    if (sortKey === 'meta_level') {
      return (a.comparison.type.meta_level ?? 0) - (b.comparison.type.meta_level ?? 0);
    }
    if (sortKey === 'price') {
      return (
        (a.comparison.average_price ?? Number.NEGATIVE_INFINITY) -
        (b.comparison.average_price ?? Number.NEGATIVE_INFINITY)
      );
    }
    // Attribute columns sort by the formatted difference string, the
    // legacy accessorFn quirk.
    const id = Number(sortKey);
    const cell = (row: Row) => row.cells.find((cell) => cell.attribute_id === id);
    return (cell(a)?.difference ?? '').localeCompare(cell(b)?.difference ?? '', 'en', {
      numeric: true,
    });
  }

  const sorted = $derived.by(() => {
    if (sortKey === null) {
      return rows;
    }
    const copy = [...rows].sort(compareRows);
    return sortDesc ? copy.reverse() : copy;
  });

  function metaGroupDot(metaGroupId: number | null): string {
    switch (metaGroupId) {
      case 2:
        return 'bg-orange-500';
      case 3:
        return 'bg-green-300';
      case 4:
        return 'bg-green-500';
      case 5:
        return 'bg-purple-500';
      case 6:
        return 'bg-blue-500';
      default:
        return 'bg-gray-500';
    }
  }
</script>

<Tooltip.Provider delayDuration={200}>
  <div class="overflow-x-auto">
    <div class="rounded-md border whitespace-nowrap">
      <Table.Root>
        <Table.Header>
          <Table.Row>
            <Table.Head>
              <Button variant="ghost" class="gap-2 text-xs" onclick={() => toggleSort('type')}>
                {t('common.labels.type')}
                <ArrowUpDown class="size-3.5 opacity-60" />
              </Button>
            </Table.Head>
            <Table.Head>
              <Tooltip.Root>
                <Tooltip.Trigger>
                  {#snippet child({ props })}
                    <span {...props} class="inline-flex">
                      <Button
                        variant="ghost"
                        class="mx-auto flex gap-2"
                        onclick={() => toggleSort('meta_level')}
                      >
                        <img
                          alt={t('modules.sourceTypes.metaLevel')}
                          src="/img/icons/633.png"
                          class="h-4 w-4"
                        />
                        <ArrowUpDown class="size-3.5 opacity-60" />
                      </Button>
                    </span>
                  {/snippet}
                </Tooltip.Trigger>
                <Tooltip.Content>{t('modules.sourceTypes.sortByMetaLevel')}</Tooltip.Content>
              </Tooltip.Root>
            </Table.Head>
            {#each module.mutated_attributes as attribute (attribute.id)}
              <Table.Head>
                <HoverCard.Root>
                  <HoverCard.Trigger class="flex justify-center">
                    <Button
                      variant="ghost"
                      class="gap-2"
                      onclick={() => toggleSort(String(attribute.id))}
                    >
                      <img
                        alt={attribute.name}
                        src="/img/icons/{attribute.id}.png"
                        class="h-6 w-6"
                      />
                      <ArrowUpDown class="size-3.5 opacity-60" />
                    </Button>
                  </HoverCard.Trigger>
                  <HoverCard.Content class="w-auto bg-card-1" side="top">
                    <div class="flex items-center justify-center gap-4">
                      <img
                        alt={attribute.name}
                        src="/img/icons/{attribute.id}.png"
                        class="h-6 w-6"
                      />
                      <div>{attribute.display_name}</div>
                    </div>
                  </HoverCard.Content>
                </HoverCard.Root>
              </Table.Head>
            {/each}
            <Table.Head>
              <Button
                variant="ghost"
                class="ml-auto flex gap-2 text-xs"
                onclick={() => toggleSort('price')}
              >
                {t('common.labels.price')}
                <ArrowUpDown class="size-3.5 opacity-60" />
              </Button>
            </Table.Head>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each sorted as row (row.comparison.type.id)}
            <Table.Row>
              <Table.Cell>
                <div class="flex shrink items-center gap-2 text-xs">
                  <div
                    class="size-2 shrink-0 rounded-full {metaGroupDot(
                      row.comparison.type.meta_group_id,
                    )}"
                  ></div>
                  <div data-compact={compact} class="truncate data-[compact=true]:max-w-[8rem]">
                    {row.comparison.type.name}
                  </div>
                </div>
              </Table.Cell>
              <Table.Cell>
                <span class="block pr-6 text-center text-xs text-muted-foreground tabular-nums">
                  {row.comparison.type.meta_level ?? ''}
                </span>
              </Table.Cell>
              {#each row.cells as cell (cell.attribute_id)}
                <Table.Cell>
                  <div class="pr-6 text-center">
                    <Tooltip.Root>
                      <Tooltip.Trigger
                        data-positive={cell.is_positive}
                        class="data-[positive=false]:text-negative data-[positive=true]:text-positive"
                      >
                        <span>{cell.difference}</span>
                      </Tooltip.Trigger>
                      <Tooltip.Content class="text-base">
                        {t('modules.sourceTypes.baseValue', { value: cell.value })}
                      </Tooltip.Content>
                    </Tooltip.Root>
                  </div>
                </Table.Cell>
              {/each}
              <Table.Cell>
                {#if row.comparison.average_price !== null}
                  <Tooltip.Root>
                    <Tooltip.Trigger class="block w-full">
                      <span class="block text-right tabular-nums">
                        <span class="text-base">
                          {toMillionsCompact(row.comparison.average_price, false)}
                        </span>
                        <span class="ml-1.5 text-xs text-muted-foreground">M</span>
                      </span>
                    </Tooltip.Trigger>
                    <Tooltip.Content class="tabular-nums">
                      {Math.round(row.comparison.average_price).toLocaleString()} ISK
                    </Tooltip.Content>
                  </Tooltip.Root>
                {:else}
                  <span class="block text-right">{t('modules.card.notAvailable')}</span>
                {/if}
              </Table.Cell>
            </Table.Row>
          {:else}
            <Table.Row>
              <Table.Cell colspan={3 + module.mutated_attributes.length} class="p-4 text-center">
                {t('forms.baseTable.noResults')}
              </Table.Cell>
            </Table.Row>
          {/each}
        </Table.Body>
      </Table.Root>
    </div>
  </div>
</Tooltip.Provider>
