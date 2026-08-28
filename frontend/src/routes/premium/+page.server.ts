import type { PageServerLoad } from './$types';
import type { ModuleDetail } from '$lib/types';
import { apiGet } from '$lib/server/api';

// The premium sales page, the legacy PremiumController::index props.
export const load: PageServerLoad = async ({ fetch }) => {
	const { sample_modules } = await apiGet<{ sample_modules: ModuleDetail[] }>(
		fetch,
		'/api/premium/page'
	);

	return { sampleModules: sample_modules };
};
