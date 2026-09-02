<script lang="ts">
  // The collections index, the legacy ShowAllCollectionsPage: debounced
  // search, the create dialog, the viewer's own collections above the
  // public section.
  import { Layers, Plus, Search } from '@lucide/svelte';
  import { page } from '$app/state';
  import CollectionCard from '$lib/components/collection-card.svelte';
  import CreateCollectionDialog from '$lib/components/create-collection-dialog.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import PaginationButtons from '$lib/components/pagination-buttons.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { t } from '$lib/i18n.svelte';
  import { visitIndex } from '$lib/paginated-index';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  /** The legacy debounce(500) on the search input. */
  const SEARCH_DEBOUNCE_MS = 500;

  // The search and both section pages live in the URL (the legacy
  // `page_public` and `page` paginators), so the server load answers
  // every change.
  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  let query = $state(data.search);
  let creating = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  const collections = $derived(data.collections.data);

  function onInput() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      void visitIndex(page.url, { search: query, page_public: null, page: null }, { search: true });
    }, SEARCH_DEBOUNCE_MS);
  }
</script>

<PageMeta
  title={t('meta.allCollections.title')}
  description={t('meta.allCollections.description')}
/>

<PageHeader title={t('meta.allCollections.title')} subtitle={t('collections.index.subtitle')}>
  {#snippet icon()}
    <div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
      <Layers class="size-5 text-primary" stroke-width={1.5} />
    </div>
  {/snippet}
  {#snippet actions()}
    <div class="flex items-center gap-3">
      <div class="flex h-10 w-56 items-center gap-2 rounded-md border border-border bg-card-2 px-3">
        <Search class="size-4 shrink-0 text-muted-foreground" />
        <Input
          bind:value={query}
          oninput={onInput}
          placeholder={t('collections.index.searchPlaceholder')}
          class="h-full border-0 bg-transparent p-0 shadow-none focus-visible:ring-0 dark:bg-transparent"
        />
      </div>
      {#if data.nav}
        <Button class="h-10 gap-2" onclick={() => (creating = true)}>
          <Plus class="size-4" />
          {t('collections.index.createCollection')}
        </Button>
      {/if}
    </div>
  {/snippet}
</PageHeader>

{#if data.personal !== null}
  <section class="mb-8">
    <div class="mb-3 flex items-center justify-between gap-4">
      <h2 class="hud-label">{t('collections.index.yourCollections')}</h2>
      <PaginationButtons
        meta={data.personal.meta}
        onPage={(target) => void visitIndex(page.url, { page: target })}
      />
    </div>
    {#if data.personal.data.length > 0}
      <div class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
        {#each data.personal.data as collection (collection.id)}
          <CollectionCard {collection} owned />
        {/each}
      </div>
    {:else}
      <p class="text-sm text-muted-foreground">{t('collections.index.noPersonalCollections')}</p>
    {/if}
  </section>
{/if}

<section>
  <div class="mb-3 flex items-center justify-between gap-4">
    {#if data.personal !== null}
      <h2 class="hud-label">{t('collections.index.publicCollections')}</h2>
    {:else}
      <span></span>
    {/if}
    <PaginationButtons
      meta={data.collections.meta}
      onPage={(target) => void visitIndex(page.url, { page_public: target })}
    />
  </div>
  {#if collections.length > 0}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
      {#each collections as collection (collection.id)}
        <CollectionCard {collection} />
      {/each}
    </div>
  {:else}
    <p class="text-muted-foreground">{t('collections.index.noPublicCollections')}</p>
  {/if}
</section>

<CreateCollectionDialog bind:open={creating} />
