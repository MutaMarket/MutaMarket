<script lang="ts">
  import { currentDisplaySettings } from '$lib/display';
  import ModuleBrowser from '$lib/components/module-browser.svelte';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  const settings = $state(currentDisplaySettings(data.displaySettings));
</script>

<PageMeta
  title={data.panel?.type_name ?? 'Historic sales'}
  description="Find the perfect abyssal module for your needs on MutaMarket, the best place to buy and sell abyssal modules!"
  keywords="contracts, public, search, find"
/>

<ModuleBrowser {data} {settings} />
