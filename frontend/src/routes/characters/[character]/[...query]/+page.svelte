<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  // A character's modules with the full filter band, mirroring the
  // legacy ShowCharacterModulesPage: for-sale/created scope, misc
  // chips, value slider and the attribute grid.
  import FilterBand from '$lib/components/filter-band.svelte';
  import ModuleDisplay from '$lib/components/module-display.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { countStat, scopedModuleStats } from '$lib/module-stats';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { t } from '$lib/i18n.svelte';
  import { characterOgImage } from '$lib/meta';
  import { parseQueryUi } from '$lib/query';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();
  const search = $derived(parseQueryUi(data.query));
  const prefix = $derived(`characters/${data.page.character.slug}`);
</script>

<PageMeta
  title={data.page.character.name}
  description={t('meta.character.description', { name: data.page.character.name })}
  image={characterOgImage(data.page.character.id)}
  keywords={[data.page.character.name, 'character', 'modules']}
/>

<PageHeader
  title={data.page.character.name}
  subtitle={data.page.character.description ?? t('characters.show.capsuleer')}
  stats={scopedModuleStats(
    [
      countStat(t('stats.overview.modulesForSale'), data.page.for_sale_count, 'primary'),
      countStat(t('stats.overview.modulesCreated'), data.page.created_count),
    ],
    data.page.stats,
  )}
>
  {#snippet icon()}
    <img
      alt=""
      class="size-10 rounded-lg"
      src="https://images.evetech.net/characters/{data.page.character.id}/portrait?size=64"
    />
  {/snippet}
</PageHeader>
<FilterBand
  {prefix}
  {search}
  panel={data.panel}
  unknownType={data.unknownType}
  variant="character"
/>
<div class="my-4 w-full">
  <ModuleDisplay
    entries={data.page.modules.map((module) => ({ module }))}
    {settings}
    panel={data.panel}
    {search}
    {prefix}
  />
</div>
