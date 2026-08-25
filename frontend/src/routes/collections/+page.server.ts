import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { CollectionCardData } from '$lib/types-social';

export const load: PageServerLoad = async ({ fetch }) => ({
	collections: await apiGet<CollectionCardData[]>(fetch, '/api/collections')
});
