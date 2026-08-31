<script lang="ts">
	// One row of the list view, the legacy List/ListModule.vue tree with
	// its cell components (ListHeader, ListAttribute, ListTraining,
	// ListContract, ListAsset, ListEstimatedValue) inlined. The note,
	// collection-note and asking-price cell is its own component.
	import { ArrowLeftRight, Cpu, EllipsisVertical, Gavel, Sparkles } from '@lucide/svelte';
	import GameImage from './game-image.svelte';
	import ListAttribute from './list-attribute.svelte';
	import ModuleEditCell from './module-edit-cell.svelte';
	import ModuleMenuItems from './module-menu-items.svelte';
	import { isVisual, metaGroupKey } from '$lib/attributes';
	import { Button } from '$lib/components/ui/button';
	import * as ContextMenu from '$lib/components/ui/context-menu';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as HoverCard from '$lib/components/ui/hover-card';
	import type { DisplaySettings } from '$lib/display';
	import { parseDbTimestamp, relativeTime } from '$lib/duration';
	import { toIskCompact } from '$lib/format-number';
	import { locationFlagLabel } from '$lib/location-flags';
	import type { AssetLocationView, FilterAttribute, ModuleDetail } from '$lib/types';

	let {
		module,
		location = null,
		columns = null,
		settings
	}: {
		module: ModuleDetail;
		/** The owner's asset location (personal page rows). */
		location?: AssetLocationView | null;
		/** Attribute columns when a category is selected; null flows. */
		columns?: FilterAttribute[] | null;
		settings: DisplaySettings;
	} = $props();

	const columnar = $derived((columns?.length ?? 0) > 0);
	const visualAttributes = $derived(module.mutated_attributes.filter(isVisual));

	// Each column resolves to the module's matching visible attribute, or
	// stays empty (the legacy column_cells computed).
	const columnCells = $derived(
		(columns ?? []).map((column) => ({
			column,
			attribute:
				module.mutated_attributes.find(
					(attribute) => attribute.id === column.attribute_id && isVisual(attribute)
				) ?? null
		}))
	);

	const metaBorder = $derived.by(() => {
		switch (metaGroupKey(module.source_type?.meta_group_id ?? null)) {
			case 't2':
				return 'border-r-orange-500';
			case 'storyline':
				return 'border-r-green-300';
			case 'faction':
				return 'border-r-green-500';
			case 'officer':
				return 'border-r-purple-500';
			case 'deadspace':
				return 'border-r-blue-500';
			default:
				return 'border-r-gray-500';
		}
	});

	const estimateLine = $derived(
		module.estimated_value !== null
			? `est. ${toIskCompact(module.estimated_value)}`
			: 'No estimate available'
	);

	const soldAgo = $derived.by(() => {
		const soldAt = module.training_module?.sold_at;
		if (!soldAt) {
			return '';
		}
		return relativeTime(parseDbTimestamp(soldAt) - Date.now() / 1000);
	});
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger>
		{#snippet child({ props })}
			<div {...props} class="group col-span-full grid grid-cols-subgrid">
				<!-- ListHeader: the abyssal icon with the meta-group accent
				     and the source/mutaplasmid hover card. -->
				<div>
					<HoverCard.Root>
						<HoverCard.Trigger>
							{#snippet child({ props: triggerProps })}
								<a
									{...triggerProps}
									href="/modules/{module.slug}"
									class="grid h-full content-center items-center gap-x-2 border-r-2 bg-card p-2 {metaBorder}"
								>
									<GameImage
										src="/img/icons/{module.type.id}.png"
										alt={module.type.name}
										class="row-span-2 size-8 rounded-lg"
									/>
								</a>
							{/snippet}
						</HoverCard.Trigger>
						<HoverCard.Content class="border" side="right">
							<div class="grid grid-cols-[auto_1fr] items-center gap-2 px-2">
								{#if module.source_type}
									<GameImage
										src="https://images.evetech.net/types/{module.source_type.id}/icon?size=64"
										alt={module.source_type.name}
										class="size-8 rounded-lg"
									/>
									<span class="text-sm leading-tight">{module.source_type.name}</span>
								{/if}
								{#if module.mutaplasmid}
									<GameImage
										src="https://images.evetech.net/types/{module.mutaplasmid.id}/icon?size=64"
										alt={module.mutaplasmid.name}
										class="size-8 rounded-lg"
									/>
									<span class="text-sm leading-tight">{module.mutaplasmid.name}</span>
								{/if}
							</div>
						</HoverCard.Content>
					</HoverCard.Root>
				</div>

				{#if columnar}
					<div
						class="grid min-w-0 bg-card-1 group-hover:bg-card-2"
						style="grid-template-columns: repeat({columnCells.length}, minmax(0, 1fr));"
					>
						{#each columnCells as cell (cell.column.attribute_id)}
							{#if cell.attribute}
								<ListAttribute attribute={cell.attribute} {settings} compact />
							{:else}
								<div class="flex items-center justify-center px-2 py-1 text-sm text-muted-foreground">
									N/A
								</div>
							{/if}
						{/each}
					</div>
				{:else}
					<div
						class="grid min-w-0 grid-flow-col auto-cols-[minmax(90px,1fr)] overflow-x-auto bg-card-1 group-hover:bg-card-2"
					>
						{#each visualAttributes as attribute (attribute.id)}
							<ListAttribute {attribute} {settings} />
						{/each}
					</div>
				{/if}

				<ModuleEditCell {module} />

				<div class="grid grid-cols-[1fr_auto] items-center gap-2 px-2">
					{#if module.training_module}
						<!-- ListTraining: what the roll actually sold for. -->
						<a
							href="/modules/{module.slug}"
							class="grid grid-cols-[36px_1fr] items-center gap-x-2 px-2"
						>
							<div class="grid place-items-center text-green-500">
								<Sparkles stroke-width={1} class="h-[1em] w-[1em]" />
							</div>
							<div class="grid text-right">
								<span>{toIskCompact(module.training_module.sold_for)}</span>
								<span class="text-sm leading-4 text-muted-foreground">
									{estimateLine} | {soldAgo}
								</span>
							</div>
						</a>
					{:else if module.contract}
						<!-- ListContract: sale type icon and price. -->
						<a
							href="/modules/{module.slug}"
							class="grid grid-cols-[36px_1fr] items-center gap-2 px-2"
						>
							<div class="relative grid place-items-center text-amber-500">
								{#if module.contract.type === 'item_exchange'}
									<ArrowLeftRight stroke-width={1} class="h-[1em] w-[1em]" />
								{:else}
									<Gavel stroke-width={1} class="h-[1em] w-[1em]" />
								{/if}
								{#if module.contract.abyssal_modules_count > 1}
									<span class="absolute top-1/2 left-[80%] -translate-y-1/2 text-xs">
										+{module.contract.abyssal_modules_count - 1}
									</span>
								{/if}
							</div>
							<div class="grid text-right">
								<span class="whitespace-nowrap">{toIskCompact(module.contract.price)}</span>
								<span class="text-sm leading-4 whitespace-nowrap text-muted-foreground">
									{estimateLine}
								</span>
							</div>
						</a>
					{:else if location}
						<!-- ListAsset: where the owner's module sits. -->
						<a
							class="relative grid grid-cols-[36px_1fr_auto] items-center gap-2 p-2"
							href="/locations/{location.parent_slug}"
						>
							{#if location.parent_type_id !== null}
								<GameImage
									src="https://images.evetech.net/types/{location.parent_type_id}/icon?size=64"
									alt={location.parent_name}
									class="size-8 overflow-hidden rounded-lg"
								/>
							{:else}
								<span></span>
							{/if}
							<div class="overflow-hidden py-[3px] text-xs text-muted-foreground">
								<span class="block truncate font-medium">{location.parent_name}</span>
								<span class="truncate">{locationFlagLabel(location.location_flag)} |</span>
								<span class="truncate">
									{module.estimated_value !== null
										? `Est. ${toIskCompact(module.estimated_value)}`
										: 'No estimate available'}
								</span>
							</div>
							<div class="pr-2 pl-4 font-medium">{location.location_index + 1}</div>
						</a>
					{:else}
						<!-- ListEstimatedValue fallback. -->
						<a
							href="/modules/{module.slug}"
							class="grid grid-cols-[36px_1fr] items-center gap-2 px-2"
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

					<DropdownMenu.Root>
						<DropdownMenu.Trigger>
							{#snippet child({ props: triggerProps })}
								<span {...triggerProps} class="grid cursor-pointer self-center">
									<Button size="icon" variant="ghost">
										<EllipsisVertical class="size-4" />
									</Button>
								</span>
							{/snippet}
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="start" side="right" class="w-60 rounded-lg border">
							<ModuleMenuItems {module} statistics={null} kind="dropdown" />
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				</div>
			</div>
		{/snippet}
	</ContextMenu.Trigger>
	<ContextMenu.Content class="w-60 rounded-lg border">
		<ModuleMenuItems {module} statistics={null} kind="context" />
	</ContextMenu.Content>
</ContextMenu.Root>
