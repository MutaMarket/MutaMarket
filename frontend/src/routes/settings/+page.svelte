<script lang="ts">
	// The account settings page, the legacy ShowSettingsPage: the
	// notification-character card, the three linked-account connection
	// cards with their show-on-profiles toggles, and the raffle-wins
	// card (empty until the raffle system is ported).
	import { Bell, Copy, Eye, EyeOff, Mail, Star } from '@lucide/svelte';
	import { invalidateAll } from '$app/navigation';
	import BrandIcon from '$lib/components/brand-icon.svelte';
	import GameImage from '$lib/components/game-image.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Select from '$lib/components/ui/select';
	import { Switch } from '$lib/components/ui/switch';
	import { maskCode, type LinkedAccount } from '$lib/settings';
	import { notifyError, notifySuccess } from '$lib/toast';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const EVE_CODE_ACTIVATION_URL = 'https://secure.eveonline.com/code-activation';

	// The effective notify target mirrors the sender's fallback: the
	// explicit pick, or the account's first character.
	const notifyCharacter = $derived(
		data.settings.character_to_notify ?? data.settings.characters[0] ?? null
	);

	async function pickNotifyCharacter(value: string | undefined) {
		if (value === undefined) {
			return;
		}
		const response = await fetch(`/settings?character_to_notify=${value}`, { method: 'PUT' });
		if (response.ok) {
			notifySuccess('Settings updated.', 'Your settings have been updated.');
			await invalidateAll();
		} else {
			const body = await response.json().catch(() => ({ message: undefined }));
			notifyError('Settings not updated', body.message ?? 'Something went wrong.');
		}
	}

	const BRAND_LABEL = { discord: 'Discord', twitch: 'Twitch', patreon: 'Patreon' } as const;

	async function toggleVisibility(brand: keyof typeof BRAND_LABEL, account: LinkedAccount) {
		const next = account.is_public ? '0' : '1';
		const response = await fetch(`/${brand}?is_public=${next}`, { method: 'PUT' });
		if (response.ok || response.redirected) {
			notifySuccess(
				`${BRAND_LABEL[brand]} settings updated`,
				`You have successfully updated your ${BRAND_LABEL[brand]} settings.`
			);
			await invalidateAll();
		} else {
			notifyError('Settings not updated', 'Something went wrong.');
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
		notifySuccess('Copied!', 'The prize code is in your clipboard.');
	}

	const connections = $derived(
		(['discord', 'twitch', 'patreon'] as const).map((brand) => ({
			brand,
			account: data.settings[brand]
		}))
	);
</script>

<svelte:head><title>Settings - MutaMarket</title></svelte:head>

<PageHeader title="Settings" subtitle="Notifications, linked accounts and your prizes" />

<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-2 xl:grid-cols-4">
	<!-- Notification character -->
	<div class="hud-frame relative flex flex-col p-6">
		<Mail class="absolute top-4 right-4 size-20 text-white/5" />
		<h2 class="relative flex items-center gap-2 font-medium">
			<Bell class="size-4 text-primary" />
			Notifications
		</h2>
		<p class="relative mt-1 text-sm text-muted-foreground">
			Offers and messages are sent to this character by EVE mail.
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
					<span class="text-xs text-muted-foreground">(default)</span>
				{/if}
			</div>
			<div class="mt-auto pt-4">
				<Select.Root
					type="single"
					value={String(notifyCharacter.id)}
					onValueChange={pickNotifyCharacter}
				>
					<Select.Trigger class="h-10 w-full">Change character</Select.Trigger>
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
			<p class="mt-4 text-sm text-muted-foreground">No characters on this account yet.</p>
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
					? `Your ${BRAND_LABEL[brand]} account is connected.`
					: `Connect your ${BRAND_LABEL[brand]} account to show it on your profiles.`}
			</p>
			<div class="mt-4 flex items-center gap-3">
				{#if account?.avatar}
					<img src={account.avatar} alt="" class="size-12 rounded-xl ring-2 ring-border/10" />
				{:else}
					<div
						class="grid size-12 place-items-center rounded-xl border border-dashed border-border"
					>
						<BrandIcon {brand} class="size-6 text-muted-foreground" />
					</div>
				{/if}
				<div class="flex items-center gap-2">
					<span
						class="inline-block size-2 rounded-full {account ? 'bg-green-500' : 'bg-red-500'}"
					></span>
					<span class="text-lg font-medium">{account?.name ?? 'Not connected'}</span>
				</div>
			</div>
			<div class="mt-auto flex items-center justify-between gap-3 pt-4">
				{#if account}
					<label class="flex items-center gap-2 text-sm">
						<Switch
							checked={account.is_public}
							onCheckedChange={() => toggleVisibility(brand, account)}
						/>
						Show on profiles
					</label>
					<Button variant="outline" size="sm" href="/{brand}?switch=true" rel="external">
						Switch account
					</Button>
				{:else}
					<Button href="/{brand}" rel="external">Connect</Button>
				{/if}
			</div>
		</div>
	{/each}
</div>

<!-- Raffle wins -->
<div class="hud-frame relative mt-4 mb-4 p-6">
	<Star class="absolute top-4 right-4 size-20 text-white/5" />
	<h2 class="relative flex items-center gap-2 font-medium">
		<Star class="size-4 text-primary" />
		Your Prizes
	</h2>
	<p class="relative mt-1 text-sm text-muted-foreground">
		{data.settings.raffle_wins.length > 0
			? 'Redeem your prize codes on the EVE Online code activation page.'
			: 'Prizes you win in raffles and giveaways show up here.'}
	</p>
	<a
		href={EVE_CODE_ACTIVATION_URL}
		target="_blank"
		rel="noopener noreferrer"
		class="relative mt-2 inline-flex text-sm font-medium text-primary hover:underline"
	>
		Open code activation
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
				<Button size="icon" variant="secondary" onclick={() => toggleReveal(win.id)}>
					{#if revealed.has(win.id)}
						<EyeOff class="size-4" />
					{:else}
						<Eye class="size-4" />
					{/if}
				</Button>
				<Button size="icon" variant="secondary" onclick={() => copyCode(win.code)}>
					<Copy class="size-4" />
				</Button>
			{/each}
		</div>
	{:else}
		<div class="mt-4 flex items-center gap-3 py-2 text-muted-foreground">
			<Star class="size-5" />
			<span class="text-sm">No prizes yet. Keep an eye on the raffles!</span>
		</div>
	{/if}
</div>
