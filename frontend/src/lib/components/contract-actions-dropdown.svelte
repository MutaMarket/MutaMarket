<script lang="ts">
	// The legacy Tables/ContractHistory/ContractActionsDropdown.vue: copy
	// and in-game actions for every viewer, training-data moderation for
	// admins.
	import { Copy, EllipsisVertical, ExternalLink, FilePenLine, Sparkles } from '@lucide/svelte';
	import { invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { copyWithToasts } from '$lib/export';
	import { openContractInGame } from '$lib/open-contract';
	import { notifySuccess, notifyError } from '$lib/toast';
	import type { HistoricContract } from '$lib/types';

	let { contract }: { contract: HistoricContract } = $props();

	// The same in-game link system as the module toolbar (Jita).
	const CONTRACT_LINK_SYSTEM = 30000142;

	const isAdmin = $derived(Boolean(page.data.nav?.user?.is_admin));

	const copyId = () => copyWithToasts(String(contract.id), 'Contract id');
	const copyLink = () =>
		copyWithToasts(
			`<url=contract:${CONTRACT_LINK_SYSTEM}//${contract.id}>Contract ${contract.id}</url>`,
			'Contract link',
		);

	const openInGame = () => openContractInGame(contract.id);

	async function update(fields: Record<string, unknown>) {
		const response = await fetch(`/api/historic-contracts/${contract.id}`, {
			method: 'PUT',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(fields),
		});
		if (response.ok) {
			notifySuccess('Contract updated', 'The contract has been updated successfully.');
			await invalidateAll();
		} else {
			notifyError('Update failed', 'The contract could not be updated.');
		}
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<span {...props} class="inline-flex">
				<Button class="h-8 w-8 p-0" variant="ghost">
					<EllipsisVertical class="h-4 w-4" />
					<span class="sr-only">Open menu</span>
				</Button>
			</span>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end">
		<DropdownMenu.Item onclick={copyId}>
			<Copy class="size-4" />
			Copy contract id
		</DropdownMenu.Item>
		{#if contract.status !== 'failed'}
			<DropdownMenu.Item onclick={copyLink}>
				<FilePenLine class="size-4" />
				Copy contract link
			</DropdownMenu.Item>
			<DropdownMenu.Item onclick={openInGame}>
				<ExternalLink class="size-4" />
				Open contract in game
			</DropdownMenu.Item>
		{/if}
		{#if isAdmin && contract.status !== 'outstanding'}
			<DropdownMenu.Separator />
			<DropdownMenu.Item
				onclick={() => update({ ignore_for_training: !contract.ignore_for_training })}
			>
				<Sparkles class="size-4" />
				{contract.ignore_for_training ? 'Include in training' : 'Ignore for training'}
			</DropdownMenu.Item>
			<DropdownMenu.Item onclick={() => update({ non_abyssal_modules_count: 600 })}>
				<Sparkles class="size-4" />
				Set non-abyssal modules ({contract.non_abyssal_modules_count})
			</DropdownMenu.Item>
			<DropdownMenu.Separator />
			<DropdownMenu.Item onclick={() => update({ status: 'failed' })}>
				<Sparkles class="size-4" />
				Set status failed
			</DropdownMenu.Item>
			<DropdownMenu.Item onclick={() => update({ status: 'completed' })}>
				<Sparkles class="size-4" />
				Set status completed
			</DropdownMenu.Item>
			<DropdownMenu.Item onclick={() => update({ status: 'unknown' })}>
				<Sparkles class="size-4" />
				Set status unknown
			</DropdownMenu.Item>
		{/if}
	</DropdownMenu.Content>
</DropdownMenu.Root>
