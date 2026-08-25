<script lang="ts">
	// Placeholder for pages whose feature milestone has not landed yet in
	// the rewrite: the route exists and renders, the content is on its way.
	let { title }: { title: string } = $props();
</script>

<svelte:head><title>{title} - MutaMarket</title></svelte:head>

<section class="placeholder">
	<h1>{title}</h1>
	<p>This part of MutaMarket is being rebuilt and will be back shortly.</p>
</section>
