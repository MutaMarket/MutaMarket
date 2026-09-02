<script lang="ts">
  // The top-characters tab: the creator leaderboard in one HUD frame,
  // zero-padded mono ranks with lime chips on the podium, filters and
  // paging in the toolbar row.
  import { ArrowUpDown } from '@lucide/svelte';
  import { goto } from '$app/navigation';
  import GameImage from '$lib/components/game-image.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import StatisticsTabs from '$lib/components/statistics-tabs.svelte';
  import TypeDialog from '$lib/components/type-dialog.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as Table from '$lib/components/ui/table';
  import { abyssalBySlug } from '$lib/abyssals';
  import { buildQueryPath, moduleSlug, parseQueryUi } from '$lib/query';
  import { pageCount, type TopCharacterRow } from '$lib/statistics';
  import { t } from '$lib/i18n.svelte';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const search = $derived(parseQueryUi(data.query));
  // The static abyssals list resolves the URL segment back to the
  // selected type so the trigger shows it without a server round-trip.
  const currentType = $derived(abyssalBySlug(search.typeSlug));

  /** The legacy search debounce before navigating. */
  const SEARCH_DEBOUNCE_MS = 500;

  // The name filter follows the URL; typing debounces into a
  // navigation so results stay shareable.
  // svelte-ignore state_referenced_locally -- seeded, then synced below
  let nameInput = $state(data.name);
  $effect(() => {
    nameInput = data.name;
  });
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  function navigate(next: {
    page?: number;
    name?: string;
    sortField?: string | null;
    sortDirection?: string | null;
  }) {
    const path = buildQueryPath('statistics/characters', { ...search, page: next.page ?? 1 });
    const params = new URLSearchParams();
    const name = next.name ?? nameInput;
    if (name.trim() !== '') {
      params.set('name', name.trim());
    }
    const sortField = next.sortField === undefined ? data.sortField : next.sortField;
    const sortDirection =
      next.sortDirection === undefined ? data.sortDirection : next.sortDirection;
    if (sortField) {
      params.set('sort_field', sortField);
      params.set('sort_direction', sortDirection ?? 'asc');
    }
    const suffix = params.size > 0 ? `?${params}` : '';
    goto(`${path}${suffix}`, { keepFocus: true, noScroll: true });
  }

  function onNameInput() {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => navigate({ name: nameInput }), SEARCH_DEBOUNCE_MS);
  }

  // The legacy SortableHeader: same field flips the direction, a new
  // field starts descending.
  function sortBy(field: 'name' | 'rank_number') {
    if (data.sortField === field) {
      navigate({
        sortField: field,
        sortDirection: data.sortDirection === 'asc' ? 'desc' : 'asc',
        page: data.top.meta.current_page,
      });
    } else {
      navigate({ sortField: field, sortDirection: 'desc', page: data.top.meta.current_page });
    }
  }

  const totalPages = $derived(pageCount(data.top.meta.total, data.top.meta.per_page));

  function characterHref(row: TopCharacterRow): string {
    return `/characters/${moduleSlug(row.name, row.id)}`;
  }

  /** The legacy leaderboard count link: that character's created
   * modules, keeping the page's type scope. */
  function createdModulesHref(row: TopCharacterRow): string {
    const base = `characters/${moduleSlug(row.name, row.id)}`;
    return buildQueryPath(base, {
      ...parseQueryUi(''),
      typeSlug: search.typeSlug,
      created: true,
    });
  }

  const rank = (value: number) => String(value).padStart(3, '0');

  const SORT_BUTTON =
    'inline-flex items-center gap-1.5 whitespace-nowrap transition-colors hover:text-foreground';
</script>

<PageMeta title={t('meta.allStatistics.title')} description={t('meta.allStatistics.description')} />

<PageHeader title={t('nav.menu.statistics')} subtitle={t('stats.header.subtitle')} />
<StatisticsTabs />

<div class="hud-frame">
  <div class="grid items-end gap-4 border-b border-border p-4 md:grid-cols-3">
    <div>
      <h3 class="hud-label mb-2">{t('misc.typeDialog.category')}</h3>
      <TypeDialog
        prefix="statistics/characters"
        {search}
        currentTypeId={currentType?.id ?? null}
        currentTypeName={currentType?.name ?? null}
      />
    </div>
    <label class="grid gap-2">
      <span class="hud-label">{t('stats.allStatistics.searchByName')}</span>
      <Input
        type="search"
        placeholder={t('stats.allStatistics.searchPlaceholder')}
        class="h-10 bg-card-2 dark:bg-card-2"
        bind:value={nameInput}
        oninput={onNameInput}
      />
    </label>
    <div class="flex items-center justify-end gap-2 pb-1">
      <Button
        variant="outline"
        size="sm"
        disabled={data.top.meta.current_page <= 1}
        onclick={() => navigate({ page: data.top.meta.current_page - 1 })}
      >
        {t('common.actions.previous')}
      </Button>
      <span class="font-mono text-xs whitespace-nowrap text-muted-foreground tabular-nums">
        {data.top.meta.current_page} / {totalPages}
      </span>
      <Button
        variant="outline"
        size="sm"
        disabled={data.top.meta.current_page >= totalPages}
        onclick={() => navigate({ page: data.top.meta.current_page + 1 })}
      >
        {t('common.actions.next')}
      </Button>
    </div>
  </div>
  <Table.Root>
    <Table.Header>
      <Table.Row>
        <Table.Head class="w-20">
          <button type="button" class={SORT_BUTTON} onclick={() => sortBy('rank_number')}>
            {t('stats.allStatistics.columns.rank')}
            <ArrowUpDown class="size-3.5 opacity-60" />
          </button>
        </Table.Head>
        <Table.Head>
          <button type="button" class={SORT_BUTTON} onclick={() => sortBy('name')}>
            {t('stats.allStatistics.columns.character')}
            <ArrowUpDown class="size-3.5 opacity-60" />
          </button>
        </Table.Head>
        <Table.Head class="text-right">{t('stats.allStatistics.columns.modulesCreated')}</Table.Head
        >
      </Table.Row>
    </Table.Header>
    <Table.Body>
      {#each data.top.data as row (row.id)}
        <Table.Row>
          <Table.Cell>
            <span
              class="inline-flex min-w-12 justify-center rounded-[4px] border px-1.5 py-0.5 font-mono text-xs tabular-nums
								{row.rank_number <= 3
                ? 'border-primary/60 bg-primary/10 text-primary'
                : 'border-border text-muted-foreground'}"
            >
              {rank(row.rank_number)}
            </span>
          </Table.Cell>
          <Table.Cell>
            <a href={characterHref(row)} class="flex items-center gap-2 hover:underline">
              <GameImage
                src="https://images.evetech.net/characters/{row.id}/portrait?size=64"
                alt={row.name}
                class="size-8 rounded-md"
              />
              <span class="whitespace-nowrap">{row.name}</span>
            </a>
          </Table.Cell>
          <Table.Cell class="text-right">
            <a
              href={createdModulesHref(row)}
              class="rounded-md px-2 py-1 font-mono tabular-nums hover:bg-card-2"
            >
              {row.modules_created_count.toLocaleString('en-US')}
            </a>
          </Table.Cell>
        </Table.Row>
      {:else}
        <Table.Row>
          <Table.Cell colspan={3} class="py-8 text-center text-muted-foreground">
            {t('forms.baseTable.noResults')}
          </Table.Cell>
        </Table.Row>
      {/each}
    </Table.Body>
  </Table.Root>
</div>
