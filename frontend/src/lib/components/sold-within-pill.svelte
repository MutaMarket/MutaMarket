<script lang="ts">
  // The historic page's sold-within window: a chip holding a day count.
  // Enter or leaving the field applies it; an empty or non-positive
  // value drops the filter.
  import { goto } from '$app/navigation';
  import { t } from '#lib/i18n.svelte.ts';
  import { buildQueryPath, type UiSearch } from '#lib/query.ts';

  let { prefix, search }: { prefix: string; search: UiSearch } = $props();

  // svelte-ignore state_referenced_locally -- seeded, then follows the URL below
  let draft = $state(search.soldWithin === null ? '' : String(search.soldWithin));
  $effect(() => {
    draft = search.soldWithin === null ? '' : String(search.soldWithin);
  });

  function apply() {
    const days = Number.parseInt(draft, 10);
    const next = Number.isNaN(days) || days <= 0 ? null : days;
    if (next === search.soldWithin) {
      draft = next === null ? '' : String(next);
      return;
    }
    goto(buildQueryPath(prefix, { ...search, soldWithin: next, page: 1 }), { reset: false });
  }
</script>

<label
  class="flex h-7 items-center gap-1.5 rounded-[7px] border px-2.5 text-xs transition-colors {search.soldWithin !==
  null
    ? 'border-primary/60 bg-primary/15 text-foreground'
    : 'border-border bg-card-2 text-muted-foreground hover:text-foreground'}"
  title={t('forms.filters.soldWithinHint')}
>
  <span
    class="size-1.5 rounded-full {search.soldWithin !== null
      ? 'bg-primary'
      : 'bg-muted-foreground/40'}"
  ></span>
  {t('forms.filters.soldWithin')}
  <input
    type="text"
    inputmode="numeric"
    bind:value={draft}
    onblur={apply}
    onkeydown={(event) => {
      if (event.key === 'Enter') {
        apply();
      }
    }}
    class="w-8 border-b border-border bg-transparent text-center text-foreground outline-none focus:border-primary"
  />
  {t('forms.filters.soldWithinDays')}
</label>
