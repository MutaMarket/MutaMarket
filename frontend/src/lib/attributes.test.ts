// Transliterated from the Rust modules::view unit tests so the TS mirror
// emits identical strings.

import { describe, expect, it } from 'vitest';

import {
  formatDecimal,
  formatDifference,
  formatFraction,
  formatNumber,
  formatValue,
  toNormalized,
  toOriginal,
} from './attributes';

describe('number and fraction formatting', () => {
  it('formats compactly', () => {
    expect(formatNumber(241.919996)).toBe('241.92');
    expect(formatNumber(180.0)).toBe('180');
    expect(formatFraction(-0.86)).toBe('-86.0%');
    expect(formatFraction(0.67)).toBe('+67.0%');
  });
});

describe('attribute values', () => {
  it('formats by unit like the legacy formatter', () => {
    // Milliseconds display as seconds.
    expect(formatValue(5000, 'Milliseconds', 's')).toBe('5s');
    expect(formatDifference(4500, 5000, 'Milliseconds', 's')).toBe('-0.5s');

    // Modifier multipliers display as signed percent changes.
    expect(formatValue(1.15, 'Modifier Percent', '%')).toBe('15%');
    expect(formatDifference(1.2, 1.1, 'Modifier Percent', '%')).toBe('+10%');

    // Inverted modifiers: a 0.85 multiplier displays as its 15% bonus,
    // with up to three decimals.
    expect(formatValue(0.85, 'Inversed Modifier Percent', '%')).toBe('15%');

    // Per-millisecond rates display per second.
    expect(formatValue(0.0125, 'Hitpoints/Second', 'HP/s')).toBe('12.5HP/s');
    expect(formatDifference(0.0125, 0.01, 'Hitpoints/Second', 'HP/s')).toBe('+2.5HP/s');

    // Multipliers carry three decimals and no suffix on differences.
    expect(formatValue(1.2345678, 'Multiplier', 'x')).toBe('1.235x');
    expect(formatDifference(1.235, 1.2, 'Multiplier', 'x')).toBe('+0.035');

    // Unknown units fall back to the raw value plus display name.
    expect(formatValue(250, 'Meters', 'm')).toBe('250m');
    expect(formatValue(42.5, null, null)).toBe('42.5');
  });
});

describe('slider normalization', () => {
  it('maps between worst and best', () => {
    // High is good: worst 100, best 200.
    expect(toNormalized(100, 200, 100)).toBe(0);
    expect(toNormalized(200, 200, 100)).toBe(100);
    expect(toNormalized(150, 200, 100)).toBe(50);
    expect(toOriginal(50, 200, 100)).toBe(150);

    // Low is good: worst 200, best 100 - direction handled by the map.
    expect(toNormalized(200, 100, 200)).toBe(0);
    expect(toNormalized(100, 100, 200)).toBe(100);
    expect(toOriginal(100, 100, 200)).toBe(100);
  });
});

describe('formatDecimal', () => {
  it('mirrors the legacy table formatter: 4 significant digits with grouping', () => {
    // Grouped en-US at 4 significant digits (the legacy intlDecimal).
    expect(formatDecimal(123456, null, null)).toBe('123,500');
    expect(formatDecimal(1234.5678, null, 'GJ')).toBe('1,235GJ');
    expect(formatDecimal(0.0123456, null, null)).toBe('0.01235');

    // Display transforms still apply before formatting.
    expect(formatDecimal(12345, 'Milliseconds', 's')).toBe('12.35s');
    expect(formatDecimal(1.2345678, 'Modifier Percent', '%')).toBe('23.46%');
    expect(formatDecimal(0.85, 'Inversed Modifier Percent', '%')).toBe('15%');
    expect(formatDecimal(0.0125, 'Hitpoints/Second', 'HP/s')).toBe('12.5HP/s');
    expect(formatDecimal(1.2345678, 'Multiplier', 'x')).toBe('1.235x');
  });
});
