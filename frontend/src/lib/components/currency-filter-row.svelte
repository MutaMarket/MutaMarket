<script lang="ts">
  // The price / estimated-value filter row, mirroring the legacy
  // PriceFilter.vue and ValueFilter.vue: wallet title, a pair of ISK
  // bound fields, a log-scale slider, sort trio
  // (specs/browser-filters.md §4).
  import { Wallet } from '@lucide/svelte';
  import CurrencyInput from './currency-input.svelte';
  import RangeSlider, { type SliderMark } from './range-slider.svelte';
  import SortButtons from './sort-buttons.svelte';
  import { goto } from '$app/navigation';
  import { toVeryCompact } from '$lib/format-number';
  import { buildQueryPath, type UiSearch } from '$lib/query';
  import { clamp, currencyToNormalized, currencyToOriginal } from '$lib/slider-scale';

  let {
    prefix,
    search,
    kind,
  }: {
    prefix: string;
    search: UiSearch;
    /** `price` (reversed slider, single bound = maximum) or `value`. */
    kind: 'price' | 'value';
  } = $props();

  /** The legacy hardcoded ISK slider range. */
  const LOWEST = 1_000_000;
  const HIGHEST = 100_000_000_000;
  const LABEL_STEP = 20;
  const SEARCH_DEBOUNCE_MS = 200;

  const reversed = $derived(kind === 'price');
  const bounds = $derived(kind === 'price' ? search.price : search.value);

  function normalized(amount: number): number {
    return clamp(currencyToNormalized(amount, LOWEST, HIGHEST), 0, 100);
  }

  function initialValues(): [number, number] {
    if (bounds === null) {
      return [0, 100];
    }
    const [lower, upper] = bounds;
    if (upper !== null) {
      const first = normalized(lower);
      const second = normalized(upper);
      return [Math.min(first, second), Math.max(first, second)];
    }
    // Lower-bound-only: the reversed price slider reads its single
    // bound from the right handle ("at most X"), the value slider
    // from the left ("at least X").
    return reversed ? [0, normalized(lower)] : [normalized(lower), 100];
  }

  // The slider follows the URL: when the committed bounds change from
  // the outside (a match-type apply, back/forward, a shared link), the
  // handles reseed. The effect deliberately watches only the committed
  // search state — never the live handle values — and skips the change
  // this row itself just navigated to, so drags stay free.
  // svelte-ignore state_referenced_locally -- seeded before the effect
  let values: [number, number] = $state(initialValues());
  // svelte-ignore state_referenced_locally -- baseline for the effect
  let lastCommitted = JSON.stringify(bounds);
  let ownCommit = false;
  $effect(() => {
    const committed = JSON.stringify(bounds);
    if (committed === lastCommitted) {
      return;
    }
    lastCommitted = committed;
    if (ownCommit) {
      ownCommit = false;
      return;
    }
    values = initialValues();
  });

  const marks: SliderMark[] = Array.from({ length: 100 / LABEL_STEP + 1 }, (_, index) => ({
    position: index * LABEL_STEP,
    kind: 'regular' as const,
    label: toVeryCompact(currencyToOriginal(index * LABEL_STEP, LOWEST, HIGHEST)),
  }));

  function navigate([lower, upper]: [number, number]) {
    ownCommit = true;
    let next: [number, number | null] | null;
    if (lower === 0 && upper === 100) {
      next = null;
    } else if (reversed && lower === 0) {
      // Price with only the right handle moved: "at most X".
      next = [currencyToOriginal(upper, LOWEST, HIGHEST), null];
    } else if (!reversed && upper === 100) {
      // Value with only the left handle moved: "at least X".
      next = [currencyToOriginal(lower, LOWEST, HIGHEST), null];
    } else {
      next = [
        currencyToOriginal(lower, LOWEST, HIGHEST),
        currencyToOriginal(upper, LOWEST, HIGHEST),
      ];
    }
    goto(buildQueryPath(prefix, { ...search, [kind]: next, page: 1 }), {
      keepFocus: true,
      noScroll: true,
    });
  }

  let debounce: ReturnType<typeof setTimeout> | null = null;
  function searchSoon(next: [number, number]) {
    if (debounce !== null) {
      clearTimeout(debounce);
    }
    debounce = setTimeout(() => navigate(next), SEARCH_DEBOUNCE_MS);
  }

  // Seeded from the committed values so the server-rendered inputs
  // already carry them (the effect below only handles later changes).
  // svelte-ignore state_referenced_locally -- deliberate SSR seed
  let lowerInput = $state(String(Math.round(currencyToOriginal(values[0], LOWEST, HIGHEST))));
  // svelte-ignore state_referenced_locally -- deliberate SSR seed
  let upperInput = $state(String(Math.round(currencyToOriginal(values[1], LOWEST, HIGHEST))));
  $effect(() => {
    lowerInput = String(Math.round(currencyToOriginal(values[0], LOWEST, HIGHEST)));
    upperInput = String(Math.round(currencyToOriginal(values[1], LOWEST, HIGHEST)));
  });

  function submitInputs() {
    const lower = Number(lowerInput);
    const upper = Number(upperInput);
    if (Number.isNaN(lower) || Number.isNaN(upper)) {
      return;
    }
    values = [normalized(lower), normalized(upper)];
    navigate(values);
  }
</script>

<div class="flex gap-2 p-4">
  <div class="flex w-full flex-wrap items-start gap-2">
    <h2 class="flex items-center gap-2 text-sm font-medium">
      <Wallet class="size-4" />
      <span>{kind === 'price' ? 'Price' : 'Est. value'}</span>
    </h2>
    <div class="ml-auto grid w-full max-w-[300px] grid-cols-2">
      {#each [0, 1] as bound (bound)}
        <CurrencyInput
          label="{kind} {bound === 0 ? 'lower' : 'upper'} bound"
          value={bound === 0 ? lowerInput : upperInput}
          class="{bound === 0
            ? 'rounded-r-none'
            : 'rounded-l-none border-l-0'} border border-border/50 bg-input"
          onchange={(text) => {
            if (bound === 0) lowerInput = text;
            else upperInput = text;
          }}
          onblur={submitInputs}
          onenter={submitInputs}
        />
      {/each}
    </div>
    <div class="z-10 w-full grow px-4">
      <RangeSlider bind:values {marks} oninput={searchSoon}>
        {#snippet tooltip(position)}
          <div class="rounded-lg border border-primary bg-card p-2 text-sm shadow-lg">
            {toVeryCompact(currencyToOriginal(position, LOWEST, HIGHEST))}
          </div>
        {/snippet}
      </RangeSlider>
    </div>
  </div>
  <SortButtons {prefix} {search} field={kind} />
</div>
