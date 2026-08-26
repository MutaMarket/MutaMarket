<script lang="ts">
	// One character card of the index, the legacy
	// Characters/CharacterCard.vue: portrait on top, the name (gilded for
	// premium members), a "Modules" divider and the big count. Premium
	// members additionally wear the animated premium-card border (a
	// deliberate upgrade over legacy's gold name).
	import { Crown } from '@lucide/svelte';
	import type { CharacterCardData } from '$lib/types-social';

	let { character }: { character: CharacterCardData } = $props();
</script>

<a
	href="/characters/{character.slug}"
	class="row-span-4 grid grid-rows-subgrid overflow-hidden rounded-lg bg-card {character.has_premium
		? 'premium-card'
		: 'border border-border'}"
>
	<img
		alt={character.name}
		class="aspect-square w-full object-cover"
		loading="lazy"
		src="https://images.evetech.net/characters/{character.id}/portrait?size=256"
	/>
	<div class="row-span-3 grid grid-rows-subgrid text-center">
		<h2 class="flex items-center justify-center gap-1.5 truncate px-4 text-xl">
			{#if character.has_premium}
				<Crown class="size-4 shrink-0 text-[#d3b15f]" stroke-width={1.5} />
				<span class="text-gold truncate">{character.name}</span>
			{:else}
				<span class="truncate">{character.name}</span>
			{/if}
		</h2>
		<div class="flex items-center gap-2">
			<hr class="grow border-t border-border" />
			<p class="text-sm text-muted-foreground">Modules</p>
			<hr class="grow border-t border-border" />
		</div>
		<p class="p-4 text-6xl">{character.modules_count ?? 0}</p>
	</div>
</a>
