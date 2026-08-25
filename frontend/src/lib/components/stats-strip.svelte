<script lang="ts">
	// Market-wide statistics above the browser grid, the legacy
	// getAllModulesStats strip.
	import type { ModulesStats } from '$lib/types';

	let { stats }: { stats: ModulesStats | null } = $props();

	/** A compact number for the stats strip, e.g. `12.3k`. */
	function compactCount(value: number): string {
		if (value >= 1_000_000) {
			return `${(value / 1_000_000).toFixed(1)}M`;
		}
		if (value >= 1_000) {
			return `${(value / 1_000).toFixed(1)}k`;
		}
		return String(value);
	}

	const cells = $derived(
		stats === null
			? []
			: ([
					['Total modules', stats.total_count],
					['For sale', stats.contracts_count],
					['Auctions', stats.auctions_count],
					['Added today', stats.added_last_day_count],
					['Gold bars', stats.goldbars_count],
					['Diamond bars', stats.diamondbars_count]
				] as const)
	);
</script>

{#if cells.length > 0}
	<div class="mb-4 grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
		{#each cells as [label, value] (label)}
			<div class="rounded-lg border border-border bg-card-1 px-3 py-2">
				<div class="text-sm font-semibold text-foreground">{compactCount(value)}</div>
				<div class="text-xs text-muted-foreground">{label}</div>
			</div>
		{/each}
	</div>
{/if}
