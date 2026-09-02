<script lang="ts">
  // The show-page hero, mirroring Show/ModuleHero.vue: creator details,
  // the estimator statistics sheet (or its missing-data state), and the
  // toolbar — in a hud-frame with the one-shot scan sweep.
  import { Info } from '@lucide/svelte';
  import ModuleToolbar from './module-toolbar.svelte';
  import GameImage from './game-image.svelte';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import { durationLabel, parseDbTimestamp } from '$lib/duration';
  import {
    MINIMUM_TRAINING_TRADES,
    biasScore,
    scoreWord,
    starsValue,
    tradesRemaining,
    trainingProgress,
  } from '$lib/estimator-score';
  import { toIskCompact, toVeryCompact } from '$lib/format-number';
  import { t } from '$lib/i18n.svelte';
  import { notifySuccess } from '$lib/toast';
  import type { AbyssalTypeStatistic, EstimatorStatistic, ModuleDetail } from '$lib/types';

  let {
    module,
    statistic,
    typeStatistics = [],
  }: {
    module: ModuleDetail;
    statistic: EstimatorStatistic | null;
    typeStatistics?: AbyssalTypeStatistic[];
  } = $props();

  let now = $state(Math.floor(Date.now() / 1000));
  $effect(() => {
    const tick = setInterval(() => (now = Math.floor(Date.now() / 1000)), 1000);
    return () => clearInterval(tick);
  });

  const trained = $derived(statistic !== null && statistic.r2 !== null && statistic.mae !== null);

  // scoreWord names the bands in English; these are the legacy keys for them.
  const SCORE_KEYS: Record<string, string> = {
    'Very high': 'stats.estimators.veryHigh',
    High: 'stats.estimators.high',
    Moderate: 'stats.estimators.moderate',
    Low: 'stats.estimators.low',
    'Very low': 'stats.estimators.veryLow',
  };
  const scoreLabel = (label: string) => t(SCORE_KEYS[label] ?? label);

  const confidence = $derived(statistic?.r2 == null ? null : scoreWord(starsValue(statistic.r2)));
  const trainingData = $derived(
    statistic?.data_statistics ? Object.entries(statistic.data_statistics) : [],
  );
  const totalSamples = $derived(trainingData.reduce((sum, [, count]) => sum + (count ?? 0), 0));
  const bias = $derived(
    statistic?.data_statistics ? scoreWord(starsValue(biasScore(statistic.data_statistics))) : null,
  );

  // The untrained state's readout: how far this type is from a model.
  const dataCount = $derived(statistic?.data_count ?? 0);
  const progress = $derived(trainingProgress(dataCount));
  const remaining = $derived(tradesRemaining(dataCount));

  function agoLine(timestamp: string | null): string {
    if (timestamp === null) return '';
    return durationLabel(parseDbTimestamp(timestamp) - now);
  }

  async function copyEstimate() {
    if (module.estimated_value === null) return;
    await navigator.clipboard.writeText(toVeryCompact(module.estimated_value));
    notifySuccess(t('stats.estimators.copiedTitle'), t('stats.estimators.copiedBody'));
  }
</script>

<Tooltip.Provider delayDuration={300}>
  <div class="hud-frame relative flex flex-col">
    <div aria-hidden="true" class="hud-scan pointer-events-none absolute inset-0"></div>

    <!-- CreatorDetails: linked portrait + name, gold for premium. -->
    <div class="border-b border-border">
      {#if module.creator}
        <a class="flex items-center gap-4 p-4" href="/characters/{module.creator.slug}">
          <GameImage
            src="https://images.evetech.net/characters/{module.creator.id}/portrait?size=64"
            alt={module.creator.name}
            class="h-10 w-10 rounded-lg"
          />
          <div>
            <span class="block text-sm text-muted-foreground">{t('modules.card.createdBy')}</span>
            <span class="font-medium {module.creator.has_premium ? 'text-gold' : ''}">
              {module.creator.name}
            </span>
          </div>
        </a>
      {/if}
    </div>

    {#if trained && statistic}
      <div class="flex flex-col">
        <!-- The AI value prediction block. -->
        <div class="flex grow flex-col gap-1.5 p-4">
          <h2 class="hud-label flex items-center gap-1.5">
            {t('stats.estimators.aiValuePrediction')}
            <Tooltip.Root>
              <Tooltip.Trigger>
                {#snippet child({ props })}
                  <button
                    {...props}
                    type="button"
                    class="inline-flex cursor-help text-muted-foreground hover:text-foreground"
                  >
                    <Info class="size-3" stroke-width={1.5} />
                    <span class="sr-only">{t('stats.estimators.aboutPrediction')}</span>
                  </button>
                {/snippet}
              </Tooltip.Trigger>
              <Tooltip.Content class="max-w-xs">
                {t('stats.estimators.aiTooltip')}
              </Tooltip.Content>
            </Tooltip.Root>
          </h2>
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  type="button"
                  class="cursor-pointer text-left"
                  onclick={copyEstimate}
                >
                  <span
                    class="hud-readout text-2xl text-primary [text-shadow:0_0_18px_var(--glow)]"
                  >
                    {toIskCompact(module.estimated_value)}
                  </span>
                  {#if statistic.nmae !== null}
                    <span class="hud-readout ml-2 text-muted-foreground">
                      ±{statistic.nmae.toFixed(0)}%
                    </span>
                  {/if}
                </button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content>{t('stats.estimators.copyTooltip')}</Tooltip.Content>
          </Tooltip.Root>
          {#if module.estimated_value_updated_at}
            <p class="text-xs text-muted-foreground">
              {t('stats.estimators.evaluatedAgo', {
                time: agoLine(module.estimated_value_updated_at),
              })}
            </p>
          {/if}
        </div>

        <!-- The model quality grid. -->
        <div class="grid grid-cols-2 border-t border-border sm:grid-cols-3">
          <div class="flex flex-col gap-1 p-4">
            <span class="hud-label">{t('stats.estimators.confidence')}</span>
            <span class="hud-readout text-lg uppercase {confidence?.class}">
              {confidence ? scoreLabel(confidence.label) : ''}
            </span>
            <span class="text-xs text-muted-foreground">R² {statistic.r2?.toFixed(2)}</span>
          </div>
          {#if bias}
            <div class="flex flex-col gap-1 border-l border-border p-4">
              <span class="hud-label">{t('stats.estimators.biasScore')}</span>
              <span class="hud-readout text-lg uppercase {bias.class}"
                >{scoreLabel(bias.label)}</span
              >
              <span class="text-xs text-muted-foreground tabular-nums">
                {t('stats.estimators.samples', { count: totalSamples })}
              </span>
            </div>
          {/if}
          <div class="flex flex-col gap-1 border-t border-border p-4">
            <span class="hud-label">{t('stats.estimators.avgError')}</span>
            <span class="hud-readout text-lg">±{toVeryCompact(statistic.mae ?? 0)}</span>
            <span class="text-xs text-muted-foreground">{t('stats.estimators.avgErrorHint')}</span>
          </div>
          <div class="flex flex-col gap-1 border-t border-l border-border p-4">
            <span class="hud-label">{t('stats.estimators.lastTrained')}</span>
            <span class="hud-readout text-lg">
              {t('stats.estimators.timeAgo', { time: agoLine(statistic.last_trained_at) })}
            </span>
          </div>
          {#if trainingData.length > 0}
            <div
              class="flex flex-col gap-1 p-4 max-sm:col-span-2 max-sm:border-t sm:col-start-3 sm:row-span-2 sm:row-start-1 sm:border-l border-border"
            >
              <span class="hud-label">{t('stats.estimators.trainingData')}</span>
              <div class="grid grow grid-cols-[1fr_auto] content-start gap-x-3 gap-y-0.5 text-xs">
                {#each trainingData as [typeName, count] (typeName)}
                  <span class="truncate text-muted-foreground">{typeName}</span>
                  <span class="text-right tabular-nums {(count ?? 0) < 10 ? 'text-negative' : ''}">
                    {(count ?? 0).toLocaleString('en-US')}
                  </span>
                {/each}
              </div>
              <a
                class="flex items-center gap-1 text-xs text-primary hover:underline"
                href="/historic-sales/type/{module.type.id}"
              >
                {t('stats.estimators.viewHistoricSales')} →
              </a>
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <!-- The untrained state. Deliberate divergence from the legacy
		     MissingData.vue, which was an unstyled p-2 block with an h1
		     nested under an h2 and 10px body copy: this is the empty
		     state of the page's headline feature, so it wears the same
		     shape as the trained branch and shows progress toward the
		     threshold instead of only naming the shortfall. -->
      <div class="flex grow flex-col gap-2 p-4">
        <h2 class="hud-label">{t('stats.estimators.aiValuePrediction')}</h2>
        <span class="hud-readout text-2xl text-muted-foreground">
          {t('stats.estimators.notEnoughData')}
        </span>

        <div class="mt-1 flex items-center gap-3">
          <div class="h-1 grow overflow-hidden rounded-full bg-primary/20">
            <div
              class="h-full rounded-full bg-primary transition-[width] duration-500"
              style="width: {progress * 100}%"
            ></div>
          </div>
          <span class="shrink-0 text-xs text-muted-foreground tabular-nums">
            {dataCount.toLocaleString('en-US')} / {MINIMUM_TRAINING_TRADES}
          </span>
        </div>

        <p class="text-sm text-muted-foreground">
          {t('stats.estimators.trainingThreshold', { count: MINIMUM_TRAINING_TRADES })}
          {#if remaining > 0}
            {t('stats.estimators.tradesToGo', { count: remaining })}
          {:else}
            {t('stats.estimators.queuedForTraining')}
          {/if}
        </p>

        <a
          class="flex items-center gap-1 text-xs text-primary hover:underline"
          href="/historic-sales/type/{module.type.id}"
        >
          {t('stats.estimators.viewHistoricSales')} →
        </a>
      </div>
    {/if}

    <ModuleToolbar {module} {typeStatistics} />
  </div>
</Tooltip.Provider>
