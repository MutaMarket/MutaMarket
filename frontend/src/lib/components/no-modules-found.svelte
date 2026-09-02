<script lang="ts">
  // The empty state of every module display, the legacy
  // NoModulesFound.vue card.
  import { TriangleAlert } from '@lucide/svelte';
  import { t } from '$lib/i18n.svelte';
</script>

<div class="hud-frame col-span-full">
  <div class="flex items-center gap-4 p-6">
    <TriangleAlert class="size-8 shrink-0 text-primary" />
    <div>
      <span class="block text-lg font-medium">{t('modules.empty.title')}</span>
      <p>{t('modules.empty.hint')}</p>
    </div>
  </div>
</div>
