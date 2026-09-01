import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { LivePayload } from '$lib/admin-live.svelte';
import type { ServiceCharacter } from '$lib/admin-types';

export const load: PageServerLoad = async ({ fetch }) => {
	const [live, service] = await Promise.all([
		apiGet<LivePayload>(fetch, '/api/admin/live?sections=system,database,jobs'),
		apiGet<ServiceCharacter>(fetch, '/api/admin/service-character'),
	]);

	return { live, service };
};
