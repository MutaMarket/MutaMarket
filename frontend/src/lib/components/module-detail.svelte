<script lang="ts">
	// The module detail view ported from the Leptos ModuleDetailView.
	import ModuleCard from './module-card.svelte';
	import { formatFraction } from '$lib/attributes';
	import type { DisplaySettings } from '$lib/display';
	import type { ModuleDetail } from '$lib/types';

	let { module, settings }: { module: ModuleDetail; settings: DisplaySettings } = $props();
</script>

<article class="grid items-start gap-4 md:grid-cols-[minmax(280px,380px)_1fr]">
	<ModuleCard {module} {settings} />
	<section>
		<h1 class="text-xl font-semibold">{module.type.name}</h1>
		<p class="mt-1 text-sm text-muted-foreground">
			{#if module.source_type}Mutated from {module.source_type.name}{/if}{#if module.mutaplasmid}
				with {module.mutaplasmid.name}{/if}
		</p>
		{#if module.average_fraction !== null}
			<p class="mt-2 text-sm">
				Roll quality:
				<span class={module.average_fraction < 0 ? 'text-negative' : 'text-positive'}>
					{formatFraction(module.average_fraction)}
				</span>
			</p>
		{/if}
		<p class="mt-2 text-sm text-muted-foreground">Est. value: N/A</p>
	</section>
</article>
