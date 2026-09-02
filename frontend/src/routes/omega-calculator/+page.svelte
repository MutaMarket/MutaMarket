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
    notifySuccess('Copied!', `Use code '${MARKEEDRAGON_CODE}' at MarkeeDragon checkout`);
    setTimeout(() => {
      codeCopied = false;
    }, 2000);
  }

  const steps = [
    {
      title: 'Buy Discounted PLEX',
      linked: true,
    },
    {
      title: 'Wait for NES Sale',
      body: 'Keep your PLEX until the New Eden Store has an Omega sale. These typically offer 20-25% off 12 or 24 month packages.',
    },
    {
      title: 'Stack Your Savings',
      body: 'Redeem your discounted PLEX for discounted Omega. The savings compound, giving you up to 40%+ off compared to regular prices.',
    },
  ];
</script>

<PageMeta
  title="Omega Calculator"
  description="Calculate your savings by stacking EVE Store and NES Omega sales. Learn how to maximize your Omega time with PLEX discounts."
  image={{ url: '/img/omega-calculator.png', width: 1280, height: 800 }}
/>

<div class="mx-auto lg:max-w-4xl">
  <PageHeader
    title="Omega Sale Stacking Calculator"
    subtitle="Maximize your savings by combining PLEX discounts with NES Omega sales"
  >
    {#snippet icon()}
      <div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
        <Clock class="size-5 text-primary" stroke-width={1.5} />
      </div>
    {/snippet}
  </PageHeader>

  <!-- How It Works -->
  <section class="mb-8">
    <h2 class="mb-4 text-lg font-medium text-muted-foreground">How Sale Stacking Works</h2>
    <div class="grid gap-4 md:grid-cols-3">
      {#each steps as step, index (step.title)}
        <div class="hud-frame p-4">
          <div class="mb-2 text-2xl font-bold text-primary">{index + 1}</div>
          <h3 class="mb-2 font-medium">{step.title}</h3>
          {#if step.linked}
            <p class="text-sm text-muted-foreground">
              Purchase PLEX from the EVE Store during a sale, or use
              <a
                class="text-primary hover:underline"
                href={MARKEEDRAGON_URL}
                rel="noopener noreferrer"
                target="_blank"
              >
                MarkeeDragon
              </a>
              and use code <span class="font-semibold">{MARKEEDRAGON_CODE}</span> for an extra 3% off.
            </p>
          {:else}
            <p class="text-sm text-muted-foreground">{step.body}</p>
          {/if}
        </div>
      {/each}
    </div>
    <p class="mt-4 text-xs text-muted-foreground">
      <span class="font-medium text-amber-500">Tip for EU buyers:</span>
      MarkeeDragon charges in USD, so when the EUR/USD rate is favorable, you save even more!
    </p>
  </section>

  <!-- Calculator -->
  <section class="mb-8">
    <h2 class="mb-4 text-lg font-medium text-muted-foreground">Calculate Your Savings</h2>
    <div class="grid gap-6 md:grid-cols-2">
      <!-- Inputs -->
      <div class="hud-frame flex flex-col gap-4 p-4">
        <h3 class="font-medium">PLEX Purchase</h3>

        <div>
          <Label class="mb-2 block text-sm text-muted-foreground">PLEX Package</Label>
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
            PLEX Sale Discount: {plexDiscount}%
          </Label>
          <Slider type="single" bind:value={plexDiscount} min={0} max={50} step={1} />
        </div>

        <div class="flex items-center gap-2">
          <Checkbox id="markeedragon" bind:checked={useMarkeedragon} />
          <Label class="cursor-pointer text-sm leading-none" for="markeedragon">
            Use MarkeeDragon (+3% off)
          </Label>
        </div>

        <Separator />

        <h3 class="font-medium">NES Omega</h3>

        <div>
          <Label class="mb-2 block text-sm text-muted-foreground">Omega Package</Label>
          <Select.Root type="single" bind:value={selectedOmegaIndex}>
            <Select.Trigger class="w-full">
              {omegaPkg.months} Months ({omegaPkg.regularPlex} PLEX)
            </Select.Trigger>
            <Select.Content>
              <Select.Group>
                {#each OMEGA_PACKAGES as pkg, index (pkg.months)}
                  <Select.Item value={index.toString()}>
                    {pkg.months} Months ({pkg.regularPlex} PLEX)
                  </Select.Item>
                {/each}
              </Select.Group>
            </Select.Content>
          </Select.Root>
        </div>

        <div>
          <Label class="mb-2 block text-sm text-muted-foreground">
            NES Omega Discount: {nesDiscount}%
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
          <div class="mb-1 text-xs text-muted-foreground sm:text-sm">Effective Cost per Month</div>
          <div class="text-3xl font-bold text-positive sm:text-4xl">
            ${perMonth.toFixed(2)}
          </div>
          <div class="mt-1 text-xs text-muted-foreground sm:text-sm">
            vs ${regularPerMonth.toFixed(2)} without discounts
          </div>
        </div>

        <!-- Summary stats -->
        <div class="grid grid-cols-2 gap-3">
          <div class="rounded-lg bg-card-2 p-3 text-center">
            <div class="text-2xl font-bold">{months}</div>
            <div class="text-xs text-muted-foreground">months Omega</div>
          </div>
          <div class="rounded-lg bg-card-2 p-3 text-center">
            <div class="text-2xl font-bold text-positive">+{extraMonths}</div>
            <div class="text-xs text-muted-foreground">extra months</div>
          </div>
        </div>

        <Separator />

        <!-- Breakdown -->
        <div class="flex flex-col gap-2 text-sm">
          <div class="flex justify-between">
            <span class="text-muted-foreground">PLEX Cost:</span>
            <span>
              ${plexPrice.toFixed(2)}
              <span class="text-positive">(-{totalDiscount}%)</span>
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">Omega ({omegaPkg.months}mo):</span>
            <span>
              {omegaPlex.toLocaleString('en-US')} PLEX
              {#if nesDiscount > 0}
                <span class="text-positive">(-{nesDiscount}%)</span>
              {/if}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">Money Saved:</span>
            <span class="text-positive">${moneySaved.toFixed(2)}</span>
          </div>
        </div>

        <!-- MarkeeDragon code callout -->
        {#if useMarkeedragon}
          <div class="rounded-lg border border-positive/30 bg-positive/10 p-3">
            <div class="mb-2 flex items-center justify-between">
              <span class="text-xs font-medium text-positive">MarkeeDragon Code</span>
              <span
                class="rounded bg-positive/20 px-1.5 py-0.5 text-[10px] font-medium text-positive"
              >
                +3% OFF
              </span>
            </div>
            <button
              class="group flex w-full cursor-pointer items-center justify-center gap-2 rounded border border-border bg-card-2 px-4 py-2.5 transition-colors hover:border-positive/50 hover:bg-card-1"
              onclick={copyCode}
              title={codeCopied ? 'Copied!' : 'Click to copy'}
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
              Go to MarkeeDragon
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
            EXCLUSIVE
          </span>
          <span class="text-xl font-bold text-positive sm:text-2xl">+3% OFF</span>
        </div>
        <h2 class="mb-1 text-base font-medium sm:text-lg">Stack even more with MarkeeDragon</h2>
        <p class="text-xs text-muted-foreground sm:text-sm">
          Use code <span class="font-semibold">{MARKEEDRAGON_CODE}</span> for an extra 3% discount on
          top of any sale.
        </p>
      </div>
      <a
        class="group flex w-full items-center justify-center gap-2 rounded-lg bg-positive px-4 py-3 text-sm font-medium text-white shadow-lg shadow-positive/20 transition-all hover:brightness-110 hover:shadow-positive/30 sm:w-auto sm:px-6"
        href={MARKEEDRAGON_URL}
        rel="noopener noreferrer"
        target="_blank"
      >
        Buy PLEX at MarkeeDragon
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
      Savings Comparison
    </h2>
    <div class="overflow-x-auto">
      <table class="w-full text-xs sm:text-sm">
        <thead>
          <tr class="border-b border-border">
            <th class="px-2 py-2 text-left font-medium whitespace-nowrap sm:px-3">Scenario</th>
            <th
              class="hidden px-2 py-2 text-right font-medium whitespace-nowrap sm:table-cell sm:px-3"
            >
              Cost
            </th>
            <th class="px-2 py-2 text-right font-medium whitespace-nowrap sm:px-3">Omega</th>
            <th class="px-2 py-2 text-right font-medium whitespace-nowrap sm:px-3">Saved</th>
            <th class="px-2 py-2 text-right font-medium whitespace-nowrap sm:px-3">Extra</th>
            <th class="px-2 py-2 text-right font-medium whitespace-nowrap sm:px-3">$/Mo</th>
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
