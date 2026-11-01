<script lang="ts">
  // The account's Premium / Ad-free switch: a two-way radio that posts
  // the plan change and reloads the page data (the sidebar status and
  // the premium page's character table share it). Going ad-free with
  // live premium asks first, since the account then stays ad-free for
  // AD_FREE_MIN_DAYS; while that lock runs, the way back is disabled.
  import { Crown, EyeOff } from '@lucide/svelte';
  import { Button } from '#lib/components/ui/button/index.ts';
  import * as Dialog from '#lib/components/ui/dialog/index.ts';
  import { t } from '#lib/i18n.svelte.ts';
  import { AD_FREE_MIN_DAYS, switchedDays } from '#lib/premium.ts';
  import { switchPremiumPlan } from '#lib/premium-plan.ts';

  let {
    adFree,
    live,
    lockedUntil,
    factor,
    class: className = '',
  }: {
    adFree: boolean;
    /** Some character of the account has live premium to convert. */
    live: boolean;
    /** When the account may switch back (ISO 8601), null when it may. */
    lockedUntil: string | null;
    /** The ad-free cost factor the time converts at. */
    factor: number;
    class?: string;
  } = $props();

  let switching = $state(false);
  let confirming = $state(false);
  const lockEndPreview = $derived(
    new Date(Date.now() + AD_FREE_MIN_DAYS * 86_400_000).toISOString().slice(0, 10),
  );

  async function apply(toAdFree: boolean) {
    switching = true;
    try {
      await switchPremiumPlan(toAdFree);
    } finally {
      switching = false;
      confirming = false;
    }
  }

  function choose(toAdFree: boolean) {
    if (switching || toAdFree === adFree) {
      return;
    }
    if (toAdFree && live) {
      confirming = true;
      return;
    }
    void apply(toAdFree);
  }
</script>

<div class={['grid gap-1', className]}>
  <div
    class="grid grid-cols-2 gap-px border border-border bg-border text-xs"
    role="radiogroup"
    aria-label={t('premium.show.tierLabel')}
  >
    {#each [false, true] as tierAdFree (tierAdFree)}
      {@const chosen = adFree === tierAdFree}
      <button
        type="button"
        role="radio"
        aria-checked={chosen}
        disabled={switching || (!tierAdFree && lockedUntil !== null)}
        class={[
          'flex cursor-pointer items-center justify-center gap-1 px-2 py-1 whitespace-nowrap transition-colors disabled:cursor-not-allowed',
          chosen
            ? 'bg-primary text-primary-foreground'
            : 'bg-card text-muted-foreground hover:text-foreground disabled:hover:text-muted-foreground',
        ]}
        onclick={() => choose(tierAdFree)}
      >
        {#if tierAdFree}
          <EyeOff class="size-3" />
        {:else}
          <Crown class="size-3" />
        {/if}
        {t(tierAdFree ? 'premium.show.tierAdFreeShort' : 'premium.show.tierPremium')}
      </button>
    {/each}
  </div>
  {#if lockedUntil !== null}
    <p class="text-[10px] text-muted-foreground">
      {t('premium.show.adFree.lockedUntil', { date: lockedUntil.slice(0, 10) })}
    </p>
  {/if}
</div>

<Dialog.Root bind:open={confirming}>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title>{t('premium.show.adFree.confirmTitle')}</Dialog.Title>
      <Dialog.Description class="space-y-2">
        <p>
          {t('premium.show.adFree.confirmLock', {
            days: AD_FREE_MIN_DAYS,
            date: lockEndPreview,
          })}
        </p>
        <p>
          {t('premium.show.adFree.confirmConvert', {
            plain: 30,
            adFree: switchedDays(30, true, factor),
          })}
        </p>
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="secondary" onclick={() => (confirming = false)}>
        {t('common.actions.cancel')}
      </Button>
      <Button disabled={switching} onclick={() => apply(true)}>
        <EyeOff />
        {t('premium.show.adFree.confirmButton')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
