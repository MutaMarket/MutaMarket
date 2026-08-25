<script lang="ts">
	// The filter sidebar ported from the Leptos FilterPanel (legacy
	// AttributeFilters.vue and friends). The URL is the single source of
	// truth: every control edits the current UiSearch and navigates to the
	// rebuilt query path.
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Input } from '$lib/components/ui/input';
	import * as Select from '$lib/components/ui/select';
	import RangeSlider from './range-slider.svelte';
	import TypeDialog from './type-dialog.svelte';
	import { formatValue, toNormalized, toOriginal } from '$lib/attributes';
	import { buildQueryPath, type UiSearch } from '$lib/query';
	import { cycleSort, sortDirection } from '$lib/sort';
	import type { FilterAttribute, FilterPanelData } from '$lib/types';

	let {
		prefix,
		search,
		panel,
		unknownType
	}: {
		prefix: string;
		search: UiSearch;
		/** The resolved type's slider bounds, when a type is selected. */
		panel: FilterPanelData | null;
		/** A type was requested but did not resolve. */
		unknownType: boolean;
	} = $props();

	function go(next: UiSearch) {
		goto(buildQueryPath(prefix, next), { keepFocus: true, noScroll: true });
	}

	function clearType() {
		go({ ...search, typeSlug: null, attributes: [], sort: null });
	}

	// --- Attribute sliders (legacy AttributeFilter.vue + AttributeMapper) --

	function sliderInitial(attribute: FilterAttribute): [number, number] {
		const active = search.attributes.find(
			(filter) => filter.name.toLowerCase() === attribute.name.toLowerCase()
		);
		if (!active) {
			return [0, 100];
		}
		const lower = Math.round(
			Math.min(Math.max(toNormalized(active.lower, attribute.best, attribute.worst), 0), 100)
		);
		const upper =
			active.upper === null
				? 100
				: Math.round(
						Math.min(Math.max(toNormalized(active.upper, attribute.best, attribute.worst), 0), 100)
					);
		return [Math.min(lower, upper), Math.max(lower, upper)];
	}

	function sliderLabel(attribute: FilterAttribute, normalized: number): string {
		return formatValue(
			toOriginal(normalized, attribute.best, attribute.worst),
			attribute.unit_name,
			attribute.unit_display_name
		);
	}

	function commitSlider(attribute: FilterAttribute, [lower, upper]: [number, number]) {
		const attributes = search.attributes.filter(
			(filter) => filter.name.toLowerCase() !== attribute.name.toLowerCase()
		);

		// The legacy search composable: a fully open slider means no filter;
		// an open upper end keeps only the lower bound (the backend resolves
		// its direction from high-is-good).
		if (lower !== 0 || upper !== 100) {
			const rawLower = toOriginal(lower, attribute.best, attribute.worst);
			const rawUpper = toOriginal(upper, attribute.best, attribute.worst);
			if (upper === 100) {
				attributes.push({ name: attribute.name, lower: rawLower, upper: null });
			} else {
				attributes.push({
					name: attribute.name,
					lower: Math.min(rawLower, rawUpper),
					upper: Math.max(rawLower, rawUpper)
				});
			}
		}

		go({ ...search, attributes });
	}

	// Slider thumb state per attribute, keyed by name and re-seeded when
	// the panel's type changes (a drag owns its position afterwards).
	let sliderValues: Record<string, [number, number]> = $state({});
	$effect(() => {
		const seeded: Record<string, [number, number]> = {};
		for (const attribute of panel?.attributes ?? []) {
			seeded[attribute.name] = sliderInitial(attribute);
		}
		sliderValues = seeded;
	});

	// --- Sort, contracts, bounds, meta group ------------------------------

	const sortArrow = (direction: boolean | null) =>
		direction === null ? '' : direction ? ' ↓' : ' ↑';

	function toggleSort(field: string) {
		const next = cycleSort(sortDirection(search.sort, field));
		go({ ...search, sort: next === null ? null : [field, next] });
	}

	function setContractType(value: string | null) {
		go({ ...search, contractType: value });
	}

	const flags: [string, keyof UiSearch][] = [
		['For sale only', 'onlyContracts'],
		['No multi-item contracts', 'noMultiItemContracts'],
		['Without other items', 'withoutOtherItems'],
		['Gold bar rolls', 'goldbar'],
		['Diamond bar rolls', 'diamondbar'],
		['Brown bar rolls', 'brownbar']
	];

	function setFlag(flag: keyof UiSearch, on: boolean) {
		go({ ...search, [flag]: on });
	}

	// Two optional numeric bounds committed on change (price and value).
	let priceLower = $state('');
	let priceUpper = $state('');
	let valueLower = $state('');
	let valueUpper = $state('');
	$effect(() => {
		priceLower = search.price === null ? '' : String(search.price[0]);
		priceUpper = search.price?.[1] === null || search.price === null ? '' : String(search.price[1]);
		valueLower = search.value === null ? '' : String(search.value[0]);
		valueUpper = search.value?.[1] === null || search.value === null ? '' : String(search.value[1]);
	});

	function parseBounds(lowerText: string, upperText: string): [number, number | null] | null {
		const lower = lowerText === '' ? NaN : Number(lowerText);
		const upper = upperText === '' ? NaN : Number(upperText);
		if (!Number.isNaN(lower) && !Number.isNaN(upper)) {
			return [Math.min(lower, upper), Math.max(lower, upper)];
		}
		if (!Number.isNaN(lower)) {
			return [lower, null];
		}
		// A single maximum keeps the legacy single-value price semantics
		// only through both bounds, so keep it as a range from zero.
		if (!Number.isNaN(upper)) {
			return [0, upper];
		}
		return null;
	}

	const metaGroups: [string, string][] = [
		['t1', 'Tech I'],
		['t2', 'Tech II'],
		['storyline', 'Storyline'],
		['faction', 'Faction'],
		['officer', 'Officer'],
		['deadspace', 'Deadspace']
	];
	const metaGroupLabel = $derived(
		metaGroups.find(([key]) => key === search.metaGroup)?.[1] ?? 'All meta groups'
	);
</script>

<aside class="flex flex-col gap-4 rounded-lg border border-border bg-card-1 p-3">
	<section>
		<h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
			Category
		</h2>
		<TypeDialog {prefix} {search} currentTypeId={panel?.type_id} currentTypeName={panel?.type_name} />
		{#if panel}
			<div class="mt-3 mb-1 flex items-center justify-between">
				<h3 class="text-sm font-medium text-foreground">{panel.type_name}</h3>
				<button class="text-xs text-muted-foreground hover:text-foreground" onclick={clearType}>
					Clear type
				</button>
			</div>
			<div class="flex flex-col gap-3">
				{#each panel.attributes as attribute (attribute.attribute_id)}
					{#if sliderValues[attribute.name]}
						<div>
							<div class="mb-1 flex items-center justify-between text-xs">
								<span class="text-muted-foreground">
									{attribute.display_name === '' ? attribute.name : attribute.display_name}
								</span>
								<span class="text-foreground">
									{sliderLabel(attribute, sliderValues[attribute.name][0])} - {sliderLabel(
										attribute,
										sliderValues[attribute.name][1]
									)}
								</span>
							</div>
							<RangeSlider
								bind:values={sliderValues[attribute.name]}
								onCommit={(values) => commitSlider(attribute, values)}
							/>
						</div>
					{/if}
				{/each}
			</div>
		{:else if unknownType}
			<p class="mt-2 text-xs text-muted-foreground">Unknown type.</p>
		{/if}
	</section>

	<section>
		<h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Sort</h2>
		<div class="flex flex-wrap gap-1">
			{#each [['price', 'Price'], ['value', 'Est. value'], ['fraction', 'Roll quality']] as [field, label] (field)}
				<Button
					variant={sortDirection(search.sort, field) === null ? 'outline' : 'default'}
					size="sm"
					class="h-7 px-2 text-xs"
					onclick={() => toggleSort(field)}
				>
					{label}{sortArrow(sortDirection(search.sort, field))}
				</Button>
			{/each}
		</div>
	</section>

	<section>
		<h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
			Contracts
		</h2>
		<div class="flex flex-col gap-2">
			<div class="flex gap-1">
				{#each [[null, 'Any'], ['item_exchange', 'Item exchange'], ['auction', 'Auction']] as [value, label] (label)}
					<Button
						variant={search.contractType === value ? 'default' : 'outline'}
						size="sm"
						class="h-7 px-2 text-xs"
						onclick={() => setContractType(value)}
					>
						{label}
					</Button>
				{/each}
			</div>
			{#each flags as [label, flag] (flag)}
				<label class="flex items-center gap-2 text-xs text-muted-foreground">
					<Checkbox
						checked={Boolean(search[flag])}
						aria-label={label}
						onCheckedChange={(on) => setFlag(flag, on === true)}
					/>
					{label}
				</label>
			{/each}
		</div>
	</section>

	<section>
		<h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Price</h2>
		<div class="flex gap-2">
			<Input
				class="h-8 text-xs"
				placeholder="Min price"
				inputmode="decimal"
				bind:value={priceLower}
				onchange={() => go({ ...search, price: parseBounds(priceLower, priceUpper) })}
			/>
			<Input
				class="h-8 text-xs"
				placeholder="Max price"
				inputmode="decimal"
				onchange={() => go({ ...search, price: parseBounds(priceLower, priceUpper) })}
				bind:value={priceUpper}
			/>
		</div>
	</section>

	<section>
		<h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
			Estimated value
		</h2>
		<div class="flex gap-2">
			<Input
				class="h-8 text-xs"
				placeholder="Min value"
				inputmode="decimal"
				bind:value={valueLower}
				onchange={() => go({ ...search, value: parseBounds(valueLower, valueUpper) })}
			/>
			<Input
				class="h-8 text-xs"
				placeholder="Max value"
				inputmode="decimal"
				bind:value={valueUpper}
				onchange={() => go({ ...search, value: parseBounds(valueLower, valueUpper) })}
			/>
		</div>
	</section>

	<section>
		<h2 class="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
			Meta group
		</h2>
		<Select.Root
			type="single"
			value={search.metaGroup ?? 'all'}
			onValueChange={(value) => go({ ...search, metaGroup: value === 'all' ? null : value })}
		>
			<Select.Trigger class="w-full">{metaGroupLabel}</Select.Trigger>
			<Select.Content>
				<Select.Item value="all">All meta groups</Select.Item>
				{#each metaGroups as [key, label] (key)}
					<Select.Item value={key}>{label}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
	</section>
</aside>
