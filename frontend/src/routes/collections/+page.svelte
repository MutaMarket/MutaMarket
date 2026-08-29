<script lang="ts">
	// The collections index, the legacy ShowAllCollectionsPage: debounced
	// search, the create dialog, the viewer's own collections above the
	// public section.
	import { Layers, Plus, Search } from '@lucide/svelte';
	import CollectionCard from '$lib/components/collection-card.svelte';
	import CreateCollectionDialog from '$lib/components/create-collection-dialog.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import type { CollectionCardData } from '$lib/types-social';
	import type { PageProps } from './$types';
	import PageMeta from '$lib/components/page-meta.svelte';

	let { data }: PageProps = $props();

	/** The legacy debounce(500) on the search input. */
	const SEARCH_DEBOUNCE_MS = 500;

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let collections = $state<CollectionCardData[]>(data.collections);
	let query = $state('');
	let creating = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	function onInput() {
		clearTimeout(timer);
		timer = setTimeout(async () => {
			const target = query
				? `/api/collections?search=${encodeURIComponent(query)}`
				: '/api/collections';
			const response = await fetch(target);
			if (response.ok) {
				collections = await response.json();
			}
		}, SEARCH_DEBOUNCE_MS);
	}
</script>

<PageMeta
	title="Collections"
	description="Explore module collections on MutaMarket, the best place to buy and sell abyssal modules!"
/>

<PageHeader title="Collections" subtitle="Curated module showcases by the community">
	{#snippet icon()}
		<div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
			<Layers class="size-5 text-primary" stroke-width={1.5} />
		</div>
	{/snippet}
	{#snippet actions()}
		<div class="flex items-center gap-3">
			<div class="flex h-10 w-56 items-center gap-2 rounded-md border border-border bg-card-2 px-3">
				<Search class="size-4 shrink-0 text-muted-foreground" />
				<Input
					bind:value={query}
					oninput={onInput}
					placeholder="Search collections"
					class="h-full border-0 bg-transparent p-0 shadow-none focus-visible:ring-0 dark:bg-transparent"
				/>
			</div>
			{#if data.nav}
				<Button class="h-10 gap-2" onclick={() => (creating = true)}>
					<Plus class="size-4" />
					Create Collection
				</Button>
			{/if}
		</div>
	{/snippet}
</PageHeader>

{#if data.personal !== null}
	<section class="mb-8">
		<h2 class="hud-label mb-3">Your Collections</h2>
		{#if data.personal.length > 0}
			<div class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
				{#each data.personal as collection (collection.id)}
					<CollectionCard {collection} owned />
				{/each}
			</div>
		{:else}
			<p class="text-sm text-muted-foreground">You have not created any collections yet.</p>
		{/if}
	</section>
{/if}

<section>
	{#if data.personal !== null}
		<h2 class="hud-label mb-3">Public Collections</h2>
	{/if}
	{#if collections.length > 0}
		<div class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
			{#each collections as collection (collection.id)}
				<CollectionCard {collection} />
			{/each}
		</div>
	{:else}
		<p class="text-muted-foreground">There are no public collections yet.</p>
	{/if}
</section>

<CreateCollectionDialog bind:open={creating} />
