<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  import ModuleBrowser from '$lib/components/module-browser.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { typeOgImage } from '$lib/meta';
  import { t } from '$lib/i18n.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();
</script>

<PageMeta
  title={data.panel?.type_name ?? t('meta.modules.title')}
  description={t('meta.modules.description')}
  image={data.panel ? typeOgImage(data.panel.type_id) : undefined}
  keywords="contracts, public, search, find"
/>

<ModuleBrowser {data} {settings} />
