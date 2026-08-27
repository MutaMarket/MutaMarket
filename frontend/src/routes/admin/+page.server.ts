import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type {
	SchedulerStatus,
	ServiceCharacter,
	SystemStats,
	TelemetrySnapshot
} from '$lib/admin-types';

export const load: PageServerLoad = async ({ fetch }) => {
	const [status, telemetry, system, service] = await Promise.all([
		apiGet<SchedulerStatus>(fetch, '/api/admin/scheduler'),
		apiGet<TelemetrySnapshot>(fetch, '/api/admin/telemetry'),
		apiGet<SystemStats>(fetch, '/api/admin/system'),
		apiGet<ServiceCharacter>(fetch, '/api/admin/service-character')
	]);

	return { status, telemetry, system, service };
};
