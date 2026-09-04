<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  // The premium sales page, the legacy Premium/ShowPremiumPage.vue
  // content (hero, features, two price points, three steps) laid out
  // around one element: the in-game transfer ticket that names the
  // recipient, the plan and the amount. The falling module cards keep
  // the product in view behind it. Divergence: the legacy centered
  // sections and separate pricing table are gone.
  import { Copy, Crown, History, ListOrdered, PackageCheck, Palette } from '@lucide/svelte';
  import type { PageProps } from './$types';
  import ModuleCard from '$lib/components/module-card.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { ACCENT_PRESETS } from '$lib/accent';
  import { toCompact } from '$lib/format-number';
  import { t } from '$lib/i18n.svelte';
  import { heroColumns, planAmount, type PremiumPlan, yearlySavings } from '$lib/premium';
  import { notifySuccess } from '$lib/toast';

  let { data }: PageProps = $props();
  const settings = useDisplaySettings();

  const columns = $derived(heroColumns(data.sampleModules));
  const premium = $derived(data.premium);
  const character = $derived(premium.premium_character);
  /** The gold-name demo shows the visitor's own name when there is one. */
  const demoName = $derived(data.nav?.user.name ?? character);

  let plan: PremiumPlan = $state('monthly');
  const amount = $derived(toCompact(planAmount(premium, plan)));

  function copyPremiumCharacter() {
    void navigator.clipboard.writeText(character);
    notifySuccess(
      t('premium.show.characterCopiedTitle'),
      t('premium.show.characterCopiedDescription', { name: character }),
    );
  }

  const plans: { key: PremiumPlan; label: string }[] = [
    { key: 'monthly', label: 'premium.show.oneMonth' },
    { key: 'yearly', label: 'premium.show.twelveMonths' },
  ];
  const headlineFeatures = [
    { key: 'historicSales', icon: History },
    { key: 'similarSold', icon: PackageCheck },
  ];
  const steps = ['send', 'pickup', 'confirm'];
</script>

<PageMeta title={t('meta.premium.title')} description={t('meta.premium.description')} />

{#snippet ticket()}
  <div class="hud-frame bg-card/95 p-5">
    <div class="flex items-center justify-between">
      <span class="hud-label">{t('premium.show.transferTitle')}</span>
      <Crown class="size-4 text-primary" />
    </div>
    <dl class="mt-4 space-y-4 text-sm">
      <div>
        <dt class="text-muted-foreground">{t('premium.show.sendIskTo')}</dt>
        <dd class="mt-1.5">
          <button
            type="button"
            class="flex w-full cursor-pointer items-center justify-between gap-3 border border-border bg-card-2 px-3 py-2 font-mono text-base transition-colors hover:bg-muted"
            onclick={copyPremiumCharacter}
          >
            {character}
            <Copy class="size-4 text-muted-foreground" />
          </button>
        </dd>
      </div>
      <div>
        <dt class="text-muted-foreground">{t('premium.show.planLabel')}</dt>
        <dd class="mt-1.5 grid grid-cols-2 gap-1.5">
          {#each plans as option (option.key)}
            <button
              type="button"
              aria-pressed={plan === option.key}
              class="cursor-pointer border px-3 py-2 text-left transition-colors {plan ===
              option.key
                ? 'border-primary bg-primary/10 text-foreground'
                : 'border-border text-muted-foreground hover:text-foreground'}"
              onclick={() => (plan = option.key)}
            >
              <span class="block font-medium">{t(option.label)}</span>
              <span class="block text-xs {plan === option.key ? 'text-primary' : ''}">
                {option.key === 'yearly'
                  ? t('premium.show.saveAmount', { amount: toCompact(yearlySavings(premium)) })
                  : t('premium.iskAmount', { price: toCompact(premium.premium_cost) })}
              </span>
            </button>
          {/each}
        </dd>
      </div>
      <div class="flex items-baseline justify-between border-t border-border pt-4">
        <dt class="text-muted-foreground">{t('premium.show.amountLabel')}</dt>
        <dd class="hud-readout text-2xl font-semibold text-primary">
          {t('premium.iskAmount', { price: amount })}
        </dd>
      </div>
    </dl>
    <p class="mt-3 text-xs text-muted-foreground">{t('premium.show.perCharacterNote')}</p>
  </div>
{/snippet}

<div class="mx-auto max-w-5xl space-y-20 pb-12">
  <section class="hud-frame relative overflow-hidden">
    {#if columns.length > 0}
      <div
        inert
        aria-hidden="true"
        class="pointer-events-none absolute inset-y-0 right-0 hidden w-1/2 select-none [mask-image:linear-gradient(to_right,transparent,black_70%)] md:block"
      >
        <div class="flex justify-end gap-5 opacity-40">
          {#each columns.slice(0, 2) as column, columnIndex (columnIndex)}
            <div
              style:animation-duration="{45 + columnIndex * 14}s"
              style:animation-delay="-{columnIndex * 9}s"
              class="premium-fall w-72 shrink-0 space-y-5"
            >
              {#each [...column, ...column] as module, copyIndex (`${module.id}-${copyIndex}`)}
                <ModuleCard {module} {settings} />
              {/each}
            </div>
          {/each}
        </div>
      </div>
    {/if}
    <div
      class="relative z-10 grid gap-10 p-6 sm:p-10 md:grid-cols-[minmax(0,1fr)_20rem] md:items-center lg:p-14"
    >
      <div class="max-w-lg">
        <span class="hud-label">{t('premium.show.heroLabel')}</span>
        <h1 class="mt-4 text-4xl font-bold text-balance md:text-5xl">
          {t('premium.show.heroTitle')}
        </h1>
        <p class="mt-5 max-w-md text-base text-muted-foreground">
          {t('premium.show.heroDescription')}
        </p>
      </div>
      {@render ticket()}
    </div>
  </section>

  <section>
    <h2 class="text-2xl font-semibold">{t('premium.show.whatYouGetTitle')}</h2>
    <div class="mt-6 grid gap-4 md:grid-cols-2">
      {#each headlineFeatures as feature (feature.key)}
        <div class="hud-frame p-6">
          <feature.icon class="size-6 text-primary" />
          <h3 class="mt-4 text-lg font-semibold">
            {t(`premium.show.features.${feature.key}.title`)}
          </h3>
          <p class="mt-2 max-w-md text-sm text-muted-foreground">
            {t(`premium.show.features.${feature.key}.description`)}
          </p>
        </div>
      {/each}
    </div>
    <div class="mt-4 grid gap-4 md:grid-cols-3">
      <div class="border-t border-border pt-4">
        <div class="flex items-center gap-2">
          <ListOrdered class="size-4 text-primary" />
          <h3 class="font-semibold">{t('premium.show.features.priorityOrdering.title')}</h3>
        </div>
        <p class="mt-2 text-sm text-muted-foreground">
          {t('premium.show.features.priorityOrdering.description')}
        </p>
      </div>
      <div class="border-t border-border pt-4">
        <div class="flex items-center gap-2">
          <Crown class="size-4 text-primary" />
          <h3 class="font-semibold">{t('premium.show.features.goldName.title')}</h3>
        </div>
        <p class="mt-2 text-sm text-muted-foreground">
          {t('premium.show.features.goldName.description')}
        </p>
        <p class="mt-3 text-lg font-semibold">
          <span class="text-gold">{demoName}</span>
        </p>
      </div>
      <div class="border-t border-border pt-4">
        <div class="flex items-center gap-2">
          <Palette class="size-4 text-primary" />
          <h3 class="font-semibold">{t('premium.show.features.themeColor.title')}</h3>
        </div>
        <p class="mt-2 text-sm text-muted-foreground">
          {t('premium.show.features.themeColor.description')}
        </p>
        <div class="mt-3 flex gap-1.5" aria-hidden="true">
          {#each ACCENT_PRESETS as preset (preset)}
            <span class="size-4 rounded-full" style="background-color: {preset}"></span>
          {/each}
        </div>
      </div>
    </div>
  </section>

  <section>
    <h2 class="text-2xl font-semibold">{t('premium.show.howItWorksTitle')}</h2>
    <ol class="mt-6 grid gap-6 md:grid-cols-3">
      {#each steps as step, index (step)}
        <li class="border-t border-border pt-4">
          <span class="hud-readout text-sm text-primary">0{index + 1}</span>
          <h3 class="mt-2 font-semibold">{t(`premium.show.steps.${step}.title`)}</h3>
          <p class="mt-2 text-sm text-muted-foreground">
            {t(`premium.show.steps.${step}.description`, { name: character })}
          </p>
        </li>
      {/each}
    </ol>
    <p class="mt-8 max-w-2xl text-sm text-muted-foreground">{t('premium.show.partialNote')}</p>
  </section>

  <section class="grid items-center gap-8 md:grid-cols-[minmax(0,1fr)_20rem]">
    <h2 class="max-w-md text-3xl font-semibold text-balance">{t('premium.show.closingTitle')}</h2>
    {@render ticket()}
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
