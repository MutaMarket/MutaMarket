<script lang="ts">
	// The module browser, mirroring the legacy browse pages: the filter
	// band above the grid (specs/browser-filters.md §1), then the options
	// bar and the masonry card grid.
	import FilterBand from './filter-band.svelte';
	import ModuleDisplay from './module-display.svelte';
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
	<ModuleDisplay
		entries={data.modules.map((module) => ({ module }))}
		{settings}
		panel={data.panel}
		{search}
		prefix={data.prefix}
		allowSortByPrice={data.prefix === 'modules'}
	/>
</div>
