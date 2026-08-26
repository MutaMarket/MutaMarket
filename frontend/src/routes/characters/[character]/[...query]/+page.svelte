<script lang="ts">
	// A character's modules with the full filter band, mirroring the
	// legacy ShowCharacterModulesPage: for-sale/created scope, misc
	// chips, value slider and the attribute grid.
	import FilterBand from '$lib/components/filter-band.svelte';
	import ModuleDisplay from '$lib/components/module-display.svelte';
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
	<ModuleDisplay
		entries={data.page.modules.map((module) => ({ module }))}
		{settings}
		panel={data.panel}
		{search}
		{prefix}
	/>
</div>
