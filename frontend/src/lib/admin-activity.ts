// Shaping of the activity report into the console's charts. Pure
// functions, so the bucketing rules are unit-testable and the page stays
// thin.
import type { ActivityHistory } from '$lib/admin-types';
import type { ChartMinute, ChartSeries } from '$lib/components/telemetry-chart.svelte';

export const ACTIVITY_WINDOWS = ['24h', '7d', '30d'] as const;
export type ActivityWindow = (typeof ACTIVITY_WINDOWS)[number];

/** Signed-in leads in the accent; anonymous carries the muted gray, the
 * same pairing the telemetry chart uses for its folded tail. */
export const TRAFFIC_SERIES: ChartSeries[] = [
	{ key: 'signed_in', label: 'signed in', color: '#a3e635' },
	{ key: 'anonymous', label: 'anonymous', color: '#898781' }
];

/** Returning leads: it is the larger, steadier band. */
export const COHORT_SERIES: ChartSeries[] = [
	{ key: 'returning_users', label: 'returning', color: '#a3e635' },
	{ key: 'new_users', label: 'new', color: '#22d3ee' }
];

export const USERS_SERIES: ChartSeries[] = [
	{ key: 'users', label: 'active users', color: '#22d3ee' }
];

/**
 * The traffic buckets as chart columns, gap-filled with zeros back from
 * `endsAt` so a quiet bucket keeps its wall-clock position instead of
 * closing the gap.
 */
export function trafficBuckets(
	traffic: ActivityHistory['traffic'],
	stepSeconds: number,
	endsAt: number,
	count: number
): ChartMinute[] {
	const byAt = new Map(traffic.map((point) => [point.at, point]));
	const last = Math.floor(endsAt / stepSeconds) * stepSeconds;
	const buckets: ChartMinute[] = [];

	for (let index = count - 1; index >= 0; index -= 1) {
		const at = last - index * stepSeconds;
		const point = byAt.get(at);
		buckets.push({
			minuteStart: at,
			values: point
				? { signed_in: point.signed_in, anonymous: point.anonymous }
				: { signed_in: 0, anonymous: 0 }
		});
	}
	return buckets;
}

/** Distinct users per day as chart columns, gap-filled. */
export function userBuckets(
	daily: ActivityHistory['daily_users'],
	days: number,
	endsAt: number
): ChartMinute[] {
	const byDay = new Map(daily.map((entry) => [entry.day, entry]));
	const buckets: ChartMinute[] = [];

	for (let index = days - 1; index >= 0; index -= 1) {
		const at = Math.floor(endsAt / 86_400) * 86_400 - index * 86_400;
		const day = new Date(at * 1000).toISOString().slice(0, 10);
		buckets.push({
			minuteStart: at,
			values: { users: byDay.get(day)?.users ?? 0 }
		});
	}
	return buckets;
}

/** The monthly cohorts as chart columns, keyed by the month's first day. */
export function cohortBuckets(months: ActivityHistory['months']): ChartMinute[] {
	return months.map((month) => ({
		minuteStart: Math.floor(Date.parse(`${month.month}-01T00:00:00Z`) / 1000),
		values: {
			returning_users: month.returning_users,
			new_users: month.new_users
		},
		detail: `${month.signed_up} signed up`
	}));
}

/** The axis label a bucket of this width wants. */
export function bucketLabel(stepSeconds: number): (at: number) => string {
	if (stepSeconds < 86_400) {
		return (at) => {
			const date = new Date(at * 1000);
			return `${String(date.getUTCHours()).padStart(2, '0')}:${String(date.getUTCMinutes()).padStart(2, '0')}`;
		};
	}
	if (stepSeconds < 2_592_000) {
		return (at) => {
			const date = new Date(at * 1000);
			return `${String(date.getUTCDate()).padStart(2, '0')}.${String(date.getUTCMonth() + 1).padStart(2, '0')}`;
		};
	}
	return (at) =>
		new Date(at * 1000).toLocaleDateString('en-US', {
			month: 'short',
			year: '2-digit',
			timeZone: 'UTC'
		});
}

/**
 * Which buckets get a tick. An index predicate rather than a seconds
 * modulus, because months are not a fixed number of seconds apart.
 */
export function tickPredicate(count: number, wanted = 8): (at: number, index: number) => boolean {
	const every = Math.max(Math.ceil(count / wanted), 1);
	return (_at, index) => index % every === 0;
}

/** The signed-in share of traffic, as a whole percent. */
export function signedInShare(totals: ActivityHistory['totals']): number | null {
	if (totals.requests <= 0) return null;
	return (totals.signed_in_requests * 100) / totals.requests;
}

/** Requests per active user over the window. */
export function requestsPerUser(totals: ActivityHistory['totals']): number | null {
	if (totals.active_users <= 0) return null;
	return totals.signed_in_requests / totals.active_users;
}
