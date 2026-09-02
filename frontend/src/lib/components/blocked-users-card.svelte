<script lang="ts">
  // The settings page's blocked accounts, a rewrite addition: legacy
  // could block from an offer thread but never showed or undid a block.
  // Each account appears under the character its owner notifies with.
  import { Ban } from '@lucide/svelte';
  import GameImage from './game-image.svelte';
  import { Button } from '$lib/components/ui/button';
  import { t } from '$lib/i18n.svelte';
  import type { BlockedUser } from '$lib/settings';

  let {
    blocked,
    onUnblock,
  }: {
    blocked: BlockedUser[];
    onUnblock: (userId: number) => void | Promise<void>;
  } = $props();

  function blockedOn(iso: string): string {
    return new Date(iso).toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    });
  }
</script>

<div class="hud-frame relative mt-4 mb-4 p-6">
  <Ban class="absolute top-4 right-4 size-20 text-white/5" />
  <h2 class="relative flex items-center gap-2 font-medium">
    <Ban class="size-4 text-primary" />
    {t('settings.blockedUsers.title')}
  </h2>
  <p class="relative mt-1 text-sm text-muted-foreground">
    {blocked.length > 0
      ? t('settings.blockedUsers.description')
      : t('settings.blockedUsers.emptyDescription')}
  </p>
  {#if blocked.length > 0}
    <ul class="mt-4 divide-y divide-border">
      {#each blocked as user (user.user_id)}
        <li class="flex items-center gap-3 py-2">
          {#if user.character_id !== null}
            <GameImage
              src="https://images.evetech.net/characters/{user.character_id}/portrait?size=64"
              alt={user.name}
              class="size-10 rounded-md"
            />
          {:else}
            <div class="size-10 rounded-md border border-border"></div>
          {/if}
          <div class="min-w-0 grow">
            <span class="block truncate text-sm font-medium">{user.name}</span>
            <span class="block text-xs text-muted-foreground">
              {t('settings.blockedUsers.blockedOn', { date: blockedOn(user.blocked_at) })}
            </span>
          </div>
          <Button variant="secondary" size="sm" onclick={() => onUnblock(user.user_id)}>
            {t('settings.blockedUsers.unblock')}
          </Button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
