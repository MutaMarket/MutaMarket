import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import { loadPageFilters } from '$lib/server/page-filters';
import type { PersonalModuleEntry, PersonalPageData } from '$lib/types';

// Guests are sent to the login page through the API's 401.
export const load: PageServerLoad = async ({ fetch, params }) => {
	const query = params.query ?? '';
	const [personal, entries, filters] = await Promise.all([
		apiGet<PersonalPageData>(fetch, '/api/personal/page'),
		apiGet<PersonalModuleEntry[]>(fetch, `/api/personal/modules?q=${encodeURIComponent(query)}`),
		loadPageFilters(fetch, query)
	]);
	return { personal, entries, ...filters };
};
