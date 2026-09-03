<script lang="ts">
  // The account settings page, the legacy ShowSettingsPage: the
  // notification-character card, the three linked-account connection
  // cards with their show-on-profiles toggles, and the raffle-wins
  // card (empty until the raffle system is ported).
  import {
    Bell,
    Check,
    Copy,
    Eye,
    EyeOff,
    KeyRound,
    Mail,
    Minus,
    Star,
    TriangleAlert,
  } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import BlockedUsersCard from '$lib/components/blocked-users-card.svelte';
  import BrandIcon from '$lib/components/brand-icon.svelte';
  import GameImage from '$lib/components/game-image.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as Select from '$lib/components/ui/select';
  import { Switch } from '$lib/components/ui/switch';
  import { t } from '$lib/i18n.svelte';
  import { grantUrl, missingScopes, requiredScopes } from '$lib/scopes';
  import { maskCode, type LinkedAccount } from '$lib/settings';
  import { notifyError, notifySuccess } from '$lib/toast';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const EVE_CODE_ACTIVATION_URL = 'https://secure.eveonline.com/code-activation';

  // The effective notify target mirrors the sender's fallback: the
  // explicit pick, or the account's first character.
  const notifyCharacter = $derived(
    data.settings.character_to_notify ?? data.settings.characters[0] ?? null,
  );

  async function pickNotifyCharacter(value: string | undefined) {
    if (value === undefined) {
      return;
    }
    const response = await fetch(`/settings?character_to_notify=${value}`, { method: 'PUT' });
    if (response.ok) {
      notifySuccess(
        t('settings.notifications.updatedTitle'),
        t('settings.notifications.updatedBody'),
      );
      await invalidateAll();
    } else {
      const body = await response.json().catch(() => ({ message: undefined }));
      notifyError(
        t('settings.notifications.notUpdatedTitle'),
        body.message ?? t('errors.internalServerError.name'),
      );
    }
  }

  const BRAND_LABEL = { discord: 'Discord', twitch: 'Twitch', patreon: 'Patreon' } as const;

  async function toggleVisibility(brand: keyof typeof BRAND_LABEL, account: LinkedAccount) {
    const next = account.is_public ? '0' : '1';
    const response = await fetch(`/${brand}?is_public=${next}`, { method: 'PUT' });
    if (response.ok || response.redirected) {
      notifySuccess(
        t('settings.connections.updatedTitle', { brand: BRAND_LABEL[brand] }),
        t('settings.connections.updatedBody', { brand: BRAND_LABEL[brand] }),
      );
      await invalidateAll();
    } else {
      notifyError(
        t('settings.notifications.notUpdatedTitle'),
        t('errors.internalServerError.name'),
      );
    }
  }

  // The legacy reveal/copy controls of the prize codes.
  let revealed = $state<Set<number>>(new Set());
  function toggleReveal(id: number) {
    const next = new Set(revealed);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    revealed = next;
  }
  async function copyCode(code: string) {
    await navigator.clipboard.writeText(code);
    notifySuccess(t('settings.raffleWins.copiedTitle'), t('settings.raffleWins.copiedDescription'));
  }

  // One row per character: what it granted, what it still needs, and
  // the re-authorization link that closes the gap.
  const accessRows = $derived(
    (data.nav?.characters ?? []).map((character) => ({
      character,
      required: requiredScopes(data.nav?.scope_catalogue ?? []),
      missing: missingScopes(character, data.nav?.scope_catalogue ?? []),
      grantUrl: grantUrl(character, data.nav?.scope_catalogue ?? []),
    })),
  );
  // The last character cannot be removed, like the legacy guard.
  const canRemove = $derived((data.nav?.characters.length ?? 0) > 1);

  async function muteWarnings(characterId: number, muted: boolean) {
    const response = await fetch(`/characters/${characterId}/scope-warnings`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ muted }),
    });
    if (response.ok) {
      await invalidateAll();
    } else {
      notifyError(t('settings.access.warningNotUpdated'), t('settings.access.tryAgain'));
    }
  }

  async function removeCharacter(characterId: number) {
    const response = await fetch(`/auth/character/${characterId}`, {
      method: 'DELETE',
      redirect: 'manual',
    });
    if (response.ok || response.type === 'opaqueredirect') {
      notifySuccess(t('settings.access.removedTitle'), t('settings.access.removedBody'));
      await invalidateAll();
    } else {
      notifyError(t('settings.access.removeFailed'), t('settings.access.tryAgain'));
    }
  }

  const connections = $derived(
    (['discord', 'twitch', 'patreon'] as const).map((brand) => ({
      brand,
      account: data.settings[brand],
    })),
  );

  // A stale avatar URL (the provider drops old ones when the picture
  // changes) falls back to the brand placeholder instead of a broken
  // image.
  let failedAvatars = $state<Set<string>>(new Set());
  function avatarFailed(url: string) {
    failedAvatars = new Set(failedAvatars).add(url);
  }

  async function unblock(userId: number) {
    const response = await fetch(`/blocked-users/${userId}`, {
      method: 'DELETE',
      redirect: 'manual',
    });
    if (response.ok || response.type === 'opaqueredirect') {
      notifySuccess(
        t('settings.blockedUsers.unblockedTitle'),
        t('settings.blockedUsers.unblockedBody'),
      );
      await invalidateAll();
      return;
    }
    notifyError(
      t('settings.blockedUsers.unblockFailedTitle'),
      t('settings.blockedUsers.unblockFailedBody'),
    );
  }
</script>

<PageMeta title={t('meta.settings.title')} description={t('meta.settings.description')} />

<PageHeader title={t('meta.settings.title')} subtitle={t('settings.page.subtitle')} />

<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-2 xl:grid-cols-4">
  <!-- Notification character -->
  <div class="hud-frame relative flex flex-col p-6">
    <Mail class="absolute top-4 right-4 size-20 text-white/5" />
    <h2 class="relative flex items-center gap-2 font-medium">
      <Bell class="size-4 text-primary" />
      {t('settings.notificationCard.title')}
    </h2>
    <p class="relative mt-1 text-sm text-muted-foreground">
      {t('settings.notificationCard.description')}
    </p>
    {#if notifyCharacter !== null}
      <div class="mt-4 flex items-center gap-3">
        <GameImage
          src="https://images.evetech.net/characters/{notifyCharacter.id}/portrait?size=64"
          alt={notifyCharacter.name}
          class="size-12 rounded-md"
        />
        <span class="text-lg font-medium">{notifyCharacter.name}</span>
        {#if data.settings.character_to_notify === null}
          <span class="text-xs text-muted-foreground">{t('settings.notificationCard.default')}</span
          >
        {/if}
      </div>
      <div class="mt-auto pt-4">
        <Select.Root
          type="single"
          value={String(notifyCharacter.id)}
          onValueChange={pickNotifyCharacter}
        >
          <Select.Trigger class="h-10 w-full">
            {t('settings.notificationCard.changeCharacter')}
          </Select.Trigger>
          <Select.Content>
            {#each data.settings.characters as character (character.id)}
              <Select.Item value={String(character.id)}>
                <span class="flex items-center gap-2">
                  <GameImage
                    src="https://images.evetech.net/characters/{character.id}/portrait?size=64"
                    alt={character.name}
                    class="size-6 rounded-sm"
                  />
                  {character.name}
                </span>
              </Select.Item>
            {/each}
          </Select.Content>
        </Select.Root>
      </div>
    {:else}
      <p class="mt-4 text-sm text-muted-foreground">
        {t('settings.notificationCard.noCharacters')}
      </p>
    {/if}
  </div>

  <!-- Linked accounts -->
  {#each connections as { brand, account } (brand)}
    <div class="hud-frame relative flex flex-col p-6">
      <BrandIcon {brand} class="absolute top-4 right-4 size-20 text-white/5" />
      <h2 class="relative flex items-center gap-2 font-medium">
        <BrandIcon {brand} class="size-4 text-primary" />
        {BRAND_LABEL[brand]}
      </h2>
      <p class="relative mt-1 text-sm text-muted-foreground">
        {account
          ? t(`settings.${brand}Card.connectedDescription`)
          : t(`settings.${brand}Card.notConnectedDescription`)}
      </p>
      <div class="mt-4 flex items-center gap-3">
        {#if account?.avatar && !failedAvatars.has(account.avatar)}
          {@const avatar = account.avatar}
          <img
            src={avatar}
            alt={t(`settings.${brand}Card.avatarAlt`)}
            class="size-12 rounded-xl ring-2 ring-border/10"
            onerror={() => avatarFailed(avatar)}
          />
        {:else}
          <div
            class="grid size-12 place-items-center rounded-xl border border-dashed border-border"
          >
            <BrandIcon {brand} class="size-6 text-muted-foreground" />
          </div>
        {/if}
        <div class="flex items-center gap-2">
          <span class="inline-block size-2 rounded-full {account ? 'bg-green-500' : 'bg-red-500'}"
          ></span>
          <span class="text-lg font-medium">
            {account?.name ?? t('settings.connections.notConnected')}
          </span>
        </div>
      </div>
      <div class="mt-auto flex flex-wrap items-center justify-between gap-3 pt-4">
        {#if account}
          <label class="flex items-center gap-2 text-sm whitespace-nowrap">
            <Switch
              checked={account.is_public}
              onCheckedChange={() => toggleVisibility(brand, account)}
            />
            {t('settings.connections.showOnProfiles')}
          </label>
          <div class="flex gap-2">
            <!-- Re-running the connect flow re-stores the current
						     name and avatar, refreshing a stale picture. -->
            <Button
              variant="outline"
              size="sm"
              href="/{brand}"
              rel="external"
              title={t('settings.connections.refreshHint', { brand: BRAND_LABEL[brand] })}
            >
              {t('common.actions.refresh')}
            </Button>
            <Button variant="outline" size="sm" href="/{brand}?switch=true" rel="external">
              {t('settings.connections.switchAccount')}
            </Button>
          </div>
        {:else}
          <Button href="/{brand}" rel="external">{t(`settings.${brand}Card.connect`)}</Button>
        {/if}
      </div>
    </div>
  {/each}
</div>

<!-- ESI access per character -->
<section id="access" class="hud-frame relative mt-4 p-6">
  <h2 class="relative flex items-center gap-2 font-medium">
    <KeyRound class="size-4 text-primary" />
    {t('settings.access.title')}
  </h2>
  <p class="relative mt-1 text-sm text-muted-foreground">
    {t('settings.access.description')}
  </p>

  <div class="mt-5 flex flex-col gap-4">
    {#each accessRows as row (row.character.id)}
      <div class="rounded-lg border border-border p-4">
        <div class="flex flex-wrap items-center gap-3">
          <GameImage
            src="https://images.evetech.net/characters/{row.character.id}/portrait?size=64"
            alt={row.character.name}
            class="size-10 rounded-lg"
          />
          <div class="min-w-0 grow">
            <div class="flex items-center gap-2">
              <span class="truncate font-medium">{row.character.name}</span>
              {#if row.character.active}
                <span class="rounded-full bg-primary/10 px-2 py-0.5 text-[11px] text-primary">
                  {t('settings.access.acting')}
                </span>
              {/if}
            </div>
            <p class="text-xs text-muted-foreground">
              {#if row.missing.length === 0}
                {t('settings.access.allGranted')}
              {:else}
                {t('settings.access.missingCount', {
                  missing: row.missing.length,
                  required: row.required.length,
                })}
              {/if}
            </p>
          </div>
          {#if row.missing.length > 0}
            <Button href={row.grantUrl} rel="external" size="sm">
              {t('settings.access.grantAccess')}
            </Button>
          {/if}
          {#if canRemove}
            <Button
              size="sm"
              variant="ghost"
              class="text-destructive hover:bg-destructive/10"
              onclick={() => removeCharacter(row.character.id)}
            >
              {t('common.actions.remove')}
            </Button>
          {/if}
        </div>

        <ul class="mt-4 grid gap-2 sm:grid-cols-2">
          {#each data.nav?.scope_catalogue ?? [] as scope (scope.id)}
            {@const granted = row.character.granted_scopes.includes(scope.id)}
            <li class="flex items-start gap-2">
              {#if granted}
                <Check class="mt-0.5 size-4 shrink-0 text-positive" />
              {:else if scope.optional}
                <Minus class="mt-0.5 size-4 shrink-0 text-muted-foreground" />
              {:else}
                <TriangleAlert class="mt-0.5 size-4 shrink-0 text-amber-500" />
              {/if}
              <span class="min-w-0">
                <span class="block text-sm {granted ? '' : 'text-muted-foreground'}">
                  {scope.label}
                  {#if scope.optional && !granted}
                    <span class="text-xs">{t('settings.access.optional')}</span>
                  {/if}
                </span>
                <span class="block text-xs text-muted-foreground">{scope.description}</span>
              </span>
            </li>
          {/each}
        </ul>

        {#if row.missing.length > 0 || row.character.scope_warnings_muted}
          <label class="mt-4 flex items-center gap-3 text-sm">
            <Switch
              checked={row.character.scope_warnings_muted}
              onCheckedChange={(checked) => muteWarnings(row.character.id, checked)}
            />
            <span class="text-muted-foreground">{t('settings.access.muteWarning')}</span>
          </label>
        {/if}
      </div>
    {/each}
  </div>

  <div class="mt-4 flex flex-wrap gap-2">
    <Button href="/eve?add_to_account=true" rel="external" size="sm" variant="secondary">
      {t('nav.desktop.addCharacter')}
    </Button>
    <Button href="/eve/corporation" rel="external" size="sm" variant="secondary">
      {t('nav.desktop.addCorporationScopes')}
    </Button>
  </div>
</section>

<BlockedUsersCard blocked={data.settings.blocked_users} onUnblock={unblock} />

<!-- Raffle wins -->
<div class="hud-frame relative mt-4 mb-4 p-6">
  <Star class="absolute top-4 right-4 size-20 text-white/5" />
  <h2 class="relative flex items-center gap-2 font-medium">
    <Star class="size-4 text-primary" />
    {t('settings.raffleWins.title')}
  </h2>
  <p class="relative mt-1 text-sm text-muted-foreground">
    {data.settings.raffle_wins.length > 0
      ? t('settings.raffleWins.redeemDescription')
      : t('settings.raffleWins.emptyDescription')}
  </p>
  <a
    href={EVE_CODE_ACTIVATION_URL}
    target="_blank"
    rel="noopener noreferrer"
    class="relative mt-2 inline-flex text-sm font-medium text-primary hover:underline"
  >
    {t('settings.raffleWins.openCodeActivation')}
  </a>
  {#if data.settings.raffle_wins.length > 0}
    <div class="mt-4 grid grid-cols-[auto_1fr_1fr_auto_auto] items-center gap-3">
      {#each data.settings.raffle_wins as win (win.id)}
        <GameImage
          src="https://images.evetech.net/types/{win.type?.id}/icon?size=64"
          alt={win.type?.name ?? win.name}
          class="size-10 rounded-lg"
        />
        <div class="min-w-0">
          <h3 class="truncate text-sm font-medium">{win.name}</h3>
          {#if win.description}
            <p class="truncate text-xs text-muted-foreground">{win.description}</p>
          {/if}
        </div>
        <Input
          value={revealed.has(win.id) ? win.code : maskCode(win.code)}
          class="font-mono text-xs"
          readonly
        />
        <Button
          size="icon"
          variant="secondary"
          aria-label={revealed.has(win.id)
            ? t('settings.raffles.hideCode')
            : t('settings.raffles.showCode')}
          onclick={() => toggleReveal(win.id)}
        >
          {#if revealed.has(win.id)}
            <EyeOff class="size-4" />
          {:else}
            <Eye class="size-4" />
          {/if}
        </Button>
        <Button
          size="icon"
          variant="secondary"
          aria-label={t('settings.raffles.copyCode')}
          onclick={() => copyCode(win.code)}
        >
          <Copy class="size-4" />
        </Button>
      {/each}
    </div>
  {:else}
    <div class="mt-4 flex items-center gap-3 py-2 text-muted-foreground">
      <Star class="size-5" />
      <span class="text-sm">{t('settings.raffleWins.noPrizesYet')}</span>
    </div>
  {/if}
</div>
