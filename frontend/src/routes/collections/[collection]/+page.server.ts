import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { CollectionPageData } from '$lib/types-social';

// Known but private collections viewed by a non-owner answer the legacy
// 403 through the API status.
export const load: PageServerLoad = async ({ fetch, params }) => ({
	page: await apiGet<CollectionPageData>(fetch, `/api/collections/${params.collection}`)
});
