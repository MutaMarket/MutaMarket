<script lang="ts">
	// A collection's modules with the filter band, mirroring the legacy
	// ShowCollectionPage's filter set (general, misc, value, attributes).
	import FilterBand from '$lib/components/filter-band.svelte';
	import ModuleCard from '$lib/components/module-card.svelte';
	import ModuleOptionsBar from '$lib/components/module-options-bar.svelte';
	import { parseQueryUi } from '$lib/query';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
	const search = $derived(parseQueryUi(data.query));
	const prefix = $derived(`collections/${data.page.collection.slug}`);
</script>

<svelte:head><title>{data.page.collection.name} - MutaMarket</title></svelte:head>

<h1 class="mb-1 text-xl font-semibold">{data.page.collection.name}</h1>
<p class="mb-4 text-sm text-muted-foreground">
	by {data.page.collection.character_name}{data.page.collection.description
		? ` · ${data.page.collection.description}`
		: ''}
</p>
<FilterBand
	{prefix}
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant="collection"
/>
<div class="my-4 w-full">
	{#if data.page.modules.length > 0}
		<ModuleOptionsBar {settings} />
		<div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
			{#each data.page.modules as module (module.id)}
				<ModuleCard {module} {settings} />
			{/each}
		</div>
	{:else}
		<p class="text-muted-foreground">No modules match this search.</p>
	{/if}
</div>
