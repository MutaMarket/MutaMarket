<script lang="ts">
  // The legacy PaginationButtons.vue: first, previous, the numbered
  // pages with ellipses, next and last, driven by an index page's meta.
  // Hidden while everything fits on one page.
  import { ChevronsLeft, ChevronsRight } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button';
  import * as Pagination from '$lib/components/ui/pagination';
  import { t } from '$lib/i18n.svelte';
  import type { IndexMeta } from '$lib/types-social';

  let { meta, onPage }: { meta: IndexMeta; onPage: (page: number) => void } = $props();
</script>

{#if meta.total > meta.per_page}
  <Pagination.Root
    count={meta.total}
    perPage={meta.per_page}
    page={meta.current_page}
    onPageChange={onPage}
    class="mx-0 w-auto"
  >
    {#snippet children({ pages, currentPage })}
      <Pagination.Content>
        <Pagination.Item>
          <Button
            variant="ghost"
            size="icon"
            aria-label={t('misc.pagination.goToFirstPage')}
            disabled={currentPage <= 1}
            onclick={() => onPage(1)}
          >
            <ChevronsLeft />
          </Button>
        </Pagination.Item>
        <Pagination.Item>
          <Pagination.Previous />
        </Pagination.Item>
        {#each pages as page (page.key)}
          <Pagination.Item>
            {#if page.type === 'ellipsis'}
              <Pagination.Ellipsis />
            {:else}
              <Pagination.Link {page} isActive={currentPage === page.value}>
                {page.value}
              </Pagination.Link>
            {/if}
          </Pagination.Item>
        {/each}
        <Pagination.Item>
          <Pagination.Next />
        </Pagination.Item>
        <Pagination.Item>
          <Button
            variant="ghost"
            size="icon"
            aria-label={t('misc.pagination.goToLastPage')}
            disabled={currentPage >= meta.last_page}
            onclick={() => onPage(meta.last_page)}
          >
            <ChevronsRight />
          </Button>
        </Pagination.Item>
      </Pagination.Content>
    {/snippet}
  </Pagination.Root>
{/if}
