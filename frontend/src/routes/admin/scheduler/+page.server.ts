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
	last_runs: SchedulerRun[];
}

export interface SchedulerStatus {
	enabled: boolean;
	in_downtime: boolean;
	jobs: SchedulerJob[];
}

// Guests land on the login page (401), non-admins on the 403 error page.
export const load: PageServerLoad = async ({ fetch }) => ({
	status: await apiGet<SchedulerStatus>(fetch, '/api/admin/scheduler')
});
