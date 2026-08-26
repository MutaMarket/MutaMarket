<script lang="ts">
	// A collection's modules with the filter band, mirroring the legacy
	// ShowCollectionPage's filter set (general, misc, value, attributes).
	import FilterBand from '$lib/components/filter-band.svelte';
	import ModuleDisplay from '$lib/components/module-display.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Layers } from '@lucide/svelte';
	import { toIskCompact } from '$lib/format-number';
	import { parseQueryUi } from '$lib/query';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
	const search = $derived(parseQueryUi(data.query));
	const prefix = $derived(`collections/${data.page.collection.slug}`);
</script>

<svelte:head><title>{data.page.collection.name} - MutaMarket</title></svelte:head>

<PageHeader
	title={data.page.collection.name}
	subtitle={`by ${data.page.collection.character_name}${
		data.page.collection.description ? ` · ${data.page.collection.description}` : ''
	}`}
	stats={[
		{
			label: 'Modules',
			value: data.page.collection.modules_count.toLocaleString('en-US'),
			accent: 'primary'
		},
		{ label: 'Est. value', value: toIskCompact(data.page.estimated_value_total) }
	]}
>
	{#snippet icon()}
		<div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
			<Layers class="size-5 text-primary" stroke-width={1.5} />
		</div>
	{/snippet}
</PageHeader>
<FilterBand
	{prefix}
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant="collection"
/>
<div class="my-4 w-full">
	<ModuleDisplay
		entries={data.page.modules.map((module) => ({ module }))}
		{settings}
		panel={data.panel}
		{search}
		{prefix}
	/>
</div>
