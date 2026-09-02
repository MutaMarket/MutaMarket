<script lang="ts">
  // The "match a type" picker: choose a source type on the left, toggle
  // which of its attributes to filter by on the right (each chip
  // previews the bound it would apply), then Apply sets
  // "at least as good as this type" bounds for the checked attributes.
  // Replaces the legacy hidden per-attribute dropdown and center select
  // with one explicit flow.
  import { Check, Search } from '@lucide/svelte';
  import GameImage from './game-image.svelte';
  import { formatValue } from '$lib/attributes';
  import { Button } from '$lib/components/ui/button';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { metaGroupDotClass } from '$lib/filter-meta';
  import type { FilterPanelData } from '$lib/types';

  let {
    panel,
    triggerClass,
    onApply,
  }: {
    panel: FilterPanelData;
    triggerClass: string;
    /** Receives the bounds for the checked attributes. */
    onApply: (bounds: { name: string; lower: number; upper: null }[]) => void;
  } = $props();

  let open = $state(false);
  let typeFilter = $state('');
  let selectedTypeId: number | null = $state(null);
  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  let checked: Record<number, boolean> = $state(
    Object.fromEntries(panel.attributes.map((attribute) => [attribute.attribute_id, true])),
  );

  const filteredTypes = $derived(
    panel.source_types.filter((sourceType) =>
      sourceType.name.toLowerCase().includes(typeFilter.toLowerCase()),
    ),
  );
  const selectedType = $derived(
    panel.source_types.find((sourceType) => sourceType.id === selectedTypeId) ?? null,
  );

  const allChecked = $derived(
    panel.attributes.every((attribute) => checked[attribute.attribute_id]),
  );
  const noneChecked = $derived(
    panel.attributes.every((attribute) => !checked[attribute.attribute_id]),
  );

  /** The bound a chip would apply, formatted with the attribute unit. */
  function preview(attributeId: number): string | null {
    const value = selectedType?.attributes.find(
      (candidate) => candidate.attribute_id === attributeId,
    )?.value;
    const attribute = panel.attributes.find((candidate) => candidate.attribute_id === attributeId);
    if (value === undefined || !attribute) {
      return null;
    }
    return formatValue(value, attribute.unit_name, attribute.unit_display_name);
  }

  function apply() {
    if (!selectedType) {
      return;
    }
    const bounds = selectedType.attributes
      .filter((value) => checked[value.attribute_id])
      .flatMap((value) => {
        const attribute = panel.attributes.find(
          (candidate) => candidate.attribute_id === value.attribute_id,
        );
        return attribute ? [{ name: attribute.name, lower: value.value, upper: null }] : [];
      });
    open = false;
    onApply(bounds);
  }
</script>

<DropdownMenu.Root bind:open>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <button
        {...props}
        type="button"
        class="{triggerClass} flex items-center justify-between gap-2"
      >
        <span class="truncate text-muted-foreground">Match a type…</span>
        <Check class="size-3.5 shrink-0 text-muted-foreground" />
      </button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="start" class="w-[420px] p-0">
    <div class="grid grid-cols-[176px_1fr]">
      <div class="flex flex-col border-r border-border">
        <div class="flex items-center gap-1.5 border-b border-border px-2">
          <Search class="size-3.5 shrink-0 text-muted-foreground" />
          <input
            class="h-8 w-full min-w-0 bg-transparent text-xs outline-none placeholder:text-muted-foreground"
            placeholder="Filter types…"
            bind:value={typeFilter}
          />
        </div>
        <div class="max-h-64 grow overflow-y-auto p-1">
          {#each filteredTypes as sourceType (sourceType.id)}
            <button
              type="button"
              class="flex w-full items-center gap-2 rounded-sm px-2 py-1 text-left text-xs transition-colors {selectedTypeId ===
              sourceType.id
                ? 'bg-secondary text-foreground'
                : 'text-muted-foreground hover:bg-secondary/50 hover:text-foreground'}"
              onclick={() => (selectedTypeId = sourceType.id)}
            >
              <span
                class="size-1.5 shrink-0 rounded-full {metaGroupDotClass(sourceType.meta_group_id)}"
              ></span>
              <span class="truncate">{sourceType.name}</span>
            </button>
          {:else}
            <p class="p-2 text-xs text-muted-foreground">No matching types.</p>
          {/each}
        </div>
      </div>
      <div class="flex flex-col">
        <div class="flex items-center justify-between px-2.5 pt-2">
          <span class="hud-label">Match attributes</span>
          <button
            type="button"
            class="cursor-pointer text-xs text-primary hover:underline"
            onclick={() =>
              (checked = Object.fromEntries(
                panel.attributes.map((attribute) => [attribute.attribute_id, !allChecked]),
              ))}
          >
            {allChecked ? 'Clear all' : 'Select all'}
          </button>
        </div>
        <div class="flex grow flex-wrap content-start gap-1 p-2">
          {#each panel.attributes as attribute (attribute.attribute_id)}
            {@const on = checked[attribute.attribute_id] ?? false}
            {@const bound = preview(attribute.attribute_id)}
            <button
              type="button"
              class="flex h-6 items-center gap-1 rounded-[6px] border px-1.5 text-[11px] transition-colors {on
                ? 'border-primary/60 bg-primary/15 text-foreground'
                : 'border-border bg-card-2 text-muted-foreground hover:text-foreground'}"
              title={attribute.display_name === '' ? attribute.name : attribute.display_name}
              onclick={() => (checked[attribute.attribute_id] = !on)}
            >
              <GameImage
                src="/img/icons/{attribute.attribute_id}.png"
                alt={attribute.display_name}
                class="size-3.5"
              />
              {#if bound !== null}
                <span class="tabular-nums">≥ {bound}</span>
              {:else}
                <span class="max-w-24 truncate">
                  {attribute.display_name === '' ? attribute.name : attribute.display_name}
                </span>
              {/if}
            </button>
          {/each}
        </div>
        <div class="border-t border-border p-2">
          <Button
            size="sm"
            class="w-full"
            disabled={selectedType === null || noneChecked}
            onclick={apply}
          >
            {selectedType === null ? 'Pick a type' : 'Apply'}
          </Button>
        </div>
      </div>
    </div>
  </DropdownMenu.Content>
</DropdownMenu.Root>
