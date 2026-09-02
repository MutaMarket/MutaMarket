// Per-job presentation of the operations console bento grid: what each
// job's headline metric means, and how much room its card deserves.
import { t } from '$lib/i18n.svelte';

export interface JobCardSeries {
  /** Key into a run's recorded `metrics`. */
  key: string;
  label: string;
  color: string;
}

export interface JobCardConfig {
  /** Card heading, the job's plain-language name. */
  title: string;
  /** What the recorded `items` metric counts. */
  itemsLabel: string;
  /** Bento footprint: wide cards span two columns. */
  size: 'wide' | 'standard';
  /** One line on what the job does (the title's tooltip). */
  description: string;
  /** Per-run sub-metric lines (same unit); without them the card
   * shows the work-per-run columns. */
  series?: JobCardSeries[];
}

/** The designed cards, with translation keys where the resolved card
 * carries text; `jobCard` resolves them at call time. */
export const JOB_CARDS: Record<string, JobCardConfig> = {
  'region-contracts': {
    title: 'admin.jobs.cards.regionContracts.title',
    itemsLabel: 'admin.jobs.cards.regionContracts.itemsLabel',
    size: 'wide',
    description: 'admin.jobs.cards.regionContracts.description',
    series: [
      { key: 'new', label: 'admin.jobs.series.new', color: '#a3e635' },
      { key: 'invalidated', label: 'admin.jobs.series.invalidated', color: '#d95926' },
    ],
  },
  'character-assets': {
    title: 'admin.jobs.cards.characterAssets.title',
    itemsLabel: 'admin.jobs.cards.characterAssets.itemsLabel',
    size: 'wide',
    description: 'admin.jobs.cards.characterAssets.description',
    series: [
      { key: 'found', label: 'admin.jobs.series.found', color: '#22d3ee' },
      { key: 'imported', label: 'admin.jobs.series.imported', color: '#a3e635' },
      { key: 'failed', label: 'admin.jobs.series.failed', color: '#d03b3b' },
    ],
  },
  'character-contracts': {
    title: 'admin.jobs.cards.characterContracts.title',
    itemsLabel: 'admin.jobs.cards.characterContracts.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.characterContracts.description',
  },
  estimates: {
    title: 'admin.jobs.cards.estimates.title',
    itemsLabel: 'admin.jobs.cards.estimates.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.estimates.description',
  },
  'auction-bids': {
    title: 'admin.jobs.cards.auctionBids.title',
    itemsLabel: 'admin.jobs.cards.auctionBids.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.auctionBids.description',
  },
  'market-histories': {
    title: 'admin.jobs.cards.marketHistories.title',
    itemsLabel: 'admin.jobs.cards.marketHistories.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.marketHistories.description',
  },
  'character-names': {
    title: 'admin.jobs.cards.characterNames.title',
    itemsLabel: 'admin.jobs.cards.characterNames.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.characterNames.description',
  },
  'stale-asset-imports': {
    title: 'admin.jobs.cards.staleAssetImports.title',
    itemsLabel: 'admin.jobs.cards.staleAssetImports.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.staleAssetImports.description',
  },
  structures: {
    title: 'admin.jobs.cards.structures.title',
    itemsLabel: 'admin.jobs.cards.structures.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.structures.description',
  },
  alliances: {
    title: 'admin.jobs.cards.alliances.title',
    itemsLabel: 'admin.jobs.cards.alliances.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.alliances.description',
  },
  'training-modules': {
    title: 'admin.jobs.cards.trainingModules.title',
    itemsLabel: 'admin.jobs.cards.trainingModules.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.trainingModules.description',
  },
  'estimator-training': {
    title: 'admin.jobs.cards.estimatorTraining.title',
    itemsLabel: 'admin.jobs.cards.estimatorTraining.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.estimatorTraining.description',
  },
  'eve-mails': {
    title: 'admin.jobs.cards.eveMails.title',
    itemsLabel: 'admin.jobs.cards.eveMails.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.eveMails.description',
  },
  'metric-samples': {
    title: 'admin.jobs.cards.metricSamples.title',
    itemsLabel: 'admin.jobs.cards.metricSamples.itemsLabel',
    size: 'standard',
    description: 'admin.jobs.cards.metricSamples.description',
  },
};

/** The bento order: heavy movers first. */
export const JOB_CARD_ORDER = [
  'region-contracts',
  'character-assets',
  'character-contracts',
  'estimates',
  'auction-bids',
  'market-histories',
  'character-names',
  'stale-asset-imports',
  'structures',
  'alliances',
  'eve-mails',
  'training-modules',
  'estimator-training',
  'metric-samples',
];

/**
 * The card for a job with no designed entry above. Every registered job
 * gets a place on the board: the console used to render only the jobs
 * named in `JOB_CARD_ORDER`, so a newly registered one was invisible
 * until someone remembered to design its card.
 */
export function defaultJobCard(name: string): JobCardConfig {
  return {
    title: name.replace(/-/g, ' ').replace(/^./, (first) => first.toUpperCase()),
    itemsLabel: t('admin.jobs.cards.default.itemsLabel'),
    size: 'standard',
    description: t('admin.jobs.cards.default.description'),
  };
}

/** The designed card with its keys resolved in the current locale. */
function localize(config: JobCardConfig): JobCardConfig {
  return {
    ...config,
    title: t(config.title),
    itemsLabel: t(config.itemsLabel),
    description: t(config.description),
    ...(config.series
      ? { series: config.series.map((series) => ({ ...series, label: t(series.label) })) }
      : {}),
  };
}

export function jobCard(name: string): JobCardConfig {
  const designed = JOB_CARDS[name];
  return designed ? localize(designed) : defaultJobCard(name);
}

/** Designed cards in their bento order, then the rest alphabetically. */
export function jobBoardOrder(names: string[]): string[] {
  const designed = JOB_CARD_ORDER.filter((name) => names.includes(name));
  const rest = names.filter((name) => !JOB_CARD_ORDER.includes(name)).sort();
  return [...designed, ...rest];
}

/** A "region 2/70" style progress line yields a live meter fraction. */
export function progressFraction(progress: string | null): number | null {
  if (progress === null) return null;
  const match = /(\d+)\/(\d+)/.exec(progress);
  if (!match) return null;
  const total = Number(match[2]);
  return total > 0 ? Math.min(Number(match[1]) / total, 1) : null;
}
