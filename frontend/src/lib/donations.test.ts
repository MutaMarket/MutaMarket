import { describe, expect, it } from 'vitest';

import { donationCountLabel, isRepeatDonor, rankGradient } from './donations';

describe('donation display helpers', () => {
  it('gradients cover exactly the podium', () => {
    expect(rankGradient(1)).toBe('from-yellow-400 to-amber-600');
    expect(rankGradient(2)).toBe('from-slate-300 to-slate-500');
    expect(rankGradient(3)).toBe('from-amber-600 to-orange-700');
    expect(rankGradient(4)).toBeNull();
    expect(rankGradient(10)).toBeNull();
  });

  it('pluralizes the repeat-donor tooltip like the legacy i18n rule', () => {
    expect(donationCountLabel(1)).toBe('1 donation');
    expect(donationCountLabel(4)).toBe('4 donations');
  });

  it('marks repeat donors only past one donation', () => {
    expect(isRepeatDonor(undefined)).toBe(false);
    expect(isRepeatDonor(0)).toBe(false);
    expect(isRepeatDonor(1)).toBe(false);
    expect(isRepeatDonor(2)).toBe(true);
  });
});
