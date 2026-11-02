<!-- The hover card behind an owned module's location row: the legacy
     FindAssetTooltip, redesigned so the position reads at a glance. The
     owner and the container path sit up top, the in-game steps follow:
     open the container and paste the module's item ID into the
     inventory search, which leaves only this module. -->
<script lang="ts">
  import { ChevronRight, Copy } from '@lucide/svelte';
  import { copyItemId } from '#lib/export.ts';
  import GameImage from './game-image.svelte';
  import Trans from './trans.svelte';
  import * as HoverCard from '#lib/components/ui/hover-card/index.ts';
  import { t } from '#lib/i18n.svelte.ts';
  import { locationFlagLabel } from '#lib/location-flags.ts';
  import type { AssetLocationView, ModuleDetail } from '#lib/types.ts';

  let { module, asset }: { module: ModuleDetail; asset: AssetLocationView } = $props();

  const inHangar = $derived(asset.station !== null && asset.station.slug === asset.parent_slug);

</script>

<HoverCard.Content class="w-80 border p-0" side="top" align="start">
  <div class="flex items-center gap-3 border-b border-border p-3">
    <img
      alt=""
      class="size-10 rounded-md"
      src="https://images.evetech.net/characters/{asset.owner.id}/portrait?size=64"
    />
    <div class="min-w-0">
      <a class="block truncate font-semibold hover:underline" href="/characters/{asset.owner.slug}">
        {asset.owner.name}
      </a>
      <span class="block truncate text-xs text-muted-foreground">
        {#if asset.station}
          <a class="hover:text-foreground hover:underline" href="/locations/{asset.station.slug}">
            {asset.station.name}
          </a>
        {:else}
          {t('modules.findAsset.unknownStation')}
        {/if}
      </span>
    </div>
  </div>

  <div class="flex items-center gap-1.5 border-b border-border px-3 py-2 text-xs">
    {#if asset.parent_type_id !== null}
      <GameImage
        src="https://images.evetech.net/types/{asset.parent_type_id}/icon?size=64"
        alt={asset.parent_name}
        class="size-5 rounded"
      />
    {/if}
    {#if !inHangar && asset.station}
      <span class="truncate text-muted-foreground">{asset.station.name}</span>
      <ChevronRight class="size-3 shrink-0 text-muted-foreground" />
    {/if}
    <a class="truncate font-medium hover:underline" href="/locations/{asset.parent_slug}">
      {asset.parent_name}
    </a>
    <span class="ml-auto shrink-0 bg-primary/10 px-1.5 py-0.5 text-[0.65rem] text-primary">
      {locationFlagLabel(asset.location_flag)}
    </span>
  </div>

  <div class="p-3">
    <h3 class="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
      {t('modules.findAsset.howToFind')}
    </h3>
    <ol class="mt-2 grid list-decimal gap-1.5 pl-4 text-sm">
      <li>
        <Trans key="modules.findAsset.openContainer">
          {#snippet container()}<b>{asset.parent_name}</b>{/snippet}
        </Trans>
      </li>
      <li>
        <Trans key="modules.findAsset.pasteItemId">
          {#snippet id()}
            <button
              type="button"
              class="inline-flex items-center gap-1 font-mono font-semibold hover:underline"
              onclick={() => copyItemId(module)}
            >
              {module.id}
              <Copy class="size-3 text-muted-foreground" />
            </button>
          {/snippet}
        </Trans>
      </li>
    </ol>
    <p class="mt-2 text-xs text-muted-foreground">{t('modules.findAsset.onlyThisModule')}</p>
  </div>
</HoverCard.Content>
