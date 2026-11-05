<script lang="ts">
  // The EVE referral banner, a self-contained ad creative in the
  // style of CCP's own store ads rather than app chrome: EVE key art
  // with its dark side under a condensed white headline, an orange
  // accent line and a flat button in a thin frame that a light sweep
  // crosses now and then. A compact strip below the rotation ads,
  // pitched at players looking for an alt; the whole card is the
  // referral link.
  import { t } from '#lib/i18n.svelte.ts';

  let { href }: { href: string } = $props();
</script>

<a
  {href}
  target="_blank"
  rel="sponsored noopener"
  class="group relative isolate block aspect-[250/115] w-full overflow-hidden rounded-lg bg-[#050a16] text-white"
  data-testid="eve-referral"
>
  <img
    src="/img/eve-referral.webp"
    alt=""
    loading="lazy"
    class="absolute inset-0 -z-20 size-full object-cover object-[100%_40%] transition-transform duration-700 group-hover:scale-105"
  />
  <!-- Deepen the art's dark side under the copy. -->
  <div
    class="absolute inset-0 -z-10 bg-gradient-to-r from-[#050a16]/90 from-25% via-[#050a16]/45 via-50% to-transparent to-70%"
  ></div>

  <div class="absolute top-1/2 left-3.5 flex -translate-y-1/2 flex-col">
    <span
      class="text-[9px] font-semibold tracking-[0.18em] text-white/85 uppercase [font-stretch:85%]"
    >
      {t('misc.referral.title')}
    </span>
    <span
      class="text-[30px] leading-none font-extrabold tracking-tight [font-stretch:75%] [text-shadow:0_2px_8px_rgb(0_0_0/0.6)]"
    >
      {t('misc.referral.amount')}
    </span>
    <span class="text-sm leading-tight font-bold text-[#ff9a2e] uppercase [font-stretch:75%]">
      {t('misc.referral.unit')}
    </span>
    <span
      class="cta relative mt-2 ml-[3px] self-start overflow-hidden bg-[#ff5a0a] px-2.5 py-1 text-[11px] font-bold tracking-wide uppercase outline outline-1 outline-offset-[3px] outline-[#ff5a0a]/70 transition-colors [font-stretch:85%] group-hover:bg-[#ff7424]"
    >
      {t('misc.referral.cta')}
    </span>
  </div>
</a>

<style>
  /* A bright band that crosses the button every few seconds: motion
     catches the eye where a glow only blurs the type. */
  .cta::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      100deg,
      transparent 30%,
      rgb(255 255 255 / 0.55) 50%,
      transparent 70%
    );
    transform: translateX(-120%);
    animation: sweep 4s ease-in-out 1s infinite;
  }

  @keyframes sweep {
    0% {
      transform: translateX(-120%);
    }
    25%,
    100% {
      transform: translateX(120%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .cta::after {
      animation: none;
    }
  }
</style>
