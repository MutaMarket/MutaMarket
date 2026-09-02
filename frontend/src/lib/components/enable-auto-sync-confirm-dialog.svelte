<script lang="ts">
  // The legacy EnableAutoSyncConfirmDialog.vue: confirm before the
  // enable wipes the collection and replaces it with the tracked
  // locations' modules.
  import { invalidateAll } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import { t } from '$lib/i18n.svelte';
  import { notifySuccess } from '$lib/toast';

  let { open = $bindable(false), slug }: { open?: boolean; slug: string } = $props();

  let submitting = $state(false);

  async function enableAutoSync() {
    submitting = true;
    try {
      const response = await fetch(`/collections/${slug}/auto-sync`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({}),
      });
      if (response.ok || response.redirected) {
        // The legacy enable notification.
        notifySuccess(
          t('collections.notifications.autoSyncEnabledTitle'),
          t('collections.notifications.autoSyncEnabledBody'),
        );
        open = false;
        await invalidateAll();
      }
    } finally {
      submitting = false;
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title>{t('collections.autoSync.enableTitle')}</Dialog.Title>
      <Dialog.Description class="space-y-2">
        <p>{t('collections.autoSync.enableBody1')}</p>
        <p>{t('collections.autoSync.enableBody2')}</p>
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="secondary" onclick={() => (open = false)}
        >{t('common.actions.cancel')}</Button
      >
      <Button disabled={submitting} onclick={enableAutoSync}>
        {t('collections.autoSync.enableTitle')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
