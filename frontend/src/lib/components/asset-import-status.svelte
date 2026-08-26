<script lang="ts">
	// The asset import readout of the personal page header, replacing the
	// legacy AssetImportStatus.vue sidebar panel: a fixed-footprint
	// status block plus the action button, so the header never shifts
	// when an import starts — the button just swaps to its busy state.
	import { LoaderCircle } from '@lucide/svelte';
	import type { AssetImportView, PersonalPageData } from '$lib/types';

	let { data, current }: { data: PersonalPageData; current: AssetImportView | null } = $props();

	const active = $derived(
		current !== null && current.status !== 'completed' && current.status !== 'failed'
	);

	const stepTexts: Record<string, string> = {
		fetching_assets: 'Fetching assets from ESI',
		fetching_asset_names: 'Fetching asset names from ESI',
		fetching_corporation_assets: 'Fetching corporation assets from ESI',
		fetching_corporation_asset_names: 'Fetching corporation asset names from ESI',
		searching_abyssal_modules: 'Searching for abyssal modules'
	};

	// The failed-step wording of the legacy FailedAssetImport.vue.
	const failedActions: Record<string, string> = {
		fetching_assets: 'fetch your assets from ESI',
		fetching_asset_names: 'fetch your asset names from ESI',
		fetching_corporation_assets: 'fetch your corporation assets from ESI',
		fetching_corporation_asset_names: 'fetch your corporation asset names from ESI',
		searching_abyssal_modules: 'search for abyssal modules',
		importing_abyssal_modules: 'import abyssal modules'
	};

	/** An approximation of date-fns formatDistanceToNowStrict. */
	function distanceStrict(seconds: number): string {
		const clamped = Math.max(seconds, 0);
		let amount: number;
		let unit: string;
		if (clamped < 60) {
			[amount, unit] = [clamped, 'second'];
		} else if (clamped < 3600) {
			[amount, unit] = [Math.floor(clamped / 60), 'minute'];
		} else if (clamped < 86_400) {
			[amount, unit] = [Math.floor(clamped / 3600), 'hour'];
		} else {
			[amount, unit] = [Math.floor(clamped / 86_400), 'day'];
		}
		return amount === 1 ? `1 ${unit}` : `${amount} ${unit}s`;
	}

	const progressPercent = $derived(
		current !== null && current.abyssal_modules_count > 0
			? (current.abyssal_modules_imported_count / current.abyssal_modules_count) * 100
			: 0
	);

	const statusLine = $derived.by(() => {
		if (!data.has_assets_scope) {
			return 'Grant the "Read Assets" ESI scope to import your modules';
		}
		if (current === null) {
			return 'No assets imported yet';
		}
		switch (current.status) {
			case 'pending':
				return 'Import queued · this may take a few minutes';
			case 'processing':
				return (
					stepTexts[current.step] ??
					`Importing modules ${current.abyssal_modules_imported_count}/${current.abyssal_modules_count}`
				);
			case 'completed':
				return `Imported ${current.abyssal_modules_imported_count} modules · ${distanceStrict(current.updated_seconds_ago)} ago`;
			default:
				return `Import failed while trying to ${failedActions[current.step] ?? 'import your assets'}`;
		}
	});

	const showBar = $derived(
		current !== null && current.status === 'processing' && !stepTexts[current.step]
	);
</script>

<div class="flex items-center gap-4">
	<div class="flex h-10 w-64 flex-col items-end justify-center gap-1.5">
		<p
			class="hud-label max-w-full truncate normal-case {current?.status === 'pending'
				? 'animate-pulse'
				: ''}"
			title={statusLine}
		>
			{statusLine}
		</p>
		<!-- The bar keeps its slot in every state so nothing jumps. -->
		<div class="h-1 w-full rounded-full {showBar ? 'bg-card' : 'bg-transparent'}">
			{#if showBar}
				<div
					class="h-1 rounded-full bg-primary transition-[width] duration-1000"
					style="width: {progressPercent}%"
				></div>
			{/if}
		</div>
	</div>
	{#if !data.has_assets_scope}
		<a
			href={data.grant_scope_url}
			rel="external"
			class="inline-flex h-10 w-36 items-center justify-center rounded-md bg-primary text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
		>
			Grant ESI scope
		</a>
	{:else}
		<form method="post" action="/personal/modules">
			<button
				type="submit"
				disabled={active}
				class="inline-flex h-10 w-36 items-center justify-center gap-2 rounded-md bg-primary text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:cursor-default disabled:opacity-60 disabled:hover:bg-primary"
			>
				{#if active}
					<LoaderCircle class="size-4 animate-spin" />
					Importing…
				{:else}
					Start Import
				{/if}
			</button>
		</form>
	{/if}
</div>
