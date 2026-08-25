<script lang="ts">
	import { page } from '$app/state';
</script>

<svelte:head><title>MutaMarket</title></svelte:head>

{#if page.status === 404}
	<h1>Page not found</h1>
{:else}
	<h1>{page.error?.message ?? 'Something went wrong'}</h1>
{/if}
