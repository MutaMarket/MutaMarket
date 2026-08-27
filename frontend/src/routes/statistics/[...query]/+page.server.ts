import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { PersonalStats, StatisticsOverview, TopCharacters } from '$lib/statistics';

// The unified statistics page: market overview, top-creators
// leaderboard (type scope via the query segment, name/sort via query
// params like the legacy StatisticsController) and, for a signed-in
// user, the personal creation stats.
export const load: PageServerLoad = async ({ fetch, params, url }) => {
	const query = params.query ?? '';
	const search = new URLSearchParams();
	for (const key of ['name', 'sort_field', 'sort_direction']) {
		const value = url.searchParams.get(key);
		if (value) {
			search.set(key, value);
		}
	}
	const topPath = query === '' ? '/api/statistics/top' : `/api/statistics/top/${query}`;
	const suffix = search.size > 0 ? `?${search}` : '';

	const [overview, top, personal] = await Promise.all([
		apiGet<StatisticsOverview>(fetch, '/api/statistics/overview'),
		apiGet<TopCharacters>(fetch, `${topPath}${suffix}`),
		// Guests simply have no personal section.
		fetch('/api/personal/stats')
			.then((response) => (response.ok ? (response.json() as Promise<PersonalStats>) : null))
			.catch(() => null)
	]);

	return {
		query,
		overview,
		top,
		personal,
		name: url.searchParams.get('name') ?? '',
		sortField: url.searchParams.get('sort_field'),
		sortDirection: url.searchParams.get('sort_direction')
	};
};
