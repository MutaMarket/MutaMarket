// The shared premium config (the legacy AppData props served through
// /api/sidebar) and the yearly-savings computed.

import { describe, expect, it } from 'vitest';

import {
  DEFAULT_PREMIUM,
  adFreeFactor,
  clampGiftDays,
  daysLeft,
  demoCharacter,
  planAmount,
  premiumFromSidebar,
  premiumHolder,
  switchedDays,
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
        premium_ad_free_cost: 150_000_000,
        premium_ad_free_yearly_cost: 1_500_000_000,
      }),
    ).toEqual({
      premium_character: 'Other Mate',
      premium_cost: 50_000_000,
      premium_yearly_cost: 500_000_000,
      premium_ad_free_cost: 150_000_000,
      premium_ad_free_yearly_cost: 1_500_000_000,
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
      yearlySavings({
        premium_character: 'X',
        premium_cost: 10,
        premium_yearly_cost: 100,
        premium_ad_free_cost: 30,
        premium_ad_free_yearly_cost: 300,
      }),
    ).toBe(20);
  });

  it('is two ad-free months on the ad-free plan', () => {
    expect(yearlySavings(DEFAULT_PREMIUM, true)).toBe(600_000_000);
  });
});

describe('planAmount', () => {
  it('asks for the monthly or the yearly price', () => {
    expect(planAmount(DEFAULT_PREMIUM, 'monthly')).toBe(100_000_000);
    expect(planAmount(DEFAULT_PREMIUM, 'yearly')).toBe(1_000_000_000);
  });

  it('asks three times as much for ad-free premium', () => {
    expect(planAmount(DEFAULT_PREMIUM, 'monthly', true)).toBe(300_000_000);
    expect(planAmount(DEFAULT_PREMIUM, 'yearly', true)).toBe(3_000_000_000);
  });
});

describe('plan switching', () => {
  const now = Date.parse('2026-11-01T00:00:00Z');

  it('reads the factor off the price points', () => {
    expect(adFreeFactor(DEFAULT_PREMIUM)).toBe(3);
  });

  it('counts whole days left, none once lapsed', () => {
    expect(daysLeft('2026-12-01T12:00:00Z', now)).toBe(30);
    expect(daysLeft('2026-10-01T00:00:00Z', now)).toBe(0);
    expect(daysLeft(null, now)).toBe(0);
  });

  it('converts the days at their value', () => {
    expect(switchedDays(30, true, 3)).toBe(10);
    expect(switchedDays(10, false, 3)).toBe(30);
    expect(switchedDays(29, true, 3)).toBe(9);
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
    expect(demoCharacter(nav, modules)).toEqual({
      id: 2,
      name: 'Main',
      own: true,
    });
  });

  it('falls back to the first sample creator, or nothing', () => {
    expect(demoCharacter(null, modules)).toEqual({
      id: 7,
      name: 'Roll Smith',
      own: false,
    });
    expect(demoCharacter(null, [])).toBeNull();
  });
});

describe('premiumHolder', () => {
  const entry = (id: number, active: boolean, until: string | null) => ({
    id,
    active,
    premium_paid_until: until,
  });

  it('picks the character whose premium runs longest', () => {
    expect(
      premiumHolder([
        entry(1, true, '2027-01-01T00:00:00Z'),
        entry(2, false, '2030-06-01T00:00:00Z'),
        entry(3, false, null),
      ])?.id,
    ).toBe(2);
  });

  it('falls back to the active character, then the first', () => {
    expect(premiumHolder([entry(1, false, null), entry(2, true, null)])?.id).toBe(2);
    expect(premiumHolder([entry(1, false, null), entry(2, false, null)])?.id).toBe(1);
    expect(premiumHolder([])).toBeNull();
  });
});
