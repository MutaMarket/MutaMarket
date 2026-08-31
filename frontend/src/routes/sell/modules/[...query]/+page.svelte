<script lang="ts">
	// The sell page, the legacy ShowSellModulesPage: your published
	// modules under the full filter grammar, shaped like My Modules —
	// with the import status in the header plus the select-modules
	// dialog for publishing containers.
	import { Coins, PackagePlus } from '@lucide/svelte';
	import { invalidateAll } from '$app/navigation';
	import { importRefreshGate, subscribeAssetImport } from '$lib/asset-import-stream';
	import FilterBand from '$lib/components/filter-band.svelte';
	import ModuleDisplay from '$lib/components/module-display.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import SelectModulesDialog from '$lib/components/select-modules-dialog.svelte';
	import { Button } from '$lib/components/ui/button';
	import { toIskCompact } from '$lib/format-number';
	import { editSession, startEdit } from '$lib/module-edits';
	import { parseQueryUi } from '$lib/query';
	import type { AssetImportView, PersonalModuleEntry } from '$lib/types';
	import type { PageProps } from './$types';
	import PageMeta from '$lib/components/page-meta.svelte';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
	const search = $derived(parseQueryUi(data.query));

	let selecting = $state(false);

	// The live import state, shared with the personal page pattern (the
	// AssetImportUpdated event on the user's channel). Modules found for
	// already-published containers stream in while the import runs.
	let currentImport = $state<AssetImportView | null>(null);
	let entries = $state<PersonalModuleEntry[]>([]);
	$effect(() => {
		currentImport = data.personal.asset_import;
	});
	$effect(() => {
		entries = data.entries;
	});

	async function refreshEntries() {
		const response = await fetch(`/api/sell/modules?q=${encodeURIComponent(data.query)}`);
		if (response.ok) {
			entries = await response.json();
		}
	}

	$effect(() => {
		const gate = importRefreshGate();
		return subscribeAssetImport(data.personal.user_id, (view) => {
			currentImport = view;
			const verdict = gate(view);
			if (verdict === 'stream') {
				void refreshEntries();
			} else if (verdict === 'completed') {
				void refreshEntries();
				void invalidateAll();
			}
		});
	});
</script>

<PageMeta
	title="Sell modules"
	description="Find the perfect abyssal module for your needs on MutaMarket, the best place to buy and sell abyssal modules!"
	keywords="contracts, public, search, find"
/>

<PageHeader
	title="Sell Modules"
	subtitle="Publish containers to list their modules for sale"
	stats={[
		{
			label: 'Published',
			value: data.sell.published_count.toLocaleString('en-US'),
			accent: 'primary'
		},
		{ label: 'Est. value', value: toIskCompact(data.sell.estimated_value_total) }
	]}
>
	{#snippet icon()}
		<img
			alt=""
			class="size-10 rounded-lg"
			src="https://images.evetech.net/characters/{data.sell.character_id}/portrait?size=64"
		/>
	{/snippet}
	{#snippet actions()}
		<Button
			class="h-8 gap-2"
			variant="secondary"
			disabled={$editSession !== null}
			onclick={() => startEdit('price')}
		>
			<Coins class="size-4" />
			Edit asking prices
		</Button>
		<Button class="h-8 gap-2" onclick={() => (selecting = true)}>
			<PackagePlus class="size-4" />
			Select modules
		</Button>
	{/snippet}
</PageHeader>
<FilterBand
	prefix="sell/modules"
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant="sell"
/>
<div class="my-4 w-full">
	<ModuleDisplay
		{entries}
		{settings}
		panel={data.panel}
		{search}
		prefix="sell/modules"
	/>
</div>

<SelectModulesDialog bind:open={selecting} personal={data.personal} current={currentImport} />
