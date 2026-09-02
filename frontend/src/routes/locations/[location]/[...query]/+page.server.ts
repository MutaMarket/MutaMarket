import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import { loadPageFilters } from '$lib/server/page-filters';
import type { LocationShowData } from '$lib/types';

// One asset location's module browser (legacy ShowLocationPage).
export const load: PageServerLoad = async ({ fetch, params }) => {
  const query = params.query ?? '';
  const path =
    query === ''
      ? `/api/locations/${params.location}`
      : `/api/locations/${params.location}/${query}`;
  const [show, filters] = await Promise.all([
    apiGet<LocationShowData>(fetch, path),
    loadPageFilters(fetch, query),
  ]);
  return { ...show, ...filters, locationSlug: params.location };
};
