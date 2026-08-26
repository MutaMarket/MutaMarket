import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';
import type { OfferListItem } from '$lib/types-offers';

// Guests are sent to the login page through the API's 401.
export const load: PageServerLoad = async ({ fetch }) => {
	const offers = await apiGet<OfferListItem[]>(fetch, '/api/offers');
	return { offers };
};
