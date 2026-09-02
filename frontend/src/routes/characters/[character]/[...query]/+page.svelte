<script lang="ts">
  import { currentDisplaySettings } from '$lib/display';
  // A character's modules with the full filter band, mirroring the
  // legacy ShowCharacterModulesPage: for-sale/created scope, misc
  // chips, value slider and the attribute grid.
  import FilterBand from '$lib/components/filter-band.svelte';
  import ModuleDisplay from '$lib/components/module-display.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { characterOgImage } from '$lib/meta';
  import { parseQueryUi } from '$lib/query';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  const settings = $state(currentDisplaySettings(data.displaySettings));
  const search = $derived(parseQueryUi(data.query));
  const prefix = $derived(`characters/${data.page.character.slug}`);
</script>

<PageMeta
  title={data.page.character.name}
  description={`Browse ${data.page.character.name}'s abyssal modules on MutaMarket.`}
  image={characterOgImage(data.page.character.id)}
  keywords={[data.page.character.name, 'character', 'modules']}
/>

<PageHeader
  title={data.page.character.name}
  subtitle={data.page.character.description ?? 'Capsuleer'}
  stats={[
    {
      label: 'For sale',
      value: data.page.for_sale_count.toLocaleString('en-US'),
      accent: 'primary',
    },
    { label: 'Created', value: data.page.created_count.toLocaleString('en-US') },
  ]}
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
