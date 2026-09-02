<script lang="ts">
  // The console shell every admin page renders inside: the identity
  // rail with the live status pills, and the segmented section bar.
  // Pages render only their own body, so moving between sections swaps
  // the content and leaves the chrome alone.
  import type { Snippet } from 'svelte';
  import { page } from '$app/state';
  import { apply, live, subscribe } from '$lib/admin-live.svelte';
  import { formatUptime } from '$lib/admin-vitals';
  import type { LayoutProps } from './$types';

  let { data, children }: LayoutProps & { children: Snippet } = $props();

  const SECTIONS = [
    {
      href: '/admin',
      label: 'Overview',
      exact: true,
      subtitle: 'Service character, container vitals and what the ingestion has landed',
    },
    {
      href: '/admin/jobs',
      label: 'Jobs',
      exact: false,
      subtitle: 'Every scheduled background job, its history and its controls',
    },
    {
      href: '/admin/telemetry',
      label: 'Telemetry',
      exact: false,
      subtitle: 'Outgoing ESI requests and failures over the last hour',
    },
    {
      href: '/admin/activity',
      label: 'Activity',
      exact: false,
      subtitle: 'Who is using the site, how much, and how many of them come back',
    },
    {
      href: '/admin/advertisements',
      label: 'Advertisements',
      exact: false,
      subtitle: 'The sidebar rotation, scheduled and prioritized',
    },
    {
      href: '/admin/gear-items',
      label: 'Gear items',
      exact: false,
      subtitle: 'The recommended-gear rotation in the sidebar',
    },
    {
      href: '/admin/raffles',
      label: 'Raffles',
      exact: false,
      subtitle: 'The prize pool the hourly draw picks winners from',
    },
  ];

  const path = $derived(page.url.pathname);
  const active = $derived(
    SECTIONS.find((section) =>
      section.exact ? path === section.href : path.startsWith(section.href),
    ) ?? SECTIONS[0],
  );

  // The SSR payload seeds the store; the poll keeps it moving. Both the
  // seed and the subscription are the layout's, so the pills stay live
  // on pages that draw nothing else.
  $effect(() => {
    apply(data.live);
  });
  $effect(() => subscribe(['header']));

  const header = $derived(live.header ?? data.live.header ?? null);
</script>

<div class="mb-5 flex flex-wrap items-end gap-x-6 gap-y-3">
  <div class="min-w-0">
    <span class="hud-label">Admin // Operations</span>
    <h1 class="mt-1 text-2xl leading-none font-bold">{active.label}</h1>
    <p class="mt-1.5 text-sm text-muted-foreground">{active.subtitle}</p>
  </div>
  {#if header}
    <span class="ml-auto flex flex-wrap items-center gap-2">
      <span
        class="flex items-center gap-1.5 rounded-full border border-border px-2.5 py-0.5 text-xs {header.enabled
          ? 'text-positive'
          : 'text-muted-foreground'}"
      >
        <span
          class="size-1.5 rounded-full {header.enabled
            ? 'bg-positive animate-pulse'
            : 'bg-muted-foreground/40'}"
        ></span>
        {header.enabled ? 'loops running' : 'loops disabled'}
      </span>
      <span class="rounded-full border border-border px-2.5 py-0.5 text-xs text-muted-foreground">
        up {formatUptime(header.uptime_seconds)}
      </span>
      {#if header.in_downtime}
        <span class="rounded-full border border-border px-2.5 py-0.5 text-xs text-[#fab219]">
          EVE downtime
        </span>
      {/if}
    </span>
  {/if}
</div>

<!-- The section bar: one control group for the whole console, matching
     the vitals timeframe toggle's segmented idiom. -->
<nav
  class="mb-6 flex w-full overflow-x-auto rounded-[9px] border border-border bg-card-2 p-0.5"
  aria-label="Admin sections"
>
  {#each SECTIONS as section (section.href)}
    <a
      href={section.href}
      class="flex h-8 flex-1 items-center justify-center rounded-[6px] px-3 font-mono text-2xs tracking-[0.12em] whitespace-nowrap uppercase transition-colors
				{section === active
        ? 'bg-primary text-primary-foreground'
        : 'text-muted-foreground hover:bg-white/[0.04] hover:text-foreground'}"
      aria-current={section === active ? 'page' : undefined}
    >
      {section.label}
    </a>
  {/each}
</nav>

{@render children()}
