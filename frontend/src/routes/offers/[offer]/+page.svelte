<script lang="ts">
  // One offer thread. The chat behavior is the legacy Chat.vue
  // (Discord-style message groups, Enter-to-send, leave rules); the
  // layout is a deliberate redesign of the legacy ShowOffer.vue two
  // column arrangement: one deal-room panel where the chat and the
  // tinted deal rail (module, numbers, participants, tips) share a
  // single frame instead of floating side cards.
  import { Copy, HandCoins, SendHorizontal } from '@lucide/svelte';
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
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const offer = $derived(data.offer);
  const isReceiver = $derived(offer.own_character_id === offer.receiver.id);
  const other = $derived(isReceiver ? offer.sender : offer.receiver);
  const otherLeft = $derived(isReceiver ? offer.left_by_sender : offer.left_by_receiver);
  const userLeft = $derived(isReceiver ? offer.left_by_receiver : offer.left_by_sender);
  const title = $derived(isReceiver ? `Offer from ${other.name}` : `Offer to ${other.name}`);
  const description = $derived(
    isReceiver
      ? `${other.name} wants to buy your ${offer.module?.type.name ?? 'module'}`
      : `You want to buy ${other.name}'s ${offer.module?.type.name ?? 'module'}`,
  );
  const groups = $derived(groupMessages(offer.messages));

  /** Offered price against the estimate, e.g. "43% below estimate". */
  const estimateDelta = $derived.by(() => {
    const estimate = offer.module?.estimated_value;
    if (estimate == null || estimate <= 0) return null;
    const ratio = offer.price / estimate - 1;
    const percent = Math.round(Math.abs(ratio) * 100);
    if (percent === 0) return { text: 'matches the estimate', good: true };
    return ratio > 0
      ? { text: `${percent}% above estimate`, good: true }
      : { text: `${percent}% below estimate`, good: false };
  });

  const TIPS = [
    'Be clear about your offer amount in ISK',
    'Explain your reasoning for the price',
    'Be polite and professional',
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
    }),
  );

  async function send() {
    if (reply.trim() === '' || sending) return;
    sending = true;
    try {
      const response = await fetch('/messages', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ offer_id: offer.id, content: reply.trim() }),
        redirect: 'manual',
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

<PageMeta
  title="Your offers"
  description="Manage your offers on MutaMarket."
  keywords="contracts, public, search, find"
/>

<div
  class="hud-frame grid overflow-hidden lg:h-[calc(100vh-8rem)] lg:grid-cols-[1fr_340px] xl:grid-cols-[1fr_380px]"
>
  <!-- The chat column. -->
  <div class="grid h-[70vh] min-w-0 grid-rows-[auto_1fr_auto] lg:h-auto">
    <div class="flex items-center gap-3 border-b border-border px-4 py-3 lg:px-6">
      <img
        alt={other.name}
        class="size-10 shrink-0 rounded-lg"
        src="https://images.evetech.net/characters/{other.id}/portrait?size=64"
      />
      <div class="min-w-0 grow">
        <h1 class="truncate text-base font-semibold">{title}</h1>
        <p class="truncate text-xs text-muted-foreground">{description}</p>
      </div>
      <Button
        variant="ghost"
        class="shrink-0 text-muted-foreground hover:text-foreground"
        onclick={() => (confirmingLeave = true)}
      >
        Leave offer
      </Button>
    </div>

    <div bind:this={scroller} class="flex min-h-0 flex-col overflow-y-auto px-4 py-4 lg:px-6">
      <!-- mt-auto pins a short thread to the bottom, like a messenger. -->
      <div class="mt-auto space-y-1">
        <!-- The offer itself opens the thread as an event. -->
        <div class="flex flex-col items-center gap-2 pb-6 text-center">
          <div class="grid size-10 place-items-center rounded-full bg-primary/15">
            <HandCoins class="size-5 text-primary" stroke-width={1.5} />
          </div>
          <p class="text-sm">
            <span class="font-medium">{offer.sender.name}</span>
            offered
            <span class="font-medium text-primary">{toIskCompact(offer.price)}</span>
          </p>
          <p class="-mt-1.5 text-xs text-muted-foreground">
            for the {offer.module?.type.name ?? 'module'} · this is the start of your conversation
          </p>
        </div>

        {#each groups as group (group.messages[0].id)}
          <div
            class="-mx-2 flex items-start gap-3 rounded-md px-2 py-2 transition-colors hover:bg-card-2/50"
          >
            <img
              alt={group.sender.name}
              class="mt-0.5 size-9 rounded-lg"
              src="https://images.evetech.net/characters/{group.sender.id}/portrait?size=64"
            />
            <div class="min-w-0 flex-1">
              <div class="flex items-baseline gap-2">
                <a
                  href="/characters/{moduleSlug(group.sender.name, group.sender.id)}"
                  class="truncate text-sm font-semibold hover:underline {group.mine
                    ? 'text-primary'
                    : 'text-foreground'}"
                >
                  {group.sender.name}
                </a>
                <button
                  type="button"
                  class="cursor-pointer self-center text-muted-foreground/60 hover:text-foreground"
                  aria-label="Copy name"
                  onclick={() => copyName(group.sender.name)}
                >
                  <Copy class="size-3.5" />
                </button>
                <span class="shrink-0 text-xs text-muted-foreground">{group.time}</span>
              </div>
              <div class="space-y-0.5">
                {#each group.messages as message (message.id)}
                  <p class="text-sm leading-relaxed whitespace-pre-wrap text-foreground/90">
                    {message.content}
                  </p>
                {/each}
              </div>
            </div>
          </div>
        {/each}

        {#if userLeft || otherLeft}
          <div class="flex items-center gap-3 pt-4 text-xs text-red-500">
            <hr class="flex-1 border-t border-red-500/40" />
            {userLeft ? 'You have left the offer' : 'User has left the offer'}
            <hr class="flex-1 border-t border-red-500/40" />
          </div>
        {/if}
      </div>
    </div>

    <div class="border-t border-border p-3 lg:p-4">
      <form
        class="flex items-end gap-2 rounded-xl border border-border bg-card-2 p-2 focus-within:ring-1 focus-within:ring-primary/60"
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
          class="max-h-40 grow resize-none bg-transparent px-2 py-1.5 text-sm text-foreground outline-none placeholder:text-muted-foreground disabled:opacity-50"
          onkeydown={onKeydown}></textarea>
        <Button
          type="submit"
          size="icon"
          class="size-8 shrink-0 rounded-lg"
          aria-label="Send message"
          disabled={sending || otherLeft || userLeft || reply.trim() === ''}
        >
          <SendHorizontal class="size-4" />
        </Button>
      </form>
    </div>
  </div>

  <!-- The deal rail: same frame, tinted, divided into sections. -->
  <aside
    class="flex min-h-0 flex-col overflow-y-auto border-t border-border bg-card-1 lg:border-t-0 lg:border-l"
  >
    <section class="border-b border-border p-4">
      <div class="flex items-baseline justify-between">
        <span class="hud-label">Offered</span>
        <span class="text-xl font-semibold text-primary">{toIskCompact(offer.price)}</span>
      </div>
      {#if offer.module?.estimated_value != null}
        <div class="mt-2 flex items-baseline justify-between">
          <span class="hud-label">Estimate</span>
          <span class="text-sm text-muted-foreground">
            {toIskCompact(offer.module.estimated_value)}
          </span>
        </div>
      {/if}
      {#if estimateDelta !== null}
        <p
          class="mt-2 text-right text-xs {estimateDelta.good ? 'text-green-500' : 'text-amber-500'}"
        >
          {estimateDelta.text}
        </p>
      {/if}
    </section>

    <section class="flex items-center gap-3 border-b border-border p-4">
      <img
        alt={offer.sender.name}
        class="size-9 rounded-lg"
        src="https://images.evetech.net/characters/{offer.sender.id}/portrait?size=64"
      />
      <div class="min-w-0 grow">
        <span class="block truncate text-sm">{offer.sender.name}</span>
        <span class="hud-label">Buyer</span>
      </div>
      <div class="min-w-0 text-right">
        <span class="block truncate text-sm">{offer.receiver.name}</span>
        <span class="hud-label">Seller</span>
      </div>
      <img
        alt={offer.receiver.name}
        class="size-9 rounded-lg"
        src="https://images.evetech.net/characters/{offer.receiver.id}/portrait?size=64"
      />
    </section>

    {#if offer.module}
      <section class="border-b border-border p-4">
        <ModuleCard module={offer.module} settings={defaultDisplaySettings()} />
      </section>
    {/if}

    <section class="mt-auto p-4">
      <span class="hud-label">Negotiation tips</span>
      <ul class="mt-2 space-y-1.5 text-xs text-muted-foreground">
        {#each TIPS as tip (tip)}
          <li class="flex items-start gap-2">
            <span class="mt-0.5 text-primary/70">•</span>
            {tip}
          </li>
        {/each}
      </ul>
    </section>
  </aside>
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
