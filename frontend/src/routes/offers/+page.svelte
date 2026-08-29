<script lang="ts">
	// The offers index, the legacy ShowOffersPage: every thread involving
	// the user's characters, newest conversation first, unread threads
	// marked. Live: a MessageReceived push refreshes the list.
	import { HandCoins } from '@lucide/svelte';
	import { invalidateAll } from '$app/navigation';
	import GameImage from '$lib/components/game-image.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { subscribeUserEvent } from '$lib/asset-import-stream';
	import { parseDbTimestamp, relativeTime } from '$lib/duration';
	import { toIskCompact } from '$lib/format-number';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const unreadCount = $derived(data.offers.filter((offer) => !offer.is_read).length);

	$effect(() =>
		subscribeUserEvent('MessageReceived', () => {
			void invalidateAll();
		})
	);

	function ago(timestamp: string): string {
		return relativeTime(parseDbTimestamp(timestamp) - Date.now() / 1000);
	}
</script>

<svelte:head><title>Offers - MutaMarket</title></svelte:head>

<PageHeader
	title="Offers"
	subtitle="Price talks about modules you buy and sell"
	stats={[
		{ label: 'Threads', value: data.offers.length.toLocaleString('en-US'), accent: 'primary' },
		{ label: 'Unread', value: unreadCount.toLocaleString('en-US') }
	]}
>
	{#snippet icon()}
		<div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
			<HandCoins class="size-5 text-primary" stroke-width={1.5} />
		</div>
	{/snippet}
</PageHeader>

{#if data.offers.length === 0}
	<div class="hud-frame p-6">
		<span class="block text-lg font-medium">No offers yet</span>
		<p class="text-muted-foreground">
			Find a module for sale and make its seller an offer — the conversation lands here.
		</p>
	</div>
{:else}
	<ul class="flex flex-col gap-2">
		{#each data.offers as offer (offer.id)}
			<li>
				<a
					href="/offers/{offer.id}"
					class="flex items-center gap-4 rounded-lg border border-border bg-card p-3 transition-colors hover:bg-card-2"
				>
					<GameImage
						src="https://images.evetech.net/types/{offer.module.type_id}/icon?size=64"
						alt=""
						class="size-10 rounded"
					/>
					<div class="min-w-0 grow">
						<span class="flex items-center gap-2 text-sm font-medium">
							<span class="truncate">{offer.module.type_name}</span>
							{#if !offer.is_read}
								<span class="size-2 shrink-0 rounded-full bg-primary" title="Unread"></span>
							{/if}
						</span>
						<span class="block truncate text-xs text-muted-foreground">
							{offer.latest_message.content}
						</span>
					</div>
					<div class="shrink-0 text-right">
						<span class="block text-sm">{toIskCompact(offer.price)}</span>
						<span class="text-xs text-muted-foreground">
							{offer.sender.name} → {offer.receiver.name} · {ago(
								offer.latest_message.created_at
							)}
						</span>
					</div>
				</a>
			</li>
		{/each}
	</ul>
{/if}
