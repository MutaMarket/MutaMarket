<script lang="ts">
  // The recommended-gear management page, the legacy Admin/GearItemPage:
  // an in-place create/edit form card above the list card (editing
  // scrolls back up to the form), plus toggle and delete. Divergence:
  // images are referenced by URL like the advertisements port - the
  // legacy file upload needs a public-disk story the rewrite does not
  // have yet.
  import { Pencil, ShoppingBag, Trash2 } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Switch } from '$lib/components/ui/switch';
  import { notifyError, notifySuccess } from '$lib/toast';
  import { t } from '$lib/i18n.svelte';
  import type { PageProps } from './$types';
  import type { AdminGearItem } from './+page.server';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  let editing = $state<AdminGearItem | null>(null);
  let confirmingDelete = $state<AdminGearItem | null>(null);
  let submitting = $state(false);

  // The form card's state, shared by create and edit.
  let form = $state({
    name: '',
    description: '',
    image_url: '',
    link: '',
    priority: 0,
    active: true,
  });

  function resetForm() {
    form = { name: '', description: '', image_url: '', link: '', priority: 0, active: true };
  }

  function startEdit(gearItem: AdminGearItem) {
    form = {
      name: gearItem.name,
      description: gearItem.description ?? '',
      image_url: gearItem.image_url ?? '',
      link: gearItem.link,
      priority: gearItem.priority,
      active: gearItem.active,
    };
    editing = gearItem;
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function cancelEdit() {
    editing = null;
    resetForm();
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    submitting = true;
    try {
      const payload = {
        name: form.name,
        description: form.description || null,
        image_url: form.image_url,
        link: form.link,
        priority: form.priority,
        active: form.active,
      };
      const response = await fetch(
        editing === null ? '/api/admin/gear-items' : `/api/admin/gear-items/${editing.id}`,
        {
          method: editing === null ? 'POST' : 'PUT',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify(payload),
        },
      );
      if (response.ok) {
        notifySuccess(
          editing === null ? t('admin.gearItems.createdTitle') : t('admin.gearItems.updatedTitle'),
          editing === null
            ? t('admin.gearItems.createdBody', { name: form.name })
            : t('admin.gearItems.updatedBody', { name: form.name }),
        );
        cancelEdit();
        await invalidateAll();
      } else {
        const body = (await response.json().catch(() => null)) as {
          errors?: Record<string, string[]>;
          message?: string;
        } | null;
        const first = body?.errors ? Object.values(body.errors)[0]?.[0] : undefined;
        notifyError(
          t('admin.common.notSavedTitle'),
          first ?? body?.message ?? t('admin.common.somethingWentWrong'),
        );
      }
    } finally {
      submitting = false;
    }
  }

  async function toggle(gearItem: AdminGearItem) {
    await fetch(`/api/admin/gear-items/${gearItem.id}/toggle`, { method: 'PATCH' });
    notifySuccess(
      t('admin.gearItems.updatedTitle'),
      gearItem.active ? t('admin.gearItems.deactivatedBody') : t('admin.gearItems.activatedBody'),
    );
    await invalidateAll();
  }

  async function destroy() {
    if (confirmingDelete === null) return;
    await fetch(`/api/admin/gear-items/${confirmingDelete.id}`, { method: 'DELETE' });
    notifySuccess(
      t('admin.gearItems.deletedTitle'),
      t('admin.gearItems.deletedBody', { name: confirmingDelete.name }),
    );
    confirmingDelete = null;
    await invalidateAll();
  }
</script>

<PageMeta
  title={t('meta.adminGearItems.title')}
  description={t('meta.adminGearItems.description')}
/>

<!-- The form card: create, or edit the selected item in place. -->
<div class="hud-frame mb-6 p-4">
  <div class="mb-3">
    <span class="block text-lg font-medium">
      {editing === null ? t('admin.gearItems.createTitle') : t('admin.gearItems.editTitle')}
    </span>
    <p class="text-sm text-muted-foreground">
      {editing === null
        ? t('admin.gearItems.createDescription')
        : t('admin.gearItems.editDescription')}
    </p>
  </div>
  <form class="grid gap-4" onsubmit={submit}>
    <div class="flex flex-col gap-1.5">
      <Label for="gear-name">{t('common.labels.name')}</Label>
      <Input
        id="gear-name"
        bind:value={form.name}
        placeholder={t('admin.gearItems.namePlaceholder')}
        required
      />
    </div>
    <div class="flex flex-col gap-1.5">
      <Label for="gear-description">{t('admin.gearItems.descriptionOptional')}</Label>
      <textarea
        id="gear-description"
        bind:value={form.description}
        placeholder={t('admin.gearItems.descriptionPlaceholder')}
        rows="2"
        class="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-xs transition-colors placeholder:text-muted-foreground focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-hidden"
      ></textarea>
    </div>
    <div class="flex flex-col gap-1.5">
      <Label for="gear-link">{t('admin.gearItems.link')}</Label>
      <Input
        id="gear-link"
        bind:value={form.link}
        placeholder="https://geni.us/..."
        required
        type="url"
      />
      <p class="text-xs text-muted-foreground">
        {t('admin.gearItems.linkHint')}
      </p>
    </div>
    <div class="flex flex-col gap-1.5">
      <Label for="gear-image">{t('admin.gearItems.imageUrl')}</Label>
      <Input
        id="gear-image"
        bind:value={form.image_url}
        placeholder="https://…"
        required
        type="url"
      />
      {#if form.image_url.startsWith('http')}
        <img
          src={form.image_url}
          alt={t('admin.gearItems.currentImageAlt')}
          class="mt-1 aspect-square w-24 rounded-lg object-cover"
        />
      {/if}
    </div>
    <div class="flex flex-col gap-1.5 sm:max-w-48">
      <Label for="gear-priority">{t('admin.gearItems.priority')}</Label>
      <Input id="gear-priority" bind:value={form.priority} type="number" min="0" />
      <p class="text-xs text-muted-foreground">{t('admin.gearItems.priorityHint')}</p>
    </div>
    <div class="flex items-center gap-2">
      <Switch id="gear-active" bind:checked={form.active} />
      <Label for="gear-active">{t('admin.gearItems.active')}</Label>
    </div>
    <div class="flex gap-2">
      <Button
        type="submit"
        disabled={submitting || form.name === '' || form.image_url === '' || form.link === ''}
      >
        {editing === null ? t('admin.gearItems.createTitle') : t('admin.gearItems.saveChanges')}
      </Button>
      {#if editing !== null}
        <Button type="button" variant="outline" onclick={cancelEdit}>
          {t('common.actions.cancel')}
        </Button>
      {/if}
    </div>
  </form>
</div>

<!-- The list card. -->
<div class="hud-frame p-4">
  <div class="mb-3">
    <span class="block text-lg font-medium">{t('admin.gearItems.listTitle')}</span>
    <p class="text-sm text-muted-foreground">
      {t('admin.gearItems.totalCount', { count: data.gearItems.length })}
    </p>
  </div>
  {#if data.gearItems.length === 0}
    <p class="text-sm text-muted-foreground">{t('admin.gearItems.empty')}</p>
  {:else}
    <ul class="flex flex-col">
      {#each data.gearItems as gearItem (gearItem.id)}
        <li
          class="flex flex-wrap items-center gap-4 border-b border-border py-3 last:border-b-0 last:pb-0 first:pt-0"
        >
          {#if gearItem.image_url}
            <img
              src={gearItem.image_url}
              alt={gearItem.name}
              class="aspect-square w-12 rounded object-cover"
            />
          {:else}
            <div class="grid size-12 place-items-center rounded bg-card-2">
              <ShoppingBag class="size-5 text-muted-foreground" />
            </div>
          {/if}
          <div class="min-w-40 flex-1">
            <span class="font-medium">{gearItem.name}</span>
            <a
              href={gearItem.link}
              class="block max-w-64 truncate text-xs text-muted-foreground hover:underline"
              rel="noopener noreferrer"
              target="_blank"
            >
              {gearItem.link}
            </a>
          </div>
          <div class="text-xs text-muted-foreground">
            {t('admin.gearItems.priorityValue', { priority: gearItem.priority })}
          </div>
          <div class="flex items-center gap-2">
            <Switch checked={gearItem.active} onCheckedChange={() => toggle(gearItem)} />
            <Button
              variant="ghost"
              size="icon"
              class="size-8"
              aria-label={t('common.actions.edit')}
              onclick={() => startEdit(gearItem)}
            >
              <Pencil class="size-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              class="size-8 text-muted-foreground hover:text-red-500"
              aria-label={t('common.actions.delete')}
              onclick={() => (confirmingDelete = gearItem)}
            >
              <Trash2 class="size-4" />
            </Button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<Dialog.Root
  open={confirmingDelete !== null}
  onOpenChange={(open) => !open && (confirmingDelete = null)}
>
  <Dialog.Content class="sm:max-w-sm">
    <Dialog.Header>
      <Dialog.Title>
        {t('admin.gearItems.deleteConfirm', { name: confirmingDelete?.name ?? '' })}
      </Dialog.Title>
      <Dialog.Description>{t('admin.gearItems.deleteDescription')}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="secondary" onclick={() => (confirmingDelete = null)}>
        {t('common.actions.cancel')}
      </Button>
      <Button variant="destructive" onclick={destroy}>{t('common.actions.delete')}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
