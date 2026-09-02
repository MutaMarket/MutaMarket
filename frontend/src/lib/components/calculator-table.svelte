<script lang="ts">
  // The mutation-probability results, the legacy BaseTable with
  // MutationProbabilitiyColumns: sortable Type / Mutaplasmid /
  // Probability / Cost columns, a name search, the cost breakdown on
  // hover and the Jita cost disclaimer.
  import { ArrowUpDown } from '@lucide/svelte';
  import GameImage from './game-image.svelte';
  import {
    filterRows,
    oneIn,
    sortRows,
    toPercentage,
    type CalculatorSortKey,
    type ProbabilityRow,
  } from '$lib/calculator';
  import * as HoverCard from '$lib/components/ui/hover-card';
  import { Input } from '$lib/components/ui/input';
  import * as Table from '$lib/components/ui/table';
  import { toCompact } from '$lib/format-number';

  let { rows }: { rows: ProbabilityRow[] } = $props();

  let needle = $state('');
  let sortKey = $state<CalculatorSortKey>('cost');
  let ascending = $state(true);

  const displayed = $derived(sortRows(filterRows(rows, needle), sortKey, ascending));

  function sortBy(key: CalculatorSortKey) {
    // The legacy getSortDirection: ascending unless already ascending.
    if (sortKey === key) {
      ascending = !ascending;
    } else {
      sortKey = key;
      ascending = true;
    }
  }

  const COLUMNS: { key: CalculatorSortKey; label: string; centered: boolean }[] = [
    { key: 'type', label: 'Type', centered: false },
    { key: 'mutaplasmid', label: 'Mutaplasmid', centered: false },
    { key: 'probability', label: 'Probability', centered: true },
    { key: 'cost', label: 'Cost', centered: true },
  ];
</script>

<div class="hud-frame">
  <div class="grid items-center gap-3 border-b border-border p-4 md:grid-cols-3">
    <label class="grid gap-1">
      <span class="hud-label">Search combinations</span>
      <Input type="search" placeholder="Search for a module or combination" bind:value={needle} />
    </label>
    <p class="text-center text-sm text-balance text-muted-foreground italic md:col-start-3">
      Keep in mind that the cost of the modules are based on the daily average price in Jita and may
      vary depending on the market.
    </p>
  </div>

  <Table.Root>
    <Table.Header>
      <Table.Row>
        {#each COLUMNS as column (column.key)}
          <Table.Head class={column.centered ? 'text-center' : ''}>
            <button
              type="button"
              class="inline-flex items-center gap-1.5 whitespace-nowrap transition-colors hover:text-foreground {sortKey ===
              column.key
                ? 'text-foreground'
                : ''}"
              onclick={() => sortBy(column.key)}
            >
              {column.label}
              <ArrowUpDown class="size-3.5 opacity-60" />
            </button>
          </Table.Head>
        {/each}
      </Table.Row>
    </Table.Header>
    <Table.Body>
      {#each displayed as row (`${row.mutaplasmid.id}-${row.type.id}`)}
        <Table.Row>
          <Table.Cell>
            <div class="flex items-center gap-2">
              <GameImage
                src="https://images.evetech.net/types/{row.type.id}/icon?size=32"
                alt={row.type.name}
                class="size-8 rounded-md"
              />
              <span class="whitespace-nowrap">{row.type.name}</span>
            </div>
          </Table.Cell>
          <Table.Cell>
            <div class="flex items-center gap-2">
              <GameImage
                src="https://images.evetech.net/types/{row.mutaplasmid.id}/icon?size=32"
                alt={row.mutaplasmid.name}
                class="size-8 rounded-md"
              />
              <span class="whitespace-nowrap">{row.mutaplasmid.name}</span>
            </div>
          </Table.Cell>
          <Table.Cell class="text-center">
            {#if row.probability > 0}
              <span class="tabular-nums">{toPercentage(row.probability)}</span>
              <span class="ml-2 font-medium whitespace-nowrap tabular-nums">
                {oneIn(row.probability)}
              </span>
            {:else}
              <span class="text-muted-foreground">Impossible</span>
            {/if}
          </Table.Cell>
          <Table.Cell class="text-center">
            <HoverCard.Root openDelay={200}>
              <HoverCard.Trigger
                class="cursor-default rounded-md px-2 py-1 tabular-nums hover:bg-card-2 {row.cost ===
                null
                  ? 'text-muted-foreground'
                  : ''}"
              >
                {row.cost === null ? 'N/A' : toCompact(row.cost)}
              </HoverCard.Trigger>
              <HoverCard.Content class="w-64">
                <table class="w-full text-sm">
                  <tbody>
                    <tr class="border-b border-dotted border-border">
                      <td class="py-1 text-muted-foreground">Type</td>
                      <td class="py-1 text-right tabular-nums">
                        {row.cost_type === null ? 'N/A' : toCompact(row.cost_type)}
                      </td>
                    </tr>
                    <tr class="border-b border-dotted border-border">
                      <td class="py-1 text-muted-foreground">Mutaplasmid</td>
                      <td class="py-1 text-right tabular-nums">
                        {row.cost_mutaplasmid > 0 ? toCompact(row.cost_mutaplasmid) : 'N/A'}
                      </td>
                    </tr>
                    <tr class="border-b border-dotted border-border">
                      <td class="py-1 text-muted-foreground">Probability</td>
                      <td class="py-1 text-right tabular-nums">
                        {row.probability > 0 ? toPercentage(row.probability) : 'Impossible'}
                      </td>
                    </tr>
                  </tbody>
                  <tfoot>
                    <tr class="font-medium">
                      <td class="py-1">Total</td>
                      <td class="py-1 text-right tabular-nums">
                        {row.cost === null ? 'N/A' : toCompact(row.cost)}
                      </td>
                    </tr>
                  </tfoot>
                </table>
              </HoverCard.Content>
            </HoverCard.Root>
          </Table.Cell>
        </Table.Row>
      {:else}
        <Table.Row>
          <Table.Cell colspan={4} class="py-8 text-center text-muted-foreground">
            No combinations match your search.
          </Table.Cell>
        </Table.Row>
      {/each}
    </Table.Body>
  </Table.Root>
</div>
