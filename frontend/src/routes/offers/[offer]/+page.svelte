<script lang="ts">
	// One offer thread, the legacy Offers/ShowOffer.vue + Chat.vue: the
	// full-height chat panel (Discord-style message groups under one
	// avatar/name/time header, the conversation-start pill, red rules
	// when a side leaves, Enter-to-send) beside the sidebar with the
	// module card, price information and negotiation tips. Unported
	// legacy extras: the block-user dialog, typing whispers and the
	// Discord-integration card. Price information shows our offered
	// price (the legacy card needed the unported asking price).
	import { Copy, Mail } from '@lucide/svelte';
	import { invalidateAll, goto } from '$app/navigation';
	import ModuleCard from '$lib/components/module-card.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { subscribeUserEvent } from '$lib/asset-import-stream';
	import { groupMessages } from '$lib/chat-groups';
	import { defaultDisplaySettings } from '$lib/display';
	import { toIskCompact } from '$lib/format-number';
	import { moduleSlug } from '$lib/query';
	import { notifySuccess } from '$lib/toast';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const offer = $derived(data.offer);
	const isReceiver = $derived(offer.own_character_id === offer.receiver.id);
	const other = $derived(isReceiver ? offer.sender : offer.receiver);
	const otherLeft = $derived(isReceiver ? offer.left_by_sender : offer.left_by_receiver);
	const userLeft = $derived(isReceiver ? offer.left_by_receiver : offer.left_by_sender);
	const title = $derived(isReceiver ? `Offer from ${other.name}` : `Offer to ${other.name}`);
	const description = $derived(
		isReceiver
			? `View offer from ${other.name} for your ${offer.module?.type.name ?? 'module'}`
			: `View offer to ${other.name} for their ${offer.module?.type.name ?? 'module'}`
	);
	const groups = $derived(groupMessages(offer.messages));

	const TIPS = [
		'Be clear about your offer amount in ISK',
		'Explain your reasoning for the price',
		'Be polite and professional'
	];

	let reply = $state('');
	let sending = $state(false);
	let confirmingLeave = $state(false);
	let leaving = $state(false);
	let scroller = $state<HTMLDivElement | null>(null);
	let textarea = $state<HTMLTextAreaElement | null>(null);

	// The legacy scrollToBottom: pinned to the newest message.
	$effect(() => {
		void offer.messages.length;
		if (scroller !== null) {
			scroller.scrollTop = scroller.scrollHeight;
		}
	});

	$effect(() =>
		subscribeUserEvent<{ offer_id: number }>('MessageReceived', (event) => {
			if (event.offer_id === offer.id) {
				void invalidateAll();
			}
		})
	);

	async function send() {
		if (reply.trim() === '' || sending) return;
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
				textarea?.focus();
			}
		} finally {
			sending = false;
		}
	}

	// The legacy onKeydown: Enter sends, Shift+Enter breaks the line.
	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			void send();
		}
	}

	function copyName(name: string) {
		void navigator.clipboard.writeText(name);
		notifySuccess('Name copied!', 'The name has been copied to your clipboard!');
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
</script>

<svelte:head><title>{title} - MutaMarket</title></svelte:head>

<div class="grid gap-4 lg:grid-cols-[1fr_350px] lg:gap-8 xl:grid-cols-[1fr_400px]">
	<!-- Main content: the chat panel. -->
	<div
		class="grid h-[calc(100vh-12rem)] grid-rows-[auto_1fr_auto] rounded-lg border border-border bg-card lg:h-[calc(100vh-8rem)]"
	>
		<div
			class="flex h-auto flex-col items-start justify-between gap-3 border-b border-border px-4 py-3 sm:h-20 sm:flex-row sm:items-center sm:gap-0 lg:h-24 lg:px-6 lg:py-4"
		>
			<div class="flex w-full items-center gap-3 sm:w-auto lg:gap-4">
				<img
					alt={other.name}
					class="size-10 shrink-0 rounded-lg lg:size-12"
					src="https://images.evetech.net/characters/{other.id}/portrait?size=64"
				/>
				<div class="min-w-0">
					<h1 class="truncate text-base font-semibold lg:text-lg">{title}</h1>
					<p class="truncate text-xs text-muted-foreground lg:text-sm">{description}</p>
				</div>
			</div>
			<div class="flex w-full items-center justify-end gap-2 sm:w-auto lg:gap-4">
				<Button variant="secondary" onclick={() => (confirmingLeave = true)}>Leave offer</Button>
			</div>
		</div>

		<div bind:this={scroller} class="h-full self-start overflow-y-auto px-6 py-4">
			<div class="mb-8 text-center">
				<div
					class="inline-flex items-center gap-2 rounded-full bg-card-1 px-4 py-2 text-sm text-muted-foreground"
				>
					<span>This is the start of your conversation</span>
				</div>
			</div>

			<div class="space-y-8">
				{#each groups as group (group.messages[0].id)}
					<div class="flex items-start gap-4">
						<img
							alt={group.sender.name}
							class="size-10 rounded-lg"
							src="https://images.evetech.net/characters/{group.sender.id}/portrait?size=64"
						/>
						<div class="flex-1">
							<div class="mb-2 flex items-center gap-2">
								<a
									href="/characters/{moduleSlug(group.sender.name, group.sender.id)}"
									class="font-medium text-foreground hover:underline"
								>
									{group.sender.name}
								</a>
								<button
									type="button"
									class="cursor-pointer text-muted-foreground hover:text-foreground"
									aria-label="Copy name"
									onclick={() => copyName(group.sender.name)}
								>
									<Copy class="size-4" />
								</button>
								<span class="text-xs text-muted-foreground">{group.time}</span>
							</div>
							<div class="space-y-1">
								{#each group.messages as message (message.id)}
									<p class="text-sm whitespace-pre-wrap text-foreground">{message.content}</p>
								{/each}
							</div>
						</div>
					</div>
				{/each}
			</div>

			<div class="mt-8 space-y-4">
				{#if userLeft}
					<div class="flex items-center gap-2 text-sm text-red-500">
						<hr class="flex-1 border-t border-red-500" />
						You have left the offer
						<hr class="flex-1 border-t border-red-500" />
					</div>
				{:else if otherLeft}
					<div class="flex items-center gap-2 text-sm text-red-500">
						<hr class="flex-1 border-t border-red-500" />
						User has left the offer
						<hr class="flex-1 border-t border-red-500" />
					</div>
				{/if}
			</div>
		</div>

		<div class="mt-auto border-t border-border bg-card p-4">
			<form
				class="relative"
				onsubmit={(event) => {
					event.preventDefault();
					void send();
				}}
			>
				<textarea
					bind:this={textarea}
					bind:value={reply}
					disabled={sending || otherLeft || userLeft}
					placeholder="Send a message to {other.name}"
					rows="1"
					class="w-full resize-none rounded-lg border border-border bg-transparent px-4 py-3 pr-12 text-sm text-foreground placeholder:text-muted-foreground focus:ring-2 focus:ring-primary focus:outline-none disabled:opacity-50"
					onkeydown={onKeydown}
				></textarea>
				<div class="absolute top-1/2 right-4 -translate-y-1/2">
					<Mail class="size-5 text-muted-foreground" />
				</div>
			</form>
		</div>
	</div>

	<!-- Sidebar: module details and info. -->
	<div class="space-y-4 lg:space-y-6">
		{#if offer.module}
			<div class="rounded-lg border border-border bg-card p-4 lg:p-6">
				<h2 class="mb-3 text-base font-medium lg:mb-4 lg:text-lg">Module Details</h2>
				<ModuleCard module={offer.module} settings={defaultDisplaySettings()} />
			</div>
		{/if}

		<div class="rounded-lg border border-blue-500/20 bg-blue-500/5 p-4 lg:p-6">
			<h2 class="mb-3 text-base font-medium lg:mb-4 lg:text-lg">Price Information</h2>
			<div class="space-y-3 lg:space-y-4">
				<div class="flex items-center justify-between">
					<span class="text-xs text-muted-foreground lg:text-sm">Offered price</span>
					<span class="text-lg font-semibold lg:text-xl">{toIskCompact(offer.price)}</span>
				</div>
				{#if offer.module?.estimated_value != null}
					<div class="flex items-center justify-between">
						<span class="text-xs text-muted-foreground lg:text-sm">Estimated value</span>
						<span class="text-base lg:text-lg">{toIskCompact(offer.module.estimated_value)}</span>
					</div>
				{/if}
			</div>
		</div>

		<div class="rounded-lg border border-yellow-500/20 bg-yellow-500/5 p-4 lg:p-6">
			<h2 class="mb-3 text-base font-medium lg:mb-4 lg:text-lg">Negotiation Tips</h2>
			<ul class="space-y-2 text-xs text-muted-foreground lg:space-y-3 lg:text-sm">
				{#each TIPS as tip (tip)}
					<li class="flex items-start gap-2">
						<span class="mt-1 text-yellow-500">•</span>
						{tip}
					</li>
				{/each}
			</ul>
		</div>
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
