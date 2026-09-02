<script lang="ts">
  import { currentDisplaySettings } from '$lib/display';
  // The personal contracts page, mirroring the legacy
  // ShowAllPersonalContractsPage.vue: the refresh action, the totals
  // cards, and the merged sortable/searchable contract table with the
  // date-range filter. The legacy popover range calendar is two plain
  // date inputs here (no range-calendar component in this kit).
  import { ArrowUpDown, FileText, Minus, TriangleAlert } from '@lucide/svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import CharacterDetails from '$lib/components/character-details.svelte';
  import ContractDate from '$lib/components/contract-date.svelte';
  import ContractStatusBadge from '$lib/components/contract-status-badge.svelte';
  import GameImage from '$lib/components/game-image.svelte';
  import ModuleCard from '$lib/components/module-card.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as HoverCard from '$lib/components/ui/hover-card';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as Table from '$lib/components/ui/table';
  import { parseDbTimestamp } from '$lib/duration';
  import { toCompact } from '$lib/format-number';
  import {
    CONTRACT_COLUMNS,
    contractTotals,
    isModuleCard,
    matchesSearch,
    mergeContracts,
    sortContracts,
  } from '$lib/personal-contracts';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  const settings = $state(currentDisplaySettings(data.displaySettings));

  const characterIds = $derived(data.nav?.characters.map((character) => character.id) ?? []);
  const merged = $derived(mergeContracts(data.page.contracts, characterIds));
  const totals = $derived(contractTotals(merged));

  let search = $state('');
  let sortKey: string | null = $state(null);
  let sortDesc = $state(false);

  function toggleSort(key: string) {
    sortDesc = sortKey === key && !sortDesc;
    sortKey = key;
  }

  const rows = $derived(
    sortContracts(
      merged.filter((contract) => matchesSearch(contract, search)),
      sortKey,
      sortDesc,
    ),
  );

  function day(timestamp: string): string {
    return new Date(parseDbTimestamp(timestamp) * 1000).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  }

  // The date inputs carry plain days, like the legacy yyyy-MM-dd reload.
  let dateStart = $state('');
  let dateEnd = $state('');
  $effect(() => {
    dateStart = data.page.date_start.slice(0, 10);
    dateEnd = data.page.date_end.slice(0, 10);
  });

  function applyDateRange() {
    if (dateStart === '' || dateEnd === '') {
      return;
    }
    void goto(`/personal/contracts?date_start=${dateStart}&date_end=${dateEnd}`, {
      invalidateAll: true,
    });
  }

  let refreshing = $state(false);

  async function refreshContracts() {
    refreshing = true;
    try {
      // The synchronous legacy dispatchSync loop: the response only
      // lands once every character's contracts are refreshed.
      await fetch('/personal/contracts', { method: 'POST', redirect: 'manual' });
      await invalidateAll();
    } finally {
      refreshing = false;
    }
  }
</script>

<PageMeta title="Your Contracts" description="View all your contracts" />

<PageHeader
  title="Your contracts"
  subtitle="{day(data.page.date_start)} - {day(data.page.date_end)}"
>
  {#snippet icon()}
    <div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
      <FileText class="size-5 text-primary" stroke-width={1.5} />
    </div>
  {/snippet}
  {#snippet actions()}
    <Button onclick={refreshContracts} disabled={refreshing}>
      {refreshing ? 'Refreshing…' : 'Refresh contracts'}
    </Button>
  {/snippet}
</PageHeader>

<div class="mb-8 grid gap-8 md:grid-cols-2 2xl:grid-cols-3">
  <div class="hud-frame grid gap-2 p-8">
    <span class="text-sm">Total earnings</span>
    <span class="text-4xl font-semibold">{toCompact(totals.earnings)}</span>
  </div>
  <div class="hud-frame grid gap-2 p-8">
    <span class="text-sm">Total spent</span>
    <span class="text-4xl font-semibold">{toCompact(totals.spent)}</span>
  </div>
  <div class="hud-frame grid gap-2 p-8">
    <span class="text-sm">Outstanding contracts</span>
    <span class="text-4xl font-semibold">{toCompact(totals.outstandingCount)}</span>
  </div>
  <div class="hud-frame grid gap-2 p-8">
    <span class="text-sm">Outstanding value</span>
    <span class="text-4xl font-semibold">{toCompact(totals.outstandingValue)}</span>
  </div>
  <div class="hud-frame grid gap-2 p-8">
    <span class="text-sm">
      <span
        class="mr-1 inline-block size-2 rounded-full {totals.profit > 0
          ? 'bg-green-500'
          : 'bg-red-500'}"
      ></span>
      Profit
    </span>
    <span class="text-4xl font-semibold">{toCompact(totals.profit)}</span>
  </div>
</div>

<div class="mb-4 grid items-center gap-2 gap-x-8 md:grid-cols-[1fr_1fr_1fr]">
  <div>
    <Label for="contract-search">Search</Label>
    <Input
      id="contract-search"
      class="max-w-md"
      placeholder="Search in contracts"
      bind:value={search}
    />
  </div>
  <div class="grid items-center gap-2">
    <Label>Date range</Label>
    <div class="flex items-center gap-2">
      <Input type="date" class="w-40" bind:value={dateStart} onchange={applyDateRange} />
      <span class="text-muted-foreground">-</span>
      <Input type="date" class="w-40" bind:value={dateEnd} onchange={applyDateRange} />
    </div>
  </div>
  <p class="text-center text-sm text-balance italic text-muted-foreground">
    Due to the nature EVEs API, some contracts may be missing data. This is not a bug, but a
    limitation of the API itself.
  </p>
</div>

<div class="hud-frame overflow-x-auto whitespace-nowrap">
  <Table.Root>
    <Table.Header>
      <Table.Row>
        {#each CONTRACT_COLUMNS as column (column.key)}
          <Table.Head class={column.key === 'modules' ? 'text-center' : ''}>
            {#if column.sortable}
              <Button
                variant="ghost"
                class="flex items-center gap-2 {column.key === 'price' ? 'ml-auto' : ''}"
                onclick={() => toggleSort(column.key)}
              >
                {column.label}
                <ArrowUpDown class="size-3.5" />
              </Button>
            {:else}
              {column.label}
            {/if}
          </Table.Head>
        {/each}
      </Table.Row>
    </Table.Header>
    <Table.Body>
      {#each rows as contract (contract.id)}
        <Table.Row>
          <Table.Cell><CharacterDetails character={contract.issuer} /></Table.Cell>
          <Table.Cell>
            {#if !contract.acceptor && contract.status === 'completed'}
              <div class="grid grid-cols-[2rem_1fr] items-center">
                <TriangleAlert class="size-5 text-orange-500" />
                <span>Missing data</span>
              </div>
            {:else if !contract.acceptor}
              <Minus class="size-4 text-muted-foreground" />
            {:else if contract.acceptor_type === 'corporation'}
              <div class="grid grid-cols-[2rem_1fr] items-center gap-2">
                <GameImage
                  src="https://images.evetech.net/corporations/{contract.acceptor.id}/logo?size=64"
                  alt={contract.acceptor.name}
                  class="size-8 rounded-md"
                />
                <span>{contract.acceptor.name}</span>
              </div>
            {:else}
              <CharacterDetails character={contract.acceptor} />
            {/if}
          </Table.Cell>
          <Table.Cell><ContractDate date={contract.date_issued} /></Table.Cell>
          <Table.Cell>
            {#if contract.date_accepted}
              <ContractDate date={contract.date_accepted} />
            {:else}
              <Minus class="size-4 text-muted-foreground" />
            {/if}
          </Table.Cell>
          <Table.Cell><ContractDate date={contract.date_expired} /></Table.Cell>
          <Table.Cell><ContractStatusBadge status={contract.status} /></Table.Cell>
          <Table.Cell>
            <div
              class="grid place-items-center items-center justify-center gap-2"
              style="grid-template-columns: repeat({Math.min(contract.modules.length, 3) ||
                1}, 2rem)"
            >
              <!-- Keyed by position, not id: the types fallback carries
							     one entry per item row, so a contract holding two of the
							     same module type repeats that type id. -->
              {#each contract.modules.slice(0, 3) as entry, index (index)}
                {#if isModuleCard(entry)}
                  <HoverCard.Root>
                    <HoverCard.Trigger>
                      {#snippet child({ props })}
                        <a {...props} href="/modules/{entry.slug}">
                          <GameImage
                            src="/img/icons/{entry.type.id}.png"
                            alt={entry.type.name}
                            class="size-8 shrink-0"
                          />
                        </a>
                      {/snippet}
                    </HoverCard.Trigger>
                    <HoverCard.Content class="w-80 border p-0">
                      <ModuleCard module={entry} {settings} />
                    </HoverCard.Content>
                  </HoverCard.Root>
                {:else}
                  <!-- A bare item type of an ESI contract: greyed, no card. -->
                  <GameImage
                    src="/img/icons/{entry.id}.png"
                    alt={entry.name}
                    class="size-8 shrink-0 opacity-25 grayscale"
                  />
                {/if}
              {/each}
            </div>
          </Table.Cell>
          <Table.Cell class="text-right">
            {#if contract.price}
              {toCompact(contract.price)}
            {:else}
              <Minus class="ml-auto size-4 text-muted-foreground" />
            {/if}
          </Table.Cell>
        </Table.Row>
      {:else}
        <Table.Row>
          <Table.Cell colspan={CONTRACT_COLUMNS.length} class="p-4 text-center"
            >No results.</Table.Cell
          >
        </Table.Row>
      {/each}
    </Table.Body>
  </Table.Root>
</div>
