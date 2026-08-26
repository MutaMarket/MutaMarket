<script lang="ts">
	// The list button floating dead-center over the attribute grid,
	// mirroring SourceTypeAttributeSelect.vue: pick a source type, toggle
	// attributes, and search "at least as good as this type" across all
	// checked attributes at once (specs/browser-filters.md §3.5).
	import { Check, List } from '@lucide/svelte';
	import GameImage from './game-image.svelte';
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Label } from '$lib/components/ui/label';
	import * as Select from '$lib/components/ui/select';
	import { Switch } from '$lib/components/ui/switch';
	import { metaGroupDotClass } from '$lib/filter-meta';
	import { buildQueryPath, type UiSearch } from '$lib/query';
	import type { FilterPanelData } from '$lib/types';

	let {
		prefix,
		search,
		panel
	}: {
		prefix: string;
		search: UiSearch;
		panel: FilterPanelData;
	} = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let selectedTypeId = $state(panel.source_types[0]?.id ?? 0);
	const selectedType = $derived(
		panel.source_types.find((sourceType) => sourceType.id === selectedTypeId)
	);

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let checked: Record<number, boolean> = $state(
		Object.fromEntries(panel.attributes.map((attribute) => [attribute.attribute_id, true]))
	);
	const allSelected = $derived(
		panel.attributes.every((attribute) => checked[attribute.attribute_id])
	);
	const noneSelected = $derived(
		panel.attributes.every((attribute) => !checked[attribute.attribute_id])
	);

	function toggleAll(value: boolean) {
		checked = Object.fromEntries(
			panel.attributes.map((attribute) => [attribute.attribute_id, value])
		);
	}

	function apply() {
		if (!selectedType) {
			return;
		}
		// Lower-bound-only filters at the chosen type's base values.
		const attributes = selectedType.attributes
			.filter((value) => checked[value.attribute_id])
			.flatMap((value) => {
				const attribute = panel.attributes.find(
					(candidate) => candidate.attribute_id === value.attribute_id
				);
				return attribute
					? [{ name: attribute.name, lower: value.value, upper: null }]
					: [];
			});
		goto(buildQueryPath(prefix, { ...search, attributes }), { noScroll: true });
	}
</script>

<div class="absolute top-1/2 left-1/2 z-30 hidden -translate-x-1/2 -translate-y-1/2 xl:block">
	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<span {...props} class="inline-flex">
					<Button class="bg-card-1" size="icon" variant="outline" title="Select type">
						<List class="size-4" />
					</Button>
				</span>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content>
			<div class="w-72 p-4">
				<Select.Root
					type="single"
					value={String(selectedTypeId)}
					onValueChange={(value) => (selectedTypeId = Number(value))}
				>
					<Select.Trigger class="w-full">
						{#if selectedType}
							<span class="flex items-center gap-2 truncate text-xs">
								<span
									class="size-2 shrink-0 rounded-full {metaGroupDotClass(
										selectedType.meta_group_id
									)}"
								></span>
								{selectedType.name}
							</span>
						{/if}
					</Select.Trigger>
					<Select.Content>
						{#each panel.source_types as sourceType (sourceType.id)}
							<Select.Item value={String(sourceType.id)}>
								<span class="flex items-center gap-2 text-xs">
									<span
										class="size-2 rounded-full {metaGroupDotClass(sourceType.meta_group_id)}"
									></span>
									{sourceType.name}
								</span>
							</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
				<div class="my-4 grid grid-cols-[auto_auto_1fr] items-center gap-2">
					<Switch id="all-selected" checked={allSelected} onCheckedChange={toggleAll} />
					<Label class="col-span-2 grid grid-cols-subgrid items-center" for="all-selected">
						<span class="flex items-center justify-center"><Check class="size-4" /></span>
						<span>Select all</span>
					</Label>
					{#each panel.attributes as attribute (attribute.attribute_id)}
						<Switch
							id="attribute-{attribute.attribute_id}"
							checked={checked[attribute.attribute_id] ?? false}
							onCheckedChange={(value) => (checked[attribute.attribute_id] = value)}
						/>
						<Label
							for="attribute-{attribute.attribute_id}"
							class="col-span-2 grid grid-cols-subgrid items-center gap-2"
						>
							<GameImage
								src="/img/icons/{attribute.attribute_id}.png"
								alt={attribute.display_name}
								class="size-6"
							/>
							{attribute.display_name === '' ? attribute.name : attribute.display_name}
						</Label>
					{/each}
				</div>
				<Button class="w-full" variant="secondary" disabled={noneSelected} onclick={apply}>
					Apply
				</Button>
			</div>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
