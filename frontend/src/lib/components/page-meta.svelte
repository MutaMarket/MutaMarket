<script lang="ts">
  // The legacy Components/Meta/Headers.vue. Every page renders exactly
  // one of these; it owns the document title so no page emits two.
  import { page } from '$app/state';
  import { buildMetaTags, documentTitle, type MetaImage } from '$lib/meta';

  let {
    title,
    description,
    image,
    keywords,
    url,
  }: {
    title: string;
    description: string;
    image?: MetaImage;
    keywords?: string | string[];
    /** Overrides the canonical path; defaults to the current route. */
    url?: string;
  } = $props();

  // page.url carries the real request origin through SSR (adapter-node
  // reads ORIGIN in production), so scrapers get absolute URLs without a
  // hardcoded host.
  // The legacy title strings carried the site name themselves, so the
  // social cards read the same suffixed title.
  const fullTitle = $derived(documentTitle(title));
  const tags = $derived(
    buildMetaTags({
      origin: page.url.origin,
      path: url ?? page.url.pathname,
      title: fullTitle,
      description,
      image,
      keywords,
    }),
  );
</script>

<svelte:head>
  <title>{fullTitle}</title>
  {#each tags as tag (tag.key)}
    {#if tag.attr === 'property'}
      <meta property={tag.key} content={tag.content} />
    {:else}
      <meta name={tag.key} content={tag.content} />
    {/if}
  {/each}
</svelte:head>
