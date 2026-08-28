import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { PersonalContractsPage } from '$lib/personal-contracts';

// The personal contracts page, the legacy ContractController::index:
// the optional date window rides through to the API as query params.
export const load: PageServerLoad = async ({ fetch, url }) => {
	const params = new URLSearchParams();
	for (const name of ['date_start', 'date_end']) {
		const value = url.searchParams.get(name);
		if (value) {
			params.set(name, value);
		}
	}
	const suffix = params.size > 0 ? `?${params}` : '';
	const page = await apiGet<PersonalContractsPage>(fetch, `/api/personal/contracts${suffix}`);
	return { page };
};
