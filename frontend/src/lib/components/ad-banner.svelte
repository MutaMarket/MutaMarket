<script lang="ts">
  // The inline banner, the legacy InlineAd.vue: one fixed-size unit per
  // breakpoint, centered; the widest one yields to the rails at 3xl.
  import { dev } from '$app/environment';
  import { page } from '$app/state';
  import AdSlot from './ad-slot.svelte';
  import { adsVisible } from '$lib/adsense';
</script>

{#if adsVisible(page.data.nav, dev)}
  <div class="my-2 flex items-center justify-center" data-testid="ad-banner">
    <AdSlot unit="bannerWide" width={970} height={90} class="hidden lg:block 3xl:hidden" />
    <AdSlot unit="bannerMedium" width={728} height={90} class="hidden md:block lg:hidden" />
    <AdSlot unit="bannerMobile" width={300} height={100} class="md:hidden" />
  </div>
{/if}
