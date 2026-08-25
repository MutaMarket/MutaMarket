<script lang="ts">
	// The module card ported from the Leptos ModuleCard (legacy Grid
	// Module.vue): meta-group accent header, per-attribute rows, and the
	// masonry row span so attribute rows align across neighboring cards.
	import AttributeRow from './attribute-row.svelte';
	import { isVisual, metaGroupKey } from '$lib/attributes';
	import type { DisplaySettings } from '$lib/display';
	import type { ModuleDetail } from '$lib/types';

	let { module, settings }: { module: ModuleDetail; settings: DisplaySettings } = $props();

	const headerBorder = $derived.by(() => {
		switch (metaGroupKey(module.source_type?.meta_group_id ?? null)) {
			case 't2':
				return 'border-b-orange-500';
			case 'storyline':
				return 'border-b-green-300';
			case 'faction':
				return 'border-b-green-500';
			case 'officer':
				return 'border-b-purple-500';
			case 'deadspace':
				return 'border-b-blue-500';
			default:
				return 'border-b-gray-500';
		}
	});

	const visualAttributes = $derived(module.mutated_attributes.filter(isVisual));
	// Masonry alignment: one container row per content row (header +
	// attributes + footer), so rows line up across cards.
	const rowSpan = $derived(2 + visualAttributes.length);
	const iconUrl = $derived(`https://images.evetech.net/types/${module.type.id}/icon?size=64`);
</script>

<div
	class="grid overflow-hidden rounded-lg border border-border"
	style="grid-row: span {rowSpan}"
>
	<div
		class="relative grid h-[50px] grid-cols-[36px_1fr] content-center items-center gap-x-2 border-b-2 bg-card-1 p-2 {headerBorder}"
	>
		<img alt="" class="row-span-2 size-8 rounded-lg" src={iconUrl} />
		<a class="truncate text-sm text-foreground" href="/modules/{module.slug}">
			{module.source_type?.name ?? module.type.name}
			<span aria-hidden="true" class="absolute inset-0"></span>
		</a>
		<span class="mt-1 truncate text-xs text-muted-foreground">
			{module.mutaplasmid?.name ?? ''}
		</span>
	</div>
	{#each visualAttributes as attribute (attribute.id)}
		<AttributeRow {attribute} {settings} />
	{/each}
	<div class="grid h-[50px] content-center bg-card-1 px-2 text-xs text-muted-foreground">
		Est. value: N/A
	</div>
</div>
