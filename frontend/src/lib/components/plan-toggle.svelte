<script lang="ts">
  // The account's Premium / Ad-free switch: a two-way radio that posts
  // the plan change and reloads the page data (the sidebar status and
  // the premium page's character table share it). Going ad-free with
  // live premium asks first, since the account then stays ad-free for
  // AD_FREE_MIN_DAYS; while that lock runs, the way back is disabled.
  import { Crown, EyeOff } from '@lucide/svelte';
  import AdFreeConfirmDialog from './ad-free-confirm-dialog.svelte';
  import { t } from '#lib/i18n.svelte.ts';
  import type { PremiumConfig } from '#lib/premium.ts';
  import { switchPremiumPlan } from '#lib/premium-plan.ts';

  let {
    adFree,
    characters,
    lockedUntil,
    premium,
    class: className = '',
  }: {
    adFree: boolean;
    /** The account's characters, whose live premium the switch converts. */
    characters: { id: number; name: string; premium_paid_until: string | null }[];
    /** When the account may switch back (ISO 8601), null when it may. */
    lockedUntil: string | null;
    /** The price points the warning compares. */
    premium: PremiumConfig;
    class?: string;
  } = $props();

  const live = $derived(characters.some((entry) => entry.premium_paid_until !== null));

  let switching = $state(false);
  let confirming = $state(false);

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

<AdFreeConfirmDialog
  bind:open={confirming}
  {characters}
  {premium}
  busy={switching}
  onconfirm={() => apply(true)}
/>
