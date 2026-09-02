<script lang="ts">
  // The personal statistics tab: the three legacy headline totals as
  // one HUD numeral row, the searchable creation table beneath.
  import { ArrowUpDown, LogIn } from '@lucide/svelte';
  import { page } from '$app/state';
  import GameImage from '$lib/components/game-image.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import StatisticsTabs from '$lib/components/statistics-tabs.svelte';
  import { Input } from '$lib/components/ui/input';
  import * as Table from '$lib/components/ui/table';
  import { toIskCompact } from '$lib/format-number';
  import { moduleSlug } from '$lib/query';
  import { filterPersonalRows, sortPersonalRows, type PersonalSortKey } from '$lib/statistics';
  import { t } from '$lib/i18n.svelte';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const signedIn = $derived(Boolean(page.data.nav?.user));

  // Search and sort run client-side, the legacy BaseTable behavior.
  let needle = $state('');
  let sortKey = $state<PersonalSortKey>('count');
  let ascending = $state(false);
  const rows = $derived(
    data.personal === null
      ? []
      : sortPersonalRows(filterPersonalRows(data.personal.stats, needle), sortKey, ascending),
  );
  function sortBy(key: PersonalSortKey) {
    if (sortKey === key) {
      ascending = !ascending;
    } else {
      sortKey = key;
      ascending = key !== 'count';
    }
  }

  const COLUMNS: { key: PersonalSortKey; label: string }[] = $derived([
    { key: 'type', label: t('common.labels.type') },
    { key: 'creator', label: t('stats.columns.creator') },
    { key: 'count', label: t('stats.columns.count') },
  ]);

  const headline = $derived(
    data.personal === null
      ? []
      : [
          {
            label: t('stats.personal.modulesCreated'),
            value: data.personal.total_modules.toLocaleString('en-US'),
            detail: t('stats.personal.modulesCreatedDescription'),
            accent: true,
          },
          {
            label: t('stats.personal.moneySpent'),
            value: toIskCompact(data.personal.total_spent),
            detail: t('stats.personal.moneySpentDescription'),
            accent: false,
          },
          {
            label: t('stats.personal.totalValue'),
            value: toIskCompact(data.personal.total_value),
            detail: t('stats.personal.totalValueDescription'),
            accent: false,
          },
        ],
  );
</script>

<PageMeta title={t('meta.personalStats.title')} description={t('meta.personalStats.description')} />

<PageHeader title={t('nav.menu.statistics')} subtitle={t('stats.header.subtitle')} />
<StatisticsTabs />

{#if data.personal !== null}
  <div class="hud-frame divide-y divide-border">
    <div class="grid divide-y divide-border md:grid-cols-3 md:divide-x md:divide-y-0">
      {#each headline as stat (stat.label)}
        <div class="p-6">
          <h2 class="hud-label">{stat.label}</h2>
          <div
            class="mt-2 font-mono text-3xl {stat.accent
              ? 'text-primary [text-shadow:0_0_24px_color-mix(in_srgb,var(--color-primary)_35%,transparent)]'
              : ''}"
          >
            {stat.value}
          </div>
          <p class="mt-2 text-sm text-muted-foreground">{stat.detail}</p>
        </div>
      {/each}
    </div>
    <div class="grid items-center gap-3 p-4 md:grid-cols-3">
      <label class="grid gap-1">
        <span class="hud-label">{t('stats.personal.searchLabel')}</span>
        <Input
          type="search"
          placeholder={t('stats.personal.searchPlaceholder')}
          class="h-10 bg-card-2 dark:bg-card-2"
          bind:value={needle}
        />
      </label>
      <p class="text-center text-sm text-balance text-muted-foreground italic md:col-start-3">
        {t('stats.personal.disclaimer')}
      </p>
    </div>
    <Table.Root>
      <Table.Header>
        <Table.Row>
          {#each COLUMNS as column (column.key)}
            <Table.Head class={column.key === 'count' ? 'text-right' : ''}>
              <button
                type="button"
                class="inline-flex items-center gap-1.5 whitespace-nowrap transition-colors hover:text-foreground {sortKey ===
                column.key
                  ? 'text-foreground'
                  : ''}"
                onclick={() => sortBy(column.key)}
              >
                {column.label}
                <ArrowUpDown class="size-3.5 opacity-60" />
              </button>
            </Table.Head>
          {/each}
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each rows as row (`${row.type.id}-${row.creator.id}`)}
          <Table.Row>
            <Table.Cell>
              <div class="flex items-center gap-2">
                <GameImage
                  src="https://images.evetech.net/types/{row.type.id}/icon?size=32"
                  alt={row.type.name}
                  class="size-8 rounded-md"
                />
                <span class="whitespace-nowrap">{row.type.name}</span>
              </div>
            </Table.Cell>
            <Table.Cell>
              <a
                href="/characters/{moduleSlug(row.creator.name, row.creator.id)}"
                class="flex items-center gap-2 hover:underline"
              >
                <GameImage
                  src="https://images.evetech.net/characters/{row.creator.id}/portrait?size=64"
                  alt={row.creator.name}
                  class="size-8 rounded-md"
                />
                <span class="whitespace-nowrap">{row.creator.name}</span>
              </a>
            </Table.Cell>
            <Table.Cell class="text-right font-mono tabular-nums">
              {row.count.toLocaleString('en-US')}
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
{:else}
  <div class="hud-frame flex items-center justify-center gap-4 p-10">
    <LogIn class="size-6 text-muted-foreground" />
    <span class="text-muted-foreground">
      {signedIn ? t('stats.personal.loadFailed') : t('stats.personal.signInPrompt')}
    </span>
  </div>
{/if}
