import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import { loadBrowser } from '$lib/server/browser';
import { moduleIdFromSlug } from '$lib/query';
import type { ModuleDetail } from '$lib/types';

// A slug ending in digits is a module lookup, anything else is the
// browser with filter segments (like the old /modules/{query} route).
export const load: PageServerLoad = async ({ fetch, params }) => {
	if (moduleIdFromSlug(params.query) !== null) {
		const detail = await apiGet<{ data: ModuleDetail }>(fetch, `/api/modules/${params.query}`);
		return { module: detail.data };
	}

	return { module: null, ...(await loadBrowser(fetch, params.query, false)) };
};
