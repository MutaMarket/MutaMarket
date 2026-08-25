<script lang="ts">
	// The personal modules page ported from the Leptos PersonalModulesPage
	// (legacy ShowAllPersonalModulesPage.vue): the user's owned modules
	// next to the asset import panel.
	import AssetImportPanel from '$lib/components/asset-import-panel.svelte';
	import ModuleCard from '$lib/components/module-card.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
</script>

<svelte:head><title>Your Modules - MutaMarket</title></svelte:head>

<h1 class="mb-4 text-xl font-semibold">Your Modules</h1>
<div class="my-4 flex flex-col items-start gap-4 lg:grid lg:grid-cols-[280px_1fr]">
	<div class="w-full rounded-lg border border-border bg-card-1">
		<AssetImportPanel data={data.personal} />
	</div>
	<div class="w-full">
		{#if data.entries.length > 0}
			<div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
				{#each data.entries as entry (entry.module.id)}
					<ModuleCard module={entry.module} {settings} asset={entry.location} />
				{/each}
			</div>
		{:else}
			<p class="text-muted-foreground">No owned modules yet - import your assets to see them here.</p>
		{/if}
	</div>
</div>
