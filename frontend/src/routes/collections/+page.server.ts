import type { PageServerLoad } from './$types';
import { indexQuery, pageParam } from '$lib/paginated-index';
import { apiGet } from '$lib/server/api';
import type { CollectionCardData, IndexPage } from '$lib/types-social';

// The two sections page independently, like the legacy `page_public`
// and `page` paginators.
export const load: PageServerLoad = async ({ fetch, parent, url }) => {
  const { nav } = await parent();
  const search = url.searchParams.get('search') ?? '';
  const publicPage = pageParam(url.searchParams, 'page_public');
  const personalPage = pageParam(url.searchParams, 'page');
  const [collections, personal] = await Promise.all([
    apiGet<IndexPage<CollectionCardData>>(
      fetch,
      `/api/collections${indexQuery({ search, page_public: publicPage })}`,
    ),
    nav === null
      ? Promise.resolve(null)
      : apiGet<IndexPage<CollectionCardData>>(
          fetch,
          `/api/collections${indexQuery({ personal: 'true', search, page: personalPage })}`,
        ),
  ]);
  return { collections, personal, search };
};
