<script lang="ts">
	// The module browser, mirroring the legacy browse pages: the filter
	// band above the grid (specs/browser-filters.md §1), then the options
	// bar and the masonry card grid.
	import FilterBand from './filter-band.svelte';
	import ModuleCard from './module-card.svelte';
	import ModuleOptionsBar from './module-options-bar.svelte';
	import type { DisplaySettings } from '$lib/display';
	import { parseQueryUi } from '$lib/query';
	import type { BrowserData } from '$lib/server/browser';

	let { data, settings }: { data: BrowserData; settings: DisplaySettings } = $props();

	const search = $derived(parseQueryUi(data.query));
</script>

<h1 class="mb-4 text-xl font-semibold">Abyssal Modules</h1>
<FilterBand
	prefix={data.prefix}
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant={data.prefix === 'modules' ? 'market' : 'archive'}
/>
<div class="my-4 w-full">
	{#if data.modules.length > 0}
		<ModuleOptionsBar {settings} />
		<div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
			{#each data.modules as module (module.id)}
				<ModuleCard {module} {settings} />
			{/each}
		</div>
	{:else}
		<p class="text-muted-foreground">No modules match this search.</p>
	{/if}
</div>
