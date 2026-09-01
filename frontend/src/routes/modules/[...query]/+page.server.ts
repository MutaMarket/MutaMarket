import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import { loadBrowser } from '$lib/server/browser';
import { moduleIdFromSlug } from '$lib/query';
import type { ModulePageData } from '$lib/types';

// A slug ending in digits is a module lookup, anything else is the
// browser with filter segments (like the old /modules/{query} route).
export const load: PageServerLoad = async ({ cookies, fetch, params }) => {
	if (moduleIdFromSlug(params.query) !== null) {
		const page = await apiGet<ModulePageData>(fetch, `/api/module-page/${params.query}`);
		return {
			module: page.module,
			estimatorStatistic: page.estimator_statistic,
			sourceTypeComparisons: page.source_type_comparisons,
			historicContracts: page.historic_contracts,
			typeStatistics: page.abyssal_type_statistics,
			showTab: cookies.get('module_show_tab') ?? 'market',
		};
	}

	return { module: null, ...(await loadBrowser(fetch, params.query, false)) };
};
