<script lang="ts">
  // The account's Premium / Ad-free switch: a two-way radio that posts
  // the plan change and reloads the page data (the sidebar status and
  // the premium page's character table share it).
  import { Crown, EyeOff } from '@lucide/svelte';
  import { t } from '#lib/i18n.svelte.ts';
  import { switchPremiumPlan } from '#lib/premium-plan.ts';

  let { adFree, class: className = '' }: { adFree: boolean; class?: string } = $props();

  let switching = $state(false);

  async function choose(toAdFree: boolean) {
    if (switching || toAdFree === adFree) {
      return;
    }
    switching = true;
    try {
      await switchPremiumPlan(toAdFree);
    } finally {
      switching = false;
    }
  }
</script>

<div
  class={['grid grid-cols-2 gap-px border border-border bg-border text-xs', className]}
  role="radiogroup"
  aria-label={t('premium.show.tierLabel')}
>
  {#each [false, true] as tierAdFree (tierAdFree)}
    {@const chosen = adFree === tierAdFree}
    <button
      type="button"
      role="radio"
      aria-checked={chosen}
      disabled={switching}
      class={[
        'flex cursor-pointer items-center justify-center gap-1 px-2 py-1 whitespace-nowrap transition-colors disabled:cursor-wait',
        chosen
          ? 'bg-primary text-primary-foreground'
          : 'bg-card text-muted-foreground hover:text-foreground',
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
