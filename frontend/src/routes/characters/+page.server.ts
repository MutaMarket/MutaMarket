import type { PageServerLoad } from './$types';
import { indexQuery, pageParam } from '$lib/paginated-index';
import { apiGet } from '$lib/server/api';
import type { CharacterCardData, IndexPage } from '$lib/types-social';

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
