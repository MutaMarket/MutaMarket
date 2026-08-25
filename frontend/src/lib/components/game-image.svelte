<script lang="ts">
	// An image that degrades to the legacy Fallback square (bordered,
	// rounded, muted question mark) when it fails to load — the UseImage
	// + Fallback pair every legacy icon component wraps.
	import { CircleQuestionMark } from '@lucide/svelte';

	let { src, alt, class: className = '' }: { src: string; alt: string; class?: string } = $props();

	let failed = $state(false);
	// A changed source gets a fresh chance.
	$effect(() => {
		void src;
		failed = false;
	});
</script>

{#if failed}
	<div class="grid aspect-square place-items-center rounded-lg border border-border {className}">
		<CircleQuestionMark stroke-width={1} class="h-full w-full text-muted-foreground" />
	</div>
{:else}
	<img {src} {alt} class={className} onerror={() => (failed = true)} />
{/if}
