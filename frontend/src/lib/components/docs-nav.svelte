<script lang="ts">
	// The documentation sidebar, shared by the markdown pages and the
	// generated API reference so the two never drift apart.
	import type { DocNavSection } from '$lib/docs';

	let {
		sections,
		current,
	}: {
		sections: DocNavSection[];
		/** The active page's slug, or `api` for the generated reference. */
		current: string;
	} = $props();

	const link = (active: boolean) =>
		`block border-l-2 px-3 py-1.5 text-sm transition-colors ${
			active
				? 'border-primary bg-primary/5 text-foreground'
				: 'border-transparent text-muted-foreground hover:text-foreground'
		}`;
</script>

<nav class="hud-frame hidden space-y-5 self-start p-4 lg:sticky lg:top-20 lg:block">
	{#each sections as section (section.title)}
		<div>
			<span class="hud-label">{section.title}</span>
			<ul class="mt-2 space-y-0.5">
				{#each section.pages as entry (entry.slug)}
					<li>
						<a href="/documentation/{entry.slug}" class={link(entry.slug === current)}>
							{entry.title}
						</a>
					</li>
				{/each}
				{#if section.title === 'API'}
					<li>
						<a href="/documentation/api" class={link(current === 'api')}>Endpoint reference</a>
					</li>
				{/if}
			</ul>
		</div>
	{/each}
</nav>
