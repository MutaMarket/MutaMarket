<script lang="ts">
	// The module show page, mirroring Pages/Modules/ShowModulePage.vue:
	// the card beside the hero on a 12-column grid. The tabs section
	// (source types, contract history, similar sold) arrives with its
	// backend features — specs/module-show.md §4.
	import ModuleCard from './module-card.svelte';
	import ModuleHero from './module-hero.svelte';
	import type { DisplaySettings } from '$lib/display';
	import type { EstimatorStatistic, ModuleDetail } from '$lib/types';

	let {
		module,
		statistic,
		settings
	}: {
		module: ModuleDetail;
		statistic: EstimatorStatistic | null;
		settings: DisplaySettings;
	} = $props();
</script>

<div class="grid grid-cols-12 gap-4">
	<div class="col-span-full md:col-span-4">
		<ModuleCard {module} {settings} />
	</div>
	<div class="col-span-full md:col-span-8">
		<ModuleHero {module} {statistic} />
	</div>
</div>
