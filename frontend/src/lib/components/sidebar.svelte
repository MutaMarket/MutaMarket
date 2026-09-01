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
		Coffee,
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
	import { routeIcon, sortBookmarks } from '$lib/bookmark-routes';
	import { toCompact, toCompactShort } from '$lib/format-number';
	import { premiumFromSidebar } from '$lib/premium';
	import {
		KOFI_LINK,
		PATREON_LINK,
		createBookmark,
		deleteBookmark,
		refreshSidebar,
		renameBookmark,
		sidebarData,
		visibleDiscordInvites,
	} from '$lib/sidebar';
	import { notifySuccess } from '$lib/toast';

	const data = $derived($sidebarData);
	const premium = $derived(premiumFromSidebar(data));
	const bookmarks = $derived(data?.bookmarks == null ? null : sortBookmarks(data.bookmarks));
	const discordInvites = $derived(visibleDiscordInvites(data?.discord_invites ?? []));

	$effect(() => {
		void refreshSidebar();
	});

	// The legacy add-current-page defaults: the panel type names the
	// bookmark when the page carries one.
	function addCurrentPage() {
		const panel = page.data.panel as { type_id: number; type_name: string } | undefined | null;
		void createBookmark(location.pathname, panel?.type_name ?? 'Bookmark', panel?.type_id ?? null);
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

	// Ten-second ad rotation, the legacy embla autoplay.
	const AD_ROTATE_MS = 10_000;
	const GEAR_ROTATE_MS = 12_000;
	let adIndex = $state(0);
	let gearIndex = $state(0);
	$effect(() => {
		const ads = setInterval(() => {
			adIndex += 1;
		}, AD_ROTATE_MS);
		const gear = setInterval(() => {
			gearIndex += 1;
		}, GEAR_ROTATE_MS);
		return () => {
			clearInterval(ads);
			clearInterval(gear);
		};
	});
	const currentAd = $derived(
		data !== null && data.advertisements.length > 0
			? data.advertisements[adIndex % data.advertisements.length]
			: null,
	);
	const currentGear = $derived(
		data !== null && data.gear_items.length > 0
			? data.gear_items[gearIndex % data.gear_items.length]
			: null,
	);

	function copyMutaMate() {
		void navigator.clipboard.writeText(premium.premium_character);
		notifySuccess(
			'Name copied!',
			`${premium.premium_character} has been copied to your clipboard!`,
		);
	}

	// The legacy Sidebar Donations.vue copy handler and its
	// premium.copyNotification strings.
	function copyDonationCharacter() {
		void navigator.clipboard.writeText(premium.premium_character);
		notifySuccess('Copied to clipboard', `Send ISK to "${premium.premium_character}"`);
	}

	/** The legacy MarkeeDragon coupon (+3% off, calculator locale). */
	const MARKEE_CODE = 'mutamarket';
</script>

<div class="hidden w-[250px] shrink-0 flex-col gap-4 self-start xl:flex">
	{#if bookmarks !== null}
		<div class="hud-frame overflow-hidden">
			<div class="flex items-center justify-between px-3 py-2">
				<div class="flex items-center gap-1.5">
					<BookmarkIcon class="size-3.5 text-primary" />
					<span class="text-sm font-medium">Bookmarks</span>
					{#if bookmarks.length > 0}
						<span class="rounded-full bg-card-2 px-1.5 text-xs text-muted-foreground tabular-nums">
							{bookmarks.length}
						</span>
					{/if}
				</div>
				<button
					type="button"
					class="flex cursor-pointer items-center justify-center rounded-full bg-primary/10 p-1 text-primary transition-colors hover:bg-primary/20"
					title="Add current page"
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
							<button type="submit" class="cursor-pointer text-primary" aria-label="Save name">
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
								aria-label="Rename bookmark"
								onclick={() => startRename(bookmark.id, bookmark.name)}
							>
								<Pencil class="size-3" />
							</button>
							<button
								type="button"
								class="relative z-10 cursor-pointer text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-red-500"
								aria-label="Delete bookmark"
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
							<p class="text-xs text-muted-foreground">No bookmarks yet</p>
							<button
								type="button"
								class="cursor-pointer text-xs text-primary hover:underline"
								onclick={addCurrentPage}
							>
								Add this page
							</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	{#if currentAd?.image_url}
		<div
			class="relative w-full overflow-hidden {currentAd.link?.includes('markeedragon')
				? 'rounded-t-lg'
				: 'rounded-lg'}"
		>
			<a href={currentAd.link ?? '#'} rel="noopener noreferrer" target="_blank">
				<img
					alt={currentAd.name}
					src={currentAd.image_url}
					class="aspect-[250/300] w-full object-cover"
				/>
			</a>
			<div
				class="pointer-events-none absolute top-0 left-0 rounded-br-lg bg-black/30 p-1 py-0.5 text-xs"
			>
				Advertisement
			</div>
		</div>
		{#if currentAd.link?.includes('markeedragon')}
			<!-- The affiliate coupon as the creative's own bottom section:
			     the same link as the artwork, no hover chrome. -->
			<a
				href={currentAd.link}
				rel="noopener noreferrer"
				target="_blank"
				class="-mt-4 flex w-full items-center justify-center gap-2 rounded-b-lg border-t border-white/15 bg-black px-3 py-3 text-sm font-bold text-white select-none"
			>
				Use code
				<code class="font-mono font-bold text-primary">{MARKEE_CODE}</code>
				for 3% off
			</a>
		{/if}
	{/if}

	{#if currentGear?.image_url}
		<a
			href={currentGear.link}
			rel="sponsored nofollow noopener"
			target="_blank"
			class="w-full overflow-hidden rounded-lg"
		>
			<img
				alt={currentGear.name}
				title={currentGear.description ?? currentGear.name}
				src={currentGear.image_url}
				class="aspect-square w-full rounded-lg object-cover"
			/>
		</a>
	{/if}

	<div class="hud-frame">
		<div class="flex items-center justify-between border-b border-border px-3 py-2">
			<div class="flex items-center gap-1.5">
				<Crown class="size-3.5 text-primary" />
				<span class="text-sm font-medium">Premium</span>
			</div>
			<span class="flex items-center gap-0.5 text-xs text-muted-foreground">
				<ChevronRight class="size-3" />
			</span>
		</div>
		<div class="space-y-2 p-3">
			<p class="text-xs text-muted-foreground">
				Unlock historic sales, similar modules, priority ordering, and more.
			</p>
			<div class="space-y-1 text-xs">
				<div class="flex items-baseline justify-between">
					<span class="text-muted-foreground">Monthly</span>
					<span class="font-medium">{toCompact(premium.premium_cost)} ISK</span>
				</div>
				<div class="flex items-baseline justify-between">
					<span class="text-muted-foreground">Yearly</span>
					<span class="font-medium">{toCompact(premium.premium_yearly_cost)} ISK</span>
				</div>
				<p class="text-[10px] text-primary">Save 2 months with yearly</p>
			</div>
		</div>
		<div class="border-t border-border px-3 py-2">
			<button
				type="button"
				class="flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground"
				onclick={copyMutaMate}
			>
				<Send class="size-3" />
				<span>Send ISK to</span>
				<code class="rounded bg-card-2 px-1 py-0.5 font-mono">MutaMate</code>
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
			<div class="grid size-10 place-items-center rounded-md bg-card p-2">
				<Star class="size-5 text-primary" />
			</div>
			<div>
				<span class="mb-1 block leading-none">Block Ads with Patreon!</span>
				<span class="block text-xs leading-none text-muted-foreground">
					Help me feed the server hamsters
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
				<span class="text-sm font-medium">Top Donors</span>
				<span class="text-xs text-muted-foreground">14d</span>
			</div>
			<a
				href="/donations"
				class="flex items-center gap-0.5 text-xs text-muted-foreground transition-colors hover:text-foreground"
			>
				<span>All</span>
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
				onclick={copyDonationCharacter}
			>
				<Heart class="size-3" />
				<span>Donate to</span>
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
				<Coffee class="size-5 text-primary" />
			</div>
			<div>
				<span class="mb-1 block leading-none">Buy me some Quafe</span>
				<span class="block text-xs leading-none text-muted-foreground">
					Help me stay awake and code more
				</span>
			</div>
		</a>
	{/if}

	{#if discordInvites.length > 0}
		<div class="hud-frame">
			<div class="border-b border-border px-3 py-2">
				<span class="text-sm font-medium">Partner Discords</span>
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
				<span class="text-sm font-medium">Partner</span>
			</div>
		</div>
		<a
			href="https://wormhole.systems"
			target="_blank"
			rel="noopener noreferrer"
			class="group flex items-center gap-3 p-3 transition-colors hover:bg-card-2"
		>
			<div class="flex size-10 shrink-0 items-center justify-center rounded bg-black">
				<Star class="size-6 text-white" />
			</div>
			<div class="min-w-0 flex-1">
				<div class="flex items-center gap-1.5">
					<span class="truncate text-sm font-medium">WormholeSystems</span>
					<ExternalLink
						class="size-3 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
					/>
				</div>
				<span class="block text-xs text-muted-foreground">Wormhole mapping & intel</span>
			</div>
		</a>
	</div>
</div>
