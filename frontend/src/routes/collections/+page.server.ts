import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { CollectionCardData } from '$lib/types-social';

export const load: PageServerLoad = async ({ fetch, parent }) => {
  const { nav } = await parent();
  const [collections, personal] = await Promise.all([
    apiGet<CollectionCardData[]>(fetch, '/api/collections'),
    nav === null
      ? Promise.resolve(null)
      : apiGet<CollectionCardData[]>(fetch, '/api/collections?personal=true'),
  ]);
  return { collections, personal };
};
