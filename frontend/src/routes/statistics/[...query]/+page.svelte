<script lang="ts">
	// The unified statistics page, replacing the legacy split between
	// /statistics (top creators) and /personal/stats: the market
	// overview up top, the creator leaderboard with its type/name
	// filters, and the signed-in user's own creation stats below.
	import { ArrowUpDown, LogIn } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import GameImage from '$lib/components/game-image.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import TypeDialog from '$lib/components/type-dialog.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Table from '$lib/components/ui/table';
	import { toIskCompact } from '$lib/format-number';
	import { buildQueryPath, moduleSlug, parseQueryUi } from '$lib/query';
	import {
		filterPersonalRows,
		pageCount,
		sortPersonalRows,
		type PersonalSortKey,
		type TopCharacterRow
	} from '$lib/statistics';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const search = $derived(parseQueryUi(data.query));
	const signedIn = $derived(Boolean(page.data.nav?.user));

	/** The legacy search debounce before navigating. */
	const SEARCH_DEBOUNCE_MS = 500;

	// The leaderboard's name filter follows the URL; typing debounces
	// into a navigation so results stay shareable.
	// svelte-ignore state_referenced_locally -- seeded, then synced below
	let nameInput = $state(data.name);
	$effect(() => {
		nameInput = data.name;
	});
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	function navigate(next: {
		page?: number;
		name?: string;
		sortField?: string | null;
		sortDirection?: string | null;
	}) {
		const path = buildQueryPath('statistics', { ...search, page: next.page ?? 1 });
		const params = new URLSearchParams();
		const name = next.name ?? nameInput;
		if (name.trim() !== '') {
			params.set('name', name.trim());
		}
		const sortField = next.sortField === undefined ? data.sortField : next.sortField;
		const sortDirection =
			next.sortDirection === undefined ? data.sortDirection : next.sortDirection;
		if (sortField) {
			params.set('sort_field', sortField);
			params.set('sort_direction', sortDirection ?? 'asc');
		}
		const suffix = params.size > 0 ? `?${params}` : '';
		goto(`${path}${suffix}`, { keepFocus: true, noScroll: true });
	}

	function onNameInput() {
		if (debounceTimer !== null) {
			clearTimeout(debounceTimer);
		}
		debounceTimer = setTimeout(() => navigate({ name: nameInput }), SEARCH_DEBOUNCE_MS);
	}

	// The legacy SortableHeader: same field flips the direction, a new
	// field starts descending.
	function sortBy(field: 'name' | 'rank_number') {
		if (data.sortField === field) {
			navigate({
				sortField: field,
				sortDirection: data.sortDirection === 'asc' ? 'desc' : 'asc',
				page: data.top.meta.current_page
			});
		} else {
			navigate({ sortField: field, sortDirection: 'desc', page: data.top.meta.current_page });
		}
	}

	const totalPages = $derived(pageCount(data.top.meta.total, data.top.meta.per_page));

	function characterHref(row: TopCharacterRow): string {
		return `/characters/${moduleSlug(row.name, row.id)}`;
	}

	/** The legacy leaderboard count link: that character's created
	 * modules, keeping the page's type scope. */
	function createdModulesHref(row: TopCharacterRow): string {
		const base = `characters/${moduleSlug(row.name, row.id)}`;
		return buildQueryPath(base, {
			...parseQueryUi(''),
			typeSlug: search.typeSlug,
			created: true
		});
	}

	// The personal table searches and sorts client-side, the legacy
	// BaseTable behavior.
	let personalNeedle = $state('');
	let personalSort = $state<PersonalSortKey>('count');
	let personalAscending = $state(false);
	const personalRows = $derived(
		data.personal === null
			? []
			: sortPersonalRows(
					filterPersonalRows(data.personal.stats, personalNeedle),
					personalSort,
					personalAscending
				)
	);
	function sortPersonalBy(key: PersonalSortKey) {
		if (personalSort === key) {
			personalAscending = !personalAscending;
		} else {
			personalSort = key;
			personalAscending = key !== 'count';
		}
	}

	const PERSONAL_COLUMNS: { key: PersonalSortKey; label: string }[] = [
		{ key: 'type', label: 'Type' },
		{ key: 'creator', label: 'Creator' },
		{ key: 'count', label: 'Count' }
	];

	const overviewTiles = $derived([
		{ label: 'For sale', value: data.overview.stats.listed_count },
		{ label: 'Contracts', value: data.overview.stats.contracts_count },
		{ label: 'Item exchanges', value: data.overview.stats.item_exchanges_count },
		{ label: 'Auctions', value: data.overview.stats.auctions_count },
		{ label: 'Goldbars', value: data.overview.stats.goldbars_count },
		{ label: 'Brownbars', value: data.overview.stats.brownbars_count },
		{ label: 'Diamondbars', value: data.overview.stats.diamondbars_count },
		{ label: 'Added last hour', value: data.overview.stats.added_last_hour_count },
		{ label: 'Added last day', value: data.overview.stats.added_last_day_count },
		{ label: 'Added last week', value: data.overview.stats.added_last_week_count },
		{ label: 'Known creators', value: data.overview.creators_count },
		{ label: 'Tracked characters', value: data.overview.characters_count }
	]);

	const SORT_BUTTON =
		'inline-flex items-center gap-1.5 whitespace-nowrap transition-colors hover:text-foreground';
</script>

<svelte:head><title>Statistics - MutaMarket</title></svelte:head>

<PageHeader
	title="Statistics"
	subtitle="The abyssal market at a glance, its top creators, and your own numbers"
	stats={[
		{
			label: 'Modules in database',
			value: data.overview.stats.total_count.toLocaleString('en-US'),
			accent: 'primary'
		},
		{ label: 'Total value', value: toIskCompact(data.overview.total_value) },
		{ label: 'Average value', value: toIskCompact(data.overview.average_value) }
	]}
/>

<!-- Market overview -->
<div class="grid grid-cols-2 gap-3 md:grid-cols-4 xl:grid-cols-6">
	{#each overviewTiles as tile (tile.label)}
		<div class="hud-panel p-4">
			<h3 class="hud-label">{tile.label}</h3>
			<div class="mt-1 text-xl font-semibold tabular-nums">
				{tile.value.toLocaleString('en-US')}
			</div>
		</div>
	{/each}
</div>

<!-- Top creators -->
<section class="mt-8">
	<h2 class="mb-3 text-lg font-semibold">Top Characters</h2>
	<div class="rounded-lg border border-border bg-card">
		<div class="grid items-end gap-4 border-b border-border p-4 md:grid-cols-3">
			<div>
				<h3 class="hud-label mb-2">Category</h3>
				<TypeDialog prefix="statistics" {search} />
			</div>
			<label class="grid gap-2">
				<span class="hud-label">Search by name</span>
				<Input
					type="search"
					placeholder="Search statistics..."
					bind:value={nameInput}
					oninput={onNameInput}
				/>
			</label>
			<div class="flex items-center justify-end gap-2 pb-1">
				<Button
					variant="outline"
					size="sm"
					disabled={data.top.meta.current_page <= 1}
					onclick={() => navigate({ page: data.top.meta.current_page - 1 })}
				>
					Previous
				</Button>
				<span class="text-sm whitespace-nowrap text-muted-foreground tabular-nums">
					Page {data.top.meta.current_page} / {totalPages}
				</span>
				<Button
					variant="outline"
					size="sm"
					disabled={data.top.meta.current_page >= totalPages}
					onclick={() => navigate({ page: data.top.meta.current_page + 1 })}
				>
					Next
				</Button>
			</div>
		</div>
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head class="w-16">
						<button type="button" class={SORT_BUTTON} onclick={() => sortBy('rank_number')}>
							#
							<ArrowUpDown class="size-3.5 opacity-60" />
						</button>
					</Table.Head>
					<Table.Head>
						<button type="button" class={SORT_BUTTON} onclick={() => sortBy('name')}>
							Character
							<ArrowUpDown class="size-3.5 opacity-60" />
						</button>
					</Table.Head>
					<Table.Head class="text-right">Modules created</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each data.top.data as row (row.id)}
					<Table.Row>
						<Table.Cell class="font-medium tabular-nums">{row.rank_number}</Table.Cell>
						<Table.Cell>
							<a href={characterHref(row)} class="flex items-center gap-2 hover:underline">
								<GameImage
									src="https://images.evetech.net/characters/{row.id}/portrait?size=64"
									alt={row.name}
									class="size-8 rounded-md"
								/>
								<span class="whitespace-nowrap">{row.name}</span>
							</a>
						</Table.Cell>
						<Table.Cell class="text-right">
							<a
								href={createdModulesHref(row)}
								class="rounded-md px-2 py-1 tabular-nums hover:bg-card-2"
							>
								{row.modules_created_count.toLocaleString('en-US')}
							</a>
						</Table.Cell>
					</Table.Row>
				{:else}
					<Table.Row>
						<Table.Cell colspan={3} class="py-8 text-center text-muted-foreground">
							No creators match your search.
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	</div>
</section>

<!-- Personal statistics -->
<section class="mt-8 mb-4">
	<h2 class="mb-3 text-lg font-semibold">Your Statistics</h2>
	{#if data.personal !== null}
		<div class="mb-4 grid gap-4 lg:grid-cols-3">
			<div class="hud-panel p-6">
				<h3 class="hud-label">Modules created</h3>
				<div class="mt-2 text-4xl font-semibold tabular-nums">
					{data.personal.total_modules.toLocaleString('en-US')}
				</div>
				<p class="mt-2 text-sm text-muted-foreground">
					The total amount of modules you have created with all your characters.
				</p>
			</div>
			<div class="hud-panel p-6">
				<h3 class="hud-label">Money spent</h3>
				<div class="mt-2 text-4xl font-semibold tabular-nums">
					{toIskCompact(data.personal.total_spent)}
				</div>
				<p class="mt-2 text-sm text-muted-foreground">
					The total amount of ISK spent on creating modules with today's prices (avg. in
					Jita).
				</p>
			</div>
			<div class="hud-panel p-6">
				<h3 class="hud-label">Total value</h3>
				<div class="mt-2 text-4xl font-semibold tabular-nums">
					{toIskCompact(data.personal.total_value)}
				</div>
				<p class="mt-2 text-sm text-muted-foreground">
					The total value of all modules you created with all your characters.
				</p>
			</div>
		</div>
		<div class="rounded-lg border border-border bg-card">
			<div class="grid items-center gap-3 border-b border-border p-4 md:grid-cols-3">
				<label class="grid gap-1">
					<span class="hud-label">Search stats</span>
					<Input type="search" placeholder="Search stats..." bind:value={personalNeedle} />
				</label>
				<p class="text-center text-sm text-balance text-muted-foreground italic md:col-start-3">
					Keep in mind that these stats are based on the average prices in Jita, and may
					not reflect the actual value of the modules and their source materials.
				</p>
			</div>
			<Table.Root>
				<Table.Header>
					<Table.Row>
						{#each PERSONAL_COLUMNS as column (column.key)}
							<Table.Head class={column.key === 'count' ? 'text-right' : ''}>
								<button
									type="button"
									class="{SORT_BUTTON} {personalSort === column.key ? 'text-foreground' : ''}"
									onclick={() => sortPersonalBy(column.key)}
								>
									{column.label}
									<ArrowUpDown class="size-3.5 opacity-60" />
								</button>
							</Table.Head>
						{/each}
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each personalRows as row (`${row.type.id}-${row.creator.id}`)}
						<Table.Row>
							<Table.Cell>
								<div class="flex items-center gap-2">
									<GameImage
										src="https://images.evetech.net/types/{row.type.id}/icon?size=32"
										alt={row.type.name}
										class="size-8 rounded-md"
									/>
									<span class="whitespace-nowrap">{row.type.name}</span>
								</div>
							</Table.Cell>
							<Table.Cell>
								<a
									href="/characters/{moduleSlug(row.creator.name, row.creator.id)}"
									class="flex items-center gap-2 hover:underline"
								>
									<GameImage
										src="https://images.evetech.net/characters/{row.creator.id}/portrait?size=64"
										alt={row.creator.name}
										class="size-8 rounded-md"
									/>
									<span class="whitespace-nowrap">{row.creator.name}</span>
								</a>
							</Table.Cell>
							<Table.Cell class="text-right tabular-nums">
								{row.count.toLocaleString('en-US')}
							</Table.Cell>
						</Table.Row>
					{:else}
						<Table.Row>
							<Table.Cell colspan={3} class="py-8 text-center text-muted-foreground">
								{data.personal.stats.length === 0
									? 'No created modules yet.'
									: 'No rows match your search.'}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
	{:else}
		<div
			class="flex items-center justify-center gap-4 rounded-lg border border-dashed border-border p-8"
		>
			<LogIn class="size-6 text-muted-foreground" />
			<span class="text-muted-foreground">
				{signedIn
					? 'Your personal statistics could not be loaded.'
					: 'Sign in to see your own creation statistics.'}
			</span>
		</div>
	{/if}
</section>
