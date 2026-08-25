import { describe, expect, it } from 'vitest';

import { humanizeInterval, parseDbTimestamp, relativeTime } from './duration';

describe('humanizeInterval', () => {
	it('names the scheduler cadences', () => {
		expect(humanizeInterval(60)).toBe('every 1 min');
		expect(humanizeInterval(5 * 60)).toBe('every 5 min');
		expect(humanizeInterval(30 * 60)).toBe('every 30 min');
		expect(humanizeInterval(3600)).toBe('hourly');
		expect(humanizeInterval(24 * 3600)).toBe('daily');
	});
});

describe('relativeTime', () => {
	it('renders both directions', () => {
		expect(relativeTime(2)).toBe('just now');
		expect(relativeTime(-30)).toBe('30 s ago');
		expect(relativeTime(-180)).toBe('3 min ago');
		expect(relativeTime(720)).toBe('in 12 min');
		expect(relativeTime(2 * 24 * 3600)).toBe('in 2 d');
	});
});

describe('parseDbTimestamp', () => {
	it('parses the timestamptz text format', () => {
		expect(parseDbTimestamp('1970-01-01 00:01:00+00')).toBe(60);
		expect(parseDbTimestamp('1970-01-01 00:00:01.5+00')).toBe(1.5);
		expect(parseDbTimestamp('nonsense')).toBe(0);
	});
});
