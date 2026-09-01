<script lang="ts">
	// The module browser, mirroring the legacy browse pages: the filter
	// band above the grid (specs/browser-filters.md §1), then the options
	// bar and the masonry card grid.
	import FilterBand from './filter-band.svelte';
	import Logo from './logo.svelte';
	import ModuleDisplay from './module-display.svelte';
	import PageHeader, { type HeaderStat } from './page-header.svelte';
	import type { DisplaySettings } from '$lib/display';
	import { parseQueryUi } from '$lib/query';
	import type { BrowserData } from '$lib/server/browser';

	let { data, settings }: { data: BrowserData; settings: DisplaySettings } = $props();

	const search = $derived(parseQueryUi(data.query));
	const archive = $derived(data.prefix === 'all-modules');
	const historic = $derived(data.prefix === 'historic-sales');

	const count = (value: number) => value.toLocaleString('en-US');

	const stats = $derived.by((): HeaderStat[] => {
		if (!data.stats) {
			return [];
		}
		if (archive) {
			return [
				{ label: 'Archived', value: count(data.stats.total_count), accent: 'primary' },
				{ label: 'Gold bars', value: count(data.stats.goldbars_count), accent: 'gold' },
				{ label: 'Diamond bars', value: count(data.stats.diamondbars_count), accent: 'diamond' },
				{ label: 'Added 24h', value: count(data.stats.added_last_day_count) },
			];
		}
		return [
			{ label: 'For sale', value: count(data.stats.listed_count), accent: 'primary' },
			{ label: 'Auctions', value: count(data.stats.auctions_count) },
			{ label: 'Exchanges', value: count(data.stats.item_exchanges_count) },
			{ label: 'Added 24h', value: count(data.stats.added_last_day_count) },
		];
	});
</script>

<PageHeader
	title={historic ? 'Historic Sales' : archive ? 'All Modules' : 'Modules for Sale'}
	subtitle={historic
		? 'Recorded sales · what abyssal modules actually went for'
		: archive
			? 'The archive · every module ever indexed'
			: 'All modules on contracts and public assets'}
	{stats}
>
	{#snippet icon()}
		<Logo class="size-9 {archive || historic ? 'text-muted-foreground' : 'text-primary'}" />
	{/snippet}
</PageHeader>
<FilterBand
	prefix={data.prefix}
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant={data.prefix === 'modules' ? 'market' : historic ? 'historic' : 'archive'}
/>
<div class="my-4 w-full">
	<ModuleDisplay
		entries={data.modules.map((module) => ({ module }))}
		{settings}
		panel={data.panel}
		{search}
		prefix={data.prefix}
		allowSortByPrice={data.prefix === 'modules' || historic}
	/>
</div>
