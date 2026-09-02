<script lang="ts">
  // The raffle prize pool, the legacy Admin/RafflePage: a create card
  // that loads one prize per redemption code above the pool list with
  // the prize's type icon, its winner's portrait, the masked code with
  // reveal and copy, and the coloured status. Prizes are only created
  // and drawn, never edited or deleted, like the legacy page.
  import { Check, Copy, Eye, EyeOff } from '@lucide/svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import GameImage from '$lib/components/game-image.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { STATUS_CLAIMED } from '$lib/raffle-status';
  import { hasWinner, maskCode, poolCounts, statusColor, statusLabel } from '$lib/raffles';
  import { notifyError, notifySuccess } from '$lib/toast';
  import { t } from '$lib/i18n.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  let submitting = $state(false);
  // Mirrors the applied search from the URL; typing edits it until the
  // next navigation re-seeds it.
  let typeSearch = $state('');
  $effect(() => {
    typeSearch = data.raffles.type_search;
  });

  let form = $state({ name: '', description: '', type_id: null as number | null, codes: '' });

  const counts = $derived(poolCounts(data.raffles.raffle_items));

  // The legacy revealedIds set: codes stay masked until toggled.
  let revealed = $state(new Set<number>());

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
    notifySuccess(t('admin.raffles.copiedTitle'), t('admin.raffles.copiedBody'));
  }

  // A plain navigation, not a nested form: the type search shares the
  // create card with the create form.
  async function searchTypes() {
    const query = typeSearch ? `?type_search=${encodeURIComponent(typeSearch)}` : '';
    await goto(`/admin/raffles${query}`, { keepFocus: true, noScroll: true });
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    submitting = true;
    try {
      const response = await fetch('/raffles', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(form),
      });
      if (response.ok || response.redirected) {
        notifySuccess(t('admin.raffles.createdTitle'), t('admin.raffles.createdBody'));
        form = { name: '', description: '', type_id: null, codes: '' };
        await invalidateAll();
        return;
      }
      const body = await response.json().catch(() => null);
      notifyError(
        t('admin.raffles.createFailedTitle'),
        body?.message ?? t('admin.raffles.createFailedBody'),
      );
    } finally {
      submitting = false;
    }
  }
</script>

<PageMeta title={t('meta.adminRaffle.title')} description={t('meta.adminRaffle.description')} />

<div class="space-y-6">
  <section class="hud-frame p-4">
    <h2 class="text-lg font-medium">{t('admin.raffles.createTitle')}</h2>
    <p class="text-muted-foreground mb-4 text-sm">{t('admin.raffles.createDescription')}</p>

    <form class="grid gap-4 sm:grid-cols-2" onsubmit={create}>
      <div class="space-y-1">
        <Label for="raffle-name">{t('admin.raffles.itemName')}</Label>
        <Input
          id="raffle-name"
          bind:value={form.name}
          placeholder={t('admin.raffles.itemNamePlaceholder')}
          required
        />
      </div>
      <div class="space-y-1">
        <Label for="raffle-description">{t('admin.raffles.descriptionOptional')}</Label>
        <Input
          id="raffle-description"
          bind:value={form.description}
          placeholder={t('admin.raffles.descriptionPlaceholder')}
        />
      </div>

      <div class="space-y-1 sm:col-span-2">
        <Label for="raffle-codes">{t('admin.raffles.redemptionCodes')}</Label>
        <textarea
          id="raffle-codes"
          bind:value={form.codes}
          class="border-input bg-background min-h-24 w-full rounded-md border px-3 py-2 font-mono text-sm"
          placeholder={t('admin.raffles.codesPlaceholder')}
          required></textarea>
        <p class="text-muted-foreground text-xs">{t('admin.raffles.codesHint')}</p>
      </div>

      <div class="space-y-1 sm:col-span-2">
        <Label>{t('admin.raffles.typeOptional')}</Label>
        <div class="flex gap-2">
          <Input bind:value={typeSearch} placeholder={t('admin.raffles.searchTypesPlaceholder')} />
          <Button type="button" variant="secondary" onclick={searchTypes}>
            {t('common.actions.search')}
          </Button>
        </div>
        {#if data.raffles.types.length > 0}
          <div class="mt-2 flex flex-wrap gap-2">
            {#each data.raffles.types as type (type.id)}
              <Button
                size="sm"
                type="button"
                variant={form.type_id === type.id ? 'default' : 'secondary'}
                onclick={() => (form.type_id = form.type_id === type.id ? null : type.id)}
              >
                {type.name}
              </Button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="sm:col-span-2">
        <Button disabled={submitting} type="submit">{t('admin.raffles.createItems')}</Button>
      </div>
    </form>
  </section>

  <section class="hud-frame p-4">
    <div class="mb-4 flex flex-wrap items-baseline gap-x-4 gap-y-1">
      <h2 class="text-lg font-medium">{t('admin.raffles.listTitle')}</h2>
      <span class="text-muted-foreground text-sm">
        {t('admin.raffles.poolCounts', {
          pending: counts.pending,
          active: counts.active,
          claimed: counts.claimed,
        })}
      </span>
    </div>

    {#if data.raffles.raffle_items.length > 0}
      <div class="grid grid-cols-[auto_1fr_1fr_auto_auto] gap-3">
        <div
          class="text-muted-foreground col-span-full grid grid-cols-subgrid border-b pb-2 text-xs font-medium"
        >
          <div>{t('common.labels.type')}</div>
          <div>{t('common.labels.name')}</div>
          <div>{t('admin.raffles.winner')}</div>
          <div>{t('admin.raffles.code')}</div>
          <div>{t('common.labels.status')}</div>
        </div>
        {#each data.raffles.raffle_items as item (item.id)}
          <div class="col-span-full grid grid-cols-subgrid items-center gap-3 py-2">
            {#if item.type}
              <GameImage
                alt={item.type.name ?? ''}
                class="size-8 rounded"
                src="https://images.evetech.net/types/{item.type.id}/icon?size=64"
              />
            {:else}
              <div class="border-border size-8 rounded border"></div>
            {/if}
            <div class="min-w-0">
              <p class="truncate text-sm font-medium">{item.name}</p>
              {#if item.description}
                <p class="text-muted-foreground truncate text-xs">{item.description}</p>
              {/if}
            </div>
            <div class="flex min-w-0 items-center gap-2">
              {#if hasWinner(item) && item.winner}
                {#if item.winner.character_id}
                  <GameImage
                    alt={item.winner.name ?? ''}
                    class="size-6 rounded"
                    src="https://images.evetech.net/characters/{item.winner
                      .character_id}/portrait?size=64"
                  />
                {/if}
                <span class="truncate text-sm">{item.winner.name ?? '-'}</span>
              {:else}
                <span class="text-muted-foreground text-sm">-</span>
              {/if}
            </div>
            <div class="flex items-center gap-1">
              <Input
                value={revealed.has(item.id) ? item.code : maskCode(item.code)}
                class="h-7 w-40 font-mono text-xs"
                readonly
              />
              <Button
                size="icon"
                variant="ghost"
                class="size-7"
                aria-label={revealed.has(item.id)
                  ? t('admin.raffles.hideCode')
                  : t('admin.raffles.showCode')}
                onclick={() => toggleReveal(item.id)}
              >
                {#if revealed.has(item.id)}
                  <EyeOff class="size-3.5" />
                {:else}
                  <Eye class="size-3.5" />
                {/if}
              </Button>
              <Button
                size="icon"
                variant="ghost"
                class="size-7"
                aria-label={t('admin.raffles.copyCode')}
                onclick={() => copyCode(item.code)}
              >
                <Copy class="size-3.5" />
              </Button>
            </div>
            <div class="flex items-center gap-2">
              {#if item.status === STATUS_CLAIMED}
                <Check class="size-4 text-green-500" />
              {/if}
              <span class="text-sm {statusColor(item.status)}">{statusLabel(item.status)}</span>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-muted-foreground text-sm">{t('admin.raffles.empty')}</p>
    {/if}
  </section>
</div>
