<script lang="ts">
  // The inline banner, the legacy InlineAd.vue: one fixed-size unit per
  // breakpoint, centered; the widest one yields to the rails at 3xl.
  import { dev } from '$app/environment';
  import { page } from '$app/state';
  import AdSlot from './ad-slot.svelte';
  import { AD_MEDIA, adsVisible, type AdStatus } from '$lib/adsense';

  // Only the unit of the current width mounts, so one status suffices;
  // the strip's margins exist only once that unit is filled.
  let status = $state<AdStatus>('pending');
</script>

{#if adsVisible(page.data.nav, dev)}
  <div
    class="flex items-center justify-center {status === 'filled' ? 'my-2' : ''}"
    data-testid="ad-banner"
  >
    <AdSlot
      unit="bannerWide"
      width={970}
      height={90}
      media={AD_MEDIA.wide}
      onstatus={(next) => (status = next)}
    />
    <AdSlot
      unit="bannerMedium"
      width={728}
      height={90}
      media={AD_MEDIA.medium}
      onstatus={(next) => (status = next)}
    />
    <AdSlot
      unit="bannerMobile"
      width={300}
      height={100}
      media={AD_MEDIA.mobile}
      onstatus={(next) => (status = next)}
    />
  </div>
{/if}
