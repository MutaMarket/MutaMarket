<script lang="ts">
  // The gilded call to donate above the settings page's donation
  // history, a rewrite addition: what a month of premium costs and
  // unlocks, who to send the ISK to, and a pointer to the ad-free plan
  // for accounts that still see ads. The gold follows the premium name.
  import {
    Bell,
    Copy,
    Crown,
    EyeOff,
    History,
    ListOrdered,
    PackageCheck,
    Palette,
  } from '@lucide/svelte';
  import { Button } from '#lib/components/ui/button/index.ts';
  import { toCompact } from '#lib/format-number.ts';
  import { t } from '#lib/i18n.svelte.ts';
  import { planAmount, type PremiumConfig } from '#lib/premium.ts';
  import { notifySuccess } from '#lib/toast.ts';

  let {
    premium,
    hasDonated,
    hasPremium,
    adFree,
  }: {
    premium: PremiumConfig;
    /** Any character of the account donated before. */
    hasDonated: boolean;
    hasPremium: boolean;
    /** The account is on live ad-free premium, so the hint is moot. */
    adFree: boolean;
  } = $props();

  const character = $derived(premium.premium_character);

  const title = $derived(
    hasPremium
      ? 'settings.donations.cta.titleExtend'
      : hasDonated
        ? 'settings.donations.cta.titleReturn'
        : 'settings.donations.cta.titleFirst',
  );

  // The premium page's features, by their titles there.
  const FEATURES = [
    { key: 'historicSales', icon: History },
    { key: 'similarSold', icon: PackageCheck },
    { key: 'searchAlerts', icon: Bell },
    { key: 'priorityOrdering', icon: ListOrdered },
    { key: 'goldName', icon: Crown },
    { key: 'themeColor', icon: Palette },
  ] as const;

  function copyCharacter() {
    void navigator.clipboard.writeText(character);
    notifySuccess(
      t('premium.show.characterCopiedTitle'),
      t('premium.show.characterCopiedDescription', { name: character }),
    );
  }
</script>

<div
  class="relative overflow-hidden rounded-lg border border-[#d3b15f]/40 bg-gradient-to-br from-[#d3b15f]/20 via-[#d3b15f]/5 to-transparent p-6"
>
  <Crown class="absolute -top-2 -right-2 size-32 rotate-12 text-[#d3b15f]/10" />
  <span class="hud-label relative">{t('premium.show.heroLabel')}</span>
  <h2 class="relative mt-1 text-xl font-semibold">
    <span class="text-gold">{t(title)}</span>
  </h2>
  <p class="relative mt-2 max-w-prose text-sm text-muted-foreground">
    {t('settings.donations.cta.description', {
      price: toCompact(planAmount(premium, 'monthly')),
      name: character,
    })}
  </p>
  <ul class="relative mt-3 flex flex-wrap gap-2">
    {#each FEATURES as feature (feature.key)}
      <li
        class="flex items-center gap-1.5 rounded-full border border-[#d3b15f]/30 bg-background/40 px-2.5 py-1 text-xs"
      >
        <feature.icon class="size-3.5 text-[#d3b15f]" />
        {t(`premium.show.features.${feature.key}.title`)}
      </li>
    {/each}
  </ul>
  <div class="relative mt-5 flex flex-wrap items-center gap-2">
    <Button onclick={copyCharacter} class="gap-2">
      <Copy class="size-4" />
      {t('premium.show.copyName', { name: character })}
    </Button>
    <Button href="/premium" variant="secondary">{t('premium.card.details')}</Button>
  </div>
  {#if !adFree}
    <p
      class="relative mt-5 flex flex-wrap items-center gap-x-2 gap-y-1 border-t border-[#d3b15f]/20 pt-4 text-sm text-muted-foreground"
      data-testid="ad-free-hint"
    >
      <EyeOff class="size-4 shrink-0 text-[#d3b15f]" />
      <span>
        {t('settings.donations.cta.adFreeHint', {
          plan: t('premium.show.tierAdFree'),
          price: toCompact(planAmount(premium, 'monthly', true)),
        })}
      </span>
      <a href="/premium" class="font-medium text-primary hover:underline">
        {t('premium.show.adFree.confirmButton')}
      </a>
    </p>
  {/if}
</div>
