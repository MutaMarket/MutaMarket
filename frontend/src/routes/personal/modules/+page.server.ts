import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { PersonalModuleEntry, PersonalPageData } from '$lib/types';

// Guests are sent to the login page through the API's 401.
export const load: PageServerLoad = async ({ fetch }) => {
	const [page, entries] = await Promise.all([
		apiGet<PersonalPageData>(fetch, '/api/personal/page'),
		apiGet<PersonalModuleEntry[]>(fetch, '/api/personal/modules')
	]);

	return { personal: page, entries };
};
