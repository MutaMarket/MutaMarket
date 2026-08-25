<script lang="ts">
	// The module show page, mirroring Pages/Modules/ShowModulePage.vue:
	// the card beside the hero on a 12-column grid, the tab strip
	// (source types, contract history, similar sold) full-width below.
	import ModuleCard from './module-card.svelte';
	import ModuleHero from './module-hero.svelte';
	import ModuleTabs from './module-tabs.svelte';
	import type { DisplaySettings } from '$lib/display';
	import type {
		AbyssalTypeStatistic,
		EstimatorStatistic,
		HistoricContract,
		ModuleDetail,
		SourceTypeComparison
	} from '$lib/types';

	let {
		module,
		statistic,
		comparisons,
		historicContracts = [],
		typeStatistics = null,
		initialTab = 'market',
		settings
	}: {
		module: ModuleDetail;
		statistic: EstimatorStatistic | null;
		comparisons: SourceTypeComparison[];
		historicContracts?: HistoricContract[];
		/** Roll extremes feeding the card's search menus. */
		typeStatistics?: AbyssalTypeStatistic[] | null;
		initialTab?: string;
		settings: DisplaySettings;
	} = $props();
</script>

<div class="grid grid-cols-12 gap-4">
	<div class="col-span-full md:col-span-4">
		<ModuleCard {module} {settings} statistics={typeStatistics} />
	</div>
	<div class="col-span-full md:col-span-8">
		<ModuleHero {module} {statistic} />
	</div>
	<div class="col-span-full">
		<ModuleTabs {module} {comparisons} {historicContracts} {initialTab} {settings} />
	</div>
</div>
