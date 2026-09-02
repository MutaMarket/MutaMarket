<script lang="ts">
  // One attribute cell of a list row, the legacy List/ListAttribute.vue:
  // icon + value (plus score when enabled) with a tooltip carrying the
  // name, base value and colored difference, and the roll bar below.
  import AttributeBar from './attribute-bar.svelte';
  import GameImage from './game-image.svelte';
  import {
    attributeFormattedDifference,
    attributeScoreClass,
    attributeScoreLabel,
    attributeVariant,
    formatValue,
  } from '$lib/attributes';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { t } from '$lib/i18n.svelte';
  import type { DisplaySettings } from '$lib/display';
  import type { ModuleAttributeView } from '$lib/types';

  let {
    attribute,
    settings,
    compact = false,
  }: {
    attribute: ModuleAttributeView;
    settings: DisplaySettings;
    compact?: boolean;
  } = $props();

  const variant = $derived(attributeVariant(attribute));
  const formattedBase = $derived(
    formatValue(
      attribute.base_value,
      attribute.unit?.name ?? null,
      attribute.unit?.display_name ?? null,
    ),
  );
  const formattedValue = $derived(
    formatValue(
      attribute.value,
      attribute.unit?.name ?? null,
      attribute.unit?.display_name ?? null,
    ),
  );

  const variantText: Record<string, string> = {
    gold: 'text-gold',
    diamond: 'text-diamond',
    brown: 'text-brown',
    positive: 'text-positive',
    'positive-derived': 'text-positive-derived',
    'negative-derived': 'text-negative-derived',
    negative: 'text-negative',
  };
</script>

<div
  class="grid min-w-0 content-center items-center gap-x-2 overflow-hidden px-2 py-1 {compact
    ? 'grid-cols-[16px_minmax(0,1fr)_auto]'
    : 'grid-cols-[24px_minmax(0,1fr)_auto]'}"
>
  <GameImage
    src="/img/icons/{attribute.id}.png"
    alt={attribute.name}
    class={compact ? 'size-4' : 'size-6'}
  />
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <div {...props} class="flex min-w-0 gap-1 text-sm text-white">
          <span>{formattedValue}</span>
          {#if settings.show_attribute_scores}
            <span class="inline-block text-sm font-medium {attributeScoreClass(attribute)}">
              {attributeScoreLabel(attribute)}
            </span>
          {/if}
        </div>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content>
      <div class="grid gap-0.5">
        <span class="font-medium">
          {attribute.display_name === '' ? attribute.name : attribute.display_name}
        </span>
        <span class="text-xs text-muted-foreground">
          {t('modules.card.base', { value: formattedBase })}
        </span>
        <span class="text-xs {variantText[variant]}">
          {attributeFormattedDifference(attribute)}
        </span>
      </div>
    </Tooltip.Content>
  </Tooltip.Root>
  <div class="col-span-full my-1">
    {#if settings.attribute_bar_mode !== 'none'}
      <AttributeBar {attribute} mode={settings.attribute_bar_mode} />
    {/if}
  </div>
</div>
