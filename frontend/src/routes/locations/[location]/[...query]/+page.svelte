<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  // One asset location, the legacy ShowLocationPage: the location
  // header with its breadcrumb and stats, the filter band, the module
  // grid, and the create-a-collection action.
  import { FolderPlus } from '@lucide/svelte';
  import { goto } from '$app/navigation';
  import FilterBand from '$lib/components/filter-band.svelte';
  import GameImage from '$lib/components/game-image.svelte';
  import ModuleDisplay from '$lib/components/module-display.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { countStat, scopedModuleStats } from '$lib/module-stats';
  import { Button } from '$lib/components/ui/button';
  import { t } from '$lib/i18n.svelte';
  import { notifyError } from '$lib/toast';
  import { parseQueryUi } from '$lib/query';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();
  const search = $derived(parseQueryUi(data.query));
  const prefix = $derived(`locations/${data.locationSlug}`);

  const name = $derived(
    data.location.name || data.location.type?.name || t('meta.location.unknownLocation'),
  );

  let creating = $state(false);
  async function createCollection() {
    creating = true;
    const response = await fetch('/location-collections', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ location_id: data.location.id }),
    });
    creating = false;
    if (response.redirected) {
      goto(new URL(response.url).pathname);
    } else if (response.ok) {
      goto('/collections');
    } else {
      notifyError(
        t('misc.locations.collectionNotCreatedTitle'),
        t('misc.locations.collectionNotCreatedBody'),
      );
    }
  }
</script>

<PageMeta
  title={data.panel
    ? t('meta.location.titleWithType', {
        type: data.panel.type_name,
        location: data.location.name ?? name,
      })
    : name}
  description={t('meta.location.description')}
  keywords="contracts, public, search, find"
/>

{#snippet parentLink()}
  {@const parent = data.location.location!}
  <a href="/locations/{parent.slug}" class="truncate hover:text-foreground hover:underline">
    {parent.type?.name || t('meta.location.unknownLocation')}
  </a>
{/snippet}

<PageHeader
  title={name}
  subtitle={data.location.type?.name ?? t('forms.filters.location')}
  context={data.location.location ? parentLink : undefined}
  stats={scopedModuleStats(
    [countStat(t('stats.overview.modules'), data.stats.total_count, 'primary')],
    data.stats,
  )}
>
  {#snippet icon()}
    <GameImage
      src="https://images.evetech.net/types/{data.location.type?.id ?? 0}/icon?size=64"
      alt={name}
      class="size-10 rounded-lg"
    />
  {/snippet}
  {#snippet actions()}
    <Button onclick={createCollection} disabled={creating}>
      <FolderPlus class="size-4" />
      {t('misc.locations.createCollection')}
    </Button>
  {/snippet}
</PageHeader>
<FilterBand
  {prefix}
  {search}
  panel={data.panel}
  unknownType={data.unknownType}
  variant="location"
/>
<div class="my-4 w-full">
  <ModuleDisplay
    entries={data.modules.map((module) => ({ module }))}
    {settings}
    panel={data.panel}
    {search}
    {prefix}
  />
</div>
