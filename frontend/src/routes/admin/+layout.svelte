<script lang="ts">
  // The console shell every admin page renders inside: the identity
  // rail with the live status pills, and the segmented section bar.
  // Pages render only their own body, so moving between sections swaps
  // the content and leaves the chrome alone.
  import type { Snippet } from 'svelte';
  import { page } from '$app/state';
  import { apply, live, subscribe } from '$lib/admin-live.svelte';
  import { formatUptime } from '$lib/admin-vitals';
  import { t } from '$lib/i18n.svelte';
  import type { LayoutProps } from './$types';

  let { data, children }: LayoutProps & { children: Snippet } = $props();

  const SECTIONS = $derived([
    {
      href: '/admin',
      label: t('admin.console.sections.overview'),
      exact: true,
      subtitle: t('admin.console.sections.overviewSubtitle'),
    },
    {
      href: '/admin/jobs',
      label: t('admin.console.sections.jobs'),
      exact: false,
      subtitle: t('admin.console.sections.jobsSubtitle'),
    },
    {
      href: '/admin/telemetry',
      label: t('admin.console.sections.telemetry'),
      exact: false,
      subtitle: t('admin.console.sections.telemetrySubtitle'),
    },
    {
      href: '/admin/activity',
      label: t('admin.console.sections.activity'),
      exact: false,
      subtitle: t('admin.console.sections.activitySubtitle'),
    },
    {
      href: '/admin/advertisements',
      label: t('nav.menu.advertisements'),
      exact: false,
      subtitle: t('admin.console.sections.advertisementsSubtitle'),
    },
    {
      href: '/admin/gear-items',
      label: t('nav.menu.gearItems'),
      exact: false,
      subtitle: t('admin.console.sections.gearItemsSubtitle'),
    },
    {
      href: '/admin/raffles',
      label: t('nav.menu.raffles'),
      exact: false,
      subtitle: t('admin.console.sections.rafflesSubtitle'),
    },
  ]);

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
    <span class="hud-label">{t('admin.console.eyebrow')}</span>
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
        {header.enabled ? t('admin.console.loopsRunning') : t('admin.console.loopsDisabled')}
      </span>
      <span class="rounded-full border border-border px-2.5 py-0.5 text-xs text-muted-foreground">
        {t('admin.console.uptime', { uptime: formatUptime(header.uptime_seconds) })}
      </span>
      {#if header.in_downtime}
        <span class="rounded-full border border-border px-2.5 py-0.5 text-xs text-[#fab219]">
          {t('admin.console.eveDowntime')}
        </span>
      {/if}
    </span>
  {/if}
</div>

<!-- The section bar: one control group for the whole console, matching
     the vitals timeframe toggle's segmented idiom. -->
<nav
  class="mb-6 flex w-full overflow-x-auto rounded-[9px] border border-border bg-card-2 p-0.5"
  aria-label={t('admin.console.sectionsAriaLabel')}
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
