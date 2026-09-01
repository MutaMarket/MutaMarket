<script lang="ts">
	// The variance/attribute form inside the three search submenus,
	// mirroring Menus/SearchMenuForm.vue.
	import { Check } from '@lucide/svelte';
	import type { Snippet } from 'svelte';
	import GameImage from './game-image.svelte';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';

	let {
		attributes,
		enabledIds = $bindable(),
		variance = $bindable(),
		footer,
	}: {
		attributes: { id: number; display_name: string }[];
		enabledIds: number[];
		variance: number;
		footer: Snippet;
	} = $props();

	const allEnabled = $derived(
		attributes.length > 0 && attributes.every((attribute) => enabledIds.includes(attribute.id)),
	);

	function toggle(id: number) {
		enabledIds = enabledIds.includes(id)
			? enabledIds.filter((enabled) => enabled !== id)
			: [...enabledIds, id];
	}

	function toggleAll() {
		enabledIds = allEnabled ? [] : attributes.map((attribute) => attribute.id);
	}
</script>

<div class="col-span-full -m-1 flex max-w-80 min-w-64 flex-col">
	<div class="p-3">
		<Label class="hud-label" for="search-variance">Variance (%)</Label>
		<Input
			id="search-variance"
			type="number"
			min="1"
			class="mt-1 bg-input"
			bind:value={variance}
			onclick={(event) => event.stopPropagation()}
		/>
	</div>
	<div class="border-t border-border p-1.5">
		<div class="mb-1 flex items-center justify-between px-2 pt-0.5">
			<span class="hud-label">Match attributes</span>
			<button
				class="cursor-pointer text-xs text-primary hover:underline"
				type="button"
				onclick={(event) => {
					event.stopPropagation();
					toggleAll();
				}}
			>
				{allEnabled ? 'Clear all' : 'Select all'}
			</button>
		</div>
		{#each attributes as attribute (attribute.id)}
			{@const enabled = enabledIds.includes(attribute.id)}
			<button
				class="flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-sm transition-colors hover:bg-secondary"
				type="button"
				onclick={(event) => {
					event.stopPropagation();
					toggle(attribute.id);
				}}
			>
				<GameImage
					src="/img/icons/{attribute.id}.png"
					alt={attribute.display_name}
					class="size-5"
				/>
				<span class="grow truncate text-left {enabled ? '' : 'text-muted-foreground'}">
					{attribute.display_name}
				</span>
				<Check class="size-4 shrink-0 text-primary {enabled ? 'opacity-100' : 'opacity-0'}" />
			</button>
		{/each}
	</div>
	<div class="grid gap-2 border-t border-border p-3">
		{@render footer()}
	</div>
</div>
