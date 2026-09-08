<script lang="ts">
  // The donations page, the legacy Donations/DonationsPage.vue: the
  // support CTA with the copy button, the 14-day and all-time
  // leaderboards, and the recent activity list.
  import { Copy, Crown, Sparkles, Trophy } from '@lucide/svelte';
  import type { PageProps } from './$types';
  import DonationsList from '$lib/components/donations-list.svelte';
  import Trans from '$lib/components/trans.svelte';
  import { t } from '$lib/i18n.svelte';
  import { Button } from '$lib/components/ui/button';
  import { notifySuccess } from '$lib/toast';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const character = $derived(data.premium.premium_character);

  // The legacy handleDonate with the premium.donations.copied* strings.
  function handleDonate() {
    void navigator.clipboard.writeText(character);
    notifySuccess(
      t('premium.donations.copiedTitle'),
      t('premium.donations.copiedDescription', { name: character }),
    );
  }
</script>

<PageMeta
  title={t('meta.donations.title')}
  description={t('meta.donations.description')}
  keywords="donations, support, isk"
/>

<div class="space-y-6">
  <!-- Support CTA -->
  <div class="rounded-lg border bg-gradient-to-r from-primary/10 via-primary/5 to-transparent p-4">
    <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <h2 class="font-semibold">{t('premium.donations.supportTitle')}</h2>
        <p class="text-sm text-muted-foreground">
          <Trans key="premium.donations.supportDescription">
            {#snippet name()}<strong class="text-foreground">{character}</strong>{/snippet}
          </Trans>
        </p>
      </div>
      <Button onclick={handleDonate} class="shrink-0 gap-2">
        <Copy class="size-4" />
        {t('premium.donations.copyCharacterName')}
      </Button>
    </div>
  </div>

  <!-- Leaderboards -->
  <div class="grid gap-4 lg:grid-cols-2">
    <div class="hud-frame">
      <div class="flex items-center gap-2 border-b px-4 py-3">
        <Trophy class="size-4 text-yellow-500" />
        <h3 class="font-medium">{t('premium.donations.top14Days')}</h3>
      </div>
      <div class="p-3">
        <DonationsList
          donations={data.donations.recent}
          emptyMessage={t('premium.donations.noRecentDonations')}
          showRank={true}
        />
      </div>
    </div>

    <div class="hud-frame">
      <div class="flex items-center gap-2 border-b px-4 py-3">
        <Crown class="size-4 text-amber-500" />
        <h3 class="font-medium">{t('premium.donations.hallOfFame')}</h3>
      </div>
      <div class="p-3">
        <DonationsList donations={data.donations.highest} showRank={true} />
      </div>
    </div>
  </div>

  <!-- Recent Activity -->
  <div class="hud-frame">
    <div class="flex items-center gap-2 border-b px-4 py-3">
      <Sparkles class="size-4 text-primary" />
      <h3 class="font-medium">{t('premium.donations.recentDonations')}</h3>
    </div>
    <div class="p-3">
      <DonationsList donations={data.donations.latest} />
    </div>
  </div>
</div>
