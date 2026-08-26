<script lang="ts">
	// The filter band above the module grid, mirroring the legacy
	// Pages/ModulesFilters.vue composition (specs/browser-filters.md §1):
	// general filters and the switch columns (with the stats flip) on the
	// left, price/value sliders on the right, the attribute grid below.
	import { ChartColumn, Funnel } from '@lucide/svelte';
	import AttributeFilterRow from './attribute-filter-row.svelte';
	import CurrencyFilterRow from './currency-filter-row.svelte';
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

	// Deliberate divergence from legacy: the hidden per-attribute type
	// dropdown and the floating center select were invisible affordances;
	// one labeled baseline select over the attribute grid replaces both.
	// Applying bumps the stamp so the rows reseed from the new URL.
	let baselineStamp = $state(0);
	function applyBaseline(typeId: string) {
		const sourceType = panel?.source_types.find(
			(candidate) => candidate.id === Number(typeId)
		);
		if (!panel || !sourceType) {
			return;
		}
		const bounds = sourceType.attributes.flatMap((value) => {
			const attribute = panel.attributes.find(
				(candidate) => candidate.attribute_id === value.attribute_id
			);
			return attribute ? [{ name: attribute.name, lower: value.value, upper: null }] : [];
		});
		baselineStamp += 1;
		goto(buildQueryPath(prefix, { ...search, attributes: bounds }), { noScroll: true });
	}

	/** The category button's look, shared by all three general-filter
	 * triggers so the row reads as one control set. */
	const TRIGGER_CLASS =
		'w-full data-[size=default]:h-10 rounded-md border-border bg-card-2 dark:bg-card-2 ' +
		'dark:hover:bg-card-2 px-3 text-sm transition hover:brightness-125';
</script>

<div class="relative z-10 divide-y divide-border rounded-lg border border-border bg-card">
	<div class="grid divide-y divide-border 2xl:grid-cols-[3fr_2fr] 2xl:divide-x 2xl:divide-y-0">
		<div class="divide-y divide-border">
			<!-- GeneralFilter: type picker + narrowed meta selects. -->
			<div class="relative grid items-start gap-4 p-4 xl:grid-cols-3">
				<div>
					<h2 class="hud-label mb-2">Category</h2>
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
					<h2 class="hud-label mb-2">Meta group</h2>
					<Select.Root
						type="single"
						value={search.metaGroup ?? 'all'}
						onValueChange={(value) =>
							go({ ...search, metaGroup: value === 'all' ? null : value })}
					>
						<Select.Trigger class={TRIGGER_CLASS}>
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
					<h2 class="hud-label mb-2">Meta level</h2>
					<Select.Root
						type="single"
						value={search.metaLevel ?? 'all'}
						onValueChange={(value) =>
							go({ ...search, metaLevel: value === 'all' ? null : value })}
					>
						<Select.Trigger class={TRIGGER_CLASS}>
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
								<h2 class="hud-label mb-2">Availability</h2>
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
								<h2 class="hud-label mb-2">Contract type</h2>
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
							<h2 class="hud-label mb-2">Miscellaneous</h2>
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
		{#key `${panel.type_id}:${baselineStamp}`}
			<div class="relative p-0">
				<!-- The dedicated baseline area: pick a source type, get
				     "at least as good as it" bounds on every attribute. -->
				<div class="flex flex-wrap items-center justify-between gap-4 border-b border-border p-4">
					<h2 class="hud-label">Attributes</h2>
					<div class="flex items-center gap-3">
						<span class="text-xs text-muted-foreground">Set bounds from a type</span>
						<Select.Root type="single" value="" onValueChange={applyBaseline}>
							<Select.Trigger class="{TRIGGER_CLASS} w-72" data-testid="baseline-type">
								<span class="text-muted-foreground">Match a source type…</span>
							</Select.Trigger>
							<Select.Content>
								{#each panel.source_types as sourceType (sourceType.id)}
									<Select.Item value={String(sourceType.id)}>
										<span class="flex items-center gap-2 text-xs">
											<span
												class="size-2 rounded-full {META_GROUPS.find(
													(group) => group.id === sourceType.meta_group_id
												)?.dotClass ?? 'bg-gray-500'}"
											></span>
											{sourceType.name}
										</span>
									</Select.Item>
								{/each}
							</Select.Content>
						</Select.Root>
					</div>
				</div>
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
			</div>
		{/key}
	{/if}
</div>
