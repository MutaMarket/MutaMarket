import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';

export interface SchedulerRun {
	started_at: string;
	finished_at: string | null;
	outcome: string | null;
	summary: string | null;
	error: string | null;
}

export interface SchedulerJob {
	name: string;
	interval_seconds: number;
	downtime_guarded: boolean;
	paused: boolean;
	running: boolean;
	/** Unix seconds of the next scheduled tick; null while loops are off. */
	next_run_at: number | null;
	/** The in-flight run's live progress line, if it reported one. */
	progress: string | null;
	last_runs: SchedulerRun[];
}

export interface DatabaseCounts {
	modules: number;
	modules_without_estimate: number;
	contracts: number;
	contract_items: number;
	characters: number;
	users: number;
	assets: number;
	public_ownerships: number;
	market_history_days: number;
}

export interface SchedulerStatus {
	enabled: boolean;
	in_downtime: boolean;
	database: DatabaseCounts;
	jobs: SchedulerJob[];
}

export interface TelemetryCounts {
	requests: number;
	success: number;
	client_errors: number;
	server_errors: number;
	transport_errors: number;
	total_ms: number;
}

export interface TelemetryBucket {
	/** Unix seconds of the minute's start. */
	minute_start: number;
	endpoints: Record<string, TelemetryCounts>;
}

export interface TelemetrySnapshot {
	window_minutes: number;
	buckets: TelemetryBucket[];
}

// Guests land on the login page (401), non-admins on the 403 error page.
export const load: PageServerLoad = async ({ fetch }) => {
	const [status, telemetry] = await Promise.all([
		apiGet<SchedulerStatus>(fetch, '/api/admin/scheduler'),
		apiGet<TelemetrySnapshot>(fetch, '/api/admin/telemetry')
	]);

	return { status, telemetry };
};
