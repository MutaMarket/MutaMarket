<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  import ModuleBrowser from '$lib/components/module-browser.svelte';
  import ModuleDetail from '$lib/components/module-detail.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { t } from '$lib/i18n.svelte';
  import { moduleMetaDescription, moduleMetaTitle, moduleOgImage, typeOgImage } from '$lib/meta';
  import type { BrowserData } from '$lib/server/browser';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();

  const browser = $derived(data as unknown as BrowserData);
</script>

{#if data.module}
  <PageMeta
    title={moduleMetaTitle(data.module)}
    description={moduleMetaDescription(data.module, data.estimatorStatistic ?? null)}
    image={moduleOgImage(data.module.id, data.module.mutated_attributes)}
    keywords="contracts, public, search, find"
  />
  <ModuleDetail
    module={data.module}
    statistic={data.estimatorStatistic ?? null}
    comparisons={data.sourceTypeComparisons ?? []}
    historicContracts={data.historicContracts ?? []}
    typeStatistics={data.typeStatistics ?? null}
    initialTab={data.showTab ?? 'market'}
    {settings}
  />
{:else}
  <PageMeta
    title={browser.panel?.type_name ?? t('meta.modules.title')}
    description={t('meta.modules.description')}
    image={browser.panel ? typeOgImage(browser.panel.type_id) : undefined}
    keywords="contracts, public, search, find"
  />
  <ModuleBrowser data={browser} {settings} />
{/if}
