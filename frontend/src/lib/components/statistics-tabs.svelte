<script lang="ts">
  // The statistics section rail: three URL-backed sub-pages styled as
  // a HUD segment strip.
  import { page } from '$app/state';
  import { t } from '$lib/i18n.svelte';

  const TABS = $derived([
    { href: '/statistics', label: t('stats.tabs.overview'), exact: true },
    { href: '/statistics/characters', label: t('stats.allStatistics.topCharacters'), exact: false },
    { href: '/statistics/personal', label: t('stats.tabs.personal'), exact: false },
  ]);

  const path = $derived(page.url.pathname);
  function active(tab: (typeof TABS)[number]): boolean {
    return tab.exact ? path === tab.href : path.startsWith(tab.href);
  }
</script>

<nav class="mb-4 flex gap-1 border-b border-border" aria-label={t('stats.tabs.ariaLabel')}>
  {#each TABS as tab (tab.href)}
    <a
      href={tab.href}
      class="relative -mb-px px-4 py-2.5 font-mono text-xs tracking-[0.14em] uppercase transition-colors
				{active(tab)
        ? 'border-b-2 border-primary text-primary [text-shadow:0_0_12px_color-mix(in_srgb,var(--color-primary)_60%,transparent)]'
        : 'border-b-2 border-transparent text-muted-foreground hover:text-foreground'}"
      aria-current={active(tab) ? 'page' : undefined}
    >
      {tab.label}
    </a>
  {/each}
</nav>
