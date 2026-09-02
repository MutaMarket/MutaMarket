// The estimator quality scores of the show-page hero, ported from the
// legacy Composables/useEstimatorStatistics.ts.

/** Star value 1..5 quantized to halves, like the legacy getStarsArray. */
export function starsValue(value: number): number {
  const clamped = Math.min(Math.max(value, 0), 1);
  const stars = clamped * 4 + 1;
  const full = Math.floor(stars);
  const hasHalf = Math.round(stars * 2) / 2 > full;
  return Math.min(full + (hasHalf ? 0.5 : 0), 5);
}

/**
 * How evenly the training data spreads over source types: normalized
 * Shannon entropy of the sample counts, scaled by a total-count factor
 * (0 below 10 samples, 1 above 100, linear between).
 */
export function biasScore(dataCounts: Record<string, number>): number {
  const MIN_TOTAL = 10;
  const MAX_TOTAL = 100;

  const counts = Object.values(dataCounts);
  const total = counts.reduce((sum, count) => sum + count, 0);
  if (total === 0) {
    return 0;
  }

  const proportions = counts.map((count) => count / total);
  const entropy = -proportions.reduce((sum, p) => sum + (p > 0 ? p * Math.log2(p) : 0), 0);
  const maxEntropy = Math.log2(counts.length);
  const normalized = maxEntropy > 0 ? entropy / maxEntropy : 0;

  const totalFactor =
    total < MIN_TOTAL ? 0 : total > MAX_TOTAL ? 1 : (total - MIN_TOTAL) / (MAX_TOTAL - MIN_TOTAL);

  return normalized * totalFactor;
}

export interface ScoreWord {
  label: string;
  class: string;
}

/** The word ladder of the hero's confidence/bias cells. */
export function scoreWord(stars: number): ScoreWord {
  if (stars >= 4.5) return { label: 'Very high', class: 'text-positive' };
  if (stars >= 3.5) return { label: 'High', class: 'text-positive' };
  if (stars >= 2.5) return { label: 'Moderate', class: 'text-primary' };
  if (stars >= 1.5) return { label: 'Low', class: 'text-negative' };
  return { label: 'Very low', class: 'text-negative' };
}

/**
 * Recorded trades a module type needs before its estimator is trained.
 * The authority is `MINIMUM_DATA_COUNT` in src/estimator/training.rs; the
 * show page only reads it to explain the shortfall, so it is mirrored
 * here rather than widening the module-show JSON contract with a
 * constant that never varies per module.
 */
export const MINIMUM_TRAINING_TRADES = 50;

/** Progress toward `MINIMUM_TRAINING_TRADES`, clamped to 0..1. */
export function trainingProgress(dataCount: number): number {
  if (!Number.isFinite(dataCount) || dataCount <= 0) return 0;
  return Math.min(dataCount / MINIMUM_TRAINING_TRADES, 1);
}

/** Trades still missing before the model can be trained. */
export function tradesRemaining(dataCount: number): number {
  if (!Number.isFinite(dataCount) || dataCount <= 0) return MINIMUM_TRAINING_TRADES;
  return Math.max(MINIMUM_TRAINING_TRADES - Math.floor(dataCount), 0);
}
