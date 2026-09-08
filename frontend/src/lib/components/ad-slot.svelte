<script lang="ts">
  // One AdSense ad unit, the legacy Adsense.vue: client-only `<ins>`,
  // pushed to the adsbygoogle queue once it has a width (a unit hidden
  // by a breakpoint is never requested), and re-created on every
  // navigation like the legacy `:key="current_route"`. Fixed sizes and
  // the min-height reserve the box so ads never shift the layout.
  // In development every unit, dormant ones included, draws a labeled
  // placeholder of its box instead (AdSense never serves on localhost).
  import { onMount } from 'svelte';
  import { dev } from '$app/environment';
  import { page } from '$app/state';
  import { AD_SLOTS, ADSENSE_CLIENT_ID, adsVisible } from '$lib/adsense';
  import { t } from '$lib/i18n.svelte';

  let {
    unit,
    format = 'auto',
    width,
    height,
    minHeight = 0,
    fullWidthResponsive = false,
    labeled = false,
    class: className = '',
  }: {
    unit: keyof typeof AD_SLOTS;
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
    <div class={className} data-testid="ad-slot">
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
          data-full-width-responsive={fixed ? undefined : String(fullWidthResponsive)}
        ></ins>
      {/if}
    </div>
  {/key}
{/if}
