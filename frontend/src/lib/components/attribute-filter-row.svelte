<script lang="ts">
  // One attribute filter, mirroring the legacy Attributes/
  // AttributeFilter.vue: icon + name title, the bound inputs with the
  // related-types dropdown, the pip slider, and the sort trio
  //.
  import { List } from '@lucide/svelte';
  import GameImage from './game-image.svelte';
  import RangeSlider, { type SliderMark } from './range-slider.svelte';
  import SliderNodeTypes from './slider-node-types.svelte';
  import SortButtons from './sort-buttons.svelte';
  import { goto } from '$app/navigation';
  import { formatValue, revertTransformValue, transformValue } from '$lib/attributes';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { Input } from '$lib/components/ui/input';
  import { metaGroupDotClass, sortByMetaAndName } from '$lib/filter-meta';
  import { t } from '$lib/i18n.svelte';
  import { buildQueryPath, type UiSearch } from '$lib/query';
  import { attributeToNormalized, attributeToOriginal, clamp } from '$lib/slider-scale';
  import type { FilterAttribute, FilterSourceType } from '$lib/types';

  let {
    prefix,
    search,
    attribute,
    sourceTypes,
    allowSort = true,
  }: {
    prefix: string;
    search: UiSearch;
    attribute: FilterAttribute;
    sourceTypes: FilterSourceType[];
    allowSort?: boolean;
  } = $props();

  /** Regular label positions, every 20 slider points (legacy steps). */
  const LABEL_STEP = 20;
  /** The legacy slider debounce before navigating. */
  const SEARCH_DEBOUNCE_MS = 200;

  function normalized(value: number): number {
    return clamp(attributeToNormalized(value, attribute.best, attribute.worst), 0, 100);
  }

  function initialValues(): [number, number] {
    const active = search.attributes.find(
      (filter) => filter.name.toLowerCase() === attribute.name.toLowerCase(),
    );
    if (!active) {
      return [0, 100];
    }
    const lower = normalized(active.lower);
    if (active.upper === null) {
      return [lower, 100];
    }
    const upper = normalized(active.upper);
    return [Math.min(lower, upper), Math.max(lower, upper)];
  }

  // The slider follows the URL: when the committed bounds change from
  // the outside (a match-type apply, back/forward, a shared link), the
  // handles reseed. The effect deliberately watches only the committed
  // search state — never the live handle values — and skips the change
  // this row itself just navigated to, so drags stay free.
  // svelte-ignore state_referenced_locally -- seeded before the effect
  let values: [number, number] = $state(initialValues());
  // svelte-ignore state_referenced_locally -- baseline for the effect
  let lastCommitted = JSON.stringify(
    search.attributes.find(
      (filter) => filter.name.toLowerCase() === attribute.name.toLowerCase(),
    ) ?? null,
  );
  let ownCommit = false;
  $effect(() => {
    const committed = JSON.stringify(
      search.attributes.find(
        (filter) => filter.name.toLowerCase() === attribute.name.toLowerCase(),
      ) ?? null,
    );
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

  function formatted(position: number): string {
    return formatValue(
      attributeToOriginal(position, attribute.best, attribute.worst),
      attribute.unit_name,
      attribute.unit_display_name,
    );
  }

  // Source types carrying this attribute, pip positions dedup'd.
  const related = $derived(
    sourceTypes
      .map((sourceType) => ({
        id: sourceType.id,
        name: sourceType.name,
        meta_group_id: sourceType.meta_group_id,
        value: sourceType.attributes.find((value) => value.attribute_id === attribute.attribute_id)
          ?.value,
      }))
      .filter(
        (sourceType): sourceType is typeof sourceType & { value: number } =>
          sourceType.value !== undefined,
      )
      .sort(sortByMetaAndName),
  );

  const marks = $derived.by(() => {
    const byPosition = new Map<number, SliderMark>();
    for (const sourceType of related) {
      const position = normalized(sourceType.value);
      const mark = byPosition.get(position) ?? {
        position,
        kind: 'pip' as const,
        formatted: formatted(position),
        types: [],
      };
      mark.types?.push(sourceType);
      byPosition.set(position, mark);
    }
    for (let position = 0; position <= 100; position += LABEL_STEP) {
      if (!byPosition.has(position)) {
        byPosition.set(position, {
          position,
          kind: 'regular',
          label: formatted(position),
        });
      }
    }
    return [...byPosition.values()];
  });

  // The source types sitting exactly at a slider position (a pip), for the
  // drag tooltip: as a handle lands on a node it names the types there.
  function typesAt(position: number): NonNullable<SliderMark['types']> {
    const mark = marks.find(
      (entry) => entry.kind === 'pip' && Math.abs(entry.position - position) < 0.01,
    );
    return mark?.types ?? [];
  }

  // Source types grouped by their value for this attribute, ascending, so
  // the type dropdown reads as value headings with the types under each.
  const relatedGroups = $derived.by(() => {
    const byValue = new Map<number, { value: number; types: typeof related }>();
    for (const type of related) {
      const group = byValue.get(type.value) ?? { value: type.value, types: [] };
      group.types.push(type);
      byValue.set(type.value, group);
    }
    return [...byValue.values()].sort((a, b) => a.value - b.value);
  });

  function navigate([lower, upper]: [number, number]) {
    ownCommit = true;
    const attributes = search.attributes.filter(
      (filter) => filter.name.toLowerCase() !== attribute.name.toLowerCase(),
    );
    if (lower !== 0 || upper !== 100) {
      const low = attributeToOriginal(lower, attribute.best, attribute.worst);
      const high = attributeToOriginal(upper, attribute.best, attribute.worst);
      if (upper === 100) {
        attributes.push({ name: attribute.name, lower: low, upper: null });
      } else {
        attributes.push({
          name: attribute.name,
          lower: Math.min(low, high),
          upper: Math.max(low, high),
        });
      }
    }
    goto(buildQueryPath(prefix, { ...search, attributes, page: 1 }), {
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

  // Bound inputs show display-transformed true values, seeded from the
  // committed values so the server-rendered inputs already carry them
  // (the effect only handles later changes).
  // svelte-ignore state_referenced_locally -- deliberate SSR seed
  let lowerInput = $state(displayValue(values[0]));
  // svelte-ignore state_referenced_locally -- deliberate SSR seed
  let upperInput = $state(displayValue(values[1]));
  $effect(() => {
    lowerInput = displayValue(values[0]);
    upperInput = displayValue(values[1]);
  });

  function displayValue(position: number): string {
    const raw = attributeToOriginal(position, attribute.best, attribute.worst);
    const transformed = transformValue(raw, attribute.unit_name);
    return String(Math.round(transformed * 100) / 100);
  }

  function submitInputs() {
    const lower = revertTransformValue(Number(lowerInput), attribute.unit_name);
    const upper = revertTransformValue(Number(upperInput), attribute.unit_name);
    if (Number.isNaN(lower) || Number.isNaN(upper)) {
      return;
    }
    values = [normalized(lower), normalized(upper)];
    navigate(values);
  }

  // Picking a source type sets the lower bound to that type's value for
  // this attribute (upper stays at the maximum), the legacy handleTypeSelect.
  function selectType(value: number) {
    values = [normalized(value), 100];
    navigate(values);
  }
</script>

<div class="flex gap-2 p-4">
  <div class="flex w-full flex-wrap gap-2">
    <h2 class="flex items-center gap-2 text-sm font-medium">
      <GameImage
        src="/img/icons/{attribute.attribute_id}.png"
        alt={attribute.display_name}
        class="size-6"
      />
      <span>{attribute.display_name === '' ? attribute.name : attribute.display_name}</span>
    </h2>
    <div class="ml-auto flex w-full max-w-[320px] items-start">
      <div class="grid flex-1 grid-cols-2 items-start">
        {#each [0, 1] as bound (bound)}
          <div class="isolate grid grid-cols-[auto_1fr] items-center focus-within:z-10">
            <Input
              class="col-span-full col-start-1 row-start-1 h-8 w-full min-w-0 {bound === 0
                ? 'rounded-r-none'
                : 'rounded-none border-l-0'} border border-border/50 bg-input pl-11 text-right text-xs"
              type="number"
              aria-label={t(
                bound === 0 ? 'forms.rangeInput.lowerBound' : 'forms.rangeInput.upperBound',
                { name: attribute.name },
              )}
              value={bound === 0 ? lowerInput : upperInput}
              oninput={(event) => {
                const text = (event.target as HTMLInputElement).value;
                if (bound === 0) lowerInput = text;
                else upperInput = text;
              }}
              onblur={submitInputs}
              onkeydown={(event) => event.key === 'Enter' && submitInputs()}
            />
            <div
              class="pointer-events-none z-10 col-start-1 row-start-1 px-2 text-xs text-muted-foreground"
            >
              {attribute.unit_display_name ?? ''}
            </div>
          </div>
        {/each}
      </div>

      {#if related.length > 0}
        <!-- Set the bound from a source type's value for this attribute. -->
        <DropdownMenu.Root>
          <DropdownMenu.Trigger
            title={t('forms.filters.selectType')}
            class="flex h-8 shrink-0 items-center justify-center rounded-l-none border border-l-0 border-border/50 bg-input/20 px-2 text-muted-foreground transition hover:text-foreground focus-visible:z-10 focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30 focus-visible:outline-none dark:bg-input/30"
          >
            <List class="size-4" />
            <span class="sr-only">{t('forms.filters.selectType')}</span>
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end" class="max-h-80 w-56 overflow-y-auto">
            {#each relatedGroups as group (group.value)}
              <DropdownMenu.Group>
                <DropdownMenu.GroupHeading
                  class="px-2 py-1 text-[0.62rem] tracking-wider text-muted-foreground/70 tabular-nums"
                >
                  {formatted(normalized(group.value))}
                </DropdownMenu.GroupHeading>
                {#each group.types as type (type.id)}
                  <DropdownMenu.Item class="gap-2 text-xs" onSelect={() => selectType(type.value)}>
                    <span
                      class="size-2 shrink-0 rounded-full {metaGroupDotClass(type.meta_group_id)}"
                    ></span>
                    <span class="truncate">{type.name}</span>
                  </DropdownMenu.Item>
                {/each}
              </DropdownMenu.Group>
            {/each}
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      {/if}
    </div>
    <div class="z-10 w-full grow px-4">
      <RangeSlider
        bind:values
        {marks}
        labels={[
          t('forms.rangeSlider.lowerHandle', { name: attribute.name }),
          t('forms.rangeSlider.upperHandle', { name: attribute.name }),
        ]}
        valueText={formatted}
        oninput={searchSoon}
      >
        {#snippet tooltip(position)}
          <SliderNodeTypes value={formatted(position)} types={typesAt(position)} />
        {/snippet}
      </RangeSlider>
    </div>
  </div>
  {#if allowSort}
    <SortButtons {prefix} {search} field={attribute.name} />
  {/if}
</div>
