<script lang="ts">
  // The make-offer dialog, the legacy CreateOfferDialog.vue reduced to
  // its essentials — with our divergence: the price is a real field
  // (legacy asked buyers to type the amount into the message) and the
  // message is optional on top of it. Mounted once in the layout,
  // opened from any card's public-asset row via the make-offer store.
  import { goto } from '$app/navigation';
  import GameImage from './game-image.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { toIskCompact } from '$lib/format-number';
  import { t } from '$lib/i18n.svelte';
  import {
    closeMakeOffer,
    defaultOfferMessage,
    offerModule,
    refreshSentOffers,
  } from '$lib/make-offer';
  import { notifyError, notifySuccess } from '$lib/toast';

  const module = $derived($offerModule);

  let price = $state('');
  let message = $state('');
  let submitting = $state(false);

  const parsedPrice = $derived.by(() => {
    const value = Number(price.replaceAll(',', '').trim());
    return Number.isFinite(value) && value > 0 ? value : null;
  });

  $effect(() => {
    // A fresh module resets the form.
    if (module !== null) {
      price = '';
      message = '';
    }
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (module === null || module.public_asset === null || parsedPrice === null) return;
    submitting = true;
    try {
      const response = await fetch('/offers', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          receiver_id: module.public_asset.owner.id,
          module_id: module.id,
          price: parsedPrice,
          message: message.trim() === '' ? null : message.trim(),
        }),
      });
      if (response.ok) {
        notifySuccess(t('offers.create.sentTitle'), t('offers.create.sentBody'));
        void refreshSentOffers();
        closeMakeOffer();
        await goto(new URL(response.url).pathname);
        return;
      }
      const body = (await response.json().catch(() => null)) as { message?: string } | null;
      notifyError(
        t('offers.create.notSentTitle'),
        body?.message ?? t('errors.internalServerError.name'),
      );
    } finally {
      submitting = false;
    }
  }
</script>

<Dialog.Root open={module !== null} onOpenChange={(open) => !open && closeMakeOffer()}>
  <Dialog.Content class="sm:max-w-md">
    {#if module}
      <Dialog.Header>
        <Dialog.Title>{t('offers.create.title')}</Dialog.Title>
        <Dialog.Description>{t('offers.create.body')}</Dialog.Description>
      </Dialog.Header>

      <div class="flex items-center gap-3 rounded-lg border border-border bg-card-1 p-3">
        <GameImage
          src="https://images.evetech.net/types/{module.type.id}/icon?size=64"
          alt=""
          class="size-10 rounded"
        />
        <div class="min-w-0">
          <span class="block truncate text-sm font-medium">{module.type.name}</span>
          <span class="text-xs text-muted-foreground">
            {module.estimated_value !== null
              ? t('modules.card.estimatedShortCap', { value: toIskCompact(module.estimated_value) })
              : t('modules.card.noEstimate')}
          </span>
        </div>
      </div>

      <form class="flex flex-col gap-4" onsubmit={submit}>
        <div class="flex flex-col gap-1.5">
          <Label for="offer-price">{t('offers.create.priceLabel')}</Label>
          <Input
            id="offer-price"
            bind:value={price}
            inputmode="numeric"
            placeholder="1,500,000,000"
            autocomplete="off"
          />
          {#if parsedPrice !== null}
            <span class="text-xs text-muted-foreground">= {toIskCompact(parsedPrice)}</span>
          {/if}
        </div>
        <div class="flex flex-col gap-1.5">
          <Label for="offer-message">{t('offers.create.messageOptional')}</Label>
          <textarea
            id="offer-message"
            bind:value={message}
            rows="3"
            placeholder={defaultOfferMessage(parsedPrice)}
            class="rounded-md border border-border bg-card-2 px-3 py-2 text-sm outline-none focus-visible:ring-1 focus-visible:ring-ring"
          ></textarea>
        </div>
        <Dialog.Footer>
          <Button type="button" variant="secondary" onclick={closeMakeOffer}>
            {t('common.actions.cancel')}
          </Button>
          <Button type="submit" disabled={parsedPrice === null || submitting}>
            {submitting ? t('offers.create.sending') : t('offers.create.sendOffer')}
          </Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
