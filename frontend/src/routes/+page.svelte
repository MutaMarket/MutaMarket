<script lang="ts">
  import { currentDisplaySettings } from '$lib/display';
  import ModuleBrowser from '$lib/components/module-browser.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { typeOgImage } from '$lib/meta';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  // Shared with the options bar, which mutates it for instant re-renders
  // (the cookie persists in the background for the next visit).
  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  const settings = $state(currentDisplaySettings(data.displaySettings));
</script>

<PageMeta
  title={data.panel?.type_name ?? 'Modules for sale'}
  description="Find the perfect abyssal module for your needs on MutaMarket, the best place to buy and sell abyssal modules!"
  image={data.panel ? typeOgImage(data.panel.type_id) : undefined}
  keywords="contracts, public, search, find"
/>

<ModuleBrowser {data} {settings} />
