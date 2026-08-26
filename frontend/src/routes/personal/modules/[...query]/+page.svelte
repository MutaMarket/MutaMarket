<script lang="ts">
	// The personal modules page (legacy ShowAllPersonalModulesPage): the
	// filter band with the fitted/asset chips, the asset import panel and
	// the owned-module grid with locations.
	import AssetImportPanel from '$lib/components/asset-import-panel.svelte';
	import FilterBand from '$lib/components/filter-band.svelte';
	import ModuleCard from '$lib/components/module-card.svelte';
	import ModuleOptionsBar from '$lib/components/module-options-bar.svelte';
	import { parseQueryUi } from '$lib/query';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
	const search = $derived(parseQueryUi(data.query));
</script>

<svelte:head><title>Your Modules - MutaMarket</title></svelte:head>

<h1 class="mb-4 text-xl font-semibold">Your Modules</h1>
<FilterBand
	prefix="personal/modules"
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant="personal"
/>
<div class="my-4 flex flex-col items-start gap-4 lg:grid lg:grid-cols-[280px_1fr]">
	<div class="w-full rounded-lg border border-border bg-card-1">
		<AssetImportPanel data={data.personal} />
	</div>
	<div class="w-full">
		{#if data.entries.length > 0}
			<ModuleOptionsBar {settings} />
			<div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
				{#each data.entries as entry (entry.module.id)}
					<ModuleCard module={entry.module} {settings} asset={entry.location} />
				{/each}
			</div>
		{:else}
			<p class="text-muted-foreground">No modules match this search.</p>
		{/if}
	</div>
</div>
