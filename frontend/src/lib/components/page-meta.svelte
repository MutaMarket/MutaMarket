<script lang="ts">
  // The legacy Components/Meta/Headers.vue. Every page renders exactly
  // one of these; it owns the document title so no page emits two.
  import { page } from '$app/state';
  import { buildMetaTags, type MetaImage } from '$lib/meta';

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
  const tags = $derived(
    buildMetaTags({
      origin: page.url.origin,
      path: url ?? page.url.pathname,
      title,
      description,
      image,
      keywords,
    }),
  );
</script>

<svelte:head>
  <title>{title}</title>
  {#each tags as tag (tag.key)}
    {#if tag.attr === 'property'}
      <meta property={tag.key} content={tag.content} />
    {:else}
      <meta name={tag.key} content={tag.content} />
    {/if}
  {/each}
</svelte:head>
