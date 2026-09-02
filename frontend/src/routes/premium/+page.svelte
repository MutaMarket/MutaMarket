<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  // The premium sales page, the legacy Premium/ShowPremiumPage.vue:
  // the falling module-card hero, the feature grid, the two price
  // points and the three-step how-it-works — with the copyable service
  // character throughout.
  import { Copy, Crown, History, ListOrdered, PackageCheck } from '@lucide/svelte';
  import type { PageProps } from './$types';
  import ModuleCard from '$lib/components/module-card.svelte';
  import { Badge } from '$lib/components/ui/badge';
  import { toCompact } from '$lib/format-number';
  import { heroColumns, yearlySavings } from '$lib/premium';
  import { notifySuccess } from '$lib/toast';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { t } from '$lib/i18n.svelte';

  let { data }: PageProps = $props();
  const settings = useDisplaySettings();

  const columns = $derived(heroColumns(data.sampleModules));
  const premium = $derived(data.premium);
  const character = $derived(premium.premium_character);

  function copyPremiumCharacter() {
    void navigator.clipboard.writeText(character);
    notifySuccess(
      t('premium.show.characterCopiedTitle'),
      t('premium.show.characterCopiedDescription', { name: character }),
    );
  }

  // The legacy feature cards and how-it-works steps, keyed under
  // premium.show.features.* and premium.show.steps.*.
  const features = [
    { key: 'historicSales', icon: History },
    { key: 'similarSold', icon: PackageCheck },
    { key: 'priorityOrdering', icon: ListOrdered },
    { key: 'goldName', icon: Crown },
  ];

  const steps = ['send', 'pickup', 'confirm'];
</script>

<PageMeta title={t('meta.premium.title')} description={t('meta.premium.description')} />

<div class="mx-auto max-w-5xl space-y-24 pb-12">
  <!-- Hero -->
  <div
    class="relative overflow-hidden rounded-3xl border border-border shadow-[0_25px_80px_-20px_rgba(0,0,0,0.9)]"
  >
    {#if columns.length > 0}
      <div aria-hidden="true" class="pointer-events-none absolute inset-0 select-none">
        <div class="flex justify-center gap-6 opacity-70 blur-[2px]">
          {#each columns as column, columnIndex (columnIndex)}
            <div
              style:animation-duration="{45 + columnIndex * 14}s"
              style:animation-delay="-{columnIndex * 9}s"
              class="premium-fall w-72 shrink-0 space-y-6"
            >
              {#each [...column, ...column] as module, copyIndex (`${module.id}-${copyIndex}`)}
                <ModuleCard {module} {settings} />
              {/each}
            </div>
          {/each}
        </div>
        <div class="absolute inset-0 bg-background/25"></div>
        <div
          class="absolute inset-0 rounded-3xl [box-shadow:inset_0_0_120px_50px_rgba(4,5,10,0.85)]"
        ></div>
      </div>
    {/if}
    <div
      class="relative z-10 flex min-h-[36rem] flex-col items-center justify-center px-4 py-24 text-center"
    >
      <span class="hud-label">{t('premium.show.heroLabel')}</span>
      <h1
        class="mt-3 text-5xl font-bold text-balance text-primary [text-shadow:0_0_24px_var(--glow)]"
      >
        {t('premium.show.heroTitle')}
      </h1>
      <p class="mx-auto mt-5 max-w-xl text-lg text-foreground/90">
        {t('premium.show.heroDescription')}
      </p>
      <div class="mt-8 flex items-center gap-2 text-sm">
        <span class="text-muted-foreground">{t('premium.show.sendIskTo')}</span>
        <button
          class="inline-flex cursor-pointer items-center gap-2 border border-border bg-card px-3 py-1.5 font-mono text-sm transition-colors hover:bg-muted"
          onclick={copyPremiumCharacter}
        >
          {character}
          <Copy class="size-3.5 text-muted-foreground" />
        </button>
      </div>
      <p class="mt-3 text-sm text-muted-foreground">
        {t('premium.show.pricePerMonthHint', { price: toCompact(premium.premium_cost) })}
      </p>
    </div>
  </div>

  <!-- What you get -->
  <section>
    <span class="hud-label block text-center">{t('premium.show.whatYouGetLabel')}</span>
    <h2 class="mt-2 text-center text-2xl font-semibold">{t('premium.show.whatYouGetTitle')}</h2>
    <div class="mt-8 grid gap-4 sm:grid-cols-2">
      {#each features as feature (feature.key)}
        <div class="hud-frame flex gap-4 p-5">
          <div class="grid size-10 shrink-0 place-items-center bg-primary/10">
            <feature.icon class="size-5 text-primary" />
          </div>
          <div>
            <h3 class="font-semibold">{t(`premium.show.features.${feature.key}.title`)}</h3>
            <p class="mt-1 text-sm text-muted-foreground">
              {t(`premium.show.features.${feature.key}.description`)}
            </p>
          </div>
        </div>
      {/each}
    </div>
  </section>

  <!-- Pricing -->
  <section class="mx-auto w-full max-w-lg">
    <span class="hud-label block text-center">{t('premium.show.pricingLabel')}</span>
    <h2 class="mt-2 text-center text-2xl font-semibold">{t('premium.show.pricingTitle')}</h2>
    <div class="hud-frame mt-8 divide-y divide-border">
      <div class="flex items-center justify-between gap-4 p-5">
        <span>{t('premium.show.oneMonth')}</span>
        <span class="hud-readout whitespace-nowrap">
          {t('premium.iskAmount', { price: toCompact(premium.premium_cost) })}
        </span>
      </div>
      <div class="flex items-center justify-between gap-4 p-5">
        <div class="flex flex-wrap items-center gap-2">
          <span>{t('premium.show.twelveMonths')}</span>
          <Badge variant="positive">
            {t('premium.show.saveAmount', { amount: toCompact(yearlySavings(premium)) })}
          </Badge>
        </div>
        <span class="hud-readout whitespace-nowrap">
          {t('premium.iskAmount', { price: toCompact(premium.premium_yearly_cost) })}
        </span>
      </div>
    </div>
    <p class="mt-4 text-center text-sm text-muted-foreground">
      {t('premium.show.perCharacterNote')}
    </p>
  </section>

  <!-- How it works -->
  <section class="mx-auto max-w-2xl">
    <span class="hud-label block text-center">{t('premium.show.howItWorksLabel')}</span>
    <h2 class="mt-2 text-center text-2xl font-semibold">{t('premium.show.howItWorksTitle')}</h2>
    <ol class="mt-10 ml-5 space-y-10 border-l border-border">
      {#each steps as step, index (step)}
        <li class="relative pl-10">
          <span
            class="hud-readout absolute top-0 -left-5 grid size-10 place-items-center border border-border bg-card text-primary"
          >
            0{index + 1}
          </span>
          <h3 class="pt-2 font-semibold">{t(`premium.show.steps.${step}.title`)}</h3>
          <p class="mt-1 text-sm text-muted-foreground">
            {t(`premium.show.steps.${step}.description`, { name: character })}
          </p>
        </li>
      {/each}
    </ol>
    <p class="mt-10 text-center text-sm text-muted-foreground">
      {t('premium.show.partialNote')}
    </p>
    <div class="mt-6 flex items-center justify-center gap-2 text-sm">
      <span class="text-muted-foreground">{t('premium.show.sendIskTo')}</span>
      <button
        class="inline-flex cursor-pointer items-center gap-2 border border-border bg-card px-3 py-1.5 font-mono text-sm transition-colors hover:bg-muted"
        onclick={copyPremiumCharacter}
      >
        {character}
        <Copy class="size-3.5 text-muted-foreground" />
      </button>
    </div>
  </section>
</div>

<style>
  @keyframes premium-fall {
    from {
      transform: translateY(-50%);
    }
    to {
      transform: translateY(0);
    }
  }

  .premium-fall {
    animation: premium-fall 45s linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .premium-fall {
      animation: none;
    }
  }
</style>
