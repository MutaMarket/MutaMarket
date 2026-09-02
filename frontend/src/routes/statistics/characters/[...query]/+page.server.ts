import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { TopCharacters } from '$lib/statistics';

// The top-characters tab: the legacy StatisticsController leaderboard,
// type scope via the query segment, name/sort via query params.
export const load: PageServerLoad = async ({ fetch, params, url }) => {
  const query = params.query ?? '';
  const search = new URLSearchParams();
  for (const key of ['name', 'sort_field', 'sort_direction']) {
    const value = url.searchParams.get(key);
    if (value) {
      search.set(key, value);
    }
  }
  const path = query === '' ? '/api/statistics/top' : `/api/statistics/top/${query}`;
  const suffix = search.size > 0 ? `?${search}` : '';

  return {
    query,
    top: await apiGet<TopCharacters>(fetch, `${path}${suffix}`),
    name: url.searchParams.get('name') ?? '',
    sortField: url.searchParams.get('sort_field'),
    sortDirection: url.searchParams.get('sort_direction'),
  };
};
