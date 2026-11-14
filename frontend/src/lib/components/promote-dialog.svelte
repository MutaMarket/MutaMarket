<script lang="ts">
  // The promote dialog (no legacy counterpart): what to send in game to
  // put a module at the top of the browser. Nothing is submitted here,
  // the donation itself is the order, so the dialog only works out the
  // amount and hands over the three things to paste. Mounted once in the
  // layout, opened from any module menu via the promotion store.
  import { Copy } from '@lucide/svelte';
  import { page } from '$app/state';
  import GameImage from './game-image.svelte';
  import { Button } from '#lib/components/ui/button/index.ts';
  import * as Dialog from '#lib/components/ui/dialog/index.ts';
  import { Input } from '#lib/components/ui/input/index.ts';
  import { Label } from '#lib/components/ui/label/index.ts';
  import { copyWithToasts } from '#lib/export.ts';
  import { toIsk } from '#lib/format-number.ts';
  import { t } from '#lib/i18n.svelte.ts';
  import { premiumFromSidebar } from '#lib/premium.ts';
  import {
    MAX_DIALOG_DAYS,
    clampDays,
    closePromote,
    dailyCostFromSidebar,
    promotedUntil,
    promotionModule,
    promotionReason,
  } from '#lib/promotion.ts';
  import { sidebarData } from '#lib/sidebar.ts';

  const module = $derived($promotionModule);
  const sidebar = $derived($sidebarData ?? page.data.sidebar ?? null);
  const character = $derived(premiumFromSidebar(sidebar).premium_character);
  const dailyCost = $derived(dailyCostFromSidebar(sidebar));

  /** The day field's starting value: long enough to be seen, short
   * enough to try out. */
  const DEFAULT_DAYS = '3';
  let days = $state(DEFAULT_DAYS);
  $effect(() => {
    // A fresh module resets the field.
    if (module !== null) {
      days = DEFAULT_DAYS;
    }
  });

  const amount = $derived(clampDays(days) * dailyCost);
  const until = $derived(module ? promotedUntil(module) : null);
  const rows = $derived(
    module === null
      ? []
      : [
          {
            label: t('modules.promote.amountLabel'),
            shown: toIsk(amount),
            copied: String(amount),
            subject: t('modules.export.subjects.amount'),
          },
          {
            label: t('modules.promote.recipientLabel'),
            shown: character,
            copied: character,
            subject: t('modules.export.subjects.characterName'),
          },
          {
            label: t('modules.promote.reasonLabel'),
            shown: promotionReason(module),
            copied: promotionReason(module),
            subject: t('modules.export.subjects.reason'),
          },
        ],
  );
</script>

<Dialog.Root open={module !== null} onOpenChange={(open) => !open && closePromote()}>
  <Dialog.Content class="sm:max-w-md">
    {#if module}
      <Dialog.Header>
        <Dialog.Title>{t('modules.promote.title')}</Dialog.Title>
        <Dialog.Description>{t('modules.promote.body')}</Dialog.Description>
      </Dialog.Header>

      <div class="flex items-center gap-3 rounded-lg border border-border bg-card-1 p-3">
        <GameImage src="/img/icons/{module.type.id}.png" alt="" class="size-10 rounded" />
        <div class="min-w-0">
          <span class="block truncate text-sm font-medium">
            {module.source_type?.name ?? module.type.name}
          </span>
          <span class="text-xs text-muted-foreground">
            {until
              ? t('modules.promote.runningUntil', { date: until.toISOString().slice(0, 10) })
              : t('modules.promote.notRunning')}
          </span>
        </div>
      </div>

      <div class="flex flex-col gap-1.5">
        <Label for="promote-days">{t('modules.promote.daysLabel')}</Label>
        <Input
          id="promote-days"
          type="number"
          min="1"
          max={MAX_DIALOG_DAYS}
          step="1"
          bind:value={days}
          autocomplete="off"
        />
        <span class="text-xs text-muted-foreground">
          {t('modules.promote.rate', { cost: toIsk(dailyCost) })}
        </span>
      </div>

      <dl class="grid grid-cols-[auto_1fr_auto] items-center gap-x-3 gap-y-2 text-sm">
        {#each rows as row (row.label)}
          <dt class="text-muted-foreground">{row.label}</dt>
          <dd class="min-w-0 truncate font-mono">{row.shown}</dd>
          <Button
            variant="ghost"
            size="icon"
            aria-label={t('modules.promote.copy', { what: row.label })}
            onclick={() => copyWithToasts(row.copied, row.subject)}
          >
            <Copy class="size-4" />
          </Button>
        {/each}
      </dl>

      <p class="text-xs text-muted-foreground">{t('modules.promote.finePrint')}</p>

      <Dialog.Footer>
        <Button type="button" variant="secondary" onclick={closePromote}>
          {t('common.actions.close')}
        </Button>
      </Dialog.Footer>
    {/if}
  </Dialog.Content>
</Dialog.Root>
