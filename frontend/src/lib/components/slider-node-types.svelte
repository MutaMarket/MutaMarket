<script lang="ts">
  // The value at a slider node and the source types sitting on it. Shared
  // by the pip hover popover and the drag tooltip so both read the same.
  import { metaGroupDotClass } from '$lib/filter-meta';

  interface NodeType {
    id: number;
    name: string;
    meta_group_id: number | null;
  }

  let { value, types = [] }: { value: string; types?: NodeType[] } = $props();
</script>

<div class="rounded-lg border border-primary bg-popover p-2 text-sm text-foreground shadow-lg">
  <div class="text-center leading-none font-medium tabular-nums">{value}</div>
  {#if types.length > 0}
    <div class="mt-1.5 space-y-0.5 border-t border-white/10 pt-1.5">
      {#each types as type (type.id)}
        <div class="flex items-center gap-1.5 text-xs whitespace-nowrap">
          <span class="size-1.5 shrink-0 rounded-full {metaGroupDotClass(type.meta_group_id)}"
          ></span>
          {type.name}
        </div>
      {/each}
    </div>
  {/if}
</div>
