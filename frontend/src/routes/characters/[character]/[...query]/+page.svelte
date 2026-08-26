<script lang="ts">
	// A character's modules with the full filter band, mirroring the
	// legacy ShowCharacterModulesPage: for-sale/created scope, misc
	// chips, value slider and the attribute grid.
	import FilterBand from '$lib/components/filter-band.svelte';
	import ModuleCard from '$lib/components/module-card.svelte';
	import ModuleOptionsBar from '$lib/components/module-options-bar.svelte';
	import { parseQueryUi } from '$lib/query';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
	const search = $derived(parseQueryUi(data.query));
	const prefix = $derived(`characters/${data.page.character.slug}`);
</script>

<svelte:head><title>{data.page.character.name} - MutaMarket</title></svelte:head>

<h1 class="mb-1 text-xl font-semibold">{data.page.character.name}</h1>
<p class="mb-4 text-sm text-muted-foreground">{data.page.character.description ?? ''}</p>
<FilterBand
	{prefix}
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant="character"
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
