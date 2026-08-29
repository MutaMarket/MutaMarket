<script lang="ts">
	// The filter band above the module grid, a compacted take on the
	// legacy Pages/ModulesFilters.vue composition (specs/browser-filters
	// .md §1): general filters, the type baseline and the toggle chips on
	// the left, price/value sliders on the right, the attribute grid
	// below. Deliberate divergences: the stats flip is dropped and the
	// switch columns are one chip line.
	import AttributeFilterRow from './attribute-filter-row.svelte';
	import CurrencyFilterRow from './currency-filter-row.svelte';
	import MatchTypeMenu from './match-type-menu.svelte';
	import TypeDialog from './type-dialog.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import { META_GROUPS, META_LEVELS } from '$lib/filter-meta';
	import { buildQueryPath, type UiSearch } from '$lib/query';
	import type { FilterPanelData } from '$lib/types';

	let {
		prefix,
		search,
		panel,
		unknownType,
		variant = 'market'
	}: {
		prefix: string;
		search: UiSearch;
		panel: FilterPanelData | null;
		unknownType: boolean;
		/** Which page hosts the band, mirroring the legacy per-page
		 * filter variants: `market` (browse), `archive` (all-modules),
		 * `character`, `collection`, `personal` and `calculator` (no
		 * market options, no bar chips, no sort, no virtual attributes,
		 * like the legacy CalculatorFilters). */
		variant?:
			| 'market'
			| 'archive'
			| 'historic'
			| 'character'
			| 'collection'
			| 'personal'
			| 'sell'
			| 'location'
			| 'calculator';
	} = $props();

	const marketPage = $derived(variant === 'market');
	const calculatorPage = $derived(variant === 'calculator');

	const signedIn = $derived(Boolean(page.data.nav?.user));

	function go(next: UiSearch) {
		// A filter change restarts at page 1, like the legacy builder.
		goto(buildQueryPath(prefix, { ...next, page: 1 }), { keepFocus: true, noScroll: true });
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
		(panel?.attributes ?? []).filter(
			(attribute) =>
				attribute.best !== attribute.worst && !(calculatorPage && attribute.is_virtual)
		)
	);

	/** The availability and bar filters as one row of toggle chips. */
	const chips = $derived.by(() => {
		const market = marketPage
			? [
					{
						label: 'Personal modules',
						on: search.withPersonalModules,
						disabled: !signedIn,
						title: signedIn ? undefined : 'Sign in to filter your own modules',
						toggle: () => go({ ...search, withPersonalModules: !search.withPersonalModules })
					},
					{
						label: 'Only contracts',
						on: search.onlyContracts,
						disabled: false,
						title: undefined,
						toggle: () => go({ ...search, onlyContracts: !search.onlyContracts })
					},
					{
						label: 'Multi-item contracts',
						on: !search.noMultiItemContracts,
						disabled: false,
						title: undefined,
						toggle: () => go({ ...search, noMultiItemContracts: !search.noMultiItemContracts })
					},
					{
						label: 'Jita 4-4',
						on: search.inJita,
						disabled: false,
						title: undefined,
						toggle: () => go({ ...search, inJita: !search.inJita })
					}
				]
			: [];
		const personal =
			variant === 'personal'
				? [
						{
							label: 'Without fitted',
							on: search.withoutFitted,
							disabled: false,
							title: 'Hide modules currently fitted to a ship',
							toggle: () => go({ ...search, withoutFitted: !search.withoutFitted })
						},
						{
							label: 'Without assets',
							on: search.withoutAssets,
							disabled: false,
							title: 'Hide modules sitting in your assets',
							toggle: () => go({ ...search, withoutAssets: !search.withoutAssets })
						}
					]
				: [];
		if (calculatorPage) {
			return [];
		}
		return [
			...market,
			...personal,
			{
				label: 'Gold bars',
				on: search.goldbar,
				disabled: false,
				title: undefined,
				toggle: () => go({ ...search, goldbar: !search.goldbar })
			},
			{
				label: 'Brown bars',
				on: search.brownbar,
				disabled: false,
				title: undefined,
				toggle: () => go({ ...search, brownbar: !search.brownbar })
			},
			{
				label: 'Diamond bars',
				on: search.diamondbar,
				disabled: false,
				title: undefined,
				toggle: () => go({ ...search, diamondbar: !search.diamondbar })
			}
		];
	});

	// Deliberate divergence from legacy: the hidden per-attribute type
	// dropdown and the floating center select were invisible affordances;
	// the match-a-type pill replaces both. The slider rows follow the URL
	// on their own.
	function applyBaseline(bounds: { name: string; lower: number; upper: null }[]) {
		goto(buildQueryPath(prefix, { ...search, attributes: bounds, page: 1 }), { noScroll: true });
	}

	/** The category button's look, shared by all three general-filter
	 * triggers so the row reads as one control set. */
	const TRIGGER_CLASS =
		'w-full data-[size=default]:h-10 rounded-md border-border bg-card-2 dark:bg-card-2 ' +
		'dark:hover:bg-card-2 px-3 text-sm transition hover:brightness-125';
</script>

<div class="hud-frame z-10 divide-y divide-border">
	<div
		class="grid divide-y divide-border {calculatorPage
			? ''
			: '2xl:grid-cols-[3fr_2fr] 2xl:divide-x 2xl:divide-y-0'}"
	>
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

			<!-- One compact line: the contract-type segments plus
			     availability/bar toggle chips. -->
			<div class="flex flex-wrap items-center gap-x-4 gap-y-2 p-4">
				{#if variant === 'character'}
					<!-- The legacy CharacterModuleAvailability radio. -->
					<div class="flex rounded-[7px] border border-border bg-card-2 p-0.5">
						{#each [[false, 'For sale'], [true, 'Created']] as [created, label] (label)}
							<button
								type="button"
								class="flex h-7 items-center rounded-[5px] px-2.5 text-xs transition-colors {search.created ===
								created
									? 'bg-primary text-primary-foreground'
									: 'text-muted-foreground hover:text-foreground'}"
								onclick={() => go({ ...search, created: created === true })}
							>
								{label}
							</button>
						{/each}
					</div>
				{/if}
				{#if marketPage}
						<div class="flex rounded-[7px] border border-border bg-card-2 p-0.5">
							{#each [[null, 'All'], ['item_exchange', 'Exchange'], ['auction', 'Auction']] as [value, label] (label)}
								<button
									type="button"
									class="flex h-7 items-center rounded-[5px] px-2.5 text-xs transition-colors {search.contractType ===
									value
										? 'bg-primary text-primary-foreground'
										: 'text-muted-foreground hover:text-foreground'}"
									onclick={() => go({ ...search, contractType: value })}
								>
									{label}
								</button>
							{/each}
						</div>
					{/if}
					{#if panel !== null}
					<MatchTypeMenu
						{panel}
						triggerClass="h-7 rounded-[7px] border border-border bg-card-2 px-2.5 text-xs transition hover:brightness-125"
						onApply={applyBaseline}
					/>
				{/if}
				{#each chips as chip (chip.label)}
						<button
							type="button"
							class="flex h-7 items-center gap-1.5 rounded-[7px] border px-2.5 text-xs transition-colors disabled:cursor-not-allowed disabled:opacity-40 {chip.on
								? 'border-primary/60 bg-primary/15 text-foreground'
								: 'border-border bg-card-2 text-muted-foreground hover:text-foreground'}"
							disabled={chip.disabled}
							title={chip.title}
							onclick={chip.toggle}
						>
							<span
								class="size-1.5 rounded-full {chip.on ? 'bg-primary' : 'bg-muted-foreground/40'}"
							></span>
							{chip.label}
						</button>
					{/each}
				</div>
		</div>
		{#if !calculatorPage}
			<div class="divide-y divide-border">
				<!-- The historic page filters the recorded sale price, the
				     legacy TrainingFilters' PriceFilter. -->
				{#if marketPage || variant === 'historic'}
					<CurrencyFilterRow {prefix} {search} kind="price" />
				{/if}
				<CurrencyFilterRow {prefix} {search} kind="value" />
			</div>
		{/if}
	</div>

	{#if panel !== null}
		{#key panel.type_id}
			<div class="relative p-0">
				<div class="grid gap-x-8 xl:grid-cols-2">
					{#each attributes as attribute (attribute.attribute_id)}
						<AttributeFilterRow
							{prefix}
							{search}
							{attribute}
							sourceTypes={panel.source_types}
							allowSort={!calculatorPage}
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
