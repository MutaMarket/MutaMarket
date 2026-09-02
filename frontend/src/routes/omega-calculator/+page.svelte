<script lang="ts">
  // The omega sale-stacking calculator, the legacy
  // OmegaCalculatorPage.vue: how-it-works steps, the PLEX/NES inputs
  // with live results, the MarkeeDragon CTA and the five-scenario
  // comparison table. All math lives in $lib/omega. Divergence: the
  // legacy page received two env-driven store sale strings it never
  // used; they are gone.
  import Check from '@lucide/svelte/icons/check';
  import Clock from '@lucide/svelte/icons/clock';
  import Copy from '@lucide/svelte/icons/copy';
  import PageHeader from '$lib/components/page-header.svelte';
  import Trans from '$lib/components/trans.svelte';
  import { t } from '$lib/i18n.svelte';
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Label } from '$lib/components/ui/label';
  import * as Select from '$lib/components/ui/select';
  import { Separator } from '$lib/components/ui/separator';
  import { Slider } from '$lib/components/ui/slider';
  import { notifySuccess } from '$lib/toast';
  import {
    OMEGA_PACKAGES,
    PLEX_PACKAGES,
    calculateScenario,
    costPerMonth,
    discountedOmegaPlex,
    discountedPlexPrice,
    effectiveTotalDiscount,
    omegaMonthsAffordable,
    regularCostPerMonth,
    regularOmegaMonths,
    scenarios,
  } from '$lib/omega';
  import { MARKEEDRAGON_CODE, MARKEEDRAGON_URL } from '$lib/partner-links';
  import PageMeta from '$lib/components/page-meta.svelte';

  // Calculator state, the legacy defaults.
  let selectedPlexIndex = $state('0');
  let plexDiscount = $state(20);
  let useMarkeedragon = $state(true);
  let selectedOmegaIndex = $state('0');
  let nesDiscount = $state(20);

  const plexPkg = $derived(PLEX_PACKAGES[Number(selectedPlexIndex)]);
  const omegaPkg = $derived(OMEGA_PACKAGES[Number(selectedOmegaIndex)]);

  const plexPrice = $derived(discountedPlexPrice(plexPkg, plexDiscount, useMarkeedragon));
  const totalDiscount = $derived(effectiveTotalDiscount(plexPkg, plexDiscount, useMarkeedragon));
  const omegaPlex = $derived(discountedOmegaPlex(omegaPkg, nesDiscount));
  const months = $derived(omegaMonthsAffordable(plexPkg, omegaPkg, nesDiscount));
  const perMonth = $derived(
    costPerMonth(plexPkg, omegaPkg, plexDiscount, useMarkeedragon, nesDiscount),
  );
  const regularMonths = $derived(regularOmegaMonths(plexPkg, omegaPkg));
  const regularPerMonth = $derived(regularCostPerMonth(plexPkg, omegaPkg));
  const moneySaved = $derived(plexPkg.basePrice - plexPrice);
  const extraMonths = $derived(months - regularMonths);
  const comparisonRows = $derived(
    scenarios(plexDiscount, useMarkeedragon, nesDiscount, omegaPkg.months),
  );

  let codeCopied = $state(false);
  function copyCode() {
    navigator.clipboard.writeText(MARKEEDRAGON_CODE);
    codeCopied = true;
    notifySuccess(t('calculator.omega.copiedTitle'), t('calculator.omega.copiedBody'));
    setTimeout(() => {
      codeCopied = false;
    }, 2000);
  }

  // The legacy how-it-works steps, calculator.omega.step{n}Title/Body;
  // the first body carries the MarkeeDragon link and code.
  const steps = [1, 2, 3];
</script>

<PageMeta
  title={t('meta.omegaCalculator.title')}
  description={t('meta.omegaCalculator.description')}
  image={{ url: '/img/omega-calculator.png', width: 1280, height: 800 }}
/>

<div class="mx-auto lg:max-w-4xl">
  <PageHeader title={t('calculator.omega.heading')} subtitle={t('calculator.omega.subheading')}>
    {#snippet icon()}
      <div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
        <Clock class="size-5 text-primary" stroke-width={1.5} />
      </div>
    {/snippet}
  </PageHeader>

  <!-- How It Works -->
  <section class="mb-8">
    <h2 class="mb-4 text-lg font-medium text-muted-foreground">
      {t('calculator.omega.howItWorks')}
    </h2>
    <div class="grid gap-4 md:grid-cols-3">
      {#each steps as step (step)}
        <div class="hud-frame p-4">
          <div class="mb-2 text-2xl font-bold text-primary">{step}</div>
          <h3 class="mb-2 font-medium">{t(`calculator.omega.step${step}Title`)}</h3>
          {#if step === 1}
            <p class="text-sm text-muted-foreground">
              <Trans key="calculator.omega.step1Body">
                {#snippet link()}
                  <a
                    class="text-primary hover:underline"
                    href={MARKEEDRAGON_URL}
                    rel="noopener noreferrer"
                    target="_blank"
                  >
                    MarkeeDragon
                  </a>
                {/snippet}
                {#snippet code()}<span class="font-semibold">{MARKEEDRAGON_CODE}</span>{/snippet}
              </Trans>
            </p>
          {:else}
            <p class="text-sm text-muted-foreground">{t(`calculator.omega.step${step}Body`)}</p>
          {/if}
        </div>
      {/each}
    </div>
    <p class="mt-4 text-xs text-muted-foreground">
      <span class="font-medium text-amber-500">{t('calculator.omega.euTipLabel')}</span>
      {t('calculator.omega.euTipBody')}
    </p>
  </section>

  <!-- Calculator -->
  <section class="mb-8">
    <h2 class="mb-4 text-lg font-medium text-muted-foreground">
      {t('calculator.omega.calculateSavings')}
    </h2>
    <div class="grid gap-6 md:grid-cols-2">
      <!-- Inputs -->
      <div class="hud-frame flex flex-col gap-4 p-4">
        <h3 class="font-medium">{t('calculator.omega.plexPurchase')}</h3>

        <div>
          <Label class="mb-2 block text-sm text-muted-foreground">
            {t('calculator.omega.plexPackage')}
          </Label>
          <Select.Root type="single" bind:value={selectedPlexIndex}>
            <Select.Trigger class="w-full">
              {plexPkg.label} (${plexPkg.basePrice})
            </Select.Trigger>
            <Select.Content>
              <Select.Group>
                {#each PLEX_PACKAGES as pkg, index (pkg.plex)}
                  <Select.Item value={index.toString()}>
                    {pkg.label} (${pkg.basePrice})
                  </Select.Item>
                {/each}
              </Select.Group>
            </Select.Content>
          </Select.Root>
        </div>

        <div>
          <Label class="mb-2 block text-sm text-muted-foreground">
            {t('calculator.omega.plexSaleDiscount', { percent: plexDiscount })}
          </Label>
          <Slider type="single" bind:value={plexDiscount} min={0} max={50} step={1} />
        </div>

        <div class="flex items-center gap-2">
          <Checkbox id="markeedragon" bind:checked={useMarkeedragon} />
          <Label class="cursor-pointer text-sm leading-none" for="markeedragon">
            {t('calculator.omega.useMarkeedragon')}
          </Label>
        </div>

        <Separator />

        <h3 class="font-medium">{t('calculator.omega.nesOmega')}</h3>

        <div>
          <Label class="mb-2 block text-sm text-muted-foreground">
            {t('calculator.omega.omegaPackage')}
          </Label>
          <Select.Root type="single" bind:value={selectedOmegaIndex}>
            <Select.Trigger class="w-full">
              {t('calculator.omega.monthsLabel', { count: omegaPkg.months })} ({omegaPkg.regularPlex}
              PLEX)
            </Select.Trigger>
            <Select.Content>
              <Select.Group>
                {#each OMEGA_PACKAGES as pkg, index (pkg.months)}
                  <Select.Item value={index.toString()}>
                    {t('calculator.omega.monthsLabel', { count: pkg.months })} ({pkg.regularPlex} PLEX)
                  </Select.Item>
                {/each}
              </Select.Group>
            </Select.Content>
          </Select.Root>
        </div>

        <div>
          <Label class="mb-2 block text-sm text-muted-foreground">
            {t('calculator.omega.nesOmegaDiscount', { percent: nesDiscount })}
          </Label>
          <Slider
            type="single"
            bind:value={nesDiscount}
            min={0}
            max={omegaPkg.maxDiscount || 25}
            step={1}
          />
        </div>
      </div>

      <!-- Results -->
      <div class="hud-frame flex flex-col gap-4 p-4">
        <!-- Hero: cost per month -->
        <div class="rounded-lg bg-card-2 p-3 text-center sm:p-4">
          <div class="mb-1 text-xs text-muted-foreground sm:text-sm">
            {t('calculator.omega.effectiveCostPerMonth')}
          </div>
          <div class="text-3xl font-bold text-positive sm:text-4xl">
            ${perMonth.toFixed(2)}
          </div>
          <div class="mt-1 text-xs text-muted-foreground sm:text-sm">
            {t('calculator.omega.vsWithoutDiscounts', { price: regularPerMonth.toFixed(2) })}
          </div>
        </div>

        <!-- Summary stats -->
        <div class="grid grid-cols-2 gap-3">
          <div class="rounded-lg bg-card-2 p-3 text-center">
            <div class="text-2xl font-bold">{months}</div>
            <div class="text-xs text-muted-foreground">{t('calculator.omega.monthsOmega')}</div>
          </div>
          <div class="rounded-lg bg-card-2 p-3 text-center">
            <div class="text-2xl font-bold text-positive">+{extraMonths}</div>
            <div class="text-xs text-muted-foreground">{t('calculator.omega.extraMonths')}</div>
          </div>
        </div>

        <Separator />

        <!-- Breakdown -->
        <div class="flex flex-col gap-2 text-sm">
          <div class="flex justify-between">
            <span class="text-muted-foreground">{t('calculator.omega.plexCost')}</span>
            <span>
              ${plexPrice.toFixed(2)}
              <span class="text-positive">(-{totalDiscount}%)</span>
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">
              {t('calculator.omega.omegaDuration', { months: omegaPkg.months })}
            </span>
            <span>
              {omegaPlex.toLocaleString('en-US')} PLEX
              {#if nesDiscount > 0}
                <span class="text-positive">(-{nesDiscount}%)</span>
              {/if}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">{t('calculator.omega.moneySaved')}</span>
            <span class="text-positive">${moneySaved.toFixed(2)}</span>
          </div>
        </div>

        <!-- MarkeeDragon code callout -->
        {#if useMarkeedragon}
          <div class="rounded-lg border border-positive/30 bg-positive/10 p-3">
            <div class="mb-2 flex items-center justify-between">
              <span class="text-xs font-medium text-positive">
                {t('calculator.omega.markeedragonCode')}
              </span>
              <span
                class="rounded bg-positive/20 px-1.5 py-0.5 text-[10px] font-medium text-positive"
              >
                {t('calculator.omega.threePercentOff')}
              </span>
            </div>
            <button
              class="group flex w-full cursor-pointer items-center justify-center gap-2 rounded border border-border bg-card-2 px-4 py-2.5 transition-colors hover:border-positive/50 hover:bg-card-1"
              onclick={copyCode}
              title={codeCopied
                ? t('calculator.omega.copiedTitle')
                : t('calculator.omega.clickToCopy')}
            >
              <code class="font-mono text-lg font-bold tracking-widest">{MARKEEDRAGON_CODE}</code>
              {#if codeCopied}
                <Check class="size-4 text-positive" />
              {:else}
                <Copy
                  class="size-4 text-muted-foreground transition-colors group-hover:text-positive"
                />
              {/if}
            </button>
            <a
              class="mt-2 flex items-center justify-center gap-1 text-xs text-positive hover:underline"
              href={MARKEEDRAGON_URL}
              rel="noopener noreferrer"
              target="_blank"
            >
              {t('calculator.omega.goToMarkeedragon')}
              <svg
                class="size-3"
                fill="none"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                viewBox="0 0 24 24"
              >
                <path d="M7 17L17 7M17 7H7M17 7V17" />
              </svg>
            </a>
          </div>
        {/if}
      </div>
    </div>
  </section>

  <!-- MarkeeDragon CTA -->
  <section
    class="relative mb-8 overflow-hidden rounded-lg border border-positive/30 bg-gradient-to-br from-positive/10 to-card p-4 sm:p-6"
  >
    <div class="flex flex-col items-center gap-4 md:flex-row md:justify-between">
      <div class="text-center md:text-left">
        <div class="mb-1 flex items-center justify-center gap-2 md:justify-start">
          <span class="rounded bg-positive/20 px-2 py-0.5 text-xs font-medium text-positive">
            {t('calculator.omega.exclusive')}
          </span>
          <span class="text-xl font-bold text-positive sm:text-2xl">
            {t('calculator.omega.threePercentOff')}
          </span>
        </div>
        <h2 class="mb-1 text-base font-medium sm:text-lg">{t('calculator.omega.stackMore')}</h2>
        <p class="text-xs text-muted-foreground sm:text-sm">
          <Trans key="calculator.omega.useCodeExtra">
            {#snippet code()}<span class="font-semibold">{MARKEEDRAGON_CODE}</span>{/snippet}
          </Trans>
        </p>
      </div>
      <a
        class="group flex w-full items-center justify-center gap-2 rounded-lg bg-positive px-4 py-3 text-sm font-medium text-white shadow-lg shadow-positive/20 transition-all hover:brightness-110 hover:shadow-positive/30 sm:w-auto sm:px-6"
        href={MARKEEDRAGON_URL}
        rel="noopener noreferrer"
        target="_blank"
      >
        {t('calculator.omega.buyPlex')}
        <svg
          class="size-4 transition-transform group-hover:translate-x-0.5"
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          viewBox="0 0 24 24"
        >
          <path d="M7 17L17 7M17 7H7M17 7V17" />
        </svg>
      </a>
    </div>
  </section>

  <!-- Comparison table -->
  <section class="-mx-2 mb-8 sm:mx-0">
    <h2 class="mb-4 px-2 text-base font-medium text-muted-foreground sm:px-0 md:text-lg">
      {t('calculator.omega.savingsComparison')}
    </h2>
    <div class="overflow-x-auto">
      <table class="w-full text-xs sm:text-sm">
        <thead>
          <tr class="border-b border-border">
            <th class="px-2 py-2 text-left font-medium whitespace-nowrap sm:px-3">
              {t('calculator.omega.tableScenario')}
            </th>
            <th
              class="hidden px-2 py-2 text-right font-medium whitespace-nowrap sm:table-cell sm:px-3"
            >
              {t('calculator.omega.tableCost')}
            </th>
            <th class="px-2 py-2 text-right font-medium whitespace-nowrap sm:px-3">
              {t('calculator.omega.tableOmega')}
            </th>
            <th class="px-2 py-2 text-right font-medium whitespace-nowrap sm:px-3">
              {t('calculator.omega.tableSaved')}
            </th>
            <th class="px-2 py-2 text-right font-medium whitespace-nowrap sm:px-3">
              {t('calculator.omega.tableExtra')}
            </th>
            <th class="px-2 py-2 text-right font-medium whitespace-nowrap sm:px-3">
              {t('calculator.omega.tablePerMonth')}
            </th>
          </tr>
        </thead>
        <tbody>
          {#each comparisonRows as scenario (scenario.name)}
            {@const result = calculateScenario(plexPkg, scenario)}
            <tr class="border-b border-border/50 {scenario.isFullStack ? 'bg-positive/5' : ''}">
              <td class="px-2 py-2 whitespace-nowrap sm:px-3">
                <span class={scenario.isFullStack ? 'font-medium' : ''}>{scenario.name}</span>
              </td>
              <td class="hidden px-2 py-2 text-right whitespace-nowrap sm:table-cell sm:px-3">
                ${result.plexCost}
              </td>
              <td class="px-2 py-2 text-right whitespace-nowrap sm:px-3">{result.months}</td>
              <td class="px-2 py-2 text-right whitespace-nowrap sm:px-3">
                {#if parseFloat(result.moneySaved) > 0}
                  <span class="text-positive">${result.moneySaved}</span>
                {:else}
                  <span class="text-muted-foreground">-</span>
                {/if}
              </td>
              <td class="px-2 py-2 text-right whitespace-nowrap sm:px-3">
                {#if result.extraMonths > 0}
                  <span class="text-positive">+{result.extraMonths}</span>
                {:else}
                  <span class="text-muted-foreground">-</span>
                {/if}
              </td>
              <td
                class="px-2 py-2 text-right font-bold whitespace-nowrap sm:px-3 {scenario.plexDiscount >
                  0 || scenario.nesDiscount > 0
                  ? 'text-positive'
                  : ''}"
              >
                ${result.costPerMonth}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>
</div>
