import type { PageServerLoad } from './$types';
import { indexQuery, pageParam } from '#lib/paginated-index.js';
import { apiGet } from '#lib/server/api.js';
import type { CharacterCardData, IndexPage } from '#lib/types-social.js';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const search = url.searchParams.get('search') ?? '';
  const page = pageParam(url.searchParams, 'page');
  return {
    characters: await apiGet<IndexPage<CharacterCardData>>(
      fetch,
      `/api/characters${indexQuery({ search, page })}`,
    ),
    search,
  };
};
