<script lang="ts">
  // The contract-history tab table, mirroring the legacy
  // ContractHistory/ContractColums.ts: id, issuer, dates, multi-item
  // badge, status, price and the actions dropdown, sortable headers.
  import { ArrowUpDown, Check, X } from '@lucide/svelte';
  import CharacterDetails from './character-details.svelte';
  import ContractActionsDropdown from './contract-actions-dropdown.svelte';
  import ContractDate from './contract-date.svelte';
  import ContractStatusBadge from './contract-status-badge.svelte';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import * as Table from '$lib/components/ui/table';
  import { parseDbTimestamp } from '$lib/duration';
  import { toCompact } from '$lib/format-number';
  import type { HistoricContract } from '$lib/types';

  let { contracts }: { contracts: HistoricContract[] } = $props();

  let sortKey: string | null = $state(null);
  let sortDesc = $state(false);

  function toggleSort(key: string) {
    sortDesc = sortKey === key && !sortDesc;
    sortKey = key;
  }

  function compare(a: HistoricContract, b: HistoricContract): number {
    switch (sortKey) {
      case 'id':
        return a.id - b.id;
      case 'issuer':
        return (a.issuer?.name ?? '').localeCompare(b.issuer?.name ?? '');
      case 'date_issued':
      case 'date_expired': {
        const key = sortKey as 'date_issued' | 'date_expired';
        const seconds = (contract: HistoricContract) =>
          contract[key] !== null ? parseDbTimestamp(contract[key]) : 0;
        return seconds(a) - seconds(b);
      }
      case 'status': {
        // The legacy quirk: outstanding first, completed last.
        if (a.status === 'outstanding') return -1;
        if (b.status === 'outstanding') return 1;
        if (a.status === 'completed') return 1;
        if (b.status === 'completed') return -1;
        return a.id - b.id;
      }
      case 'price':
        return (a.price ?? 0) - (b.price ?? 0);
      default:
        return 0;
    }
  }

  const sorted = $derived.by(() => {
    if (sortKey === null) {
      return contracts;
    }
    const copy = [...contracts].sort(compare);
    return sortDesc ? copy.reverse() : copy;
  });

  function isMultiItem(contract: HistoricContract): boolean {
    return contract.non_abyssal_modules_count + contract.abyssal_modules_count > 1;
  }
</script>

<div class="overflow-x-auto">
  <div class="rounded-md border whitespace-nowrap">
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>
            <Button variant="ghost" class="gap-2" onclick={() => toggleSort('id')}>
              Id
              <ArrowUpDown class="size-3.5 opacity-60" />
            </Button>
          </Table.Head>
          <Table.Head>
            <Button variant="ghost" class="gap-2" onclick={() => toggleSort('issuer')}>
              Issuer
              <ArrowUpDown class="size-3.5 opacity-60" />
            </Button>
          </Table.Head>
          <Table.Head>
            <Button variant="ghost" class="gap-2" onclick={() => toggleSort('date_issued')}>
              Issued at
              <ArrowUpDown class="size-3.5 opacity-60" />
            </Button>
          </Table.Head>
          <Table.Head>
            <Button variant="ghost" class="gap-2" onclick={() => toggleSort('date_expired')}>
              Expiry
              <ArrowUpDown class="size-3.5 opacity-60" />
            </Button>
          </Table.Head>
          <Table.Head class="text-center">Multi-item contract</Table.Head>
          <Table.Head>
            <Button variant="ghost" class="mx-auto flex gap-2" onclick={() => toggleSort('status')}>
              Status
              <ArrowUpDown class="size-3.5 opacity-60" />
            </Button>
          </Table.Head>
          <Table.Head>
            <Button variant="ghost" class="ml-auto flex gap-2" onclick={() => toggleSort('price')}>
              Price
              <ArrowUpDown class="size-3.5 opacity-60" />
            </Button>
          </Table.Head>
          <Table.Head></Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each sorted as contract (contract.id)}
          <Table.Row>
            <Table.Cell>{contract.id}</Table.Cell>
            <Table.Cell>
              <CharacterDetails character={contract.issuer} />
            </Table.Cell>
            <Table.Cell>
              <ContractDate date={contract.date_issued} />
            </Table.Cell>
            <Table.Cell>
              <ContractDate date={contract.date_expired} />
            </Table.Cell>
            <Table.Cell>
              <div class="text-center">
                {#if isMultiItem(contract)}
                  <Badge variant="positive">
                    <Check class="h-3" />
                    Yes
                  </Badge>
                {:else}
                  <Badge variant="muted">
                    <X class="h-3" />
                    No
                  </Badge>
                {/if}
              </div>
            </Table.Cell>
            <Table.Cell>
              <ContractStatusBadge status={contract.status} />
            </Table.Cell>
            <Table.Cell>
              <div class="text-right text-lg">{toCompact(contract.price ?? 0)}</div>
            </Table.Cell>
            <Table.Cell>
              <ContractActionsDropdown {contract} />
            </Table.Cell>
          </Table.Row>
        {:else}
          <Table.Row>
            <Table.Cell colspan={8} class="p-4 text-center">No results.</Table.Cell>
          </Table.Row>
        {/each}
      </Table.Body>
    </Table.Root>
  </div>
</div>
