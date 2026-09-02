import { describe, expect, it } from 'vitest';

import {
  MINIMUM_TRAINING_TRADES,
  biasScore,
  scoreWord,
  starsValue,
  tradesRemaining,
  trainingProgress,
} from './estimator-score';

describe('starsValue', () => {
  it('quantizes 0..1 onto the 1..5 half-star scale', () => {
    expect(starsValue(0)).toBe(1);
    expect(starsValue(1)).toBe(5);
    expect(starsValue(0.5)).toBe(3);
    expect(starsValue(0.87)).toBe(4.5);
    expect(starsValue(-2)).toBe(1);
  });
});

describe('biasScore', () => {
  it('rewards even spreads with enough samples', () => {
    expect(biasScore({})).toBe(0);
    expect(biasScore({ a: 3, b: 3 })).toBe(0); // under the 10-sample floor
    expect(biasScore({ a: 100, b: 100 })).toBe(1); // even and plentiful
    expect(biasScore({ a: 200 })).toBe(0); // single source: zero entropy
    const skewed = biasScore({ a: 190, b: 10 });
    expect(skewed).toBeGreaterThan(0);
    expect(skewed).toBeLessThan(0.5);
  });
});

describe('scoreWord', () => {
  it('walks the legacy ladder', () => {
    expect(scoreWord(5).label).toBe('Very high');
    expect(scoreWord(4).label).toBe('High');
    expect(scoreWord(3)).toEqual({ label: 'Moderate', class: 'text-primary' });
    expect(scoreWord(2).label).toBe('Low');
    expect(scoreWord(1).class).toBe('text-negative');
  });
});

describe('training progress', () => {
  it('fills the meter proportionally up to the threshold', () => {
    expect(trainingProgress(0)).toBe(0);
    expect(trainingProgress(25)).toBe(0.5);
    expect(trainingProgress(MINIMUM_TRAINING_TRADES)).toBe(1);
  });

  it('clamps a count past the threshold and a nonsensical one', () => {
    // A type can sit above the threshold and still be untrained: the
    // job runs on its own cadence, so the meter must not overflow.
    expect(trainingProgress(9999)).toBe(1);
    expect(trainingProgress(-5)).toBe(0);
    expect(trainingProgress(Number.NaN)).toBe(0);
  });

  it('counts down the trades still missing', () => {
    expect(tradesRemaining(0)).toBe(MINIMUM_TRAINING_TRADES);
    expect(tradesRemaining(12)).toBe(38);
    expect(tradesRemaining(MINIMUM_TRAINING_TRADES)).toBe(0);
    expect(tradesRemaining(9999)).toBe(0);
    expect(tradesRemaining(Number.NaN)).toBe(MINIMUM_TRAINING_TRADES);
  });
});
