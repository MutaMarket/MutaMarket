<script lang="ts">
  // The legacy Tables/Contracts/CharacterDetails.vue: linked portrait
  // and name.
  import GameImage from './game-image.svelte';
  import { t } from '$lib/i18n.svelte';
  import type { CharacterRef } from '$lib/types';

  let { character }: { character: CharacterRef | null } = $props();
</script>

{#if character}
  <a href="/characters/{character.slug}" class="grid grid-cols-[2rem_1fr] items-center gap-2">
    <GameImage
      src="https://images.evetech.net/characters/{character.id}/portrait?size=64"
      alt={character.name}
      class="size-8 rounded-md"
    />
    <span>{character.name || t('contracts.table.unknownCharacter')}</span>
  </a>
{:else}
  <span class="text-muted-foreground">{t('contracts.table.unknownCharacter')}</span>
{/if}
