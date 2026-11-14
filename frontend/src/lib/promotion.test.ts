import { describe, expect, it } from 'vitest';

import {
  DEFAULT_DAILY_COST,
  MAX_DIALOG_DAYS,
  canPromote,
  clampDays,
  dailyCostFromSidebar,
  promotedUntil,
  promotionReason,
} from './promotion';
import type { ModuleDetail } from './types';

const module = (overrides: Partial<ModuleDetail>): ModuleDetail =>
  ({ id: 1047593018621, contract: null, public_asset: null, ...overrides }) as ModuleDetail;

describe('canPromote', () => {
  it('needs a contract or a sell listing', () => {
    expect(canPromote(module({}))).toBe(false);
    expect(canPromote(module({ contract: {} as ModuleDetail['contract'] }))).toBe(true);
    expect(canPromote(module({ public_asset: {} as ModuleDetail['public_asset'] }))).toBe(true);
  });
});

describe('promotionReason', () => {
  it('is the keyword and the item id', () => {
    expect(promotionReason(module({}))).toBe('promote 1047593018621');
  });
});

describe('dailyCostFromSidebar', () => {
  it('reads the payload and degrades to the backend default', () => {
    expect(dailyCostFromSidebar({ promotion_daily_cost: 40_000_000 })).toBe(40_000_000);
    expect(dailyCostFromSidebar({})).toBe(DEFAULT_DAILY_COST);
    expect(dailyCostFromSidebar(null)).toBe(DEFAULT_DAILY_COST);
  });
});

describe('clampDays', () => {
  it('keeps whole days within the range', () => {
    expect(clampDays('7')).toBe(7);
    expect(clampDays('3.9')).toBe(3);
    expect(clampDays('0')).toBe(1);
    expect(clampDays('-4')).toBe(1);
    expect(clampDays('abc')).toBe(1);
    expect(clampDays('')).toBe(1);
    expect(clampDays('9999')).toBe(MAX_DIALOG_DAYS);
  });
});

describe('promotedUntil', () => {
  const now = Date.parse('2026-11-14T12:00:00Z');

  it('is the end of a running promotion', () => {
    const until = promotedUntil(module({ promoted_until: '2026-11-17T12:00:00Z' }), now);
    expect(until?.toISOString()).toBe('2026-11-17T12:00:00.000Z');
  });

  it('is null without one or once it lapsed', () => {
    expect(promotedUntil(module({}), now)).toBeNull();
    expect(promotedUntil(module({ promoted_until: '2026-11-14T11:59:59Z' }), now)).toBeNull();
  });
});
