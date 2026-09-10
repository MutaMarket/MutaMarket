<script lang="ts">
  // The edit-collection dialog, the legacy EditCollectionModal.vue: the
  // form seeded from the collection, PUT to the collection route and
  // following its redirect (the slug follows the name).
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as RadioGroup from '$lib/components/ui/radio-group';
  import { t } from '$lib/i18n.svelte';
  import { notifySuccess } from '$lib/toast';
  import type { CollectionCardData } from '$lib/types-social';

  let {
    open = $bindable(false),
    collection,
  }: {
    open?: boolean;
    collection: CollectionCardData;
  } = $props();

  const VISIBILITIES = ['public', 'private', 'unlisted'] as const;
  const VISIBILITY_HELP_KEY: Record<string, string> = {
    public: 'collections.dialogs.publicHelp',
    private: 'collections.dialogs.privateHelp',
    unlisted: 'collections.dialogs.unlistedHelp',
  };

  let name = $state('');
  let description = $state('');
  let visibility = $state('private');
  let errors = $state<Record<string, string[]>>({});
  let message = $state<string | null>(null);
  let submitting = $state(false);

  // Reseed from the collection each time the dialog opens, so a
  // cancelled edit does not leak into the next one.
  $effect(() => {
    if (open) {
      name = collection.name;
      description = collection.description ?? '';
      visibility = collection.visibility;
      errors = {};
      message = null;
    }
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    submitting = true;
    errors = {};
    message = null;
    try {
      const response = await fetch(`/collections/${collection.slug}`, {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ name, description: description || null, visibility }),
        redirect: 'follow',
      });
      if (response.redirected) {
        open = false;
        notifySuccess(
          t('collections.notifications.updatedTitle'),
          t('collections.notifications.updatedBody'),
        );
        await goto(new URL(response.url).pathname, { invalidateAll: true });
        return;
      }
      const body: { message?: string; errors?: Record<string, string[]> } = await response
        .json()
        .catch(() => ({}));
      errors = body.errors ?? {};
      if (Object.keys(errors).length === 0) {
        message = body.message ?? t('collections.dialogs.updateFailed');
      }
    } finally {
      submitting = false;
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="sm:max-w-3xl">
    <Dialog.Title>{t('collections.show.editCollection')}</Dialog.Title>
    <Dialog.Description>{t('collections.dialogs.editBody')}</Dialog.Description>
    <form class="grid gap-4" onsubmit={submit}>
      <div class="grid gap-1.5">
        <Label for="edit-collection-name">{t('common.labels.name')}</Label>
        <Input id="edit-collection-name" bind:value={name} type="text" />
        {#if errors.name}
          <p class="text-sm text-negative">{errors.name[0]}</p>
        {/if}
      </div>
      <div class="grid gap-1.5">
        <Label for="edit-collection-description">{t('collections.dialogs.description')}</Label>
        <textarea
          id="edit-collection-description"
          bind:value={description}
          rows="3"
          class="w-full resize-none rounded-md border border-border bg-transparent px-3 py-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30"
        ></textarea>
        {#if errors.description}
          <p class="text-sm text-negative">{errors.description[0]}</p>
        {/if}
      </div>
      <div class="grid gap-1.5">
        <Label>{t('collections.dialogs.visibility')}</Label>
        <RadioGroup.Root bind:value={visibility} class="grid gap-4 md:grid-cols-3">
          {#each VISIBILITIES as option (option)}
            <div class="grid grid-cols-[auto_1fr] items-center gap-x-2">
              <RadioGroup.Item id="edit-collection-visibility-{option}" value={option} />
              <Label for="edit-collection-visibility-{option}">
                {t(`collections.visibility.${option}`)}
              </Label>
              <p class="col-start-2 self-start text-xs text-muted-foreground">
                {t(VISIBILITY_HELP_KEY[option])}
              </p>
            </div>
          {/each}
        </RadioGroup.Root>
        {#if errors.visibility}
          <p class="text-sm text-negative">{errors.visibility[0]}</p>
        {/if}
      </div>
      {#if message}
        <p class="text-sm text-negative">{message}</p>
      {/if}
      <Dialog.Footer>
        <Button type="button" variant="secondary" onclick={() => (open = false)}>
          {t('common.actions.cancel')}
        </Button>
        <Button type="submit" disabled={submitting || name.trim() === ''}>
          {t('common.actions.save')}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
