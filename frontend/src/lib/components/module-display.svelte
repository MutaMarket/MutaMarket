<script lang="ts">
	// The display dispatcher, the legacy Modules.vue: the options bar,
	// the grid / list / table view the display setting selects, and the
	// options bar again below.
	import ModuleCard from './module-card.svelte';
	import ModuleList from './module-list.svelte';
	import ModuleOptionsBar from './module-options-bar.svelte';
	import ModuleTable from './module-table.svelte';
	import NoModulesFound from './no-modules-found.svelte';
	import type { DisplaySettings } from '$lib/display';
	import type { UiSearch } from '$lib/query';
	import type { DisplayEntry, FilterPanelData } from '$lib/types';

	let {
		entries,
		settings,
		panel,
		search,
		prefix,
		allowSortByPrice = false
	}: {
		entries: DisplayEntry[];
		settings: DisplaySettings;
		panel: FilterPanelData | null;
		search: UiSearch;
		prefix: string;
		allowSortByPrice?: boolean;
	} = $props();
</script>

<ModuleOptionsBar {settings} {search} {prefix} />
{#if settings.display === 'table'}
	<ModuleTable {entries} {settings} {panel} {search} {prefix} {allowSortByPrice} />
{:else if settings.display === 'list'}
	<ModuleList {entries} {settings} {panel} {search} {prefix} {allowSortByPrice} />
{:else}
	<div class="relative my-4 grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
		{#each entries as entry (entry.module.id)}
			<ModuleCard module={entry.module} {settings} asset={entry.location ?? null} />
		{/each}
		{#if entries.length === 0}
			<NoModulesFound />
		{/if}
	</div>
{/if}
<ModuleOptionsBar {settings} {search} {prefix} />
