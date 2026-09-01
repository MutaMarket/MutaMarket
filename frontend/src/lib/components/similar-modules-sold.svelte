<script lang="ts">
	// The similar-sold pane, mirroring Modules/SimilarModulesSold.vue:
	// premium accounts get a deferred-loaded stat strip plus the nearest
	// sold rolls as full module cards; everyone else the blurred teaser
	// under the upgrade card.
	import { Check, MoveRight } from '@lucide/svelte';
	import ModuleCard from './module-card.svelte';
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button';
	import type { DisplaySettings } from '$lib/display';
	import { toIskCompact } from '$lib/format-number';
	import { teaserModules } from '$lib/teaser-modules';
	import type { ModuleDetail } from '$lib/types';

	let { module, settings }: { module: ModuleDetail; settings: DisplaySettings } = $props();

	const hasPremium = $derived(Boolean(page.data.nav?.user?.has_premium));

	let similar: ModuleDetail[] | null = $state(null);
	let hoveredStat: 'lowest' | 'highest' | null = $state(null);

	$effect(() => {
		if (!hasPremium) {
			return;
		}
		similar = null;
		fetch(`/api/module-page/${module.slug}/similar`)
			.then((response) => response.json())
			.then((data) => {
				similar = data.similar_modules ?? [];
			})
			.catch(() => {
				similar = [];
			});
	});

	const stats = $derived.by(() => {
		const entries = (similar ?? [])
			.map((entry) => ({ id: entry.id, price: Number(entry.training_module?.sold_for) }))
			.filter((entry) => Number.isFinite(entry.price) && entry.price > 0);
		if (entries.length === 0) {
			return null;
		}
		const sum = entries.reduce((total, entry) => total + entry.price, 0);
		const lowest = entries.reduce((a, b) => (b.price < a.price ? b : a));
		const highest = entries.reduce((a, b) => (b.price > a.price ? b : a));
		return {
			average: sum / entries.length,
			lowest: lowest.price,
			highest: highest.price,
			lowest_id: lowest.id,
			highest_id: highest.id,
		};
	});

	const highlightedId = $derived.by(() => {
		if (!stats || !hoveredStat) {
			return null;
		}
		return hoveredStat === 'lowest' ? stats.lowest_id : stats.highest_id;
	});

	const statSkeletons = [
		{ label: '52px', value: '88px' },
		{ label: '44px', value: '64px' },
		{ label: '60px', value: '104px' },
	];

	const teaserStats = [
		{ label: 'Average', value: '142 million ISK', class: '' },
		{ label: 'Lowest', value: '98 million ISK', class: 'text-positive' },
		{ label: 'Highest', value: '215 million ISK', class: 'text-negative' },
	];

	const teasers = $derived(teaserModules(module));
</script>

{#if hasPremium}
	{#if similar === null}
		<!-- The legacy Deferred fallback skeleton. -->
		<div class="flex flex-wrap border-b border-border">
			{#each statSkeletons as skeleton, index (index)}
				<div class="w-40 animate-pulse p-4 {index > 0 ? 'border-l border-border' : ''}">
					<div class="mb-2 h-3 rounded bg-white/25" style="width: {skeleton.label}"></div>
					<div class="h-5 rounded bg-white/25" style="width: {skeleton.value}"></div>
				</div>
			{/each}
		</div>
		<div class="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4 p-4">
			{#each Array.from({ length: 8 }), index (index)}
				<div
					class="animate-pulse rounded-lg border border-border bg-muted/50"
					style="height: 280px"
				></div>
			{/each}
		</div>
	{:else}
		{#if stats}
			<div class="flex flex-wrap items-stretch border-b border-border">
				<div class="flex flex-col gap-1 p-4">
					<span class="hud-label">Average</span>
					<span class="hud-readout text-lg whitespace-nowrap">
						{toIskCompact(stats.average)}
					</span>
				</div>
				<div
					class="flex cursor-default flex-col gap-1 border-l border-border p-4 transition-colors hover:bg-positive/5"
					onmouseenter={() => (hoveredStat = 'lowest')}
					onmouseleave={() => (hoveredStat = null)}
					role="presentation"
				>
					<span class="hud-label">Lowest</span>
					<span class="hud-readout text-lg whitespace-nowrap text-positive">
						{toIskCompact(stats.lowest)}
					</span>
				</div>
				<div
					class="flex cursor-default flex-col gap-1 border-l border-border p-4 transition-colors hover:bg-negative/5"
					onmouseenter={() => (hoveredStat = 'highest')}
					onmouseleave={() => (hoveredStat = null)}
					role="presentation"
				>
					<span class="hud-label">Highest</span>
					<span class="hud-readout text-lg whitespace-nowrap text-negative">
						{toIskCompact(stats.highest)}
					</span>
				</div>
				<div class="flex grow items-center justify-end p-3">
					<Button variant="outline" href="/historic-sales/type/{module.type.id}">
						View historic sales
						<MoveRight class="size-4" />
					</Button>
				</div>
			</div>
		{/if}
		{#if similar.length > 0}
			<div class="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4 p-4">
				{#each similar as sold (sold.id)}
					<div
						class="rounded-lg ring-2 transition-all {highlightedId === sold.id
							? hoveredStat === 'lowest'
								? 'ring-emerald-500'
								: 'ring-rose-500'
							: 'ring-transparent'}"
					>
						<ModuleCard module={sold} {settings} />
					</div>
				{/each}
			</div>
		{:else}
			<p class="p-4 text-sm text-muted-foreground">No similar modules with historic sales found.</p>
		{/if}
	{/if}
{:else}
	<div class="relative overflow-hidden">
		<div aria-hidden="true" class="pointer-events-none blur-[14px] select-none">
			<div class="flex flex-wrap items-stretch border-b border-border">
				{#each teaserStats as stat (stat.label)}
					<div class="flex flex-col gap-1 p-4 not-first:border-l not-first:border-border">
						<span class="hud-label">{stat.label}</span>
						<span class="hud-readout text-lg whitespace-nowrap {stat.class}">
							{stat.value}
						</span>
					</div>
				{/each}
			</div>
			<div class="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4 p-4">
				{#each teasers as teaser (teaser.id)}
					<ModuleCard module={teaser} {settings} />
				{/each}
			</div>
		</div>
		<div class="absolute inset-0 z-10 flex items-center justify-center p-6">
			<div class="w-full max-w-sm border border-border bg-card/90 p-6 shadow-xl backdrop-blur-sm">
				<span class="font-mono text-2xs tracking-[0.12em] text-primary uppercase">Premium</span>
				<h3 class="mt-2 text-xl font-semibold">See what similar modules sold for</h3>
				<ul class="mt-4 space-y-2.5 text-sm">
					<li class="flex gap-2.5">
						<Check class="mt-0.5 size-4 shrink-0 text-primary" />
						<span>Modules with rolls like this one and the prices they actually sold for</span>
					</li>
					<li class="flex gap-2.5">
						<Check class="mt-0.5 size-4 shrink-0 text-primary" />
						<span>Average, lowest and highest sale price at a glance</span>
					</li>
					<li class="flex gap-2.5">
						<Check class="mt-0.5 size-4 shrink-0 text-primary" />
						<span>Historic sales for every module type</span>
					</li>
				</ul>
				<Button class="mt-6 w-full" href="/premium">Upgrade to Premium</Button>
			</div>
		</div>
	</div>
{/if}
