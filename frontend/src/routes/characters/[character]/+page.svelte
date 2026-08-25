<script lang="ts">
	// A character's public modules, ported from the Leptos CharacterPage.
	import ModuleCard from '$lib/components/module-card.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
</script>

<svelte:head><title>{data.page.character.name} - MutaMarket</title></svelte:head>

<h1 class="mb-1 text-xl font-semibold">{data.page.character.name}</h1>
<p class="mb-4 text-sm text-muted-foreground">{data.page.character.description ?? ''}</p>
<div class="relative grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
	{#each data.page.modules as module (module.id)}
		<ModuleCard {module} {settings} />
	{/each}
</div>
