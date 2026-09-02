<!-- A message whose placeholders render markup: the legacy <i18n-t>.
     Pass a snippet named after a placeholder to render it there; any
     other placeholder falls back to its param value.
     <Trans key="modules.findAsset.belongsTo">
       {#snippet character()}<a href=...>...</a>{/snippet}
     </Trans> -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { type Params, segments } from '$lib/i18n.svelte';

  let {
    key,
    params = {},
    ...slots
  }: {
    key: string;
    params?: Params;
    [slot: string]: Snippet | Params | string | undefined;
  } = $props();

  const parts = $derived(segments(key, params));
</script>

{#each parts as part, index (index)}
  {#if part.slot !== undefined}
    {@const snippet = slots[part.slot]}
    {#if typeof snippet === 'function'}
      {@render snippet()}
    {:else if part.slot in params}
      {params[part.slot]}
    {:else}
      {`{${part.slot}}`}
    {/if}
  {:else}
    {part.text}
  {/if}
{/each}
