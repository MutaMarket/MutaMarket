// Shaping of the outgoing-ESI telemetry snapshot into the minute
// columns the console's charts draw. Pure functions, so the pages stay
// thin and the windowing rules are unit-testable.
import type { TelemetryBucket, TelemetrySnapshot } from '$lib/admin-types';
import type { ChartMinute, ChartSeries } from '$lib/components/telemetry-chart.svelte';

/** Minutes shown on the charts (the API keeps the same window). */
export const CHART_WINDOW_MINUTES = 60;

/** The key the endpoints beyond the colored slots fold into. */
export const OTHER_KEY = '__other';

// The accent leads, then distinct partner hues; the gray carries the
// folded "other" tail.
export const ENDPOINT_COLORS = ['#a3e635', '#22d3ee', '#a78bfa', '#f59e0b'];
export const OTHER_COLOR = '#898781';

// Error classes wear the reserved status colors; this stack order
// passes the adjacency gates on this surface.
export const ERROR_SERIES: ChartSeries[] = [
	{ key: 'client_errors', label: '4xx', color: '#ec835a' },
	{ key: 'server_errors', label: '5xx', color: '#d03b3b' },
	{ key: 'transport_errors', label: 'no response', color: '#fab219' },
];

/** Total requests per endpoint across the whole window. */
export function endpointTotals(buckets: TelemetryBucket[]): Map<string, number> {
	const totals = new Map<string, number>();
	for (const bucket of buckets) {
		for (const [endpoint, counts] of Object.entries(bucket.endpoints)) {
			totals.set(endpoint, (totals.get(endpoint) ?? 0) + counts.requests);
		}
	}
	return totals;
}

/**
 * Sticky endpoint -> color slot assignment: color follows the entity, so
 * a poll that reshuffles volumes never repaints existing series. Held
 * slots survive as long as their endpoint is still in the window; freed
 * ones go to the busiest endpoint that has none.
 */
export function assignSlots(previous: string[], totals: Map<string, number>): string[] {
	const kept = previous.filter((endpoint) => totals.has(endpoint));
	const busiest = [...totals.entries()].sort((a, b) => b[1] - a[1]);
	for (const [endpoint] of busiest) {
		if (kept.length >= ENDPOINT_COLORS.length) break;
		if (!kept.includes(endpoint)) kept.push(endpoint);
	}
	return kept;
}

/** The request chart's series: one per held slot, plus the folded tail. */
export function requestSeries(slots: string[], hasOther: boolean): ChartSeries[] {
	const series: ChartSeries[] = slots.map((endpoint, index) => ({
		key: endpoint,
		label: endpoint,
		color: ENDPOINT_COLORS[index],
	}));
	if (hasOther) {
		series.push({ key: OTHER_KEY, label: 'other', color: OTHER_COLOR });
	}
	return series;
}

export interface TelemetryMinutes {
	requests: ChartMinute[];
	errors: ChartMinute[];
}

/**
 * The fixed chart window ending at `minuteNow`, gaps filled with zeros
 * so the columns keep their wall-clock positions when a minute recorded
 * no traffic.
 */
export function chartMinutes(
	snapshot: TelemetrySnapshot,
	slots: string[],
	minuteNow: number,
): TelemetryMinutes {
	const byMinute = new Map(snapshot.buckets.map((bucket) => [bucket.minute_start, bucket]));
	const requests: ChartMinute[] = [];
	const errors: ChartMinute[] = [];

	for (let offset = CHART_WINDOW_MINUTES - 1; offset >= 0; offset -= 1) {
		const minuteStart = minuteNow - offset * 60;
		const bucket = byMinute.get(minuteStart);

		const request: ChartMinute = { minuteStart, values: {} };
		const error: ChartMinute = { minuteStart, values: {} };
		if (bucket) {
			let totalRequests = 0;
			let totalMs = 0;
			for (const [endpoint, counts] of Object.entries(bucket.endpoints)) {
				const key = slots.includes(endpoint) ? endpoint : OTHER_KEY;
				request.values[key] = (request.values[key] ?? 0) + counts.requests;
				for (const errorClass of ERROR_SERIES) {
					error.values[errorClass.key] =
						(error.values[errorClass.key] ?? 0) + counts[errorClass.key as keyof typeof counts];
				}
				totalRequests += counts.requests;
				totalMs += counts.total_ms;
			}
			if (totalRequests > 0) {
				request.detail = `avg ${Math.round(totalMs / totalRequests)} ms`;
			}
		}
		requests.push(request);
		errors.push(error);
	}

	return { requests, errors };
}

export interface HourTotals {
	requests: number;
	errors: number;
	averageMs: number;
	/** The busiest endpoint and its request count, if any traffic. */
	busiest: [string, number] | null;
}

/** The headline readout above the charts, over the whole window. */
export function hourTotals(buckets: TelemetryBucket[], totals: Map<string, number>): HourTotals {
	let requests = 0;
	let errors = 0;
	let totalMs = 0;
	for (const bucket of buckets) {
		for (const counts of Object.values(bucket.endpoints)) {
			requests += counts.requests;
			errors += counts.client_errors + counts.server_errors + counts.transport_errors;
			totalMs += counts.total_ms;
		}
	}
	return {
		requests,
		errors,
		averageMs: requests > 0 ? Math.round(totalMs / requests) : 0,
		busiest: [...totals.entries()].sort((a, b) => b[1] - a[1])[0] ?? null,
	};
}
