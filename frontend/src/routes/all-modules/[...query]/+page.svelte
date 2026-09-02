<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  import ModuleBrowser from '$lib/components/module-browser.svelte';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { typeOgImage } from '$lib/meta';
  import { t } from '$lib/i18n.svelte';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();
</script>

<PageMeta
  title={data.panel?.type_name ?? t('meta.allModules.title')}
  description={t('meta.allModules.description')}
  image={data.panel ? typeOgImage(data.panel.type_id) : undefined}
  keywords="contracts, public, search, find"
/>

<ModuleBrowser {data} {settings} />
