import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.ts';
import { loadPageFilters } from '#lib/server/page-filters.ts';
import type { CharacterPageData } from '#lib/types-social.ts';

export const load: PageServerLoad = async ({ fetch, params }) => {
  const query = params.query ?? '';
  const [page, filters] = await Promise.all([
    apiGet<CharacterPageData>(
      fetch,
      `/api/characters/${params.character}?q=${encodeURIComponent(query)}`,
    ),
    loadPageFilters(fetch, query),
  ]);
  return { page, ...filters };
};
