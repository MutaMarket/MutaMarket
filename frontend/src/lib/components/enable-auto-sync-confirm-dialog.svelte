<script lang="ts">
  // The legacy EnableAutoSyncConfirmDialog.vue: confirm before the
  // enable wipes the collection and replaces it with the tracked
  // locations' modules.
  import { invalidateAll } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
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
          'Auto-sync enabled',
          'This collection will now automatically sync with the selected locations on each asset import.',
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
      <Dialog.Title>Enable Auto-Sync</Dialog.Title>
      <Dialog.Description class="space-y-2">
        <p>
          Enabling auto-sync will clear all current modules in this collection and replace them with
          modules from the selected locations.
        </p>
        <p>This will happen automatically whenever your assets are imported.</p>
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="secondary" onclick={() => (open = false)}>Cancel</Button>
      <Button disabled={submitting} onclick={enableAutoSync}>Enable Auto-Sync</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
