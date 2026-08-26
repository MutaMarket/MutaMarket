<script lang="ts">
	// The personal modules page (legacy ShowAllPersonalModulesPage): the
	// filter band with the fitted/asset chips, the asset import panel and
	// the owned-module grid with locations.
	import AssetImportPanel from '$lib/components/asset-import-panel.svelte';
	import FilterBand from '$lib/components/filter-band.svelte';
	import ModuleDisplay from '$lib/components/module-display.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { toIskCompact } from '$lib/format-number';
	import { parseQueryUi } from '$lib/query';
	import type { AssetImportView } from '$lib/types';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
	const search = $derived(parseQueryUi(data.query));
	const activeCharacter = $derived(
		data.nav?.characters.find((character) => character.active) ?? null
	);

	// The live import state, shared by the header button and the panel
	// (the legacy AssetImportUpdated event on the user's channel,
	// replacing 2-second polling).
	let currentImport = $state<AssetImportView | null>(null);
	$effect(() => {
		currentImport = data.personal.asset_import;
	});
	$effect(() => {
		const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
		const socket = new WebSocket(`${scheme}://${location.host}/ws`);
		const channel = `Users.${data.personal.user_id}`;

		socket.onmessage = (event) => {
			try {
				const envelope = JSON.parse(event.data as string) as {
					channel: string;
					event: string;
					data: AssetImportView | null;
				};
				if (envelope.channel === channel && envelope.event === 'AssetImportUpdated') {
					currentImport = envelope.data;
				}
			} catch {
				// Not an envelope; ignore.
			}
		};

		return () => socket.close();
	});

	const importActive = $derived(
		currentImport !== null &&
			currentImport.status !== 'completed' &&
			currentImport.status !== 'failed'
	);
</script>

<svelte:head><title>Your Modules - MutaMarket</title></svelte:head>

<PageHeader
	title="Your Modules"
	subtitle={activeCharacter ? `Acting as ${activeCharacter.name}` : null}
	stats={[
		{
			label: 'Owned',
			value: data.personal.modules_count.toLocaleString('en-US'),
			accent: 'primary'
		},
		{ label: 'Est. value', value: toIskCompact(data.personal.estimated_value_total) }
	]}
>
	{#snippet icon()}
		{#if activeCharacter}
			<img
				alt=""
				class="size-10 rounded-lg"
				src="https://images.evetech.net/characters/{activeCharacter.id}/portrait?size=64"
			/>
		{/if}
	{/snippet}
	{#snippet actions()}
		{#if !importActive}
			{#if data.personal.has_assets_scope}
				<form method="post" action="/personal/modules">
					<button
						type="submit"
						class="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
					>
						Start Import
					</button>
				</form>
			{:else}
				<a
					href={data.personal.grant_scope_url}
					rel="external"
					class="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
				>
					Grant ESI scope
				</a>
			{/if}
		{/if}
	{/snippet}
</PageHeader>
<FilterBand
	prefix="personal/modules"
	{search}
	panel={data.panel}
	unknownType={data.unknownType}
	variant="personal"
/>
<div class="my-4 flex flex-col items-start gap-4 lg:grid lg:grid-cols-[280px_1fr]">
	<div class="w-full rounded-lg border border-border bg-card-1">
		<AssetImportPanel data={data.personal} current={currentImport} />
	</div>
	<div class="w-full">
		<ModuleDisplay
			entries={data.entries}
			{settings}
			panel={data.panel}
			{search}
			prefix="personal/modules"
		/>
	</div>
</div>
