<script lang="ts">
	import ModuleBrowser from '$lib/components/module-browser.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	// svelte-ignore state_referenced_locally -- deliberate one-time seed
	const settings = $state({ ...data.displaySettings });
</script>

<svelte:head><title>All Modules - MutaMarket</title></svelte:head>

<ModuleBrowser {data} {settings} />
