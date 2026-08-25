<script lang="ts">
	// The public collections index ported from the Leptos CollectionsPage.
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head><title>Collections - MutaMarket</title></svelte:head>

<h1 class="mb-4 text-xl font-semibold">Collections</h1>
{#if data.collections.length > 0}
	<div class="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
		{#each data.collections as collection (collection.id)}
			<a
				class="rounded-lg border border-border bg-card-1 p-3 hover:brightness-110"
				href="/collections/{collection.slug}"
			>
				<span class="block text-sm text-foreground">{collection.name}</span>
				<span class="block text-xs text-muted-foreground">
					by {collection.character_name} · {collection.modules_count} modules
				</span>
			</a>
		{/each}
	</div>
{:else}
	<p class="text-muted-foreground">No public collections yet.</p>
{/if}
