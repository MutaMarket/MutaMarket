<script lang="ts">
	// The characters index ported from the Leptos CharactersPage: cards of
	// characters with public modules.
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head><title>Characters - MutaMarket</title></svelte:head>

<h1 class="mb-4 text-xl font-semibold">Characters</h1>
{#if data.characters.length > 0}
	<div class="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
		{#each data.characters as character (character.id)}
			<a
				class="flex items-center gap-3 rounded-lg border border-border bg-card-1 p-3 hover:brightness-110"
				href="/characters/{character.slug}"
			>
				<img
					alt=""
					class="size-12 rounded-lg"
					src="https://images.evetech.net/characters/{character.id}/portrait?size=64"
				/>
				<span>
					<span class="block text-sm text-foreground">{character.name}</span>
					<span class="block text-xs text-muted-foreground">
						{character.modules_count ?? 0} public modules
					</span>
				</span>
			</a>
		{/each}
	</div>
{:else}
	<p class="text-muted-foreground">No characters with public modules yet.</p>
{/if}
