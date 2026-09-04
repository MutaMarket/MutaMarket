// The shared premium config (the legacy AppData props served through
// /api/sidebar) and the yearly-savings computed.

import { describe, expect, it } from 'vitest';

import {
  DEFAULT_PREMIUM,
  clampGiftDays,
  demoCharacter,
  planAmount,
  premiumFromSidebar,
  yearlySavings,
} from './premium';
import type { ModuleDetail, NavState } from './types';

describe('premiumFromSidebar', () => {
  it('reads the payload values', () => {
    expect(
      premiumFromSidebar({
        premium_character: 'Other Mate',
        premium_cost: 50_000_000,
        premium_yearly_cost: 500_000_000,
      }),
    ).toEqual({
      premium_character: 'Other Mate',
      premium_cost: 50_000_000,
      premium_yearly_cost: 500_000_000,
    });
  });

  it('degrades to the backend defaults without a payload', () => {
    expect(premiumFromSidebar(null)).toEqual(DEFAULT_PREMIUM);
    expect(premiumFromSidebar(undefined)).toEqual(DEFAULT_PREMIUM);
    expect(premiumFromSidebar({})).toEqual(DEFAULT_PREMIUM);
  });
});

describe('yearlySavings', () => {
  it('is two free months on the defaults', () => {
    expect(yearlySavings(DEFAULT_PREMIUM)).toBe(200_000_000);
  });

  it('follows the configured prices', () => {
    expect(
      yearlySavings({ premium_character: 'X', premium_cost: 10, premium_yearly_cost: 100 }),
    ).toBe(20);
  });
});

describe('planAmount', () => {
  it('asks for the monthly or the yearly price', () => {
    expect(planAmount(DEFAULT_PREMIUM, 'monthly')).toBe(100_000_000);
    expect(planAmount(DEFAULT_PREMIUM, 'yearly')).toBe(1_000_000_000);
  });
});

describe('clampGiftDays', () => {
  it('keeps the days whole and within the donor balance', () => {
    expect(clampGiftDays(5, 29)).toBe(5);
    expect(clampGiftDays(40, 29)).toBe(29);
    expect(clampGiftDays(0, 29)).toBe(1);
    expect(clampGiftDays(2.9, 29)).toBe(2);
    expect(clampGiftDays(Number.NaN, 29)).toBe(1);
    expect(clampGiftDays(3, 0)).toBe(1);
  });
});

describe('demoCharacter', () => {
  const creator = { id: 7, name: 'Roll Smith' } as ModuleDetail['creator'];
  const modules = [{ creator: null }, { creator }] as ModuleDetail[];

  it("prefers the visitor's active character", () => {
    const nav = {
      user: { active_character_id: 2 },
      characters: [
        { id: 1, name: 'Alt' },
        { id: 2, name: 'Main' },
      ],
    } as NavState;
    expect(demoCharacter(nav, modules)).toEqual({ id: 2, name: 'Main', own: true });
  });

  it('falls back to the first sample creator, or nothing', () => {
    expect(demoCharacter(null, modules)).toEqual({ id: 7, name: 'Roll Smith', own: false });
    expect(demoCharacter(null, [])).toBeNull();
  });
});
