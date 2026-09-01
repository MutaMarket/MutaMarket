<script lang="ts">
	// One asset location, the legacy ShowLocationPage: the location
	// header with its breadcrumb and stats, the filter band, the module
	// grid, and the create-a-collection action.
	import { FolderPlus } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import FilterBand from '$lib/components/filter-band.svelte';
	import GameImage from '$lib/components/game-image.svelte';
	import ModuleDisplay from '$lib/components/module-display.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import { toIskCompact } from '$lib/format-number';
	import { notifyError } from '$lib/toast';
	import { parseQueryUi } from '$lib/query';
	import type { PageProps } from './$types';
	import PageMeta from '$lib/components/page-meta.svelte';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
	const search = $derived(parseQueryUi(data.query));
	const prefix = $derived(`locations/${data.locationSlug}`);

	const name = $derived(data.location.name || data.location.type?.name || 'Unknown Location');

	let creating = $state(false);
	async function createCollection() {
		creating = true;
		const response = await fetch('/location-collections', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ location_id: data.location.id }),
		});
		creating = false;
		if (response.redirected) {
			goto(new URL(response.url).pathname);
		} else if (response.ok) {
			goto('/collections');
		} else {
			notifyError('Collection not created', 'Something went wrong.');
		}
	}
</script>

<PageMeta
	title={data.panel ? `${data.panel.type_name} in ${data.location.name}` : name}
	description="Find the perfect abyssal module for your needs on MutaMarket, the best place to buy and sell abyssal modules!"
	keywords="contracts, public, search, find"
/>

<PageHeader
	title={name}
	subtitle={data.location.type?.name ?? 'Location'}
	stats={[
		{
			label: 'Modules',
			value: data.stats.total_count.toLocaleString('en-US'),
			accent: 'primary',
		},
		{ label: 'Total value', value: toIskCompact(data.stats.total_value) },
		{
			label: 'Gold bars',
			value: data.stats.goldbars_count.toLocaleString('en-US'),
			accent: 'gold',
		},
	]}
>
	{#snippet icon()}
		<GameImage
			src="https://images.evetech.net/types/{data.location.type?.id ?? 0}/icon?size=64"
			alt={name}
			class="size-10 rounded-lg"
		/>
	{/snippet}
	{#snippet actions()}
		<div class="flex items-center gap-3">
			{#if data.location.location}
				<a
					href="/locations/{data.location.location.slug}"
					class="text-sm text-muted-foreground hover:text-foreground hover:underline"
				>
					in {data.location.location.type?.name ?? 'Unknown Location'}
				</a>
			{/if}
			<Button onclick={createCollection} disabled={creating}>
				<FolderPlus class="size-4" />
				Create Collection
			</Button>
		</div>
	{/snippet}
</PageHeader>
<FilterBand
	{prefix}
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant="location"
/>
<div class="my-4 w-full">
	<ModuleDisplay
		entries={data.modules.map((module) => ({ module }))}
		{settings}
		panel={data.panel}
		{search}
		{prefix}
	/>
</div>
