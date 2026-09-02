<script lang="ts">
  // The right sidebar, the legacy Sidebar/Sidebar.vue composition:
  // bookmarks, the in-app advertisement rotation, recommended gear,
  // the premium card, the top-donors card, partner links and the
  // wormhole.systems partner card. Unported legacy extra,
  // deliberately: the Adsense slots (need a Google client id).
  import {
    BookmarkIcon,
    Check,
    ChevronRight,
    Copy,
    Crown,
    ExternalLink,
    Heart,
    Pencil,
    Plus,
    Send,
    Star,
    Trophy,
    X,
  } from '@lucide/svelte';
  import { page } from '$app/state';
  import DonationsList from './donations-list.svelte';
  import GameImage from './game-image.svelte';
  import Logo from './logo.svelte';
  import Trans from './trans.svelte';
  import Autoplay from 'embla-carousel-autoplay';
  import PatreonIcon from './patreon-icon.svelte';
  import * as Carousel from '$lib/components/ui/carousel';
  import WormholeSystemsIcon from './wormhole-systems-icon.svelte';
  import { routeIcon, sortBookmarks } from '$lib/bookmark-routes';
  import { toCompact, toCompactShort } from '$lib/format-number';
  import { t } from '$lib/i18n.svelte';
  import { MARKEEDRAGON_CODE } from '$lib/partner-links';
  import { premiumFromSidebar } from '$lib/premium';
  import {
    KOFI_LINK,
    PATREON_LINK,
    createBookmark,
    deleteBookmark,
    renameBookmark,
    sidebarData,
    visibleDiscordInvites,
  } from '$lib/sidebar';
  import { notifySuccess } from '$lib/toast';

  const data = $derived($sidebarData ?? page.data.sidebar ?? null);
  const premium = $derived(premiumFromSidebar(data));
  const bookmarks = $derived(data?.bookmarks == null ? null : sortBookmarks(data.bookmarks));
  const discordInvites = $derived(visibleDiscordInvites(data?.discord_invites ?? []));

  // The legacy add-current-page defaults: the panel type names the
  // bookmark when the page carries one.
  function addCurrentPage() {
    const panel = page.data.panel as { type_id: number; type_name: string } | undefined | null;
    void createBookmark(
      location.pathname,
      panel?.type_name ?? t('misc.sidebar.defaultBookmarkName'),
      panel?.type_id ?? null,
    );
  }

  let editingId = $state<number | null>(null);
  let editName = $state('');

  function startRename(id: number, name: string) {
    editingId = id;
    editName = name;
  }

  async function submitRename() {
    if (editingId !== null && editName.trim() !== '') {
      await renameBookmark(editingId, editName.trim());
    }
    editingId = null;
  }

  // The legacy AppAdvertisements.vue and RecommendedGear.vue: looping
  // embla carousels that autoplay every 10 and 12 seconds.
  const AD_ROTATE_MS = 10_000;
  const GEAR_ROTATE_MS = 12_000;
  const ads = $derived(data?.advertisements.filter((ad) => ad.image_url !== null) ?? []);
  const gear = $derived(data?.gear_items.filter((item) => item.image_url !== null) ?? []);

  // The legacy PremiumCard.vue and Sidebar Donations.vue copy handlers
  // share the premium.copyNotification strings.
  function copyPremiumCharacter() {
    void navigator.clipboard.writeText(premium.premium_character);
    notifySuccess(
      t('premium.copyNotification.title'),
      t('premium.copyNotification.description', { name: premium.premium_character }),
    );
  }

  /** The legacy MarkeeDragon coupon (+3% off, calculator locale). */
  const MARKEE_CODE = MARKEEDRAGON_CODE;
</script>

<div class="hidden w-[250px] shrink-0 flex-col gap-4 self-start xl:flex">
  {#if bookmarks !== null}
    <div class="hud-frame overflow-hidden">
      <div class="flex items-center justify-between px-3 py-2">
        <div class="flex items-center gap-1.5">
          <BookmarkIcon class="size-3.5 text-primary" />
          <span class="text-sm font-medium">{t('misc.sidebar.bookmarks')}</span>
          {#if bookmarks.length > 0}
            <span class="rounded-full bg-card-2 px-1.5 text-xs text-muted-foreground tabular-nums">
              {bookmarks.length}
            </span>
          {/if}
        </div>
        <button
          type="button"
          class="flex cursor-pointer items-center justify-center rounded-full bg-primary/10 p-1 text-primary transition-colors hover:bg-primary/20"
          title={t('misc.sidebar.addCurrentPage')}
          onclick={addCurrentPage}
        >
          <Plus class="size-3.5" />
        </button>
      </div>
      <div class="max-h-64 space-y-px overflow-y-auto px-2 pb-2">
        {#each bookmarks as bookmark (bookmark.id)}
          {@const Icon = routeIcon(bookmark.query)}
          {#if editingId === bookmark.id}
            <form
              class="flex items-center gap-1.5 rounded px-1.5 py-1"
              onsubmit={(event) => {
                event.preventDefault();
                void submitRename();
              }}
            >
              <Icon class="size-3.5 shrink-0 text-muted-foreground" />
              <!-- svelte-ignore a11y_autofocus -->
              <input
                bind:value={editName}
                autofocus
                class="w-full min-w-0 rounded border border-border bg-card-2 px-1 text-xs outline-none"
                onkeydown={(event) => {
                  if (event.key === 'Escape') editingId = null;
                }}
              />
              <button
                type="submit"
                class="cursor-pointer text-primary"
                aria-label={t('common.actions.save')}
              >
                <Check class="size-3" />
              </button>
            </form>
          {:else}
            <div
              class="group relative flex items-center gap-1.5 rounded px-1.5 py-1 transition-colors hover:bg-card-2"
            >
              {#if bookmark.type_id !== null}
                <GameImage
                  src="https://images.evetech.net/types/{bookmark.type_id}/icon?size=64"
                  alt=""
                  class="size-3.5 shrink-0 rounded-sm"
                />
              {:else}
                <Icon class="size-3.5 shrink-0 text-muted-foreground" />
              {/if}
              <a href={bookmark.query} class="min-w-0 flex-1 truncate text-xs font-medium">
                {bookmark.name}
                <span class="absolute inset-0"></span>
              </a>
              <button
                type="button"
                class="relative z-10 cursor-pointer text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-foreground"
                aria-label={t('common.actions.edit')}
                onclick={() => startRename(bookmark.id, bookmark.name)}
              >
                <Pencil class="size-3" />
              </button>
              <button
                type="button"
                class="relative z-10 cursor-pointer text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-red-500"
                aria-label={t('common.actions.delete')}
                onclick={() => deleteBookmark(bookmark.id)}
              >
                <X class="size-3" />
              </button>
            </div>
          {/if}
        {:else}
          <div class="flex flex-col items-center gap-2 py-6">
            <div class="flex size-10 items-center justify-center rounded-full bg-card-2">
              <Star class="size-5 text-muted-foreground/50" />
            </div>
            <div class="text-center">
              <p class="text-xs text-muted-foreground">{t('misc.sidebar.noBookmarks')}</p>
              <button
                type="button"
                class="cursor-pointer text-xs text-primary hover:underline"
                onclick={addCurrentPage}
              >
                {t('misc.sidebar.addThisPage')}
              </button>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if ads.length > 0}
    <Carousel.Root
      opts={{ loop: true }}
      plugins={[Autoplay({ delay: AD_ROTATE_MS })]}
      class="relative w-full overflow-hidden rounded-lg"
    >
      <Carousel.Content>
        {#each ads as ad (ad.id)}
          <Carousel.Item>
            <a href={ad.link ?? '#'} rel="noopener noreferrer" target="_blank">
              <img
                alt={ad.name}
                src={ad.image_url}
                class="aspect-[250/300] w-full object-cover {ad.link?.includes('markeedragon') &&
                MARKEE_CODE !== ''
                  ? 'rounded-t-lg'
                  : 'rounded-lg'}"
              />
            </a>
            {#if ad.link?.includes('markeedragon') && MARKEE_CODE !== ''}
              <!-- The affiliate coupon as the creative's own bottom section:
                   the same link as the artwork, no hover chrome. -->
              <a
                href={ad.link}
                rel="noopener noreferrer"
                target="_blank"
                class="-mt-4 flex w-full items-center justify-center gap-2 rounded-b-lg border-t border-white/15 bg-black px-3 py-3 text-sm font-bold text-white"
              >
                <Trans key="premium.ads.useCodeForDiscount">
                  {#snippet code()}
                    <code class="font-mono font-bold text-primary">{MARKEE_CODE}</code>
                  {/snippet}
                </Trans>
              </a>
            {/if}
          </Carousel.Item>
        {/each}
      </Carousel.Content>
      <div
        class="pointer-events-none absolute top-0 left-0 rounded-br-lg bg-black/30 p-1 py-0.5 text-xs"
      >
        {t('premium.ads.advertisement')}
      </div>
    </Carousel.Root>
  {/if}

  {#if gear.length > 0}
    <div class="w-full">
      <Carousel.Root
        opts={{ loop: true }}
        plugins={[Autoplay({ delay: GEAR_ROTATE_MS })]}
        class="relative w-full overflow-hidden rounded-lg"
      >
        <Carousel.Content>
          {#each gear as item (item.id)}
            <Carousel.Item>
              <a href={item.link} rel="sponsored nofollow noopener" target="_blank">
                <img
                  alt={item.name}
                  title={item.description ?? item.name}
                  src={item.image_url}
                  class="aspect-square w-full rounded-lg object-cover"
                />
              </a>
            </Carousel.Item>
          {/each}
        </Carousel.Content>
      </Carousel.Root>
      <p class="px-1 pt-1 text-[10px] leading-snug text-muted-foreground">
        {t('misc.gear.disclosure')}
      </p>
    </div>
  {/if}

  <div class="hud-frame">
    <div class="flex items-center justify-between border-b border-border px-3 py-2">
      <div class="flex items-center gap-1.5">
        <Crown class="size-3.5 text-primary" />
        <span class="text-sm font-medium">{t('premium.card.title')}</span>
      </div>
      <span class="flex items-center gap-0.5 text-xs text-muted-foreground">
        <ChevronRight class="size-3" />
      </span>
    </div>
    <div class="space-y-2 p-3">
      <p class="text-xs text-muted-foreground">
        {t('premium.card.upsell')}
      </p>
      <div class="space-y-1 text-xs">
        <div class="flex items-baseline justify-between">
          <span class="text-muted-foreground">{t('premium.card.monthly')}</span>
          <span class="font-medium">
            {t('premium.iskAmount', { price: toCompact(premium.premium_cost) })}
          </span>
        </div>
        <div class="flex items-baseline justify-between">
          <span class="text-muted-foreground">{t('premium.card.yearly')}</span>
          <span class="font-medium">
            {t('premium.iskAmount', { price: toCompact(premium.premium_yearly_cost) })}
          </span>
        </div>
        <p class="text-[10px] text-primary">{t('premium.card.yearlySavings')}</p>
      </div>
    </div>
    <div class="border-t border-border px-3 py-2">
      <button
        type="button"
        class="flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground"
        onclick={copyPremiumCharacter}
      >
        <Send class="size-3" />
        <span>{t('premium.card.sendIskTo')}</span>
        <code class="rounded bg-card-2 px-1 py-0.5 font-mono">{premium.premium_character}</code>
        <Copy class="size-3" />
      </button>
    </div>
  </div>

  {#if PATREON_LINK !== ''}
    <a
      href={PATREON_LINK}
      class="hud-frame flex items-center gap-4 p-2"
      rel="noopener noreferrer"
      target="_blank"
    >
      <div class="grid size-10 place-items-center rounded-md bg-white p-2">
        <PatreonIcon class="size-6 text-black" />
      </div>
      <div>
        <span class="mb-1 block leading-none">{t('premium.ads.patreonTitle')}</span>
        <span class="block text-xs leading-none text-muted-foreground">
          {t('premium.ads.patreonDescription')}
        </span>
      </div>
    </a>
  {/if}

  <!-- The legacy Sidebar Donations.vue: the 14-day top donors with the
	     donate-to shortcut, between the Patreon and Ko-fi cards. -->
  <div class="hud-frame">
    <div class="flex items-center justify-between border-b border-border px-3 py-2">
      <div class="flex items-center gap-1.5">
        <Trophy class="size-3.5 text-primary" />
        <span class="text-sm font-medium">{t('premium.donations.topDonors')}</span>
        <span class="text-xs text-muted-foreground">{t('premium.donations.days14')}</span>
      </div>
      <a
        href="/donations"
        class="flex items-center gap-0.5 text-xs text-muted-foreground transition-colors hover:text-foreground"
      >
        <span>{t('common.labels.all')}</span>
        <ChevronRight class="size-3" />
      </a>
    </div>
    <div class="p-2">
      <DonationsList donations={data?.donations.recent ?? []} showRank={true} />
    </div>
    <div class="border-t border-border px-3 py-2">
      <button
        type="button"
        class="flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground"
        onclick={copyPremiumCharacter}
      >
        <Heart class="size-3" />
        <span>{t('premium.donations.donateTo')}</span>
        <code class="rounded bg-muted px-1 py-0.5 font-mono">{premium.premium_character}</code>
        <Copy class="size-3" />
      </button>
    </div>
  </div>

  {#if KOFI_LINK !== ''}
    <a
      href={KOFI_LINK}
      class="hud-frame flex items-center gap-4 p-2"
      rel="noopener noreferrer"
      target="_blank"
    >
      <div class="grid size-10 place-items-center rounded-md bg-card p-2">
        <img src="/img/kofi.webp" alt="Ko-fi" class="max-h-full max-w-full" />
      </div>
      <div>
        <span class="mb-1 block leading-none">{t('premium.ads.kofiTitle')}</span>
        <span class="block text-xs leading-none text-muted-foreground">
          {t('premium.ads.kofiDescription')}
        </span>
      </div>
    </a>
  {/if}

  {#if discordInvites.length > 0}
    <div class="hud-frame">
      <div class="border-b border-border px-3 py-2">
        <span class="text-sm font-medium">{t('misc.sidebar.partnerDiscords')}</span>
      </div>
      <div class="space-y-px p-2">
        {#each discordInvites as invite (invite.name)}
          <a
            href={invite.url}
            target="_blank"
            rel="noopener noreferrer"
            class="group relative flex items-center gap-2 rounded px-1.5 py-1 transition-colors hover:bg-card-2"
          >
            {#if invite.image}
              <img src={invite.image} alt={invite.name} class="size-5 shrink-0 rounded" />
            {:else}
              <span class="flex size-5 shrink-0 items-center justify-center">
                <Logo class="size-4 text-primary" />
              </span>
            {/if}
            <span class="flex-1 truncate text-xs font-medium">{invite.name}</span>
            {#if invite.member_count}
              <span
                class="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground tabular-nums"
              >
                {toCompactShort(invite.member_count)}
              </span>
            {/if}
            <ExternalLink
              class="absolute right-1.5 size-3 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
            />
          </a>
        {/each}
      </div>
    </div>
  {/if}

  <div class="hud-frame">
    <div class="flex items-center justify-between border-b border-border px-3 py-2">
      <div class="flex items-center gap-1.5">
        <Star class="size-3.5 text-primary" />
        <span class="text-sm font-medium">{t('misc.sidebar.partner')}</span>
      </div>
    </div>
    <a
      href="https://wormhole.systems"
      target="_blank"
      rel="noopener noreferrer"
      class="group flex items-center gap-3 p-3 transition-colors hover:bg-card-2"
    >
      <div class="flex size-10 shrink-0 items-center justify-center rounded bg-black">
        <WormholeSystemsIcon class="size-6 text-white" />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1.5">
          <span class="truncate text-sm font-medium">WormholeSystems</span>
          <ExternalLink
            class="size-3 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
          />
        </div>
        <span class="block text-xs text-muted-foreground">{t('misc.sidebar.wormholeMapping')}</span>
      </div>
    </a>
  </div>
</div>
