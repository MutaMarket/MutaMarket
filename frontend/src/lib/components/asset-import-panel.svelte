<script lang="ts">
	// The asset import panel, the legacy AssetImportStatus.vue family:
	// current import state and live progress. The state and its /ws
	// subscription live in the page (the Start Import button moved into
	// the page header and needs them too); this panel only renders.
	// Legacy wording is ported faithfully, including the "Your have"
	// typo of the empty state.
	import type { AssetImportView, PersonalPageData } from '$lib/types';

	let { data, current }: { data: PersonalPageData; current: AssetImportView | null } = $props();

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
</script>

<div class="p-4">
	<h2 class="mb-2">Asset Import</h2>
	<div>
		{#if current === null}
			<div class="my-2 text-sm text-muted-foreground">
				<p>Your have not imported any assets yet. Click the button below to start your first import.</p>
			</div>
		{:else if current.status === 'pending'}
			<div class="my-2 text-sm text-muted-foreground">
				<p class="animate-pulse">Your asset import has been queued. This may take a few minutes.</p>
			</div>
		{:else if current.status === 'processing'}
			<div class="grid grid-cols-[auto_1fr] items-center gap-2 text-sm">
				<svg class="size-3 animate-spin" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
					<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
					></circle>
					<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v4a4 4 0 00-4 4H4z"></path>
				</svg>
				{#if stepTexts[current.step]}
					<p class="col-start-2 row-start-1">{stepTexts[current.step]}</p>
				{:else}
					<div class="col-start-2 row-start-1">
						<p>
							Importing abyssal modules {current.abyssal_modules_imported_count}/{current.abyssal_modules_count}
						</p>
						<div class="mt-2 h-1 rounded-full bg-card">
							<div
								class="h-1 rounded-full bg-primary transition-[width] duration-1000"
								style="width: {progressPercent}%"
							></div>
						</div>
					</div>
				{/if}
			</div>
		{:else if current.status === 'completed'}
			<div class="my-2 text-sm text-muted-foreground">
				<p>
					We successfully imported {current.abyssal_modules_imported_count} modules from your assets
					{distanceStrict(current.updated_seconds_ago)} ago.
				</p>
			</div>
		{:else if current.status === 'failed'}
			<div class="my-2 text-sm text-muted-foreground">
				<p>
					Your import failed while we were trying to {failedActions[current.step] ??
						'import your assets'}.
				</p>
			</div>
		{/if}
		{#if !data.has_assets_scope}
			<!-- The legacy missing-scope notification, inlined; the grant
			     button sits in the page header. -->
			<div class="my-2 text-sm text-muted-foreground">
				<p>You need to grant the "Read Assets" ESI scope to import your personal modules.</p>
			</div>
		{/if}
	</div>
</div>
