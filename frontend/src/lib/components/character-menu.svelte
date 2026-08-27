<script lang="ts">
	// The account character menu, ported from the Leptos character_menu
	// (itself the legacy AuthenticatedAsButton.vue + character dialog):
	// the trigger shows the acting character with a warning ping when any
	// character lacks the asset scope; the menu lists the account's
	// characters to act as, plus add-character, corporation-scope and
	// remove actions.
	import { invalidateAll } from '$app/navigation';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import type { AccountCharacter } from '$lib/types';

	let { characters }: { characters: AccountCharacter[] } = $props();

	const active = $derived(characters.find((character) => character.active));
	const missingScopes = $derived(characters.some((character) => !character.has_asset_token));
	const removable = $derived(characters.length > 1);

	function portrait(characterId: number): string {
		return `https://images.evetech.net/characters/${characterId}/portrait?size=64`;
	}

	// The switch/remove endpoints answer with the legacy 303; the reload
	// of nav-state (and any page data) happens through invalidateAll.
	async function act(method: 'PUT' | 'DELETE', characterId: number) {
		await fetch(`/auth/character/${characterId}`, { method, redirect: 'manual' });
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
			{#if missingScopes}
				<span class="absolute -top-1 -right-1 size-2 animate-ping rounded-full bg-red-500"
				></span>
			{/if}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="min-w-64" align="end">
			<span class="block px-2 py-1.5 text-xs font-semibold text-muted-foreground">
				Characters
			</span>
			{#each characters as character (character.id)}
				<div class="flex items-center gap-1">
					<DropdownMenu.Item
						class="grow px-2 py-1.5"
						onclick={() => {
							if (!character.active) {
								act('PUT', character.id);
							}
						}}
					>
						<img alt="" class="size-6 rounded" src={portrait(character.id)} />
						<span class="grow truncate">{character.name}</span>
						{#if character.active}
							<span class="text-xs text-muted-foreground">acting</span>
						{/if}
						{#if !character.has_asset_token}
							<span class="size-1.5 rounded-full bg-red-500" title="missing asset scope"></span>
						{/if}
					</DropdownMenu.Item>
					{#if removable && !character.active}
						<DropdownMenu.Item
							class="w-auto shrink-0 px-2 py-1.5 text-xs"
							variant="destructive"
							onclick={() => act('DELETE', character.id)}
						>
							Remove
						</DropdownMenu.Item>
					{/if}
				</div>
			{/each}
			<DropdownMenu.Separator class="my-1" />
			<DropdownMenu.Item class="px-2 py-1.5">
				{#snippet child({ props })}
					<a {...props} href="/eve?add_to_account=true" rel="external">Add Character</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Item class="px-2 py-1.5">
				{#snippet child({ props })}
					<a {...props} href="/eve/corporation" rel="external">Add Corporation Scopes</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Item class="px-2 py-1.5">
				{#snippet child({ props })}
					<a {...props} href="/settings">Settings</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Separator class="my-1" />
			<form method="post" action="/logout">
				<button
					type="submit"
					class="inline-flex w-full items-center gap-2 px-2 py-1.5 text-left text-sm text-destructive transition-colors hover:bg-destructive/10"
				>
					Log out
				</button>
			</form>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
{/if}
