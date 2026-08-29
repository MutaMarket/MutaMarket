<script lang="ts">
	// The documentation pages, ported from the Leptos DocumentationView
	// (legacy ShowDocumentationPage.vue): sticky section sidebar, HUD-panel
	// content frame with the section label, GitHub edit link, rendered
	// markdown article, and previous/next footer links. The mobile page
	// picker stays a native select like the Leptos port.
	import { goto } from '$app/navigation';
	import type { PageProps } from './$types';
	import PageMeta from '$lib/components/page-meta.svelte';

	let { data }: PageProps = $props();

	const doc = $derived(data.doc);
</script>

<PageMeta title={doc.title} description={`MutaMarket documentation: ${doc.title}`} />

<div class="lg:grid lg:grid-cols-[240px_minmax(0,1fr)] lg:gap-6">
	<nav class="hud-frame hidden space-y-5 self-start p-4 lg:sticky lg:top-20 lg:block">
		{#each doc.sections as section (section.title)}
			<div>
				<span class="hud-label">{section.title}</span>
				<ul class="mt-2 space-y-0.5">
					{#each section.pages as entry (entry.slug)}
						<li>
							<a
								href="/documentation/{entry.slug}"
								class="block border-l-2 px-3 py-1.5 text-sm transition-colors {entry.slug ===
								doc.slug
									? 'border-primary bg-primary/5 text-foreground'
									: 'border-transparent text-muted-foreground hover:text-foreground'}"
							>
								{entry.title}
							</a>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
	</nav>

	<div class="hud-frame min-w-0">
		<div class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-6 py-4">
			<div>
				<span class="hud-label">Documentation // {doc.section}</span>
				<h1 class="mt-1 text-2xl font-bold">{doc.title}</h1>
			</div>
			<a
				href={doc.edit_url}
				class="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
				rel="noopener noreferrer"
				target="_blank"
			>
				Edit this page on GitHub
			</a>
		</div>

		<div class="border-b border-border px-6 py-3 lg:hidden">
			<select
				class="w-full border border-border bg-background px-3 py-2 text-sm"
				onchange={(event) => {
					const value = event.currentTarget.value;
					if (value) goto(`/documentation/${value}`);
				}}
			>
				{#each doc.sections as section (section.title)}
					<optgroup label={section.title}>
						{#each section.pages as entry (entry.slug)}
							<option value={entry.slug} selected={entry.slug === doc.slug}>
								{entry.title}
							</option>
						{/each}
					</optgroup>
				{/each}
			</select>
		</div>

		<!-- eslint-disable-next-line svelte/no-at-html-tags -- server-rendered
		     markdown, sanitized by the API's hardened renderer -->
		<article class="docs-prose px-6 py-6 md:px-8">{@html doc.html}</article>

		<div class="grid grid-cols-2 border-t border-border">
			{#if doc.previous}
				<a
					href="/documentation/{doc.previous.slug}"
					class="group flex flex-col gap-1 p-4 transition-colors hover:bg-secondary/40"
				>
					<span class="hud-label inline-flex items-center gap-1.5">{'← Previous'}</span>
					<span class="text-sm font-medium transition-colors group-hover:text-primary">
						{doc.previous.title}
					</span>
				</a>
			{:else}
				<div></div>
			{/if}
			{#if doc.next}
				<a
					href="/documentation/{doc.next.slug}"
					class="group flex flex-col items-end gap-1 border-l border-border p-4 text-right transition-colors hover:bg-secondary/40"
				>
					<span class="hud-label inline-flex items-center gap-1.5">{'Next →'}</span>
					<span class="text-sm font-medium transition-colors group-hover:text-primary">
						{doc.next.title}
					</span>
				</a>
			{/if}
		</div>
	</div>
</div>
