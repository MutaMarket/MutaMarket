<script lang="ts">
	// The shared page frame ported from the Leptos layout: navigation with
	// the login state, the routed page content, and the footer.
	import './layout.css';
	import type { Snippet } from 'svelte';
	import favicon from '$lib/assets/favicon.svg';
	import MainNav from '$lib/components/main-nav.svelte';
	import MakeOfferDialog from '$lib/components/make-offer-dialog.svelte';
	import Sidebar from '$lib/components/sidebar.svelte';
	import WorkbenchDrawer from '$lib/components/workbench-drawer.svelte';
	import { Toaster } from '$lib/components/ui/sonner';
	import { refreshSentOffers } from '$lib/make-offer';
	import { refreshWorkbench } from '$lib/workbench';
	import type { LayoutData } from './$types';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	// The signed-in user's active sent offers, for the cards' Go to
	// offer swap (the legacy withLatestOfferMadeByAuthenticatedUser).
	$effect(() => {
		if (data.nav?.user) {
			void refreshSentOffers();
			void refreshWorkbench();
		}
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<header class="bg-card-1">
	<MainNav nav={data.nav} />
</header>
<!-- The container grows by the sidebar's width on xl so the page
     content keeps its full max-w-7xl beside it. -->
<main class="mx-auto flex w-full max-w-7xl flex-1 gap-6 px-4 py-6 xl:max-w-[calc(var(--container-7xl)+250px+--spacing(6))]">
	<div class="min-w-0 flex-1">
		{@render children()}
	</div>
	<Sidebar />
</main>
<footer class="border-t border-border">
	<p class="mx-auto w-full max-w-7xl xl:max-w-[calc(var(--container-7xl)+250px+--spacing(6))] px-4 py-4 text-xs text-muted-foreground">
		MutaMarket - the marketplace and toolbox for abyssal modules in EVE Online.
	</p>
</footer>
<Toaster position="top-center" />
<MakeOfferDialog />
<WorkbenchDrawer />
