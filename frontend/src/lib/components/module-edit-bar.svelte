<script lang="ts">
  // The floating save bar for a running edit session, the legacy
  // NoteMenu.vue and PricingMenu.vue. One bar for all three modes,
  // since only one runs at a time.
  import { Coins, NotebookPen } from '@lucide/svelte';
  import { refreshAll } from '$app/navigation';
  import { Button } from '#lib/components/ui/button/index.js';
  import { t } from '#lib/i18n.svelte.js';
  import { cancelEdit, editSession, isValid, saveEdits } from '#lib/module-edits.js';
  import { notifyError, notifySuccess } from '#lib/toast.js';

  const session = $derived($editSession);

  const label = $derived.by(() => {
    switch (session?.mode) {
      case 'price':
        return t('misc.pricing.editingEnabled');
      case 'collection-note':
        return t('collections.noteMenu.editingEnabled');
      default:
        return t('misc.notes.editingEnabled');
    }
  });

  let saving = $state(false);

  async function save() {
    saving = true;
    const outcome = await saveEdits();
    saving = false;

    if (outcome === 'failed') {
      notifyError(t('misc.editBar.saveFailedTitle'), t('misc.editBar.saveFailedBody'));
      return;
    }
    if (outcome === 'saved') {
      notifySuccess(t('misc.editBar.savedTitle'), t('misc.editBar.savedBody'));
      await refreshAll();
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
      <NotebookPen class="size-4 text-primary" />
    {/if}
    <span class="text-sm">{label}</span>
    <Button variant="secondary" size="sm" onclick={cancelEdit}>{t('common.actions.cancel')}</Button>
    <Button size="sm" disabled={saving || !isValid(session)} onclick={save}>
      {saving ? t('misc.editBar.saving') : t('common.actions.save')}
    </Button>
  </div>
{/if}
