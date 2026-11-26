<script lang="ts">
  // The settings page's donation history, a rewrite addition: legacy
  // only confirmed a donation by EVE mail. Every donation sent by one
  // of the account's characters, newest first, with the running total.
  import { Heart } from '@lucide/svelte';
  import GameImage from './game-image.svelte';
  import { toIsk } from '#lib/format-number.ts';
  import { t } from '#lib/i18n.svelte.ts';
  import type { AccountDonation } from '#lib/settings.ts';

  let { donations }: { donations: AccountDonation[] } = $props();

  const total = $derived(donations.reduce((sum, donation) => sum + donation.amount, 0));

  function donatedOn(iso: string): string {
    return new Date(iso).toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
      timeZone: 'UTC',
    });
  }
</script>

<div class="hud-frame relative p-6">
  <Heart class="absolute top-4 right-4 size-20 text-white/5" />
  <h2 class="relative flex items-center gap-2 font-medium">
    <Heart class="size-4 text-primary" />
    {t('settings.donations.title')}
  </h2>
  <p class="relative mt-1 max-w-prose text-sm text-muted-foreground">
    {donations.length > 0
      ? t('settings.donations.description')
      : t('settings.donations.emptyDescription')}
    {t('premium.show.steps.pickup.description')}
  </p>
  <a
    href="/donations"
    class="relative mt-2 inline-flex text-sm font-medium text-primary hover:underline"
  >
    {t('settings.donations.howTo')}
  </a>
  {#if donations.length > 0}
    <ul class="mt-4 divide-y divide-border">
      {#each donations as donation (donation.id)}
        <li class="flex items-center gap-3 py-2">
          <GameImage
            src="https://images.evetech.net/characters/{donation.character.id}/portrait?size=64"
            alt={donation.character.name}
            class="size-10 rounded-md"
          />
          <div class="min-w-0 grow">
            <span class="block truncate text-sm font-medium">{donation.character.name}</span>
            <time datetime={donation.date} class="block text-xs text-muted-foreground">
              {donatedOn(donation.date)}
            </time>
          </div>
          <span class="shrink-0 text-sm tabular-nums">{toIsk(donation.amount)}</span>
        </li>
      {/each}
    </ul>
    <p class="mt-3 flex justify-between border-t border-border pt-3 text-sm font-medium">
      <span>{t('settings.donations.total')}</span>
      <span class="tabular-nums" data-testid="donation-total">{toIsk(total)}</span>
    </p>
  {/if}
</div>
