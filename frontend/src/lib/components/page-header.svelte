<script lang="ts">
  // The page header strip (no legacy counterpart, replacing the bare
  // h1 titles): identity on the left — an icon or portrait, the title,
  // a muted subtitle — and a right-aligned readout of the resource's
  // key numbers, hairline-separated, with the headline stat carrying an
  // accent color. Deliberately unboxed: the filter band below is
  // already a heavy panel.
  import type { Snippet } from 'svelte';

  export interface HeaderStat {
    label: string;
    value: string;
    /** The headline stat carries the theme accent; every other figure
     * stays plain. */
    accent?: 'primary';
  }

  let {
    title,
    subtitle = null,
    stats = [],
    icon,
    actions,
  }: {
    title: string;
    subtitle?: string | null;
    stats?: HeaderStat[];
    icon?: Snippet;
    /** Page-level actions rendered right of the stats. */
    actions?: Snippet;
  } = $props();

  const accentClass = (accent: HeaderStat['accent']) =>
    accent === 'primary' ? 'text-primary' : 'text-foreground';
</script>

<header class="mb-4 flex flex-wrap items-center justify-between gap-x-8 gap-y-3">
  <div class="flex min-w-0 items-center gap-3">
    {#if icon}
      {@render icon()}
    {/if}
    <div class="min-w-0">
      <h1 class="truncate text-xl leading-tight font-semibold tracking-tight">{title}</h1>
      {#if subtitle}
        <p class="truncate text-sm text-muted-foreground">{subtitle}</p>
      {/if}
    </div>
  </div>
  <div class="flex max-w-full flex-wrap items-center gap-x-6 gap-y-3">
    {#if stats.length > 0}
      <dl class="flex max-w-full flex-wrap items-center gap-x-6 gap-y-2">
        {#each stats as stat (stat.label)}
          <div
            class="flex flex-col items-end gap-1 border-l border-border pl-4 first:border-l-0 first:pl-0"
          >
            <dt class="hud-label whitespace-nowrap">{stat.label}</dt>
            <dd class="text-lg leading-none font-semibold tabular-nums {accentClass(stat.accent)}">
              {stat.value}
            </dd>
          </div>
        {/each}
      </dl>
    {/if}
    {#if actions}
      {@render actions()}
    {/if}
  </div>
</header>
