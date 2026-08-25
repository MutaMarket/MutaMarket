<script lang="ts">
	// The show-page hero, mirroring Show/ModuleHero.vue: creator details,
	// the estimator statistics sheet (or its missing-data state), and the
	// toolbar — in a hud-panel with the one-shot scan sweep.
	import ModuleToolbar from './module-toolbar.svelte';
	import GameImage from './game-image.svelte';
	import { relativeTime, parseDbTimestamp } from '$lib/duration';
	import { biasScore, scoreWord, starsValue } from '$lib/estimator-score';
	import { toIskCompact, toVeryCompact } from '$lib/format-number';
	import { notifySuccess } from '$lib/toast.svelte';
	import type { AbyssalTypeStatistic, EstimatorStatistic, ModuleDetail } from '$lib/types';

	let {
		module,
		statistic,
		typeStatistics = []
	}: {
		module: ModuleDetail;
		statistic: EstimatorStatistic | null;
		typeStatistics?: AbyssalTypeStatistic[];
	} = $props();

	let now = $state(Math.floor(Date.now() / 1000));
	$effect(() => {
		const tick = setInterval(() => (now = Math.floor(Date.now() / 1000)), 1000);
		return () => clearInterval(tick);
	});

	const trained = $derived(statistic !== null && statistic.r2 !== null && statistic.mae !== null);

	const confidence = $derived(
		statistic?.r2 == null ? null : scoreWord(starsValue(statistic.r2))
	);
	const trainingData = $derived(
		statistic?.data_statistics ? Object.entries(statistic.data_statistics) : []
	);
	const totalSamples = $derived(trainingData.reduce((sum, [, count]) => sum + (count ?? 0), 0));
	const bias = $derived(
		statistic?.data_statistics ? scoreWord(starsValue(biasScore(statistic.data_statistics))) : null
	);

	function agoLine(timestamp: string | null): string {
		if (timestamp === null) return '';
		return relativeTime(parseDbTimestamp(timestamp) - now);
	}

	async function copyEstimate() {
		if (module.estimated_value === null) return;
		await navigator.clipboard.writeText(toVeryCompact(module.estimated_value));
		notifySuccess('Copied to clipboard', 'Your estimated value has been copied to your clipboard.');
	}
</script>

<div class="hud-panel relative flex flex-col">
	<div aria-hidden="true" class="hud-scan pointer-events-none absolute inset-0"></div>

	<!-- CreatorDetails: linked portrait + name, gold for premium. -->
	<div class="border-b border-border">
		{#if module.creator}
			<a class="flex items-center gap-4 p-4" href="/characters/{module.creator.slug}">
				<GameImage
					src="https://images.evetech.net/characters/{module.creator.id}/portrait?size=64"
					alt={module.creator.name}
					class="h-10 w-10 rounded-lg"
				/>
				<div>
					<span class="block text-sm text-muted-foreground">Created by</span>
					<span class="font-medium {module.creator.has_premium ? 'text-gold' : ''}">
						{module.creator.name}
					</span>
				</div>
			</a>
		{/if}
	</div>

	{#if trained && statistic}
		<div class="flex flex-col">
			<!-- The AI value prediction block. -->
			<div class="flex grow flex-col gap-1.5 p-4">
				<h2 class="hud-label flex items-center gap-1.5" title="These models can be very inaccurate, so always do your own research by looking for similar modules on contracts.">
					AI value prediction
				</h2>
				<button class="cursor-pointer text-left" onclick={copyEstimate}>
					<span class="hud-readout text-2xl text-primary [text-shadow:0_0_18px_var(--glow)]">
						{toIskCompact(module.estimated_value)}
					</span>
					{#if statistic.nmae !== null}
						<span class="hud-readout ml-2 text-muted-foreground">
							±{statistic.nmae.toFixed(0)}%
						</span>
					{/if}
				</button>
				{#if module.estimated_value_updated_at}
					<p class="text-xs text-muted-foreground">
						Evaluated {agoLine(module.estimated_value_updated_at)}
					</p>
				{/if}
			</div>

			<!-- The model quality grid. -->
			<div class="grid grid-cols-2 border-t border-border sm:grid-cols-3">
				<div class="flex flex-col gap-1 p-4">
					<span class="hud-label">Confidence</span>
					<span class="hud-readout text-lg uppercase {confidence?.class}">
						{confidence?.label}
					</span>
					<span class="text-xs text-muted-foreground">R² {statistic.r2?.toFixed(2)}</span>
				</div>
				{#if bias}
					<div class="flex flex-col gap-1 border-l border-border p-4">
						<span class="hud-label">Bias score</span>
						<span class="hud-readout text-lg uppercase {bias.class}">{bias.label}</span>
						<span class="text-xs text-muted-foreground tabular-nums">
							{totalSamples.toLocaleString('en-US')}
							{totalSamples === 1 ? 'sample' : 'samples'}
						</span>
					</div>
				{/if}
				<div class="flex flex-col gap-1 border-t border-border p-4">
					<span class="hud-label">Avg. error (MAE)</span>
					<span class="hud-readout text-lg">±{toVeryCompact(statistic.mae ?? 0)}</span>
					<span class="text-xs text-muted-foreground">the lower, the better</span>
				</div>
				<div class="flex flex-col gap-1 border-t border-l border-border p-4">
					<span class="hud-label">Last trained</span>
					<span class="hud-readout text-lg">{agoLine(statistic.last_trained_at)}</span>
				</div>
				{#if trainingData.length > 0}
					<div
						class="flex flex-col gap-1 p-4 max-sm:col-span-2 max-sm:border-t sm:col-start-3 sm:row-span-2 sm:row-start-1 sm:border-l border-border"
					>
						<span class="hud-label">Training data</span>
						<div class="grid grow grid-cols-[1fr_auto] content-start gap-x-3 gap-y-0.5 text-xs">
							{#each trainingData as [typeName, count] (typeName)}
								<span class="truncate text-muted-foreground">{typeName}</span>
								<span
									class="text-right tabular-nums {(count ?? 0) < 10 ? 'text-negative' : ''}"
								>
									{(count ?? 0).toLocaleString('en-US')}
								</span>
							{/each}
						</div>
						<a
							class="flex items-center gap-1 text-xs text-primary hover:underline"
							href="/historic-sales/type/{module.type.id}"
						>
							View historic sales →
						</a>
					</div>
				{/if}
			</div>
		</div>
	{:else}
		<!-- The legacy MissingData state. -->
		<div class="p-2">
			<h2 class="text-sm">Missing data</h2>
			<h1 class="font-medium">No AI prediction available</h1>
			<p class="text-2xs">
				We are currently missing data to provide an AI prediction for this type of modules. We
				only recorded {statistic?.data_count ?? 0} trades so far (min. needed: 50)
			</p>
		</div>
	{/if}

	<ModuleToolbar {module} {typeStatistics} />
</div>
