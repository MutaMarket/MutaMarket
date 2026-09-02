import { describe, expect, it } from 'vitest';

import { durationLabel, humanizeInterval, parseDbTimestamp, relativeTime } from './duration';
import { t } from './i18n.svelte';

const every = (unit: string, count: number) =>
  t('misc.duration.every', { time: t(`misc.duration.${unit}`, { count }) });

describe('humanizeInterval', () => {
  it('names the scheduler cadences', () => {
    expect(humanizeInterval(60)).toBe(every('minutes', 1));
    expect(humanizeInterval(5 * 60)).toBe(every('minutes', 5));
    expect(humanizeInterval(30 * 60)).toBe(every('minutes', 30));
    expect(humanizeInterval(3600)).toBe(t('misc.duration.hourly'));
    expect(humanizeInterval(24 * 3600)).toBe(t('misc.duration.daily'));
    expect(humanizeInterval(7 * 24 * 3600)).toBe(t('misc.duration.weekly'));
    expect(humanizeInterval(2 * 24 * 3600)).toBe(every('days', 2));
  });
});

describe('relativeTime', () => {
  it('renders both directions', () => {
    expect(relativeTime(2)).toBe(t('misc.duration.justNow'));
    expect(relativeTime(-30)).toBe(t('misc.duration.ago', { time: durationLabel(30) }));
    expect(relativeTime(-180)).toBe(t('misc.duration.ago', { time: durationLabel(180) }));
    expect(relativeTime(720)).toBe(t('misc.duration.in', { time: durationLabel(720) }));
    expect(relativeTime(2 * 24 * 3600)).toBe(
      t('misc.duration.in', { time: durationLabel(2 * 24 * 3600) }),
    );
  });
});

describe('durationLabel', () => {
  it('picks the coarsest unit', () => {
    expect(durationLabel(30)).toBe(t('misc.duration.seconds', { count: 30 }));
    expect(durationLabel(-180)).toBe(t('misc.duration.minutes', { count: 3 }));
    expect(durationLabel(2 * 3600)).toBe(t('misc.duration.hours', { count: 2 }));
    expect(durationLabel(5 * 24 * 3600)).toBe(t('misc.duration.days', { count: 5 }));
  });
});

describe('parseDbTimestamp', () => {
  it('parses the timestamptz text format', () => {
    expect(parseDbTimestamp('1970-01-01 00:01:00+00')).toBe(60);
    expect(parseDbTimestamp('1970-01-01 00:00:01.5+00')).toBe(1.5);
    expect(parseDbTimestamp('nonsense')).toBe(0);
  });
});
