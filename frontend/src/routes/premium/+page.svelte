<script lang="ts">
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

  let { data }: PageProps = $props();

  const columns = $derived(heroColumns(data.sampleModules));
  const premium = $derived(data.premium);
  const character = $derived(premium.premium_character);

  function copyPremiumCharacter() {
    void navigator.clipboard.writeText(character);
    notifySuccess('Character name copied', `${character} has been copied to your clipboard`);
  }

  const features = [
    {
      title: 'Historic sales',
      description:
        'Browse every recorded sale for any module type and see what the market actually pays.',
      icon: History,
    },
    {
      title: 'Similar sold modules',
      description:
        'Every module page shows comparable rolls that sold, with average, lowest and highest prices.',
      icon: PackageCheck,
    },
    {
      title: 'Priority ordering',
      description: 'Your modules are listed first on collection and character pages.',
      icon: ListOrdered,
    },
    {
      title: 'Gold name',
      description: 'Your character name shines gold across the site.',
      icon: Crown,
    },
  ];

  const steps = $derived([
    {
      title: 'Send the ISK in-game',
      description: `Send the amount for your plan as an ISK donation to ${character} — from the character that should get premium.`,
    },
    {
      title: 'We pick it up automatically',
      description:
        "The wallet is checked every minute. EVE's API can delay new donations, so allow up to an hour for yours to appear.",
    },
    {
      title: 'Confirmation by EVE mail',
      description: `Once processed, your character receives an in-game mail from ${character} and premium is active immediately.`,
    },
  ]);
</script>

<PageMeta
  title="Premium Features"
  description="Upgrade to premium and unlock exclusive features on MutaMarket!"
/>

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
                <ModuleCard {module} settings={data.displaySettings} />
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
      <span class="hud-label">MutaMarket Premium</span>
      <h1
        class="mt-3 text-5xl font-bold text-balance text-primary [text-shadow:0_0_24px_var(--glow)]"
      >
        Know what every roll is worth
      </h1>
      <p class="mx-auto mt-5 max-w-xl text-lg text-foreground/90">
        Historic sales, similar sold modules and priority listings for your contracts — paid with
        ISK, entirely in-game.
      </p>
      <div class="mt-8 flex items-center gap-2 text-sm">
        <span class="text-muted-foreground">Send ISK in-game to</span>
        <button
          class="inline-flex cursor-pointer items-center gap-2 border border-border bg-card px-3 py-1.5 font-mono text-sm transition-colors hover:bg-muted"
          onclick={copyPremiumCharacter}
        >
          {character}
          <Copy class="size-3.5 text-muted-foreground" />
        </button>
      </div>
      <p class="mt-3 text-sm text-muted-foreground">
        {toCompact(premium.premium_cost)} ISK per month · scroll to see how it works
      </p>
    </div>
  </div>

  <!-- What you get -->
  <section>
    <span class="hud-label block text-center">What you get</span>
    <h2 class="mt-2 text-center text-2xl font-semibold">Everything premium unlocks</h2>
    <div class="mt-8 grid gap-4 sm:grid-cols-2">
      {#each features as feature (feature.title)}
        <div class="hud-frame flex gap-4 p-5">
          <div class="grid size-10 shrink-0 place-items-center bg-primary/10">
            <feature.icon class="size-5 text-primary" />
          </div>
          <div>
            <h3 class="font-semibold">{feature.title}</h3>
            <p class="mt-1 text-sm text-muted-foreground">{feature.description}</p>
          </div>
        </div>
      {/each}
    </div>
  </section>

  <!-- Pricing -->
  <section class="mx-auto w-full max-w-lg">
    <span class="hud-label block text-center">Pricing</span>
    <h2 class="mt-2 text-center text-2xl font-semibold">One subscription, two ways to pay</h2>
    <div class="hud-frame mt-8 divide-y divide-border">
      <div class="flex items-center justify-between gap-4 p-5">
        <span>1 month</span>
        <span class="hud-readout whitespace-nowrap">
          {toCompact(premium.premium_cost)} ISK
        </span>
      </div>
      <div class="flex items-center justify-between gap-4 p-5">
        <div class="flex flex-wrap items-center gap-2">
          <span>12 months</span>
          <Badge variant="positive">Save {toCompact(yearlySavings(premium))}</Badge>
        </div>
        <span class="hud-readout whitespace-nowrap">
          {toCompact(premium.premium_yearly_cost)} ISK
        </span>
      </div>
    </div>
    <p class="mt-4 text-center text-sm text-muted-foreground">
      Premium is per character — the character that sends the ISK gets it.
    </p>
  </section>

  <!-- How it works -->
  <section class="mx-auto max-w-2xl">
    <span class="hud-label block text-center">How it works</span>
    <h2 class="mt-2 text-center text-2xl font-semibold">From ISK to premium in three steps</h2>
    <ol class="mt-10 ml-5 space-y-10 border-l border-border">
      {#each steps as step, index (step.title)}
        <li class="relative pl-10">
          <span
            class="hud-readout absolute top-0 -left-5 grid size-10 place-items-center border border-border bg-card text-primary"
          >
            0{index + 1}
          </span>
          <h3 class="pt-2 font-semibold">{step.title}</h3>
          <p class="mt-1 text-sm text-muted-foreground">{step.description}</p>
        </li>
      {/each}
    </ol>
    <p class="mt-10 text-center text-sm text-muted-foreground">
      Sent a partial amount? Donations accumulate — top up the difference and premium activates as
      soon as a full month is covered.
    </p>
    <div class="mt-6 flex items-center justify-center gap-2 text-sm">
      <span class="text-muted-foreground">Send ISK in-game to</span>
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
