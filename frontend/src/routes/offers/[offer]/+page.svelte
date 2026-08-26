<script lang="ts">
	// One offer thread, the legacy ShowOfferPage: the module card beside
	// the conversation, a reply box, and the leave action. New incoming
	// messages stream in over the MessageReceived push.
	import { invalidateAll, goto } from '$app/navigation';
	import ModuleCard from '$lib/components/module-card.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { subscribeUserEvent } from '$lib/asset-import-stream';
	import { defaultDisplaySettings } from '$lib/display';
	import { parseDbTimestamp, relativeTime } from '$lib/duration';
	import { toIskCompact } from '$lib/format-number';
	import { notifySuccess } from '$lib/toast';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const offer = $derived(data.offer);
	const other = $derived(
		offer.own_character_id === offer.sender.id ? offer.receiver : offer.sender
	);
	const otherLeft = $derived(
		offer.own_character_id === offer.sender.id ? offer.left_by_receiver : offer.left_by_sender
	);

	let reply = $state('');
	let sending = $state(false);
	let confirmingLeave = $state(false);
	let leaving = $state(false);

	$effect(() =>
		subscribeUserEvent<{ offer_id: number }>('MessageReceived', (event) => {
			if (event.offer_id === offer.id) {
				void invalidateAll();
			}
		})
	);

	async function send(event: SubmitEvent) {
		event.preventDefault();
		if (reply.trim() === '') return;
		sending = true;
		try {
			const response = await fetch('/messages', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ offer_id: offer.id, content: reply.trim() }),
				redirect: 'manual'
			});
			if (response.type === 'opaqueredirect' || response.ok) {
				reply = '';
				await invalidateAll();
			}
		} finally {
			sending = false;
		}
	}

	async function leave() {
		leaving = true;
		try {
			await fetch(`/offers/${offer.id}`, { method: 'DELETE', redirect: 'manual' });
			notifySuccess('Offer left!', 'You have left the offer.');
			await goto('/offers');
		} finally {
			leaving = false;
		}
	}

	function ago(timestamp: string): string {
		return relativeTime(parseDbTimestamp(timestamp) - Date.now() / 1000);
	}
</script>

<svelte:head><title>Offer - MutaMarket</title></svelte:head>

<PageHeader
	title={offer.module?.type.name ?? 'Offer'}
	subtitle="Offer between {offer.sender.name} and {offer.receiver.name}"
	stats={[{ label: 'Offered', value: toIskCompact(offer.price), accent: 'primary' }]}
>
	{#snippet icon()}
		<img
			alt=""
			class="size-10 rounded-lg"
			src="https://images.evetech.net/characters/{other.id}/portrait?size=64"
		/>
	{/snippet}
	{#snippet actions()}
		<Button variant="secondary" class="h-8" onclick={() => (confirmingLeave = true)}>
			Leave offer
		</Button>
	{/snippet}
</PageHeader>

<div class="grid gap-6 md:grid-cols-[300px_1fr]">
	<div>
		{#if offer.module}
			<ModuleCard module={offer.module} settings={defaultDisplaySettings()} />
		{/if}
	</div>

	<div class="flex flex-col gap-3">
		{#if otherLeft}
			<p class="rounded-lg border border-border bg-card-1 px-4 py-2 text-sm text-muted-foreground">
				{other.name} has left this offer.
			</p>
		{/if}
		<ul class="flex flex-col gap-2">
			{#each offer.messages as message (message.id)}
				<li class="flex {message.mine ? 'justify-end' : 'justify-start'}">
					<div
						class="max-w-[80%] rounded-lg border px-3 py-2 {message.mine
							? 'border-primary/40 bg-primary/10'
							: 'border-border bg-card'}"
					>
						<span class="block text-xs text-muted-foreground">
							{message.sender.name} · {ago(message.created_at)}
						</span>
						<p class="text-sm whitespace-pre-wrap">{message.content}</p>
					</div>
				</li>
			{/each}
		</ul>

		<form class="flex items-end gap-2" onsubmit={send}>
			<textarea
				bind:value={reply}
				rows="2"
				placeholder="Write a message"
				class="grow rounded-md border border-border bg-card-2 px-3 py-2 text-sm outline-none focus-visible:ring-1 focus-visible:ring-ring"
			></textarea>
			<Button type="submit" disabled={sending || reply.trim() === ''}>Send</Button>
		</form>
	</div>
</div>

<Dialog.Root bind:open={confirmingLeave}>
	<Dialog.Content class="sm:max-w-sm">
		<Dialog.Header>
			<Dialog.Title>Leave this offer?</Dialog.Title>
			<Dialog.Description>
				The thread disappears from your offers; once both sides leave it is gone for good.
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="secondary" onclick={() => (confirmingLeave = false)}>Cancel</Button>
			<Button variant="destructive" disabled={leaving} onclick={leave}>Leave offer</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
