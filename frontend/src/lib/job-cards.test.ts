import { describe, expect, it } from 'vitest';

import {
  JOB_CARDS,
  JOB_CARD_ORDER,
  defaultJobCard,
  jobBoardOrder,
  jobCard,
  progressFraction,
} from './job-cards';

describe('job card configs', () => {
  it('cover every job exactly once, in the bento order', () => {
    expect([...JOB_CARD_ORDER].sort()).toEqual(Object.keys(JOB_CARDS).sort());
  });
});

describe('jobCard', () => {
  it('serves the designed config when there is one', () => {
    expect(jobCard('region-contracts')).toBe(JOB_CARDS['region-contracts']);
  });

  it('falls back to a readable default, so no job is ever card-less', () => {
    expect(defaultJobCard('discord-member-counts')).toEqual({
      title: 'Discord member counts',
      itemsLabel: 'items',
      size: 'standard',
      description: 'Scheduled background job',
    });
    expect(jobCard('not-designed-yet').title).toBe('Not designed yet');
  });
});

describe('jobBoardOrder', () => {
  it('leads with the designed cards, then the rest alphabetically', () => {
    expect(jobBoardOrder(['og-cache', 'estimates', 'admin-scopes', 'region-contracts'])).toEqual([
      'region-contracts',
      'estimates',
      'admin-scopes',
      'og-cache',
    ]);
  });

  it('places every registered job exactly once', () => {
    // The registry the API serves, including the jobs with no
    // designed card: the board must show all of them.
    const registered = [
      ...JOB_CARD_ORDER,
      'statistics-views',
      'wallet-donations',
      'admin-scopes',
      'premium-expiry',
      'raffle-draw',
      'patreon-subscribers',
      'offer-notifications',
      'notification-delivery',
      'launcher-ads',
      'discord-member-counts',
      'og-cache',
    ];
    const board = jobBoardOrder(registered);
    expect(board).toHaveLength(registered.length);
    expect([...board].sort()).toEqual([...registered].sort());
  });

  it('skips a designed card whose job is not registered', () => {
    expect(jobBoardOrder(['estimates'])).toEqual(['estimates']);
  });
});

describe('progressFraction', () => {
  it('parses the fan-out progress lines', () => {
    expect(progressFraction('region 2/70 (id 10000002): 153 contracts so far')).toBeCloseTo(2 / 70);
    expect(progressFraction('character 3/3 (id 9): 1 modules imported so far')).toBe(1);
    expect(progressFraction('running…')).toBeNull();
    expect(progressFraction(null)).toBeNull();
    expect(progressFraction('weird 4/0 line')).toBeNull();
  });
});
