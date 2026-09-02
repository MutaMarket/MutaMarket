<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  import ModuleBrowser from '$lib/components/module-browser.svelte';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { t } from '$lib/i18n.svelte';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();
</script>

<PageMeta
  title={data.panel?.type_name ?? t('meta.historicSales.title')}
  description={t('meta.historicSales.description')}
  keywords="contracts, public, search, find"
/>

<ModuleBrowser {data} {settings} />
