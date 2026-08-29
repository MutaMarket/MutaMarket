<script lang="ts">
	// The show-page tab strip, mirroring Show/ModuleTabs.vue: source
	// types / contract history / similar sold in a hud-frame, the active
	// tab persisted in the module_show_tab cookie.
	import { ChartColumn, FileClock, PackageCheck } from '@lucide/svelte';
	import ContractHistoryTable from './contract-history-table.svelte';
	import SimilarModulesSold from './similar-modules-sold.svelte';
	import SourceTypesTable from './source-types-table.svelte';
	import * as Tabs from '$lib/components/ui/tabs';
	import type { DisplaySettings } from '$lib/display';
	import type { HistoricContract, ModuleDetail, SourceTypeComparison } from '$lib/types';

	let {
		module,
		comparisons,
		historicContracts = [],
		initialTab = 'market',
		settings
	}: {
		module: ModuleDetail;
		comparisons: SourceTypeComparison[];
		historicContracts?: HistoricContract[];
		initialTab?: string;
		settings: DisplaySettings;
	} = $props();

	// The live contract joins the archive as an outstanding row, newest
	// contract first (the legacy ModuleTabs contracts computed).
	const contracts = $derived.by(() => {
		const rows = [...historicContracts];
		if (module.contract) {
			rows.push({ ...module.contract, status: 'outstanding' });
		}
		return rows.sort((a, b) => b.id - a.id);
	});

	const TAB_VALUES = ['market', 'contracts', 'similar'];

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	let activeTab = $state(TAB_VALUES.includes(initialTab) ? initialTab : 'market');

	function persistTab(value: string) {
		activeTab = value;
		document.cookie = `module_show_tab=${value}; path=/; max-age=31536000; samesite=lax`;
	}

	// Styled like the segmented controls in the module options bar: a
	// bordered card-2 pill container, lime active segment.
	const TRIGGER =
		'h-7 flex-none gap-1.5 rounded-[5px] px-2.5 text-xs ' +
		'data-active:bg-primary data-active:text-primary-foreground ' +
		'dark:data-active:border-transparent dark:data-active:bg-primary ' +
		'dark:data-active:text-primary-foreground';
</script>

<Tabs.Root value={activeTab} onValueChange={persistTab} class="hud-frame block">
	<div class="p-3">
		<Tabs.List class="rounded-[7px] border border-border bg-card-2 p-0.5 dark:bg-card-2">
			<Tabs.Trigger value="market" class={TRIGGER}>
				<ChartColumn class="size-4" />
				Source types
			</Tabs.Trigger>
			<Tabs.Trigger value="contracts" class={TRIGGER}>
				<FileClock class="size-4" />
				Contract history
			</Tabs.Trigger>
			<Tabs.Trigger value="similar" class={TRIGGER}>
				<PackageCheck class="size-4" />
				Similar sold
			</Tabs.Trigger>
		</Tabs.List>
	</div>
	<Tabs.Content class="mt-0 [&_.border]:border-0 [&>div]:m-0" value="market">
		<SourceTypesTable {module} {comparisons} />
	</Tabs.Content>
	<Tabs.Content class="mt-0 [&_.border]:border-0 [&>div]:m-0" value="contracts">
		<ContractHistoryTable {contracts} />
	</Tabs.Content>
	<Tabs.Content class="mt-0" value="similar">
		<SimilarModulesSold {module} {settings} />
	</Tabs.Content>
</Tabs.Root>
