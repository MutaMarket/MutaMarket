<script lang="ts">
  // The list-row note / collection-note / asking-price cell, the legacy
  // ListNote / ListCollectionNote / ListAskingPrice trio. It is one
  // narrow column, so a stored value shows as a hover card and only the
  // mode being edited turns into a field.
  import { Coins, NotebookPen } from '@lucide/svelte';
  import CurrencyInput from './currency-input.svelte';
  import * as HoverCard from '$lib/components/ui/hover-card';
  import {
    canEditCollectionNote,
    canSetPrice,
    navCharacterIds,
    collectionNote,
    MAX_ASKING_PRICE,
    draftValue,
    editSession,
    note,
    openCollection,
    parsePrice,
    setDraft,
    type EditMode,
  } from '$lib/module-edits';
  import { page } from '$app/state';
  import type { ModuleDetail } from '$lib/types';

  let { module }: { module: ModuleDetail } = $props();

  const session = $derived($editSession);
  const characterIds = $derived(navCharacterIds(page.data.nav));

  const allowed = $derived.by(() => {
    switch (session?.mode) {
      case 'note':
        return page.data.nav?.user != null;
      case 'collection-note':
        return canEditCollectionNote($openCollection, characterIds);
      case 'price':
        return canSetPrice(module, characterIds);
      default:
        return false;
    }
  });

  const mode = $derived<EditMode | null>(allowed && session !== null ? session.mode : null);
  const value = $derived(session === null ? '' : draftValue(session, module));

  // Outside a session the cell shows whichever notes the module carries.
  const stored = $derived(
    [note(module)?.content, collectionNote(module)?.content].filter(
      (content): content is string => (content ?? '') !== '',
    ),
  );
</script>

<div class="flex items-center px-2">
  {#if mode === 'price'}
    <div class="w-40">
      <CurrencyInput
        {value}
        label="Asking price"
        empty="no price"
        unit={false}
        max={MAX_ASKING_PRICE}
        onchange={(text) => setDraft(module, text)}
      />
    </div>
  {:else if mode !== null}
    <textarea
      {value}
      rows="1"
      aria-label={mode === 'note' ? 'Note' : 'Collection note'}
      placeholder="Note"
      class="w-full resize-none border border-border bg-background px-2 py-1 text-xs focus:outline-none"
      oninput={(event) => setDraft(module, event.currentTarget.value)}></textarea>
  {:else if stored.length > 0}
    <HoverCard.Root>
      <HoverCard.Trigger>
        {#snippet child({ props })}
          <span {...props} class="cursor-default text-lime-500">
            <NotebookPen class="size-4" />
          </span>
        {/snippet}
      </HoverCard.Trigger>
      <HoverCard.Content class="border" side="right">
        <div class="grid gap-2 text-sm">
          {#each stored as content, index (index)}
            <p>{content}</p>
          {/each}
        </div>
      </HoverCard.Content>
    </HoverCard.Root>
  {:else if canSetPrice(module, characterIds) && module.public_asset?.price}
    <span class="text-amber-500" title="Your asking price">
      <Coins class="size-4" />
    </span>
  {/if}
</div>
