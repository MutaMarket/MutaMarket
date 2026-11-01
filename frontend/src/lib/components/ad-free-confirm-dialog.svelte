<script lang="ts">
  // The warning before an account goes ad-free: what the account's
  // longest premium time shrinks to, what the plan costs next to plain
  // premium, and how long the account then stays ad-free.
  import { ArrowRight, EyeOff, Lock } from '@lucide/svelte';
  import { Button } from '#lib/components/ui/button/index.ts';
  import * as Dialog from '#lib/components/ui/dialog/index.ts';
  import { toCompact } from '#lib/format-number.ts';
  import { t } from '#lib/i18n.svelte.ts';
  import {
    AD_FREE_MIN_DAYS,
    type PremiumConfig,
    adFreeFactor,
    premiumSpan,
    tierCosts,
  } from '#lib/premium.ts';

  let {
    open = $bindable(false),
    characters,
    premium,
    busy,
    onconfirm,
  }: {
    open?: boolean;
    characters: { id: number; name: string; premium_paid_until: string | null }[];
    premium: PremiumConfig;
    busy: boolean;
    onconfirm: () => void;
  } = $props();

  const factor = $derived(adFreeFactor(premium));
  const plain = $derived(tierCosts(premium, false));
  const adFree = $derived(tierCosts(premium, true));
  // Taken when the dialog renders: the preview only has to be right to
  // the minute.
  const now = $derived(open ? Date.now() : 0);
  // The character holding the most premium stands for the account, like
  // the sidebar status; the others convert at the same rate.
  const before = $derived(
    Math.max(
      0,
      ...characters.map((entry) =>
        entry.premium_paid_until === null ? 0 : Date.parse(entry.premium_paid_until) - now,
      ),
    ),
  );
  const after = $derived(before / factor);
  const unlockDate = $derived(
    new Date(now + AD_FREE_MIN_DAYS * 86_400_000).toISOString().slice(0, 10),
  );

  function span(ms: number): string {
    const { unit, count } = premiumSpan(ms);
    return t(`premium.show.adFree.span.${unit}`, { count });
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="max-w-lg gap-5">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <EyeOff class="size-4 text-primary" />
        {t('premium.show.adFree.confirmTitle')}
      </Dialog.Title>
      <Dialog.Description>
        {t('premium.show.adFree.confirmIntro', { factor })}
      </Dialog.Description>
    </Dialog.Header>

    <div
      class="grid grid-cols-[1fr_auto_1fr] items-center gap-3 text-center"
      data-testid="ad-free-time"
    >
      <div class="border border-border px-3 py-3">
        <p class="text-xs text-muted-foreground">{t('premium.show.tierPremium')}</p>
        <p class="mt-1 text-2xl font-semibold tabular-nums">{span(before)}</p>
      </div>
      <ArrowRight class="size-5 text-muted-foreground" />
      <div class="border border-primary bg-primary/10 px-3 py-3">
        <p class="text-xs text-primary">{t('premium.show.tierAdFree')}</p>
        <p class="mt-1 text-2xl font-semibold text-primary tabular-nums">{span(after)}</p>
      </div>
    </div>

    <section class="grid gap-2" aria-labelledby="ad-free-price">
      <h3 id="ad-free-price" class="text-xs font-medium text-muted-foreground uppercase">
        {t('premium.show.adFree.confirmPriceTitle', { factor })}
      </h3>
      <table class="w-full border border-border text-sm" data-testid="ad-free-price">
        <thead>
          <tr class="border-b border-border text-xs text-muted-foreground">
            <th class="px-3 py-1.5 text-left font-normal"></th>
            <th class="px-3 py-1.5 text-right font-normal">{t('premium.show.tierPremium')}</th>
            <th class="px-3 py-1.5 text-right font-normal text-primary">
              {t('premium.show.tierAdFree')}
            </th>
          </tr>
        </thead>
        <tbody class="tabular-nums">
          <tr>
            <th class="px-3 py-1.5 text-left font-normal text-muted-foreground">
              {t('premium.show.oneMonth')}
            </th>
            <td class="px-3 py-1.5 text-right">
              {t('premium.iskAmount', { price: toCompact(plain.monthly) })}
            </td>
            <td class="px-3 py-1.5 text-right font-semibold">
              {t('premium.iskAmount', { price: toCompact(adFree.monthly) })}
            </td>
          </tr>
          <tr>
            <th class="px-3 py-1.5 text-left font-normal text-muted-foreground">
              {t('premium.show.twelveMonths')}
            </th>
            <td class="px-3 py-1.5 text-right">
              {t('premium.iskAmount', { price: toCompact(plain.yearly) })}
            </td>
            <td class="px-3 py-1.5 text-right font-semibold">
              {t('premium.iskAmount', { price: toCompact(adFree.yearly) })}
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <p
      class="flex gap-2.5 border border-amber-500/40 bg-amber-500/10 px-3 py-2.5 text-sm"
      data-testid="ad-free-lock"
    >
      <Lock class="mt-0.5 size-4 shrink-0 text-amber-500" />
      <span>
        <span class="font-medium">
          {t('premium.show.adFree.confirmLockTitle', { days: AD_FREE_MIN_DAYS })}
        </span>
        {t('premium.show.adFree.confirmLock', { date: unlockDate })}
      </span>
    </p>

    <Dialog.Footer>
      <Button variant="secondary" onclick={() => (open = false)}>
        {t('common.actions.cancel')}
      </Button>
      <Button disabled={busy} onclick={onconfirm}>
        <EyeOff />
        {t('premium.show.adFree.confirmButton')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
