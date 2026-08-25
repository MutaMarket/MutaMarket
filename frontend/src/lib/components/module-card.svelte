<script lang="ts">
	// The module card mirroring the legacy Grid/Module.vue tree
	// (specs/module-show.md §3): meta-group accent header with the local
	// abyssal icon, per-attribute rows, and exactly one location row —
	// Contract when for sale, the owner's Asset, else the EstimatedValue
	// fallback. Training/PublicAsset rows and the note/collection-note/
	// asking-price rows arrive with their backend features.
	import { ArrowLeftRight, Cpu, Gavel } from '@lucide/svelte';
	import AttributeRow from './attribute-row.svelte';
	import GameImage from './game-image.svelte';
	import { isVisual, metaGroupKey } from '$lib/attributes';
	import type { DisplaySettings } from '$lib/display';
	import { toIskCompact } from '$lib/format-number';
	import { locationFlagLabel } from '$lib/location-flags';
	import type { AssetLocationView, ModuleDetail } from '$lib/types';

	let {
		module,
		settings,
		asset = null
	}: {
		module: ModuleDetail;
		settings: DisplaySettings;
		/** The owner's asset location row, the legacy Grid Asset.vue. */
		asset?: AssetLocationView | null;
	} = $props();

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
	// Masonry alignment like the legacy getRowSpan: header + exactly one
	// location row + one per visual attribute.
	const rowSpan = $derived(2 + visualAttributes.length);

	// "est. 142 million ISK" / "No estimate available" (legacy card copy).
	const estimateLine = $derived(
		module.estimated_value !== null
			? `est. ${toIskCompact(module.estimated_value)}`
			: 'No estimate available'
	);
</script>

<div
	class="grid overflow-hidden rounded-lg border border-border *:first:rounded-t-lg *:last:rounded-b-lg"
	style="grid-row: span {rowSpan}"
>
	<div
		class="relative grid h-[50px] grid-cols-[36px_1fr_auto] content-center items-center gap-x-2 border-b-2 bg-card-1 p-2 {headerBorder}"
	>
		<GameImage
			src="/img/icons/{module.type.id}.png"
			alt={module.type.name}
			class="row-span-2 size-8 rounded-lg"
		/>
		<!-- Explicit rows: in the legacy the dropdown trigger occupies
		     column 3, which pushes the mutaplasmid line under the name. -->
		<a
			class="col-start-2 row-start-1 truncate text-sm text-foreground"
			href="/modules/{module.slug}"
		>
			{module.source_type?.name ?? module.type.name}
			<span aria-hidden="true" class="absolute inset-0"></span>
		</a>
		<span class="col-start-2 row-start-2 mt-1 truncate text-xs text-muted-foreground">
			{module.mutaplasmid?.name ?? ''}
		</span>
	</div>

	{#each visualAttributes as attribute (attribute.id)}
		<AttributeRow {attribute} {settings} />
	{/each}

	{#if module.contract}
		<!-- The legacy Grid/Contract.vue: sale type icon and price. -->
		<a
			href="/modules/{module.slug}"
			class="grid h-[50px] grid-cols-[36px_1fr] items-center bg-card px-2"
		>
			<div class="relative grid place-items-center text-amber-500">
				{#if module.contract.type === 'item_exchange'}
					<ArrowLeftRight stroke-width={1} class="h-[1em] w-[1em]" />
				{:else}
					<Gavel stroke-width={1} class="h-[1em] w-[1em]" />
				{/if}
				{#if module.contract.abyssal_modules_count > 1}
					<span class="absolute top-1/2 left-full -translate-y-1/2 text-xs">
						+{module.contract.abyssal_modules_count - 1}
					</span>
				{/if}
			</div>
			<div class="grid text-right">
				<span>{toIskCompact(module.contract.price)}</span>
				<span class="text-sm leading-4 text-muted-foreground">{estimateLine}</span>
			</div>
		</a>
	{:else if asset}
		<!-- The legacy Grid/Asset.vue: where the owner's module sits. -->
		<a
			class="relative grid grid-cols-[36px_1fr_auto] items-center gap-2 bg-card p-2"
			href="/locations/{asset.parent_slug}"
		>
			{#if asset.parent_type_id !== null}
				<GameImage
					src="https://images.evetech.net/types/{asset.parent_type_id}/icon?size=64"
					alt={asset.parent_name}
					class="size-9 rounded-lg"
				/>
			{:else}
				<span></span>
			{/if}
			<div class="overflow-hidden py-[3px] text-xs">
				<span class="block truncate font-medium">{asset.parent_name}</span>
				<span class="truncate text-muted-foreground">
					{locationFlagLabel(asset.location_flag)} | Est. {toIskCompact(module.estimated_value)}
				</span>
			</div>
			<div class="pr-2 pl-4 font-medium">{asset.location_index + 1}</div>
		</a>
	{:else}
		<!-- The legacy Grid/EstimatedValue.vue fallback row. -->
		<a
			href="/modules/{module.slug}"
			class="grid h-[50px] grid-cols-[36px_1fr] items-center bg-card px-2"
		>
			<div class="grid place-items-center text-green-500">
				<Cpu stroke-width={1} class="h-[1em] w-[1em]" />
			</div>
			<div class="grid text-right">
				<span>{estimateLine}</span>
				<span class="text-sm leading-4 text-muted-foreground">
					{module.creator ? `Created by ${module.creator.name}` : ''}
				</span>
			</div>
		</a>
	{/if}
</div>
