<script lang="ts">
	// The filter band above the module grid, mirroring the legacy
	// Pages/ModulesFilters.vue composition (specs/browser-filters.md §1):
	// general filters and the switch columns (with the stats flip) on the
	// left, price/value sliders on the right, the attribute grid below.
	import { ChartColumn, Funnel } from '@lucide/svelte';
	import AttributeFilterRow from './attribute-filter-row.svelte';
	import CurrencyFilterRow from './currency-filter-row.svelte';
	import SourceTypeAttributeSelect from './source-type-attribute-select.svelte';
	import StatsStrip from './stats-strip.svelte';
	import TypeDialog from './type-dialog.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import * as RadioGroup from '$lib/components/ui/radio-group';
	import * as Select from '$lib/components/ui/select';
	import { Switch } from '$lib/components/ui/switch';
	import { META_GROUPS, META_LEVELS } from '$lib/filter-meta';
	import { buildQueryPath, type UiSearch } from '$lib/query';
	import type { FilterPanelData, ModulesStats } from '$lib/types';

	let {
		prefix,
		search,
		panel,
		unknownType,
		stats
	}: {
		prefix: string;
		search: UiSearch;
		panel: FilterPanelData | null;
		unknownType: boolean;
		stats: ModulesStats | null;
	} = $props();

	/** The all-modules page drops the market-only sections (legacy
	 * AllModulesFilters.vue). */
	const marketPage = $derived(prefix === 'modules');

	let showStats = $state(false);

	const signedIn = $derived(Boolean(page.data.nav?.user));

	function go(next: UiSearch) {
		goto(buildQueryPath(prefix, next), { keepFocus: true, noScroll: true });
	}

	// Meta selects narrow to the groups/levels present among the type's
	// source types.
	const availableGroups = $derived(
		panel === null
			? META_GROUPS
			: META_GROUPS.filter((group) =>
					panel.source_types.some((sourceType) => sourceType.meta_group_id === group.id)
				)
	);
	const availableLevels = $derived(
		panel === null
			? META_LEVELS
			: META_LEVELS.filter((level) =>
					panel.source_types.some((sourceType) => sourceType.meta_level === level.id)
				)
	);
	const selectedGroup = $derived(
		META_GROUPS.find((group) => group.slug === search.metaGroup) ?? null
	);
	const selectedLevel = $derived(
		META_LEVELS.find((level) => String(level.id) === search.metaLevel) ?? null
	);

	const attributes = $derived(
		(panel?.attributes ?? []).filter((attribute) => attribute.best !== attribute.worst)
	);
</script>

<div class="relative z-10 divide-y divide-border rounded-lg border border-border bg-card">
	<div class="grid divide-y divide-border 2xl:grid-cols-[3fr_2fr] 2xl:divide-x 2xl:divide-y-0">
		<div class="divide-y divide-border">
			<!-- GeneralFilter: type picker + narrowed meta selects. -->
			<div class="relative grid items-start gap-4 p-4 xl:grid-cols-3">
				<div>
					<h2 class="mb-2">Category</h2>
					<TypeDialog
						{prefix}
						{search}
						currentTypeId={panel?.type_id}
						currentTypeName={panel?.type_name}
					/>
					{#if unknownType}
						<p class="mt-2 text-xs text-muted-foreground">Unknown type.</p>
					{/if}
				</div>
				<div>
					<h2 class="mb-2">Meta group</h2>
					<Select.Root
						type="single"
						value={search.metaGroup ?? 'all'}
						onValueChange={(value) =>
							go({ ...search, metaGroup: value === 'all' ? null : value })}
					>
						<Select.Trigger class="w-full">
							{#if selectedGroup}
								<span class="flex w-full items-center gap-2">
									<span class="mr-auto">{selectedGroup.name}</span>
									<span class="size-2 rounded-full {selectedGroup.dotClass}"></span>
								</span>
							{:else}
								All
							{/if}
						</Select.Trigger>
						<Select.Content>
							<Select.Item value="all">All</Select.Item>
							{#each availableGroups as group (group.id)}
								<Select.Item value={group.slug}>
									<span class="flex w-full grow items-center gap-2">
										<span class="mr-auto block">{group.name}</span>
										<span class="size-2 rounded-full {group.dotClass}"></span>
									</span>
								</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>
				<div>
					<h2 class="mb-2">Meta level</h2>
					<Select.Root
						type="single"
						value={search.metaLevel ?? 'all'}
						onValueChange={(value) =>
							go({ ...search, metaLevel: value === 'all' ? null : value })}
					>
						<Select.Trigger class="w-full">
							{#if selectedLevel}
								<span class="flex w-full items-center gap-2">
									<span class="mr-auto">{selectedLevel.name}</span>
									{#each selectedLevel.groups as groupId (groupId)}
										<span
											class="size-2 rounded-full {META_GROUPS.find((g) => g.id === groupId)
												?.dotClass}"
										></span>
									{/each}
								</span>
							{:else}
								All
							{/if}
						</Select.Trigger>
						<Select.Content>
							<Select.Item value="all">All</Select.Item>
							{#each availableLevels as level (level.id)}
								<Select.Item value={String(level.id)}>
									<span class="flex w-full grow items-center gap-2">
										<span class="mr-auto block">{level.name}</span>
										{#each level.groups as groupId (groupId)}
											<span
												class="size-2 rounded-full {META_GROUPS.find((g) => g.id === groupId)
													?.dotClass}"
											></span>
										{/each}
									</span>
								</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>
			</div>

			<!-- The switch columns, with the stats flip on the market page. -->
			<div class="relative grid *:col-start-1 *:row-start-1">
				{#if showStats}
					<div class="p-4">
						<StatsStrip {stats} />
					</div>
				{:else}
					<div class="grid items-start gap-4 p-4 sm:grid-cols-3">
						{#if marketPage}
							<div>
								<h2 class="mb-2">Availability</h2>
								<div class="grid grid-cols-[auto_1fr] items-center gap-2">
									<Switch
										id="personal-modules"
										disabled={!signedIn}
										checked={search.withPersonalModules}
										onCheckedChange={(on) => go({ ...search, withPersonalModules: on })}
									/>
									<Label for="personal-modules">Personal modules</Label>
									<Switch
										id="only-contracts"
										checked={search.onlyContracts}
										onCheckedChange={(on) => go({ ...search, onlyContracts: on })}
									/>
									<Label for="only-contracts">Only contracts</Label>
									<Switch
										id="multi-item-contracts"
										checked={!search.noMultiItemContracts}
										onCheckedChange={(on) => go({ ...search, noMultiItemContracts: !on })}
									/>
									<Label for="multi-item-contracts">Multi-item contracts</Label>
									<Switch
										id="in-jita"
										checked={search.inJita}
										onCheckedChange={(on) => go({ ...search, inJita: on })}
									/>
									<Label for="in-jita">Jita 4-4</Label>
								</div>
							</div>
							<div>
								<h2 class="mb-2">Contract type</h2>
								<RadioGroup.Root
									value={search.contractType ?? ''}
									class="grid grid-cols-[auto_1fr] items-center gap-2"
									onValueChange={(value) =>
										go({ ...search, contractType: value === '' ? null : value })}
								>
									<RadioGroup.Item id="contract-all" value="" />
									<Label for="contract-all">All</Label>
									<RadioGroup.Item id="contract-exchange" value="item_exchange" />
									<Label for="contract-exchange">Item exchange</Label>
									<RadioGroup.Item id="contract-auction" value="auction" />
									<Label for="contract-auction">Auction</Label>
								</RadioGroup.Root>
							</div>
						{:else}
							<!-- All-modules: archive stats replace the market
							     columns (legacy TotalStats). -->
							<div class="sm:col-span-2">
								<StatsStrip {stats} />
							</div>
						{/if}
						<div>
							<h2 class="mb-2">Miscellaneous</h2>
							<div class="grid grid-cols-[auto_1fr] items-center gap-2">
								<Switch
									id="goldbar"
									checked={search.goldbar}
									onCheckedChange={(on) => go({ ...search, goldbar: on })}
								/>
								<Label for="goldbar">Gold bar rolls</Label>
								<Switch
									id="brownbar"
									checked={search.brownbar}
									onCheckedChange={(on) => go({ ...search, brownbar: on })}
								/>
								<Label for="brownbar">Brown bar rolls</Label>
								<Switch
									id="diamondbar"
									checked={search.diamondbar}
									onCheckedChange={(on) => go({ ...search, diamondbar: on })}
								/>
								<Label for="diamondbar">Diamond bar rolls</Label>
							</div>
						</div>
					</div>
				{/if}
				<Button
					title="Toggle stats"
					class="absolute top-1 right-1 z-20 justify-self-end"
					size="icon"
					variant="secondary"
					onclick={() => (showStats = !showStats)}
				>
					{#if showStats}
						<Funnel class="size-4" />
					{:else}
						<ChartColumn class="size-4" />
					{/if}
				</Button>
			</div>
		</div>
		<div class="divide-y divide-border">
			{#if marketPage}
				<CurrencyFilterRow {prefix} {search} kind="price" />
			{/if}
			<CurrencyFilterRow {prefix} {search} kind="value" />
		</div>
	</div>

	{#if panel !== null}
		{#key panel.type_id}
			<div class="relative p-0">
				<div class="grid gap-x-12 xl:grid-cols-2">
					{#each attributes as attribute (attribute.attribute_id)}
						<AttributeFilterRow
							{prefix}
							{search}
							{attribute}
							sourceTypes={panel.source_types}
						/>
					{/each}
					{#if attributes.length % 2 === 1}
						<div class="hidden p-4 xl:block">
							<div class="h-full w-full border border-dashed border-border"></div>
						</div>
					{/if}
				</div>
				<SourceTypeAttributeSelect {prefix} {search} {panel} />
			</div>
		{/key}
	{/if}
</div>
