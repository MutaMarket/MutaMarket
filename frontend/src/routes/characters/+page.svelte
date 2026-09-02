<script lang="ts">
  // The characters index, the legacy ShowAllCharactersPage: heading and
  // intro with the sell-page link, debounced name search, and the card
  // grid (premium members first, from the API ordering).
  import { Search, Users } from '@lucide/svelte';
  import { page } from '$app/state';
  import CharacterCard from '$lib/components/character-card.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import PaginationButtons from '$lib/components/pagination-buttons.svelte';
  import Trans from '$lib/components/trans.svelte';
  import { Input } from '$lib/components/ui/input';
  import { t } from '$lib/i18n.svelte';
  import { visitIndex } from '$lib/paginated-index';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  /** The legacy debounce(200) on the search input. */
  const SEARCH_DEBOUNCE_MS = 200;

  // The search and the page live in the URL (the legacy paginator query
  // strings), so the server load answers every change.
  // svelte-ignore state_referenced_locally -- deliberate one-time seed
  let query = $state(data.search);
  let timer: ReturnType<typeof setTimeout> | undefined;

  const characters = $derived(data.characters.data);

  function onInput() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      void visitIndex(page.url, { search: query, page: null }, { search: true });
    }, SEARCH_DEBOUNCE_MS);
  }
</script>

<PageMeta title={t('meta.allCharacters.title')} description={t('meta.allCharacters.description')} />

<PageHeader title={t('characters.index.heading')} subtitle={t('characters.index.subtitle')}>
  {#snippet icon()}
    <div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
      <Users class="size-5 text-primary" stroke-width={1.5} />
    </div>
  {/snippet}
  {#snippet actions()}
    <div class="flex h-10 w-64 items-center gap-2 rounded-md border border-border bg-card-2 px-3">
      <Search class="size-4 shrink-0 text-muted-foreground" />
      <Input
        bind:value={query}
        oninput={onInput}
        placeholder={t('characters.index.searchPlaceholder')}
        class="h-full border-0 bg-transparent p-0 shadow-none focus-visible:ring-0 dark:bg-transparent"
      />
    </div>
  {/snippet}
</PageHeader>

<div class="mb-4 flex justify-end">
  <PaginationButtons
    meta={data.characters.meta}
    onPage={(target) => void visitIndex(page.url, { page: target })}
  />
</div>

{#if characters.length > 0}
  <div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4">
    {#each characters as character (character.id)}
      <CharacterCard {character} />
    {/each}
  </div>
{:else}
  <div class="hud-frame flex items-center gap-4 p-6">
    <Users class="size-8 shrink-0 text-primary" stroke-width={1.5} />
    <div>
      <span class="block text-lg font-medium">{t('characters.noneFound.heading')}</span>
      <p class="text-sm text-muted-foreground">
        <Trans key="characters.index.intro">
          {#snippet sellLink()}<a href="/sell/modules" class="text-primary hover:underline"
              >{t('characters.index.sellPage')}</a
            >{/snippet}
        </Trans>
      </p>
    </div>
  </div>
{/if}
