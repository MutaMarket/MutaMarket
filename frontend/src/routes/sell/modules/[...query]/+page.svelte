<script lang="ts">
	// The sell page, the legacy ShowSellModulesPage: your published
	// modules under the full filter grammar, shaped like My Modules —
	// with the import status in the header plus the select-modules
	// dialog for publishing containers.
	import { MoveRight, PackagePlus } from '@lucide/svelte';
	import AssetImportStatus from '$lib/components/asset-import-status.svelte';
	import FilterBand from '$lib/components/filter-band.svelte';
	import ModuleDisplay from '$lib/components/module-display.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import SelectModulesDialog from '$lib/components/select-modules-dialog.svelte';
	import { Button } from '$lib/components/ui/button';
	import { toIskCompact } from '$lib/format-number';
	import { parseQueryUi } from '$lib/query';
	import type { AssetImportView } from '$lib/types';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
	const search = $derived(parseQueryUi(data.query));

	let selecting = $state(false);

	// The live import state, shared with the personal page pattern (the
	// AssetImportUpdated event on the user's channel).
	let currentImport = $state<AssetImportView | null>(null);
	$effect(() => {
		currentImport = data.personal.asset_import;
	});
	$effect(() => {
		const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
		const socket = new WebSocket(`${scheme}://${location.host}/ws`);
		const channel = `Users.${data.personal.user_id}`;

		socket.onmessage = (event) => {
			try {
				const envelope = JSON.parse(event.data as string) as {
					channel: string;
					event: string;
					data: AssetImportView | null;
				};
				if (envelope.channel === channel && envelope.event === 'AssetImportUpdated') {
					currentImport = envelope.data;
				}
			} catch {
				// Not an envelope; ignore.
			}
		};

		return () => socket.close();
	});
</script>

<svelte:head><title>Sell Modules - MutaMarket</title></svelte:head>

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
		<!-- The selling flow reads left to right: import your assets,
		     then pick what goes public. -->
		<div class="flex items-center gap-3">
			<AssetImportStatus data={data.personal} current={currentImport} buttonVariant="secondary" />
			<MoveRight class="size-5 shrink-0 text-muted-foreground/60" stroke-width={1.5} />
			<Button class="h-8 gap-2" onclick={() => (selecting = true)}>
				<PackagePlus class="size-4" />
				Select modules
			</Button>
		</div>
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
		entries={data.entries}
		{settings}
		panel={data.panel}
		{search}
		prefix="sell/modules"
	/>
</div>

<SelectModulesDialog bind:open={selecting} />
