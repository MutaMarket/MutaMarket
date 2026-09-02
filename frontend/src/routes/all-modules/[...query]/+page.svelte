<script lang="ts">
  import { currentDisplaySettings } from '$lib/display';
  import ModuleBrowser from '$lib/components/module-browser.svelte';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { typeOgImage } from '$lib/meta';

  let { data }: PageProps = $props();

  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  const settings = $state(currentDisplaySettings(data.displaySettings));
</script>

<PageMeta
  title={data.panel?.type_name ?? 'All modules in the database'}
  description="Ever wanted to see the full list of abyssal modules? We got you covered!"
  image={data.panel ? typeOgImage(data.panel.type_id) : undefined}
  keywords="contracts, public, search, find"
/>

<ModuleBrowser {data} {settings} />
