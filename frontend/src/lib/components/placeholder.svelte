<script lang="ts">
  // Placeholder for pages whose feature milestone has not landed yet in
  // the rewrite: the route exists and renders, the content is on its way.
  import { t } from '$lib/i18n.svelte';
  import { documentTitle } from '$lib/meta';

  let { title }: { title: string } = $props();
</script>

<svelte:head><title>{documentTitle(title)}</title></svelte:head>

<section class="placeholder">
  <h1>{title}</h1>
  <p>{t('misc.placeholder.body')}</p>
</section>
