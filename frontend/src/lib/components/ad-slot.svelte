<script lang="ts">
  // One AdSense ad unit, the legacy Adsense.vue: client-only `<ins>`,
  // pushed to the adsbygoogle queue once it has a width (a unit hidden
  // by a breakpoint is never requested), and re-created when the page,
  // module type or page number changes (see adRouteKey). Fixed sizes and
  // the min-height reserve the box so ads never shift the layout; a unit
  // AdSense reports unfilled collapses.
  // In development every unit, dormant ones included, draws a labeled
  // placeholder of its box instead (AdSense never serves on localhost).
  import { onMount } from 'svelte';
  import { dev } from '$app/environment';
  import { page } from '$app/state';
  import { AD_SLOTS, ADSENSE_CLIENT_ID, adRouteKey, adsVisible, type AdStatus } from '$lib/adsense';
  import { t } from '$lib/i18n.svelte';

  let {
    unit,
    format = 'auto',
    width,
    height,
    minHeight = 0,
    layoutKey,
    fullWidthResponsive = false,
    labeled = false,
    class: className = '',
    onstatus,
  }: {
    unit: keyof typeof AD_SLOTS;
    /** AdSense `data-ad-format`; ignored for a fixed width/height. */
    format?: 'auto' | 'horizontal' | 'rectangle' | 'vertical' | 'fluid';
    width?: number;
    height?: number;
    /** Reserved height of a responsive unit before the ad renders. */
    minHeight?: number;
    /** AdSense `data-ad-layout-key` of a fluid (in-feed) unit. */
    layoutKey?: string;
    fullWidthResponsive?: boolean;
    /** Shows the "Advertisement" label (units sitting among content). */
    labeled?: boolean;
    class?: string;
    /** Reports the unit's fill status as AdSense answers (a placeholder
     * counts as filled), so a container can reveal or drop its cell. */
    onstatus?: (status: AdStatus) => void;
  } = $props();

  let mounted = $state(false);
  let ins = $state<HTMLElement | null>(null);
  let status = $state<AdStatus>('pending');

  const slot = $derived(AD_SLOTS[unit]);
  const fixed = $derived(width !== undefined && height !== undefined);
  const enabled = $derived((dev || slot !== '') && adsVisible(page.data.nav, dev));
  const size = $derived(fixed ? `${width}x${height}` : `responsive, min ${minHeight}px`);
  const style = $derived(
    fixed
      ? `display:inline-block;width:${width}px;height:${height}px`
      : `display:block;width:100%;min-height:${minHeight}px`,
  );

  onMount(() => {
    mounted = true;
  });

  function report(next: AdStatus) {
    status = next;
    onstatus?.(next);
  }

  $effect(() => {
    if (dev && mounted) {
      report('filled');
    }
  });

  $effect(() => {
    const element = ins;
    if (element === null) {
      return;
    }
    report('pending');
    const observer = new MutationObserver(() => {
      const value = element.dataset.adStatus;
      if (value === 'filled' || value === 'unfilled') {
        report(value);
      }
    });
    observer.observe(element, { attributes: true, attributeFilter: ['data-ad-status'] });
    return () => observer.disconnect();
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
  {#key adRouteKey(page.url.pathname)}
    <div class="{className} {status === 'unfilled' ? 'hidden' : ''}" data-testid="ad-slot">
      {#if labeled}
        <span class="mb-1 block text-2xs uppercase text-muted-foreground">
          {t('premium.ads.advertisement')}
        </span>
      {/if}
      {#if dev}
        <div
          {style}
          class="grid place-items-center border border-dashed border-negative bg-negative/5 text-center font-mono text-xs text-negative"
          data-testid="ad-placeholder"
        >
          <span>{unit}<br />{size}<br />{slot === '' ? 'no unit id' : `slot ${slot}`}</span>
        </div>
      {:else}
        <ins
          bind:this={ins}
          class="adsbygoogle"
          {style}
          data-ad-client={ADSENSE_CLIENT_ID}
          data-ad-slot={slot}
          data-ad-format={fixed ? undefined : format}
          data-ad-layout-key={layoutKey}
          data-full-width-responsive={fixed ? undefined : String(fullWidthResponsive)}
        ></ins>
      {/if}
    </div>
  {/key}
{/if}
