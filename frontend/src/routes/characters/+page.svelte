<script lang="ts">
	// The characters index, the legacy ShowAllCharactersPage: heading and
	// intro with the sell-page link, debounced name search, and the card
	// grid (premium members first, from the API ordering).
	import { Search, Users } from '@lucide/svelte';
	import CharacterCard from '$lib/components/character-card.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Input } from '$lib/components/ui/input';
	import type { CharacterCardData } from '$lib/types-social';
	import type { PageProps } from './$types';
	import PageMeta from '$lib/components/page-meta.svelte';

	let { data }: PageProps = $props();

	/** The legacy debounce(200) on the search input. */
	const SEARCH_DEBOUNCE_MS = 200;

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let characters = $state<CharacterCardData[]>(data.characters);
	let query = $state('');
	let timer: ReturnType<typeof setTimeout> | undefined;

	function onInput() {
		clearTimeout(timer);
		timer = setTimeout(async () => {
			const target = query
				? `/api/characters?search=${encodeURIComponent(query)}`
				: '/api/characters';
			const response = await fetch(target);
			if (response.ok) {
				characters = await response.json();
			}
		}, SEARCH_DEBOUNCE_MS);
	}
</script>

<PageMeta
	title="Characters"
	description="Explore the abyssal modules of the characters on MutaMarket, the best place to buy and sell abyssal modules!"
/>

<PageHeader
	title="Characters selling modules"
	subtitle="Explore the abyssal modules of all sellers"
>
	{#snippet icon()}
		<div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
			<Users class="size-5 text-primary" stroke-width={1.5} />
		</div>
	{/snippet}
	{#snippet actions()}
		<div class="flex h-10 w-64 items-center gap-2 rounded-md border border-border bg-card-2 px-3">
			<Search class="size-4 shrink-0 text-muted-foreground" />
			<Input
				bind:value={query}
				oninput={onInput}
				placeholder="Search characters"
				class="h-full border-0 bg-transparent p-0 shadow-none focus-visible:ring-0 dark:bg-transparent"
			/>
		</div>
	{/snippet}
</PageHeader>

{#if characters.length > 0}
	<div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4">
		{#each characters as character (character.id)}
			<CharacterCard {character} />
		{/each}
	</div>
{:else}
	<div class="hud-frame flex items-center gap-4 p-6">
		<Users class="size-8 shrink-0 text-primary" stroke-width={1.5} />
		<div>
			<span class="block text-lg font-medium">No characters found</span>
			<p class="text-sm text-muted-foreground">
				If you want to sell your modules, head over to the
				<a href="/sell/modules" class="text-primary hover:underline">Sell page</a>
				and make your modules public!
			</p>
		</div>
	</div>
{/if}
