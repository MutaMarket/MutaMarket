<script lang="ts">
  // The documentation pages, the legacy ShowDocumentationPage.vue:
  // sticky section sidebar, HUD-panel content frame with the section
  // label, GitHub edit link, rendered markdown article, and
  // previous/next footer links. The mobile page picker is a native
  // select.
  import { goto } from '$app/navigation';
  import type { PageProps } from './$types';
  import DocsNav from '$lib/components/docs-nav.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { t } from '$lib/i18n.svelte';

  let { data }: PageProps = $props();

  const doc = $derived(data.doc);
</script>

<PageMeta
  title={doc.title}
  description={t('meta.documentation.description', { title: doc.title })}
/>

<div class="lg:grid lg:grid-cols-[240px_minmax(0,1fr)] lg:gap-6">
  <DocsNav sections={doc.sections} current={doc.slug} />

  <div class="hud-frame min-w-0">
    <div class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-6 py-4">
      <div>
        <span class="hud-label">{t('docs.show.breadcrumb', { section: doc.section })}</span>
        <h1 class="mt-1 text-2xl font-bold">{doc.title}</h1>
      </div>
      <a
        href={doc.edit_url}
        class="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
        rel="noopener noreferrer"
        target="_blank"
      >
        {t('docs.show.editOnGithub')}
      </a>
    </div>

    <div class="border-b border-border px-6 py-3 lg:hidden">
      <select
        class="w-full border border-border bg-background px-3 py-2 text-sm"
        aria-label={t('docs.show.selectSection')}
        onchange={(event) => {
          const value = event.currentTarget.value;
          if (value) goto(`/documentation/${value}`);
        }}
      >
        {#each doc.sections as section (section.title)}
          <optgroup label={section.title}>
            {#each section.pages as entry (entry.slug)}
              <option value={entry.slug} selected={entry.slug === doc.slug}>
                {entry.title}
              </option>
            {/each}
          </optgroup>
        {/each}
      </select>
    </div>

    <!-- eslint-disable-next-line svelte/no-at-html-tags -- server-rendered
		     markdown, sanitized by the API's hardened renderer -->
    <article class="docs-prose px-6 py-6 md:px-8">{@html doc.html}</article>

    <div class="grid grid-cols-2 border-t border-border">
      {#if doc.previous}
        <a
          href="/documentation/{doc.previous.slug}"
          class="group flex flex-col gap-1 p-4 transition-colors hover:bg-secondary/40"
        >
          <span class="hud-label inline-flex items-center gap-1.5">
            {`← ${t('common.actions.previous')}`}
          </span>
          <span class="text-sm font-medium transition-colors group-hover:text-primary">
            {doc.previous.title}
          </span>
        </a>
      {:else}
        <div></div>
      {/if}
      {#if doc.next}
        <a
          href="/documentation/{doc.next.slug}"
          class="group flex flex-col items-end gap-1 border-l border-border p-4 text-right transition-colors hover:bg-secondary/40"
        >
          <span class="hud-label inline-flex items-center gap-1.5">
            {`${t('common.actions.next')} →`}
          </span>
          <span class="text-sm font-medium transition-colors group-hover:text-primary">
            {doc.next.title}
          </span>
        </a>
      {/if}
    </div>
  </div>
</div>
