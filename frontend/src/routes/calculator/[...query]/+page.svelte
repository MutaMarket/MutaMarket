<script lang="ts">
  // The mutation probability calculator, the legacy CalculatorPage:
  // the calculator filter band (category, meta, attributes; no market
  // options) over the sortable combination table, or the
  // select-a-category invitation.
  import { TriangleAlert } from '@lucide/svelte';
  import CalculatorTable from '$lib/components/calculator-table.svelte';
  import FilterBand from '$lib/components/filter-band.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { parseQueryUi } from '$lib/query';
  import { t } from '$lib/i18n.svelte';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const search = $derived(parseQueryUi(data.query));
</script>

<PageMeta
  title={t('meta.calculator.title')}
  description={t('meta.calculator.description')}
  keywords="contracts, public, search, find"
/>

<PageHeader title={t('calculator.page.title')} subtitle={t('calculator.page.subtitle')} />
<FilterBand
  prefix="calculator"
  {search}
  panel={data.panel}
  unknownType={data.unknownType}
  variant="calculator"
/>
<div class="my-4 w-full">
  {#if data.probability !== null}
    <CalculatorTable rows={data.probability} />
  {:else}
    <div class="hud-frame flex items-center justify-center gap-4 p-8">
      <TriangleAlert class="size-8 text-orange-500" />
      <span class="text-2xl">{t('calculator.page.selectCategory')}</span>
    </div>
  {/if}
</div>
