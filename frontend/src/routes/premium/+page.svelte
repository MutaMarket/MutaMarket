<script lang="ts">
  import { useDisplaySettings } from '$lib/display-settings.svelte';
  // The premium sales page, the legacy Premium/ShowPremiumPage.vue
  // content (hero, features, two price points, three steps) laid out
  // around one element: the in-game transfer ticket that names the
  // recipient, the plan and the amount. The falling module cards keep
  // the product in view behind it. Divergence: the legacy centered
  // sections and separate pricing table are gone.
  import {
    ChevronRight,
    Copy,
    Crown,
    Gift,
    History,
    ListOrdered,
    PackageCheck,
    Palette,
  } from '@lucide/svelte';
  import type { PageProps } from './$types';
  import { invalidateAll } from '$app/navigation';
  import ModuleCard from '$lib/components/module-card.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as Select from '$lib/components/ui/select';
  import { ACCENT_PRESETS, accentThemeCss } from '$lib/accent';
  import { toCompact } from '$lib/format-number';
  import { holoTilt } from '$lib/holo-tilt';
  import { t } from '$lib/i18n.svelte';
  import {
    clampGiftDays,
    demoCharacter,
    heroColumns,
    planAmount,
    type PremiumPlan,
    yearlySavings,
  } from '$lib/premium';
  import { sparkleStyle } from '$lib/premium-foil';
  import { notifyError, notifySuccess } from '$lib/toast';

  let { data }: PageProps = $props();
  const settings = useDisplaySettings();

  const columns = $derived(heroColumns(data.sampleModules));
  const premium = $derived(data.premium);
  const character = $derived(premium.premium_character);
  /** The premium card demo wears the visitor's own character when
   * there is one, else a sample creator, else just the gilded name. */
  const demo = $derived(demoCharacter(data.nav, data.sampleModules));
  const demoName = $derived(demo?.name ?? character);

  // Theme preview: hovering a swatch retints the page, clicking keeps
  // the color until the page is left (the style lives in this page's
  // head, so navigating away restores the account's own accent).
  let hoverAccent = $state<string | null>(null);
  let pinnedAccent = $state<string | null>(null);
  const previewStyle = $derived(accentThemeCss(hoverAccent ?? pinnedAccent));

  // Gifting: whole days from one of the account's premium characters.
  const giftable = $derived(data.giftable);
  let giftFromId = $state<string | undefined>(undefined);
  const donor = $derived(
    giftable.find((entry) => String(entry.id) === giftFromId) ?? giftable[0] ?? null,
  );
  let giftTo = $state('');
  let giftDaysInput = $state(1);
  const giftDays = $derived(clampGiftDays(giftDaysInput, donor?.remaining_days ?? 0));
  let gifting = $state(false);

  async function sendGift(event: SubmitEvent) {
    event.preventDefault();
    if (!donor || giftTo.trim() === '' || gifting) {
      return;
    }
    gifting = true;
    try {
      const response = await fetch('/premium/gift', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          from_character_id: donor.id,
          to_character_name: giftTo.trim(),
          days: giftDays,
        }),
      });
      const body = await response.json().catch(() => ({}));
      if (response.ok) {
        notifySuccess(
          t('premium.show.gift.sentTitle'),
          t('premium.show.gift.sentBody', {
            name: body.to_character_name,
            until: String(body.to_premium_paid_until).slice(0, 10),
          }),
        );
        giftTo = '';
        giftDaysInput = 1;
        await invalidateAll();
      } else {
        notifyError(
          t('premium.show.gift.failedTitle'),
          body.message ?? t('errors.internalServerError.name'),
        );
      }
    } finally {
      gifting = false;
    }
  }

  let plan = $state<PremiumPlan>('monthly');
  const amount = $derived(toCompact(planAmount(premium, plan)));
  const planLabel = $derived(
    plan === 'yearly' ? 'premium.show.twelveMonths' : 'premium.show.oneMonth',
  );

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

<svelte:head>
  {#if previewStyle}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- previewStyle is a strict hex-only string -->
    {@html `<style>${previewStyle}</style>`}
  {/if}
</svelte:head>

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
    <div class="mt-4 grid gap-6 md:grid-cols-[13rem_minmax(0,1fr)] md:gap-10">
      <div>
        {#if demo}
          <div
            use:holoTilt={true}
            style={sparkleStyle(demoName)}
            class="premium-card grid overflow-hidden rounded-lg bg-card"
          >
            <img
              alt=""
              class="aspect-square w-full object-cover"
              src="https://images.evetech.net/characters/{demo.id}/portrait?size=256"
            />
            <p class="flex items-center justify-center gap-1.5 truncate px-4 py-3 text-xl">
              <Crown class="size-4 shrink-0 text-[#d3b15f]" stroke-width={1.5} />
              <span class="text-gold truncate">{demoName}</span>
            </p>
          </div>
        {:else}
          <p class="text-2xl font-semibold"><span class="text-gold">{demoName}</span></p>
        {/if}
        <p class="mt-2 text-xs text-muted-foreground">
          {t(demo?.own ? 'premium.show.cardDemoHint' : 'premium.show.cardDemoHintOthers')}
        </p>
      </div>
      <div class="grid content-start gap-4">
        <div class="border-t border-border pt-4">
          <div class="flex items-center gap-2">
            <Crown class="size-4 text-primary" />
            <h3 class="font-semibold">{t('premium.show.features.goldName.title')}</h3>
          </div>
          <p class="mt-2 text-sm text-muted-foreground">
            {t('premium.show.features.goldName.description')}
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
          <div
            class="mt-3 flex flex-wrap gap-2"
            role="group"
            aria-label={t('premium.show.features.themeColor.title')}
          >
            {#each ACCENT_PRESETS as preset (preset)}
              <button
                type="button"
                aria-pressed={pinnedAccent === preset}
                aria-label={preset}
                class="size-6 cursor-pointer rounded-full border-2 transition-transform hover:scale-110 {pinnedAccent ===
                preset
                  ? 'border-foreground'
                  : 'border-transparent'}"
                style="background-color: {preset}"
                onmouseenter={() => (hoverAccent = preset)}
                onmouseleave={() => (hoverAccent = null)}
                onfocus={() => (hoverAccent = preset)}
                onblur={() => (hoverAccent = null)}
                onclick={() => (pinnedAccent = pinnedAccent === preset ? null : preset)}
              ></button>
            {/each}
          </div>
          <p class="mt-2 text-xs text-muted-foreground">{t('premium.show.themePreviewHint')}</p>
        </div>
        <div class="border-t border-border pt-4">
          <div class="flex items-center gap-2">
            <ListOrdered class="size-4 text-primary" />
            <h3 class="font-semibold">{t('premium.show.features.priorityOrdering.title')}</h3>
          </div>
          <p class="mt-2 text-sm text-muted-foreground">
            {t('premium.show.features.priorityOrdering.description')}
          </p>
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

  {#if donor}
    <section class="hud-frame grid gap-6 p-6 sm:p-8 md:grid-cols-2">
      <div>
        <div class="flex items-center gap-2">
          <Gift class="size-5 text-primary" />
          <h2 class="text-2xl font-semibold">{t('premium.show.gift.title')}</h2>
        </div>
        <p class="mt-3 max-w-md text-sm text-muted-foreground">
          {t('premium.show.gift.description')}
        </p>
      </div>
      <form class="grid gap-4 text-sm" onsubmit={sendGift}>
        <div class="grid gap-1.5">
          <span class="text-muted-foreground">{t('premium.show.gift.from')}</span>
          {#if giftable.length > 1}
            <Select.Root
              type="single"
              value={String(donor.id)}
              onValueChange={(value) => (giftFromId = value)}
            >
              <Select.Trigger class="h-9 w-full">{donor.name}</Select.Trigger>
              <Select.Content>
                {#each giftable as entry (entry.id)}
                  <Select.Item value={String(entry.id)}>
                    {entry.name}
                    <span class="ml-auto text-xs text-muted-foreground">
                      {t('premium.show.gift.remaining', { days: entry.remaining_days })}
                    </span>
                  </Select.Item>
                {/each}
              </Select.Content>
            </Select.Root>
          {:else}
            <span class="flex h-9 items-center justify-between border border-border px-3">
              {donor.name}
              <span class="text-xs text-muted-foreground">
                {t('premium.show.gift.remaining', { days: donor.remaining_days })}
              </span>
            </span>
          {/if}
        </div>
        <div class="grid grid-cols-[minmax(0,1fr)_6rem] gap-3">
          <label class="grid gap-1.5">
            <span class="text-muted-foreground">{t('premium.show.gift.to')}</span>
            <Input
              class="h-9"
              placeholder={t('premium.show.gift.toPlaceholder')}
              bind:value={giftTo}
              required
            />
          </label>
          <label class="grid gap-1.5">
            <span class="text-muted-foreground">{t('premium.show.gift.days')}</span>
            <Input
              class="h-9"
              type="number"
              min="1"
              max={donor.remaining_days}
              step="1"
              bind:value={giftDaysInput}
            />
          </label>
        </div>
        <div>
          <Button type="submit" size="lg" disabled={gifting || giftTo.trim() === ''}>
            <Gift />
            {gifting
              ? t('premium.show.gift.sending')
              : t('premium.show.gift.submit', { days: giftDays })}
          </Button>
        </div>
      </form>
    </section>
  {/if}

  <!-- The closing band repeats the decision, not the form: the plan
       chosen above, one button, one link. -->
  <section
    class="hud-frame flex flex-wrap items-center justify-between gap-x-10 gap-y-5 px-6 py-6 sm:px-8"
  >
    <div>
      <h2 class="text-2xl font-semibold text-balance">{t('premium.show.closingTitle')}</h2>
      <p class="hud-readout mt-2 text-sm text-primary">
        {t('premium.show.closingSummary', { amount, plan: t(planLabel) })}
      </p>
    </div>
    <div class="flex flex-wrap items-center gap-3">
      <Button size="lg" onclick={copyPremiumCharacter}>
        <Copy />
        {t('premium.show.copyName', { name: character })}
      </Button>
      <Button size="lg" variant="ghost" href="/documentation/premium">
        {t('premium.show.readGuide')}
        <ChevronRight />
      </Button>
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
