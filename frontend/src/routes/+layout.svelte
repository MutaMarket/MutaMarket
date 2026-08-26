<script lang="ts">
	// The shared page frame ported from the Leptos layout: navigation with
	// the login state, the routed page content, and the footer.
	import './layout.css';
	import type { Snippet } from 'svelte';
	import favicon from '$lib/assets/favicon.svg';
	import CharacterMenu from '$lib/components/character-menu.svelte';
	import { Toaster } from '$lib/components/ui/sonner';
	import type { LayoutData } from './$types';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	const navLink = 'text-sm text-muted-foreground transition-colors hover:text-foreground';
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<header class="border-b border-border bg-card-1">
	<nav class="mx-auto flex w-full max-w-7xl flex-wrap items-center gap-x-5 gap-y-2 px-4 py-3">
		<a href="/" class="text-base font-semibold tracking-tight">MutaMarket</a>
		<a href="/modules" class={navLink}>Modules</a>
		<a href="/all-modules" class={navLink}>All Modules</a>
		<a href="/characters" class={navLink}>Characters</a>
		<a href="/collections" class={navLink}>Collections</a>
		<a href="/calculator" class={navLink}>Calculator</a>
		<a href="/statistics" class={navLink}>Statistics</a>
		{#if data.nav}
			<span class="ml-auto flex items-center gap-3">
				{#if data.nav.user.is_admin}
					<a href="/admin/scheduler" class={navLink}>Admin</a>
				{/if}
				<a href="/personal/modules" class={navLink}>My modules</a>
				<CharacterMenu characters={data.nav.characters} />
			</span>
		{:else}
			<span class="ml-auto">
				<a
					href="/login"
					class="rounded-md border border-border px-3 py-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
				>
					Log in
				</a>
			</span>
		{/if}
	</nav>
</header>
<main class="mx-auto w-full max-w-7xl flex-1 px-4 py-6">
	{@render children()}
</main>
<footer class="border-t border-border">
	<p class="mx-auto w-full max-w-7xl px-4 py-4 text-xs text-muted-foreground">
		MutaMarket - the marketplace and toolbox for abyssal modules in EVE Online.
	</p>
</footer>
<Toaster position="top-center" />
