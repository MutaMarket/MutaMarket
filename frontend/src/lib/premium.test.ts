// The shared premium config (the legacy AppData props served through
// /api/sidebar) and the yearly-savings computed.

import { describe, expect, it } from 'vitest';

import { DEFAULT_PREMIUM, premiumFromSidebar, yearlySavings } from './premium';

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
