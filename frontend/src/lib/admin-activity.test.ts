import { describe, expect, it } from 'vitest';

import {
	bucketLabel,
	cohortBuckets,
	requestsPerUser,
	signedInShare,
	tickPredicate,
	trafficBuckets,
	userBuckets
} from './admin-activity';
import type { ActivityHistory } from './admin-types';

const HOUR = 3_600;
const DAY = 86_400;

describe('trafficBuckets', () => {
	const endsAt = 10 * HOUR;

	it('gap-fills a quiet bucket so columns keep their position', () => {
		const buckets = trafficBuckets(
			[{ at: 10 * HOUR, signed_in: 3, anonymous: 7 }],
			HOUR,
			endsAt,
			3
		);
		expect(buckets.map((b) => b.minuteStart)).toEqual([8 * HOUR, 9 * HOUR, 10 * HOUR]);
		expect(buckets[0].values).toEqual({ signed_in: 0, anonymous: 0 });
		expect(buckets[2].values).toEqual({ signed_in: 3, anonymous: 7 });
	});

	it('fills a hole in the middle, not just the edges', () => {
		const buckets = trafficBuckets(
			[
				{ at: 8 * HOUR, signed_in: 1, anonymous: 1 },
				{ at: 10 * HOUR, signed_in: 2, anonymous: 2 }
			],
			HOUR,
			endsAt,
			3
		);
		expect(buckets.map((b) => b.values.signed_in)).toEqual([1, 0, 2]);
	});

	it('drops a bucket that fell out of the window', () => {
		const buckets = trafficBuckets(
			[{ at: 2 * HOUR, signed_in: 9, anonymous: 9 }],
			HOUR,
			endsAt,
			3
		);
		expect(buckets.every((b) => b.values.signed_in === 0)).toBe(true);
	});
});

describe('userBuckets', () => {
	it('lines the day strings up with their bucket start', () => {
		const endsAt = Date.parse('2026-08-30T12:00:00Z') / 1000;
		const buckets = userBuckets(
			[
				{ day: '2026-08-30', users: 12, requests: 100 },
				{ day: '2026-08-28', users: 4, requests: 40 }
			],
			3,
			endsAt
		);
		expect(buckets.map((b) => b.values.users)).toEqual([4, 0, 12]);
	});
});

describe('cohortBuckets', () => {
	it('keys each month by its first day and carries the sign-ups', () => {
		const months: ActivityHistory['months'] = [
			{ month: '2026-07', active_users: 10, new_users: 3, returning_users: 7, signed_up: 5 }
		];
		const buckets = cohortBuckets(months);
		expect(buckets[0].minuteStart).toBe(Date.parse('2026-07-01T00:00:00Z') / 1000);
		expect(buckets[0].values).toEqual({ returning_users: 7, new_users: 3 });
		expect(buckets[0].detail).toBe('5 signed up');
	});
});

describe('bucketLabel', () => {
	const at = Date.parse('2026-08-30T14:05:00Z') / 1000;

	it('picks the precision the bucket width deserves', () => {
		expect(bucketLabel(HOUR)(at)).toBe('14:05');
		expect(bucketLabel(DAY)(at)).toBe('30.08');
		expect(bucketLabel(DAY * 30)(at)).toBe('Aug 26');
	});
});

describe('tickPredicate', () => {
	it('thins the ticks toward the wanted count', () => {
		const predicate = tickPredicate(24, 8);
		const ticked = Array.from({ length: 24 }, (_, i) => predicate(0, i)).filter(Boolean);
		expect(ticked).toHaveLength(8);
	});

	it('ticks every bucket when there are fewer than wanted', () => {
		const predicate = tickPredicate(4, 8);
		expect(Array.from({ length: 4 }, (_, i) => predicate(0, i))).toEqual([
			true,
			true,
			true,
			true
		]);
	});
});

describe('totals', () => {
	const totals = (overrides: Partial<ActivityHistory['totals']> = {}) => ({
		requests: 1000,
		signed_in_requests: 250,
		page_views: 200,
		active_users: 50,
		new_users: 4,
		...overrides
	});

	it('reports the signed-in share and per-user load', () => {
		expect(signedInShare(totals())).toBe(25);
		expect(requestsPerUser(totals())).toBe(5);
	});

	it('has nothing to divide by on a silent window', () => {
		expect(signedInShare(totals({ requests: 0 }))).toBeNull();
		expect(requestsPerUser(totals({ active_users: 0 }))).toBeNull();
	});
});
