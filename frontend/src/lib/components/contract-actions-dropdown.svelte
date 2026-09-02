<script lang="ts">
  // The legacy Tables/ContractHistory/ContractActionsDropdown.vue: copy
  // and in-game actions for every viewer, training-data moderation for
  // admins.
  import { Copy, EllipsisVertical, ExternalLink, FilePenLine, Sparkles } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { Button } from '$lib/components/ui/button';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { t } from '$lib/i18n.svelte';
  import { openContractInGame } from '$lib/open-contract';
  import { notifySuccess, notifyError } from '$lib/toast';
  import type { HistoricContract } from '$lib/types';

  let { contract }: { contract: HistoricContract } = $props();

  // The same in-game link system as the module toolbar (Jita).
  const CONTRACT_LINK_SYSTEM = 30000142;

  const isAdmin = $derived(Boolean(page.data.nav?.user?.is_admin));

  // The legacy per-action toasts rather than the module toolbar's
  // generic copy wording.
  async function copy(text: string, copiedTitle: string, copiedBody: string, failedBody: string) {
    try {
      await navigator.clipboard.writeText(text);
      notifySuccess(t(copiedTitle), t(copiedBody, { id: contract.id }));
    } catch {
      notifyError(t('contracts.actionsDropdown.copyFailedTitle'), t(failedBody));
    }
  }

  const copyId = () =>
    copy(
      String(contract.id),
      'contracts.actionsDropdown.idCopiedTitle',
      'contracts.actionsDropdown.idCopiedBody',
      'contracts.actionsDropdown.idCopyFailedBody',
    );
  const copyLink = () =>
    copy(
      `<url=contract:${CONTRACT_LINK_SYSTEM}//${contract.id}>Contract ${contract.id}</url>`,
      'contracts.actionsDropdown.linkCopiedTitle',
      'contracts.actionsDropdown.linkCopiedBody',
      'contracts.actionsDropdown.linkCopyFailedBody',
    );

  const openInGame = () => openContractInGame(contract.id);

  async function update(fields: Record<string, unknown>) {
    const response = await fetch(`/api/historic-contracts/${contract.id}`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(fields),
    });
    if (response.ok) {
      notifySuccess(
        t('contracts.actionsDropdown.updatedTitle'),
        t('contracts.actionsDropdown.updatedBody'),
      );
      await invalidateAll();
    } else {
      notifyError(
        t('contracts.actionsDropdown.updateFailedTitle'),
        t('contracts.actionsDropdown.updateFailedBody'),
      );
    }
  }
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <span {...props} class="inline-flex">
        <Button class="h-8 w-8 p-0" variant="ghost">
          <EllipsisVertical class="h-4 w-4" />
          <span class="sr-only">{t('contracts.actionsDropdown.openMenu')}</span>
        </Button>
      </span>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="end">
    <DropdownMenu.Item onclick={copyId}>
      <Copy class="size-4" />
      {t('contracts.actionsDropdown.copyId')}
    </DropdownMenu.Item>
    {#if contract.status !== 'failed'}
      <DropdownMenu.Item onclick={copyLink}>
        <FilePenLine class="size-4" />
        {t('contracts.actionsDropdown.copyContractLink')}
      </DropdownMenu.Item>
      <DropdownMenu.Item onclick={openInGame}>
        <ExternalLink class="size-4" />
        {t('contracts.actionsDropdown.openInGame')}
      </DropdownMenu.Item>
    {/if}
    {#if isAdmin && contract.status !== 'outstanding'}
      <DropdownMenu.Separator />
      <DropdownMenu.Item
        onclick={() => update({ ignore_for_training: !contract.ignore_for_training })}
      >
        <Sparkles class="size-4" />
        {contract.ignore_for_training
          ? t('contracts.actionsDropdown.includeInTraining')
          : t('contracts.actionsDropdown.ignoreForTraining')}
      </DropdownMenu.Item>
      <DropdownMenu.Item onclick={() => update({ non_abyssal_modules_count: 600 })}>
        <Sparkles class="size-4" />
        {t('contracts.actionsDropdown.setNonAbyssalModules', {
          count: contract.non_abyssal_modules_count,
        })}
      </DropdownMenu.Item>
      <DropdownMenu.Separator />
      <DropdownMenu.Item onclick={() => update({ status: 'failed' })}>
        <Sparkles class="size-4" />
        {t('contracts.actionsDropdown.setStatusFailed')}
      </DropdownMenu.Item>
      <DropdownMenu.Item onclick={() => update({ status: 'completed' })}>
        <Sparkles class="size-4" />
        {t('contracts.actionsDropdown.setStatusCompleted')}
      </DropdownMenu.Item>
      <DropdownMenu.Item onclick={() => update({ status: 'unknown' })}>
        <Sparkles class="size-4" />
        {t('contracts.actionsDropdown.setStatusUnknown')}
      </DropdownMenu.Item>
    {/if}
  </DropdownMenu.Content>
</DropdownMenu.Root>
