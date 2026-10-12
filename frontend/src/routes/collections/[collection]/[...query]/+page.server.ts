import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.js';
import { loadPageFilters } from '#lib/server/page-filters.js';
import type { CollectionPageData } from '#lib/types-social.js';

// Known but private collections viewed by a non-owner answer the legacy
// 403 through the API status.
export const load: PageServerLoad = async ({ fetch, params }) => {
  const query = params.query ?? '';
  const [page, filters] = await Promise.all([
    apiGet<CollectionPageData>(
      fetch,
      `/api/collections/${params.collection}?q=${encodeURIComponent(query)}`,
    ),
    loadPageFilters(fetch, query),
  ]);
  return { page, ...filters };
};
