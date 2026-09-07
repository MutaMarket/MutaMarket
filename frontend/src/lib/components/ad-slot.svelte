<script lang="ts">
  // One AdSense ad unit, the legacy Adsense.vue: client-only `<ins>`,
  // pushed to the adsbygoogle queue once it has a width (a unit hidden
  // by a breakpoint is never requested), and re-created on every
  // navigation like the legacy `:key="current_route"`. Fixed sizes and
  // the min-height reserve the box so ads never shift the layout.
  import { onMount } from 'svelte';
  import { dev } from '$app/environment';
  import { page } from '$app/state';
  import { ADSENSE_CLIENT_ID, showsAds } from '$lib/adsense';
  import { t } from '$lib/i18n.svelte';

  let {
    slot,
    format = 'auto',
    width,
    height,
    minHeight = 0,
    fullWidthResponsive = false,
    labeled = false,
    class: className = '',
  }: {
    slot: string;
    /** AdSense `data-ad-format`; ignored for a fixed width/height. */
    format?: 'auto' | 'horizontal' | 'rectangle' | 'vertical';
    width?: number;
    height?: number;
    /** Reserved height of a responsive unit before the ad renders. */
    minHeight?: number;
    fullWidthResponsive?: boolean;
    /** Shows the "Advertisement" label (units sitting among content). */
    labeled?: boolean;
    class?: string;
  } = $props();

  let mounted = $state(false);
  let ins = $state<HTMLElement | null>(null);

  const fixed = $derived(width !== undefined && height !== undefined);
  const enabled = $derived(slot !== '' && showsAds(page.data.nav ?? null, ADSENSE_CLIENT_ID));
  const style = $derived(
    fixed
      ? `display:inline-block;width:${width}px;height:${height}px`
      : `display:block;width:100%;min-height:${minHeight}px`,
  );

  onMount(() => {
    mounted = true;
  });

  $effect(() => {
    const element = ins;
    if (element === null || element.getBoundingClientRect().width === 0) {
      return;
    }
    (window.adsbygoogle = window.adsbygoogle || []).push({});
  });
</script>

{#if enabled && mounted}
  {#key page.url.pathname}
    <div
      class="{className} {dev ? 'outline outline-1 outline-negative' : ''}"
      data-testid="ad-slot"
    >
      {#if labeled}
        <span class="mb-1 block text-2xs uppercase text-muted-foreground">
          {t('premium.ads.advertisement')}
        </span>
      {/if}
      <ins
        bind:this={ins}
        class="adsbygoogle"
        {style}
        data-ad-client={ADSENSE_CLIENT_ID}
        data-ad-slot={slot}
        data-ad-format={fixed ? undefined : format}
        data-full-width-responsive={fixed ? undefined : String(fullWidthResponsive)}
      ></ins>
    </div>
  {/key}
{/if}
