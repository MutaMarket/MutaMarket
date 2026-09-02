<script lang="ts">
  // One collection card of the index, the legacy
  // Collections/CollectionCard.vue: name link with the premium badge,
  // description excerpt, the owner with portrait, and the icon strip of
  // the collection's module types (+N beyond the cap). Owners get the
  // delete action with a confirm dialog.
  import { Trash2 } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import { holoTilt } from '$lib/holo-tilt';
  import { sparkleStyle } from '$lib/premium-foil';
  import GameImage from './game-image.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';
  import type { CollectionCardData } from '$lib/types-social';

  let {
    collection,
    owned = false,
  }: {
    collection: CollectionCardData;
    /** Whether the viewer owns it (shows delete + visibility). */
    owned?: boolean;
  } = $props();

  /** Icons shown before folding into "+N", like the legacy slice(6). */
  const TYPES_SHOWN = 6;

  let confirmingDelete = $state(false);
  let deleting = $state(false);

  const visibleTypes = $derived(collection.type_ids.slice(0, TYPES_SHOWN));
  const hiddenTypes = $derived(Math.max(collection.types_count - TYPES_SHOWN, 0));

  async function destroy() {
    deleting = true;
    try {
      await fetch(`/collections/${collection.slug}`, { method: 'DELETE', redirect: 'manual' });
      confirmingDelete = false;
      await invalidateAll();
    } finally {
      deleting = false;
    }
  }
</script>

<div
  use:holoTilt={collection.character_has_premium}
  style={collection.character_has_premium ? sparkleStyle(collection.slug) : undefined}
  class="group relative flex flex-col gap-3 rounded-lg bg-card p-4 transition-all hover:shadow-lg {collection.character_has_premium
    ? 'premium-card'
    : 'border border-border'}"
>
  <div class="flex items-start gap-2">
    <div class="min-w-0 grow">
      <a
        href="/collections/{collection.slug}"
        class="block truncate text-lg leading-tight font-medium hover:underline"
      >
        {collection.name}
      </a>
      <p class="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground">
        <img
          alt=""
          class="size-4 rounded"
          src="https://images.evetech.net/characters/{collection.character_id}/portrait?size=64"
        />
        <span class="truncate">by {collection.character_name}</span>
        {#if owned && collection.visibility !== 'public'}
          <span class="rounded-full border border-border px-1.5 text-[10px] uppercase">
            {collection.visibility}
          </span>
        {/if}
      </p>
    </div>
    {#if collection.character_has_premium}
      <span
        class="shrink-0 rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary"
      >
        Premium
      </span>
    {/if}
  </div>

  {#if collection.description}
    <p class="line-clamp-2 text-sm text-muted-foreground">{collection.description}</p>
  {/if}

  <div class="mt-auto flex items-center justify-between gap-2">
    <div class="flex items-center gap-1">
      {#each visibleTypes as typeId (typeId)}
        <GameImage
          src="https://images.evetech.net/types/{typeId}/icon?size=64"
          alt=""
          class="size-6 rounded"
        />
      {/each}
      {#if hiddenTypes > 0}
        <span class="text-xs text-muted-foreground">+{hiddenTypes}</span>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <span class="text-xs text-muted-foreground tabular-nums">
        {collection.modules_count.toLocaleString('en-US')} modules
      </span>
      {#if owned}
        <Button
          variant="ghost"
          size="icon-sm"
          class="text-muted-foreground hover:text-negative"
          title="Delete collection"
          onclick={() => (confirmingDelete = true)}
        >
          <Trash2 class="size-3.5" />
        </Button>
      {/if}
    </div>
  </div>
</div>

<Dialog.Root bind:open={confirmingDelete}>
  <Dialog.Content>
    <Dialog.Title>Delete collection</Dialog.Title>
    <Dialog.Description>
      Are you sure you want to delete this collection? This action cannot be undone!
    </Dialog.Description>
    <Dialog.Footer>
      <Button variant="secondary" onclick={() => (confirmingDelete = false)}>Cancel</Button>
      <Button variant="destructive" disabled={deleting} onclick={destroy}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
