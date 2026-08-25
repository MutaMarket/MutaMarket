<script lang="ts">
	// The show-page tab strip, mirroring Show/ModuleTabs.vue: source
	// types / contract history / similar sold in a hud-panel, the active
	// tab persisted in the module_show_tab cookie. The similar-sold pane
	// arrives with its backend feature (specs/module-show.md §4.3).
	import { ChartColumn, FileClock, PackageCheck } from '@lucide/svelte';
	import ContractHistoryTable from './contract-history-table.svelte';
	import SourceTypesTable from './source-types-table.svelte';
	import * as Tabs from '$lib/components/ui/tabs';
	import type { HistoricContract, ModuleDetail, SourceTypeComparison } from '$lib/types';

	let {
		module,
		comparisons,
		historicContracts = [],
		initialTab = 'market'
	}: {
		module: ModuleDetail;
		comparisons: SourceTypeComparison[];
		historicContracts?: HistoricContract[];
		initialTab?: string;
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
</script>

<Tabs.Root value={activeTab} onValueChange={persistTab} class="hud-panel block">
	<Tabs.List>
		<Tabs.Trigger value="market">
			<ChartColumn class="size-4" />
			Source types
		</Tabs.Trigger>
		<Tabs.Trigger value="contracts">
			<FileClock class="size-4" />
			Contract history
		</Tabs.Trigger>
		<Tabs.Trigger value="similar">
			<PackageCheck class="size-4" />
			Similar sold
		</Tabs.Trigger>
	</Tabs.List>
	<Tabs.Content class="mt-0 [&_.border]:border-0 [&>div]:m-0" value="market">
		<SourceTypesTable {module} {comparisons} />
	</Tabs.Content>
	<Tabs.Content class="mt-0 [&_.border]:border-0 [&>div]:m-0" value="contracts">
		<ContractHistoryTable {contracts} />
	</Tabs.Content>
	<Tabs.Content class="mt-0" value="similar">
		<div class="p-4 text-sm text-muted-foreground">
			Similar sold modules are coming soon.
		</div>
	</Tabs.Content>
</Tabs.Root>
