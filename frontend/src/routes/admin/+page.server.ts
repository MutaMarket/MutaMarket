import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { SchedulerStatus, SystemStats, TelemetrySnapshot } from '$lib/admin-types';

export const load: PageServerLoad = async ({ fetch }) => {
	const [status, telemetry, system] = await Promise.all([
		apiGet<SchedulerStatus>(fetch, '/api/admin/scheduler'),
		apiGet<TelemetrySnapshot>(fetch, '/api/admin/telemetry'),
		apiGet<SystemStats>(fetch, '/api/admin/system')
	]);

	return { status, telemetry, system };
};
