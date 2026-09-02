<script lang="ts">
  // The shared page frame ported from the Leptos layout: navigation with
  // the login state, the routed page content, and the footer.
  import './layout.css';
  import type { Snippet } from 'svelte';
  import favicon from '$lib/assets/favicon.svg';
  import MainNav from '$lib/components/main-nav.svelte';
  import MakeOfferDialog from '$lib/components/make-offer-dialog.svelte';
  import ModuleEditBar from '$lib/components/module-edit-bar.svelte';
  import RafflePrizeDialog from '$lib/components/raffle-prize-dialog.svelte';
  import Sidebar from '$lib/components/sidebar.svelte';
  import WorkbenchDrawer from '$lib/components/workbench-drawer.svelte';
  import { Toaster } from '$lib/components/ui/sonner';
  import { page } from '$app/state';
  import type { LayoutData } from './$types';

  let { data, children }: { data: LayoutData; children: Snippet } = $props();

  // The admin console is an internal tool with wide chart grids; the
  // marketing rail it would otherwise share the row with is exactly
  // what it manages.
  const isConsole = $derived(page.url.pathname.startsWith('/admin'));
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<header class="bg-card-1">
  <MainNav nav={data.nav} />
</header>
<!-- The container grows by the sidebar's width on xl so the page
     content keeps its full max-w-7xl beside it. -->
<main
  class="mx-auto flex w-full max-w-7xl flex-1 gap-6 px-4 py-6 xl:max-w-[calc(var(--container-7xl)+250px+--spacing(6))]"
>
  <div class="min-w-0 flex-1">
    {@render children()}
  </div>
  {#if !isConsole}
    <Sidebar />
  {/if}
</main>
<footer class="border-t border-border">
  <p
    class="mx-auto w-full max-w-7xl xl:max-w-[calc(var(--container-7xl)+250px+--spacing(6))] px-4 py-4 text-xs text-muted-foreground"
  >
    MutaMarket - the marketplace and toolbox for abyssal modules in EVE Online.
  </p>
</footer>
<Toaster position="top-center" />
{#if data.nav?.raffle}
  <RafflePrizeDialog prize={data.nav.raffle} />
{/if}
<MakeOfferDialog />
<ModuleEditBar />
<WorkbenchDrawer />
