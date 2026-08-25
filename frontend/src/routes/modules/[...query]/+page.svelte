<script lang="ts">
	import ModuleBrowser from '$lib/components/module-browser.svelte';
	import ModuleDetail from '$lib/components/module-detail.svelte';
	import type { BrowserData } from '$lib/server/browser';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
</script>

<svelte:head>
	<title>
		{data.module ? `${data.module.type.name} - MutaMarket` : 'MutaMarket - Abyssal Modules'}
	</title>
</svelte:head>

{#if data.module}
	<ModuleDetail
		module={data.module}
		statistic={data.estimatorStatistic ?? null}
		{settings}
	/>
{:else}
	<ModuleBrowser data={data as unknown as BrowserData} {settings} />
{/if}
