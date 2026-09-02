<script lang="ts">
  // The vertical sort trio of a slider row, mirroring the legacy
  // SortByButtons.vue: chevron up, a tiny SORT label, chevron down.
  // The active direction pulses primary; clicking it again unsorts.
  // Links rather than buttons, so the hover preload has the result ready.
  import { ChevronUp } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button';
  import { t } from '$lib/i18n.svelte';
  import { buildQueryPath, type UiSearch } from '$lib/query';

  let {
    prefix,
    search,
    field,
  }: {
    prefix: string;
    search: UiSearch;
    /** `price`, `value` or an attribute name. */
    field: string;
  } = $props();

  const active = $derived(
    search.sort !== null && search.sort[0].toLowerCase() === field.toLowerCase(),
  );
  const activeAsc = $derived(active && search.sort?.[1] === false);
  const activeDesc = $derived(active && search.sort?.[1] === true);

  function target(descending: boolean, isActive: boolean): string {
    const next: UiSearch = {
      ...search,
      sort: isActive ? null : [field, descending],
    };
    return buildQueryPath(prefix, next);
  }
</script>

<div class="grid place-items-center gap-2">
  <Button
    data-active={activeAsc}
    variant="ghost"
    size="icon"
    class="data-[active=true]:animate-pulse data-[active=true]:text-primary"
    title={t('forms.sort.ascending')}
    href={target(false, activeAsc)}
    data-sveltekit-noscroll
    data-sveltekit-keepfocus
  >
    <ChevronUp class="size-4" />
  </Button>
  <span class="text-2xs leading-none font-medium uppercase">{t('forms.sort.sort')}</span>
  <Button
    data-active={activeDesc}
    variant="ghost"
    size="icon"
    class="data-[active=true]:animate-pulse data-[active=true]:text-primary"
    title={t('forms.sort.descending')}
    href={target(true, activeDesc)}
    data-sveltekit-noscroll
    data-sveltekit-keepfocus
  >
    <ChevronUp class="size-4 rotate-180" />
  </Button>
</div>
