<script lang="ts">
  // The account character menu, ported from the Leptos character_menu
  // (itself the legacy AuthenticatedAsButton.vue + character dialog):
  // the trigger shows the acting character with a warning ping when a
  // character is missing ESI access.
  //
  // The rows are switch targets only; removing a character and granting
  // scopes live on the settings page, which has the room to say what is
  // actually missing.
  import { LogOut, Settings, TriangleAlert, UserPlus, UsersRound } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { charactersNeedingScopes, missingScopes, warnsAboutScopes } from '$lib/scopes';
  import type { ScopeInfo } from '$lib/scopes';
  import type { AccountCharacter } from '$lib/types';

  let {
    characters,
    scopeCatalogue = [],
  }: { characters: AccountCharacter[]; scopeCatalogue?: ScopeInfo[] } = $props();

  const active = $derived(characters.find((character) => character.active));
  const needingScopes = $derived(charactersNeedingScopes(characters, scopeCatalogue));

  function portrait(characterId: number): string {
    return `https://images.evetech.net/characters/${characterId}/portrait?size=64`;
  }

  function missingLabel(character: AccountCharacter): string {
    const missing = missingScopes(character, scopeCatalogue);
    return missing.length === 1
      ? `Missing ${missing[0].label.toLowerCase()} access`
      : `Missing ${missing.length} permissions`;
  }

  // The switch endpoint answers with the legacy 303; the reload of
  // nav-state (and any page data) happens through invalidateAll.
  async function switchTo(characterId: number) {
    await fetch(`/auth/character/${characterId}`, { method: 'PUT', redirect: 'manual' });
    await invalidateAll();
  }
</script>

{#if active}
  <DropdownMenu.Root>
    <!-- The legacy square icon trigger: portrait only, the name stays
		     for screen readers. -->
    <DropdownMenu.Trigger
      class="relative flex size-10 cursor-pointer items-center justify-center border-none bg-white/[0.04] text-foreground shadow-none hover:bg-white/[0.07] focus:outline-none"
    >
      <img alt="" class="size-6" src={portrait(active.id)} />
      <span class="sr-only">{active.name}</span>
      {#if needingScopes.length > 0}
        <span class="absolute -top-1 -right-1 size-2 animate-ping rounded-full bg-amber-500"></span>
        <span class="absolute -top-1 -right-1 size-2 rounded-full bg-amber-500"></span>
      {/if}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content class="min-w-72" align="end">
      <span class="block px-2 py-1.5 text-xs font-semibold text-muted-foreground"> Acting as </span>
      {#each characters as character (character.id)}
        <DropdownMenu.Item
          class="gap-3 px-2 py-2"
          onclick={() => {
            if (!character.active) {
              switchTo(character.id);
            }
          }}
        >
          <img alt="" class="size-7 rounded" src={portrait(character.id)} />
          <span class="flex min-w-0 grow flex-col">
            <span class="truncate">{character.name}</span>
            {#if warnsAboutScopes(character, scopeCatalogue)}
              <span class="truncate text-xs text-amber-500">
                {missingLabel(character)}
              </span>
            {/if}
          </span>
          {#if character.active}
            <span class="shrink-0 rounded-full bg-primary/10 px-2 py-0.5 text-[11px] text-primary">
              acting
            </span>
          {/if}
        </DropdownMenu.Item>
      {/each}

      {#if needingScopes.length > 0}
        <DropdownMenu.Separator class="my-1" />
        <DropdownMenu.Item class="gap-2 px-2 py-1.5 text-amber-500">
          {#snippet child({ props })}
            <a {...props} href="/settings#access">
              <TriangleAlert class="size-4" />
              <span>Review missing access</span>
            </a>
          {/snippet}
        </DropdownMenu.Item>
      {/if}

      <DropdownMenu.Separator class="my-1" />
      <DropdownMenu.Item class="gap-2 px-2 py-1.5">
        {#snippet child({ props })}
          <a {...props} href="/eve?add_to_account=true" rel="external">
            <UserPlus class="size-4 text-muted-foreground" />
            <span>Add character</span>
          </a>
        {/snippet}
      </DropdownMenu.Item>
      <DropdownMenu.Item class="gap-2 px-2 py-1.5">
        {#snippet child({ props })}
          <a {...props} href="/settings#access">
            <UsersRound class="size-4 text-muted-foreground" />
            <span>Manage characters</span>
          </a>
        {/snippet}
      </DropdownMenu.Item>
      <DropdownMenu.Item class="gap-2 px-2 py-1.5">
        {#snippet child({ props })}
          <a {...props} href="/settings">
            <Settings class="size-4 text-muted-foreground" />
            <span>Settings</span>
          </a>
        {/snippet}
      </DropdownMenu.Item>
      <DropdownMenu.Separator class="my-1" />
      <form method="post" action="/logout">
        <button
          type="submit"
          class="inline-flex w-full items-center gap-2 px-2 py-1.5 text-left text-sm text-destructive transition-colors hover:bg-destructive/10"
        >
          <LogOut class="size-4" />
          <span>Log out</span>
        </button>
      </form>
    </DropdownMenu.Content>
  </DropdownMenu.Root>
{/if}
