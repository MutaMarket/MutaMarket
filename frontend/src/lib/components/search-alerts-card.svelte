<script lang="ts">
  // The settings page's alerts tab (a rewrite addition): every saved
  // search alert with what it watches for in words, its history, and
  // the way back to its search. Saving happens from the market header.
  import { Bell, Crown, ExternalLink } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button';
  import { t } from '$lib/i18n.svelte';
  import { alertCriteria, alertSearchPath, type SearchAlert } from '$lib/search-alerts';

  let {
    alerts,
    hasPremium,
    onRemove,
  }: {
    alerts: SearchAlert[];
    hasPremium: boolean;
    onRemove: (alertId: number) => void | Promise<void>;
  } = $props();

  function dateOf(iso: string): string {
    return new Date(iso).toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    });
  }
</script>

<div class="hud-frame relative p-6">
  <Bell class="absolute top-4 right-4 size-20 text-white/5" />
  <h2 class="relative flex items-center gap-2 font-medium">
    <Bell class="size-4 text-primary" />
    {t('modules.alerts.title')}
  </h2>
  <p class="relative mt-1 max-w-prose text-sm text-muted-foreground">
    {alerts.length > 0 ? t('modules.alerts.description') : t('modules.alerts.emptyDescription')}
  </p>
  <p class="relative mt-1 max-w-prose text-sm text-muted-foreground">
    {t('modules.alerts.howItWorks')}
  </p>
  {#if !hasPremium}
    <div class="relative mt-4 flex flex-wrap items-center gap-3">
      <span class="text-sm text-muted-foreground">{t('modules.alerts.premiumBody')}</span>
      <Button href="/premium" size="sm" variant="secondary">
        <Crown class="size-4" />
        {t('modules.alerts.discoverPremium')}
      </Button>
    </div>
  {/if}
  {#if alerts.length > 0}
    <div class="relative mt-5 flex flex-col gap-4">
      {#each alerts as alert (alert.id)}
        {@const criteria = alertCriteria(alert)}
        <article class="rounded-lg border border-border p-4">
          <div class="flex flex-wrap items-start gap-3">
            <div class="min-w-0 grow">
              <h3 class="truncate font-medium">
                {alert.type?.name ?? t('modules.alerts.allTypes')}
              </h3>
              <p class="text-xs text-muted-foreground">
                {t('modules.alerts.createdOn', { date: dateOf(alert.created_at) })}
                <span class="mx-1">·</span>
                {alert.last_notified_at === null
                  ? t('modules.alerts.neverNotified')
                  : t('modules.alerts.notified', {
                      count: alert.notified_count,
                      date: dateOf(alert.last_notified_at),
                    })}
              </p>
            </div>
            <div class="flex gap-2">
              <Button variant="outline" size="sm" href={alertSearchPath(alert)}>
                <ExternalLink class="size-4" />
                {t('modules.alerts.openSearch')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                class="text-destructive hover:bg-destructive/10"
                onclick={() => onRemove(alert.id)}
              >
                {t('modules.alerts.remove')}
              </Button>
            </div>
          </div>
          <ul class="mt-3 flex flex-wrap gap-2">
            {#if criteria.length === 0}
              <li class="text-sm text-muted-foreground">{t('modules.alerts.anyListing')}</li>
            {/if}
            {#each criteria as criterion, index (index)}
              <li
                class="flex h-7 items-center gap-1.5 rounded-[7px] border border-border bg-card-2 px-2.5 text-xs"
              >
                <span class="size-1.5 rounded-full bg-primary"></span>
                {t(`modules.alerts.criteria.${criterion.key}`, criterion.params)}
              </li>
            {/each}
          </ul>
        </article>
      {/each}
    </div>
  {/if}
</div>
