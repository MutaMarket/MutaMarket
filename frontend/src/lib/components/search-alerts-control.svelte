<script lang="ts">
  // The search alert control in the market page header (a rewrite
  // addition): the bell saves the current query as an alert, or removes
  // the alert that already holds it, and the chevron beside it opens the
  // account's saved alerts, each linking back to its search and
  // removable in place. Saving needs a type in the query, the server's
  // rule, and premium: without it the bell opens the premium pitch.
  import { Bell, BellRing, ChevronDown, Crown } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { t } from '$lib/i18n.svelte';
  import type { UiSearch } from '$lib/query';
  import { alertOptions, alertQuery, alertSearchPath, type SearchAlert } from '$lib/search-alerts';
  import { notifyError, notifySuccess } from '$lib/toast';

  let {
    search,
    alerts,
    hasPremium,
  }: { search: UiSearch; alerts: SearchAlert[]; hasPremium: boolean } = $props();

  const query = $derived(alertQuery(search));
  const active = $derived(alerts.find((alert) => alert.query === query) ?? null);
  const disabled = $derived(search.typeSlug === null);

  let busy = $state(false);
  let premiumPitch = $state(false);

  function notifiedOn(iso: string): string {
    return new Date(iso).toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    });
  }

  async function remove(alertId: number): Promise<boolean> {
    const response = await fetch(`/search-alerts/${alertId}`, {
      method: 'DELETE',
      redirect: 'manual',
    });
    if (!response.ok) {
      notifyError(t('modules.alerts.failedTitle'), t('modules.alerts.failedBody'));
      return false;
    }
    notifySuccess(t('modules.alerts.removedTitle'), t('modules.alerts.removedBody'));
    await invalidateAll();
    return true;
  }

  async function save(): Promise<boolean> {
    const response = await fetch('/search-alerts', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ query }),
      redirect: 'manual',
    });
    if (!response.ok) {
      // The validation payload carries the server's reason (the cap, the
      // type rule); anything else gets the generic text.
      const body: { errors?: { query?: string[] }; message?: string } = await response
        .json()
        .catch(() => ({}));
      notifyError(
        t('modules.alerts.failedTitle'),
        body.errors?.query?.[0] ?? body.message ?? t('modules.alerts.failedBody'),
      );
      return false;
    }
    notifySuccess(t('modules.alerts.savedTitle'), t('modules.alerts.savedBody'));
    await invalidateAll();
    return true;
  }

  async function toggle() {
    if (busy || disabled) {
      return;
    }
    if (!hasPremium) {
      premiumPitch = true;
      return;
    }
    busy = true;
    try {
      await (active !== null ? remove(active.id) : save());
    } finally {
      busy = false;
    }
  }

  const SEGMENT =
    'flex h-9 items-center justify-center transition-colors text-muted-foreground hover:text-foreground';
</script>

<div
  class="flex items-stretch rounded-[7px] border bg-card-2 {active !== null
    ? 'border-primary/60'
    : 'border-border'}"
>
  <Tooltip.Provider delayDuration={300}>
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            {...props}
            type="button"
            class="{SEGMENT} gap-1.5 rounded-l-[6px] px-3 text-xs {active !== null
              ? 'bg-primary/15 text-foreground'
              : ''} {disabled ? 'cursor-not-allowed opacity-40' : ''}"
            aria-pressed={active !== null}
            aria-disabled={disabled}
            onclick={toggle}
          >
            {#if active !== null}
              <BellRing class="size-4 text-primary" />
              {t('modules.alerts.saved')}
            {:else}
              <Bell class="size-4" />
              {t('modules.alerts.save')}
            {/if}
          </button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content>
        {disabled ? t('modules.alerts.pickType') : t('modules.alerts.hint')}
      </Tooltip.Content>
    </Tooltip.Root>
  </Tooltip.Provider>
  <DropdownMenu.Root>
    <DropdownMenu.Trigger
      class="{SEGMENT} relative w-8 rounded-r-[6px] border-l border-border"
      aria-label={t('modules.alerts.title')}
    >
      <ChevronDown class="size-3.5" />
      {#if alerts.length > 0}
        <span
          class="absolute -top-1 -right-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[10px] leading-none font-semibold text-primary-foreground"
        >
          {alerts.length}
        </span>
      {/if}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content sideOffset={8} align="end" class="w-80 p-3">
      <p class="hud-label mb-1">{t('modules.alerts.title')}</p>
      <p class="mb-2 text-xs text-muted-foreground">
        {alerts.length > 0 ? t('modules.alerts.description') : t('modules.alerts.emptyDescription')}
      </p>
      {#if alerts.length > 0}
        <ul class="divide-y divide-border">
          {#each alerts as alert (alert.id)}
            {@const options = alertOptions(alert)}
            <li class="flex items-center gap-3 py-2">
              <div class="min-w-0 grow">
                <a
                  href={alertSearchPath(alert)}
                  class="block truncate text-sm font-medium hover:underline"
                >
                  {alert.type?.name ?? t('modules.alerts.allTypes')}
                </a>
                <span class="block truncate font-mono text-xs text-muted-foreground">
                  {options.length > 0 ? options.join(' / ') : t('modules.alerts.noOptions')}
                </span>
                <span class="block text-xs text-muted-foreground">
                  {alert.last_notified_at === null
                    ? t('modules.alerts.neverNotified')
                    : t('modules.alerts.notified', {
                        count: alert.notified_count,
                        date: notifiedOn(alert.last_notified_at),
                      })}
                </span>
              </div>
              <Button variant="secondary" size="sm" onclick={() => remove(alert.id)}>
                {t('modules.alerts.remove')}
              </Button>
            </li>
          {/each}
        </ul>
      {/if}
    </DropdownMenu.Content>
  </DropdownMenu.Root>
</div>

<Dialog.Root bind:open={premiumPitch}>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <Crown class="size-4 text-primary" />
        {t('modules.alerts.premiumTitle')}
      </Dialog.Title>
      <Dialog.Description>{t('modules.alerts.premiumBody')}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="secondary" onclick={() => (premiumPitch = false)}>
        {t('common.actions.cancel')}
      </Button>
      <Button href="/premium">
        <Crown class="size-4" />
        {t('modules.alerts.discoverPremium')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
