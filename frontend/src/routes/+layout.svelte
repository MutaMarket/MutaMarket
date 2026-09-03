<script lang="ts">
  // The shared page frame: navigation with the login state, the routed
  // page content, and the footer.
  import './layout.css';
  import type { Snippet } from 'svelte';
  import favicon from '$lib/assets/favicon.svg';
  import { ADSENSE_CLIENT_ID, adsenseScriptUrl, showsAds } from '$lib/adsense';
  import MainNav from '$lib/components/main-nav.svelte';
  import MakeOfferDialog from '$lib/components/make-offer-dialog.svelte';
  import ModuleEditBar from '$lib/components/module-edit-bar.svelte';
  import RafflePrizeDialog from '$lib/components/raffle-prize-dialog.svelte';
  import Sidebar from '$lib/components/sidebar.svelte';
  import WorkbenchDrawer from '$lib/components/workbench-drawer.svelte';
  import { Toaster } from '$lib/components/ui/sonner';
  import { page } from '$app/state';
  import { seedLocale, t } from '$lib/i18n.svelte';
  import type { LayoutData } from './$types';

  let { data, children }: { data: LayoutData; children: Snippet } = $props();

  // The server decided the locale (cookie, then Accept-Language); the
  // browser runtime starts from it before any child renders.
  // svelte-ignore state_referenced_locally -- one-time seed
  seedLocale(data.locale);

  // The admin console is an internal tool with wide chart grids; the
  // marketing rail it would otherwise share the row with is exactly
  // what it manages.
  const isConsole = $derived(page.url.pathname.startsWith('/admin'));
</script>

<svelte:head>
  <link rel="icon" href="/favicon.ico" sizes="32x32" />
  <link rel="icon" href={favicon} type="image/svg+xml" />
  {#if showsAds(data.nav, ADSENSE_CLIENT_ID)}
    <!-- AdSense Auto ads: the loader alone, Google picks the placements. -->
    <meta name="google-adsense-account" content={ADSENSE_CLIENT_ID} />
    <script async src={adsenseScriptUrl(ADSENSE_CLIENT_ID)} crossorigin="anonymous"></script>
  {/if}
</svelte:head>

<header class="bg-card-1">
  <MainNav nav={data.nav} />
</header>
<!-- The page content and the sidebar. On xl the grid reserves the
     sidebar column up front: the sidebar's markup arrives last in the
     streamed document, and without a fixed column the content would
     paint full width and reflow when it lands. The container grows by
     the sidebar's width so the content keeps its full max-w-7xl. -->
<main
  class="mx-auto w-full max-w-7xl flex-1 px-4 py-6 {isConsole
    ? ''
    : 'xl:grid xl:max-w-[calc(var(--container-7xl)+250px+--spacing(6))] xl:grid-cols-[minmax(0,1fr)_250px] xl:gap-6'}"
>
  <div class="min-w-0">
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
    {t('nav.footer.tagline')}
  </p>
</footer>
<Toaster position="top-center" />
{#if data.nav?.raffle}
  <RafflePrizeDialog prize={data.nav.raffle} />
{/if}
<MakeOfferDialog />
<ModuleEditBar />
<WorkbenchDrawer />
