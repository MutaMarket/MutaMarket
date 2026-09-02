<script lang="ts">
  // The floating save bar for a running edit session, the legacy
  // NoteMenu.vue and PricingMenu.vue. One bar for all three modes,
  // since only one runs at a time.
  import { Coins, NotebookPen } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import { cancelEdit, editSession, isValid, saveEdits } from '$lib/module-edits';
  import { notifyError, notifySuccess } from '$lib/toast';

  const session = $derived($editSession);

  const label = $derived.by(() => {
    switch (session?.mode) {
      case 'price':
        return 'Editing asking prices';
      case 'collection-note':
        return 'Editing collection notes';
      default:
        return 'Editing notes';
    }
  });

  let saving = $state(false);

  async function save() {
    saving = true;
    const outcome = await saveEdits();
    saving = false;

    if (outcome === 'failed') {
      notifyError('Could not save your changes', 'Please try again.');
      return;
    }
    if (outcome === 'saved') {
      notifySuccess('Saved', 'Your changes are live.');
      await invalidateAll();
    }
  }
</script>

{#if session}
  <div
    class="fixed bottom-8 left-1/2 z-40 flex -translate-x-1/2 items-center gap-4 rounded-md border border-border bg-card-1 p-4 px-6 shadow-xl"
  >
    {#if session.mode === 'price'}
      <Coins class="size-4 text-amber-500" />
    {:else}
      <NotebookPen class="size-4 text-lime-500" />
    {/if}
    <span class="text-sm">{label}</span>
    <Button variant="secondary" size="sm" onclick={cancelEdit}>Cancel</Button>
    <Button size="sm" disabled={saving || !isValid(session)} onclick={save}>
      {saving ? 'Saving…' : 'Save'}
    </Button>
  </div>
{/if}
