import { describe, expect, it } from 'vitest';
import {
  attributeToNormalized,
  attributeToOriginal,
  clamp,
  currencyToNormalized,
  currencyToOriginal,
} from './slider-scale';

describe('attribute scale', () => {
  it('maps worst to 0 and best to 100, inverted ranges included', () => {
    // High-is-good attribute: worst 400, best 600.
    expect(attributeToNormalized(400, 600, 400)).toBe(0);
    expect(attributeToNormalized(600, 600, 400)).toBe(100);
    expect(attributeToNormalized(500, 600, 400)).toBe(50);
    // Low-is-good attribute: best 100, worst 300 - position still
    // runs worst to best.
    expect(attributeToNormalized(300, 100, 300)).toBe(0);
    expect(attributeToNormalized(100, 100, 300)).toBe(100);
  });

  it('round-trips through the inverse', () => {
    const back = attributeToOriginal(attributeToNormalized(512.5, 600, 400), 600, 400);
    expect(back).toBeCloseTo(512.5, 9);
  });
});

describe('currency scale', () => {
  const LOWEST = 1_000_000;
  const HIGHEST = 100_000_000_000;

  it('is logarithmic with fixed endpoints', () => {
    expect(currencyToNormalized(LOWEST, LOWEST, HIGHEST)).toBeCloseTo(0, 9);
    expect(currencyToNormalized(HIGHEST, LOWEST, HIGHEST)).toBeCloseTo(100, 9);
    // 1B sits at 60% of the 1M..100B log range (3 of 5 decades).
    expect(currencyToNormalized(1_000_000_000, LOWEST, HIGHEST)).toBeCloseTo(60, 2);
  });

  it('round-trips through the inverse', () => {
    const back = currencyToOriginal(
      currencyToNormalized(275_000_000, LOWEST, HIGHEST),
      LOWEST,
      HIGHEST,
    );
    expect(back).toBeCloseTo(275_000_000, 3);
  });
});

describe('clamp', () => {
  it('bounds URL positions into the slider domain', () => {
    expect(clamp(-3, 0, 100)).toBe(0);
    expect(clamp(140, 0, 100)).toBe(100);
    expect(clamp(55, 0, 100)).toBe(55);
  });
});
