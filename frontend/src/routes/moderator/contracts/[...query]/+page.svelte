<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  // The moderator contract review page, mirroring the legacy
  // Moderator/ContractReviewPage.vue: one random unreviewed contract
  // with its module card and facts, the status buttons behind a
  // confirm dialog, the copyable in-game link (auto-copied when the
  // next contract arrives), the status guide, the how-to section and
  // the one-time instructions acknowledgment kept in localStorage.
  import { Copy, FileSignature } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import ContractStatusBadge from '$lib/components/contract-status-badge.svelte';
  import GameImage from '$lib/components/game-image.svelte';
  import ModuleCard from '$lib/components/module-card.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import TypeDialog from '$lib/components/type-dialog.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import {
    REVIEW_ACTIONS,
    shortcutAction,
    statusLabel,
    type ReviewStatus,
  } from '$lib/contract-review';
  import { parseDbTimestamp, relativeTime } from '$lib/duration';
  import { toHistoricContractLink } from '$lib/export';
  import { t } from '$lib/i18n.svelte';
  import Trans from '$lib/components/trans.svelte';
  import { parseQueryUi } from '$lib/query';
  import { notifyError, notifySuccess } from '$lib/toast';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const settings = useDisplaySettings();
  const search = $derived(parseQueryUi(data.query));
  const contract = $derived(data.review.contract);

  /** The legacy useLocalStorage key of the acknowledgment gate. */
  const ACKNOWLEDGED_KEY = 'contractReviewInstructionsAcknowledged';

  let showConfirmDialog = $state(false);
  let selectedStatus = $state<ReviewStatus | null>(null);
  let showInstructionsDialog = $state(false);
  let hasReadInstructions = $state(false);

  $effect(() => {
    let acknowledged = false;
    try {
      acknowledged = localStorage.getItem(ACKNOWLEDGED_KEY) === 'true';
    } catch {
      // Storage-less contexts simply show the instructions.
    }
    if (acknowledged) {
      hasReadInstructions = true;
    } else {
      showInstructionsDialog = true;
    }
  });

  function acknowledgeInstructions() {
    try {
      localStorage.setItem(ACKNOWLEDGED_KEY, 'true');
    } catch {
      // Acknowledging still unlocks this visit.
    }
    hasReadInstructions = true;
    showInstructionsDialog = false;
  }

  function confirmStatusUpdate(status: ReviewStatus) {
    if (!hasReadInstructions) {
      showInstructionsDialog = true;
      return;
    }
    selectedStatus = status;
    showConfirmDialog = true;
  }

  async function updateContractStatus(status: ReviewStatus) {
    if (!contract) return;
    const response = await fetch(`/moderator/contracts/${contract.id}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status }),
      redirect: 'manual',
    });
    showConfirmDialog = false;
    if (response.status === 409 || response.status === 422) {
      const body: { message?: string } = await response.json().catch(() => ({}));
      notifyError(
        t('admin.contractReview.reviewFailedTitle'),
        body.message ?? t('admin.contractReview.reviewFailedBody'),
      );
    }
    // The legacy back() redirect reloads the page: the next random
    // contract arrives (and its link is auto-copied below).
    await invalidateAll();
  }

  function failedToCopy() {
    notifyError(
      t('admin.contractReview.copyFailedTitle'),
      t('admin.contractReview.copyFailedBody'),
    );
  }

  async function copyText(text: string): Promise<boolean> {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      return false;
    }
  }

  async function handleCopyId() {
    if (!contract) return;
    if (await copyText(contract.id.toString())) {
      notifySuccess(
        t('admin.contractReview.idCopiedTitle'),
        t('admin.contractReview.idCopiedBody', { id: contract.id }),
      );
    } else {
      failedToCopy();
    }
  }

  async function handleCopyContractLink() {
    if (!contract) return;
    if (await copyText(toHistoricContractLink(contract))) {
      notifySuccess(
        t('admin.contractReview.linkCopiedTitle'),
        t('admin.contractReview.linkCopiedBody', { name: contract.issuer.name }),
      );
    } else {
      failedToCopy();
    }
  }

  function refreshContract() {
    void invalidateAll();
  }

  // The legacy watcher: a newly loaded contract copies its link.
  let previousContractId: number | null = null;
  $effect(() => {
    const id = contract?.id ?? null;
    if (id !== null && previousContractId !== null && id !== previousContractId) {
      void handleCopyContractLink();
    }
    previousContractId = id;
  });

  // The legacy magic keys; Shift+C and Shift+X deliberately bypass the
  // confirm dialog, like the legacy watchers.
  function handleKeydown(event: KeyboardEvent) {
    const action = shortcutAction(event);
    if (action === 'copy-link') {
      void handleCopyContractLink();
    } else if (action !== null) {
      void updateContractStatus(action);
    }
  }

  function formatDate(dateString: string | null): string {
    if (!dateString) return '';
    return relativeTime(parseDbTimestamp(dateString) - Date.now() / 1000);
  }

  function absoluteDate(dateString: string | null): string {
    if (!dateString) return '';
    return new Date(parseDbTimestamp(dateString) * 1000).toLocaleDateString();
  }

  /** The legacy status guide cards; the hint carries its styled status
   * word as a Trans slot. */
  const STATUS_GUIDE = [
    {
      image: '/img/contracts/not-found.png',
      altKey: 'admin.contractReview.notFoundAlt',
      titleKey: 'admin.contractReview.notFound',
      hintKey: 'admin.contractReview.notFoundHint',
      hintStatusKey: 'admin.contractReview.statuses.failed',
      accent: 'text-red-500',
    },
    {
      image: '/img/contracts/completed.png',
      altKey: 'admin.contractReview.completedAlt',
      titleKey: 'admin.contractReview.statuses.completed',
      hintKey: 'admin.contractReview.completedHint',
      hintStatusKey: 'admin.contractReview.statuses.completed',
      accent: 'text-primary',
    },
    {
      image: '/img/contracts/expired.png',
      altKey: 'admin.contractReview.failedAlt',
      titleKey: 'admin.contractReview.statuses.failed',
      hintKey: 'admin.contractReview.failedHint',
      hintStatusKey: 'admin.contractReview.statuses.failed',
      accent: 'text-red-500',
    },
  ];

  const WORKFLOW_STEPS = [1, 2, 3, 4, 5].map((step) => [
    `admin.contractReview.step${step}Title`,
    `admin.contractReview.step${step}Body`,
  ]);

  const TIPS = [1, 2, 3].map((tip) => `admin.contractReview.tip${tip}`);
</script>

<PageMeta
  title={t('meta.contractReview.title')}
  description={t('meta.contractReview.description')}
/>
<svelte:window onkeydown={handleKeydown} />

{#snippet instructions()}
  <div class="grid gap-6">
    <div>
      <h3 class="mb-2 text-lg font-medium">{t('admin.contractReview.purposeTitle')}</h3>
      <p class="text-muted-foreground">{t('admin.contractReview.purposeBody')}</p>
    </div>
    <div>
      <h3 class="mb-2 text-lg font-medium">{t('admin.contractReview.workflowTitle')}</h3>
      <ol class="list-decimal space-y-2 pl-5 text-muted-foreground">
        {#each WORKFLOW_STEPS as [title, body] (title)}
          <li><span class="font-medium">{t(title)}</span> - {t(body)}</li>
        {/each}
      </ol>
    </div>
    <div>
      <h3 class="mb-2 text-lg font-medium">{t('admin.contractReview.tipsTitle')}</h3>
      <ul class="list-disc space-y-2 pl-5 text-muted-foreground">
        {#each TIPS as tip (tip)}
          <li>{t(tip)}</li>
        {/each}
      </ul>
    </div>
    <div class="rounded-md border border-red-500 bg-red-950/50 p-4 text-red-500">
      <h3 class="mb-1 font-medium">{t('admin.contractReview.noticeTitle')}</h3>
      <p class="text-sm">{t('admin.contractReview.noticeBody')}</p>
    </div>
  </div>
{/snippet}

<PageHeader title={t('admin.contractReview.title')}>
  {#snippet icon()}
    <div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
      <FileSignature class="size-5 text-primary" stroke-width={1.5} />
    </div>
  {/snippet}
  {#snippet actions()}
    <Button variant="outline" onclick={refreshContract}>
      {t('admin.contractReview.getNextContract')}
    </Button>
  {/snippet}
</PageHeader>

{#if contract}
  <div class="grid gap-6">
    <div class="hud-frame w-full overflow-hidden">
      <div class="border-b border-border bg-card-1 p-6">
        <div class="flex items-center justify-between gap-4">
          <div>
            <h2 class="text-lg font-semibold">
              {t('admin.contractReview.contractNumber', { id: contract.id })}
            </h2>
            <p class="mt-2 text-sm text-muted-foreground">
              {t('admin.contractReview.reviewDescription')}
            </p>
          </div>
          <div class="w-64">
            <TypeDialog
              prefix="moderator/contracts"
              {search}
              currentTypeId={data.review.search.type?.id ?? null}
              currentTypeName={data.review.search.type?.name ?? null}
            />
          </div>
        </div>
      </div>
      <div class="p-6">
        {#if contract.modules.length > 0}
          <div class="grid gap-6">
            {#each contract.modules as module (module.id)}
              <div class="grid grid-cols-1 gap-6 md:grid-cols-2">
                <div class="w-full">
                  <ModuleCard {module} {settings} />
                </div>
                <div class="grid gap-6">
                  <div class="rounded-lg border border-border bg-card-1 p-4">
                    <div class="flex items-center gap-3">
                      <GameImage
                        src="https://images.evetech.net/characters/{contract.issuer
                          .id}/portrait?size=64"
                        alt={contract.issuer.name}
                        class="size-10 rounded-lg"
                      />
                      <div>
                        <h3 class="font-medium">{t('admin.contractReview.issuedBy')}</h3>
                        <p class="text-sm text-muted-foreground">{contract.issuer.name}</p>
                      </div>
                    </div>
                  </div>
                  <div class="rounded-lg border border-border bg-card-1 p-4">
                    <div class="grid grid-cols-2 gap-4">
                      <div>
                        <h3 class="text-sm font-medium text-muted-foreground">
                          {t('admin.contractReview.dateIssued')}
                        </h3>
                        <p class="font-medium">{absoluteDate(contract.date_issued)}</p>
                        <p class="text-xs text-muted-foreground">
                          {formatDate(contract.date_issued)}
                        </p>
                      </div>
                      <div>
                        <h3 class="text-sm font-medium text-muted-foreground">
                          {t('common.labels.price')}
                        </h3>
                        <p class="font-medium">
                          {(contract.price ?? 0).toLocaleString('en-US')} ISK
                        </p>
                      </div>
                    </div>
                  </div>
                  <div class="rounded-lg border border-border bg-card-1 p-4">
                    <h3 class="mb-1 text-sm font-medium text-muted-foreground">
                      {t('admin.contractReview.contractType')}
                    </h3>
                    <p class="font-medium capitalize">{contract.type.replace('_', ' ')}</p>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
      <div class="border-t border-border p-4">
        <div class="flex w-full flex-wrap items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <Button size="sm" variant="outline" onclick={handleCopyId}>
              <Copy class="mr-1 size-4" />
              {t('admin.contractReview.copyId')}
            </Button>
            <Button size="sm" variant="outline" onclick={handleCopyContractLink}>
              <FileSignature class="mr-1 size-4" />
              {t('admin.contractReview.copyLink')}
            </Button>
          </div>
          <div class="flex flex-wrap gap-2">
            {#each REVIEW_ACTIONS as action (action.status)}
              <Button
                size="sm"
                variant={action.variant}
                onclick={() => confirmStatusUpdate(action.status)}
              >
                {t(action.labelKey)}
              </Button>
            {/each}
          </div>
        </div>
      </div>
    </div>

    <div class="hud-frame p-6">
      <h2 class="text-lg font-semibold">{t('admin.contractReview.statusGuideTitle')}</h2>
      <p class="mb-6 mt-1 text-sm text-muted-foreground">
        {t('admin.contractReview.statusGuideDescription')}
      </p>
      <div class="grid grid-cols-1 gap-6 md:grid-cols-3">
        {#each STATUS_GUIDE as guide (guide.image)}
          <div class="row-span-2 grid grid-rows-subgrid items-center gap-2">
            <div class="overflow-hidden rounded-lg border border-border">
              <img src={guide.image} alt={t(guide.altKey)} class="h-auto w-full object-cover" />
            </div>
            <div class="text-center">
              <h3 class="font-medium">{t(guide.titleKey)}</h3>
              <p class="text-sm text-muted-foreground">
                <Trans key={guide.hintKey}>
                  {#snippet status()}
                    <span class="font-medium {guide.accent}">{t(guide.hintStatusKey)}</span>
                  {/snippet}
                </Trans>
              </p>
            </div>
          </div>
        {/each}
      </div>
    </div>

    <div class="hud-frame p-6">
      <h2 class="text-lg font-semibold">{t('admin.contractReview.howToUseTitle')}</h2>
      <p class="mb-6 mt-1 text-sm text-muted-foreground">
        {t('admin.contractReview.howToUseDescription')}
      </p>
      {@render instructions()}
    </div>
  </div>
{:else}
  <div class="py-12 text-center">
    <h2 class="mb-2 text-xl font-medium">{t('admin.contractReview.noContractsTitle')}</h2>
    <p class="mb-4 text-muted-foreground">{t('admin.contractReview.noContractsBody')}</p>
    <Button variant="default" onclick={refreshContract}>
      {t('admin.contractReview.getNextContract')}
    </Button>
  </div>
{/if}

<Dialog.Root bind:open={showConfirmDialog}>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>{t('admin.contractReview.confirmTitle')}</Dialog.Title>
      <Dialog.Description>
        <Trans key="admin.contractReview.confirmBody" params={{ id: contract?.id ?? '' }}>
          {#snippet status()}
            <span class="font-medium">{selectedStatus ? statusLabel(selectedStatus) : ''}</span>
          {/snippet}
        </Trans>
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (showConfirmDialog = false)}>
        {t('common.actions.cancel')}
      </Button>
      {#if selectedStatus}
        <Button
          variant={selectedStatus === 'failed' ? 'destructive' : 'default'}
          onclick={() => updateContractStatus(selectedStatus as ReviewStatus)}
        >
          {t('common.actions.confirm')}
        </Button>
      {/if}
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={showInstructionsDialog}>
  <Dialog.Content class="max-w-3xl">
    <Dialog.Header>
      <Dialog.Title>{t('admin.contractReview.instructionsTitle')}</Dialog.Title>
      <Dialog.Description>
        {t('admin.contractReview.instructionsDescription')}
      </Dialog.Description>
    </Dialog.Header>
    <div class="max-h-[60vh] overflow-y-auto py-4">
      {@render instructions()}
    </div>
    <Dialog.Footer>
      <Button variant="default" onclick={acknowledgeInstructions}>
        {t('admin.contractReview.acknowledge')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
